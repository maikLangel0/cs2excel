use std::sync::Arc;
use std::{fs::File, io::BufReader, path::PathBuf, str::FromStr};

use iced::widget::image::Handle;
use iced::widget::text_editor::{Content, Edit};
use iced::alignment::Horizontal;
use iced::widget::{checkbox, column, container, horizontal_rule, image, row, text_editor, Column, Row};
use iced::window::{Settings, icon};
use iced::{window, Element, Length, Pixels, Size, Subscription, Task};

use crate::dprintln;
use crate::excel::excel_runtime::{self, is_user_input_valid};
use crate::gui::templates_n_methods::{btn_base, padding_inner, path_to_file_name, pick_list_template, slider_template, task_cell_if_english_alphabetic, task_col_if_english_alphabetic, text_editor_template, text_input_template, tooltip_default, ToNumeric, ToOption};
use crate::models::{price::{Currencies, PricingMode, PricingProvider}, user_sheet::{SheetInfo, UserInfo, UserSheet}, web::{ItemInfoProvider, Sites}};

use strum::IntoEnumIterator;
use rfd::AsyncFileDialog;

const FILL: Length = Length::Fill;
const NAUR_BYTES: &[u8] = include_bytes!("../../assets/images/peak_naur.png");
const ADDITIONAL_INFO: &str = "IMPORTANT INFO: \
    \n\nEXCEL FILE NEEDS TO CLOSED THE INSTANCE YOU START THE PROGRAM AND THE INSTANCE THE PROGRAM ENDS! HAVING THE FILE OPEN WHEN CLICKING 'Run' WILL RESULT IN AN ERROR. IF PROGRAM IS OPEN AT THE END OF ITERATION, WRITING TO THE EXCEL FILE WILL NOT BE SUCCESSFUL.
    \nPlease always have a recent up-to-date backup of your spreadsheet(s) \
    \nPlease make sure the rows of the table has no gaps in it. if it does the program will not recognize the whole table and add information in random, not intended places.
    \n'?' next to the name of an input means that thing is OPTIONAL";

const CS2TRADER: &str = "https://csgotrader.app/";
const CS2TRADER_REPO: &str = "https://github.com/gergelyszabo94/csgo-trader-extension";
const CS2EXCEL_REPO: &str = "https://github.com/maikLangel0/cs2excel";

#[derive(Debug, Clone)]
pub struct Progress {
    pub message: String,
    pub percent: f32,
}

#[derive(Debug, Clone)]
pub enum Exec {
    PreferMarkets(text_editor::Action),
    SteamLoginSecure(String),
    IteminfoProvider(ItemInfoProvider),
    UsdToX(Currencies),
    PricingMode(PricingMode),
    PricingProvider(PricingProvider),

    PauseTimeMs(u16),
    TextPauseTimeMs((String, u16, u16)),
    PercentThreshold(u8),
    TextPercentThreshold((String, u8, u8)),

    Steamid(String),
    SheetName(String),

    IgnoreAlreadySold,
    GroupSimularItems,
    SumQuantityPrices,
    FetchPrices,
    FetchSteam,
    IgnoreSteamNames(text_editor::Action),

    // Rows
    RowStartWrite(String),
    RowStopWrite(String),

    // Columns
    ColSteamName(String),
    ColPrice(String),
    ColGunStickerCase(String),
    ColSkinName(String),
    ColWear(String),
    ColFloat(String),
    ColPattern(String),
    ColPhase(String),
    ColQuantity(String),
    ColMarket(String),
    ColSold(String),
    ColInspectLink(String),  
    ColCsgoskinsLink(String),
    ColAssetId(String), 

    // Cell     
    CellDate(String),
    CellUsdToX(String),

    BeginPathToSheet,
    FinishPathToSheet(Option<PathBuf>),
    BeginLoadData,
    FinishLoadData(Option<PathBuf>),
    BeginSaveData,
    FinishSaveData(Option<PathBuf>),
    BeginRun,
    UpdateRun(Progress),
    FinishRun(Result<(), String>),
    RuntimeResult(text_editor::Action),

