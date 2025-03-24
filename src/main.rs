// Prevent console window in addition to Slint window in Windows release builds when, e.g., starting the app via file manager. Ignored on other platforms.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
mod excel;
mod main_structs;
mod firefox;
mod websites;
mod helpers;
// ------------------------------------

use {
    async_compat::Compat, chrono::Local, excel::main_excel::excelling, firefox::cookies::FirefoxDb, helpers::{csgoskins_url::create_csgoskins_urls, steam_item_market_name::cs_get_metadata_from_market_name}, image::ImageReader, indexmap::IndexMap, rand, regex::Regex, reqwest::header::{COOKIE, USER_AGENT}, reqwest_cookie_store::CookieStoreMutex, rfd::FileDialog, rodio::{Decoder, OutputStream, Sink}, rusqlite::{self, Connection}, scraper::{Html, Selector}, serde::{Deserialize, Serialize}, serde_json::{from_reader, from_value, Value}, slint::{Image, Rgba8Pixel, SharedPixelBuffer, SharedString}, std::{
        collections::{HashMap, HashSet}, error::Error, fs::File, io::{BufReader, Cursor}, path::PathBuf, sync::{Arc, LazyLock}, thread
    }, main_structs::{SaveLoad, SheetInfo, UserInfo}, tokio::time::{sleep, Duration}, umya_spreadsheet::{reader, writer, Spreadsheet, Worksheet}, websites::{csgoskins::Csgoskins, steam::SteamInventory}, whoami
};

// Sick macro by gipiti | only prints to console when build flag is not set
#[macro_export]
macro_rules! dprintln {
    ($( $arg:tt )*) => {
        #[cfg(debug_assertions)]
        println!( $( $arg )* )
    };
}

slint::include_modules!();

static MARKETS: LazyLock<HashSet<&str>> = LazyLock::new( || {
    HashSet::from(["skinport", "gamerpay", "buff163", "skinout", 
        "skinswap", "dmarket", "buff market", "csfloat", "shadowpay", "waxpeer", 
        "lis-skins", "haloskins", "avan.market", "cs.money", "market.csgo", "tradeit.gg", 
        "skinbaron", "steam", "mannco.store", "cs.deals", "skinflow", "skinbid"
    ])
});

static ADDITIONAL_INFO: LazyLock<&str> = LazyLock::new( || {
    "IMPORTANT INFO (scroll down): \
    \n\nEXCEL FILE NEEDS TO CLOSED THE INSTANCE YOU START THE PROGRAM AND THE INSTANCE THE PROGRAM ENDS! HAVING THE FILE OPEN WHEN CLICKING Run Program WILL RESULT IN A CRASH. IF PROGRAM IS OPEN AT THE END OF ITERATION, WRITING TO THE EXCEL FILE WILL NOT BE SUCCESSFUL.
    \n\nMAKE A BACKUP OF YOUR EXCEL FILE! \
    \nMAKE SURE THE ROWS OF THE TABLE HAS NO GAPS IN IT. IF IT DOES THE PROGRAM WILL NOT RECOGNIZE THE WHOLE TABLE AND ADD INFORMATION IN RANDOM, NOT INTENDED PLACES!
    \n[?] means that thing is OPTIONAL \
    \n\nSHEET TO UPDATE/EDIT is the name of the sheet inside your excel file that you want to edit.\
    \n\nPERCENT THRESHOLD is the difference between your first Preferred Market and the Second. \
    For example, preferred markets are buff163 and csfloat. Buff163 price is 1$, csfloat is 0.9$, \
    and with a % threshold of 5 would make the program choose csfloat because the difference between \
    buff163 price and csfloat is greater than 5% \n(1$ > 0.9$ * 1.05). \nIf percent theshold is 0, it \
    picks the cheapest out of your preferred markets.\
    \n\nTIME BETWEEN UPDATES is for how big the gap between individual price updates are in the spreadsheet. \
    \n\nSTEAMLOGINSECURE is the value of the cookie SteamLoginSecure and it's necessary to fetch items from \
    YOUR inventory that are under trade hold. If you have Firefox on your pc and are logged in to \
    steamcommunity.com, the program will fetch your SteamLoginSecure automatically. \
    \n\nPRICE CHECKING URLS TO IGNORE are for urls that you want to ignore during pricechecking in the spreadsheet. \
    \n\nCELL USD TO X is the coordinates for the cell that contains a conversion rate between usd and x currency. \
    Set to 1 if you dont want to convert, or set it as the conversion rate from usd to x currency manually. \
    \n\nROW START OF TABLE defines the top row of the table that you want to either make or update. \
    \n\nROW START OF WRITING defines which row inside the table you want to start at. \
    This can be useful if you just want to target a range inside your table, not the whole table itself. \
    \n\n ROW END OF WRITING defines which row is the end of your range between start of writing and end of writing. \
    This can be empty, which will make the program go through until the end of the table. \
    \n\nCELL DATE/TIME defines if you want to save the time of last pricecheck into your excel spreadsheet.\
    \n\nPREFERRED MARKETS are the markets that you want to use for the prices of your skins/items. \
    This is easier to explain with an example: \n'buff163, csfloat, gamerpay, buff market' \nThis will make the \
    program favor these markets, and given your set PERCENT THRESHOLD, it may favor buff163 over csfloat, \
    and csfloat over gamerpay, and gamerpay over buff market. If none of these markets are available given \
    the current skin/item, it will choose the cheapest market out of the other markets. \
    \n\nALL ALLOWED MARKETS:\
    \nskinport, gamerpay, buff163, skinout, skinswap, dmarket, buff market, csfloat, shadowpay, waxpeer, lis-skins, \
    haloskins, avan.market, cs.money, market.csgo, tradeit.gg, skinbaron, steam, mannco.store, cs.deals, skinflow, skinbid
    "
});

