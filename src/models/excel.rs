#[derive(Debug)]
pub struct ExcelData {
    pub name: String,
    pub price: f64,
    pub quantity: Option<f64>,
    pub inspect_link: Option<String>
}