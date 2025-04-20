mod excel;
use excel::get_spreadsheet;

mod models;
use models::{
    item::ItemData, 
    price::Currencies, 
    web::SteamData, 
    user_sheet::{SheetInfo, UserInfo},
    excel::ExcelData
};

mod user_sheet_tmp;
use user_sheet_tmp::{SHEET, USER};

mod pricing;
use pricing::prices;

mod browser;
use browser::{
    cookies::FirefoxDb, 
    csgotrader, 
    steamcommunity::SteamInventory
};

use std::{collections::HashMap, error::Error};
use urlencoding;
use umya_spreadsheet::{Spreadsheet, Worksheet};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // Path to my main invest: C:\Users\Mikae\OneDrive\Skrivebord\workbook
    let sheet: SheetInfo = SHEET.clone();
    let user: UserInfo = USER.clone();

    is_sheetinfo_valid(&sheet)?;

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

    // -------------------------------------------------------------------------------------------

    // BIG BRAIN; READ THE EXCEL SPREADSHEET FIRST TO GET ALL THE INFO AND THEN GET PRICES WOWOWO
    
    // Getting the Worksheet from either existing book or new book
    let mut book: Spreadsheet = get_spreadsheet(&sheet.path_to_sheet)?;
    let sheet: &mut Worksheet = {
        if let Some(sn) = &sheet.sheet_name { 
            if let Some(buk) = book.get_sheet_by_name_mut(sn) { buk } 
            else {
                println!("WARNING: Automatically fetched first sheet in spreadsheet because {} was not found.", sn);
                book.get_sheet_mut(&0).ok_or_else(|| format!(
                    "Failed to get the first sheet in the spreadsheet with path: \n{:?}", sheet.path_to_sheet)
                )? 
            }  
        } else { book.get_sheet_mut(&0).ok_or("Failed to get first sheet provided by new_file creation.")? }
    };

    let rate: f64 = get_exchange_rate(&user.usd_to_x).await?;
    let steamcookie: Option<String> = get_steamloginsecure(&user.steamloginsecure);

    let sm_inv:   SteamInventory = SteamInventory::init(user.steamid, 730, steamcookie).await?;
    let cs_inv:   Vec<SteamData> = sm_inv.get_steam_items(user.steamid, user.group_simular_items, true)?;
    let itemdata: Vec<ItemData>  = prices::get_prices(&user, &cs_inv, rate).await?;

    // wowzers
    println!("{:#?}", itemdata);
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

    Ok(())
}