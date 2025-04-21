// #[derive(Debug)]
// pub struct ExcelData {
    // pub name: String, 
    // pub price: f64,
    // pub quantity: Option<f64>,        // Hvis items ikke er group'a together, så har de None quantity
    // pub inspect_link: Option<String>, // Brukes for å inspecte + pricechecke hvis -
    // pub special: Option<String>       // - special er noe, aka hvis det er en sapphire så kan prisen 
// }                                     // for en sapphire hentes korrekt via float api'et til csgotrader 

#[derive(Debug)]
pub struct ExcelData {
    pub price: f64,
    pub quantity: Option<f64>,        // Hvis items ikke er group'a together, så har de None quantity
    pub inspect_link: Option<String>, // Brukes for å inspecte + pricechecke hvis -
    pub phase: Option<String>         // - special er noe, aka hvis det er en sapphire så kan prisen 
}                                     // for en sapphire hentes korrekt via float api'et til csgotrader 

