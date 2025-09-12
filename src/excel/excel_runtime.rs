use std::str::FromStr;

use reqwest::Client;
use strum::IntoEnumIterator;
use umya_spreadsheet::{Spreadsheet, Worksheet};
use serde_json::Value;
use ahash::HashMap;

use iced::task::{Straw, sipper};

use crate::{
    browser::{csfloat, csgotrader, steamcommunity::SteamInventory}, dprintln, excel::{excel_ops::{get_exceldata, get_spreadsheet, set_spreadsheet}, helpers::{clear_extra_iteminfo_given_quantity, get_cached_markets_data, get_exchange_rate, get_market_price, get_steamloginsecure, insert_new_exceldata, insert_number_in_sheet, insert_string_in_sheet, update_quantity_exceldata, wrapper_fetch_iteminfo_via_itemprovider_persistent}}, gui::{ice::Progress, templates_n_methods::IsEnglishAlphabetic}, models::{  
        excel::ExcelData, price::{Doppler, PricingMode}, 
        user_sheet::{SheetInfo, UserInfo}, 
        web::{ExtraItemData, ItemInfoProvider, Sites, SteamData}
    }
};

pub fn run_program(
    mut user: UserInfo, 
    mut excel: SheetInfo, 
) -> impl Straw<(), Progress, String> {
    

    sipper(async move |mut progress| {

        progress.send( Progress { message: "Running main program\n".to_owned(), percent: 0.0 }).await;

        if user.fetch_prices && user.iteminfo_provider != ItemInfoProvider::Steam && excel.col_inspect_link.is_some() {
            progress.send( Progress { 
                message: String::from("Will Fetch additional iteminfo using 3rd party API. This makes doppler prices accurate.\n"), 
                percent: 0.0
            }).await;
        }
        
        // Client for fetch_more_iteminfo
        let mut iteminfo_client_base = match &user.iteminfo_provider {
            ItemInfoProvider::Csfloat => { csfloat::new_extra_iteminfo_client() },
            ItemInfoProvider::Csgotrader => { csgotrader::new_extra_iteminfo_client() },
            ItemInfoProvider::Steam => { Client::new() }, // Not needed for steam
        };
        
        let iteminfo_client: &mut Client = &mut iteminfo_client_base;

        // -----------------------------------------------------------------------------------------------

        let steamcookie: Option<String> = if user.fetch_steam { get_steamloginsecure(&user.steamloginsecure) } else { None };

        let sm_inv: Option<SteamInventory> = {
            if user.fetch_steam { Some( SteamInventory::init(user.steamid, 730, steamcookie).await? ) } 
            else { None }
        };

        let cs_inv: Option<Vec<SteamData>> = 
            if let Some(inv) = &sm_inv { Some( inv.get_steam_items(user.group_simular_items, true)? ) }
            else { None };

        // -----------------------------------------------------------------------------------------------

        let markets_to_check: Vec<Sites> = user.prefer_markets.take()
            .unwrap_or_else(|| Sites::iter().collect::<Vec<Sites>>() );

        let all_market_prices: HashMap<Sites, Value> = get_cached_markets_data(&markets_to_check, &user.pricing_provider).await?;

        if cs_inv.is_some() {
            progress.send( Progress { 
                message: String::from("Reading data from cs inventory and applying it to spreadsheet...\n"), 
                percent: 0.0 }
            ).await;
        }
        let cs_inv_len = cs_inv.as_ref().map(|i| i.len()).unwrap_or(0);

        // -----------------------------------------------------------------------------------------------

        // BIG BRAIN; READ THE EXCEL SPREADSHEET FIRST TO GET ALL THE INFO AND THEN GET PRICES WOWOWO
        
        // Getting the Worksheet from either existing book or new book
        let mut book: Spreadsheet = get_spreadsheet(&mut excel.path_to_sheet, &mut excel.sheet_name, &mut progress).await?;

        let sheet: &mut Worksheet = {
            if let Some(sn) = &excel.sheet_name { 
                if let Some(buk) = book.get_sheet_by_name_mut(sn) { buk } 
                else {
                    dprintln!("WARNING: Automatically fetched first sheet in spreadsheet because {} was not found.", sn);

                    progress.send(Progress { 
                        message: format!("WARNING: Automatically fetched first sheet in spreadsheet because {} was not found.\n", sn), 
                        percent: 0.0 
                    }).await;

                    book.get_sheet_mut(&0).ok_or_else(|| format!(
                        "Failed to get the first sheet in the spreadsheet with path: \n{:?}", excel.path_to_sheet.as_ref())
                    )?
                }  
            } else { book.get_sheet_mut(&0).ok_or("Failed to get first sheet provided by new file creation.")? }
        };

        let rate = get_exchange_rate(&user.usd_to_x, &excel.rowcol_usd_to_x, sheet).await?;

        // -----------------------------------------------------------------------------------------------

        let mut exceldata: Vec<ExcelData> = get_exceldata(sheet, &excel, user.ignore_already_sold).await?;
        let exceldata_initial_length: usize = exceldata.len();

        progress.send( Progress { 
            message: if exceldata.is_empty() {String::from("Read empty excel spreadsheet.\n")} else {format!("Read spreadsheet. First: {} | Last: {}\n", exceldata[0].name, exceldata[exceldata.len() - 1].name)}, 
            percent: 0.0 }
        ).await;

        //  exceldata_old_len er her fordi jeg har endret måte å oppdatere prisene i spreadsheet'n på.
        //  Nå, hvis et item fra steam ikke er i spreadsheetn allerede, så oppdateres spreadsheetn med price, quantity,
        //  phase og inspect link. exceldata_old_len skal være til når resten av itemsene skal oppdateres i pris,
        //  da stopper itereringen ved exceldata_old_len i stedet for å hente prisen til item'ene som er nylig lagt
        //  til og derfor også oppdatert allerede.

        // -----------------------------------------------------------------------------------------------
        
        // Inserting and/or updating quantity + adding prices for newly inserted items | .flatten() only runs the loop if it is Some()
        for (i, steamdata) in cs_inv.iter().flatten().enumerate() { 
            
            progress.send( Progress { 
                message: if user.group_simular_items { 
                    format!(
                        "NAME: {} | QUANTITY: {} | HAS INSPECTLINK?: {}\n", 
                        steamdata.name, 
                        steamdata.quantity.unwrap_or(0), 
                        if steamdata.inspect_link.is_some() {"YES"} else {"NO"}
                    )
                } else {
                    format!(
                        "NAME: {} | HAS INSPECTLINK?: {} | ASSETID: {}\n", 
                        steamdata.name, 
                        if steamdata.inspect_link.is_some() {"YES"} else {"NO"}, 
                        steamdata.asset_id
                    )
                }, 
                percent: (i as f32 / cs_inv_len as f32 * 99.0) 
            } ).await;
        
            if user.group_simular_items {
                match exceldata.iter_mut().enumerate().find( |(_, e)| e.name == steamdata.name ) { 
                    Some((index, data)) => {

                        // Skip item if item is in ignore market names
                        if let Some(ignore) = &user.ingore_steam_names 
                        && ignore.iter().any(|n| data.name == *n.trim()) { continue; }
                        
                        let row_in_excel: usize = index + excel.row_start_write_in_table as usize;

                        // if exceldatas data has phase info AND user wants to fetch more iteminfo AND cs inventory's steamdata has an inspect link,
                        // don't update quantity and jump to next iteration of cs inv. Instead execute the logic underneath match statement
                        if data.phase.is_some() // data.phase being Some means excel.col_phase has to be Some aswell
                        && user.iteminfo_provider != ItemInfoProvider::Steam 
                        && steamdata.inspect_link.is_some() {   
                            // Only path that does not end in a 'continue; keyword. Executes the match statement below this match. 
                            // This is needed because you can have two of the same knife, but it can have different phases.
                            // Doing the check here would not cover that possibility so it has to be its´ own loop.
                        }
                        
                        // FOR CASES WHERE DOPPLER GOT FETCHED FIRST USING STEAM THEN FETCHED LATER USING 3RD PARTY API
                        else if data.phase.is_none() 
                        && user.iteminfo_provider != ItemInfoProvider::Steam 
                        && steamdata.inspect_link.is_some() 
                        && data.quantity == Some(1)
                        && let Some(col_phase) = &excel.col_phase
                        && data.name.to_lowercase().contains(" doppler") 
                        {
                            let iteminfo: ExtraItemData = wrapper_fetch_iteminfo_via_itemprovider_persistent(
                                iteminfo_client, 
                                &user.iteminfo_provider, 
                                &excel.col_inspect_link, 
                                user.pause_time_ms, 
                                steamdata, 
                                &mut progress
                            ).await?.ok_or("Iteminfo fetched is None when that shouldnt be possible.".to_string())?;

                            let (market, price) = get_market_price(
                                &user, 
                                &markets_to_check, 
                                &all_market_prices,
                                rate, 
                                &steamdata.name, 
                                &iteminfo.phase, 
                                &mut progress
                            ).await?;

                            if let Some(phase) = &iteminfo.phase { insert_string_in_sheet(sheet, col_phase, row_in_excel, phase.as_str()); }
                            if let Some(price) = price { insert_number_in_sheet(sheet, &excel.col_price, row_in_excel, price); }
                            if let Some(market) = market && let Some(col_market) = &excel.col_market { insert_string_in_sheet(sheet, col_market, row_in_excel, market); }

                            continue;
                        }
                        // "Base case" after hyper-spesific clauses above
                        else { 
                            update_quantity_exceldata(
                                steamdata, 
                                &excel.col_quantity, 
                                data, 
                                row_in_excel, 
                                sheet,
                                &mut progress
                            ).await; 

                            // If quantity is more than 1, remove data in float, pattern and inspect_link if its set
                            clear_extra_iteminfo_given_quantity(
                                sheet, 
                                data.quantity, 
                                row_in_excel,
                                (excel.col_float.as_deref(), excel.col_pattern.as_deref(), excel.col_inspect_link.as_deref()), 
                                
                            );

                            continue;
                        }
                    },
                    None => {

                        // DO NOT INSERT NEW STUFF IF THERE IS A LIMITER ON WHERE TO STOP WRITING
                        // acts on the outer loop "for steamdata in cs_inv.iter().flatten().enumerate()"
                        if excel.row_stop_write_in_table.is_some() { continue; } 

                        let row_in_excel: usize = exceldata.len() + excel.row_start_write_in_table as usize;

                        let extra_itemdata: Option<ExtraItemData> = 
                            if steamdata.quantity == Some(1) || steamdata.name.to_lowercase().contains( " doppler") {
                                // Min retarda ass bygde extra iteminfo checken inn i wrapper funksjonen så trust at hvis IteminfoProvider er Steam så blir denne None
                                wrapper_fetch_iteminfo_via_itemprovider_persistent(
                                    iteminfo_client,
                                    &user.iteminfo_provider, 
                                    &excel.col_inspect_link, 
                                    user.pause_time_ms, 
                                    steamdata,
                                    &mut progress
                                ).await?
                            } 
                            else { None };

                        exceldata.push( 
                            insert_new_exceldata(
                                &user, &excel, 
                                steamdata, 
                                &extra_itemdata,
                                &markets_to_check, 
                                &all_market_prices, 
                                rate, row_in_excel, 
                                sheet,
                                &mut progress
                            ).await? 
                        );
                        continue; 

                    }
                }

                // ONLY REACHES HERE IF ITEM HAS PHASE, ITEMINFO PROVIDER IS NOT STEAM AND HAS INSPECT LINK.

                debug_assert!(excel.col_inspect_link.is_some());
                debug_assert!(steamdata.inspect_link.is_some());
                debug_assert!(user.iteminfo_provider != ItemInfoProvider::Steam);

                // Only reached when exceldatas name is the same as steamdatas name AND 
                // exceldatas phase is something AND user wants to fetch more iteminfo AND 
                // steamdatas inspect link is something
                let extra_itemdata: ExtraItemData = wrapper_fetch_iteminfo_via_itemprovider_persistent(
                    iteminfo_client, 
                    &user.iteminfo_provider, 
                    &excel.col_inspect_link, 
                    user.pause_time_ms, 
                    steamdata,
                    &mut progress
                ).await?.ok_or("group_simular_items' path for dopplers failed WHAT")?;

                let phase: &Option<String> = &extra_itemdata.phase.as_ref()
                    .map(|p| p.as_str().to_string());

                match exceldata.iter_mut().enumerate().find( |(_, e)| e.name == steamdata.name && e.phase == *phase ) {
                    Some((index, data)) => {
                        let row_in_excel: usize = index + excel.row_start_write_in_table as usize;

                        update_quantity_exceldata(
                            steamdata, 
                            &excel.col_quantity, 
                            data, 
                            row_in_excel, 
                            sheet,
                            &mut progress
                        ).await; 
                    },
                    None => {

                        // DO NOT INSERT NEW STUFF IF THERE IS A LIMITER ON WHERE TO STOP WRITING
                        // acts on the outer loop "for steamdata in cs_inv.iter()"
                        if excel.row_stop_write_in_table.is_some() { continue; } 

                        let row_in_excel: usize = exceldata.len() + excel.row_start_write_in_table as usize;

                        exceldata.push( 
                            insert_new_exceldata(
                                &user, 
                                &excel,
                                steamdata,
                                &Some(extra_itemdata),
                                &markets_to_check,
                                &all_market_prices,
                                rate, row_in_excel, 
                                sheet,
                                &mut progress
                            ).await? 
                        );
                    }
                }
            } 

            // If not group_simular_items     
            else {

                // DO NOT INSERT NEW STUFF IF THERE IS A LIMITER ON WHERE TO STOP WRITING
                if excel.row_stop_write_in_table.is_some() { break; }

                match exceldata.iter().enumerate().find(|(_, e)| e.asset_id == Some(steamdata.asset_id) && e.name == steamdata.name) {
                    Some((index, data)) => {
                        
                        if data.phase.is_none()
                        && user.iteminfo_provider != ItemInfoProvider::Steam
                        && steamdata.inspect_link.is_some()
                        && let Some(col_phase) = &excel.col_phase
                        && data.name.to_lowercase().contains(" doppler")
                        {
                            let row_in_excel: usize = index + excel.row_start_write_in_table as usize;

                            let iteminfo: ExtraItemData = wrapper_fetch_iteminfo_via_itemprovider_persistent(
                                iteminfo_client, 
                                &user.iteminfo_provider, 
                                &excel.col_inspect_link, 
                                user.pause_time_ms, 
                                steamdata, 
                                &mut progress
                            ).await?.ok_or("Iteminfo fetched is None when that shouldnt be possible.".to_string())?;

                            let (market, price) = get_market_price(
                                &user, 
                                &markets_to_check, 
                                &all_market_prices,
                                rate, 
                                &steamdata.name, 
                                &iteminfo.phase, 
                                &mut progress
                            ).await?;

                            if let Some(phase) = &iteminfo.phase { insert_string_in_sheet(sheet, col_phase, row_in_excel, phase.as_str()); }
                            if let Some(price) = price { insert_number_in_sheet(sheet, &excel.col_price, row_in_excel, price); }
                            if let Some(market) = market && let Some(col_market) = &excel.col_market { insert_string_in_sheet(sheet, col_market, row_in_excel, market); }
                        } 
                    }
                    None => {
                        let row_in_excel: usize = exceldata.len() + excel.row_start_write_in_table as usize;

                        let extra_itemdata: Option<ExtraItemData> = wrapper_fetch_iteminfo_via_itemprovider_persistent(
                            iteminfo_client, 
                            &user.iteminfo_provider, 
                            &excel.col_inspect_link,
                            user.pause_time_ms, 
                            steamdata,
                            &mut progress
                        ).await?;

                        exceldata.push( 
                            insert_new_exceldata(
                                &user, &excel,
                                steamdata,
                                &extra_itemdata,
                                &markets_to_check,
                                &all_market_prices,
                                rate, row_in_excel, 
                                sheet,
                                &mut progress
                            ).await? 
                        );
                    }
                    
                }
            }
        }

        if user.fetch_prices {
            progress.send( Progress { 
                message: String::from("Updating prices of old items in spreadsheet...\n"), 
                percent: 99.0
            }).await;
        }
        
        // Second iteration - updates the prices of all the items other than the
        // one(s) inserted into the spreadsheet during the first iteration.
        for (i, data) in exceldata.iter().enumerate() {
            if !user.fetch_prices { break }
            if i == exceldata_initial_length { break }

            if data.sold.is_some() && user.ignore_already_sold { continue; }
            
            if let Some(ignore) = &user.ingore_steam_names {
                let mut pls_ingore = false;
                for ignore_steam_name in ignore { 
                    if data.name == *ignore_steam_name { pls_ingore = true } 
                }
                if pls_ingore { continue; }
            }

            let row_in_excel = i + excel.row_start_write_in_table as usize;
            if let Some(stop_write) = excel.row_stop_write_in_table && row_in_excel >= stop_write as usize { 
                break 
            }

            let doppler: Option<Doppler> = {
                if let Some(phase) = &data.phase {
                    Some(Doppler::from_str(phase)?)
                } else { None }
            };

            let (market, price): (Option<String>, Option<f64>) = get_market_price(
                &user, 
                &markets_to_check, 
                &all_market_prices, 
                rate, 
                &data.name, 
                &doppler,
                &mut progress
            ).await?;

            if let Some(pris) = price { insert_number_in_sheet(sheet, &excel.col_price, row_in_excel, pris); }
            if let Some(marked) = market && let Some(col_market) = &excel.col_market { insert_string_in_sheet(sheet, col_market, row_in_excel, &marked); }
        }

        let finishtime = chrono::Local::now()
            .format("%d/%m/%Y %H:%M:%S")
            .to_string();

        if let Some(cell_date) = &excel.rowcol_date {
            sheet.get_cell_value_mut( cell_date.as_ref() )
                .set_value_string( &finishtime );
        }

        // Writes the modified data to the spreadsheet
        set_spreadsheet(&excel.path_to_sheet, book).await
            .map_err(|e| format!("Couldnt write to spreadsheet! : {}", e))?;

        progress.send( Progress { 
            message: format!("End time: {}\n", finishtime), 
            percent: 100.0
        }).await;

        if let Some(inv) = &sm_inv {
            progress.send( Progress { 
                message: format!("Asset length: {}\nInventory length: {}\n", inv.get_assets_length(),  inv.get_total_inventory_length()), 
                percent: 100.0
            }).await;
        };
        Ok(())
    })

    
}

