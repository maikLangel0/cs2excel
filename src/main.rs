mod excel;
use excel::{get_exceldata, get_spreadsheet, set_spreadsheet};

mod models;
use models::{
    excel::ExcelData, price::{Currencies, Doppler, PriceType, PricingMode, PricingProvider}, user_sheet::{SheetInfo, UserInfo}, web::{ExtraItemData, ItemInfoProvider, Sites, SteamData, SITE_HAS_DOPPLER}
};

mod parsing;
use parsing::{csgoskins_url, item_csgotrader, market_name_parse};

mod browser;
use browser::{
    cookies::FirefoxDb, csfloat, csgotrader, steamcommunity::SteamInventory
};

use user_sheet_tmp::{SHEET, USER};
use std::{collections::HashMap, error::Error, str::FromStr};

mod user_sheet_tmp;
use reqwest::Client;
use strum::IntoEnumIterator;
use umya_spreadsheet::{Spreadsheet, Worksheet};
use serde_json::Value;
use rand;
use chrono;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // Path to my main invest: C:\Users\Mikae\OneDrive\Skrivebord\workbook
    let excel: SheetInfo = SHEET.clone();
    let user: UserInfo = USER.clone();

    is_user_input_valid(&user, &excel)?;

    // -----------------------------------------------------------------------------------------------

    // BIG BRAIN; READ THE EXCEL SPREADSHEET FIRST TO GET ALL THE INFO AND THEN GET PRICES WOWOWO
    
    // Getting the Worksheet from either existing book or new book
    let mut book: Spreadsheet = get_spreadsheet(&excel.path_to_sheet)?;
    let sheet: &mut Worksheet = {
        if let Some(sn) = &excel.sheet_name { 
            if let Some(buk) = book.get_sheet_by_name_mut(sn) { buk } 
            else {
                println!("WARNING: Automatically fetched first sheet in spreadsheet because {} was not found.", sn);
                book.get_sheet_mut(&0).ok_or_else(|| format!(
                    "Failed to get the first sheet in the spreadsheet with path: \n{:?}", &excel.path_to_sheet)
                )? 
            }  
        } else { book.get_sheet_mut(&0).ok_or("Failed to get first sheet provided by new_file creation.")? }
    };

    // Client for fetch_more_iteminfo
    let mut iteminfo_client_base = {
        if let Some(iip) = &user.iteminfo_provider {
            match iip {
                ItemInfoProvider::Csfloat => { csfloat::new_extra_iteminfo_client() },
                ItemInfoProvider::Csgotrader => { csgotrader::new_extra_iteminfo_client() }
            }
        } else { Client::new() }
    };
    let iteminfo_client: &mut Client = &mut iteminfo_client_base;

    // -----------------------------------------------------------------------------------------------

    let mut exceldata: Vec<ExcelData> = get_exceldata(sheet, &excel, user.ignore_already_sold)?;
    let exceldata_initial_length: usize = exceldata.len();
    
    println!("Data gotten from excel spreadsheet: \n{:#?}", exceldata);
    // if !exceldata.is_empty() { return Ok(()) }

    //  exceldata_old_len er her fordi jeg har endret måte å oppdatere prisene i spreadsheet'n på.
    //  Nå, hvis et item fra steam ikke er i spreadsheetn allerede, så oppdateres spreadsheetn med price, quantity,
    //  phase og inspect link. exceldata_old_len skal være til når resten av itemsene skal oppdateres i pris,
    //  da stopper itereringen ved exceldata_old_len i stedet for å hente prisen til item'ene som er nylig lagt
    //  til og derfor også oppdatert allerede.

    // -----------------------------------------------------------------------------------------------
    
    let steamcookie: Option<String> = get_steamloginsecure(&user.steamloginsecure);

    let (rate, sm_inv) = tokio::try_join!(
        get_exchange_rate(&user.usd_to_x, &excel.rowcol_usd_to_x, sheet), 
        SteamInventory::init(user.steamid, 730, steamcookie)
    )?;

    let cs_inv: Vec<SteamData> = sm_inv.get_steam_items(user.steamid, user.group_simular_items, true)?;

    // -----------------------------------------------------------------------------------------------

    let markets_to_check: Vec<Sites> = if let Some(markets) = &user.prefer_markets { markets.clone() } 
    else { Sites::iter().collect::<Vec<Sites>>() };

    let all_market_prices: HashMap<String, Value> = {
        let mut amp: HashMap<String, Value> = HashMap::new();
        for market in &markets_to_check {    

            let market_prices = match &user.pricing_provider {
                PricingProvider::Csgoskins => { csgotrader::get_market_data( market ).await? }, // IF I IMPLEMENT CSGOSKINS IN THE FUTURE
                PricingProvider::Csgotrader => { csgotrader::get_market_data( market ).await? },
            };

            amp.insert(market.to_string(), market_prices);
        }
        amp
    };
    
    // -----------------------------------------------------------------------------------------------
    // Inserting and/or updating quantity + adding prices for newly inserted items

    for steamdata in cs_inv.iter() {
        if !user.fetch_steam { break }

        println!("\nCURRENT STEAMDATA {:#?}", steamdata);

        if user.group_simular_items {
            assert!(excel.col_quantity.is_some());

            match exceldata.iter_mut().enumerate().find( |(_, e)| e.name == steamdata.name ) { 
                Some((index, data)) => {
                    
                    // Skip item if item is in ignore market names
                    if let Some(ignore) = &user.ingore_steam_names { 
                        if ignore.iter().any(|n| data.name == *n.trim() ) { continue; }
                    }

                    let row_in_excel: usize = index + excel.row_start_write_in_table as usize;
                    
                    // if exceldatas data has phase info AND user wants to fetch more iteminfo AND cs inventory's steamdata has an inspect link,
                    // don't update quantity and jump to next iteration of cs inv. Instead execute the logic underneath this match statement
                    if data.phase.is_some() && user.iteminfo_provider.is_some() && steamdata.inspect_link.is_some() {}
                    else { 
                        update_quantity_exceldata(
                            &steamdata, 
                            &excel.col_quantity, 
                            data, 
                            row_in_excel, 
                            sheet
                        ); 

                        // If quantity is more than 1, remove data in float, pattern and inspect_link if its set
                        if data.quantity > Some(1) {
                            if let Some(col_float) = &excel.col_float {
                                let cell = format!("{}{}", col_float, row_in_excel);
                                sheet.get_cell_value_mut(cell).set_value_string("");
                            }
                            if let Some(col_pattern) = &excel.col_pattern {
                                let cell = format!("{}{}", col_pattern, row_in_excel);
                                sheet.get_cell_value_mut(cell).set_value_string("");
                            }
                            if let Some(col_inspect) = &excel.col_inspect_link {
                                let cell = format!("{}{}", col_inspect, row_in_excel);
                                sheet.get_cell_value_mut(cell).set_value_string("");
                            }
                        }

                        continue;
                    }
                },
                None => {
                    let row_in_excel: usize = exceldata.len() + excel.row_start_write_in_table as usize;

                    let extra_itemdata: Option<ExtraItemData> = if let Some(quant) = steamdata.quantity {
                        if quant == 1 || steamdata.name.contains( " doppler ") {
                            wrapper_fetch_iteminfo_via_itemprovider_persistent(
                                iteminfo_client, 
                                &user.iteminfo_provider, 
                                &excel.col_inspect_link, 
                                user.pause_time_ms, 
                                &steamdata
                            ).await?
                        } else { None }
                    } else { None };

                    exceldata.push( 
                        insert_new_exceldata(
                            &user, &excel, 
                            &steamdata, 
                            &extra_itemdata,
                            &markets_to_check, 
                            &all_market_prices, 
                            rate, row_in_excel, 
                            sheet
                        ).await? 
                    );
                    continue;  
                }
            }
            assert!(excel.col_inspect_link.is_some());
            assert!(steamdata.inspect_link.is_some());
            assert!(user.iteminfo_provider.is_some());

            // Only reached when exceldatas name is the same as steamdatas name AND 
            // exceldatas phase is something AND user wants to fetch more iteminfo AND 
            // steamdatas inspect link is something
            let extra_itemdata: ExtraItemData = wrapper_fetch_iteminfo_via_itemprovider_persistent(
                iteminfo_client, 
                &user.iteminfo_provider, 
                &excel.col_inspect_link, 
                user.pause_time_ms, 
                &steamdata
            ).await?.ok_or_else(|| "group_simular_items' path for dopplers failed WHAT")?;

            let phase: &Option<String> = &extra_itemdata.phase
                .as_ref()
                .and_then( |p| Some( p.as_str() ) )
                .map( str::to_owned );

            match exceldata.iter_mut().enumerate().find( |(_, e)| e.name == steamdata.name && e.phase == *phase ) {
                Some((index, data)) => {
                    let row_in_excel: usize = index + excel.row_start_write_in_table as usize;

                    update_quantity_exceldata(
                        &steamdata, 
                        &excel.col_quantity, 
                        data, 
                        row_in_excel, 
                        sheet
                    ); 
                },
                None => {
                    let row_in_excel: usize = exceldata.len() + excel.row_start_write_in_table as usize;

                    exceldata.push( 
                        insert_new_exceldata(
                            &user, &excel,
                            &steamdata,
                            &Some(extra_itemdata),
                            &markets_to_check,
                            &all_market_prices,
                            rate, row_in_excel, 
                            sheet
                        ).await? 
                    );
                }
            }
        }      // If not group_simular_items
        else {
            if exceldata.iter().find(|e| e.asset_id == Some(steamdata.asset_id) && e.name == steamdata.name).is_none() {
                let row_in_excel: usize = exceldata.len() + excel.row_start_write_in_table as usize;

                let extra_itemdata: Option<ExtraItemData> = wrapper_fetch_iteminfo_via_itemprovider_persistent(
                    iteminfo_client, 
                    &user.iteminfo_provider, 
                    &excel.col_inspect_link, 
                    user.pause_time_ms, 
                    &steamdata
                ).await?;

                exceldata.push( 
                    insert_new_exceldata(
                        &user, &excel,
                        &steamdata,
                        &extra_itemdata,
                        &markets_to_check,
                        &all_market_prices,
                        rate, row_in_excel, 
                        sheet
                    ).await? 
                );
            }
        }
    }

    /*
        NOW THE ONLY THING LEFT IS THE PRICE CHECKING | REMEMBER
            - DONT NEED TO FETCH MORE ITEMDATA UNLESS ExcelData.phase.is_some()
    */

    for (i, data) in exceldata.iter().enumerate() {
        if !user.fetch_prices { break }
        if i == exceldata_initial_length { break }

        if let Some(ignore) = &user.ingore_steam_names {
            for ignore_steam_name in ignore {
                if data.name == *ignore_steam_name { continue; }
            }
        }

        if data.sold.is_some() { continue; }

        let row_in_excel = i + excel.row_start_write_in_table as usize;
        let cell_price = format!("{}{}", excel.col_price, row_in_excel);

        let doppler: Option<Doppler> = {
            if let Some(phase) = &data.phase {
                Some(Doppler::from_str(phase)?)
            } else { None }
        };

        let (market, price): (Option<String>, Option<f64>) = get_market_price(
            &user, 
            &markets_to_check, 
            &all_market_prices, 
            rate, 
            &data.name, 
            &data.phase, 
            &doppler
        )?;

        if let Some(pris) = price {
            sheet.get_cell_value_mut(cell_price).set_value_number(pris);
        }

        if let Some(marked) = market {
            if let Some(col_market) = &excel.col_market {
                let cell_market = format!("{}{}", col_market, row_in_excel);
                sheet.get_cell_value_mut(cell_market).set_value_string(marked);
            }
        }
    }

    if let Some(cell_date) = &excel.rowcol_date {
        sheet.get_cell_value_mut( cell_date.as_ref() )
            .set_value_string( 
                chrono::Local::now()
                    .format("%d/%m/%Y %H:%M:%S")
                    .to_string() 
            );
    }

    set_spreadsheet(&excel.path_to_sheet, book)
        .map_err(|e| format!("Couldnt write to spreadsheet! : {}", e))?;

    // wowzers
    println!("EXCELDATA: \n\n{:#?}\n", exceldata);
    println!("Asset length: {}", sm_inv.get_assets_length());
    println!("Inventory length: {}", sm_inv.get_total_inventory_length() );

    Ok(())
}

