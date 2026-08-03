use application::{DownloadService, DownloadSummary, PrivacyService, SettingsService};
use domain::{Download, DownloadId, ItemId, VariantId};
use uuid::Uuid;

use super::{open_context, print_error_message, print_json, report_and_exit};
use crate::cli_args::OutputFormat;
use crate::exit_code::ExitCode;

/// Runs `fut` on a fresh single-purpose tokio runtime — matches
/// `commands::source`/`commands::discover`'s own copies of this
/// helper; `add`/`resume` need real async I/O (the download's HTTP
/// fetch), everything else in this module is synchronous.
fn run_async<F: std::future::Future>(fut: F) -> F::Output {
    tokio::runtime::Runtime::new()
        .expect("could not start an async runtime")
        .block_on(fut)
}

fn parse_item_id(format: OutputFormat, quiet: bool, raw: &str) -> Result<ItemId, ExitCode> {
    Uuid::parse_str(raw).map(ItemId).map_err(|_| {
        print_error_message(format, quiet, &format!("invalid item id: '{raw}'"));
        ExitCode::InvalidArguments
    })
}

fn parse_variant_id(format: OutputFormat, quiet: bool, raw: &str) -> Result<VariantId, ExitCode> {
    Uuid::parse_str(raw).map(VariantId).map_err(|_| {
        print_error_message(format, quiet, &format!("invalid variant id: '{raw}'"));
        ExitCode::InvalidArguments
    })
}

fn parse_download_id(format: OutputFormat, quiet: bool, raw: &str) -> Result<DownloadId, ExitCode> {
    Uuid::parse_str(raw).map(DownloadId).map_err(|_| {
        print_error_message(format, quiet, &format!("invalid download id: '{raw}'"));
        ExitCode::InvalidArguments
    })
}

fn print_download(format: OutputFormat, quiet: bool, download: &Download) {
    match format {
        OutputFormat::Json | OutputFormat::Jsonl => print_json(download),
        OutputFormat::Text | OutputFormat::Table => {
            if !quiet {
                println!(
                    "{}  {:?}  {}/{} bytes  checksum={:?}",
                    download.id,
                    download.state,
                    download.bytes_received,
                    download
                        .bytes_total
                        .map(|b| b.to_string())
                        .unwrap_or_else(|| "?".to_string()),
                    download.checksum_state,
                );
                if let Some(code) = &download.failure_code {
                    println!("  ({code})");
                }
            }
        }
    }
}

/// Queues then runs a download to completion (or a paused/failed
/// outcome) in one blocking call — the service call itself failing
/// (ineligible, not found) is the only thing that produces a non-success
/// exit code; `Paused`/`Failed` are reported outcomes, matching how
/// `source health-check` treats a non-`Healthy` result as still `Success`.
pub fn add(format: OutputFormat, quiet: bool, item_id: String, variant_id: String) -> ExitCode {
    let item_id = match parse_item_id(format, quiet, &item_id) {
        Ok(v) => v,
        Err(code) => return code,
    };
    let variant_id = match parse_variant_id(format, quiet, &variant_id) {
        Ok(v) => v,
        Err(code) => return code,
    };
    let ctx = match open_context(format, quiet) {
        Ok(ctx) => ctx,
        Err(code) => return code,
    };
    let queued = match DownloadService::add(&ctx, item_id, variant_id) {
        Ok(d) => d,
        Err(err) => return report_and_exit(format, quiet, &err),
    };
    match run_async(DownloadService::run(&ctx, queued.id)) {
        Ok(download) => {
            print_download(format, quiet, &download);
            ExitCode::Success
        }
        Err(err) => report_and_exit(format, quiet, &err),
    }
}

