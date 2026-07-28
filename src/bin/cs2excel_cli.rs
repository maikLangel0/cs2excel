fn main() -> Result<(), cs2excel::MainError> {
    #[cfg(feature = "dhat-heap")]
    let _profiler = dhat::Profiler::new_heap();

    let args: Vec<String> = std::env::args().skip(1).collect();

    cs2excel::cli::cli::init_cli(args).map_err(cs2excel::MainError::CliError)
}
