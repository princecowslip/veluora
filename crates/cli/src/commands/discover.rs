use application::{DiscoverReport, DiscoverService};
use domain::SourceId;

use super::{open_context, parse_uuid_arg, print_json, report_and_exit};
use crate::cli_args::OutputFormat;
use crate::exit_code::ExitCode;

/// Runs `fut` on a fresh single-purpose tokio runtime — matches
/// `commands::source`'s own copy of this helper; `discover` fans out
/// to connectors the same way `source browse` does, needing the same
/// one-off async escape hatch in an otherwise-synchronous CLI.
fn run_async<F: std::future::Future>(fut: F) -> F::Output {
    tokio::runtime::Runtime::new()
        .expect("could not start an async runtime")
        .block_on(fut)
}

pub fn run(
    format: OutputFormat,
    quiet: bool,
    query: String,
    source_ids: Vec<String>,
    limit_per_source: u32,
) -> ExitCode {
    let mut parsed_ids = Vec::with_capacity(source_ids.len());
    for raw in &source_ids {
        match parse_uuid_arg(format, quiet, "source id", raw) {
            Ok(uuid) => parsed_ids.push(SourceId(uuid)),
            Err(code) => return code,
        }
    }
    let source_filter = if parsed_ids.is_empty() {
        None
    } else {
        Some(parsed_ids.as_slice())
    };

    let ctx = match open_context(format, quiet) {
        Ok(ctx) => ctx,
        Err(code) => return code,
    };

    match run_async(DiscoverService::discover(
        &ctx,
        &query,
        source_filter,
        limit_per_source,
    )) {
        Ok(report) => {
            print_report(format, quiet, &report);
            ExitCode::Success
        }
        Err(err) => report_and_exit(format, quiet, &err),
    }
}

fn print_report(format: OutputFormat, quiet: bool, report: &DiscoverReport) {
    match format {
        OutputFormat::Json | OutputFormat::Jsonl => print_json(report),
        OutputFormat::Text | OutputFormat::Table => {
            if quiet {
                return;
            }
            println!(
                "{} hit(s) for \"{}\" across {} source(s)",
                report.hits.len(),
                report.query,
                report.sources.len()
            );
            for status in &report.sources {
                let ok = matches!(
                    status.status,
                    domain::ConnectorResult::Success(_) | domain::ConnectorResult::Partial(_)
                );
                if !ok || !status.unsupported_clauses.is_empty() {
                    let clauses = if status.unsupported_clauses.is_empty() {
                        String::new()
                    } else {
                        format!(
                            ", unsupported clauses: {}",
                            status.unsupported_clauses.join(", ")
                        )
                    };
                    println!(
                        "  ({}: {:?}{clauses})",
                        status.source_display_name, status.status
                    );
                }
            }
            // `*` marks a hit already present in the local library —
            // repurposing `search.rs`'s favorite-marker convention,
            // since Discover results have no favorite state of their
            // own to show.
            for hit in &report.hits {
                let mark = if hit.local_item_id.is_some() {
                    "*"
                } else {
                    " "
                };
                println!(
                    "{mark} {}  {:?}  {}",
                    hit.source_display_name, hit.item.media_type, hit.item.title
                );
            }
        }
    }
}
