use std::{path::PathBuf, sync::LazyLock};

use crate::models::{
    price::{Currencies, PricingMode, PricingProvider}, user_sheet::{SheetInfo, UserInfo}, web::{ItemInfoProvider, Sites}
};

pub static SHEET: LazyLock<SheetInfo> = LazyLock::new(|| {
    // SheetInfo { 
        // path_to_sheet: Some( PathBuf::from("C:\\Users\\Mikae\\OneDrive\\Skrivebord\\workbook\\new\\swag_test_1.xlsx") ),
        // row_stop_write_in_table: None,
        // row_start_write_in_table: 2,
        // sheet_name: Some( String::from("Sheet1") ),
        // col_market_name: String::from("A"),
        // col_gun_sticker_case: Some( String::from("B") ),
        // col_skin_name: Some( String::from("C") ),
        // col_wear: Some( String::from("D") ),
        // col_float: Some( String::from("E") ),
        // col_pattern: Some( String::from("F") ),
        // col_phase: Some( String::from("G") ),
        // col_quantity: Some( String::from("I") ),      // I
        // col_already_sold: Some( String::from("M") ),  // M
        // col_price: String::from("Q"),                 // Q
        // col_market: Some( String::from("R") ),        // R
        // col_inspect_link: Some( String::from("W") ),  // W
        // col_csgoskins_link: Some( String::from("X") ),// X
        // rowcol_date: Some( String::from("$Y$2") ),    // y
        // rowcol_usd_to_x: Some( String::from("$Y$3") ),
    // }

    SheetInfo { 
        path_to_sheet: Some( PathBuf::from("C:\\Users\\Mikae\\OneDrive\\Skrivebord\\workbook\\new\\swag_test_1.xlsx") ),
        row_stop_write_in_table: None,
        row_start_write_in_table: 2,
        sheet_name: Some( String::from("Sheet1") ),
        col_market_name: String::from("A"),
        col_gun_sticker_case: Some( String::from("B") ),
        col_skin_name: Some( String::from("C") ),
        col_wear: Some( String::from("D") ),
        col_float: None,
        col_pattern: None,
        col_phase: Some( String::from("H") ),
        col_quantity: Some( String::from("I") ),      // I
        col_already_sold: Some( String::from("M") ),  // M
        col_price: String::from("Q"),                 // Q
        col_market: Some( String::from("R") ),        // R
        col_inspect_link: Some( String::from("W") ),  // W
        col_asset_id: Some( String::from("X") ),            // X
        col_csgoskins_link: Some( String::from("Y") ),// Y
        rowcol_date: Some( String::from("$Z$2") ),    // Z
        rowcol_usd_to_x: Some( String::from("$Z$3") ),
    }
}); 

pub static USER: LazyLock<UserInfo> = LazyLock::new(|| {
    UserInfo { 
        prefer_markets: Some( vec![Sites::YOUPIN, Sites::CSFLOAT, Sites::BUFF163] ),
        // prefer_markets: None,
        pricing_mode: PricingMode::Hierarchical,
        pricing_provider: PricingProvider::Csgotrader,
        iteminfo_provider: Some( ItemInfoProvider::Csfloat ),
        usd_to_x: Some( Currencies::USD ),
        ignore_market_names: None,
        pause_time_ms: 2500,
        steamid: 76561198389123475, // Min
        // steamid: 76561198043837202,   // High-end inventory
        // steamid: 76561198060504649,   // Hjalmar sin inv
        // steamid: 76561198858570641,   // rua (rich chinese)
        steamloginsecure: None,
        percent_threshold: 5,
        group_simular_items: true,
        sum_quantity_prices: true,
        fetch_prices: true,
        fetch_steam: true 
    }
});
