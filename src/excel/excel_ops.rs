use std::{path::PathBuf, str::FromStr};
use umya_spreadsheet::{new_file, reader, writer, Spreadsheet, Worksheet, XlsxError};
use whoami;

use crate::models::{excel::ExcelData, price::Doppler, user_sheet::SheetInfo};

pub async fn get_spreadsheet(path: &Option<PathBuf>) -> Result<Spreadsheet, XlsxError> {
    if let Some(pts) = path { 
        let sheet = reader::xlsx::read(pts)?; 
        Ok(sheet)
    } else { Ok(new_file()) } 
}

pub async fn set_spreadsheet(path: &Option<PathBuf>, book: Spreadsheet) -> Result<(), XlsxError> {
    if let Some(pts) = path { 
        writer::xlsx::write(&book, pts)?
    } else { 
        let std_path = PathBuf::from( format!("C:/Users/{}", whoami::username()) );
        writer::xlsx::write(&book, std_path)? 
    }
    Ok(())
}

pub async fn get_exceldata(sheet: &mut Worksheet, excel: &SheetInfo, ignore_sold: bool) -> Result<Vec<ExcelData>, String> {
    let mut exceldata: Vec<ExcelData> = Vec::new();
    let mut iter = excel.row_start_write_in_table;

    loop {
        
        let name_cell = format!("{}{}", excel.col_steam_name, iter);
        let name: String = {
            if let Some(cell) = sheet.get_cell(name_cell) {
                let cell_value = cell.get_raw_value().to_string().trim().to_string();
                println!("row: {} | name cellvalue: {}", iter, cell_value);

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
                let cell_quantity = format!("{}{}", quant, &iter);

                if let Some(cell) = sheet.get_cell(cell_quantity) {

                    let cell_value = cell.get_raw_value().to_string().trim().to_string(); 
                    Some(cell_value.parse::<u16>().map_err(|_| "Quantity failed parsing")?)

                } else { None }
            } else { None }
        };

        let phase: Option<String> = {
            if let Some(special) = &excel.col_phase {
                let cell_special = format!("{}{}", special, &iter);
                if let Some(cell) = sheet.get_cell(cell_special) {

                    let cell_value = cell.get_raw_value().to_string().trim().to_string();
                    if cell_value.is_empty() { None } 
                    else if Doppler::from_str(&cell_value).is_err() { None }
                    else { Some(cell_value) }

                } else { None }
            } else { None }
        };

        let asset_id: Option<u64> = {
            if let Some(ass_id) = &excel.col_asset_id {
                let cell_assetid = format!("{}{}", &ass_id, &iter);
                if let Some(cell) = sheet.get_cell(cell_assetid) {
                     
                    let cell_value = cell.get_raw_value().to_string().trim().to_string();
                    if cell_value.is_empty() { None } 
                    else { Some(cell_value.parse::<u64>().map_err(|_| "Assetid failed parsing")?) }

                } else { None }
            } else { None }
        };

        let sold: Option<f64> = {
            if ignore_sold { 
                if let Some(col_already_sold) = &excel.col_sold {
                    let cell = format!("{}{}", &col_already_sold, &iter);
                    if let Some(cell) = sheet.get_cell(cell) {

                        let cell_value = cell.get_raw_value().to_string().trim().to_string();
                        if cell_value.is_empty() { None } 
                        else { Some(cell_value.parse::<f64>().map_err(|_| "already_sold failed parsing")?) }

                    } else { None }
                } else { None }
            } else { None }
        };

        exceldata.push( ExcelData{name, quantity, phase, asset_id, sold} );
        iter += 1;
    }

    Ok(exceldata)
}