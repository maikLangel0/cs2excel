use std::{str::FromStr, time::Duration};

use reqwest::Client;
use strum::IntoEnumIterator;
use umya_spreadsheet::{Worksheet, Spreadsheet};
use serde_json::Value;
use ahash::{HashMap};
use indexmap::IndexSet;

use iced::{task::{Straw, sipper}};

use crate::{
    browser::{csfloat, csgotrader, steamcommunity::SteamInventory}, dprintln, excel::{
        excel_ops::{get_exceldata, get_spreadsheet, set_spreadsheet}, helpers::{
            IcedProgressSink, LastInX, ProgressSink, clear_extra_iteminfo_given_quantity, get_cached_markets_data, get_exchange_rate, get_market_price, get_steamloginsecure, insert_new_exceldata, insert_number_in_sheet, insert_string_in_sheet, update_quantity_exceldata, wrapper_fetch_iteminfo_via_itemprovider_persistent
        },
        helpers::Progress
    }, models::{
        excel::ExcelData, price::Doppler, user_sheet::{SheetInfo, UserInfo}, web::{ExtraItemData, ItemInfoProvider, Sites, SteamData}
    }
};

pub fn run_program_gui(
    user: UserInfo,
    excel: SheetInfo,
) -> impl Straw<(), Progress, String> {
    sipper(move |sender| async move {
        let sink = IcedProgressSink::new(sender);

        run_program(user, excel, sink).await
    })
}

