use std::path::PathBuf;

use application::DiagnosticsService;

use super::{open_context, print_error_message, report_and_exit};
use crate::cli_args::OutputFormat;
use crate::exit_code::ExitCode;

pub fn bundle(format: OutputFormat, quiet: bool, file: Option<PathBuf>) -> ExitCode {
    let ctx = match open_context(format, quiet) {
        Ok(ctx) => ctx,
        Err(code) => return code,
    };
    let bundle = match DiagnosticsService::support_bundle(&ctx) {
        Ok(bundle) => bundle,
        Err(err) => return report_and_exit(format, quiet, &err),
    };
    let json = serde_json::to_string_pretty(&bundle).expect("serialize support bundle");

    match file {
        Some(path) => {
            if let Err(e) = std::fs::write(&path, &json) {
                print_error_message(
                    format,
                    quiet,
                    &format!("could not write {}: {e}", path.display()),
                );
                return ExitCode::ConfigurationFailure;
            }
            if !quiet {
                println!("support bundle written to {}", path.display());
            }
        }
        None => println!("{json}"),
    }
    ExitCode::Success
}
