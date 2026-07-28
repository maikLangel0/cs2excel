use crate::{
    models::{
        user_sheet::{UserInfo, SheetInfo},
        price::{Currencies, PricingMode},
        web::{ItemInfoProvider, Sites}
    },
    dprintln
};
use iced::widget::text_editor::Content;
use std::str::FromStr;

const ENGLISH_CHARS: [char; 26] = ['A','B','C','D','E','F','G','H','I','J','K','L','M','N','O','P','Q','R','S','T','U','V','W','X','Y','Z'];

pub trait IsEnglishAlphabetic {
    fn is_english_alphabetic(&self) -> bool;
}

impl IsEnglishAlphabetic for char {
    fn is_english_alphabetic(&self) -> bool {
        ENGLISH_CHARS.contains(&self.to_ascii_uppercase())
    }
}

// Result<MAYBE WARNING , ERROR >
pub fn sanitize_and_check_user_input(
    user: &mut UserInfo,
    excel: &mut SheetInfo,
    prefer_markets: &mut Content
) -> Result<Option<String>, String> {

    let mut err_str = String::new();
    let mut warn_str = String::new();

    if user.group_simular_items {
        excel.col_asset_id = None;
    } else {
        excel.col_quantity = None;
    }

    if user.usd_to_x != Currencies::None && excel.rowcol_usd_to_x.is_some() {
        user.usd_to_x = Currencies::None;
    }

    if !user.fetch_prices {
        excel.col_price = "".to_string();
        excel.col_market = None;
        excel.col_float = None;
        user.pricing_mode = PricingMode::Cheapest;
        user.percent_threshold = 0;
        user.iteminfo_provider = ItemInfoProvider::Steam;

        if excel.col_inspect_link.is_none() {
            excel.col_phase = None;
        }
    }

    if user.fetch_prices {

        if user.iteminfo_provider == ItemInfoProvider::Steam && excel.col_inspect_link.is_some() {
            warn_str.push_str("WARNING: Inspect Link Column is defined but you're using Steam as the ItemInfoProvider so you will not be able to fetch_more_iteminfo (float, doppler phase, pattern, price of doppler).\n");
        }
        else if user.iteminfo_provider == ItemInfoProvider::Steam {
            warn_str.push_str("WARNING: Pricing for doppler phases will not be accurate with Steam as ItemInfoProvider.\n");
        }

        if user.iteminfo_provider != ItemInfoProvider::Steam && excel.col_inspect_link.is_some() && excel.col_phase.is_none() {
            warn_str.push_str("WARNING: Phase of doppler knives will not be pricechecked correctly when reading over the spreadsheet in the future because column for phase is not set.\n" );
        }
    }

    // --------------------

    if excel.path_to_sheet.is_some() && excel.sheet_name.is_none() {
        err_str.push_str( "Sheet name can't be nothing if path to sheet is given.\n" );
    }

    if (excel.path_to_sheet.is_none() || excel.sheet_name.is_none()) && !user.fetch_steam {
        err_str.push_str("Path to sheet and/or sheet name is not given.\n");
    }

    if !user.fetch_steam && user.fetch_prices && excel.sheet_name.is_none() {
        err_str.push_str("Sheet name can't be None when fetching prices without fetching Steam.\n");
    }

    if user.pause_time_ms < 1000 || user.pause_time_ms > 2500 {
        err_str.push_str("Pause Time is only allowed to be in range of 1000 (1 second) - 2500 (2.5 seconds).\n");
    }

    if excel.col_quantity.is_none() && user.group_simular_items {
        err_str.push_str("Quantity column can't be None if you want to group similar items.\n");
    }

    if excel.col_asset_id.is_none() && !user.group_simular_items {
        err_str.push_str("AssetID column can't be None if you don't want to group similar items.\n");
    }

    // if excel.col_inspect_link.is_none() {
        // if excel.col_quantity.is_none() { err_str.push_str( "Column for quantity can't be empty when no inspect link column is given.\n" ); }
        // if excel.col_float.is_some()    { err_str.push_str( "Column for float given but no column for inspect link.\n"   ); }
        // if excel.col_phase.is_some()    { err_str.push_str( "Column for phase given but no column for inspect link.\n"   ); }
        // if excel.col_pattern.is_some()  { err_str.push_str( "Column for pattern given but no column for inspect link.\n" ); }
    // }

    // Checked in the update logic of the Iced application
    // if excel.rowcol_usd_to_x.is_some() && user.usd_to_x != Currencies::None {
        // err_str.push_str("rowcol_usd_to_x can't be something if usd_to_x is set as a currency.\n") )
    // }

    if user.pricing_mode == PricingMode::Hierarchical && user.percent_threshold == 0 {
        err_str.push_str("Pricing mode can't be Hierarchical if the Percent threshold is None.\n");
    }

    if excel.col_steam_name.is_empty() {
        err_str.push_str("Column for full names of the item(s) can't be empty.\n");
    }

    if !user.fetch_prices && !user.group_simular_items && excel.col_asset_id.is_none() {
        err_str.push_str("AssetID can't be None if you dont group similar items.\n");
    }


    if user.fetch_steam {
        match user.steamid.checked_ilog10() {
            Some(check) => { if check > 17 { err_str.push_str("SteamID is invalid.\n"); } }
            None => { err_str.push_str("SteamID is invalid.\n"); }
        }
    }

    if excel.row_start_write_in_table == 0 {
        err_str.push_str("Row to start writing in the spreadsheet is invalid.\n");
    }

    if excel.col_price.is_empty() && user.fetch_prices {
        err_str.push_str("Price column has to be given if you want to fetch prices.\n");
    }

    if user.ignore_already_sold && excel.col_sold.is_none() {
        err_str.push_str("Column for sold can't be empty if you want to ingore already sold.\n");
    }

    if let Some(date) = &excel.rowcol_date && !valid_cell_check(date) {
        err_str.push_str("format of cell date is not valid.\n");
    }

    if let Some(utx) = &excel.rowcol_usd_to_x && !valid_cell_check(utx) {
        err_str.push_str("format of cell containing USD to X currency is not valid.\n");
    }

    if let Some(stop) = excel.row_stop_write_in_table && excel.row_start_write_in_table < stop {
        err_str.push_str("Start write can't be less than stop write.\n");
    }

    let preferred_markets_check = prefer_markets.text()
        .split(",")
        .map(|s| s.trim().to_owned())
        .collect::<Vec<String>>();

    if user.fetch_prices
        && preferred_markets_check.len() == 1 && preferred_markets_check[0].is_empty() {

        err_str.push_str("Preferred markets can't be empty when fetching prices.\n");

        for market in &preferred_markets_check {
            if let Err(e) = Sites::from_str(market.as_str()) {
                err_str.push_str( &format!("{}.\n", e) );
            }
        }
    };

    let mut all_excel: Vec<&String> = Vec::from([&excel.col_price, &excel.col_steam_name]);
    if let Some(x) = &excel.col_asset_id { all_excel.push(x) }
    if let Some(x) = &excel.col_csgoskins_link { all_excel.push(x) }
    if let Some(x) = &excel.col_float { all_excel.push(x) }
    if let Some(x) = &excel.col_gun_sticker_case { all_excel.push(x) }
    if let Some(x) = &excel.col_inspect_link { all_excel.push(x) }
    if let Some(x) = &excel.col_market { all_excel.push(x) }
    if let Some(x) = &excel.col_pattern { all_excel.push(x) }
    if let Some(x) = &excel.col_phase { all_excel.push(x) }
    if let Some(x) = &excel.col_quantity { all_excel.push(x) }
    if let Some(x) = &excel.col_skin_name { all_excel.push(x) }
    if let Some(x) = &excel.col_sold { all_excel.push(x) }
    if let Some(x) = &excel.col_wear { all_excel.push(x) }
    all_excel.sort();

    if let Some(w) = all_excel.windows(2).find( |w| w[0] == w[1] && !w[0].is_empty() ) {
        err_str.push_str( format!("The same column is referenced two or more times: '{}'\n",w[0]).as_str() );
    }

    if !err_str.is_empty() {
        Err(err_str)
    } else if !warn_str.is_empty() {
        Ok(Some(warn_str))
    } else {
        Ok(None)
    }
}

const VALID_SIGNATURES: [&str; 4] = ["an", "$an", "$a$n", "a$n"];

fn valid_cell_check(s: &str) -> bool {
    let mut signature: Vec<char> = Vec::with_capacity( s.len() );

    for c in s.chars() {
        if c == '$' { signature.push(c); continue; }

        let letter: char = {
            if c.is_english_alphabetic() {'a'}
            else if c.is_ascii_digit() {'n'}
            else { return false }
        };

        if !signature.is_empty() && signature[signature.len() - 1] != letter { signature.push(letter) }
        else if signature.is_empty() { signature.push(letter) }
    }
    let final_signature = signature.iter().collect::<String>();

    dprintln!("Sign: {}", final_signature);

    VALID_SIGNATURES.contains(&final_signature.as_str())
}
