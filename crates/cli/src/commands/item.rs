use application::ItemService;
use domain::ItemId;

use super::{open_context, parse_uuid_arg, print_json, report_and_exit};
use crate::cli_args::OutputFormat;
use crate::exit_code::ExitCode;

pub fn show(format: OutputFormat, quiet: bool, item_id: String) -> ExitCode {
    let uuid = match parse_uuid_arg(format, quiet, "item id", &item_id) {
        Ok(u) => u,
        Err(code) => return code,
    };
    let ctx = match open_context(format, quiet) {
        Ok(ctx) => ctx,
        Err(code) => return code,
    };
    match ItemService::get(&ctx, ItemId(uuid)) {
        Ok(detail) => {
            match format {
                OutputFormat::Json | OutputFormat::Jsonl => print_json(&detail),
                OutputFormat::Text | OutputFormat::Table => {
                    if !quiet {
                        println!("{}", detail.title);
                        println!("  id:       {}", detail.id);
                        println!("  type:     {:?}", detail.media_type);
                        println!("  favorite: {}", detail.favorite);
                        if let Some(rating) = detail.rating {
                            println!("  rating:   {rating}");
                        }
                        if !detail.tags.is_empty() {
                            println!("  tags:     {}", detail.tags.join(", "));
                        }
                        for variant in &detail.variants {
                            println!(
                                "  variant {}: {} ({})",
                                variant.id,
                                variant.local_path.as_deref().unwrap_or("<no local file>"),
                                variant.mime_type
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
