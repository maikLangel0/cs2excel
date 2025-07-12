use std::borrow::Borrow;
use std::time::Duration;
use std::{fs::File, io::BufReader, path::PathBuf, str::FromStr, sync::LazyLock, thread};

use iced_gif::Frames;
use iced::futures::{channel::mpsc, StreamExt, Stream};
use iced::alignment::{Horizontal, Vertical};
use iced::widget::{button, checkbox, column, container, horizontal_rule, vertical_rule, pick_list, row, Row, text::{Wrapping, IntoFragment}, text_editor, text_input, tooltip, Button, Column, Container, Tooltip, slider};
use iced::window::Settings;
use iced::{border::Radius, window, Background, Border, Color, Element, Length, Renderer, Shadow, Size, Task, Theme, Subscription};

use crate::excel::excel_runtime::{self, is_user_input_valid};
use crate::models::{price::{Currencies, PricingMode, PricingProvider}, user_sheet::{SheetInfo, UserInfo, UserSheet}, web::{ItemInfoProvider, Sites}};

use strum::IntoEnumIterator;
use rfd::AsyncFileDialog;
use num_traits::{FromPrimitive};

static BG_MAIN: LazyLock<Color> = LazyLock::new(|| Color::from_rgba8(83 ,203 ,227, 1.0));
static BG_SEC: LazyLock<Color> = LazyLock::new(|| Color::from_rgba8(44, 194, 218, 1.0));
static BG_TRI: LazyLock<Color> = LazyLock::new(|| Color::from_rgba8(38, 170, 191, 1.0));
static BG_QUAD: LazyLock<Color> = LazyLock::new(|| Color::from_rgba8(28, 118, 133, 1.0));

const RAD_MAIN: Radius = Radius { top_left: 2.0, top_right: 2.0, bottom_right: 2.0, bottom_left: 2.0 };
const RAD_SEC: Radius = Radius { top_left: 1.0, top_right: 1.0, bottom_right: 1.0, bottom_left: 1.0 };
const NO_LEN: Option<Length> = None;
const STD_LEN: Length = Length::FillPortion(4);

// #[derive(Debug, Clone)]
// enum RunUpdate {
    // Progress(f32),
    // Status(String),
    // Error(String),
// }

#[derive(Debug, Clone)]
enum Exec {
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
    FinishRun(Result<(), String>),

    WindowResized(Size),
    Exit,
}

// fn long_running_task() -> impl Stream<Item = RunUpdate> {
    // let (tx, rx) = mpsc::unbounded::<RunUpdate>();
    // println!("reached here!");
// 
    // thread::spawn(move || {
        // for i in 0..=100 {
            // let _ = tx.unbounded_send(RunUpdate::Progress(i as f32));
            // if i == 50 {
                // let _ = tx.unbounded_send(RunUpdate::Status(String::from("Halfway der")));
            // }
            // thread::sleep( Duration::from_millis(500));
        // }
        // 
    // });
    // rx
// }

#[derive(Debug)]
struct App {
    usersheet: UserSheet,
    loaded_data: Result<Option<String>, String>,
    saved_data: Result<Option<String>, String>,
    text_pause_time_ms: String,
    text_percent_threshold: String,
    editor_ignore_steam_names: text_editor::Content,
    editor_prefer_markets: text_editor::Content,
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

    gif: Frames,
}

impl Default for App {
    fn default() -> Self {
        App { 
            usersheet: UserSheet { 
                user:  UserInfo { 
                    prefer_markets:             Some( vec![Sites::YOUPIN, Sites::CSFLOAT, Sites::BUFF163] ),
                    steamloginsecure:           None, 
                    // iteminfo_provider:          ItemInfoProvider::Csfloat , // "Bots are temporarily not allowed on CSGOFloat Inspect API due to new rate limits imposed by Valve"
                    iteminfo_provider:          ItemInfoProvider::None,
                    usd_to_x:                   Currencies::CNY,

                    pricing_mode:               PricingMode::Hierarchical,
                    pricing_provider:           PricingProvider::Csgotrader,

                    pause_time_ms:              1500,
                    steamid:                    76561198389123475, // Angel0 - min inv
                    percent_threshold:          5,

                    ignore_already_sold:        true,
                    group_simular_items:        true,
                    sum_quantity_prices:        false,
                    fetch_prices:               true,
                    fetch_steam:                true,

                    ingore_steam_names: Some( Vec::from([
                            String::from("AK-47 | Blue Laminate (Field-Tested)"),
                            String::from("M4A1-S | Guardian (Minimal Wear)"),
                            String::from("AWP | Sun in Leo (Factory New)"),
                    ]) )
                }, 
                sheet: SheetInfo { 
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
            },
            loaded_data: Ok(None),
            saved_data: Ok(None),
            is_file_dialog_open: false,
            is_excel_running: false,
            editor_ignore_steam_names: text_editor::Content::new(),
            editor_prefer_markets: text_editor::Content::new(),
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
                let mut arr = [Currencies::USD; 52];
                for (i, item) in Currencies::iter().enumerate() {
                    arr[i] = item;
                }
                arr
            },
            gif: Frames::from_bytes( include_bytes!("../../assets/images/naur-ohnepixel.gif").to_vec() ).unwrap()
        }
    }
}