struct GlorpRadio {
    _stream: OutputStream,  // Needs to be kept alive as long as sink, thats y
    sink: Arc<Sink>,
    audio_data: &'static [u8],
}
impl GlorpRadio {
    fn new() -> Result<Self, Box<dyn Error>> {
        let (stream, stream_handle) = OutputStream::try_default()?;
        let sink = Arc::new( Sink::try_new( &stream_handle )? );
        
        Ok(Self {
            _stream: stream,
            sink,
            audio_data: include_bytes!("../ui/audio/elevator.mp3")
        })
    }

    fn play(&self) {
        let sink_clone = self.sink.clone();
        let audio_data = self.audio_data.iter().as_slice();
        
        thread::spawn(move || {
            let cursor = Cursor::new(audio_data);
            if let Ok(source) = Decoder::new(cursor) {
                sink_clone.append(source);
                sink_clone.set_volume(0.0);
                sink_clone.sleep_until_end();
            }

            // Loop here so set_volume doesnt get reset to 0.1 on new iteration
            loop {
                let cursor = Cursor::new(audio_data);
                if let Ok(source) = Decoder::new(cursor) {
                    sink_clone.append(source);
                    sink_clone.sleep_until_end();
                }
            }
        });
    }
}

// chat gipiti madness
fn load_image_from_bytes(data: &[u8]) -> Result<Image, Box<dyn Error>> {
    // Decode the PNG using the `image` crate
    let img = ImageReader::new( Cursor::new(data) )
        .with_guessed_format()?
        .decode()?;

    // Convert to RGBA8 format
    let rgba_img = img.to_rgba8();

    // Create a SharedPixelBuffer from the image data
    let buffer = SharedPixelBuffer::<Rgba8Pixel>::clone_from_slice(
        rgba_img.as_raw(),
        rgba_img.width(),
        rgba_img.height(),
    );

    // Create Slint Image from the buffer
    Ok(Image::from_rgba8(buffer))
}

