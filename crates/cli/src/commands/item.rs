use application::{
    ComicService, ItemService, OpenTarget, PlaybackService, StoryService, UserStateService,
};
use domain::{ItemId, Progress};

use super::{open_context, parse_uuid_arg, print_error_message, print_json, report_and_exit};
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

pub fn open(
    format: OutputFormat,
    quiet: bool,
    item_id: String,
    player: Option<String>,
    no_launch: bool,
) -> ExitCode {
    let uuid = match parse_uuid_arg(format, quiet, "item id", &item_id) {
        Ok(u) => u,
        Err(code) => return code,
    };
    let ctx = match open_context(format, quiet) {
        Ok(ctx) => ctx,
        Err(code) => return code,
    };
    let target = match PlaybackService::resolve_open(&ctx, ItemId(uuid)) {
        Ok(target) => target,
        Err(err) => return report_and_exit(format, quiet, &err),
    };

    // Only video/audio resolve to an external-player target — everything
    // else (images, comics, stories) is print-only from the CLI, same as
    // `item show`; a future GUI/TUI renders those directly.
    let launch_result = match (&target, &player, no_launch) {
        (OpenTarget::ExternalPlayer { local_path, .. }, Some(player_path), false) => {
            let cmd = media::build_command(player_path, std::path::Path::new(local_path), None);
            Some(media::launch(&cmd))
        }
        _ => None,
    };

    match format {
        OutputFormat::Json | OutputFormat::Jsonl => print_json(&target),
        OutputFormat::Text | OutputFormat::Table => {
            if !quiet {
                println!("{target:?}");
            }
        }
    }

    if let Some(Err(e)) = launch_result {
        print_error_message(format, quiet, &format!("could not launch player: {e}"));
        return ExitCode::InvalidArguments;
    }
    ExitCode::Success
}

pub fn progress(
    format: OutputFormat,
    quiet: bool,
    item_id: String,
    progress_json: String,
    completed: Option<bool>,
) -> ExitCode {
    let uuid = match parse_uuid_arg(format, quiet, "item id", &item_id) {
        Ok(u) => u,
        Err(code) => return code,
    };
    let progress: Progress = match serde_json::from_str(&progress_json) {
        Ok(p) => p,
        Err(e) => {
            print_error_message(format, quiet, &format!("invalid progress json: {e}"));
            return ExitCode::InvalidArguments;
        }
    };
    let ctx = match open_context(format, quiet) {
        Ok(ctx) => ctx,
        Err(code) => return code,
    };
    match PlaybackService::record_progress(&ctx, ItemId(uuid), progress, completed) {
        Ok(state) => {
            match format {
                OutputFormat::Json | OutputFormat::Jsonl => print_json(&state),
                OutputFormat::Text | OutputFormat::Table => {
                    if !quiet {
                        println!(
                            "progress recorded for {item_id}: viewed={} completed={}",
                            state.viewed, state.completed
                        );
                    }
                }
            }
            ExitCode::Success
        }
        Err(err) => report_and_exit(format, quiet, &err),
    }
}

pub fn pages(format: OutputFormat, quiet: bool, item_id: String) -> ExitCode {
    let uuid = match parse_uuid_arg(format, quiet, "item id", &item_id) {
        Ok(u) => u,
        Err(code) => return code,
    };
    let ctx = match open_context(format, quiet) {
        Ok(ctx) => ctx,
        Err(code) => return code,
    };
    match ComicService::pages(&ctx, ItemId(uuid)) {
        Ok(pages) => {
            match format {
                OutputFormat::Json | OutputFormat::Jsonl => print_json(&pages),
                OutputFormat::Text | OutputFormat::Table => {
                    if !quiet {
                        println!("{} page(s)", pages.len());
                        for page in &pages {
                            println!("  {}: {} bytes", page.index, page.size);
                        }
                    }
                }
            }
            ExitCode::Success
        }
        Err(err) => report_and_exit(format, quiet, &err),
    }
}

pub fn read(format: OutputFormat, quiet: bool, item_id: String, chapter: Option<u32>) -> ExitCode {
    let uuid = match parse_uuid_arg(format, quiet, "item id", &item_id) {
        Ok(u) => u,
        Err(code) => return code,
    };
    let ctx = match open_context(format, quiet) {
        Ok(ctx) => ctx,
        Err(code) => return code,
    };
    let item_id = ItemId(uuid);
    let content = match StoryService::read_content(&ctx, item_id) {
        Ok(content) => content,
        Err(err) => return report_and_exit(format, quiet, &err),
    };

    let text = match chapter {
        None => content,
        Some(chapter_index) => {
            let doc = match StoryService::get(&ctx, item_id) {
                Ok(Some(doc)) => doc,
                Ok(None) => {
                    print_error_message(format, quiet, "no story document for this item");
                    return ExitCode::NotFound;
                }
                Err(err) => return report_and_exit(format, quiet, &err),
            };
            match slice_chapter(&content, &doc.chapter_map, chapter_index) {
                Some(slice) => slice,
                None => {
                    print_error_message(format, quiet, &format!("no chapter {chapter_index}"));
                    return ExitCode::InvalidArguments;
                }
            }
        }
    };

    match format {
        OutputFormat::Json | OutputFormat::Jsonl => {
            print_json(&serde_json::json!({ "schema_version": 1, "content": text }))
        }
        OutputFormat::Text | OutputFormat::Table => {
            if !quiet {
                println!("{text}");
            }
        }
    }
    ExitCode::Success
}

pub fn pin(format: OutputFormat, quiet: bool, item_id: String, pinned: bool) -> ExitCode {
    let uuid = match parse_uuid_arg(format, quiet, "item id", &item_id) {
        Ok(u) => u,
        Err(code) => return code,
    };
    let ctx = match open_context(format, quiet) {
        Ok(ctx) => ctx,
        Err(code) => return code,
    };
    match UserStateService::set_pinned(&ctx, ItemId(uuid), pinned) {
        Ok(state) => {
            match format {
                OutputFormat::Json | OutputFormat::Jsonl => print_json(&state),
                OutputFormat::Text | OutputFormat::Table => {
                    if !quiet {
                        let verb = if pinned { "pinned" } else { "unpinned" };
                        println!("{verb} {item_id}");
                    }
                }
            }
            ExitCode::Success
        }
        Err(err) => report_and_exit(format, quiet, &err),
    }
}

/// Slices `content` to the span covered by chapter `index`, using the
/// character offsets in `chapter_map` (a JSON array of
/// `{"title", "char_offset"}` objects, per `StoryDocument::chapter_map`).
fn slice_chapter(content: &str, chapter_map: &serde_json::Value, index: u32) -> Option<String> {
    let chapters = chapter_map.as_array()?;
    let start = chapters.get(index as usize)?.get("char_offset")?.as_u64()? as usize;
    let end = chapters
        .get(index as usize + 1)
        .and_then(|c| c.get("char_offset"))
        .and_then(|o| o.as_u64())
        .map(|o| o as usize);

    let chars: Vec<char> = content.chars().collect();
    let end = end.unwrap_or(chars.len()).min(chars.len());
    if start > end {
        return None;
    }
    Some(chars[start..end].iter().collect())
}