pub async fn run_program<P>(
    mut user: UserInfo,
    mut excel: SheetInfo,
    mut progress: P,
) -> Result<(), String>
where
    P: ProgressSink
{
    progress.send_str("Running main program:\n\n").await;

    if user.fetch_prices && user.iteminfo_provider != ItemInfoProvider::Steam && excel.col_inspect_link.is_some() {
        progress.send_str("Will Fetch additional iteminfo using 3rd party API. This makes doppler prices accurate.\n").await;
    }

    // Client for fetch_more_iteminfo
    let mut iteminfo_client_base = match &user.iteminfo_provider {
        ItemInfoProvider::Csfloat => { csfloat::new_extra_iteminfo_client() },
        ItemInfoProvider::Csgotrader => { csgotrader::new_extra_iteminfo_client() },
        ItemInfoProvider::Steam => { Client::new() }, // Not needed for steam
    };

    let iteminfo_client: &mut Client = &mut iteminfo_client_base;

    // -----------------------------------------------------------------------------------------------

    let steamcookie: Option<Vec<String>> = if user.fetch_steam { get_steamloginsecure(&user.steamloginsecure) } else { None };

    if steamcookie.is_some() { progress.send_str("Found steamcookie(s).\n").await; }
    else if user.fetch_steam { progress.send_str("Didn't find steamcookie(s).\n").await }

    // If multiple cookies found, iterate through them with a delay and hopefully
    // find the cookie that gives all of the inventory.
    let sm_inv: Option<SteamInventory> = {
        if user.fetch_steam {
            if let Some(cookies) = &steamcookie && !cookies.is_empty() {
                let mut inv: Option<SteamInventory> = None;

                for (i, cookie) in cookies.iter().enumerate() {
                    let mut cookie_display = cookie.as_str().take_last_x(7);
                    cookie_display.pop();

                    progress.send_str(
                        &format!("Attempting to fetch inventory with cookie ending in ...{}\n", cookie_display)
                    ).await;

                    inv = Some( SteamInventory::init(user.steamid, 730, Some(cookie)).await? );

                    if let Some(v) = &inv && v.assets_len() == v.inventory_len() {
                        progress.send_str("Found full inventory.\n").await;
                        break
                    }

                    if i != cookies.len() && cookies.len() != 1 {
                        tokio::time::sleep(Duration::from_millis(500)).await;
                    }
                }
                inv

            } else { Some( SteamInventory::init(user.steamid, 730, None).await? ) }
        }
        else { None }
    };

    let cs_inv: Option<Vec<SteamData>> = if let Some(inv) = &sm_inv {
        Some( inv.get_steam_items(user.group_simular_items, true)? )
    } else {
        progress.send_str("Didn't fetch items from cs2 inventory.\n").await;
        None
    };

    // -----------------------------------------------------------------------------------------------

    let markets_to_check: Option<Vec<Sites>> = if user.fetch_prices {
        Some(
            user.prefer_markets.take()
                .unwrap_or_else(|| Sites::iter().collect::<IndexSet<Sites>>() )
                .into_iter()
                .collect::<Vec<Sites>>()
        )
    } else { None };

    let all_market_prices: Option<HashMap<Sites, Value>> = if let Some(m_t_c) = &markets_to_check {
        Some( get_cached_markets_data(m_t_c, user.pricing_provider).await? )
    } else { None };

    if let Some(m_t_p) = &markets_to_check {
        progress.send_str(
            &format!("Fetched prices from {}.\n", m_t_p.iter().map(|m| m.as_str()).collect::<Vec<&str>>().join(", "))
        ).await;
    }

    if cs_inv.is_some() {
        progress.send_str("Reading data from cs inventory and applying it to spreadsheet...\n").await;
    }
    let cs_inv_len = cs_inv.as_ref().map(|i| i.len()).unwrap_or(0);

    // -----------------------------------------------------------------------------------------------

    // BIG BRAIN; READ THE EXCEL SPREADSHEET FIRST TO GET ALL THE INFO AND THEN GET PRICES WOWOWO

    // Getting the Worksheet from either existing book or new book
    let mut book: Spreadsheet = get_spreadsheet(&mut excel.path_to_sheet, &mut excel.sheet_name, user.steamid, &mut progress).await?;

    let sheet: &mut Worksheet = {
        if let Some(sn) = &excel.sheet_name {
            if let Some(buk) = book.get_sheet_by_name_mut(sn) { buk }
            else {
                dprintln!("WARNING: Automatically fetched first sheet in spreadsheet because {} was not found.\n", sn);
                progress.send_str(&format!("WARNING: Automatically fetched first sheet in spreadsheet because {} was not found.", sn)).await;

                book.get_sheet_mut(&0).ok_or_else(|| format!(
                    "Failed to get the first sheet in the spreadsheet with path: \n{:?}", excel.path_to_sheet.as_ref())
                )?
            }
        } else { book.get_sheet_mut(&0).ok_or_else(|| "Failed to get first sheet provided by new file creation.")? }
    };

    let rate = get_exchange_rate(&user.usd_to_x, &excel.rowcol_usd_to_x, sheet).await?;

    // -----------------------------------------------------------------------------------------------

    let mut exceldata: Vec<ExcelData> = get_exceldata(sheet, &excel, user.ignore_already_sold).await?;
    let exceldata_initial_length: usize = exceldata.len();

    if exceldata.is_empty() {
        progress.send_str("Read empty excel spreadsheet.\n\n").await;
    } else {
        let mut exceldata_string = String::with_capacity(256 * exceldata_initial_length);
        exceldata_string.push_str("\nREAD FROM SPREADSHEET:\n");

        let sold_yes: &'static str = "SOLD: YES";
        let sold_no: &'static str = "SOLD: NO";

        for data in &exceldata {
            exceldata_string.push_str(
                &format!(
                    "\tNAME: {:-<75} {} {} {}\n",
                    data.name,
                    if user.group_simular_items { "QUANTITY:" } else { "ASSETID:" },
                    if user.group_simular_items { data.quantity.unwrap_or(0) as u64 } else { data.asset_id.unwrap_or(0) },
                    if user.ignore_already_sold { if data.sold.is_some() {sold_yes} else {sold_no} } else {""}
                )
            );
        }
        progress.send_str(exceldata_string.as_str()).await;
    }

    //  exceldata_old_len er her fordi jeg har endret måte å oppdatere prisene i spreadsheet'n på.
    //  Nå, hvis et item fra steam ikke er i spreadsheetn allerede, så oppdateres spreadsheetn med price, quantity,
    //  phase og inspect link. exceldata_old_len skal være til når resten av itemsene skal oppdateres i pris,
    //  da stopper itereringen ved exceldata_old_len i stedet for å hente prisen til item'ene som er nylig lagt
    //  til og derfor også oppdatert allerede.

    // -----------------------------------------------------------------------------------------------
    if cs_inv.is_some() { progress.send_str("\nDATA FROM STEAM + UPDATES TO SPREADSHEET: \n").await }

    // Inserting and/or updating quantity + adding prices for newly inserted items | .flatten() only runs the loop if it is Some()

    for (i, steamdata) in cs_inv.iter().flatten().enumerate() {

        progress.send( Progress {
            message: if user.group_simular_items {
                format!(
                    "\tNAME: {:-<75} QUANTITY: {} LINK: {}\n",
                    steamdata.name,
                    steamdata.quantity.unwrap_or(0),
                    if steamdata.inspect_link.is_some() {"YES"} else {"NO"}
                )
            } else {
                format!(
                    "\tNAME: {:-<75} ASSETID: {} LINK: {}\n",
                    steamdata.name,
                    steamdata.asset_id,
                    if steamdata.inspect_link.is_some() {"YES"} else {"NO"}
                )
            },
            percent: (i as f32 / cs_inv_len as f32 * 99.0)
        } ).await;

        if user.group_simular_items {
            match exceldata.iter_mut().enumerate().find( |(_, e)| e.name == steamdata.name ) {
                Some((index, data)) => {

                    // Skip item if item is in ignore market names
                    if let Some(ignore) = &user.ingore_steam_names && ignore.iter().any(|n| data.name == *n.trim()) { continue; }

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
                    && let Some(a_m_p) = &all_market_prices
                    && let Some(m_t_c) = &markets_to_check
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
                            m_t_c,
                            a_m_p,
                            rate,
                            &steamdata.name,
                            &iteminfo.phase,
                            &mut progress
                        ).await?;

                        if data.sold.is_none() {
                            if let Some(phase) = &iteminfo.phase { insert_string_in_sheet(sheet, col_phase, row_in_excel, phase.as_str()); }
                            if let Some(price) = price { insert_number_in_sheet(sheet, &excel.col_price, row_in_excel, price); }
                            if let Some(market) = market && let Some(col_market) = &excel.col_market { insert_string_in_sheet(sheet, col_market, row_in_excel, market); }
                        }
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
                            [excel.col_float.as_deref(), excel.col_pattern.as_deref(), excel.col_inspect_link.as_deref()],

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
                    && let Some(m_t_c) = &markets_to_check
                    && let Some(a_m_p) = &all_market_prices
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
                            m_t_c,
                            a_m_p,
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
            message: String::from("\nUpdating prices of old items in spreadsheet...\n"),
            percent: 99.0
        }).await;
    }

    // Second iteration - updates the prices of all the items other than the
    // one(s) inserted into the spreadsheet during the first iteration.
    for (i, data) in exceldata.iter().enumerate() {
        if !user.fetch_prices { break }
        if i == exceldata_initial_length { break }

        if data.sold.is_some() && user.ignore_already_sold { continue; }

        if let Some(ignore) = &user.ingore_steam_names && ignore.iter().any(|s| *s == data.name) {
            continue;
        }

        let row_in_excel = i + excel.row_start_write_in_table as usize;

        if let Some(stop_write) = excel.row_stop_write_in_table && row_in_excel >= stop_write as usize {
            break
        }

        let doppler: Option<Doppler> = data.phase.as_ref()
            .and_then(|p| Doppler::from_str(p).ok());

        let (market, price): (Option<String>, Option<f64>) = if let Some(amp) = &all_market_prices && let Some(mtc) = &markets_to_check {
            get_market_price(
                &user,
                mtc,
                amp,
                rate,
                data.name.as_str(),
                &doppler,
                &mut progress
            ).await?
        } else { (None, None) };

        if let Some(pris) = price { insert_number_in_sheet(sheet, &excel.col_price, row_in_excel, pris); }
        if let Some(marked) = market && let Some(col_market) = &excel.col_market { insert_string_in_sheet(sheet, col_market, row_in_excel, &marked); }
    }

    let finishtime = chrono::Local::now()
        .format("%d/%m/%Y %H:%M:%S")
        .to_string();

    if let Some(cell_date) = &excel.rowcol_date {
        sheet.get_cell_value_mut( cell_date.as_str() )
            .set_value_string( &finishtime );
    }

    // Writes the modified data to the spreadsheet
    set_spreadsheet(&excel.path_to_sheet, user.steamid, book).await
        .map_err(|e| format!("Couldnt write to spreadsheet! : {}", e))?;

    if let Some(inv) = &sm_inv {
        progress.send( Progress {
            message: format!(
                "Fetched items on tradehold: {}\n",
                if inv.assets_len() == inv.inventory_len() {"YES"}
                else if steamcookie.is_some() {"NO. Either cookie it out of date or wrong, or you're not fetching your own inventory."}
                else {"NO"}
            ),
            percent: 100.0
        }).await;
    };

    progress.send( Progress { message: format!("\nEnd time: {}\n", finishtime), percent: 100.0}).await;
    Ok(())
}