    WindowResized(Size),
    BeginOpenUrl(&'static str),
    FinishOpenUrl(Result<(), String>),
    //Exit,
}

#[derive(Debug)]
pub struct App {
    pub usersheet: UserSheet,
    loaded_data: Result<Option<String>, String>,
    saved_data: Result<Option<String>, String>,
    text_pause_time_ms: String,
    text_percent_threshold: String,
    editor_ignore_steam_names: text_editor::Content,
    editor_prefer_markets: text_editor::Content,
    editor_runtime_result: text_editor::Content,
    text_input_steamid: String,
    text_input_row_start_write_in_table: String,
    text_input_row_stop_write_in_table: String,
    pick_list_usd_to_x: [Currencies; 52],
    pick_list_pricing_provider: [PricingProvider; 2],
    pick_list_pricing_mode: [PricingMode; 4],
    pick_list_iteminfo_provider: [ItemInfoProvider; 3],
    is_file_dialog_open: bool,
    is_excel_running: bool,
    window_size: Size,
    runtime_progress: f32,
    ohnepixel: Handle
}

impl Default for App {
    fn default() -> Self {
        App { 
            usersheet: UserSheet {
                user:  UserInfo { 
                    prefer_markets:             None,
                    steamloginsecure:           None, 
                    // iteminfo_provider:          ItemInfoProvider::Csfloat , // "Bots are temporarily not allowed on CSGOFloat Inspect API due to new rate limits imposed by Valve"
                    iteminfo_provider:          ItemInfoProvider::None,
                    usd_to_x:                   Currencies::None,

                    pricing_mode:               PricingMode::Cheapest,
                    pricing_provider:           PricingProvider::Csgotrader,

                    pause_time_ms:              1750,
                    steamid:                    0, //76561198389123475, // Angel0 - min inv
                    percent_threshold:          0,

                    ignore_already_sold:        false,
                    group_simular_items:        false,
                    sum_quantity_prices:        false,
                    fetch_prices:               true,
                    fetch_steam:                true,

                    ingore_steam_names:         None
                }, 
                sheet: SheetInfo {
                    path_to_sheet:              None,
                    sheet_name:                 None,

                    row_start_write_in_table:   1,
                    row_stop_write_in_table:    None,

                    col_steam_name:                   String::from(""),
                    col_price:                        String::from(""),

                    col_gun_sticker_case:       None,
                    col_skin_name:              None,
                    col_wear:                   None,
                    col_float:                  None,
                    col_pattern:                None,
                    col_phase:                  None,
                    col_quantity:               None,
                    col_market:                 None,
                    col_sold:                   None,
                    col_inspect_link:           None,
                    col_csgoskins_link:         None,
                    col_asset_id:               None,                     
                    rowcol_date:                None, 
                    rowcol_usd_to_x:            None,
                }
            },
            loaded_data: Ok(None),
            saved_data: Ok(None),
            is_file_dialog_open: false,
            is_excel_running: false,
            editor_ignore_steam_names: text_editor::Content::new(),
            editor_prefer_markets: text_editor::Content::new(),
            editor_runtime_result: Content::with_text( ADDITIONAL_INFO ),
            text_pause_time_ms: String::new(),
            text_percent_threshold: String::new(),
            text_input_steamid: String::new(),
            text_input_row_start_write_in_table: String::new(),
            text_input_row_stop_write_in_table: String::new(),
            window_size: Size::default(),

            pick_list_pricing_provider: [PricingProvider::Csgoskins, PricingProvider::Csgotrader],
            pick_list_pricing_mode: [PricingMode::Cheapest, PricingMode::Hierarchical, PricingMode::MostExpensive, PricingMode::Random],
            pick_list_iteminfo_provider: [ItemInfoProvider::Csfloat, ItemInfoProvider::Csgotrader, ItemInfoProvider::None], 
            pick_list_usd_to_x: {
                let mut arr = [Currencies::None; 52];
                for (i, item) in Currencies::iter().enumerate() {
                    arr[i] = item;
                }
                arr
            },
            runtime_progress: 0.0,
            ohnepixel: Handle::from_bytes(NAUR_BYTES)
        }
    }
}

impl App {
    fn update(state: &mut Self, exec: Exec) -> Task<Exec> {
        if state.is_file_dialog_open && !matches!( exec, Exec::FinishLoadData(_) | Exec::FinishSaveData(_) /*| Exec::Exit */| Exec::FinishPathToSheet(_)) { return Task::none() }
        if state.is_excel_running && !matches!(exec, Exec::UpdateRun(_) | Exec::FinishRun(_) | Exec::BeginOpenUrl(_)) { return Task::none() }

        let user = &mut state.usersheet.user;
        let sheet = &mut state.usersheet.sheet;

        match exec {
            //Exec::Exit => window::get_latest().and_then(|id| window::close(id)),
            Exec::WindowResized(size)   => { state.window_size = size; Task::none() },
            Exec::IgnoreAlreadySold     => { user.ignore_already_sold = !user.ignore_already_sold; Task::none() },
            Exec::GroupSimularItems     => { user.group_simular_items = !user.group_simular_items; Task::none() },
            Exec::SumQuantityPrices     => { user.sum_quantity_prices = !user.sum_quantity_prices; Task::none() }
            Exec::FetchPrices           => { user.fetch_prices = !user.fetch_prices; Task::none() }
            Exec::FetchSteam            => { user.fetch_steam = !user.fetch_steam; Task::none() },
            Exec::UsdToX(c)             => { user.usd_to_x = c; Task::none() },
            Exec::PricingProvider(pp)   => { user.pricing_provider = pp; Task::none() },
            Exec::PricingMode(pm)       => { user.pricing_mode = pm; Task::none() },
            Exec::IteminfoProvider(ip)  => { user.iteminfo_provider = ip; Task::none() },
            Exec::IgnoreSteamNames(act) => {
                state.editor_ignore_steam_names.perform( act.clone() );

                if matches!(act, text_editor::Action::Edit(_)) {
                    user.ingore_steam_names = if !state.editor_ignore_steam_names.text().is_empty() { 
                        Some(
                            state.editor_ignore_steam_names.text()
                                .split(",")
                                .filter(|s| !s.is_empty() )
                                .map(|s| s.trim().to_owned())
                                .collect::<Vec<String>>()
                            ) 
                    } else { None };
                }
                Task::none()
            },
            Exec::PreferMarkets(act) => {
                if matches!(&act, &text_editor::Action::Edit(_)) { 
                    user.prefer_markets = if !state.editor_prefer_markets.text().is_empty() {
                        Some( {
                            let sites_string = state.editor_prefer_markets.text()
                                .split(",")
                                .map(|s| s.trim().to_owned())
                                .collect::<Vec<String>>();

                            sites_string.iter().filter_map( |s| Sites::from_str(s).ok() ).collect::<Vec<Sites>>()
                        } )
                    } else { None };
                };
                state.editor_prefer_markets.perform(act);
                Task::none()
            },
            Exec::RuntimeResult(act) => {
                if !matches!(act, text_editor::Action::Edit(_)) {
                    state.editor_runtime_result.perform(act);
                }
                Task::none()
            }

            Exec::Steamid(id) => {
                if id.chars().any(|c| !c.is_ascii_digit()) { return Task::none() } // Filter only numbers
                user.steamid = id.to_numeric().unwrap_or(0);
                state.text_input_steamid = id;
                Task::none()
            }
            Exec::SteamLoginSecure(sls) => { user.steamloginsecure = sls.to_option(); Task::none() }
            Exec::PauseTimeMs(ms) => {
                user.pause_time_ms = ms;
                state.text_pause_time_ms = ms.to_string();
                Task::none()
            }
            Exec::TextPauseTimeMs((ms, s, e)) => {
                if ms.chars().any(|c| !c.is_ascii_digit()) { return Task::none() } // Filter only numbers
                if let Ok(res) = ms.parse::<u16>() {
                    
                    user.pause_time_ms = 
                        if res < s { s } 
                        else if res > e { e } 
                        else { res }
                }
                state.text_pause_time_ms = ms;
                Task::none()
            }

            Exec::PercentThreshold(pt) => {
                user.percent_threshold = pt;
                state.text_percent_threshold = pt.to_string();
                Task::none()
            }
            Exec::TextPercentThreshold((pt, s, e)) => {
                if pt.chars().any(|c| !c.is_ascii_digit()) { return Task::none() } // Filter only numbers
                if let Ok(res) = pt.parse::<u8>() {
                    
                    user.percent_threshold = 
                        if res < s { s } 
                        else if res > e { e } 
                        else { res }
                }
                state.text_percent_threshold = pt;
                Task::none()
            }

            // Row, Col, RowCol and rest of sheet STUFF
            Exec::RowStartWrite(row) => {
                if row.chars().any(|c| !c.is_ascii_digit()) { return Task::none() } // Filter only numbers
                sheet.row_start_write_in_table = if let Ok(num) = row.to_numeric() { if num < 1 { 1 } else { num } } else { 1 };
                state.text_input_row_start_write_in_table = row;
                Task::none()
            }
            Exec::RowStopWrite(row) => {
                if row.chars().any(|c| !c.is_ascii_digit()) { return Task::none() } // Filter only numbers
                sheet.row_stop_write_in_table = 
                    if row.is_empty() { None } 
                    else { row.to_numeric().ok() };
                state.text_input_row_stop_write_in_table = row;
                Task::none()
            }
            Exec::SheetName(sn) =>          { sheet.sheet_name = sn.to_option(); Task::none() }
            Exec::ColSteamName(s) =>        { task_col_if_english_alphabetic(&mut sheet.col_steam_name, &s) }
            Exec::ColPrice(s) =>            { task_col_if_english_alphabetic(&mut sheet.col_price, &s) }
            Exec::ColGunStickerCase(gsc) => { task_col_if_english_alphabetic(&mut sheet.col_gun_sticker_case, &gsc) }
            Exec::ColSkinName(s) =>         { task_col_if_english_alphabetic(&mut sheet.col_skin_name, &s) }
            Exec::ColWear(s) =>             { task_col_if_english_alphabetic(&mut sheet.col_wear, &s) }
            Exec::ColFloat(s) =>            { task_col_if_english_alphabetic(&mut sheet.col_float, &s) }
            Exec::ColPattern(s) =>          { task_col_if_english_alphabetic(&mut sheet.col_pattern, &s) }
            Exec::ColPhase(s) =>            { task_col_if_english_alphabetic(&mut sheet.col_phase, &s) }
            Exec::ColQuantity(s) =>         { task_col_if_english_alphabetic(&mut sheet.col_quantity, &s) }
            Exec::ColMarket(s) =>           { task_col_if_english_alphabetic(&mut sheet.col_market, &s) }
            Exec::ColSold(s) =>             { task_col_if_english_alphabetic(&mut sheet.col_sold, &s) }
            Exec::ColInspectLink(s) =>      { task_col_if_english_alphabetic(&mut sheet.col_inspect_link, &s) }
            Exec::ColCsgoskinsLink(s) =>    { task_col_if_english_alphabetic(&mut sheet.col_csgoskins_link, &s) }
            Exec::ColAssetId(s) =>          { task_col_if_english_alphabetic(&mut sheet.col_asset_id, &s) }
            Exec::CellDate(s) =>            { task_cell_if_english_alphabetic(&mut sheet.rowcol_date, &s) }
            Exec::CellUsdToX(s) =>          { task_cell_if_english_alphabetic(&mut sheet.rowcol_usd_to_x, &s) }

            // Async tasks :D
            Exec::BeginPathToSheet => {
                state.is_file_dialog_open = true;

                Task::perform(
                    async {
                        let save_file = AsyncFileDialog::new()
                            .set_directory( std::env::current_dir().unwrap_or(std::env::home_dir().expect("what")) )
                            .add_filter("EXCEL files", &["xlsx"])
                            .set_title("Get xlsx file")
                            .pick_file()
                            .await;
                        save_file.map(|f| f.inner().to_path_buf() )
                    }, 
                    Exec::FinishPathToSheet
                )
            }
            Exec::FinishPathToSheet(file) => {
                state.is_file_dialog_open = false;

                if let Some(path) = file {
                    sheet.path_to_sheet = Some(path);
                }
                Task::none()
            },
            Exec::BeginSaveData => {
                state.is_file_dialog_open = true;
        
                Task::perform(
                    async {
                        let save_file = AsyncFileDialog::new()
                            .set_directory( std::env::current_dir().unwrap_or(std::env::home_dir().expect("what")) )
                            .add_filter("JSON Files", &["json"])
                            .set_title("Save JSON file")
                            .save_file()
                            .await;
                        save_file.map(|f| f.inner().to_path_buf() )
                    }, 
                    Exec::FinishSaveData
                )
            }
            Exec::FinishSaveData(file) => {
                state.is_file_dialog_open = false;

                if let Some(path) = &file {
                    match File::create(path) {
                        Ok(file) => {
                            if serde_json::to_writer(&file, &state.usersheet).is_ok() {
                                state.saved_data = Ok( path_to_file_name(path) );
                                state.loaded_data = Ok(None);
                            }
                            else { state.saved_data = Err( String::from("Failed writing save file") )}
                        },
                        Err(_) => { state.saved_data = Err( String::from("Failed creating file")) }
                    }
                }
                Task::none()
            },
            Exec::BeginLoadData => {
                state.is_file_dialog_open = true;
                Task::perform( 
                    async {
                        let pick_file = AsyncFileDialog::new()
                            .set_directory( std::env::current_dir().unwrap_or(std::env::home_dir().expect("what")) )
                            .add_filter("JSON Files", &["json"])
                            .set_title("Load JSON file")
                            .pick_file()
                            .await;
                        pick_file.map(|f| f.inner().to_path_buf() )
                    }, 
                    Exec::FinishLoadData
                )
            },
            Exec::FinishLoadData(file) => {
                state.is_file_dialog_open = false;

                if let Some(path) = &file {
                    if let Ok(file) = File::open(path) {
                        let read = BufReader::new(file);

                        match serde_json::from_reader::<_, UserSheet>(read) {
                            Ok(load) => {
                                *user = load.user;
                                *sheet = load.sheet;

                                let isn_input: String = if let Some(isn) = &user.ingore_steam_names { 
                                    isn.join(", ") 
                                } else { String::new() };
                                
                                let pm_input: String = if let Some(pm) = &user.prefer_markets { 
                                    pm.iter().map(|m| m.as_str()).collect::<Vec<&str>>().join(", ")
                                } else { String::new() };
                                
                                state.loaded_data = Ok( path_to_file_name(path) );
                                state.saved_data = Ok(None);
                                state.editor_ignore_steam_names = text_editor::Content::with_text( &isn_input );
                                state.editor_prefer_markets = text_editor::Content::with_text( &pm_input );
                                state.text_pause_time_ms = user.pause_time_ms.to_string();
                                state.text_percent_threshold = user.percent_threshold.to_string();
                                state.text_input_steamid = user.steamid.to_string();
                                state.text_input_row_start_write_in_table = sheet.row_start_write_in_table.to_string();
                                state.text_input_row_stop_write_in_table = if let Some(srit) = sheet.row_stop_write_in_table {srit.to_string()} else {String::new()};
                                
                                dprintln!("STATE: {:#?}", state);
                            },
                            Err(_e) => { dprintln!("{_e}"); state.loaded_data = Err( String::from("Failed parsing file") ) }
                        } 
                    } else { state.loaded_data = Err( String::from("Failed reading file")) }
                }
                Task::none()
            }  
            Exec::BeginRun => {
                match is_user_input_valid(user, sheet) {
                    Ok(_) => {
                        state.is_excel_running = true;
                        state.editor_runtime_result = text_editor::Content::new();
                        dprintln!("Attempt to run.");

                        if user.iteminfo_provider == ItemInfoProvider::None {
                            state.editor_runtime_result.perform( text_editor::Action::Edit( Edit::Paste( Arc::new("WARNING: Pricing for doppler phases will not be accurate when fetch more iteminfo is off.\n".to_string()) ) ) );
                        }
                        if user.iteminfo_provider == ItemInfoProvider::None && sheet.col_inspect_link.is_some() {
                            state.editor_runtime_result.perform( text_editor::Action::Edit( Edit::Paste( Arc::new("WARNING: col inspect link is not defined so you will not be able to fetch more iteminfo (float, doppler phase, pattern, correct price of dopplers).\n".to_string()) ) ) );
                        }
                        if user.usd_to_x != Currencies::None && sheet.rowcol_usd_to_x.is_some() {
                            user.usd_to_x = Currencies::None;
                        }
                        // ---------------------
                        let user = user.clone();
                        let sheet = sheet.clone();

                        let (task, _handle) = Task::sip(
                            excel_runtime::run_program(user, sheet), 
                            Exec::UpdateRun, 
                            Exec::FinishRun
                        ).abortable();

                        task
                    },
                    Err(e) => { 
                        dprintln!("User input is not valid! \n{}", e); 
                        state.editor_runtime_result = text_editor::Content::new();
                        state.editor_runtime_result.perform(text_editor::Action::Edit( Edit::Paste(Arc::new(e)) ));
                        Task::none() 
                    },
                }
            }
            Exec::UpdateRun(update) => {
                state.editor_runtime_result.perform( text_editor::Action::Edit( Edit::Paste( Arc::new(update.message) ) ) );
                state.runtime_progress = update.percent;
                Task::none()
            }
            Exec::FinishRun(res) => {
                match res {
                    Ok(_) => { state.editor_runtime_result.perform( text_editor::Action::Edit( Edit::Paste( Arc::new("\nFinished successfully!".to_string())) ) ) },
                    Err(e) => { state.editor_runtime_result.perform( text_editor::Action::Edit( Edit::Paste( Arc::new(format!("\nError!\n{}", e)) ) ) ) },
                }
                state.is_excel_running = false;
                Task::none()
            },
            Exec::BeginOpenUrl(s) => {
                Task::perform(
                    async move { open::that(s).map_err(|_| String::from("Failed to open URL")) }, 
                    Exec::FinishOpenUrl
                )
            },
            Exec::FinishOpenUrl(res) => {
                match res {
                    Ok(_) => {dprintln!("Worked :D")},
                    Err(e) => {
                        state.editor_runtime_result.perform( text_editor::Action::Edit( Edit::Paste( Arc::new(format!("\nError!\n{}", e))))); 
                        dprintln!("{}", e)
                    },
                };
                Task::none()
            }
        }
    }

