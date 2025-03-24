use crate::{Deserialize, Serialize, PathBuf};

#[derive(Debug, Deserialize, Serialize)]
pub struct UserInfo {
    pub prefer_markets: Vec<String>, 
    pub ignore_urls: Vec<String>, 
    pub steamid: u64, 
    pub pause_time_ms: u64, 
    pub appid: u32, 
    pub steamloginsecure: Option<String>,    
    pub percent_threshold: u8, 
    pub update_prices: bool, 
    pub fetch_steam: bool
}

#[derive(Debug, Deserialize, Serialize)]
pub struct SheetInfo {
    pub path_to_sheet: PathBuf, // Full path to the spreadsheet
    pub row_start_table: u32, // Row of where the start of the table is
    pub row_start_write_in_table: u32, // Row of where to start reading and writing to the spreadsheet
    pub row_stop_write_in_table: Option<u32>, // Row of where to stop adding to the 
    pub sheet_name: String, // Name of the sheet user wants to access
    pub rowcol_usd_to_x: String, // Cell where the conversion from usd to x is located
    pub rowcol_date: Option<String>, // Cell to put what the time is at completion of the script
    pub col_url: String, // Column where the url to the site used to pricecheck is
    pub col_market: Option<String>, // Column for the market used for the price of item
    pub col_price: String, // Column for the price of item
    pub col_quantity: String, // Column for the item quantity
    pub col_gun_sticker_case: String, // Where to put name of gun/tournament year
    pub col_skin_name: String, // Where to put the name of skin/player/team
    pub col_wear: String, // Where to put float of skin/rarity of sticker 
}

#[derive(Deserialize, Serialize)]
pub struct SaveLoad {
    pub user: UserInfo,
    pub sheet: SheetInfo
}