impl App {
    fn update(state: &mut Self, exec: Exec) -> Task<Exec> {
        if state.is_file_dialog_open && !matches!( exec, Exec::FinishLoadData(_) | Exec::FinishSaveData(_) | Exec::Exit | Exec::FinishPathToSheet(_)) { return Task::none() }
        if state.is_excel_running && !matches!(exec, Exec::FinishRun(_)) { return Task::none() }

        let user = &mut state.usersheet.user;
        let sheet = &mut state.usersheet.sheet;

        match exec {
            Exec::Exit => window::get_latest().and_then(|id| window::close(id)),
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

                            let sites = sites_string.iter().filter_map( |s| Sites::from_str(s).ok() ).collect::<Vec<Sites>>();
                            sites
                        } )
                    } else { None };
                };
                state.editor_prefer_markets.perform(act);
                Task::none()
            },
            Exec::Steamid(id) => {
                if id.chars().any(|c| !c.is_numeric()) { return Task::none() } // Filter only numbers
                user.steamid = id.to_numeric().unwrap_or_else(|_| 0);
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
                if ms.chars().any(|c| !c.is_numeric()) { return Task::none() } // Filter only numbers
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
                if pt.chars().any(|c| !c.is_numeric()) { return Task::none() } // Filter only numbers
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
                if row.chars().any(|c| !c.is_numeric()) { return Task::none() } // Filter only numbers
                sheet.row_start_write_in_table = row.to_numeric().unwrap_or_else(|_| 1);
                state.text_input_row_start_write_in_table = row;
                Task::none()
            }
            Exec::RowStopWrite(row) => {
                if row.chars().any(|c| !c.is_numeric()) { return Task::none() } // Filter only numbers
                sheet.row_stop_write_in_table = 
                    if row.is_empty() { None } 
                    else if let Ok(rsw) = row.to_numeric() { Some(rsw) }
                    else { None };
                state.text_input_row_stop_write_in_table = row;
                Task::none()
            }
            Exec::SheetName(sn) => {
                sheet.sheet_name = sn.to_option();
                Task::none()
            }
            Exec::ColSteamName(s) => { 
                if s.chars().any(|c| !c.is_alphabetic() ) { return Task::none() } 
                sheet.col_steam_name = s; 
                Task::none()
            }
            Exec::ColPrice(s) => {
                if s.chars().any(|c| !c.is_alphabetic() ) { return Task::none() }
                sheet.col_price = s; 
                Task::none() 
            }
            Exec::ColGunStickerCase(gsc) => {
                if gsc.chars().any(|c| !c.is_alphabetic() ) { return Task::none() }
                sheet.col_gun_sticker_case = gsc.to_option();
                Task::none()
            }
            Exec::ColSkinName(s) => {
                if s.chars().any(|c| !c.is_alphabetic() ) { return Task::none() }
                sheet.col_skin_name = s.to_option();
                Task::none()
            }
            Exec::ColWear(s) => {
                if s.chars().any(|c| !c.is_alphabetic() ) { return Task::none() }
                sheet.col_wear = s.to_option();
                Task::none()
            }
            Exec::ColFloat(s) => {
                if s.chars().any(|c| !c.is_alphabetic() ) { return Task::none() }
                sheet.col_float = s.to_option();
                Task::none()
            }
            Exec::ColPattern(s) => {
                if s.chars().any(|c| !c.is_alphabetic() ) { return Task::none() }
                sheet.col_pattern = s.to_option();
                Task::none()
            }
            Exec::ColPhase(s) => {
                if s.chars().any(|c| !c.is_alphabetic() ) { return Task::none() }
                sheet.col_phase = s.to_option();
                Task::none()
            }
            Exec::ColQuantity(s) => {
                if s.chars().any(|c| !c.is_alphabetic() ) { return Task::none() }
                sheet.col_quantity = s.to_option();
                Task::none()
            }
            Exec::ColMarket(s) => {
                if s.chars().any(|c| !c.is_alphabetic() ) { return Task::none() }
                sheet.col_market = s.to_option();
                Task::none()
            }
            Exec::ColSold(s) => {
                if s.chars().any(|c| !c.is_alphabetic() ) { return Task::none() }
                sheet.col_sold = s.to_option();
                Task::none()
            }
            Exec::ColInspectLink(s) => {
                if s.chars().any(|c| !c.is_alphabetic() ) { return Task::none() }
                sheet.col_inspect_link = s.to_option();
                Task::none()
            }
            Exec::ColCsgoskinsLink(s) => {
                if s.chars().any(|c| !c.is_alphabetic() ) { return Task::none() }
                sheet.col_csgoskins_link = s.to_option();
                Task::none()
            }
            Exec::ColAssetId(s) => {
                if s.chars().any(|c| !c.is_alphabetic() ) { return Task::none() }
                sheet.col_asset_id = s.to_option();
                Task::none()
            }
            Exec::CellDate(s) => {
                if s.chars().any(|c| !c.is_alphabetic() && !c.is_numeric() && c != '$' ) { return Task::none() }
                sheet.rowcol_date = s.to_option();
                Task::none()
            }
            Exec::CellUsdToX(s) => {
                if s.chars().any(|c| !c.is_alphabetic() && !c.is_numeric() && c != '$') { return Task::none() }
                sheet.rowcol_usd_to_x = s.to_option();
                Task::none()
            }

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
                        save_file.and_then(|f| Some( f.inner().to_path_buf() ))
                    }, 
                    |file| Exec::FinishPathToSheet(file)
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
                        save_file.and_then(|f| Some( f.inner().to_path_buf() ))
                    }, 
                    |file| Exec::FinishSaveData(file)
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
                        Err(_) => { state.saved_data = Err(String::from("Failed creating file")) }
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
                        pick_file.and_then(|f| Some( f.inner().to_path_buf() ))
                    }, 
                    |file| Exec::FinishLoadData(file)
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
                                    let res = pm.iter().map(|m| m.as_str()).collect::<Vec<&str>>().join(", ");
                                    res
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
                                
                                println!("STATE: {:#?}", state);
                            },
                            Err(e) => { println!("{e}"); state.loaded_data = Err( String::from("Failed parsing file") ) }
                        } 
                    } else { state.loaded_data = Err( String::from("Failed reading file")) }
                }
                Task::none()
            }  
            Exec::BeginRun => {
                match is_user_input_valid(user, sheet) {
                    Ok(_) => {
                        let user = user.clone();
                        let sheet = sheet.clone();
                        
                        state.is_excel_running = true;
                        println!("Attempt to run.");

                        Task::perform(
                            async move { excel_runtime::run_program(user, sheet).await }, 
                            |s| Exec::FinishRun(s)
                        )
                    },
                    Err(e) => { println!("User input is not valid! \n{}", e); Task::none() },
                }
            }
            Exec::FinishRun(res) => {
                state.is_excel_running = false;
                match res {
                    Ok(_) => { println!("Finished running!") },
                    Err(e) => { println!("{}", e) },
                }
                Task::none()
            }
        }
    }

    // VIEW LOGIC ----------state.is_excel_running = false;--------------------------------
    fn view(state: &Self) -> Element<Exec> {
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
            content = content.push( iced_gif::Gif::new(&state.gif).width( Length::Fill ) );
            return content.align_x( Horizontal::Center).into();
        }

        // Pick lists ------------------------------
        let usd_to_x = pick_list_template(
            "Pick your primary currency, choose USD if you want to keep USD pricing, or choose NONE to prioritize Cell USD to X.",
            "Convert USD to X",
            state.pick_list_usd_to_x,
            Some( user.usd_to_x ),
            Exec::UsdToX,
            STD_LEN
        );

        let pricing_provider = pick_list_template(
            "Which site/API that fetches the prices. \nPS: ONLY CsgoTrader IMPLEMENTED",
            "Pricing provider",
            state.pick_list_pricing_provider, 
            Some( user.pricing_provider ), 
            Exec::PricingProvider,
            STD_LEN
        );

        let pricing_mode = pick_list_template(
            "Chooses how the price of your items are calculated if you have chosen multiple preferred markets.",
            "Pricing mode",
            state.pick_list_pricing_mode, 
            Some( user.pricing_mode ), 
            Exec::PricingMode,
            STD_LEN
        );
        
        let iteminfo_provider = pick_list_template(
            "Which site/API fetches the dditional info about your items like float, pattern etc... \nPS: ONLY CSFLOAT IMPLEMENTED AND IT MIGHT BE DOWN DUE TO VALVE CHANGING API RULES UNLUCKY.",
            "Iteminfo provider",
            state.pick_list_iteminfo_provider, 
            Some( user.iteminfo_provider ),
            Exec::IteminfoProvider,
            STD_LEN
        );

        // All convert to numbers ------------------------------
        let steamid = text_input_template(
            "Your (or someone elses) steamID64.", 
            (300.0, 50.0), 
            "SteamID", 
            "Ex: 76561198389123475", 
            Some( &state.text_input_steamid ), 
            Exec::Steamid, 
            STD_LEN
        );
        let steamloginsecure = text_input_template(
            "Your SteamLoginSecure token. You can get this by inspecting the developer console in your browser as you do any authenitcated action on steamcommunity.com (it will be under the 'cookie' field). If you use Firefox and is on Windows, you can log in and the program will fetch this token for you. \nPS: THIS DOES NOT HAVE TO BE SET TO SOMETHING, ONLY IF YOU WANT THE MOST UP-TO-DATE INFO OF THE INVENTORY.",
            (900.0, 100.0), 
            "SteamLoginSecure?", 
            "Ex: 76561198389123475%7C%7CeyAidHlwIjogIkpXVCIsICJhbGciOiAiRWREU0EiIH0...", 
            user.steamloginsecure.as_ref(), 
            Exec::SteamLoginSecure, 
            STD_LEN
        );
        let row_start_write = text_input_template(
            "Which row you want the program to start reading and/or writing to your spreadsheet.",
            (300.0, 100.0), 
            "Row start write in table", 
            "Ex: 2", 
            Some( &state.text_input_row_start_write_in_table ), 
            Exec::RowStartWrite, 
            STD_LEN
        );
        let row_stop_write = text_input_template(
            "Which row you want the program to stop reading and writing to your spreadsheet. Keep empty to read the whole table.",
            (300.0, 100.0), 
            "Row stop write in table?", 
            "Ex: '' OR 99", 
            Some( &state.text_input_row_stop_write_in_table ), 
            Exec::RowStopWrite, 
            STD_LEN
        );

        // Sliders and text editors ------------------------------
        let pause_time_ms = slider_template(
            "If you fetch additional iteminfo, this is the time between each fetch.",
            "Pause time (in ms)",
            (300.0, 100.0),
            1000..=2500,
            user.pause_time_ms,
            &state.text_pause_time_ms,
            Exec::PauseTimeMs,
            Exec::TextPauseTimeMs,
            STD_LEN
        );
        let ignore_steam_names = text_editor_template( 
            "Names of items you dont want to evaluate the price of. If you're unsure about the format of the names, see the names inside the column for steam_name in your generated spreadsheet.",
            "Ignore Steam Names?",
            "( Full Names Seperated By , )",
            &state.editor_ignore_steam_names, 
            STD_LEN,
            Exec::IgnoreSteamNames
        );
        let prefer_markets = text_editor_template( 
            "Names of markets you want to use to calculate the price of your items. If empty, it will use all markets available. \nALL MARKETS: \nYoupin, Csfloat, Csmoney, Buff163, Steam, Skinport, Bitskins",
            "Prefer Markets",
            "( Market Names Seperated By , )",
            &state.editor_prefer_markets, 
            STD_LEN,
            Exec::PreferMarkets
        );
        let percent_threshold = if matches!(user.pricing_mode, PricingMode::Hierarchical) {
            slider_template(
                "When Pricing Mode is Hierarchical, this sets the minimum percent price difference required to switch to a lower-ranked market. The program selects the cheapest market only if its price is at least this much lower than the previous one.",
                "Percent threshold",
                (700.0, 100.0),
                0..=100,
                user.percent_threshold,
                &state.text_percent_threshold,
                Exec::PercentThreshold,
                Exec::TextPercentThreshold,
                STD_LEN)
        } else { column![] };

        // Misc
        let sheet_name = text_input_template(
            "Name of the sheet in your xlsx file that you want to alter. Default name in new spreadsheet is Sheet1.", 
            (300.0, 100.0), 
            "Sheet name?", 
            "Ex: Sheet1", 
            sheet.sheet_name.as_ref(), 
            Exec::SheetName, 
            STD_LEN
        );

        // Cols
        let col_steam_name = text_input_template(
            "Name of column where the name of the item IN FULL is put (Ex: AK-47 | Blue Laminate (Field-Tested). This is needed to index the spreadsheet.", 
            (300.0, 100.0), 
            "Col steam name", 
            "Ex: A", 
            Some( &sheet.col_steam_name ), 
            Exec::ColSteamName, 
            STD_LEN
        );
        let col_price = text_input_template(
            "Name of column where the price of items will be written and wread.",
            (300.0, 100.0), 
            "Col price", 
            "Ex: I", 
            Some( &sheet.col_price ), 
            Exec::ColPrice, 
            STD_LEN
        );
        let col_gun_sticker_case = text_input_template(
            "Name of column where the gun name can be written (Ex: M4A4)", 
            (300.0, 100.0), 
            "Col gun/sticker/case?", 
            "Ex: B", 
            sheet.col_gun_sticker_case.as_ref(), 
            Exec::ColGunStickerCase, 
            STD_LEN
        );
        let col_skin_name = text_input_template(
            "Name of column where the skin name can be written (Ex: blue laminate)",
            (300.0, 100.0), 
            "Col skin name?", 
            "Ex: C", 
            sheet.col_skin_name.as_ref(), 
            Exec::ColSkinName, 
            STD_LEN
        );
        let col_wear = text_input_template(
            "Name of column where the wear/variant can be written (Ex: fn, holo, holo-foil)",
            (300.0, 100.0), 
            "Col wear?", 
            "Ex: D", 
            sheet.col_wear.as_ref(), 
            Exec::ColWear, 
            STD_LEN
        );
        let col_float = text_input_template(
            "Name of column where the float can be written (Ex: 0.169067069888115)",
            (300.0, 100.0), 
            "Col float?", 
            "Ex: E", 
            sheet.col_float.as_ref(), 
            Exec::ColFloat, 
            STD_LEN
        );
        let col_pattern = text_input_template(
            "Name of column where the pattern can be written and read (Ex: 661)",
            (300.0, 100.0), 
            "Col pattern?", 
            "Ex: F", 
            sheet.col_pattern.as_ref(), 
            Exec::ColPattern, 
            STD_LEN
        );
        let col_phase = text_input_template(
            "Name of column where the phase can be written (Ex: phase 4, emerald)",
            (300.0, 100.0), 
            "Col phase?", 
            "Ex: G", 
            sheet.col_phase.as_ref(), 
            Exec::ColPhase, 
            STD_LEN
        );
        let col_quantity = text_input_template(
            "Name of column where the quantity can be written and read (Ex: 67)",
            (300.0, 100.0), 
            if user.group_simular_items {"Col quantity"} else {"Col quantity?"}, 
            "Ex: H", 
            sheet.col_quantity.as_ref(), 
            Exec::ColQuantity, 
            STD_LEN
        );
        let col_market = text_input_template(
            "Name of column where the market can be written (Ex: youpin, buff)",
            (300.0, 100.0), 
            "Col market?", 
            "Ex: J", 
            sheet.col_market.as_ref(), 
            Exec::ColMarket, 
            STD_LEN
        );
        let col_sold = text_input_template(
            "Name of column where items that are already sold can be read from.",
            (300.0, 100.0), 
            "Col sold", 
            "Ex: K", 
            sheet.col_sold.as_ref(), 
            Exec::ColSold, 
            STD_LEN
        );
        let col_inspect_link = text_input_template(
            "Name of column where the inspect link for the items can be written and read \n(Ex: steam://rungame/730/76561202255233023/+csgo_econ_action_preview%20S76561198389123475A34543022281D9279926981479153949)",
            (300.0, 100.0), 
            if !matches!(user.iteminfo_provider, ItemInfoProvider::None) {"Col inspect link?"} else {"Col inspect link"}, 
            "Ex: L", 
            sheet.col_inspect_link.as_ref(), 
            Exec::ColInspectLink, 
            STD_LEN
        );
        let col_csgoskins_link = text_input_template(
            "Name of column where a generated csgoskins.gg link can be written. This is if you want to check the price yourself.",
            (300.0, 100.0), 
            "Col csgoskins link?", 
            "Ex: M", 
            sheet.col_csgoskins_link.as_ref(), 
            Exec::ColCsgoskinsLink, 
            STD_LEN
        );
        let col_assetid = text_input_template(
            "Name of column where the assetID of the items in your inventory can be written and read. This is to seperate items with the same name when you do not want to group simular items.",
            (300.0, 100.0), 
            "Col assetid", 
            "Ex: N", 
            sheet.col_asset_id.as_ref(), 
            Exec::ColAssetId, 
            STD_LEN
        );

        // Cells 
        let cell_date = text_input_template(
            "Coordinates of cell where the current time and date can be written.",
            (300.0, 100.0), 
            "Cell date?", 
            "Ex: O2 | $O2 | O$2 | $O$2", 
            sheet.rowcol_date.as_ref(), 
            Exec::CellDate, 
            STD_LEN
        );
        let cell_usd_to_x = text_input_template(
            "Coortinates of cell where the conversion rate from USD to X can be read to use with calculating the price of the items.",
            (300.0, 100.0), 
            "Cell USD to X", 
            "Ex: P2 | $P2 | P$2 | $P$2", 
            sheet.rowcol_usd_to_x.as_ref(), 
            Exec::CellUsdToX, 
            STD_LEN
        );

        // Buttons ------------------------------
        let save = btn_base( 
            match &state.saved_data {
                Ok(mb_file) => {
                    if let Some(file) = mb_file { format!("Saved {}", file) }
                    else { String::from("Save") }},
                Err(e) => { e.to_string() }
            },
            Some( STD_LEN ), NO_LEN,
            Exec::BeginSaveData
        );
        let load = btn_base(
            match &state.loaded_data {
                Ok(mb_file) => {
                    if let Some(file) = mb_file { format!("Loaded {}", file) }
                    else { String::from("Load") }},
                Err(e) => { e.to_string() }
            },
            Some( STD_LEN ), NO_LEN,
            Exec::BeginLoadData
        );
        let path_to_sheet = btn_base( 
            match &state.usersheet.sheet.path_to_sheet {
                Some(path) => format!(
                    "Found {}", {
                        let tmp = path.to_str().unwrap_or_else(|| "file").split("\\").collect::<Vec<_>>();
                        tmp[tmp.len() - 1]
                    }
                ),
                None => String::from("Path to sheet"),
            },
            Some( STD_LEN ), NO_LEN,
            Exec::BeginPathToSheet 
        );
        let run_program = btn_base("Run", Some( STD_LEN ), NO_LEN, Exec::BeginRun);

        // Main pushes ------------------------------
        content = content.push( column![
            row![path_to_sheet, load, save, run_program].padding(4).spacing(5),
            horizontal_rule(5),
        ]);

        content = content.push( column![
            row![usd_to_x, pricing_provider, pricing_mode, iteminfo_provider].padding(4).spacing(5),
            horizontal_rule(5),

            row![ steamid, steamloginsecure, sheet_name, row_start_write, row_stop_write ].padding(4).spacing(5),
            horizontal_rule(5),

            row![ pause_time_ms, ignore_steam_names, prefer_markets, if matches!(user.pricing_mode, PricingMode::Hierarchical) {percent_threshold} else {column![]} ].padding(4).spacing(5),
            horizontal_rule(5),

            row![col_steam_name, col_gun_sticker_case, col_skin_name, col_wear, col_float].padding(4).spacing(5),
            horizontal_rule(5),

            row![col_pattern, col_phase, col_quantity, col_price, col_market, col_sold].padding(4).spacing(5),
            horizontal_rule(5),

            row![col_inspect_link, col_csgoskins_link, col_assetid, cell_date, cell_usd_to_x].padding(4).spacing(5),
            horizontal_rule(5),

            btn_base("Exit", Some(100), Some(50), Exec::Exit),
        ].align_x( Horizontal::Center));

        content.into()
    }

    // Dette er for the most part for å teste at jeg somewhat forstår subscriptions i Iced lol
    fn sub_window_resize() -> Subscription<Exec> {
        window::resize_events().map(|(_, size)| Exec::WindowResized( size ))
    }
}

