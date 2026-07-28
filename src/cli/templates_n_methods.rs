use std::collections::HashSet;
use std::path::PathBuf;
use std::slice::Iter;

use crate::excel::helpers::{ProgressSink, Progress};

pub fn args_next_or_error<'a>(
    args: &mut Iter<'a, String>,
    caller_arg: &str,
) -> Result<&'a str, String>
{
    args.next()
        .map(String::as_str)
        .ok_or_else(|| format!("No argument given after {}.", caller_arg))
}

pub fn bool_action_or_error<F>(
    actions: &mut HashSet<CliActionQueue>,
    action: F,
    arg: &str,
    caller_arg: &str
) -> Result<(), String>
where
   F: FnOnce(bool) -> CliActionQueue
{
    match arg {
        "y" => {
           actions.insert(action(true));
           Ok(())
        },
        "n" => {
            actions.insert(action(false));
            Ok(())
        },
        err => {
            return Err( format!("Invalid argument {} for {}.", err, caller_arg) );
        }
    }
}

pub struct ConsoleProgressSink;
pub struct NoProgressSink;

impl ProgressSink for ConsoleProgressSink {
    async fn send(&mut self, progress: Progress) {
        // Dont print the percent yo :|
        println!("{}", progress.message);
    }
    async fn send_str(&mut self, progress: &str) {
        println!("{}", progress);
    }
}

impl ProgressSink for NoProgressSink {
    async fn send(&mut self, _: Progress) { }
    async fn send_str(&mut self, _: &str) { }
}

#[derive(Hash, Eq, PartialEq, Debug)]
pub enum CliActionQueue {
    SteamLoginSecure(String),
    PathToSheet(PathBuf),
    SteamId(u64),
    FetchPrices(bool),
    FetchSteam(bool),
    IgnoreSold(bool)
}
