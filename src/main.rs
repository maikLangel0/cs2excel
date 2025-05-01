mod excel;
use excel::{get_exceldata, get_spreadsheet};

mod models;
use models::{
    excel::ExcelData, price::{Currencies, Doppler, PriceType, PricingMode, PricingProvider}, user_sheet::{SheetInfo, UserInfo}, web::{ExtraItemData, ItemInfoProvider, Sites, SteamData, SITE_HAS_DOPPLER}
};

mod parsing;
use parsing::item_csgotrader;

mod browser;
use browser::{
    cookies::FirefoxDb, csfloat, csgotrader, steamcommunity::SteamInventory
};

use user_sheet_tmp::{SHEET, USER};
use std::{collections::HashMap, error::Error};

mod user_sheet_tmp;
use reqwest::Client;
use strum::IntoEnumIterator;
use umya_spreadsheet::{Spreadsheet, Worksheet};
use serde_json::Value;
use rand;

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

    let mut exceldata: Vec<ExcelData> = get_exceldata(sheet, &excel)?;
    let exceldata_initial_length: usize = exceldata.len();
    
    println!("Data gotten from excel spreadsheet: \n{:#?}", exceldata);
    if !exceldata.is_empty() { return Ok(()) }

    //  exceldata_old_len er her fordi jeg har endret måte å oppdatere prisene i spreadsheet'n på.
    //  Nå, hvis et item fra steam ikke er i spreadsheetn allerede, så oppdateres spreadsheetn med price, quantity,
    //  phase og inspect link. exceldata_old_len skal være til når resten av itemsene skal oppdateres i pris,
    //  da stopper itereringen ved exceldata_old_len i stedet for å hente prisen til item'ene som er nylig lagt
    //  til og derfor også oppdatert allerede.

    // -----------------------------------------------------------------------------------------------
    
    let steamcookie: Option<String> = get_steamloginsecure(&user.steamloginsecure);

    let (rate, sm_inv) = tokio::try_join!(
        get_exchange_rate(&user.usd_to_x), 
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

    for steamdata in cs_inv {
        if !user.fetch_steam { break }

        println!("\nCURRENT STEAMDATA {:#?}", steamdata);
        //println!("\nCURRENT STEAMDATA NAME: {}", steamdata.name);

        if user.group_simular_items {
            assert!( excel.col_quantity.is_some() );

            let index_of_item: usize = exceldata.iter()
                .position(|e| e.name == steamdata.name)
                .unwrap_or( exceldata.len() );
            let row: usize = index_of_item + excel.row_start_write_in_table as usize - 1;

            match exceldata.iter_mut().find( |e| (e.name == steamdata.name) ) { 
                Some(data) => {
                    
                    // if exceldatas data has phase info AND user wants to fetch more iteminfo AND cs inventory's steamdata has an inspect link,
                    // don't update quantity and jump to next iteration of cs inv. Instead execute the logic underneath this match statement
                    if data.phase.is_some() && user.iteminfo_provider.is_some() && steamdata.inspect_link.is_some() {}
                    else { 
                        update_quantity_exceldata(
                            &steamdata, 
                            &excel.col_quantity, 
                            data, 
                            row, 
                            sheet
                        ); 
                        continue;
                    }
                },
                None => {
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
                            rate, row, sheet
                        ).await? 
                    );
                    continue;  
                }
            }

            assert!( excel.col_inspect_link.is_some() );
            assert!( steamdata.inspect_link.is_some() );
            assert!( user.iteminfo_provider.is_some() );

            // Only reached when exceldatas name is the same as steamdatas name AND 
            // exceldatas phase is something AND user wants to fetch more iteminfo AND 
            // steamdatas inspect link is something
            let extra_itemdata: ExtraItemData = wrapper_fetch_iteminfo_via_itemprovider_persistent(
                iteminfo_client, 
                &user.iteminfo_provider, 
                &excel.col_inspect_link, 
                user.pause_time_ms, 
                &steamdata
            ).await?.ok_or_else(|| "should never happen")?;

            let phase: Option<String> = extra_itemdata.phase
                .and_then( |p| Some( p.as_str() ) )
                .map( str::to_owned );
        
            let index_of_item: usize = exceldata.iter()
                .position(|e| e.name == steamdata.name && e.phase == phase )
                .unwrap_or( exceldata.len() );
            let row: usize = index_of_item + excel.row_start_write_in_table as usize - 1;

            match exceldata.iter_mut().find( |e| e.name == steamdata.name && e.phase == phase ) {
                Some(data) => {

                },
                
                None => {

                }
            }
        } // If not group_simular_items
        else {
            let row: usize = exceldata.len() + excel.row_start_write_in_table as usize - 1;

            if exceldata.iter()
                .find(|e| e.asset_id == Some(steamdata.asset_id) && e.name == steamdata.name)
                .is_none() {

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
                        rate, row, sheet
                    ).await? 
                );
            }
        }
    }

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

