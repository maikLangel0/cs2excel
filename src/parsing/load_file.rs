use std::{path::Path, fs::File, io::BufReader};
use serde_json;
use crate::models::user_sheet::{SheetInfo, UserInfo, UserSheet};
use crate::dprintln;

pub fn load_usersheet(
    path: &Path,
    user: &mut UserInfo,
    sheet: &mut SheetInfo
) -> Result<(), String>
{

    if let Ok(file) = File::open(path) {
        let read = BufReader::new(file);

        match serde_json::from_reader::<_, UserSheet>(read) {
            Ok(load) => {
                *user = load.user;
                *sheet = load.sheet;
                Ok(())
            },
            Err(_e) => { dprintln!("File load parse error:\n{_e}\n"); Err( String::from("Failed parsing file.\n")) }
        }
    } else {
        Err( String::from("Failed reading file."))
    }
}