// -------------------------------------------------------------------------------------------

pub fn is_user_input_valid(user: &UserInfo, excel: &SheetInfo) -> Result<(), String> {
    if user.iteminfo_provider == ItemInfoProvider::Steam {
        dprintln!("WARNING: Pricing for doppler phases will not be accurate with Steam as ItemInfoProvider.")
    }

    if user.iteminfo_provider == ItemInfoProvider::Steam && excel.col_inspect_link.is_some() {
        dprintln!("WARNING: col_inspect_link is not defined so you will not be able to fetch_more_iteminfo (float, doppler phase, pattern, price of doppler).")
    }
    
    // --------------------

    if excel.path_to_sheet.is_some() && excel.sheet_name.is_none() {
        return Err( String::from( "Sheet name can't be nothing if path to sheet is given." ) )
    }

    if excel.col_inspect_link.is_none() {
        if excel.col_quantity.is_none(){ return Err( String::from( "Quantity can't be empty when no inspect link column is given." ) ) }
        if excel.col_float.is_some()   { return Err( String::from( "Column for float given but no column for inspect link."   ) ) }
        if excel.col_phase.is_some()   { return Err( String::from( "Column for phase given but no column for inspect link."   ) ) }
        if excel.col_pattern.is_some() { return Err( String::from( "Column for pattern given but no column for inspect link." ) ) }
    }

    if user.iteminfo_provider != ItemInfoProvider::Steam && excel.col_inspect_link.is_some() && excel.col_phase.is_none() {
        return Err( String::from( "Phase of doppler knives will not be pricechecked correctly when reading over the spreadsheet in the future becuase col_phase is not set!" ))
    }

    if user.pause_time_ms < 1000 || user.pause_time_ms > 2500 {
        return Err( String::from("pause_time_ms is only allowed to be in range of 1000 (1 second) - 2500 (2.5 seconds).") )
    }

    if excel.col_quantity.is_none() && user.group_simular_items {
        return Err( String::from("col_quantity can't be None if you want to group simular items!") )
    }

    if excel.col_asset_id.is_none() && !user.group_simular_items {
        return Err( String::from("col_asset_id can't be None if you don't want to group simular items!") )
    }

    // Checked in the update logic of the Iced application
    // if excel.rowcol_usd_to_x.is_some() && user.usd_to_x != Currencies::None {
        // return Err( String::from("rowcol_usd_to_x can't be something if usd_to_x is set as a currency!") )
    // }

    if user.pricing_mode == PricingMode::Hierarchical && user.percent_threshold == 0 {
        return Err( String::from("pricing_mode can't be Hierarchical if percent_threshold is None!") )
    }

    if user.steamid == 0 && user.steamid.checked_ilog10().unwrap_or(0) > 17 {
        return Err(String::from("steamid64 is invalid!"));
    }

    if excel.row_start_write_in_table == 0 {
        return Err(String::from("row_start_write_in_table is invalid!"))
    }

    if excel.col_price.is_empty() && user.fetch_prices {
        return Err(String::from("col_price has to be given if you want to fetch prices!"))
    }

    if let Some(date) = &excel.rowcol_date && !valid_cell_check(date) { 
        return Err( String::from("format of cell date is not valid!") )
    }

    if let Some(utx) = &excel.rowcol_usd_to_x && !valid_cell_check(utx) { 
        return Err( String::from("format of cell usd_to_x is not valid!") )
    }

    if let Some(stop) = excel.row_stop_write_in_table && excel.row_start_write_in_table < stop { 
        return Err( String::from("Start write can't be less than stop write!")) 
    }

    let mut all_excel: Vec<String> = Vec::from([excel.col_price.to_string(), excel.col_steam_name.to_string()]);
    if let Some(x) = &excel.col_asset_id { all_excel.push( x.to_string() ); }
    if let Some(x) = &excel.col_csgoskins_link { all_excel.push( x.to_string() ); }
    if let Some(x) = &excel.col_float { all_excel.push( x.to_string() ); }
    if let Some(x) = &excel.col_gun_sticker_case { all_excel.push( x.to_string() ); }
    if let Some(x) = &excel.col_inspect_link { all_excel.push( x.to_string() ); }
    if let Some(x) = &excel.col_market { all_excel.push( x.to_string() ); }
    if let Some(x) = &excel.col_pattern { all_excel.push( x.to_string() ); }
    if let Some(x) = &excel.col_phase { all_excel.push( x.to_string() ); }
    if let Some(x) = &excel.col_quantity { all_excel.push( x.to_string() ); }
    if let Some(x) = &excel.col_skin_name { all_excel.push( x.to_string() ); }
    if let Some(x) = &excel.col_sold { all_excel.push( x.to_string() ); }
    if let Some(x) = &excel.col_wear { all_excel.push( x.to_string() ); }
    all_excel.sort();
    
    if let Some(w) = all_excel.windows(2).find(|w| w[0] == w[1]) {
        return Err(
            format!("The same column is referenced two or more times: '{}'",w[0])
    );
}

    Ok(())
}

fn valid_cell_check(s: &str) -> bool {
    let mut signature: Vec<char> = Vec::with_capacity( s.len() );
    let valid_signatures: Vec<&str> = Vec::from(["an", "$an", "$a$n", "a$n"]);

    for c in s.chars() {
        if c == '$' { signature.push(c); continue; }

        let letter: char = {
            if c.is_english_alphabetic() {'a'}
            else if c.is_ascii_digit() {'n'}
            else {'x'}
        };

        if !signature.is_empty() && signature[signature.len() - 1] != letter { signature.push(letter) }
        else if signature.is_empty() { signature.push(letter) }
    }
    let final_signature = signature.iter().collect::<String>();

    dprintln!("Sign: {}", final_signature);
    
    valid_signatures.contains(&final_signature.as_str())
}