fn path_to_file_name(path: &PathBuf) -> Option<String> {
    let p = path.to_str()
        .and_then(|s| Some (s.split("\\")
        .collect::<Vec<&str>>() ));

    match p {
        Some(p) => { Some(p[p.len() - 1].to_string()) }
        None => None
    }
}

fn padding_inner<'a, Exec>(width: impl Into<Length>) -> Container<'a, Exec, Theme, Renderer> {
    container("").width(width)
}

fn tooltip_default<'a, Exec> (  
    content: impl Into<String>,
    x: impl Into<Length>,
    y: impl Into<Length>
) -> Tooltip<'a, Exec, Theme, Renderer> where Exec: 'a {
    tooltip( 
        container( iced::widget::text( "?" ) )
            .center(25)
            .style( |_| 
                container::Style { 
                    text_color: Some( Color::WHITE ), 
                    background: Some( Background::Color( Color::from_rgba8(72, 76, 92, 1.0) ) ), 
                    border: Border { color: Color::from_rgba8(88, 98, 99, 1.0), width: 1.0, radius: RAD_SEC },
                    shadow: Shadow::default() 
                } 
            ), 
        container( iced::widget::text( content.into() ).size(15).center() )
            .padding(10)
            .center_x(x)
            .center_y(y)
            .style( |_| 
                container::Style { 
                    text_color: Some( Color::BLACK ), 
                    background: Some( Background::Color(*BG_MAIN) ), 
                    border: Border { color: *BG_TRI, width: 1.0, radius: RAD_MAIN }, 
                    shadow: Shadow::default() 
                } 
            ), 
        tooltip::Position::FollowCursor
    )  
}

