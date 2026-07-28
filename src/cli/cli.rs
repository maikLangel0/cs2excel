use std::collections::HashSet;
use std::path::{Path, PathBuf};

use crate::excel::excel_runtime;
use crate::models::user_sheet::UserSheet;
use crate::parsing::load_file;
use crate::cli::templates_n_methods::{
    args_next_or_error,
    bool_action_or_error,
    CliActionQueue,
    ConsoleProgressSink,
    NoProgressSink
};

use tokio;

const HELP_MSG: &'static str = r#"--- HELP ---

It is recommended that you use the GUI for everything that you can't set using the CLI tool.
When you need/want more options (like managing multiple spreadsheets at once), use different savefiles
made using the GUI instead of altering an existing savefile.
The only argument required to run is -l / -load, given that your savefile includes everything necessary.
Example of what to call when you want to run this program:
On Windows:
    .\cs2excel_cli -l C:\Users\SumYoungGuy\savefile.json -sls test -id 123456789 -fetchsteam n
On Linux:
    .\cs2excel -load /home/SumYoungGuy/Documents/savefile.json -steamloginsecure lule -steamid 67 -fs y


-l OR -load [path]  |  Changes/Provides the savefile that will be used to run the program with.
	[save_file_path] has to be a full path (ex: C:\Users\SumYoungGuy\savefile.json).
	You can create a savefile using the GUI app.

-pts OR -pathtosheet [path]  |  Changes/Provides the spreadsheet that you want to operate on.
	[spreadsheet_path] has to be a full path (ex: C:\Users\SumYoungGuy\spreadsheet.xlsx).

-id OR -steamid [number]  |  Changes/Provides the steamid of the loaded savefile to the given [number].

-sls OR -steamloginsecure [text]  |  Changes/Provides the steamLoginSecure of the loaded savefile to the given [text].
	This is useful if you want the most up-to-date items from your inventory.
	Program still fetches your steamLoginSecure from Firefox if you're on Windows.
	Set [text] to a bogus value if you dont want to automatically fetch from Firefox.

-is OR -ignoresold [y/n]  |  Do/Don't skip over items that are already flagged as sold,
    given a 'sold column' is set in the savefile.

-fs OR -fetchsteam [y/n]  |  Do/Don't fetch the cs inventory of the steamID provided.

-fp OR -fetchprices [y/n]  |  Do/Don't update the prices in the Spreadsheet.

-np OR -noprint  |  Disables printing progress and warnings to stdout, but errors still get printed.
"#;

pub fn init_cli(args: Vec<String>) -> Result<(), String> {
    let mut actions: HashSet<CliActionQueue> = HashSet::new();
    let mut data: Option<UserSheet> = None;

    let mut verbose_cli_out: bool = true;

    let mut caller_arg: String;

    let mut args = args.iter();

    // first pass to load file and queue actions
    while let Some(arg) = args.next() {
        caller_arg = arg.to_string();

        match arg.as_str() {
            "--help" |"-help" | "--h" | "-h" => {
                println!("{}", HELP_MSG);
                return Ok(());
            },
            "-load" | "--load" | "-l" | "--l"  => {
                let to_load = args_next_or_error(&mut args, &caller_arg)?;

                if to_load.ends_with(".json") {
                    let path = Path::new(to_load);

                    if !path.exists() {
                        return Err( format!("Path {} could not be found.", to_load));
                    }

                    let mut data_init = UserSheet::default();
                    let user = &mut data_init.user;
                    let sheet = &mut data_init.sheet;

                    match load_file::load_usersheet(path, user, sheet) {
                        Ok(()) => { data = Some(data_init); },
                        Err(err) => { return Err(err) }
                    }
                } else {
                    return Err( format!("Filepath {} does not end in '.json'", to_load) );
                }
            },
            "-steamloginsecure" | "--steamloginsecure" | "-sls" | "--sls" => {
                let sls = args_next_or_error(&mut args, &caller_arg)?;

                actions.insert( CliActionQueue::SteamLoginSecure(sls.to_string()) );
            },
            "-pathtosheet" | "--pathtosheet" | "-pts" | "--pts" => {
                let pts = args_next_or_error(&mut args, &caller_arg)?;

                let pathbuf = PathBuf::from(pts);

                if !pathbuf.exists() {
                    return Err( format!("Path {} could not be found.", pts) );
                }

                actions.insert( CliActionQueue::PathToSheet(PathBuf::from(pts)) );
            },
            "-steamid" | "--steamid" | "-id" | "--id" => {
                let id = args_next_or_error(&mut args, &caller_arg)?;

                match id.parse::<u64>() {
                    Ok(id) => { actions.insert( CliActionQueue::SteamId(id) ); },
                    Err(_) => { return Err( format!("couldn't read {} as a steamid.", id) ); }
                }
            },
            "-noprint" | "-np" | "--noprint" | "--np" => {
                verbose_cli_out = false;
            },
            "-fs" | "-fetchsteam" | "--fs" | "--fetchsteam" => {
                let arg = args_next_or_error(&mut args, &caller_arg)?;

                bool_action_or_error(
                    &mut actions,
                    CliActionQueue::FetchSteam,
                    arg,
                    &caller_arg
                )?;
            },
            "-fp" | "-fetchprices" | "--fp" | "--fetchprices" => {
                let arg = args_next_or_error(&mut args, &caller_arg)?;

                bool_action_or_error(
                    &mut actions,
                    CliActionQueue::FetchPrices,
                    arg,
                    &caller_arg
                )?;
            },
            "-is" | "-ignoresold" | "--is" | "--ignoresold" => {
                let arg = args_next_or_error(&mut args, &caller_arg)?;

                bool_action_or_error(
                    &mut actions,
                    CliActionQueue::IgnoreSold,
                    arg,
                    &caller_arg
                )?;
            },
            err => { return Err(format!("No argument called {}.", err)); }
        }
    }

    let mut data = data.ok_or("No savefile/preset loaded. Use the argument '-l [path_to_save]'.".to_string())?;

    // 2nd pass to apply the actions
    for action in actions {
        match action {
            CliActionQueue::SteamLoginSecure(sls) => {
                data.user.steamloginsecure = Some(sls);
            },
            CliActionQueue::PathToSheet(pts) => {
                data.sheet.path_to_sheet = Some(pts);
            },
            CliActionQueue::SteamId(id) => {
                data.user.steamid = id;
            },
            CliActionQueue::FetchPrices(b) => {
                data.user.fetch_prices = b;
            },
            CliActionQueue::IgnoreSold(b) => {
                data.user.ignore_already_sold = b;
            }
            CliActionQueue::FetchSteam(b) => {
                data.user.fetch_steam = b;
            }
        }
    }

    let tokio_runtime = tokio::runtime::Runtime::new().map_err(|_| "Failed to start async runtime.")?;

    if verbose_cli_out {
        tokio_runtime.block_on(
            excel_runtime::run_program(data.user, data.sheet, ConsoleProgressSink)
        )
    } else {
        tokio_runtime.block_on(
            excel_runtime::run_program(data.user, data.sheet, NoProgressSink)
        )
    }
}
