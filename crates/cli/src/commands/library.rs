use std::path::PathBuf;

use application::{LibraryRootService, LibraryService, ScanService};
use domain::LibraryRootId;

use super::{open_context, parse_uuid_arg, print_error_message, print_json, report_and_exit};
use crate::cli_args::OutputFormat;
use crate::exit_code::ExitCode;

pub fn add(
    format: OutputFormat,
    quiet: bool,
    path: PathBuf,
    display_name: Option<String>,
) -> ExitCode {
    let ctx = match open_context(format, quiet) {
        Ok(ctx) => ctx,
        Err(code) => return code,
    };
    match LibraryRootService::add(&ctx, &path, display_name) {
        Ok(root) => {
            match format {
                OutputFormat::Json | OutputFormat::Jsonl => print_json(&root),
                OutputFormat::Text | OutputFormat::Table => {
                    if !quiet {
                        println!("added library root {} ({})", root.path, root.id);
                    }
                }
            }
            ExitCode::Success
        }
        Err(err) => report_and_exit(format, quiet, &err),
    }
}

pub fn list(format: OutputFormat, quiet: bool) -> ExitCode {
    let ctx = match open_context(format, quiet) {
        Ok(ctx) => ctx,
        Err(code) => return code,
    };
    match LibraryRootService::list(&ctx) {
        Ok(roots) => {
            match format {
                OutputFormat::Json | OutputFormat::Jsonl => print_json(&roots),
                OutputFormat::Text | OutputFormat::Table => {
                    if !quiet {
                        if roots.is_empty() {
                            println!("no library roots registered");
                        }
                        for root in &roots {
                            println!("{}  {}  enabled={}", root.id, root.path, root.enabled);
                        }
                    }
                }
            }
            ExitCode::Success
        }
        Err(err) => report_and_exit(format, quiet, &err),
    }
}

pub fn remove(format: OutputFormat, quiet: bool, root_id: String, yes: bool) -> ExitCode {
    if !yes {
        print_error_message(
            format,
            quiet,
            "refusing to remove a library root without --yes",
        );
        return ExitCode::InvalidArguments;
    }
    let uuid = match parse_uuid_arg(format, quiet, "root id", &root_id) {
        Ok(u) => u,
        Err(code) => return code,
    };
    let ctx = match open_context(format, quiet) {
        Ok(ctx) => ctx,
        Err(code) => return code,
    };
    match LibraryRootService::remove(&ctx, LibraryRootId(uuid)) {
        Ok(()) => {
            match format {
                OutputFormat::Json | OutputFormat::Jsonl => {
                    print_json(&serde_json::json!({ "schema_version": 1, "ok": true }))
                }
                OutputFormat::Text | OutputFormat::Table => {
                    if !quiet {
                        println!("removed library root {root_id}");
                    }
                }
            }
            ExitCode::Success
        }
        Err(err) => report_and_exit(format, quiet, &err),
    }
}

pub fn scan(format: OutputFormat, quiet: bool, path: Option<PathBuf>) -> ExitCode {
    let ctx = match open_context(format, quiet) {
        Ok(ctx) => ctx,
        Err(code) => return code,
    };
    let result = match path {
        Some(p) => ScanService::scan_path(&ctx, &p),
        None => ScanService::scan_all(&ctx),
    };
    match result {
        Ok(report) => {
            match format {
                OutputFormat::Json | OutputFormat::Jsonl => print_json(&report),
                OutputFormat::Text | OutputFormat::Table => {
                    if !quiet {
                        for root in &report.roots {
                            if let Some(err) = &root.error {
                                println!("{}: error — {err}", root.path);
                                continue;
                            }
                            println!(
                                "{}: {} seen, {} added, {} updated, {} moved, {} missing, {} unsupported",
                                root.path,
                                root.files_seen,
                                root.added,
                                root.updated,
                                root.moved,
                                root.missing,
                                root.unsupported
                            );
                        }
                        if report.skipped_total > 0 {
                            println!("{} file(s) skipped", report.skipped_total);
                        }
                    }
                }
            }
            ExitCode::Success
        }
        Err(err) => report_and_exit(format, quiet, &err),
    }
}

pub fn status(format: OutputFormat, quiet: bool) -> ExitCode {
    let ctx = match open_context(format, quiet) {
        Ok(ctx) => ctx,
        Err(code) => return code,
    };
    match LibraryService::status(&ctx) {
        Ok(status) => {
            match format {
                OutputFormat::Json | OutputFormat::Jsonl => print_json(&status),
                OutputFormat::Text | OutputFormat::Table => {
                    if !quiet {
                        println!("roots: {}", status.root_count);
                        println!("items: {}", status.item_count);
                    }
                }
            }
            ExitCode::Success
        }
        Err(err) => report_and_exit(format, quiet, &err),
    }
}