    // VIEW LOGIC ----------state.is_excel_running = false;--------------------------------
    fn view<'a>(state: &'a Self) -> Element<'a, Exec> {
        let mut content: Column<Exec> = column![];
        let user = &state.usersheet.user;
        let sheet = &state.usersheet.sheet;
        
        // All checkboxes
        let radio_buttons: Row<Exec> = row![
            row![
                checkbox("Ignore already sold?", user.ignore_already_sold)
                    .on_toggle( |_| Exec::IgnoreAlreadySold ),
                tooltip_default("Ignore items that are already sold, given you have a column defined for that in your spreadsheet.", 300, 100),
            ].width( Length::FillPortion(5) ).spacing(5),
            row![
                checkbox("Group simular items?", user.group_simular_items)
                    .on_toggle( |_| Exec::GroupSimularItems ),
                tooltip_default("If you have multiple of the same item, lets say Fracture Cases, it will group them together under one row and fill the quantity column set for your spreadsheet.", 300, 100),
            ].width( Length::FillPortion(5) ).spacing(5),
            row![
                checkbox("Multiply quantity and price?", user.sum_quantity_prices)
                    .on_toggle( |_| Exec::SumQuantityPrices ),
                tooltip_default("If you want the program to automatically calculate the price of items given the quantity of them * price.", 300, 100),
            ].width( Length::FillPortion(5) ).spacing(5),
            row![
                padding_inner(20),
                checkbox("Fetch prices?", user.fetch_prices)
                    .on_toggle( |_| Exec::FetchPrices ),
                tooltip_default("Fetch prices and update the prices in your spreadsheet.", 300, 100),
            ].width( Length::FillPortion(5) ).spacing(5),
            row![
                checkbox("Fetch from Steam?", user.fetch_steam)
                    .on_toggle( |_| Exec::FetchSteam ),
                tooltip_default("Fetch your inventory from steam to fill new data in your spreadsheet.", 300, 100),
            ].width( Length::FillPortion(5) ).spacing(5) 
        ].padding(2).spacing(5);

