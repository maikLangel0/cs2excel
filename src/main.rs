mod excel;
use excel::{get_exceldata, get_spreadsheet};

mod models;
use indexmap::IndexMap;
use models::{
    excel::ExcelData, item::ItemData, price::{Currencies, Doppler, PriceType, PricingProvider}, user_sheet::{SheetInfo, UserInfo}, web::{Sites, SteamData}
};

mod user_sheet_tmp;
use strum::IntoEnumIterator;
use user_sheet_tmp::{SHEET, USER};

mod pricing;
use pricing::{price_csgotrader, prices};

mod browser;
use browser::{
    cookies::FirefoxDb, 
    csgotrader, 
    steamcommunity::SteamInventory
};

use std::{collections::HashMap, error::Error, str::FromStr, time::Duration};
use urlencoding;
use umya_spreadsheet::{Spreadsheet, Worksheet};
use serde_json::Value;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // Path to my main invest: C:\Users\Mikae\OneDrive\Skrivebord\workbook
    let excel: SheetInfo = SHEET.clone();
    let user: UserInfo = USER.clone();

    is_sheetinfo_valid(&excel)?;

    if !user.update_prices && !user.update_steam {
        // just to test encode produces the same result
        assert_eq!(
            "steam%3A%2F%2Frungame%2F730%2F76561202255233023%2F%2Bcsgo_econ_action_preview%2520S76561198389123475A34543022281D9279926981479153949", 
            urlencoding::encode("steam://rungame/730/76561202255233023/+csgo_econ_action_preview%20S76561198389123475A34543022281D9279926981479153949") 
        );


        let steam_json_bluelam_action_link_raw = "steam://rungame/730/76561202255233023/+csgo_econ_action_preview%20S%owner_steamid%A%assetid%D9279926981479153949".to_string();
        let owner_steamid = user.steamid.to_string();
        let assetid = "34543022281".to_string();

        let steam_json_bluelam_inspect_link = steam_json_bluelam_action_link_raw
            .replace("%owner_steamid%", &owner_steamid)
            .replace("%assetid%", &assetid);

        println!("BLUE LAM INSPECT LINK FINAL: {}", steam_json_bluelam_inspect_link);

        // https://steam://rungame/730/76561202255233023/+csgo_econ_action_preview%20S76561198389123475A35874511553D1039920615144687297
        // steam://rungame/730/76561202255233023/+csgo_econ_action_preview%20S%owner_steamid%A%assetid%D1039920615144687297

        // wanted end result: https://api.csgotrader.app/float?url={ encoded_inspect_link }
        // https://github.com/gergelyszabo94/csgo-trader-extension/blob/8a950287ac2a7738531be68291282fa87cc27182/extension/src/backgroundScripts/messaging.js#L98
        // USE EXCEL AS CACHE IF FETCHING THE FLOAT IS NECCESSARY OR NOT!

        return Ok(());
    } 

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
                    "Failed to get the first sheet in the spreadsheet with path: \n{:?}", excel.path_to_sheet)
                )? 
            }  
        } else { book.get_sheet_mut(&0).ok_or("Failed to get first sheet provided by new_file creation.")? }
    };

    // -----------------------------------------------------------------------------------------------

    //let exceldata: Vec<ExcelData> = get_exceldata(sheet, &excel)?;
    let mut exceldata: IndexMap<String, ExcelData> = get_exceldata(sheet, &excel)?;
    let exceldata_unaltered_length: usize = exceldata.len(); 

    //  exceldata_unaltered_length er her fordi jeg har endret måte å oppdatere prisene i spreadsheet'n på.
    //  Nå, hvis et item fra steam ikke er i spreadsheetn allerede, så oppdateres spreadsheetn med price, quantity,
    //  phase og inspect link. exceldata_unaltered_length skal være til når resten av itemsene skal oppdateres i pris,
    //  da stopper itereringen ved exceldata_unaltered_length i stedet for å hente prisen til item'ene som er nylig lagt
    //  til og derfor også oppdatert allerede.

    println!("{:#?}", exceldata);

    if !exceldata.is_empty() { return Ok(()) }

    // -----------------------------------------------------------------------------------------------

    let rate: f64 = get_exchange_rate(&user.usd_to_x).await?;
    let steamcookie: Option<String> = get_steamloginsecure(&user.steamloginsecure);

    // -----------------------------------------------------------------------------------------------

    let sm_inv: SteamInventory = SteamInventory::init(user.steamid, 730, steamcookie).await?;
    let cs_inv: Vec<SteamData> = sm_inv.get_steam_items(user.steamid, user.group_simular_items, true)?;
    let itemdata: HashMap<String, ItemData> = prices::get_prices(&user, &cs_inv, rate).await?;          // Changed to hashmap for quicker lookup and no O(N^2) complexity

    // -----------------------------------------------------------------------------------------------

    let markets_to_check: Vec<Sites> = if let Some(markets) = &user.prefer_markets { markets.clone() } 
    else { Sites::iter().collect::<Vec<Sites>>() };

    let all_market_prices: HashMap<String, Value> = {
        let mut amp: HashMap<String, Value> = HashMap::new();
        for market in &markets_to_check {    

            let market_prices = match &user.pricing_provider {
                PricingProvider::Csgoskins => { csgotrader::get_market_data( market ).await? }, // IF I IMPLEMENT CSGOSKINS IN THE FUTURE
                PricingProvider::Csgotrader => { csgotrader::get_market_data( market ).await? }
            };

            amp.insert(market.to_string(), market_prices);
        }
        amp
    };

    // -----------------------------------------------------------------------------------------------

    for steamdata in cs_inv {
        if !user.update_steam { break }

        let old_length:    usize = exceldata.len();
        let index_of_name: usize = exceldata.get_index_of( &steamdata.name ).unwrap_or( old_length ); 
        let row:           usize = index_of_name + excel.row_start_write_in_table as usize;
        
        // Tanken bak dette: Hvis 
        exceldata.entry( steamdata.name.clone() )
            .and_modify(|data| 
                if let Some(col_quantity) = &excel.col_quantity {
                    let cell_quantity: String = format!("{}{}", col_quantity, row);
                    
                    if let Some(steam_quantity) = steamdata.quantity {
                        
                        if let Some(data_quantity) = data.quantity {
                            
                            if data_quantity < steam_quantity as f64 {
                                data.quantity = Some(steam_quantity as f64);
                                sheet.get_cell_value_mut( cell_quantity.as_ref() ).set_value_number(steam_quantity);

                                println!("UPDATED {} QUANTITY TO {:?} | ROW {}\n", &steamdata.name, &data.quantity, &row);
                                println!("LENGTH OF SHEET_URLS_QUANTITIES: {}", &old_length);
                            }
                        }
                    }
                } 
            )
            // DETTE ER ALLTID NYTT, SÅ ER RESPONSIBLE Å HENTE DATA FRA FLOAT API'EN.
            .or_insert( {
                let extra_itemdata: Value = {
                    if let Some(inspect) = &steamdata.inspect_link { csgotrader::get_iteminfo(inspect).await? } 
                    else { Value::Null }
                };

                let doppler: Option<Doppler> = {
                    if !extra_itemdata.is_null() {
                        //println!("EXTRA_ITEMDATA IS NOT EMPTY WOHOO");
                        if steamdata.name.to_lowercase().contains(" doppler ") { 
                            extra_itemdata.get("phase")
                                .and_then( |p| p.as_str() )
                                .and_then( |p| match Doppler::from_str(p) {
                                    Ok(s) => Some(s), 
                                    Err(_) => None
                                })
                        } else { None }
                    } else { None }
                };

                let quantity: Option<f64> = if let Some(quant) = &steamdata.quantity { Some(*quant as f64) } else { None }; // Just to change from Option<u16> to Option<f64>
                let phase: Option<String> = if let Some(dplr) = &doppler { Some(dplr.to_string()) } else { None };          // Just to change from Option<Doppler> to Option<String>
                let price: f64 = {
                    if !user.update_prices { 0.0; }

                    let mut prices: HashMap<String, f64> = HashMap::new();

                    for market in &markets_to_check {
                        if let Some(market_prices) = all_market_prices.get( market.as_str() ) {
                            if let Some(price) = price_csgotrader::get_price(&steamdata.name, &market_prices, &PriceType::StartingAt, &doppler) {
                                prices.insert(market.to_string(), price * rate);
                            }    
                        }
                    }

                    // IMPLEMENT WAY TO FETCH CORRECT PRICE GIVEN THE USERS CHOSEN PRICINGMODE
                    0.0
                };
                // ALSO REMEMBER TO ACTUALLY WRITE TO THE SPREADSHEET

                tokio::time::sleep( Duration::from_millis(user.pause_time_ms) ).await;

                ExcelData { price, quantity, phase, inspect_link: steamdata.inspect_link } 
            } );
        
         
    }

    // wowzers
    println!("EXCELDATA: \n\n{:#?}\n", exceldata);
    println!("ITEMDATA: \n\n{:#?}\n", itemdata);
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
        let rates: HashMap<String, f64> = csgotrader::get_exchange_rates().await?;
        Ok( *rates.get( currency.as_str() ).unwrap_or( &1.0 ) )
    } else { 
        Ok(1.0) 
    }
}

fn is_sheetinfo_valid(sheet: &SheetInfo) -> Result<(), String> {
    if sheet.col_inspect_link.is_none() { 
        if sheet.col_quantity.is_none() { 
            return Err( String::from( "Quantity can't be empty when no inspect link column is given." ) )
        }
    }

    if sheet.path_to_sheet.is_some() { 
        if sheet.sheet_name.is_none() {
            return Err( String::from( "Sheet name can't be nothing if path to sheet is given." ) )
        }
    }

    if sheet.col_inspect_link.is_none() {
        if sheet.col_float.is_some()   { return Err( String::from( "Column for float given but no column for inspect link."   ) ) }
        if sheet.col_phase.is_some()   { return Err( String::from( "Column for phase given but no column for inspect link."   ) ) }
        if sheet.col_pattern.is_some() { return Err( String::from( "Column for pattern given but no column for inspect link." ) ) }
    }

    Ok(())
}