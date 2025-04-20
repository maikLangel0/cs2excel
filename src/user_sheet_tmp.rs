use std::{path::PathBuf, sync::LazyLock};

use crate::models::{
    web::Sites, 
    price::{PricingMode, PricingProvider, Currencies}, 
    user_sheet::{UserInfo, SheetInfo}
};

pub static SHEET: LazyLock<SheetInfo> = LazyLock::new(|| {
    SheetInfo { 
        path_to_sheet: Some( PathBuf::from("C:\\Users\\Mikae\\OneDrive\\Skrivebord\\workbook\\new\\swag_test_1.xlsx") ),
        row_stop_write_in_table: None,
        row_start_write_in_table: 2,
        sheet_name: Some( String::from("Sheet1") ),
        col_market_name: String::from("A"),
        col_gun_sticker_case: Some( String::from("B") ),
        col_skin_name: Some( String::from("C") ),
        col_wear: Some( String::from("D") ),
        col_special: Some( String::from( "E" )),
        col_quantity: Some( String::from("F") ),
        col_already_sold: Some( String::from("J") ),
        col_price: String::from("M"),
        col_market: Some( String::from("N") ),
        col_inspect_link: Some( String::from("S") ),
        col_csgoskins_link: Some( String::from("T") ),
        rowcol_date: Some( String::from("$U$2") ),
        rowcol_usd_to_x: Some( String::from("$U$3") ),
    }
}); 

pub static USER: LazyLock<UserInfo> = LazyLock::new(|| {
    UserInfo { 
        prefer_markets: Some( vec![Sites::YOUPIN, Sites::CSFLOAT, Sites::BUFF163] ),
        // prefer_markets: None,
        pricing_mode: PricingMode::Hierarchical,
        pricing_provider: PricingProvider::Csgotrader,
        usd_to_x: Some( Currencies::USD ),
        ignore_market_names: None,
        pause_time_ms: 100,
        steamid: 76561198389123475,
        steamloginsecure: None,
        percent_threshold: 5,
        group_simular_items: true,
        sum_quantity_prices: false,
        update_prices: false,
        update_steam: true 
    }
});