// -------------------------------------------------------------------------------------------

fn get_steamloginsecure(sls: &Option<String>) -> Option<String> {
    if let Some(sls) = sls { 
        Some(sls.to_string())
    } else { 
        if let Ok(db) = FirefoxDb::init() {
            match db.get_cookies(vec!["name", "value"], "steamcommunity.com", vec!["steamLoginSecure"]) {
                Ok(cookie) => Some(cookie),
                Err(e) => { println!("FRICK.\n{}", e); None }
            }      
        } else { println!("WARNING: Failed to connect to firefox."); None }
    }
}

async fn get_exchange_rate(
    usd_to_x: &Option<Currencies>, 
    rowcol_usd_to_x: &Option<String>, 
    sheet: &mut Worksheet
) -> Result<f64, String> {
    
    debug_assert!( !( usd_to_x.is_some() && rowcol_usd_to_x.is_some() ) );

    if let Some(currency) = usd_to_x {
        if currency == &Currencies::USD { return Ok(1.0); }
        let rates: HashMap<String, f64> = csgotrader::get_exchange_rates().await?;
        Ok( *rates.get( currency.as_str() ).unwrap_or( &1.0 ) )
    } else { 
        
        if let Some(cell) = rowcol_usd_to_x {
            let res = sheet.get_cell_value( cell.as_ref() )
                .get_raw_value()
                .to_string()
                .trim()
                .to_string();
            
            if res.is_empty() { Err( String::from("usd_to_x cell is empty!") ) }
            else {
                Ok(
                    res.parse::<f64>()
                        .map_err(|_| String::from(
                            "usd_to_x cell was not able to be converted to a number!"
                        ) 
                    )?
                )
            }

        } else { Ok(1.0) }
    }
}

