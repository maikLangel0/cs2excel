use std::env;

use crate::excel::helpers::{ProgressSink, Progress};

pub fn init_cli(vars: env::Args) -> Result<(), String> {

    Ok(())
}

pub struct ConsoleProgressSink;

impl ProgressSink for ConsoleProgressSink {
    async fn send(&mut self, progress: Progress) {
        println!("{}, {}", progress.message, progress.percent);
    }
    async fn send_str(&mut self, progress: &str) {
        println!("{}", progress);
    }
}

impl ConsoleProgressSink {
    pub fn new() -> ConsoleProgressSink {
        Self
    }
}