pub fn resume(format: OutputFormat, quiet: bool, download_id: String) -> ExitCode {
    let id = match parse_download_id(format, quiet, &download_id) {
        Ok(v) => v,
        Err(code) => return code,
    };
    let ctx = match open_context(format, quiet) {
        Ok(ctx) => ctx,
        Err(code) => return code,
    };
    // Repairs this one row if a killed `local-api`/GUI process left it
    // stuck `Active` (see `DownloadService::claim`'s doc comment) —
    // scoped to just this id, unlike `repair_stale_active`, since this
    // short-lived CLI process must not risk touching a row a still-live
    // sibling process genuinely owns.
    let _ =
        DownloadService::repair_if_stale(&ctx, id, DownloadService::DEFAULT_STALE_ACTIVE_THRESHOLD);
    match run_async(DownloadService::resume(&ctx, id)) {
        Ok(download) => {
            print_download(format, quiet, &download);
            ExitCode::Success
        }
        Err(err) => report_and_exit(format, quiet, &err),
    }
}

pub fn list(format: OutputFormat, quiet: bool, item: Option<String>) -> ExitCode {
    let item_id = match item {
        Some(raw) => match parse_item_id(format, quiet, &raw) {
            Ok(v) => Some(v),
            Err(code) => return code,
        },
        None => None,
    };
    let ctx = match open_context(format, quiet) {
        Ok(ctx) => ctx,
        Err(code) => return code,
    };
    match DownloadService::list(&ctx, item_id) {
        Ok(downloads) => {
            print_list(format, quiet, &downloads);
            ExitCode::Success
        }
        Err(err) => report_and_exit(format, quiet, &err),
    }
}

fn print_list(format: OutputFormat, quiet: bool, downloads: &[DownloadSummary]) {
    match format {
        OutputFormat::Json | OutputFormat::Jsonl => print_json(&downloads),
        OutputFormat::Text | OutputFormat::Table => {
            if quiet {
                return;
            }
            if downloads.is_empty() {
                println!("(no downloads)");
                return;
            }
            for summary in downloads {
                let source = summary.source_display_name.as_deref().unwrap_or("-");
                println!(
                    "{}  {:?}  {}  ({})  {}/{} bytes",
                    summary.download.id,
                    summary.download.state,
                    summary.item_title,
                    source,
                    summary.download.bytes_received,
                    summary
                        .download
                        .bytes_total
                        .map(|b| b.to_string())
                        .unwrap_or_else(|| "?".to_string()),
                );
            }
        }
    }
}

pub fn pause(format: OutputFormat, quiet: bool, download_id: String) -> ExitCode {
    let id = match parse_download_id(format, quiet, &download_id) {
        Ok(v) => v,
        Err(code) => return code,
    };
    let ctx = match open_context(format, quiet) {
        Ok(ctx) => ctx,
        Err(code) => return code,
    };
    match DownloadService::pause(&ctx, id) {
        Ok(()) => {
            if !quiet && format == OutputFormat::Text {
                println!("paused download {download_id}");
            }
            ExitCode::Success
        }
        Err(err) => report_and_exit(format, quiet, &err),
    }
}

pub fn cancel(format: OutputFormat, quiet: bool, download_id: String) -> ExitCode {
    let id = match parse_download_id(format, quiet, &download_id) {
        Ok(v) => v,
        Err(code) => return code,
    };
    let ctx = match open_context(format, quiet) {
        Ok(ctx) => ctx,
        Err(code) => return code,
    };
    match DownloadService::cancel(&ctx, id) {
        Ok(()) => {
            if !quiet && format == OutputFormat::Text {
                println!("canceled download {download_id}");
            }
            ExitCode::Success
        }
        Err(err) => report_and_exit(format, quiet, &err),
    }
}

pub fn remove(
    format: OutputFormat,
    quiet: bool,
    download_id: String,
    delete_file: bool,
) -> ExitCode {
    let id = match parse_download_id(format, quiet, &download_id) {
        Ok(v) => v,
        Err(code) => return code,
    };
    let ctx = match open_context(format, quiet) {
        Ok(ctx) => ctx,
        Err(code) => return code,
    };
    match DownloadService::remove(&ctx, id, delete_file) {
        Ok(()) => {
            if !quiet && format == OutputFormat::Text {
                println!("removed download {download_id}");
            }
            ExitCode::Success
        }
        Err(err) => report_and_exit(format, quiet, &err),
    }
}