fn is_user_input_valid(user: &UserInfo, excel: &SheetInfo) -> Result<(), String> {
    if !user.iteminfo_provider.is_some() {
        println!("WARNING: Pricing for doppler phases will not be accurate with fetch_more_iteminfo off.")
    }

    if !user.iteminfo_provider.is_some() && excel.col_inspect_link.is_some() {
        println!("WARNING: col_inspect_link is not defined so you will not be able to fetch_more_iteminfo (float, doppler phase, pattern, price of doppler).")
    }
    
    // --------------------

    if excel.path_to_sheet.is_some() { 
        if excel.sheet_name.is_none() {
            return Err( String::from( "Sheet name can't be nothing if path to sheet is given." ) )
        }
    }

    if excel.col_inspect_link.is_none() {
        if excel.col_quantity.is_none(){ return Err( String::from( "Quantity can't be empty when no inspect link column is given." ) ) }
        if excel.col_float.is_some()   { return Err( String::from( "Column for float given but no column for inspect link."   ) ) }
        if excel.col_phase.is_some()   { return Err( String::from( "Column for phase given but no column for inspect link."   ) ) }
        if excel.col_pattern.is_some() { return Err( String::from( "Column for pattern given but no column for inspect link." ) ) }
    }

    if user.iteminfo_provider.is_some() && excel.col_inspect_link.is_some() && excel.col_phase.is_none() {
        return Err( String::from( "Phase of doppler knives will not be pricechecked correctly when reading over the spreadsheet in the future becuase col_phase is not set!" ))
    }

    if user.pause_time_ms < 1000 || user.pause_time_ms > 10000 {
        return Err( String::from("pause_time_ms is only allowed to be in range of 1000 (2 seconds) - 10000 (10 seconds).") )
    }

    if excel.col_quantity.is_none() && user.group_simular_items {
        return Err( String::from("col_quantity can't be None if you want to group simular items!") )
    }

    if excel.col_asset_id.is_none() && !user.group_simular_items {
        return Err( String::from("col_asset_id can't be None if you don't want to group simular items!") )
    }

    if excel.rowcol_usd_to_x.is_some() && user.usd_to_x.is_some() {
        return Err( String::from("rowcol_usd_to_x can't be something if usd_to_x is set as a currency!") )
    }

    Ok(())
}

