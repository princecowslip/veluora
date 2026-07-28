mod cli_args;
mod commands;
mod exit_code;

use clap::Parser;
use cli_args::{
    Cli, CollectionAction, Command, DbAction, DiagnosticsAction, FavoriteAction, ItemAction,
    LibraryAction,
};

fn main() {
    let cli = Cli::parse();
    let format = cli.output;
    let quiet = cli.quiet;

    let code = match cli.command {
        Command::Doctor => commands::doctor(format, quiet),
        Command::Db { action } => match action {
            DbAction::Check => commands::db_check(format, quiet),
            DbAction::Backup { path } => commands::db_backup(format, quiet, path),
            DbAction::Restore { path } => commands::db_restore(format, quiet, path),
            DbAction::CacheStatus => commands::db_cache_status(format, quiet),
            DbAction::CacheQuota { bytes, clear } => {
                commands::db_cache_quota(format, quiet, bytes, clear)
            }
            DbAction::CacheEnforceQuota => commands::db_cache_enforce_quota(format, quiet),
        },
        Command::Diagnostics { action } => match action {
            DiagnosticsAction::Bundle { file } => {
                commands::diagnostics::bundle(format, quiet, file)
            }
        },
        Command::Library { action } => match action {
            LibraryAction::Add { path, display_name } => {
                commands::library::add(format, quiet, path, display_name)
            }
            LibraryAction::List => commands::library::list(format, quiet),
            LibraryAction::Remove { root_id, yes } => {
                commands::library::remove(format, quiet, root_id, yes)
            }
            LibraryAction::Scan { path } => commands::library::scan(format, quiet, path),
            LibraryAction::Status => commands::library::status(format, quiet),
        },
        Command::Search {
            query,
            limit,
            offset,
        } => commands::search::run(format, quiet, query, limit, offset),
        Command::Favorite { action } => match action {
            FavoriteAction::Add { item_id } => commands::favorite::add(format, quiet, item_id),
            FavoriteAction::Remove { item_id } => {
                commands::favorite::remove(format, quiet, item_id)
            }
        },
        Command::Collection { action } => match action {
            CollectionAction::Create { name, description } => {
                commands::collection::create(format, quiet, name, description)
            }
            CollectionAction::List => commands::collection::list(format, quiet),
            CollectionAction::Add {
                item_id,
                collection_id,
            } => commands::collection::add_item(format, quiet, item_id, collection_id),
            CollectionAction::Remove {
                item_id,
                collection_id,
            } => commands::collection::remove_item(format, quiet, item_id, collection_id),
        },
        Command::Item { action } => match action {
            ItemAction::Show { item_id } => commands::item::show(format, quiet, item_id),
            ItemAction::Open {
                item_id,
                player,
                no_launch,
            } => commands::item::open(format, quiet, item_id, player, no_launch),
            ItemAction::Progress {
                item_id,
                progress_json,
                completed,
            } => commands::item::progress(format, quiet, item_id, progress_json, completed),
            ItemAction::Pages { item_id } => commands::item::pages(format, quiet, item_id),
            ItemAction::Read { item_id, chapter } => {
                commands::item::read(format, quiet, item_id, chapter)
            }
            ItemAction::Pin { item_id, unpin } => {
                commands::item::pin(format, quiet, item_id, !unpin)
            }
        },
    };

    std::process::exit(code.as_i32());
}