fn btn_style_base() -> impl Fn(&Theme, button::Status) -> button::Style {
    |_, status: button::Status| button::Style {
        background: match status {
            button::Status::Active => Some( Background::Color(*BG_MAIN) ),
            button::Status::Hovered => Some( Background::Color(*BG_SEC) ),
            button::Status::Pressed => Some( Background::Color(*BG_TRI) ),
            button::Status::Disabled => Some( Background::Color(*BG_MAIN) ),
        },
        text_color: match status {
            button::Status::Active => Color::BLACK,
            button::Status::Hovered => Color::WHITE,
            button::Status::Pressed => Color::WHITE,
            button::Status::Disabled => Color::BLACK,
        },
        border: match status {
            button::Status::Active => Border { color: *BG_SEC, width: 1.0, radius: RAD_MAIN },
            button::Status::Hovered => Border { color: *BG_TRI, width: 1.0, radius: RAD_SEC },
            button::Status::Pressed => Border { color: *BG_QUAD, width: 1.0, radius: RAD_SEC },
            button::Status::Disabled => Border { color: Color::BLACK, width: 0.0, radius: RAD_SEC },
        }, 
        shadow: Shadow::default()
    }
}

fn btn_base<'a, Exec, D> (
    txt: impl Into<String>, 
    width: Option<D>, 
    height: Option<D>,
    exec: Exec
) -> Button<'a, Exec> 
where 
    D: Into<Length>,
{
    let mut btn = button( iced::widget::text(txt.into() ).align_x( Horizontal::Center ).align_y( Vertical::Center ) ).on_press(exec);

    btn = if let Some(w) = width { btn.width( w ) } else { btn };
    btn = if let Some(h) = height { btn.height( h ) } else { btn };
    btn.style( btn_style_base() )
}

