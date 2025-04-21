use std::{error::Error, path::PathBuf};
use indexmap::IndexMap;
use umya_spreadsheet::{new_file, reader, writer, Spreadsheet, Worksheet, XlsxError};
use whoami;

use crate::models::{excel::ExcelData, item::ItemData, user_sheet::{SheetInfo, UserInfo}};

async fn excelling(user: &UserInfo, excel: &SheetInfo, items: &Vec<ItemData>) -> Result<(), Box<dyn Error>> {    
    Ok(())
}

pub fn get_spreadsheet(path: &Option<PathBuf>) -> Result<Spreadsheet, XlsxError> {
    if let Some(pts) = path { 
        let sheet = reader::xlsx::read(pts)?; 
        Ok(sheet)
    } else { Ok(new_file()) } 
}

pub fn set_spreadsheet(path: &Option<PathBuf>, book: Spreadsheet) -> Result<(), XlsxError> {
    if let Some(pts) = path { 
        writer::xlsx::write(&book, pts)? 
    } else { 
        let std_path = PathBuf::from( format!("C:/Users/{}", whoami::username()) );
        writer::xlsx::write(&book, std_path)? 
    }
    Ok(())
}

// pub fn get_exceldata(sheet: &mut Worksheet, excel: &SheetInfo) -> Result<Vec<ExcelData>, String> {
pub fn get_exceldata(sheet: &mut Worksheet, excel: &SheetInfo) -> Result<IndexMap< String, ExcelData >, String> {
    // let mut exceldata: Vec<ExcelData> = Vec::new();
    let mut exceldata: IndexMap<String, ExcelData> = IndexMap::new();
    let mut iter = excel.row_start_write_in_table;

    loop {
        let name_cell = format!("{}{}", excel.col_market_name, iter);
        let price_cell = format!("{}{}", excel.col_price, iter);

        let name = {
            if let Some(cell) = sheet.get_cell(name_cell) {
                cell.get_raw_value().to_string()
            } else { break }
        };

        let price = {
            if let Some(cell) = sheet.get_cell(price_cell) {
                cell.get_raw_value()
                    .to_string()
                    .parse::<f64>()
                    .map_err(|_| "Price failed parsing")?
            } else { break }
        };

        let inspect_link = {
            if let Some(inspect) = &excel.col_inspect_link {
                let cell_inspect = format!("{}{}", inspect, &iter);

                if let Some(cell) = sheet.get_cell(cell_inspect) {
                    Some( cell.get_raw_value().to_string() )  
                } 
                else { None }
            } else { None }
        };

        let quantity = {
            if let Some(quant) = &excel.col_quantity {
                let cell_quantity = format!("{}{}", quant, &iter);

                if let Some(cell) = sheet.get_cell(cell_quantity) {
                    Some( 
                        cell.get_raw_value()
                            .to_string()
                            .parse::<f64>()
                            .map_err(|_| "Quantity failed parsing")?
                    )
                }
                else { None }
            } else { None }
        };

        let phase = {
            if let Some(special) = &excel.col_phase {
                let cell_special = format!("{}{}", special, &iter);

                if let Some(cell) = sheet.get_cell(cell_special) {
                    Some( cell.get_raw_value().to_string() )
                }
                else { None }
            } else { None }
        };

        // exceldata.push( ExcelData{ name, price, quantity, inspect_link, special } );
        exceldata.insert(name, ExcelData{price, quantity, inspect_link, phase} );

        iter += 1;
    }

    // if exceldata.iter().all( |data| data.quantity.is_none() && data.inspect_link.is_none() ) && iter > 5 {
        // return Err( String::from("Both quantity and inspect_link column are completely empty!") );
    // }

    Ok(exceldata)
}