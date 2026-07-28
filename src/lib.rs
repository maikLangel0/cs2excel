#[cfg(feature = "dhat-heap")]
#[global_allocator]
static ALLOC: dhat::Alloc = dhat::Alloc;

pub mod excel;
pub mod models;
pub mod parsing;
pub mod browser;

#[cfg(feature = "gui")]
pub mod gui;

#[cfg(feature = "cli")]
pub mod cli;


const CACHE_TIME: std::time::Duration = std::time::Duration::from_secs(60 * 60 * 6);

#[derive(Debug)]
pub enum MainError {
    IcedError(iced::Error),
    CliError(String)
}

#[macro_export]
macro_rules! dprintln {
    ($( $arg:tt )*) => {
        #[cfg(debug_assertions)]
        println!( $( $arg )* )
    };
}

// --------- MAIN YO ---------------------------------------------------------------

// fn main() -> Result<(), MainError> {

//     #[cfg(feature = "dhat-heap")]
//     let _profiler = dhat::Profiler::new_heap();

//     #[cfg(any(feature = "cli_only", feature = "both"))]
//     let args: Vec<String> = env::args().skip(1).collect();

//     #[cfg(feature = "both")]
//     if args.is_empty() {
//         return gui::ice::init_gui().map_err(MainError::IcedError);
//     } else {
//         return cli::cli::init_cli(args).map_err(MainError::CliError);
//     }

//     #[cfg(feature = "gui_only")]
//     return gui::ice::init_gui().map_err(MainError::IcedError);

//     #[cfg(feature = "cli_only")]
//     return cli::cli::init_cli(args).map_err(MainError::CliError);
// }