        content = content.push( radio_buttons );
        content = content.push( horizontal_rule(2) );
        
        if !user.fetch_prices && !user.fetch_steam {            
            content = content.push( container("").height( state.window_size.height / 2.0 - 75.0 ));
            content = content.push( image( &state.ohnepixel ).width( Length::Fill ) );
            return content.align_x(Horizontal::Center).into();
        }

        // Pick lists ------------------------------
        let usd_to_x = if sheet.rowcol_usd_to_x.is_some() || !user.fetch_prices { column![] } 
        else { 
            pick_list_template(
                "Pick your primary currency, choose USD if you want to keep USD pricing, or choose NONE to prioritize Cell USD to X.",
                "Convert USD to X",
                state.pick_list_usd_to_x,
                Some( user.usd_to_x ),
                Exec::UsdToX,
                FILL
            ) 
        };

        let pricing_provider = if !user.fetch_prices { column![] } 
        else { 
            pick_list_template(
                "Which site/API that fetches the prices. \nPS: ONLY CsgoTrader IMPLEMENTED",
                "Pricing provider",
                state.pick_list_pricing_provider, 
                Some( user.pricing_provider ), 
                Exec::PricingProvider,
                FILL
            )
        };

        let pricing_mode = if !user.fetch_prices { column![] } 
        else {
            pick_list_template(
                "Chooses how the price of your items are calculated if you have chosen multiple preferred markets.",
                "Pricing mode",
                state.pick_list_pricing_mode, 
                Some( user.pricing_mode ), 
                Exec::PricingMode,
                FILL
            )
        };
        
