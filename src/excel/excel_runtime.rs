use std::{collections::HashMap, str::FromStr};

use reqwest::Client;
use strum::IntoEnumIterator;
use umya_spreadsheet::{Spreadsheet, Worksheet};
use serde_json::Value;

use crate::{
    browser::{csfloat, csgotrader, steamcommunity::SteamInventory}, 
    excel::{excel_ops::{get_exceldata, get_spreadsheet, set_spreadsheet}, helpers::{get_exchange_rate, get_market_price, get_steamloginsecure, insert_new_exceldata, update_quantity_exceldata, wrapper_fetch_iteminfo_via_itemprovider_persistent}}, 
    models::{  
        excel::ExcelData, price::{Currencies, Doppler, PricingMode, PricingProvider}, 
        user_sheet::{SheetInfo, UserInfo}, 
        web::{ExtraItemData, ItemInfoProvider, Sites, SteamData}
    },
};


pub async fn run_program(user: UserInfo, excel: SheetInfo) -> Result<(), String> {
    // Path to my main invest: C:\Users\Mikae\OneDrive\Skrivebord\workbook
    let user = &mut user.clone();
    let excel = &mut excel.clone();

    println!("user: {:?}", user);
    println!("sheet: {:?}", excel);

    // -----------------------------------------------------------------------------------------------

    // BIG BRAIN; READ THE EXCEL SPREADSHEET FIRST TO GET ALL THE INFO AND THEN GET PRICES WOWOWO
    
    // Getting the Worksheet from either existing book or new book
    let mut book: Spreadsheet = get_spreadsheet(&excel.path_to_sheet).await
        .unwrap_or_else( |_| {
                excel.sheet_name = None;
                
                let filename = excel.path_to_sheet.as_ref()
                    .map(|p| p.to_str().unwrap_or_else(|| "| Failed PathBuf to_str |" ))
                    .unwrap_or_else(||"| Failed Option<&str> unwrap |")
                    .split("\\")
                    .collect::<Vec<&str>>();

                println!("WARNING: Created a new spreadsheet as one with the path {} didn't exist.", filename[filename.len() - 1]); 
                umya_spreadsheet::new_file() 
            }
        );
    let sheet: &mut Worksheet = {
        if let Some(sn) = &excel.sheet_name { 
            if let Some(buk) = book.get_sheet_by_name_mut(sn) { buk } 
            else {
                println!("WARNING: Automatically fetched first sheet in spreadsheet because {} was not found.", sn);
                book.get_sheet_mut(&0).ok_or_else(|| format!(
                    "Failed to get the first sheet in the spreadsheet with path: \n{:?}", excel.path_to_sheet.as_ref())
                )?
            }  
        } else { book.get_sheet_mut(&0).ok_or("Failed to get first sheet provided by new_file creation.")? }
    };

    // Client for fetch_more_iteminfo
    let mut iteminfo_client_base = match &user.iteminfo_provider {
        ItemInfoProvider::Csfloat => { csfloat::new_extra_iteminfo_client() },
        ItemInfoProvider::Csgotrader => { csgotrader::new_extra_iteminfo_client() }
        ItemInfoProvider::None => { Client::new() }
    };
    
    let iteminfo_client: &mut Client = &mut iteminfo_client_base;

    // -----------------------------------------------------------------------------------------------

    let mut exceldata: Vec<ExcelData> = get_exceldata(sheet, &excel, user.ignore_already_sold).await?;
    let exceldata_initial_length: usize = exceldata.len();
    
    // println!("Data gotten from excel spreadsheet: \n{:#?}", exceldata);
    // if !exceldata.is_empty() { return Ok(()) }

    //  exceldata_old_len er her fordi jeg har endret måte å oppdatere prisene i spreadsheet'n på.
    //  Nå, hvis et item fra steam ikke er i spreadsheetn allerede, så oppdateres spreadsheetn med price, quantity,
    //  phase og inspect link. exceldata_old_len skal være til når resten av itemsene skal oppdateres i pris,
    //  da stopper itereringen ved exceldata_old_len i stedet for å hente prisen til item'ene som er nylig lagt
    //  til og derfor også oppdatert allerede.

    // -----------------------------------------------------------------------------------------------
    
    let steamcookie: Option<String> = get_steamloginsecure(&user.steamloginsecure);

    let rate = get_exchange_rate(&user.usd_to_x, &excel.rowcol_usd_to_x, sheet).await?;
    
    let sm_inv: Option<SteamInventory> = {
        if user.fetch_steam { Some( SteamInventory::init(user.steamid, 730, steamcookie).await? ) } 
        else { None }
    };

    let cs_inv: Option<Vec<SteamData>> = 
        if let Some(inv) = &sm_inv { Some( inv.get_steam_items(user.group_simular_items, true)? ) }
        else { None };

    // -----------------------------------------------------------------------------------------------

    let markets_to_check: &Vec<Sites> = &user.prefer_markets.take()
        .unwrap_or_else(|| Sites::iter().collect::<Vec<Sites>>() );

    let all_market_prices: HashMap<String, Value> = {
        let mut amp: HashMap<String, Value> = HashMap::new();
        for market in markets_to_check {    

            let market_prices = match &user.pricing_provider {
                PricingProvider::Csgoskins => { csgotrader::get_market_data( market ).await? }, // IF I IMPLEMENT CSGOSKINS IN THE FUTURE
                PricingProvider::Csgotrader => { csgotrader::get_market_data( market ).await? },
            };

            amp.insert(market.to_string(), market_prices);
        }
        amp
    };
    
    // -----------------------------------------------------------------------------------------------
    // Inserting and/or updating quantity + adding prices for newly inserted items | .flatten() only runs the loop if it is Some()

    for steamdata in cs_inv.iter().flatten() {     

        // println!("\nCURRENT STEAMDATA {:#?}", steamdata);

        if user.group_simular_items {
            assert!(excel.col_quantity.is_some());

            match exceldata.iter_mut().enumerate().find( |(_, e)| e.name == steamdata.name ) { 
                Some((index, data)) => {
                    
                    // Skip item if item is in ignore market names
                    if let Some(ignore) = &user.ingore_steam_names { 
                        if ignore.iter().any(|n| data.name == *n.trim() ) { continue; }
                    }

                    let row_in_excel: usize = index + excel.row_start_write_in_table as usize;
                    
                    // if exceldatas data has phase info AND user wants to fetch more iteminfo AND cs inventory's steamdata has an inspect link,
                    // don't update quantity and jump to next iteration of cs inv. Instead execute the logic underneath this match statement
                    if data.phase.is_some() && user.iteminfo_provider != ItemInfoProvider::None && steamdata.inspect_link.is_some() {}
                    else { 
                        update_quantity_exceldata(
                            &steamdata, 
                            &excel.col_quantity, 
                            data, 
                            row_in_excel, 
                            sheet
                        ); 

                        // If quantity is more than 1, remove data in float, pattern and inspect_link if its set
                        if data.quantity > Some(1) {
                            if let Some(col_float) = &excel.col_float {
                                let cell = format!("{}{}", col_float, row_in_excel);
                                sheet.get_cell_value_mut(cell).set_value_string("");
                            }
                            if let Some(col_pattern) = &excel.col_pattern {
                                let cell = format!("{}{}", col_pattern, row_in_excel);
                                sheet.get_cell_value_mut(cell).set_value_string("");
                            }
                            if let Some(col_inspect) = &excel.col_inspect_link {
                                let cell = format!("{}{}", col_inspect, row_in_excel);
                                sheet.get_cell_value_mut(cell).set_value_string("");
                            }
                        }

                        continue;
                    }
                },
                None => {

                    // DO NOT INSERT NEW STUFF IF THERE IS A LIMITER ON WHERE TO STOP WRITING
                    // acts on the outer loop "for steamdata in cs_inv.iter()"
                    if excel.row_stop_write_in_table.is_some() { continue; } 

                    let row_in_excel: usize = exceldata.len() + excel.row_start_write_in_table as usize;

                    let extra_itemdata: Option<ExtraItemData> = if let Some(quant) = steamdata.quantity {
                        if quant == 1 || steamdata.name.contains( " doppler ") {
                            wrapper_fetch_iteminfo_via_itemprovider_persistent(
                                iteminfo_client, 
                                &user.iteminfo_provider, 
                                &excel.col_inspect_link, 
                                user.pause_time_ms, 
                                &steamdata
                            ).await?
                        } else { None }
                    } else { None };

                    exceldata.push( 
                        insert_new_exceldata(
                            &user, &excel, 
                            &steamdata, 
                            &extra_itemdata,
                            &markets_to_check, 
                            &all_market_prices, 
                            rate, row_in_excel, 
                            sheet
                        ).await? 
                    );
                    continue; 

                }
            }
            assert!(excel.col_inspect_link.is_some());
            assert!(steamdata.inspect_link.is_some());
            assert!(user.iteminfo_provider != ItemInfoProvider::None);

            // Only reached when exceldatas name is the same as steamdatas name AND 
            // exceldatas phase is something AND user wants to fetch more iteminfo AND 
            // steamdatas inspect link is something
            let extra_itemdata: ExtraItemData = wrapper_fetch_iteminfo_via_itemprovider_persistent(
                iteminfo_client, 
                &user.iteminfo_provider, 
                &excel.col_inspect_link, 
                user.pause_time_ms, 
                &steamdata
            ).await?.ok_or_else(|| "group_simular_items' path for dopplers failed WHAT")?;

            let phase: &Option<String> = &extra_itemdata.phase
                .as_ref()
                .and_then( |p| Some( p.as_str() ) )
                .map( str::to_owned );

            match exceldata.iter_mut().enumerate().find( |(_, e)| e.name == steamdata.name && e.phase == *phase ) {
                Some((index, data)) => {
                    let row_in_excel: usize = index + excel.row_start_write_in_table as usize;

                    update_quantity_exceldata(
                        &steamdata, 
                        &excel.col_quantity, 
                        data, 
                        row_in_excel, 
                        sheet
                    ); 
                },
                None => {
                    
                    // DO NOT INSERT NEW STUFF IF THERE IS A LIMITER ON WHERE TO STOP WRITING
                    // acts on the outer loop "for steamdata in cs_inv.iter()"
                    if excel.row_stop_write_in_table.is_some() { continue; } 

                    let row_in_excel: usize = exceldata.len() + excel.row_start_write_in_table as usize;

                    exceldata.push( 
                        insert_new_exceldata(
                            &user, &excel,
                            &steamdata,
                            &Some(extra_itemdata),
                            &markets_to_check,
                            &all_market_prices,
                            rate, row_in_excel, 
                            sheet
                        ).await? 

                    );
                }
            }
        } 
        
        // If not group_simular_items     
        else {
            
            // DO NOT INSERT NEW STUFF IF THERE IS A LIMITER ON WHERE TO STOP WRITING
            if excel.row_stop_write_in_table.is_some() { break; }

            if exceldata.iter().find(|e| e.asset_id == Some(steamdata.asset_id) && e.name == steamdata.name).is_none() {
                let row_in_excel: usize = exceldata.len() + excel.row_start_write_in_table as usize;

                let extra_itemdata: Option<ExtraItemData> = wrapper_fetch_iteminfo_via_itemprovider_persistent(
                    iteminfo_client, 
                    &user.iteminfo_provider, 
                    &excel.col_inspect_link,
                    user.pause_time_ms, 
                    &steamdata
                ).await?;

                exceldata.push( 
                    insert_new_exceldata(
                        &user, &excel,
                        &steamdata,
                        &extra_itemdata,
                        &markets_to_check,
                        &all_market_prices,
                        rate, row_in_excel, 
                        sheet
                    ).await? 
                );
            }
        }
    }

    // Second iteration - updates the prices of all the items other than the 
    // one(s) inserted into the spreadsheet during the first iteration.
    for (i, data) in exceldata.iter().enumerate() {
        if !user.fetch_prices { break }
        if i == exceldata_initial_length { break }

        if data.sold.is_some() { continue; }
        if let Some(ignore) = &user.ingore_steam_names {
            let mut pls_ingore = false;
            for ignore_steam_name in ignore { 
                if data.name == *ignore_steam_name { pls_ingore = true } 
            }
            if pls_ingore { continue; }
        }

        let row_in_excel = i + excel.row_start_write_in_table as usize;
        if let Some(stop_write) = excel.row_stop_write_in_table {
            if row_in_excel >= stop_write as usize { break }
        }
        
        let cell_price = format!("{}{}", excel.col_price, row_in_excel);

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
            &data.phase, 
            &doppler
        )?;

        if let Some(pris) = price {
            sheet.get_cell_value_mut(cell_price).set_value_number(pris);
        }

        if let Some(marked) = market {
            if let Some(col_market) = &excel.col_market {
                let cell_market = format!("{}{}", col_market, row_in_excel);
                sheet.get_cell_value_mut(cell_market).set_value_string(marked);
            }
        }
    }

    if let Some(cell_date) = &excel.rowcol_date {
        sheet.get_cell_value_mut( cell_date.as_ref() )
            .set_value_string( 
                chrono::Local::now()
                    .format("%d/%m/%Y %H:%M:%S")
                    .to_string() 
            );
    }

    set_spreadsheet(&excel.path_to_sheet, book).await
        .map_err(|e| format!("Couldnt write to spreadsheet! : {}", e))?;

    println!("STEAMDATA: \n{:#?}\n", &cs_inv);
    println!("EXCELDATA: \n{:#?}\n", &exceldata);

    if let Some(inv) = &sm_inv {
        println!("Asset length: {}", inv.get_assets_length());
        println!("Inventory length: {}", inv.get_total_inventory_length() );
    };

    println!("Finished!");

    Ok(())
}