// Helper function to convert from UserInfo -> UserInfoUI & SheetInfo -> SheetInfoUI
fn set_user_info_sheet_info(user: &UserInfo, params: &SheetInfo, app: &slint::Weak<AppWindow>) {
    let app = app.upgrade().unwrap();
    
    app.set_user_info( 
        UserInfoUI { 
            appid: SharedString::from( user.appid.to_string() ), 
            fetch_steam: user.fetch_steam, 
            steamid: SharedString::from( user.steamid.to_string() ), 
            update_prices: user.update_prices,
            pause_time_ms: user.pause_time_ms as f32,
            percent_threshold: SharedString::from( user.percent_threshold.to_string() ), 
            
            // Vec<String> -> SharedString
            ignore_urls: SharedString::from( 
                user.ignore_urls.join(", ").trim()
             ), 
            // Vec<String> -> SharedString
            prefer_markets: SharedString::from(
                user.prefer_markets.join(", ").trim()
            ),
            // None is equal to ""
            steamloginsecure: 
            if let Some(sls) = &user.steamloginsecure {
                SharedString::from(sls)
            } else {
                SharedString::from("")
            }
        }
    );

    let path = params.path_to_sheet.to_str().unwrap_or("corrupted\\corrupted");
    let path_split = path.split("\\").collect::<Vec<&str>>();
    let excel_name = &path_split[path_split.len() - 1].to_string();
    
    app.set_path_to_sheet( path.into() );
    app.set_btn_name_of_sheet( excel_name.into() );
    app.set_pause_time_ms( user.pause_time_ms as f32 );

    app.set_sheet_info(
        SheetInfoUI { 
            col_gun_sticker_case: SharedString::from( &params.col_gun_sticker_case ),
            col_price: SharedString::from( &params.col_price ),
            col_quantity: SharedString::from( &params.col_quantity ),
            col_skin_name: SharedString::from( &params.col_skin_name ),
            col_url: SharedString::from( &params.col_url ),
            col_wear: SharedString::from( &params.col_wear ),
            path_to_sheet: SharedString::from( path ),
            row_start_table: SharedString::from( params.row_start_table.to_string() ),
            row_start_write_in_table: SharedString::from( params.row_start_write_in_table.to_string() ),
            rowcol_usd_to_x: SharedString::from( &params.rowcol_usd_to_x ),
            sheet_name: SharedString::from( &params.sheet_name ),

            // None is equal to ""
            rowcol_date: if let Some(date) = &params.rowcol_date {
                SharedString::from( date.to_string() )
            } else {
                SharedString::from("")
            },
            // None is equal to ""
            row_stop_write_in_table: if let Some(stop) = &params.row_stop_write_in_table {
                SharedString::from( stop.to_string() )
            } else {
                SharedString::from("")
            },
            // None is equal to ""
            col_market: if let Some(market) = &params.col_market {
                SharedString::from(market)
            } else {
                SharedString::from("")
            }
        }
    )
}

