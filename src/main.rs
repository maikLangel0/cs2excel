// #![windows_subsystem = "windows"]

mod excel;
mod models;
mod parsing;
mod browser;
mod gui;

// Sick macro by gipiti | only prints to console when build flag is not set
#[macro_export]
macro_rules! dprintln {
    ($( $arg:tt )*) => {
        #[cfg(debug_assertions)]
        println!( $( $arg )* )
    };
}

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
