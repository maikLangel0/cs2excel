use std::{path::PathBuf, sync::LazyLock};

use crate::models::{
    price::{Currencies, PricingMode, PricingProvider}, user_sheet::{SheetInfo, UserInfo}, web::{Sites, ItemInfoProvider}
};

pub static SHEET: LazyLock<SheetInfo> = LazyLock::new(|| {
    SheetInfo { 
        path_to_sheet:              Some( PathBuf::from("C:\\Users\\Mikae\\Desktop\\invest\\cs\\CS2_invest_new_main.xlsx") ),
        sheet_name:                 Some( String::from("Sheet1") ),
        
        row_start_write_in_table:   2,
        row_stop_write_in_table:    None,
        
        col_steam_name:                   String::from("A"),
        col_price:                        String::from("J"),

        col_gun_sticker_case:       Some( String::from("B") ),
        col_skin_name:              Some( String::from("C") ),
        col_wear:                   Some( String::from("D") ),
        col_float:                  Some( String::from("E") ),
        col_pattern:                Some( String::from("F") ),
        col_phase:                  Some( String::from("G") ),
        col_quantity:               Some( String::from("I") ),
        col_market:                 Some( String::from("K") ),
        col_sold:                   Some( String::from("P") ),
        col_inspect_link:           Some( String::from("U") ),
        col_csgoskins_link:         Some( String::from("V") ),
        col_asset_id:               None,                     
        rowcol_date:                Some( String::from("$X$2") ), 
        rowcol_usd_to_x:            None,
    }
}); 

pub static USER: LazyLock<UserInfo> = LazyLock::new(|| {
    UserInfo { 
        prefer_markets:             Some( vec![Sites::YOUPIN, Sites::CSFLOAT, Sites::BUFF163] ),
        steamloginsecure:           None,

        // iteminfo_provider:          Some( ItemInfoProvider::Csfloat ), // "Bots are temporarily not allowed on CSGOFloat Inspect API due to new rate limits imposed by Valve"
        iteminfo_provider:          None,
        usd_to_x:                   Some( Currencies::CNY ),

        pricing_mode:               PricingMode::Hierarchical,
        pricing_provider:           PricingProvider::Csgotrader,
        
        pause_time_ms:              2500,
        steamid:                    76561198389123475, // Angel0 - min inv
        percent_threshold:          5,

        ignore_already_sold:        true,
        group_simular_items:        true,
        sum_quantity_prices:        false,
        fetch_prices:               true,
        fetch_steam:                true,

        ingore_steam_names: Some( 
            Vec::from([
                "AK-47 | Blue Laminate (Field-Tested)",
                "M4A1-S | Guardian (Minimal Wear)",
                "AWP | Sun in Leo (Factory New)",
            ]).iter().map(|e| e.to_string()).collect::<Vec<String>>() 
        ),

        // steamid:                     76561198043837202,   // High-end inventory
        // steamid:                     76561198060504649,   // Hjalmar sin inv
        // steamid:                     76561198858570641,   // rua (rich chinese)
    }
});
