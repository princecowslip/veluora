pub mod collection;
pub mod favorite;
pub mod item;
pub mod library;
pub mod search;

use application::{AppContext, AppError, DiagnosticsService, DiagnosticsSummary};
use serde::Serialize;

use crate::cli_args::OutputFormat;
use crate::exit_code::ExitCode;

fn print_summary(format: OutputFormat, quiet: bool, summary: &DiagnosticsSummary) {
    match format {
        OutputFormat::Json | OutputFormat::Jsonl => print_json(summary),
        OutputFormat::Text | OutputFormat::Table => {
            if quiet {
                return;
            }
            println!("veloura doctor");
            println!("  data dir:           {}", summary.data_dir);
            println!("  database:           {}", summary.db_path);
            println!("  applied migrations: {}", summary.applied_migrations);
            println!("  status:             ok");
        }
    }
}

pub fn doctor(format: OutputFormat, quiet: bool) -> ExitCode {
    let ctx = match open_context(format, quiet) {
        Ok(ctx) => ctx,
        Err(code) => return code,
    };
    match DiagnosticsService::summary(&ctx) {
        Ok(summary) => {
            print_summary(format, quiet, &summary);
            ExitCode::Success
        }
        Err(err) => report_and_exit(format, quiet, &err),
    }
}

#[derive(Serialize)]
struct DbCheckOutput {
    schema_version: u32,
    ok: bool,
    applied_migrations: i64,
    db_path: String,
}

pub fn db_check(format: OutputFormat, quiet: bool) -> ExitCode {
    let ctx = match open_context(format, quiet) {
        Ok(ctx) => ctx,
        Err(code) => return code,
    };
    let applied_migrations = match ctx.db.applied_migration_count() {
        Ok(count) => count,
        Err(err) => return report_and_exit(format, quiet, &application::AppError::from(err)),
    };

    let output = DbCheckOutput {
        schema_version: 1,
        ok: true,
        applied_migrations,
        db_path: ctx.db.path().display().to_string(),
    };

    match format {
        OutputFormat::Json | OutputFormat::Jsonl => print_json(&output),
        OutputFormat::Text | OutputFormat::Table => {
            if !quiet {
                println!(
                    "database ok — {} migration(s) applied ({})",
                    output.applied_migrations, output.db_path
                );
            }
        }
    }
    ExitCode::Success
}

/// Resolves the default `AppContext`, converting a failure directly into
/// an already-reported `ExitCode` — the common first line of every
/// command below.
pub(crate) fn open_context(
    format: OutputFormat,
    quiet: bool,
) -> std::result::Result<AppContext, ExitCode> {
    AppContext::open_default().map_err(|err| report_and_exit(format, quiet, &err))
}

pub(crate) fn print_json<T: Serialize>(value: &T) {
    println!(
        "{}",
        serde_json::to_string(value).expect("serialize command output")
    );
}

pub(crate) fn print_error_message(format: OutputFormat, quiet: bool, message: &str) {
    match format {
        OutputFormat::Json | OutputFormat::Jsonl => {
            print_json(&serde_json::json!({ "schema_version": 1, "ok": false, "error": message }))
        }
        OutputFormat::Text | OutputFormat::Table => {
            if !quiet {
                eprintln!("error: {message}");
            }
        }
    }
}

pub(crate) fn report_and_exit(format: OutputFormat, quiet: bool, err: &AppError) -> ExitCode {
    print_error_message(format, quiet, &err.to_string());
    ExitCode::from(err)
}

/// Parses a UUID-shaped CLI argument (item/collection/root id), reporting
/// and converting a parse failure into `ExitCode::InvalidArguments`
/// through the same error-reporting path as every other failure.
pub(crate) fn parse_uuid_arg(
    format: OutputFormat,
    quiet: bool,
    label: &str,
    raw: &str,
) -> std::result::Result<uuid::Uuid, ExitCode> {
    uuid::Uuid::parse_str(raw).map_err(|_| {
        print_error_message(format, quiet, &format!("invalid {label}: '{raw}'"));
        ExitCode::InvalidArguments
    })
}