// Helper function to sanitize and convert from UserInfoUI -> UserInfo & SheetInfoUI -> SheetInfo
fn get_user_info_sheet_info(user_ui: &UserInfoUI, sheet_ui: &SheetInfoUI) -> Result<(UserInfo, SheetInfo), String> {    
    let mut err_text = String::new();

    if sheet_ui.sheet_name.is_empty() { err_text.push_str("Sheet to update/edit cannot be empty.\n") };

    let percent_threshold = if let Ok(nummer) = user_ui.percent_threshold.trim().parse::<u8>() {
        if nummer > 0 && nummer <= 100 {
            nummer
        } else {
            err_text.push_str("Percent threshold is invalid, needs to be a number between 0 -> 100.\n");
            0
        }
    } else {
        err_text.push_str("Percent threshold is invalid.\n");
        0
    };

    // Error checking for user_ui
    let appid = if let Ok(nummer) = user_ui.appid.trim().parse::<u32>() {
        nummer
    } else {
        err_text.push_str("AppID is invalid, needs to be a positive number (730 for CS2).\n");
        0
    };

    let steamid = if let Ok(nummer) = user_ui.steamid.trim().parse::<u64>() {
        nummer
    } else {
        err_text.push_str("SteamID is invalid, needs to be a positive number.\n");
        0
    };

    let prefer_markets: Vec<String> = {
        
        // Convert from SharedString to Vec<String>
        let tmp = user_ui.prefer_markets.split(", ")
            .map( |s| s.trim().to_lowercase() )
            .map( |s| { 
                if !MARKETS.contains( s.as_str() ) { 
                    err_text.push_str( &format!("Market named '{}' not allowed/available.\n", s) )
                }
                s
            })
            .collect::<Vec<String>>();
        tmp
    };
    
    // Error checking for sheet_ui
    if !sheet_ui.path_to_sheet.trim().ends_with(".xlsx") {
        err_text.push_str("Path to sheet is an invalid system path.\n");
    }

    let row_start_table = if let Ok(nummer) = sheet_ui.row_start_table.trim().parse::<u32>() {
        nummer
    } else {
        err_text.push_str("Row start of table is invalid, needs to be a positive number.\n");
        0
    };

    let row_start_write_in_table = if let Ok(nummer) = sheet_ui.row_start_write_in_table.trim().parse::<u32>() {
        nummer
    } else {
        err_text.push_str("Row start of writing is invalid, needs to be a positive number.\n");
        0
    };

    let row_stop_write_in_table = if sheet_ui.row_stop_write_in_table.is_empty() { None } else { 
        Some( 
            // Convert from a SharedString to Option<u32> where if equals "" -> None
            if let Ok(nummer) = sheet_ui.row_stop_write_in_table.trim().parse::<u32>() {
                nummer
            } else {
                err_text.push_str("Row stop of writing is invalid, needs to be a positive number or an empty field.\n");
                1
            }
        ) 
    };

    let rowcol_usd_to_x = { 
        let utx = sheet_ui.rowcol_usd_to_x.trim().to_string();
        
        if ( !utx.chars().rev().take(1).any( |c| c.is_numeric() ) && utx.len() != 1          ) ||
           ( !utx.chars().take(1).any( |c| c.is_alphabetic() || c == '$' ) && utx.len() != 1 ) {
            err_text.push_str("Cell USD to x conversion is invalid, needs to be valid Cell coordinates, or 1.\n");
            String::from("1")
        } else {
            utx
        }
    };

    let rowcol_date: Option<String> = { 
        let date = sheet_ui.rowcol_date.trim().to_string();
        
        if !date.is_empty() {
            if ( !date.chars().rev().take(1).any( |c| c.is_numeric() ) && date.len() != 1          ) ||
               ( !date.chars().take(1).any( |c| c.is_alphabetic() || c == '$' ) && date.len() != 1 ) {
                err_text.push_str("Cell date is invalid, needs to be valid Cell coordinates, or 1.\n");
                Some(String::from("1"))
            } 
            else { Some(date) }
        } else { None }
    };

    if !sheet_ui.col_gun_sticker_case.chars().any( |c| c.is_alphabetic() ) {
        err_text.push_str("Column gun/sticker/case is invalid, needs to be only A-Z letter(s).\n");
    }
    if !sheet_ui.col_skin_name.chars().any( |c| c.is_alphabetic() ) {
        err_text.push_str("Column skin/name is invalid, needs to be only A-Z letter(s).\n");
    }
    if !sheet_ui.col_wear.chars().any( |c| c.is_alphabetic() ) {
        err_text.push_str("Column name of wear is invalid, needs to be only A-Z letter(s).\n");
    }
    if !sheet_ui.col_quantity.chars().any( |c| c.is_alphabetic() ) {
        err_text.push_str("Column quantity is invalid, needs to be only A-Z letter(s).\n");
    }
    if !sheet_ui.col_price.chars().any( |c| c.is_alphabetic() ) {
        err_text.push_str("Column price is invalid, needs to be only A-Z letter(s).\n");
    }
    if !sheet_ui.col_market.chars().any( |c| c.is_alphabetic() ) && !sheet_ui.col_market.is_empty() {
        err_text.push_str("Column market is invalid, needs to be only A-Z letter(s) or empty.\n");
    }

    if !err_text.is_empty() {
        return Err( err_text )
    }

    let user = UserInfo {
        appid,
        steamid,
        percent_threshold,
        pause_time_ms: user_ui.pause_time_ms as u64,
        update_prices: user_ui.update_prices,
        fetch_steam: user_ui.fetch_steam,
        prefer_markets,
        steamloginsecure: if user_ui.steamloginsecure.is_empty() { None } else { 
            Some( user_ui.steamloginsecure.trim().to_string() ) 
        },
        ignore_urls: user_ui.ignore_urls.split(", ")
            .map( |s| s.trim().to_string() )
            .collect::<Vec<String>>(),
    };

    let sheet = SheetInfo {
        path_to_sheet: PathBuf::from( sheet_ui.path_to_sheet.trim() ),
        row_start_table,
        row_start_write_in_table,
        sheet_name: sheet_ui.sheet_name.trim().to_string(),
        rowcol_usd_to_x,
        col_url: sheet_ui.col_url.trim().to_string(),
        col_gun_sticker_case: sheet_ui.col_gun_sticker_case.trim().to_string(),
        col_skin_name: sheet_ui.col_skin_name.trim().to_string(),
        col_wear: sheet_ui.col_wear.trim().to_string(),
        col_price: sheet_ui.col_price.trim().to_string(),
        col_quantity: sheet_ui.col_quantity.trim().to_string(),

        rowcol_date,
        
        col_market: if sheet_ui.col_market.is_empty() { None } else {
            Some( sheet_ui.col_market.trim().to_string() )
        },
        row_stop_write_in_table
    };
    
    Ok((user, sheet))
}

