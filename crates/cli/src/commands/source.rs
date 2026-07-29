use application::SourceService;
use domain::{ConnectorId, RemoteItem, SourceId};
use uuid::Uuid;

use super::{open_context, print_error_message, print_json, report_and_exit};
use crate::cli_args::OutputFormat;
use crate::exit_code::ExitCode;

/// Runs `fut` on a fresh single-purpose tokio runtime — the CLI is
/// otherwise entirely synchronous; only source health-checks/browsing
/// need real async I/O (an outbound HTTP fetch for network
/// connectors), so a whole-program async rewrite isn't worth it for
/// two commands.
fn run_async<F: std::future::Future>(fut: F) -> F::Output {
    tokio::runtime::Runtime::new()
        .expect("could not start an async runtime")
        .block_on(fut)
}

fn parse_source_id(format: OutputFormat, quiet: bool, raw: &str) -> Result<SourceId, ExitCode> {
    Uuid::parse_str(raw).map(SourceId).map_err(|_| {
        print_error_message(format, quiet, &format!("invalid source id: '{raw}'"));
        ExitCode::InvalidArguments
    })
}

pub fn list(format: OutputFormat, quiet: bool) -> ExitCode {
    let ctx = match open_context(format, quiet) {
        Ok(ctx) => ctx,
        Err(code) => return code,
    };
    match SourceService::list(&ctx) {
        Ok(sources) => {
            match format {
                OutputFormat::Json | OutputFormat::Jsonl => print_json(&sources),
                OutputFormat::Text | OutputFormat::Table => {
                    if !quiet {
                        if sources.is_empty() {
                            println!("(no sources configured)");
                        }
                        for source in &sources {
                            println!(
                                "{}  {}  connector={}  enabled={}  health={:?}",
                                source.id,
                                source.display_name,
                                source.connector_id,
                                source.enabled,
                                source.health_state
                            );
                        }
                    }
                }
            }
            ExitCode::Success
        }
        Err(err) => report_and_exit(format, quiet, &err),
    }
}

pub fn add(
    format: OutputFormat,
    quiet: bool,
    connector_id: String,
    display_name: String,
    config: String,
) -> ExitCode {
    let Ok(connector_id) = Uuid::parse_str(&connector_id) else {
        print_error_message(
            format,
            quiet,
            &format!("invalid connector id: '{connector_id}'"),
        );
        return ExitCode::InvalidArguments;
    };
    let configuration_json: serde_json::Value = match serde_json::from_str(&config) {
        Ok(v) => v,
        Err(e) => {
            print_error_message(format, quiet, &format!("invalid --config JSON: {e}"));
            return ExitCode::InvalidArguments;
        }
    };
    let ctx = match open_context(format, quiet) {
        Ok(ctx) => ctx,
        Err(code) => return code,
    };
    match SourceService::add(
        &ctx,
        ConnectorId(connector_id),
        display_name,
        configuration_json,
    ) {
        Ok(source) => {
            match format {
                OutputFormat::Json | OutputFormat::Jsonl => print_json(&source),
                OutputFormat::Text | OutputFormat::Table => {
                    if !quiet {
                        println!("added source {} ({})", source.id, source.display_name);
                    }
                }
            }
            ExitCode::Success
        }
        Err(err) => report_and_exit(format, quiet, &err),
    }
}

pub fn remove(format: OutputFormat, quiet: bool, source_id: String) -> ExitCode {
    let id = match parse_source_id(format, quiet, &source_id) {
        Ok(id) => id,
        Err(code) => return code,
    };
    let ctx = match open_context(format, quiet) {
        Ok(ctx) => ctx,
        Err(code) => return code,
    };
    match SourceService::remove(&ctx, id) {
        Ok(()) => {
            if !quiet && format == OutputFormat::Text {
                println!("removed source {source_id}");
            }
            ExitCode::Success
        }
        Err(err) => report_and_exit(format, quiet, &err),
    }
}

pub fn set_enabled(
    format: OutputFormat,
    quiet: bool,
    source_id: String,
    enabled: bool,
) -> ExitCode {
    let id = match parse_source_id(format, quiet, &source_id) {
        Ok(id) => id,
        Err(code) => return code,
    };
    let ctx = match open_context(format, quiet) {
        Ok(ctx) => ctx,
        Err(code) => return code,
    };
    match SourceService::set_enabled(&ctx, id, enabled) {
        Ok(()) => {
            if !quiet && format == OutputFormat::Text {
                let verb = if enabled { "enabled" } else { "disabled" };
                println!("{verb} source {source_id}");
            }
            ExitCode::Success
        }
        Err(err) => report_and_exit(format, quiet, &err),
    }
}

pub fn health_check(format: OutputFormat, quiet: bool, source_id: String) -> ExitCode {
    let id = match parse_source_id(format, quiet, &source_id) {
        Ok(id) => id,
        Err(code) => return code,
    };
    let ctx = match open_context(format, quiet) {
        Ok(ctx) => ctx,
        Err(code) => return code,
    };
    match run_async(SourceService::health_check(&ctx, id)) {
        Ok(health) => {
            match format {
                OutputFormat::Json | OutputFormat::Jsonl => print_json(&health),
                OutputFormat::Text | OutputFormat::Table => {
                    if !quiet {
                        println!("{health:?}");
                    }
                }
            }
            ExitCode::Success
        }
        Err(err) => report_and_exit(format, quiet, &err),
    }
}

pub fn browse(
    format: OutputFormat,
    quiet: bool,
    source_id: String,
    query: Option<String>,
) -> ExitCode {
    let id = match parse_source_id(format, quiet, &source_id) {
        Ok(id) => id,
        Err(code) => return code,
    };
    let ctx = match open_context(format, quiet) {
        Ok(ctx) => ctx,
        Err(code) => return code,
    };
    match run_async(SourceService::browse(&ctx, id, query.as_deref())) {
        Ok(report) => {
            match format {
                OutputFormat::Json | OutputFormat::Jsonl => print_json(&report),
                OutputFormat::Text | OutputFormat::Table => {
                    if !quiet {
                        if !report.unsupported_clauses.is_empty() {
                            println!(
                                "(query clauses not supported by this connector, ignored: {})",
                                report.unsupported_clauses.join(", ")
                            );
                        }
                        println!("{:?}", report.result);
                    }
                }
            }
            ExitCode::Success
        }
        Err(err) => report_and_exit(format, quiet, &err),
    }
}

pub fn import(
    format: OutputFormat,
    quiet: bool,
    source_id: String,
    remote_item_json: String,
) -> ExitCode {
    let id = match parse_source_id(format, quiet, &source_id) {
        Ok(id) => id,
        Err(code) => return code,
    };
    let remote_item: RemoteItem = match serde_json::from_str(&remote_item_json) {
        Ok(v) => v,
        Err(e) => {
            print_error_message(format, quiet, &format!("invalid --json remote item: {e}"));
            return ExitCode::InvalidArguments;
        }
    };
    let ctx = match open_context(format, quiet) {
        Ok(ctx) => ctx,
        Err(code) => return code,
    };
    match SourceService::import_remote_item(&ctx, id, remote_item) {
        Ok(item_id) => {
            match format {
                OutputFormat::Json | OutputFormat::Jsonl => print_json(
                    &serde_json::json!({ "schema_version": 1, "item_id": item_id.to_string() }),
                ),
                OutputFormat::Text | OutputFormat::Table => {
                    if !quiet {
                        println!("imported as item {item_id}");
                    }
                }
            }
            ExitCode::Success
        }
        Err(err) => report_and_exit(format, quiet, &err),
    }
}
