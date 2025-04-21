use std::collections::HashMap;

#[derive(Debug)]
pub struct ItemData { 
    pub asset_id: u64, // IF MODE IS !GROUP_SIMULAR_ITEMS, THIS IS UNIQUE IDENTIFIER
    pub price: HashMap<String, f64>,
    pub inspect_link: Option<String>,
    pub quantity: Option<u16>,
}

// #[derive(Debug)]
// pub struct ItemData {
//     pub name: String, 
//     pub asset_id: u64, // IF MODE IS !GROUP_SIMULAR_ITEMS, THIS IS UNIQUE IDENTIFIER
//     pub price: HashMap<String, f64>,
//     pub inspect_link: Option<String>,
//     pub quantity: Option<u16>,
// }