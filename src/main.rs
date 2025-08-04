// #![windows_subsystem = "windows"]

use std::time::Duration;

mod excel;
mod models;
mod parsing;
mod browser;
mod gui;

// TTL for the pricing cache
const CACHE_TIME: Duration = Duration::from_secs(60 * 60 * 6);

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
