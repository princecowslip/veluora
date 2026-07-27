use application::{AppContext, DiagnosticsService, DiagnosticsSummary};
use serde::Serialize;

use crate::cli_args::OutputFormat;
use crate::exit_code::ExitCode;

fn print_summary(format: OutputFormat, quiet: bool, summary: &DiagnosticsSummary) {
    match format {
        OutputFormat::Json | OutputFormat::Jsonl => {
            println!(
                "{}",
                serde_json::to_string(summary).expect("serialize diagnostics summary")
            );
        }
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
    let ctx = match AppContext::open_default() {
        Ok(ctx) => ctx,
        Err(err) => return report_and_exit(format, quiet, &err),
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
    let ctx = match AppContext::open_default() {
        Ok(ctx) => ctx,
        Err(err) => return report_and_exit(format, quiet, &err),
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
        OutputFormat::Json | OutputFormat::Jsonl => {
            println!(
                "{}",
                serde_json::to_string(&output).expect("serialize db check output")
            );
        }
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

fn report_and_exit(format: OutputFormat, quiet: bool, err: &application::AppError) -> ExitCode {
    let code = ExitCode::from(err);
    match format {
        OutputFormat::Json | OutputFormat::Jsonl => {
            let payload = serde_json::json!({
                "schema_version": 1,
                "ok": false,
                "error": err.to_string(),
            });
            println!("{payload}");
        }
        OutputFormat::Text | OutputFormat::Table => {
            if !quiet {
                eprintln!("error: {err}");
            }
        }
    }
    code
}
