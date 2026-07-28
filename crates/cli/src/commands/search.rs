use application::SearchService;

use super::{open_context, print_json, report_and_exit};
use crate::cli_args::OutputFormat;
use crate::exit_code::ExitCode;

pub fn run(format: OutputFormat, quiet: bool, query: String, limit: u32, offset: u32) -> ExitCode {
    let ctx = match open_context(format, quiet) {
        Ok(ctx) => ctx,
        Err(code) => return code,
    };
    match SearchService::search(&ctx, &query, limit, offset) {
        Ok(results) => {
            match format {
                OutputFormat::Json | OutputFormat::Jsonl => print_json(&results),
                OutputFormat::Text | OutputFormat::Table => {
                    if !quiet {
                        println!("{} result(s) for \"{}\"", results.total, results.query);
                        for hit in &results.items {
                            let fav = if hit.favorite { "*" } else { " " };
                            println!("{fav} {}  {:?}  {}", hit.item_id, hit.media_type, hit.title);
                        }
                    }
                }
            }
            ExitCode::Success
        }
        Err(err) => report_and_exit(format, quiet, &err),
    }
}