fn gui() -> Result<(), Box<dyn Error>> {
    let app = AppWindow::new()?;
    let app_weak = app.as_weak();

    app.set_gui_output_text( ADDITIONAL_INFO.to_string().into() );

    // Sets the glorp loop
    let audio = GlorpRadio::new()?;
    audio.play();

    // Sets background image
    let bg_image = load_image_from_bytes( include_bytes!("../ui/icons/GLORP.png") )?;
    app.set_background_image(bg_image);

    // Uses Arc<Sink> to update song audio for the app
    // - callback "update_volume(float)"
    // - in-out property "volume"
    let volume_sink = audio.sink.clone();
    app.on_update_volume(move |new_vol| {
        volume_sink.set_volume(new_vol);
    });

    // Tries to convert pause_time_ms from SharedString to f32
    // - callback "try-time-conversion(string) -> float"
    // - in-out property pause-time-ms-text.text
    app.on_try_time_conversion(move |millisecond| {
        if let Ok(ms) = millisecond.to_string().parse() {
            if !(1000.0 ..= 5000.0).contains(&ms) { 2500.0 } // "default" value
            else { ms }
        } 
        else { 2500.0 }
    });

    // Main function that calls the backend given user (UserInfoUI)
    // and sheet (SheetInfoUI)
    // - callback start-excel-write(UserInfoUI, SheetInfoUI)
    app.on_start_excel_write( {
        let app_weak = app_weak.clone();

        move |user_ui, sheet_ui| {
            match get_user_info_sheet_info(&user_ui, &sheet_ui) {
                Ok((user, sheet)) => {
                    dprintln!("USER: {:#?}\n\nSHEET: {:#?}", user, sheet);
                    
                    let app = app_weak.upgrade().unwrap();
                    app.set_btn_run_enabled(false);

                    let app_weak = app_weak.clone();
                    let _ = slint::spawn_local( 
                        
                        Compat::new( 
                            async move {
                                
                                match excelling(&user, &sheet, &app_weak).await {
                                    Ok(_) => { dprintln!("Finished successfully!") },
                                    Err(e) => { 
                                        app_weak.upgrade()
                                            .unwrap()
                                            .set_gui_output_text( 
                                                format!("Error! : {}", e).into() 
                                            );
                                        eprintln!("Error! : {}", e);
                                    }
                                }
                                app.set_progress( 0.0 );
                                app.set_btn_run_enabled(true);

                            }
                        )

                    ); 
                },
                Err(err_text) => {
                    let app = app_weak.upgrade().unwrap();

                    app.set_gui_output_text( "".into() );
                    app.set_gui_output_text( err_text.into() );
                }
            }
        }
    });

    // Selection button for the excel file
    // - callback "select-excel-file -> string"
    // - in-out property "selected-file"
    app.on_select_excel_file( {
        let app_weak = app_weak.clone();

        move || {
            // Uses fileDialog crate to pick file and convert it to string
            let file_path = FileDialog::new()
                .add_filter("", &["xlsx"])
                .set_title("Find .xlsx file")
                .pick_file()
                .and_then( |p| Some( p.to_string_lossy().to_string() ) );

            if let Some(path) = file_path {
                // app_weak.upgrade().unwrap()... is the same as app... , it's
                // used since app cannot be used inside the closure as it is the
                // one that's instanciating the closure. That's why the weak 
                // pointer is used instead.
                let app_from_weak = &app_weak.upgrade().unwrap();

                let path_split = &path.split("\\").collect::<Vec<&str>>();
                let excel_name = &path_split[path_split.len() - 1].to_string();

                // set_path_to_sheet updates the in-out property "selected-file".
                app_from_weak.set_path_to_sheet( path.as_str().into() );
                app_from_weak.set_btn_name_of_sheet( excel_name.into() );
            }
        }
    });

    // Loading JSON file to the GUI
    // - callback "load"
    app.on_load( {
        let app_weak = app_weak.clone();
        
        move || {
            let app = app_weak.upgrade().unwrap();

            // Uses fileDialog crate to pick file and convert it to string
            let file_path = FileDialog::new()
                .add_filter("JSON Files", &["json"])
                .set_title("Find and load .json save file")
                .pick_file();

            if let Some(path) = file_path {
                if let Ok(file) = File::open(path) {
                    let read = BufReader::new(file);

                    if let Ok(load) = from_reader::<_, SaveLoad>(read) {
                        set_user_info_sheet_info(&load.user, &load.sheet, &app_weak);
                    } 
                    else { app.set_gui_output_text( "Error! : Failed to properly parse JSON in file.".into()) }
                } else { app.set_gui_output_text( "Error! : Couldn't read file path.".into() ) }
            }
        }
    });

    // Saving information given by user in GUI to a JSON file
    // - callback "save(UserInfoUI, SheetInfoUI)"
    app.on_save( {
        let app = app_weak.upgrade().unwrap();

        move |user_ui, sheet_ui| {
            match get_user_info_sheet_info(&user_ui, &sheet_ui) {
                Ok((user, sheet)) => {
                    let data = SaveLoad {user, sheet};

                    // Uses fileDialog crate to pick file and convert it to string
                    let file_path = FileDialog::new()
                        .add_filter("JSON Files", &["json"])
                        .set_title("Save .json file")
                        .save_file();

                    if let Some(path) = file_path {
                        if let Ok(file) = File::create(path) {
                            if serde_json::to_writer_pretty(&file, &data).is_ok() {
                                app.set_gui_output_text( "Saved successfully!".into() );
                            }
                            else { app.set_gui_output_text( "Error! : Save Failed.".into() ) }
                        } else { app.set_gui_output_text( "Error! : Failed to create new file.".into() ) }
                    }
                },
                Err(err_text) => {
                    app.set_gui_output_text( err_text.into() )
                }
            } 
        }
    });

    // For future custom close request
    app.window().on_close_requested(|| {
        slint::CloseRequestResponse::HideWindow
    });

    app.run()?;

    Ok(())
}

