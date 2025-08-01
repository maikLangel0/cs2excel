/// Used to keep track of the data fetched from the Excel spreadsheet. Only contains information that is neccessary to have when update / inserting data in the spreadsheet

pub struct ExcelData {
    pub name: String, 
    // pub price: f64,
    pub quantity: Option<u16>,        // Hvis items ikke er group'a together, så har de None quantity
    // pub inspect_link: Option<String>, // Brukes for å inspecte + pricechecke hvis special er noe, aka hvis det er en sapphire så kan prisen 
    pub phase: Option<String>,        // for en sapphire hentes korrekt via float api'et til csgotrader // csfloat
    pub asset_id: Option<u64>,        // Unik ID brukes hvis man ikke grupperer samme items
    pub sold: Option<f64>
}                                   
// 
// #[derive(Debug)]
// pub struct ExcelData {
    // pub price: f64,
    // pub quantity: Option<f64>,        // Hvis items ikke er group'a together, så har de None quantity
    // pub inspect_link: Option<String>, // Brukes for å inspecte + pricechecke hvis -
    // pub phase: Option<String>         // - special er noe, aka hvis det er en sapphire så kan prisen 
// }                                     // for en sapphire hentes korrekt via float api'et til csgotrader 