async fn get_exchange_rate(usd_to_x: &Option<Currencies>) -> Result<f64, String> {
    if let Some(currency) = usd_to_x {
        if currency == &Currencies::USD { return Ok(1.0); }
        let rates: HashMap<String, f64> = csgotrader::get_exchange_rates().await?;
        Ok( *rates.get( currency.as_str() ).unwrap_or( &1.0 ) )
    } else { 
        Ok(1.0) 
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

    if user.pause_time_ms < 1 || user.pause_time_ms > 10000 {
        return Err( String::from("pause_time_ms is only allowed to be in range of 1000 (2 seconds) - 10000 (10 seconds).") )
    }

    if excel.col_quantity.is_none() && user.group_simular_items {
        return Err( String::from("col_quantity can't be None if you want to group simular items!") )
    }

    Ok(())
}

fn update_quantity_exceldata(
    steamdata: &SteamData, 
    col_quantity: &Option<String>,
    data: &mut ExcelData, 
    row: usize, 
    sheet: &mut Worksheet, 
) {
    if let Some(col_quantity) = col_quantity {
        let cell_quantity = format!("{}{}", col_quantity, row);
        
        if let Some(steam_quantity) = steamdata.quantity {
            if let Some(data_quantity) = data.quantity { 
                if data_quantity < steam_quantity {
                    data.quantity = Some(steam_quantity);
                    sheet.get_cell_value_mut( cell_quantity.as_ref() ).set_value_number(steam_quantity);
                    println!("UPDATED {} QUANTITY TO {:?} | ROW {}\n", &steamdata.name, &data.quantity, &row);
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
    curr_row: usize,
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

    let (market, price): (Option<String>, Option<f64>) = {

        if !user.fetch_prices { (None, None) } 
        else {
            // Legit just to make shit more pretty
            #[derive(Debug, Clone, Copy)]
            struct MarketPrice { market: &'static str, price: f64 }

            let mut prices: Vec<MarketPrice> = Vec::new();

            for market in markets_to_check {

                // If site does not have doppler pricings AND doppler is something, SKIP
                if phase.is_some() && !*SITE_HAS_DOPPLER.get(market)
                    .ok_or_else(|| "Should never happen")? { continue; }
                
                if let Some(market_prices) = all_market_prices.get( market.as_str() ) {
                    if let Some(price) = item_csgotrader::get_price(
                        &steamdata.name, 
                        &market_prices, 
                        &market, 
                        &PriceType::StartingAt, 
                        &doppler
                    ) { prices.push( MarketPrice { market: market.as_str(), price: price * rate } ) }    
                }
            }
            println!("{:#?}", prices);

            if prices.is_empty() { (Some("No Market(s) Found".to_string()), None) } 
            else {
                match user.pricing_mode {
                    PricingMode::Cheapest => { 
                        prices.sort_by(|a,b| a.price.partial_cmp(&b.price).unwrap());
                        (Some(prices[0].market.to_string()), Some(prices[0].price))
                    },
                    PricingMode::MostExpensive => { 
                        prices.sort_by(|a,b| b.price.partial_cmp(&a.price).unwrap());
                        (Some(prices[0].market.to_string()), Some(prices[0].price))
                    },                          
                    PricingMode::Random => {
                        let wiener = prices.get(rand::random_range(0..prices.len()))
                            .ok_or_else(|| "what")
                            .map(|mp| *mp)?;
                        (Some(wiener.market.to_string()), Some(wiener.price))
                    },
                    PricingMode::Hierarchical => { 
                        prices.sort_by(|a,b| a.price.partial_cmp(&b.price).unwrap());
                        let mut curr = MarketPrice { market: prices[0].market, price: prices[0].price };
                        for mp in prices.iter().skip(1) {
                            if curr.price > mp.price * user.percent_threshold as f64 { 
                                curr = *mp 
                            }
                        }
                        (Some(curr.market.to_string()), Some(curr.price)) 
                    }
                }
            }
        }
    };

    println!("Doppler phase? : {:?}", phase);

    // ALSO REMEMBER TO ACTUALLY WRITE TO THE SPREADSHEET

    Ok(ExcelData { 
        name: steamdata.name.clone(), 
        quantity: steamdata.quantity, 
        phase, 
        price: price.map_or_else(|| 0.0, |p| p), 
        inspect_link: steamdata.inspect_link.clone(),
        asset_id: if !user.group_simular_items { Some(steamdata.asset_id) } else { None }
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

/* 
TODO:
    FIX THE LOGIC OF WHEN WE WANT TO GROUP ITEMS, BUT ITS A DOPPLER WITH THE SAME NAME AS ONE IN THE SPREADSHEET
    ALSO .find() ONLY TAKES FIRST ELEMENT FUCK
    maybe make it so that group_simular_items does not need to have asset_id set (?)
*/
