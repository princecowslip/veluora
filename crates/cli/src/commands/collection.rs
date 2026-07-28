use application::CollectionService;
use domain::{CollectionId, ItemId};

use super::{open_context, parse_uuid_arg, print_json, report_and_exit};
use crate::cli_args::OutputFormat;
use crate::exit_code::ExitCode;

pub fn create(
    format: OutputFormat,
    quiet: bool,
    name: String,
    description: Option<String>,
) -> ExitCode {
    let ctx = match open_context(format, quiet) {
        Ok(ctx) => ctx,
        Err(code) => return code,
    };
    match CollectionService::create(&ctx, &name, description.as_deref()) {
        Ok(collection) => {
            match format {
                OutputFormat::Json | OutputFormat::Jsonl => print_json(&collection),
                OutputFormat::Text | OutputFormat::Table => {
                    if !quiet {
                        println!(
                            "created collection \"{}\" ({})",
                            collection.name, collection.id
                        );
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
    match CollectionService::list(&ctx) {
        Ok(collections) => {
            match format {
                OutputFormat::Json | OutputFormat::Jsonl => print_json(&collections),
                OutputFormat::Text | OutputFormat::Table => {
                    if !quiet {
                        if collections.is_empty() {
                            println!("no collections");
                        }
                        for c in &collections {
                            println!("{}  {}", c.id, c.name);
                        }
                    }
                }
            }
            ExitCode::Success
        }
        Err(err) => report_and_exit(format, quiet, &err),
    }
}

pub fn add_item(
    format: OutputFormat,
    quiet: bool,
    item_id: String,
    collection_id: String,
) -> ExitCode {
    modify_item(format, quiet, item_id, collection_id, true)
}

pub fn remove_item(
    format: OutputFormat,
    quiet: bool,
    item_id: String,
    collection_id: String,
) -> ExitCode {
    modify_item(format, quiet, item_id, collection_id, false)
}

fn modify_item(
    format: OutputFormat,
    quiet: bool,
    item_id: String,
    collection_id: String,
    add: bool,
) -> ExitCode {
    let item_uuid = match parse_uuid_arg(format, quiet, "item id", &item_id) {
        Ok(u) => u,
        Err(code) => return code,
    };
    let collection_uuid = match parse_uuid_arg(format, quiet, "collection id", &collection_id) {
        Ok(u) => u,
        Err(code) => return code,
    };
    let ctx = match open_context(format, quiet) {
        Ok(ctx) => ctx,
        Err(code) => return code,
    };
    let result = if add {
        CollectionService::add_item(&ctx, CollectionId(collection_uuid), ItemId(item_uuid))
    } else {
        CollectionService::remove_item(&ctx, CollectionId(collection_uuid), ItemId(item_uuid))
    };
    match result {
        Ok(()) => {
            match format {
                OutputFormat::Json | OutputFormat::Jsonl => {
                    print_json(&serde_json::json!({ "schema_version": 1, "ok": true }))
                }
                OutputFormat::Text | OutputFormat::Table => {
                    if !quiet {
                        let verb = if add { "added to" } else { "removed from" };
                        println!("{item_id} {verb} collection {collection_id}");
                    }
                }
            }
            ExitCode::Success
        }
        Err(err) => report_and_exit(format, quiet, &err),
    }
}