// -------------------------------------------------------------------------------------------

pub fn is_user_input_valid(user: &UserInfo, excel: &SheetInfo) -> Result<(), String> {
    if user.iteminfo_provider == ItemInfoProvider::None {
        println!("WARNING: Pricing for doppler phases will not be accurate with fetch_more_iteminfo off.")
    }

    if user.iteminfo_provider == ItemInfoProvider::None && excel.col_inspect_link.is_some() {
        println!("WARNING: col_inspect_link is not defined so you will not be able to fetch_more_iteminfo (float, doppler phase, pattern, price of doppler).")
    }
    
    // --------------------

    if excel.path_to_sheet.is_some() { 
        if excel.sheet_name.is_none() {
            return Err( String::from( "Sheet name can't be nothing if path to sheet is given." ) )
        }
    }

    if excel.col_inspect_link.is_none() {
        if excel.col_quantity.is_none(){ return Err( String::from( "Quantity can't be empty when no inspect link column is given." ) ) }
        if excel.col_float.is_some()   { return Err( String::from( "Column for float given but no column for inspect link."   ) ) }
        if excel.col_phase.is_some()   { return Err( String::from( "Column for phase given but no column for inspect link."   ) ) }
        if excel.col_pattern.is_some() { return Err( String::from( "Column for pattern given but no column for inspect link." ) ) }
    }

    if user.iteminfo_provider != ItemInfoProvider::None && excel.col_inspect_link.is_some() && excel.col_phase.is_none() {
        return Err( String::from( "Phase of doppler knives will not be pricechecked correctly when reading over the spreadsheet in the future becuase col_phase is not set!" ))
    }

    if user.pause_time_ms < 1000 || user.pause_time_ms > 10000 {
        return Err( String::from("pause_time_ms is only allowed to be in range of 1000 (2 seconds) - 10000 (10 seconds).") )
    }

    if excel.col_quantity.is_none() && user.group_simular_items {
        return Err( String::from("col_quantity can't be None if you want to group simular items!") )
    }

    if excel.col_asset_id.is_none() && !user.group_simular_items {
        return Err( String::from("col_asset_id can't be None if you don't want to group simular items!") )
    }

    if excel.rowcol_usd_to_x.is_some() && user.usd_to_x != Currencies::None {
        return Err( String::from("rowcol_usd_to_x can't be something if usd_to_x is set as a currency!") )
    }

    if user.pricing_mode == PricingMode::Hierarchical && user.percent_threshold == 0 {
        return Err( String::from("pricing_mode can't be Hierarchical if percent_threshold is None!") )
    }

    Ok(())
}