//#[tokio::main]
fn main() -> Result<(), Box<dyn Error>> {

    // let user = UserInfo{
        // update_prices: true,
        // fetch_steam: true,
        // appid: 730,
        // steamid: 76561198389123475,
        // steamloginsecure: None,
        // percent_threshold: 5, // If 0, take cheapest out of prefer_markets
        // pause_time_ms: 1800,
        // prefer_markets: vec!["csfloat", "buff163", "gamerpay", "buff market"]
            // .iter()
            // .map(|market| market.to_string())
            // .collect(),
        // ignore_urls: vec!["https://csgoskins.gg/items/ak-47-blue-laminate/field-tested", "https://csgoskins.gg/items/m4a1-s-guardian/minimal-wear", "https://csgoskins.gg/items/awp-sun-in-leo/factory-new"]
            // .iter()
            // .map(|url| url.to_string())
            // .collect(),
    // };
// 
    // let excel_params = SheetInfo {
        // path_to_sheet: PathBuf::from("C:\\Users\\Mikae\\OneDrive\\Skrivebord\\rusty business\\http_stuff\\src\\workbook\\CS2_invest_sheet_alt_main.xlsx"),
        // sheet_name: "Main".to_string(),
        // row_start_table: 2,
        // row_start_write_in_table: 1,
        // row_stop_write_in_table: None,
        // rowcol_usd_to_x: "T8".to_string(),
        // rowcol_date: Some("$S$2".to_string()),
        // col_url: "Q".to_string(),
        // col_market: Some("".to_string()),
        // col_price: "M".to_string(),
        // col_quantity: "F".to_string(),
        // col_gun_sticker_case: "A".to_string(),
        // col_skin_name: "B".to_string(),
        // col_wear: "C".to_string(),
    // };

    gui()?;
    
    Ok(())
}


/* WHATS LEFT:
    - EVEN BETTER ERROR HANDLING (MAYBE)
*/
