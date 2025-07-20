#![windows_subsystem = "windows"]

mod excel;
mod models;
mod parsing;
mod browser;
mod gui;

fn main() -> Result<(), iced::Error> {
    gui::ice::init_gui()
}
// 
// #[tokio::main]
// async fn main() -> Result<(), String> {
    // let app = gui::ice::App::default();
    // let _ = excel::excel_runtime::run_program(app.usersheet.user, app.usersheet.sheet).await?;
    // Ok(())
// }