fn update_quantity_exceldata(
    steamdata: &SteamData, 
    col_quantity: &Option<String>,
    data: &mut ExcelData, 
    row_in_excel: usize, 
    sheet: &mut Worksheet, 
) {
    if let Some(col_quantity) = col_quantity {
        if let Some(steam_quantity) = steamdata.quantity {
            if let Some(data_quantity) = data.quantity { 
                if data_quantity < steam_quantity {
                    let cell_quantity = format!("{}{}", col_quantity, row_in_excel);

                    data.quantity = Some(steam_quantity);
                    sheet.get_cell_value_mut( cell_quantity.as_ref() ).set_value_number(steam_quantity);
                    println!("UPDATED {} QUANTITY TO {:?} | ROW {}\n", &steamdata.name, &data.quantity, &row_in_excel);
                }
            }
        }
    }
}

async fn insert_new_exceldata(
    user: &UserInfo, 
    excel: &SheetInfo, 
    steamdata: &SteamData, 
    extra_itemdata: &Option<ExtraItemData>,
    markets_to_check: &Vec<Sites>, 
    all_market_prices: &HashMap<String, Value>, 
    rate: f64, 
    row_in_excel: usize,
    sheet: &mut Worksheet
) -> Result<ExcelData, String> {

    let (doppler, phase): (Option<Doppler>, Option<String>) = {

        if let Some(itemdata) = extra_itemdata {
            let phase: Option<String> = itemdata.phase.clone()
                .and_then( |p| Some( p.as_str() ) )
                .map( str::to_owned );

            (itemdata.phase.clone(), phase)
        } else { (None, None) }
    };

    let (market, price): (Option<String>, Option<f64>) = get_market_price(
        user, 
        markets_to_check, 
        all_market_prices, 
        rate, 
        &steamdata.name, 
        &phase, 
        &doppler
    )?;

    // Inserting into the spreadsheet
    let cell_steam_name = format!("{}{}", excel.col_steam_name, row_in_excel);
    sheet.get_cell_value_mut(cell_steam_name).set_value_string(&steamdata.name);

    if excel.col_gun_sticker_case.is_some() || excel.col_skin_name.is_some() || excel.col_wear.is_some() {
        let [gun_sticker_case, skin_name, wear] = market_name_parse::metadata_from_market_name(&steamdata.name);

        if let Some(col_gun_sticker_case) = &excel.col_gun_sticker_case {
            if !gun_sticker_case.is_empty() {
                let cell_gsc = format!("{}{}", col_gun_sticker_case, row_in_excel);
                sheet.get_cell_value_mut(cell_gsc).set_value_string(gun_sticker_case);
            }
        }

        if let Some(col_skin_name) = &excel.col_skin_name {
            if !skin_name.is_empty() {
                let cell_sn = format!("{}{}", col_skin_name, row_in_excel);
                sheet.get_cell_value_mut(cell_sn).set_value_string(skin_name);
            }
        }

        if let Some(col_wear) = &excel.col_wear {
            if !wear.is_empty() {
                let cell_wear = format!("{}{}", col_wear, row_in_excel);
                sheet.get_cell_value_mut(cell_wear).set_value_string(wear);
            }
        }
    }

    if let Some(itemdata) = extra_itemdata {
        
        if let Some(col_float) = &excel.col_float {
            if let Some(float) = itemdata.float {
                let cell_float = format!("{}{}", col_float, row_in_excel);
                sheet.get_cell_value_mut(cell_float).set_value_number(float);
            }
        }

        if let Some(col_pattern) = &excel.col_pattern {
            if let Some(pattern) = itemdata.paintseed {
                let cell_pattern = format!("{}{}", col_pattern, row_in_excel);
                sheet.get_cell_value_mut(cell_pattern).set_value_number(pattern);
            }
        }

        if let Some(col_phase) = &excel.col_phase {
            if let Some(faze) = &itemdata.phase {
                let cell_phase = format!("{}{}", col_phase, row_in_excel);
                sheet.get_cell_value_mut(cell_phase).set_value_string(faze.as_str());
            }
        }
    }

    if let Some(col_quantity) = &excel.col_quantity {
        if let Some(quantity) = steamdata.quantity {
            let cell_quantity = format!("{}{}", col_quantity, row_in_excel);
            sheet.get_cell_value_mut(cell_quantity).set_value_number(quantity);
        }
    }

    if let Some(monetary) = price {
        let cell_price = format!("{}{}", &excel.col_price, row_in_excel);
        sheet.get_cell_value_mut(cell_price).set_value_number(monetary);
    }

    if let Some(col_market) = &excel.col_market {
        if let Some(marquet) = market {
            let cell_market = format!("{}{}", col_market, row_in_excel);
            sheet.get_cell_value_mut(cell_market).set_value_string(marquet);
        }
    }

    if let Some(col_inspect_link) = &excel.col_inspect_link {
        if let Some(inspect_link) = &steamdata.inspect_link {
            let cell = format!("{}{}", col_inspect_link, row_in_excel);
            sheet.get_cell_value_mut(cell).set_value_string(inspect_link);
        }
    }

    if let Some(col_asset_id) = &excel.col_asset_id {
        if !user.group_simular_items {
            let cell = format!("{}{}", col_asset_id, row_in_excel);
            sheet.get_cell_value_mut(cell).set_value_number(steamdata.asset_id as f64);
        }
    }

    if let Some(col_csgoskins_link) = &excel.col_csgoskins_link {
        let csgoskins_url = csgoskins_url::create_csgoskins_urls(&steamdata.name);

        let cell = format!("{}{}", col_csgoskins_link, row_in_excel);
        let link = format!("https://csgoskins.gg/items/{}", csgoskins_url);

        sheet.get_cell_value_mut(cell).set_value_string(link);
    }

    Ok(ExcelData { 
        name: steamdata.name.clone(), 
        quantity: steamdata.quantity, 
        phase, 
        price: price.map_or_else(|| 0.0, |p| p), 
        inspect_link: steamdata.inspect_link.clone(),
        asset_id: if !user.group_simular_items { Some(steamdata.asset_id) } else { None },
        sold: None
    })
}