        let iteminfo_provider = pick_list_template(
            "Which site/API fetches the dditional info about your items like float, pattern etc... \nPS: ONLY CSFLOAT IMPLEMENTED AND IT MIGHT BE DOWN DUE TO VALVE CHANGING API RULES UNLUCKY.",
            "Iteminfo provider",
            state.pick_list_iteminfo_provider, 
            Some( user.iteminfo_provider ),
            Exec::IteminfoProvider,
            FILL
        );

        // All convert to numbers ------------------------------
        let steamid = if !user.fetch_steam { column![] } 
        else {
            text_input_template(
                "Your (or someone elses) steamID64.", 
                (300.0, 50.0), 
                "SteamID", 
                "Ex: 76561198389123475", 
                Some( &state.text_input_steamid ), 
                Exec::Steamid, 
                FILL
            )
        };
        let steamloginsecure = if !user.fetch_steam { column![] } 
        else {
            text_input_template(
                "Your SteamLoginSecure token. You can get this by inspecting the developer console in your browser as you do any authenitcated action on steamcommunity.com (it will be under the 'cookie' field). If you use Firefox and is on Windows, you can log in and the program will fetch this token for you. \nPS: THIS DOES NOT HAVE TO BE SET TO SOMETHING, ONLY IF YOU WANT THE MOST UP-TO-DATE INFO OF THE INVENTORY.",
                (900.0, 100.0), 
                "SteamLoginSecure?", 
                "Ex: 76561198389123475%7C%7CeyAidHlwIjogIkpXVCIsICJhbGciOiAiRWREU0EiIH0...", 
                user.steamloginsecure.as_ref(), 
                Exec::SteamLoginSecure, 
                FILL
            )
        };
        let row_start_write = text_input_template(
            "Which row you want the program to start reading and/or writing to your spreadsheet.",
            (300.0, 100.0), 
            "Row start write in table", 
            "Ex: 2", 
            Some( &state.text_input_row_start_write_in_table ), 
            Exec::RowStartWrite, 
            FILL
        );
        let row_stop_write = text_input_template(
            "Which row you want the program to stop reading and writing to your spreadsheet. Keep empty to read the whole table.",
            (300.0, 100.0), 
            "Row stop write in table?", 
            "Ex: 99 OR no input", 
            Some( &state.text_input_row_stop_write_in_table ), 
            Exec::RowStopWrite, 
            FILL
        );