fn text_editor_template<'a, Exec, F>(
    tooltip_content: impl Into<String>,
    field_name: impl Into<String>,
    field_placeholder: impl Into<String>,
    editor_state: &'a text_editor::Content,
    width: impl Into<Length>,
    exec: F,
) -> Column<'a, Exec>
where
    Exec: 'a + Clone,
    F: 'a + Fn(text_editor::Action) -> Exec + Copy, 
{
    column![
        row![
            tooltip_default(tooltip_content, 400, 125),
            iced::widget::text( field_name.into() ).width(Length::Fill).center(),
            padding_inner(30)
        ]
        .padding(2)
        .width(Length::Fill),

        text_editor(editor_state)
            .on_action(move |act| exec(act))
            .height(100)
            .placeholder( field_placeholder.into() )
            .wrapping(Wrapping::WordOrGlyph),
    ]
    .width(width)
}

fn pick_list_template<'a, T, L, V, F, Exec>(
    tooltip_content: impl Into<String>,
    field_name: impl Into<String>,
    options: L,
    selected: Option<V>,
    on_selected: F,
    width: impl Into<Length>,
) -> Column<'a, Exec>
where
    Exec: Clone + 'a,
    T: ToString + PartialEq + Clone + 'a,
    L: Borrow<[T]> + 'a,
    V: Borrow<T> + 'a,
    F: Fn(T) -> Exec + 'a,
{
    column![
        row![
            tooltip_default(tooltip_content, 400, 100).padding(3),
            iced::widget::text(field_name.into()).width(Length::Fill).center(),
            padding_inner(30)
        ].spacing(5),
        pick_list( 
            options, 
            selected, 
            on_selected
        ).width( Length::Fill ),
    ].width( width )
    .padding(5)
    .spacing(5)
}

