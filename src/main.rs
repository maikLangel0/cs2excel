// #![windows_subsystem = "windows"]

use std::env;

mod excel;
mod models;
mod parsing;
mod browser;
mod gui;
mod cli;

#[cfg(feature = "dhat-heap")]
#[global_allocator]
static ALLOC: dhat::Alloc = dhat::Alloc;

// TTL for the pricing cache
const CACHE_TIME: std::time::Duration = std::time::Duration::from_secs(60 * 60 * 6);

// Sick macro by gipiti | only prints to console when build flag is not set
/// Only creates the println! macro if not in release mode
#[macro_export]
macro_rules! dprintln {
    ($( $arg:tt )*) => {
        #[cfg(debug_assertions)]
        println!( $( $arg )* )
    };
}

fn main() -> Result<(), iced::Error> {

    #[cfg(feature = "dhat-heap")]
    let _profiler = dhat::Profiler::new_heap();

    println!("Haii");
    println!("{:?}", env::args());
    gui::ice::init_gui()
}
