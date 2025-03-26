use crate::{
    create_csgoskins_urls, cs_get_metadata_from_market_name, sleep, 
    main_structs::{SheetInfo, UserInfo}, AppWindow, Csgoskins, Duration, 
    Error, FirefoxDb, HashMap, IndexMap, LazyLock, SteamInventory, 
    rand, reader, writer, Spreadsheet, Worksheet, Local, dprintln
};

static CSGOSKINS_COOKIES: LazyLock<String> = LazyLock::new( || { 
    let cookie: String;
    if let Ok(fox) = FirefoxDb::init() {
        
        cookie = fox.get_cookies(
            vec!["name", "value"], 
            "csgoskins.gg", 
            vec!["XSRF-TOKEN", "csgoskinsgg_session", "GvZ18GVkcDBO"]
        ).unwrap_or( String::from("FRICK") );
    } 
    else { cookie = String::from("FRICK") }
    
    if cookie == "FRICK" { dprintln!("Cookie fetch for csgoskins was not successful!") }
    cookie
});

static STEAM_COOKIES: LazyLock<String> = LazyLock::new( || {
    let cookie: String;
    if let Ok(fox) = FirefoxDb::init() {
    
        cookie = fox.get_cookies(
            vec!["name", "value"],
            "steamcommunity.com",
            vec!["steamLoginSecure"]
        ).unwrap_or( String::from("FRICK") );
    } 
    else { cookie = String::from("FRICK") }

    if cookie == "FRICK" { dprintln!("Cookie fetch for steam was not successful!") }
    cookie
});

struct ItemData {
    url_suffix: String,
    quantity: u16,
}

async fn get_excel_urls_quantities(sheet: &&mut Worksheet, params: &SheetInfo) -> Result<IndexMap<String, u16>, Box<dyn Error>> {
    let mut urls: IndexMap<String, u16> = IndexMap::new();
    let mut url: String;
    let mut quant: u16;

    let mut iter = params.row_start_table;

    loop {
        let url_coord = format!( "{}{}",params.col_url, iter );
        let quant_coord = format!( "{}{}",params.col_quantity, iter );
        
        if let Some(cell) = sheet.get_cell(url_coord) {
            url = cell.get_raw_value()
                .to_string()
        } 
        else { break } // If nothing in cell at url_coord, break from loop (no more url to fetch)

        if !url.trim_start().starts_with("http") { break } // If url is something but does not start with a valid URL_PREFIX, break

        if let Some(cell) = sheet.get_cell(quant_coord) {
            quant = cell.get_raw_value()
                .to_string()
                .parse::<u16>()
                .unwrap_or(1);
        }
        else { quant = 1 } // If bogus value in quantity column, replace with 1
        
        urls.insert(
            url.trim().to_string(), 
            quant,
        );
        iter += 1;
    }

    Ok(urls)
}

/// Returns: \
/// - HashMap of 'ITEM NAME', ('URL_SUFFIX', 'QUANTITY') \
/// - length of assets from steam (the actual items gotten froms steam) \
/// - length of total items in steam inventory (total items in users inventory)
async fn get_steam_data(user: &UserInfo) -> Result<(HashMap<String, ItemData>, usize, usize), Box<dyn Error>> {
    let mut steam_info: HashMap<String, ItemData> = HashMap::new();
    let mut suf: String;
    
    if user.fetch_steam {
        let cs_inventory = SteamInventory::init(
            user.steamid, 
            user.appid, 
            if let Some(manual_cookies) = &user.steamloginsecure { manual_cookies } else { &STEAM_COOKIES }
        ).await?;

        let temp = cs_inventory.get_item_names(true)?;

        for (name, quantity) in temp.iter() {
            // If I ever want to add more shite
            // "suf" is the suffix of the url, where in this case the base is thought of as "https://csgoskins.gg/items/"
            // and suf is what needs to be appended to create the full url, use this pattern for future suf with different
            // appid needing a different website to pricecheck
            match user.appid {
                730 => suf = create_csgoskins_urls(name)?,
                _ =>   suf = create_csgoskins_urls(name)?
            }

            steam_info.insert(
                name.clone(), 
                ItemData { 
                    url_suffix: suf,
                    quantity: *quantity
                }
            );
        }
        Ok( (steam_info, cs_inventory.get_assets_length(), cs_inventory.get_total_inventory_length()) )

    } else {
        Ok( (steam_info, 0, 0) )
    }
    
}

async fn cs_get_market_prices(url: &str, cookie: &String, user_agent: &str)  -> Result<IndexMap<String, f64>, Box<dyn Error>> {
    let cs_markets: IndexMap<String, f64>;

    let csgoskins = Csgoskins::init(url, cookie, user_agent).await?;    
    cs_markets = csgoskins.get_name_price().await?;
    
    Ok(cs_markets)
}

