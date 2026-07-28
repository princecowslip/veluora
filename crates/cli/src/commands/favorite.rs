use application::UserStateService;
use domain::ItemId;

use super::{open_context, parse_uuid_arg, print_json, report_and_exit};
use crate::cli_args::OutputFormat;
use crate::exit_code::ExitCode;

pub fn add(format: OutputFormat, quiet: bool, item_id: String) -> ExitCode {
    set_favorite(format, quiet, item_id, true)
}

pub fn remove(format: OutputFormat, quiet: bool, item_id: String) -> ExitCode {
    set_favorite(format, quiet, item_id, false)
}

fn set_favorite(format: OutputFormat, quiet: bool, item_id: String, favorite: bool) -> ExitCode {
    let uuid = match parse_uuid_arg(format, quiet, "item id", &item_id) {
        Ok(u) => u,
        Err(code) => return code,
    };
    let ctx = match open_context(format, quiet) {
        Ok(ctx) => ctx,
        Err(code) => return code,
    };
    match UserStateService::set_favorite(&ctx, ItemId(uuid), favorite) {
        Ok(state) => {
            match format {
                OutputFormat::Json | OutputFormat::Jsonl => print_json(&state),
                OutputFormat::Text | OutputFormat::Table => {
                    if !quiet {
                        let verb = if favorite { "favorited" } else { "unfavorited" };
                        println!("{verb} {item_id}");
                    }
                }
            }
            ExitCode::Success
        }
        Err(err) => report_and_exit(format, quiet, &err),
    }
}