async fn fetch_iteminfo_via_itemprovider_persistent(
    client: &mut Client,
    col_inspect_link: &Option<String>,
    iteminfo_provider: &ItemInfoProvider,
    inspect_link: &Option<String>,
    pause_time_ms: u64
) -> Result<Option<Value>, String> {
    
    if col_inspect_link.is_some() {
        if let Some(inspect) = inspect_link {
            match iteminfo_provider {
                ItemInfoProvider::Csfloat => {
                    let tmp = csfloat::fetch_iteminfo_persistent(client, inspect, 10, pause_time_ms).await?;
                    Ok(tmp)
                }
                ItemInfoProvider::Csgotrader => {
                    let tmp = csgotrader::fetch_iteminfo_persistent(client, inspect, 10, pause_time_ms).await?;
                    Ok(tmp)
                }
            }
            
        } else { Ok(None) }
    } else { Ok(None) }
}

async fn wrapper_fetch_iteminfo_via_itemprovider_persistent(
    client: &mut Client,
    iteminfo_provider: &Option<ItemInfoProvider>,
    col_inspect_link: &Option<String>,
    pause_time_ms: u64,
    steamdata: &SteamData,
) -> Result<Option<ExtraItemData>, String> {
    
    if let Some(item_provide) = iteminfo_provider {
        let tmp = fetch_iteminfo_via_itemprovider_persistent(
            client, 
            col_inspect_link, 
            item_provide,
            &steamdata.inspect_link, 
            pause_time_ms
        ).await?;
        
        if let Some(raw) = tmp {
            match item_provide {
                ItemInfoProvider::Csfloat => { 
                    let res = parsing::item_csfloat::parse_iteminfo_min(&raw, &steamdata)?;
                    Ok(Some(res)) 
                },
                ItemInfoProvider::Csgotrader => { 
                    let res = parsing::item_csgotrader::parse_iteminfo_min(&raw, &steamdata)?;
                    Ok(Some(res))
                }
            }
        } else { Ok(None) }
    } else { Ok(None) }
}

