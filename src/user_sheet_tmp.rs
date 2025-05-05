use std::{path::PathBuf, sync::LazyLock};

use crate::models::{
    price::{Currencies, PricingMode, PricingProvider}, user_sheet::{SheetInfo, UserInfo}, web::{ItemInfoProvider, Sites}
};

pub static SHEET: LazyLock<SheetInfo> = LazyLock::new(|| {
    SheetInfo { 
        path_to_sheet: Some( PathBuf::from("C:\\Users\\Mikae\\OneDrive\\Skrivebord\\cs_invest\\CS2_invest_new_main.xlsx") ),
        row_stop_write_in_table: None,
        row_start_write_in_table: 2,
        sheet_name: Some( String::from("Sheet1") ),
        col_steam_name: String::from("A"),
        col_gun_sticker_case: Some( String::from("B") ),
        col_skin_name: Some( String::from("C") ),
        col_wear: Some( String::from("D") ),
        col_float: Some( String::from("E") ),
        col_pattern: Some( String::from("F") ),
        col_phase: Some( String::from("G") ),
        col_quantity: Some( String::from("I") ),      // I
        col_sold: Some( String::from("P") ),          // P
        col_price: String::from("J"),                 // J
        col_market: Some( String::from("K") ),        // K
        col_inspect_link: Some( String::from("T") ),  // T
        col_asset_id: None,                           // V
        col_csgoskins_link: Some( String::from("U") ),// U
        rowcol_date: Some( String::from("$W$2") ),    // W
        rowcol_usd_to_x: None,
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
        ingore_steam_names: None,
        pause_time_ms: 2500,
        steamid: 76561198389123475, // Min
        // steamid: 76561198043837202,   // High-end inventory
        // steamid: 76561198060504649,   // Hjalmar sin inv
        // steamid: 76561198858570641,   // rua (rich chinese)
        steamloginsecure: None,
        percent_threshold: 5,
        ignore_already_sold: true,
        group_simular_items: true,
        sum_quantity_prices: true,
        fetch_prices: true,
        fetch_steam: true 
    }
});