// -----------------------------------------------------------------------------------------

pub async fn excelling(user: &UserInfo, excel: &SheetInfo, app_weak: &slint::Weak<AppWindow>) -> Result<(), Box<dyn Error>> {
    match user.appid {
        730 => cs_excel(user, excel, app_weak).await?, // CS2
        _ => test_excel(user, excel).await?
    }

    Ok(())
}

async fn cs_excel(user: &UserInfo, excel: &SheetInfo, app_weak: &slint::Weak<AppWindow>) -> Result<(), Box<dyn Error>> {
    // Set up connection with the spreadsheet
    let mut book: Spreadsheet = reader::xlsx::read(&excel.path_to_sheet)?;
    let sheet: &mut Worksheet = book.get_sheet_by_name_mut(&excel.sheet_name).ok_or(
        format!("Couldn't instanciate the worksheet of name {}", &excel.sheet_name)
    )?;
    
    // Set up connection to the GUI and the output text to GUI
    let app = app_weak.upgrade().unwrap();
    let mut output_text = String::new();

    // Fetch data from steam and spreadsheet
    let (steam_names_urls_quantities, assets_length, inventory_length): (HashMap<String, ItemData>, usize, usize) = get_steam_data(user).await?;
    let mut sheet_urls_quantities: IndexMap<String, u16> = get_excel_urls_quantities(&sheet, excel).await?;

    // Document the found length of the table to the GUI
    // if assets_length is less than inventory length, means that fetch did not get all items in inventory (items on trade-hold)
    if assets_length < inventory_length { 
        output_text.push_str(
            &format!(
                "-- NOTE --\nFetched {} items, but {} items are in inventory!\nIf this is not the desired result, login to steam using Firefox or set SteamLoginSecure manually again.\n\n", 
                assets_length, 
                inventory_length
            )   
        );
    } else { output_text.push_str( &format!("Fetched all {} items from inventory successfully.\n\n", assets_length) ) }
    output_text.push_str( &format!("-- FOUND LENGTH OF TABLE --\n {}\n\n", sheet_urls_quantities.len()) );
    output_text.push_str( &format!("-- FIRST URL AND QUANTITY --\n{:?}\n\n", sheet_urls_quantities.first().unwrap_or( (&String::from("Nothing (for now)"), &0) ) ) );
    output_text.push_str( &format!("-- LAST URL AND QUANTITY --\n{:?}\n\n", sheet_urls_quantities.last().unwrap_or(   (&String::from("Nothing (for now)"), &0) ) ) );
    app.set_gui_output_text( output_text.as_str().into() );

    let url_prefix = "https://csgoskins.gg/items/";

    // First loop. Inserts data from steam if need be
    for (name, steam_data) in steam_names_urls_quantities.iter() {
        if !user.fetch_steam { break }

        let curr_url = format!("{}{}", url_prefix, steam_data.url_suffix);

        // If curr_url is not in sheet_urls_quantities, means its going to be a new url to add to spreadsheet, thats why .unwrap_or(old_length)
        let old_length: usize = sheet_urls_quantities.len();
        let index_of_url: usize = sheet_urls_quantities
            .get_index_of(&curr_url)
            .unwrap_or(old_length);

        let row = index_of_url + excel.row_start_table as usize;
        let coord_quantity = format!("{}{}", excel.col_quantity, row);
        
        // If sheet_urls_quantities contains curr_url, update quantity IF new quantity is more than old. 
        // Else insert the url and the accompanying quantity to sheet_urls_quantities.
        // This is for the most part used to keep track of all elements in the sheet at runtime.
        let row_of_curr_url = sheet_urls_quantities.get_index_of(&curr_url ).unwrap_or( 0 ) + excel.row_start_table as usize;
        sheet_urls_quantities
            .entry( curr_url.clone() )
            .and_modify(|val| 
                if *val < steam_data.quantity {
                    *val = steam_data.quantity;      
                    sheet.get_cell_value_mut( coord_quantity.as_ref() ).set_value_number(steam_data.quantity);

                    dprintln!("UPDATED {} QUANTITY TO {} | ROW {}\n", &curr_url, &val, &row_of_curr_url);
                    dprintln!("LENGTH OF SHEET_URLS_QUANTITIES: {}", &old_length);
                    dprintln!("LENGTH OF STEAM_NAMES_URLS_QUANTITIES: {}\n", steam_names_urls_quantities.len());

                    output_text.push_str( &format!("-- UPDATED ITEM AT ROW --\n{}\n-- URL --\n{}\n-- QUANTITY --\n{}\n\n", &row_of_curr_url, &curr_url, &val) );
                    app.set_gui_output_text( output_text.as_str().into() );
                } 
            ).or_insert(steam_data.quantity);

        // if these are the same, sheet_urls_quantities must have been inserted into (AKA new item in spreadsheet)
        if index_of_url == old_length {
            let metadata: [String; 3];
            metadata = cs_get_metadata_from_market_name(name)?; //gun_sticker_case, skin/name, wear
        
            sheet.get_cell_value_mut( format!("{}{}", &excel.col_gun_sticker_case, &row) ).set_value_string( &metadata[0]        ); // gun_sticker_case
            sheet.get_cell_value_mut( format!("{}{}", &excel.col_skin_name, &row)        ).set_value_string( &metadata[1]        ); // skin / item / case
            sheet.get_cell_value_mut( format!("{}{}", &excel.col_wear, &row)             ).set_value_string( &metadata[2]        ); // wear
            sheet.get_cell_value_mut( format!("{}{}", &excel.col_url, &row)              ).set_value_string( &curr_url           ); // Full URL
            sheet.get_cell_value_mut( coord_quantity.as_ref()                            ).set_value_number( steam_data.quantity ); // quantity

            dprintln!("NEW ITEM AT ROW: {} \nURL: {} \nQuantity: {}\n", &row, &curr_url, steam_data.quantity);
            dprintln!("LENGTH OF SHEET_URLS_QUANTITIES: {}", &sheet_urls_quantities.len());
            dprintln!("LENGTH OF STEAM_NAMES_URLS_QUANTITIES: {}\n", steam_names_urls_quantities.len());

            output_text.push_str( &format!("-- NEW ITEM AT ROW --\n{}\n-- URL --\n{}\n-- Quantity --\n{}\n\n", &row, &curr_url, steam_data.quantity) );
            app.set_gui_output_text( output_text.as_str().into() );
        }
    }

    // EITHER fetch conversion rate from spreadsheet, or if excel.rowcol_usd_to_x is only numbers, use that as conversion rate
    // Had to make dumbass workaround cuz for some reason get_value_number() wouldn't work twice in a row...
    let convertion_rate: f64;
    if excel.rowcol_usd_to_x.chars().any( |c| c.is_alphabetic() ) {
        convertion_rate = sheet.get_cell_value( excel.rowcol_usd_to_x.as_ref() )
            .get_value()
            .trim()
            .parse::<f64>()
            .map_err(|_| format!( "Failed to parse conversion rate as an f64 found in cell {}.", excel.rowcol_usd_to_x.as_str() ) )?;
    } else {
        convertion_rate = excel.rowcol_usd_to_x.trim()
            .parse::<f64>()
            .map_err(|_| "Failed to parse conversion rate as an f64 of custom conversion rate fetched from spreadsheet." )?
    }

    /* Thought behind this: 
    row_start_table is the absolute row where the table begins (Ex: row 40).
    row_start_write_in_table specifies which row INSIDE THE TABLE to start writing at.
    Since us hue-mans count table rows from 1, but machines count from 0,
        I subtract 1 to align the logic with spreadsheet row numbering.
    */
    let row_write = (excel.row_start_table + excel.row_start_write_in_table) as usize - 1;
    
    let row_stop: usize;
    if let Some(stop) = excel.row_stop_write_in_table {
        if stop < row_write as u32 { row_stop = row_write } 
        else                       { row_stop = (excel.row_start_table + stop) as usize - 1 }
    } else                         { row_stop = sheet_urls_quantities.len() + 1 }                   // <------ EXPERIMENTAL

    let iterations = row_stop + 1 - row_write;
    dprintln!("ITERATIONS: {}\n", iterations);

    // Second loop. Updates prices and market (if market is set) in sheet
    // THIS IS ONLY FOR CSGOSKINS.GG AS OF NOW.
    for (i, url) in sheet_urls_quantities.keys().enumerate() {
        if !user.update_prices { break }
        
        let row = excel.row_start_table as usize + i;

        dprintln!("\nROW WRITE: {}", row_write);
        dprintln!("CURR ROW: {}", row);
        dprintln!("ROW STOP: {}\n", row_stop);
        
        if row > row_stop { break }
        if row < row_write { continue }
        if user.ignore_urls.contains(url) { continue }

        let market_price: IndexMap< String, f64> = cs_get_market_prices(
            url, 
            &CSGOSKINS_COOKIES, 
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:136.0) Gecko/20100101 Firefox/136.0",
        ).await?;

        let mut market_price_preferred: IndexMap<&String, &f64> = IndexMap::new();
    
        let price : &f64;
        let market: &String;
        let progress = (i + 1) as f32 / iterations as f32;
        // Have to do this and not use a filter_map cuz this preserves the order of user.prefer_markets
        for marked in user.prefer_markets.iter() {
            if let Some(pris) = market_price.get(marked) { 
                market_price_preferred.insert(marked, pris); 
            }
        }

        if !market_price_preferred.is_empty() {
            if user.percent_threshold != 0 {
                let mut iter = market_price_preferred.iter();
                let (mut best_market, mut best_price) = iter.next().unwrap();
                
                // Finds the correct market and price given percent threshold 
                for (curr_market, curr_price) in iter {
                    let threshold = *best_price * (1.0 - user.percent_threshold as f64 / 100.0);
                            
                    if **curr_price <= threshold {
                        best_market = curr_market;
                        best_price = curr_price;
                    } 
                    else { break }
                } 
                market = best_market;
                price = *best_price;
            }
            // Takes cheapest market out of preferred  
            else { 
                (market, price) = market_price_preferred.get_index(0)
                .ok_or("Market_price_preferred IndexMap is empty!")? 
            } 
        } 
        // Takes cheapest market out of all
        else { 
            (market, price) = market_price.get_index(0)
            .ok_or("Market_price IndexMap is empty! Possible you're flagged by the website and can't fetch prices anymore.")? 
        } 
        
        // Set price
        sheet.get_cell_value_mut( format!("{}{}", &excel.col_price, &row) )
            .set_value_number( *price * convertion_rate);
    

        // market is optional
        if let Some(col_market) = &excel.col_market {
            if col_market.trim() != "" { 
                sheet.get_cell_value_mut( format!("{}{}", col_market, &row) )
                    .set_value_string(market); 
            }
        }
        
        output_text.push_str( 
            &format!(
                "-- ROW -- \n{}\n-- PREFERRED MARKETS -- \n{:?}\n--MARKET-- \n{}\n-- PRICE -- \n{:.2}\n-- URL -- \n{}\n\n",
                &row, 
                market_price_preferred, 
                market, 
                *price * convertion_rate, 
                url
            ) 
        );

        app.set_gui_output_text( output_text.as_str().into() );
        app.set_progress( (i + 1) as f32 / iterations as f32 );
        
        dprintln!("PROGRESS: {}", ( (i + 1) as f32 / iterations as f32) );
        //dprintln!("ALL MARKETS: {:?}", market_price);
        dprintln!("PREFERRED MARKETS: {:?}", market_price_preferred);
        dprintln!("WRITING PRICE TO CELL: {}{}", &excel.col_price, &row);
        dprintln!("MARKET: {}\nPRICE: {:.2} RMB \nURL: {}", market, *price * convertion_rate, url);

        let actual_pause_time = user.pause_time_ms - rand::random_range( 0..= (user.pause_time_ms / 5) ) + rand::random_range( 0..= (user.pause_time_ms / 5) );
        dprintln!("PAUSE TIME: {actual_pause_time}\n");

        // Timeout to not raise any alarms (if statement so last iteration doesnt sleep an extra turn)
        if progress != 1.0 { sleep(Duration::from_millis( actual_pause_time )).await }
    }

    // Last thing to be written to the worksheet. Outside both loops so runs only at the end
    // date is optional
    if let Some(date) = &excel.rowcol_date {
        if !date.trim().is_empty() {
            let time = Local::now()
                .format("%d/%m/%Y %H:%M:%S")
                .to_string();

            sheet.get_cell_value_mut( date.as_str() ) // THIS WILL CRASH IF CELL VALUE IS TOO LARGE!
                .set_value_string(&time);
        }
    }
    // TODO: MAKE THE ABOVE CODE SAFE SO IT DOESNT PANIC MAIN! realized this isnt possible as library doesnt have a way to fetch values and return an option or result :(

    // WRITING TO THE SPREADSHEET WOWIE
    writer::xlsx::write(&book, &excel.path_to_sheet)?;

    output_text.push_str("Finished successfully!");
    app.set_gui_output_text( output_text.as_str().into() );

    dprintln!("wrote to the spreadsheet!");
    Ok(())
}


async fn test_excel(_: &UserInfo, excel: &SheetInfo) -> Result<(), Box<dyn Error>> {
    let mut book: Spreadsheet = umya_spreadsheet::reader::xlsx::read(&excel.path_to_sheet)?;
    let sheet: &mut Worksheet = book.get_sheet_by_name_mut(&excel.sheet_name).ok_or(
        format!("Couldn't instanciate the worksheet of name {}", &excel.sheet_name)
    )?;

    let coord = excel.rowcol_usd_to_x.as_str();

    let datatype = sheet.get_cell_value(coord).get_data_type();
    let value = sheet.get_cell_value(coord).get_raw_value().to_string();
    let number_q = sheet.get_cell_value(coord).get_value_number().unwrap_or( 0.0 );

    dprintln!("FOR COORD: {}: \nDATATYPE: {} \nVALUE: {} \nNUMBER? : {} \n", coord, datatype, value, number_q, );

    let time = Local::now().format("%d/%m/%Y %H:%M:%S");
    dprintln!("time {}", time);
    return Ok(());
}