pub fn pin(format: OutputFormat, quiet: bool, download_id: String, unpin: bool) -> ExitCode {
    let id = match parse_download_id(format, quiet, &download_id) {
        Ok(v) => v,
        Err(code) => return code,
    };
    let ctx = match open_context(format, quiet) {
        Ok(ctx) => ctx,
        Err(code) => return code,
    };
    match DownloadService::set_pinned(&ctx, id, !unpin) {
        Ok(()) => {
            if !quiet && format == OutputFormat::Text {
                let verb = if unpin { "unpinned" } else { "pinned" };
                println!("{verb} download {download_id}");
            }
            ExitCode::Success
        }
        Err(err) => report_and_exit(format, quiet, &err),
    }
}

pub fn eligibility(
    format: OutputFormat,
    quiet: bool,
    item_id: String,
    variant_id: String,
) -> ExitCode {
    let item_id = match parse_item_id(format, quiet, &item_id) {
        Ok(v) => v,
        Err(code) => return code,
    };
    let variant_id = match parse_variant_id(format, quiet, &variant_id) {
        Ok(v) => v,
        Err(code) => return code,
    };
    let ctx = match open_context(format, quiet) {
        Ok(ctx) => ctx,
        Err(code) => return code,
    };
    match DownloadService::check_eligibility(&ctx, item_id, variant_id) {
        Ok(report) => {
            match format {
                OutputFormat::Json | OutputFormat::Jsonl => print_json(&report),
                OutputFormat::Text | OutputFormat::Table => {
                    if !quiet {
                        if report.eligible {
                            println!("eligible");
                        } else {
                            println!("not eligible: {}", report.reasons.join("; "));
                        }
                    }
                }
            }
            ExitCode::Success
        }
        Err(err) => report_and_exit(format, quiet, &err),
    }
}

pub fn quota(format: OutputFormat, quiet: bool, bytes: Option<u64>, clear: bool) -> ExitCode {
    if bytes.is_some() && clear {
        print_error_message(
            format,
            quiet,
            "pass either a byte value or --clear, not both",
        );
        return ExitCode::InvalidArguments;
    }
    if bytes.is_none() && !clear {
        print_error_message(
            format,
            quiet,
            "pass a byte value, or --clear to remove the quota",
        );
        return ExitCode::InvalidArguments;
    }
    let ctx = match open_context(format, quiet) {
        Ok(ctx) => ctx,
        Err(code) => return code,
    };
    match SettingsService::set_download_quota_bytes(&ctx, bytes) {
        Ok(()) => {
            match format {
                OutputFormat::Json | OutputFormat::Jsonl => print_json(&serde_json::json!({
                    "schema_version": 1,
                    "ok": true,
                    "quota_bytes": bytes,
                })),
                OutputFormat::Text | OutputFormat::Table => {
                    if !quiet {
                        match bytes {
                            Some(b) => println!("download quota set to {b} bytes"),
                            None => println!("download quota cleared (unlimited)"),
                        }
                    }
                }
            }
            ExitCode::Success
        }
        Err(err) => report_and_exit(format, quiet, &err),
    }
}

pub fn enforce_quota(format: OutputFormat, quiet: bool) -> ExitCode {
    let ctx = match open_context(format, quiet) {
        Ok(ctx) => ctx,
        Err(code) => return code,
    };
    match PrivacyService::enforce_download_quota(&ctx) {
        Ok(report) => {
            match format {
                OutputFormat::Json | OutputFormat::Jsonl => print_json(&report),
                OutputFormat::Text | OutputFormat::Table => {
                    if !quiet {
                        println!(
                            "evicted {} file(s), {} bytes — {} bytes remaining",
                            report.evicted_files, report.evicted_bytes, report.remaining_bytes
                        );
                    }
                }
            }
            ExitCode::Success
        }
        Err(err) => report_and_exit(format, quiet, &err),
    }
}
