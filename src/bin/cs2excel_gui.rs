#![windows_subsystem = "windows"]

fn main() -> Result<(), cs2excel::MainError> {
    #[cfg(feature = "dhat-heap")]
    let _profiler = dhat::Profiler::new_heap();

    cs2excel::gui::ice::init_gui().map_err(cs2excel::MainError::IcedError)
}
