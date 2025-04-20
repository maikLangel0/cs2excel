use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use super::{price::{PricingMode, PricingProvider, Currencies}, web::Sites};


#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct UserInfo {
    pub prefer_markets: Option< Vec<Sites> >, 
    pub ignore_market_names: Option< Vec<String> >,  
    pub pause_time_ms: u64, 
    pub steamid: u64, 
    pub pricing_mode: PricingMode,
    pub pricing_provider: PricingProvider,
    pub usd_to_x: Option<Currencies>,
    pub steamloginsecure: Option<String>,    
    pub percent_threshold: u8, 
    pub group_simular_items: bool,
    pub sum_quantity_prices: bool,
    pub update_prices: bool, 
    pub update_steam: bool
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct SheetInfo {
    pub path_to_sheet: Option<PathBuf>, // Full path to the spreadsheet
    pub row_stop_write_in_table: Option<u32>, // Row of where to stop adding to the 
    pub row_start_write_in_table: u32, // Row of where to start reading and writing to the spreadsheet
    pub rowcol_date: Option<String>, // Cell to put what the time is at completion of the script
    pub rowcol_usd_to_x: Option<String>, // Cell to put the conversion rate from usd to x if they want to use it to calculate other stuff in the spreadsheet
    pub col_market: Option<String>, // Column for the market used for the price of item
    pub col_gun_sticker_case: Option<String>, // Where to put name of gun/tournament year
    pub col_skin_name: Option<String>, // Where to put the name of skin/player/team
    pub col_wear: Option<String>, // Where to put float of skin/rarity of sticker 
    pub sheet_name: Option<String>, // Name of the sheet user wants to access
    pub col_already_sold: Option<String>, // IF PROVIDED, ignore updating price of stuff that is already sold
    pub col_market_name: String, // Column where the full market name to the site used to pricecheck is
    pub col_price: String, // Column for the price of item
    pub col_quantity: Option<String>, // Column for the item quantity
    pub col_inspect_link: Option<String>,
    pub col_csgoskins_link: Option<String>,
    pub col_special: Option<String>, // IF YOU WANT THE CORRECT DOPPLER PRICES, SET THIS ROW
}