        // Sliders and text editors ------------------------------
        let pause_time_ms = if user.iteminfo_provider == ItemInfoProvider::None { column![] } 
        else {
            slider_template(
                "If you fetch additional iteminfo, this is the time between each fetch.",
                "Pause time (in ms)",
                (300.0, 100.0),
                1000..=2500,
                user.pause_time_ms,
                &state.text_pause_time_ms,
                Exec::PauseTimeMs,
                Exec::TextPauseTimeMs,
                FILL
            )
        };
        let ignore_steam_names = text_editor_template( 
            "Names of items you dont want to evaluate the price of. If you're unsure about the format of the names, see the names inside the column for full name in your generated spreadsheet.",
            "Ignore Items by Full Name?",
            "(Names Seperated By ',')",
            &state.editor_ignore_steam_names, 
            100,
            FILL,
            (400.0, 125.0),
            Exec::IgnoreSteamNames
        );
        let prefer_markets = if !user.fetch_prices { column![] } 
        else {
            text_editor_template( 
                "Names of markets you want to use to calculate the price of your items. If empty, it will use all markets available. \nALL MARKETS: \nYoupin, Csfloat, Csmoney, Buff163, Steam, Skinport, Bitskins",
                "Prefer Markets",
                "(Market Names Seperated By ',')",
                &state.editor_prefer_markets, 
                100,
                FILL,
                (400.0, 125.0),
                Exec::PreferMarkets
            )
        };
        let percent_threshold = if matches!(user.pricing_mode, PricingMode::Hierarchical) && user.fetch_prices {
            slider_template(
                "When Pricing Mode is Hierarchical, this sets the minimum percent price difference required to switch to a lower-ranked market. The program selects the cheapest market only if its price is at least this much lower than the previous one.",
                "Percent threshold",
                (700.0, 100.0),
                0..=100,
                user.percent_threshold,
                &state.text_percent_threshold,
                Exec::PercentThreshold,
                Exec::TextPercentThreshold,
                FILL)
        } else { column![] };

        // Misc
        let sheet_name = text_input_template(
            "Name of the sheet in your xlsx file that you want to alter. Default name in new spreadsheet is Sheet1.", 
            (300.0, 100.0), 
            "Sheet name?", 
            "Ex: Sheet1", 
            sheet.sheet_name.as_ref(), 
            Exec::SheetName, 
            FILL
        );