fn get_market_price(
    user: &UserInfo,
    markets_to_check: &Vec<Sites>,
    all_market_prices: &HashMap<String, Value>,
    rate: f64,
    item_name: &String,
    phase: &Option<String>,
    doppler: &Option<Doppler>
) -> Result<(Option<String>, Option<f64>), String> {
    if !user.fetch_prices { Ok((None, None)) } 
    else {
        #[derive(Debug, Clone, Copy)]
        struct MarketPrice { market: &'static str, price: f64 }
        
        let mut prices: Vec<MarketPrice> = Vec::new();
        
        // Finds the prices for each market
        for market in markets_to_check {
            // If site does not have doppler pricings AND doppler is something, SKIP
            if phase.is_some() && !*SITE_HAS_DOPPLER.get(market)
                .ok_or_else(|| "Didnt find market in SITE_HAS_DOPPLER what?")? { continue; }
            
            if let Some(market_prices) = all_market_prices.get( market.as_str() ) {
                if let Some(price) = item_csgotrader::get_price(
                    item_name, 
                    market_prices, 
                    market, 
                    &PriceType::StartingAt, 
                    doppler
                ) { prices.push( MarketPrice { market: market.as_str(), price: price * rate } ) }    
            }
        }
        if prices.is_empty() { Ok((Some("No Market(s) Found".to_string()), None)) } 
        else {
            match user.pricing_mode {
                PricingMode::Cheapest => { 
                    prices.sort_by(|a,b| a.price.partial_cmp(&b.price).unwrap());
                    Ok((Some(prices[0].market.to_string()), Some(prices[0].price)))
                },
                PricingMode::MostExpensive => { 
                    prices.sort_by(|a,b| b.price.partial_cmp(&a.price).unwrap());
                    Ok((Some(prices[0].market.to_string()), Some(prices[0].price)))
                },                          
                PricingMode::Random => {
                    let wiener = prices.get( rand::random_range(0..prices.len()) )
                        .ok_or_else(|| "PricingMode::Random failed what.")
                        .map(|mp| *mp)?;
                    Ok((Some(wiener.market.to_string()), Some(wiener.price)))
                },
                PricingMode::Hierarchical => { 
                    prices.sort_by(|a,b| a.price.partial_cmp(&b.price).unwrap());
                    let mut curr = MarketPrice { market: prices[0].market, price: prices[0].price };
                    for mp in prices.iter().skip(1) {
                        if curr.price > mp.price * user.percent_threshold as f64 { 
                            curr = *mp 
                        }
                    }
                    Ok((Some(curr.market.to_string()), Some(curr.price)) )
                }
            }
        }
    }
}
