mod excel;
mod models;
mod parsing;
mod browser;
mod gui;

fn main() -> Result<(), iced::Error> {
    gui::ice::init_gui()
}

// #[tokio::main]
// async fn main() -> Result<(), String> {
    // let _ = excel::excel_runtime::run_program(&mut USER.clone(), &mut SHEET.clone()).await?;
    // Ok(())
// }