        // Cols
        let col_full_name = text_input_template(
            "Name of column where the name of the item IN FULL is put (Ex: AK-47 | Blue Laminate (Field-Tested). This is needed to index the spreadsheet.", 
            (300.0, 100.0), 
            "Col full name", 
            "Ex: A", 
            Some( &sheet.col_steam_name ), 
            Exec::ColSteamName, 
            FILL
        );
        let col_price = if !user.fetch_prices { column![] } 
        else {
            text_input_template(
                "Name of column where the price of items will be written and read.",
                (300.0, 100.0), 
                "Col price", 
                "Ex: I", 
                Some( &sheet.col_price ), 
                Exec::ColPrice, 
                FILL
            )
        };
        let col_gun_sticker_case = text_input_template(
            "Name of column where the gun name can be written (Ex: M4A4)", 
            (300.0, 100.0), 
            "Col gun/sticker/case?", 
            "Ex: B", 
            sheet.col_gun_sticker_case.as_ref(), 
            Exec::ColGunStickerCase, 
            FILL
        );
        let col_skin_name = text_input_template(
            "Name of column where the skin name can be written (Ex: blue laminate)",
            (300.0, 100.0), 
            "Col skin name?", 
            "Ex: C", 
            sheet.col_skin_name.as_ref(), 
            Exec::ColSkinName, 
            FILL
        );
        let col_wear = text_input_template(
            "Name of column where the wear/variant can be written (Ex: fn, holo, holo-foil)",
            (300.0, 100.0), 
            "Col wear?", 
            "Ex: D", 
            sheet.col_wear.as_ref(), 
            Exec::ColWear, 
            FILL
        );
        let col_float = if user.iteminfo_provider == ItemInfoProvider::None || sheet.col_inspect_link.is_none() { column![] }
        else {
            text_input_template(
                "Name of column where the float can be written (Ex: 0.169067069888115)",
                (300.0, 100.0), 
                "Col float?", 
                "Ex: E", 
                sheet.col_float.as_ref(), 
                Exec::ColFloat, 
                FILL
            )
        };
        let col_pattern = if user.iteminfo_provider == ItemInfoProvider::None || sheet.col_inspect_link.is_none() { column![] }
        else {
            text_input_template(
                "Name of column where the pattern can be written and read (Ex: 661)",
                (300.0, 100.0), 
                "Col pattern?", 
                "Ex: F", 
                sheet.col_pattern.as_ref(), 
                Exec::ColPattern, 
                FILL
            )
        };
        let col_phase = if user.iteminfo_provider == ItemInfoProvider::None || sheet.col_inspect_link.is_none() { column![] }
        else {
            text_input_template(
                "Name of column where the phase can be written (Ex: phase 4, emerald)",
                (300.0, 100.0), 
                "Col phase?", 
                "Ex: G", 
                sheet.col_phase.as_ref(), 
                Exec::ColPhase, 
                FILL
            )
        };
        let col_quantity = text_input_template(
            "Name of column where the quantity can be written and read (Ex: 67)",
            (300.0, 100.0), 
            if user.group_simular_items {"Col quantity"} else {"Col quantity?"}, 
            "Ex: H", 
            sheet.col_quantity.as_ref(), 
            Exec::ColQuantity, 
            FILL
        );
        let col_market = if !user.fetch_prices { column![] } 
        else {
            text_input_template(
                "Name of column where the market can be written (Ex: youpin, buff)",
                (300.0, 100.0), 
                "Col market?", 
                "Ex: J", 
                sheet.col_market.as_ref(), 
                Exec::ColMarket, 
                FILL
            )
        };
        let col_sold = if !user.fetch_prices || !user.ignore_already_sold { column![] } 
        else {
            text_input_template(
                "Name of column where items that are already sold can be read from.",
                (300.0, 100.0), 
                "Col sold", 
                "Ex: K", 
                sheet.col_sold.as_ref(), 
                Exec::ColSold, 
                FILL
            )
        };
        let col_inspect_link = if !user.fetch_prices { column![] } 
        else { 
            text_input_template(
                "Name of column where the inspect link for the items can be written and read \n(Ex: steam://rungame/730/76561202255233023/+csgo_econ_action_preview%20S76561198389123475A34543022281D9279926981479153949)",
                (300.0, 100.0), 
                if matches!(user.iteminfo_provider, ItemInfoProvider::None) {"Col inspect link?"} else {"Col inspect link"}, 
                "Ex: L", 
                sheet.col_inspect_link.as_ref(), 
                Exec::ColInspectLink, 
                FILL
            )
        };
        let col_csgoskins_link = text_input_template(
            "Name of column where a generated csgoskins.gg link can be written. This is if you want to check the price yourself.",
            (300.0, 100.0), 
            "Col csgoskins link?", 
            "Ex: M", 
            sheet.col_csgoskins_link.as_ref(), 
            Exec::ColCsgoskinsLink, 
            FILL
        );
        let col_assetid = if user.group_simular_items { column![] } 
        else {
            text_input_template(
                "Name of column where the assetID of the items in your inventory can be written and read. This is to seperate items with the same name when you do not want to group simular items.",
                (350.0, 100.0), 
                "Col assetid", 
                "Ex: N", 
                sheet.col_asset_id.as_ref(), 
                Exec::ColAssetId, 
                FILL
            )
        };