fn text_input_template<'a, F, Exec>(
    tooltip_content: impl Into<String>,
    tooltip_size: impl Into<Size>,
    field_name: impl Into<String>,
    field_placeholder: impl Into<String>,
    field_value: Option<&String>,
    on_input: F,
    width: impl Into<Length>
) -> Column<'a, Exec>
where 
    Exec: Clone + 'a,
    F: Fn(String) -> Exec + 'a
{
    let tt_size: Size = tooltip_size.into();
    let empty = &String::new();

    column![
        row![
            tooltip_default(tooltip_content, tt_size.width, tt_size.height),
            iced::widget::text( field_name.into() ).width(Length::Fill).center(),
            padding_inner(30),
        ].spacing(5),
        text_input(
            &field_placeholder.into(), 
            field_value.as_ref().unwrap_or_else(|| &empty)
        ).on_input(move |id| on_input(id))
        .width( Length::Fill)
    ].width( width )
    .padding(5)
    .spacing(5)
}

fn slider_template<'a, T, F, G, Exec>(
    tooltip_content: impl Into<String>,
    field_name: impl Into<String>,
    tooltip_size: impl Into<Size>,
    range: std::ops::RangeInclusive<T>,
    value: T,
    text_value: &str,
    on_submit: F,
    on_input: G,
    width: impl Into<Length>
) -> Column<'a, Exec>
where
    T: From<u8> + Into<f64> + FromPrimitive + ToString + Copy + PartialOrd + IntoFragment<'a> + 'a,
    Exec: Clone + 'a,
    F: Fn(T) -> Exec + Clone + 'a,
    G: Fn((String, T, T)) -> Exec + 'a
{
    let tt_size: Size = tooltip_size.into();
    let on_input_range = range.clone();

    column![
        row![
            tooltip_default(tooltip_content, tt_size.width, tt_size.height),
            iced::widget::text( field_name.into() ).width(Length::Fill).center(),
            padding_inner(20),
        ].spacing(5),
        slider(range.clone(), value, on_submit.clone()),
        //iced::widget::text(value).align_x( Horizontal::Center ).width( Length::Fill ),
        text_input( &value.to_string(), text_value )
        .on_input(move |e| on_input((e, *on_input_range.start(), *on_input_range.end())))
        .on_submit_maybe(
            Some( 
                (
                    || { 
                        on_submit( {
                            if value < *range.start() { *range.start() } 
                            else if value > *range.end() { *range.end() } 
                            else { value }
                        } )
                    }
                )()
            ) 
        )
    ].width(width)
    .padding(5)
    .spacing(5)
}

pub trait ToNumeric {
    fn to_numeric<T>(&self) -> Result<T, T::Err> 
    where
        T: FromStr;
}
pub trait ToOption {
    fn to_option<T>(&self) -> Option<T> 
    where
        T: FromStr;
}

impl ToNumeric for String {
    fn to_numeric<T>(&self) -> Result<T, T::Err> 
    where
        T: FromStr, 
    {
        let mut res = String::with_capacity( self.len() );

        for char in self.chars() {
            if char.is_numeric() { res.push(char) }
        }
        res.parse::<T>()
    }
}
impl ToOption for str {
    fn to_option<T>(&self) -> Option<T> 
    where
        T: FromStr 
    {
        if self.trim().is_empty() { None } else { T::from_str(self).ok() }
    }
}

pub fn init_gui() -> Result<(), iced::Error> {
    let app = iced::application("cs2exe", App::update, App::view)
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
                ..Default::default() 
            }
        );

    app.run()
}