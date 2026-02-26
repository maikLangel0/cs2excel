use std::{path::PathBuf, str::FromStr};
use sipper::Sender;
use umya_spreadsheet::{reader, writer, Spreadsheet, Worksheet, XlsxError};

use crate::{dprintln, excel::helpers::{generate_fallback_path, spot, ToColumn}, gui::ice::Progress, models::{excel::ExcelData, price::Doppler, user_sheet::SheetInfo}};

pub async fn get_spreadsheet(path: &mut Option<PathBuf>, sheet_name: &mut Option<String>, progress: &mut Sender<Progress>, steamid: u64) -> Result<Spreadsheet, String> {
    if let Some(pts) = path {
        let sheet = reader::xlsx::read(pts).map_err(|_| String::from("Failed to read file"))?;
        Ok(sheet)
    } else {
        *sheet_name = None;
        let mut new = false;

        if path.is_none() { generate_fallback_path(path, steamid); new = true };

        if !new {
            let filename = path.as_ref()
                .and_then(|p| p.to_str())
                .unwrap_or("Path to the spreadsheet not set!")
                .split("\\")
                .collect::<Vec<&str>>();

            spot(progress, &format!("WARNING: Created a new spreadsheet as one with the name {} didn't exist.\n", filename[filename.len() - 1])).await;
        }
        else {
            spot(progress, &format!("WARNING: Created a new spreadsheet as one with the path\n{}\ndidn't exist.\n",path.clone().map(|p| p.to_string_lossy().to_string()).unwrap_or("C\\Users\\Goober".to_string()))).await;
        }

        Ok(umya_spreadsheet::new_file())
     }
}

pub async fn set_spreadsheet(path: &Option<PathBuf>, steamid: u64, book: Spreadsheet) -> Result<(), XlsxError> {
    if let Some(pts) = path { writer::xlsx::write(&book, pts)?}
    else {
        let mut path: Option<PathBuf> = None;
        generate_fallback_path(&mut path, steamid);
        writer::xlsx::write(&book, path.unwrap())?
    }
    Ok(())
}

pub async fn get_exceldata(sheet: &mut Worksheet, excel: &SheetInfo, ignore_sold: bool) -> Result<Vec<ExcelData>, String> {
    let mut exceldata: Vec<ExcelData> = Vec::new();
    let mut iter = excel.row_start_write_in_table;
    dprintln!("Started reading excel file.");

    loop {
        let name_cell = (excel.col_steam_name.as_str().to_column().unwrap_or(1), iter);

        let name: String = {
            if let Some(cell) = sheet.get_cell(name_cell) {
                let cell_value = cell.get_value().trim().to_string();
                // println!("row: {} | name cellvalue: {}", iter, cell_value);

                if cell_value.is_empty() { break } else { cell_value }
            } else { break }
        };

        // let price_cell = format!("{}{}", excel.col_price, iter);
        // let price: f64 = {
        //     if let Some(cell) = sheet.get_cell(price_cell) {
        //         let cell_value = cell.get_raw_value().to_string().trim().to_string();
        //         if cell_value.is_empty() { break }
        //         else { cell_value.parse::<f64>().map_err(|_| "Price failed parsing")? }
        //     } else { break }
        // };

        // let inspect_link: Option<String> = {
            // if let Some(inspect) = &excel.col_inspect_link {
                // let cell_inspect = format!("{}{}", inspect, &iter);
                // if let Some(cell) = sheet.get_cell(cell_inspect) {

                    // let cell_value = cell.get_raw_value().to_string().trim().to_string();
                    // if cell_value.is_empty() { None } else { Some(cell_value) }

                // } else { None }
            // } else { None }
        // };

        let quantity: Option<u16> = {
            if let Some(quant) = &excel.col_quantity {
                let cell_quantity = (quant.as_str().to_column().unwrap_or(2), iter);

                sheet.get_cell(cell_quantity)
                    .map(|c| c.get_cell_value().get_value())
                    .and_then(|c| c.parse::<u16>().ok())

            } else { None }
        };

        let phase: Option<String> = {
            if let Some(special) = &excel.col_phase {
                let cell_special = (special.as_str().to_column().unwrap_or(3), iter);

                if let Some(cell) = sheet.get_cell(cell_special) {

                    let cell_value = cell.get_value().to_string();
                    if Doppler::from_str(&cell_value).is_err() { None }
                    else { Some(cell_value) }

                } else { None }
            } else { None }
        };

        let asset_id: Option<u64> = {
            if let Some(ass_id) = &excel.col_asset_id {
                let cell_assetid = (ass_id.as_str().to_column().unwrap_or(4), iter);

                sheet.get_cell(cell_assetid).and_then(|c| c.get_value_number().map(|n| n as u64))
            } else { None }
        };

        let sold: Option<f64> = {
            if ignore_sold {
                if let Some(col_already_sold) = &excel.col_sold {
                    let cell = ( col_already_sold.as_str().to_column().unwrap_or(5), iter);

                    // HAS TO BE THIS WAY because reading just the value might fetch it as the formula expression, not the numeric value >:(
                    sheet.get_cell(cell)
                        .map(|c| c.get_cell_value().get_value())
                        .and_then(|c| c.parse::<f64>().ok())

                } else { None }
            } else { None }
        };

        exceldata.push( ExcelData{name, quantity, phase, asset_id, sold} );
        iter += 1;
    }

    dprintln!("Finished reading excel file.");
    Ok(exceldata)
}