        // Cells 
        let cell_date = text_input_template(
            "Coordinates of cell where the current time and date can be written.",
            (300.0, 100.0), 
            "Cell date?", 
            "Ex: O2 | $O2 | O$2 | $O$2", 
            sheet.rowcol_date.as_ref(), 
            Exec::CellDate, 
            FILL
        );
        let cell_usd_to_x = if user.usd_to_x != Currencies::None || !user.fetch_prices { column![] }
        else { 
            text_input_template(
                "Coortinates of cell where the conversion rate from USD to X can be read to use with calculating the price of the items.",
                (300.0, 100.0), 
                "Cell USD to X", 
                "Ex: P2 | $P2 | P$2 | $P$2", 
                sheet.rowcol_usd_to_x.as_ref(), 
                Exec::CellUsdToX, 
                FILL
            )
        };

        // Buttons ------------------------------
        let save = btn_base( 
            match &state.saved_data {
                Ok(mb_file) => {
                    if let Some(file) = mb_file { format!("Saved {}", file) }
                    else { String::from("Save") }},
                Err(e) => { e.to_string() }
            },
            None::<Pixels>,
            Some( FILL ), 
            None::<Length>,
            Exec::BeginSaveData
        );
        let load = btn_base(
            match &state.loaded_data {
                Ok(mb_file) => {
                    if let Some(file) = mb_file { format!("Loaded {}", file) }
                    else { String::from("Load") }},
                Err(e) => { e.to_string() }
            },
            None::<Pixels>,
            Some( FILL ), 
            None::<Length>,
            Exec::BeginLoadData
        );
        let path_to_sheet = btn_base( 
            match &state.usersheet.sheet.path_to_sheet {
                Some(path) => format!(
                    "Found {}", {
                        let tmp = path.to_str().unwrap_or("file").split("\\").collect::<Vec<_>>();
                        tmp[tmp.len() - 1]
                    }
                ),
                None => String::from("Path to sheet"),
            },
            None::<Pixels>,
            Some( FILL ), 
            None::<Length>,
            Exec::BeginPathToSheet 
        );
        let run_program = btn_base(
            "Run", 
            None::<Pixels>, 
            Some( FILL ), 
            None::<Length>, 
            Exec::BeginRun
        );

        let cs2traderapp = btn_base(
            "Get The cs2trader Extension!", 
            Some(13), 
            Some(Length::Fixed(200.0)), 
            None::<Length>, 
            Exec::BeginOpenUrl(CS2TRADER)
        );
        let cs2excel_repo = btn_base(
            "Cs2excel Github", 
            Some(13), 
            Some(Length::Fixed(200.0)), 
            None::<Length>, 
            Exec::BeginOpenUrl(CS2EXCEL_REPO)
        );
        let cs2traderapp_repo = btn_base(
            "Cs2trader Github", 
            Some(13), 
            Some(Length::Fixed(200.0)), 
            None::<Length>, 
            Exec::BeginOpenUrl(CS2TRADER_REPO)
        );

        // Main pushes ------------------------------
        content = content.push( column![
            row![path_to_sheet, load, save, run_program].padding(4).spacing(5),
            horizontal_rule(5),
        ]);

        if user.fetch_prices {
            content = content.push( column![
                row![usd_to_x, pricing_provider, pricing_mode, iteminfo_provider].padding(4).spacing(5),
                horizontal_rule(5),
                ]
            )
        };

        content = content.push( column![
            row![ steamid, steamloginsecure, sheet_name, row_start_write, row_stop_write ].padding(4).spacing(5),
            horizontal_rule(5),

            row![ pause_time_ms, ignore_steam_names, prefer_markets, percent_threshold ].padding(4).spacing(5),
            horizontal_rule(5),

            row![col_full_name, col_gun_sticker_case, col_skin_name, col_wear, col_float ].padding(4).spacing(5),
            horizontal_rule(5),

            row![col_pattern, col_phase, col_quantity, col_price, col_market, col_sold].padding(4).spacing(5),
            horizontal_rule(5),

            row![col_inspect_link, col_csgoskins_link, col_assetid, cell_date, cell_usd_to_x].padding(4).spacing(5),
            horizontal_rule(5),

            row![ text_editor_template(ADDITIONAL_INFO, "-#- Program Output -#-", "", &state.editor_runtime_result, Length::Fill, Length::Fill, (1000.0, 300.0), Exec::RuntimeResult)],

            row![cs2traderapp, cs2traderapp_repo, cs2excel_repo].padding(4).spacing(150)

            //btn_base("Exit", Some(100), Some(50), Exec::Exit),
        ].align_x( Horizontal::Center));

        content.into()
    }

    // Dette er for the most part for å teste at jeg somewhat forstår subscriptions i Iced lol
    fn sub_window_resize() -> Subscription<Exec> {
        window::resize_events().map(|(_, size)| Exec::WindowResized( size ))
    }
}

pub fn init_gui() -> Result<(), iced::Error> {
    let app = iced::application(App::default, App::update, App::view)
        .title( "CS2EXCEL V2 | @maiklangel0" )
        .theme( |_| iced::Theme::TokyoNight )
        .subscription(|_| App::sub_window_resize() )
        .window( 
            Settings { 
                size: Size {width: 1280.0, height: 960.0}, 
                min_size: Some(Size { width: 1280.0, height: 960.0 }), 
                max_size: None, 
                resizable: true, 
                decorations: true, 
                position: window::Position::Centered,
                icon: icon::from_file(".\\assets\\images\\whatsapp_is_calling.ico").ok(), 
                ..Default::default() 
            }
        );

    app.run()
}