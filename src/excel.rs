use std::{error::Error, path::PathBuf};
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

pub fn get_exceldata(sheet: &mut Worksheet, excel: &SheetInfo) -> Result<Vec<ExcelData>, String> {
    todo!()
}