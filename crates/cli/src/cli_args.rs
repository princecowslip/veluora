use std::path::PathBuf;

use clap::{Parser, Subcommand, ValueEnum};

/// The `veloura` command-line interface. See `docs/10-cli.md` for the
/// full, eventual command tree — this covers the Milestone A/B subset
/// (`doctor`, `db check`, `library`, `search`, `favorite`, `collection`,
/// `item`) plus the global options every future command will share.
#[derive(Parser)]
#[command(name = "veloura", version, about = "Veloura command-line interface")]
pub struct Cli {
    /// Path to a config file. Reserved: no config-file parsing exists
    /// yet, so this is accepted but currently unused.
    #[arg(long, global = true, value_name = "PATH")]
    pub config: Option<PathBuf>,

    #[arg(long, global = true, value_enum, default_value_t = OutputFormat::Text)]
    pub output: OutputFormat,

    #[arg(long, global = true)]
    pub no_color: bool,

    #[arg(long, global = true)]
    pub quiet: bool,

    #[arg(long, global = true)]
    pub verbose: bool,

    #[command(subcommand)]
    pub command: Command,
}

#[derive(Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum OutputFormat {
    Text,
    Json,
    Jsonl,
    Table,
}

#[derive(Subcommand)]
pub enum Command {
    /// Environment, configuration, and database sanity check.
    Doctor,
    /// Database maintenance commands.
    Db {
        #[command(subcommand)]
        action: DbAction,
    },
    /// Diagnostics and support-bundle export.
    Diagnostics {
        #[command(subcommand)]
        action: DiagnosticsAction,
    },
    /// Local library folder management and scanning.
    Library {
        #[command(subcommand)]
        action: LibraryAction,
    },
    /// Search the local library.
    Search {
        query: String,
        #[arg(long, default_value_t = 50)]
        limit: u32,
        #[arg(long, default_value_t = 0)]
        offset: u32,
    },
    /// Favorite/unfavorite an item.
    Favorite {
        #[command(subcommand)]
        action: FavoriteAction,
    },
    /// Manual collections.
    Collection {
        #[command(subcommand)]
        action: CollectionAction,
    },
    /// Item details.
    Item {
        #[command(subcommand)]
        action: ItemAction,
    },
    /// Plugin manifest validation and local registry governance
    /// (`docs/18-plugin-system.md`). Infrastructure only — there is no
    /// real connector-backed plugin to install yet (Milestone F,
    /// connectors, was skipped), so nothing here executes a plugin
    /// against real library data.
    Plugin {
        #[command(subcommand)]
        action: PluginAction,
    },
}

#[derive(Subcommand)]
pub enum DbAction {
    /// Verify the database is reachable and migrations are applied.
    Check,
    /// Hot-backs-up the live database to a file.
    Backup { path: PathBuf },
    /// Validates and restores from a backup file, replacing the live
    /// database. Restart `veloura`/the GUI afterward — this only
    /// replaces the file on disk, it doesn't affect an already-open
    /// process.
    Restore { path: PathBuf },
    /// Cache size breakdown (thumbnails/stories/other) and the current
    /// eviction quota, if one is set.
    CacheStatus,
    /// Sets or clears the cache eviction quota in bytes. Pass no value
    /// (with `--clear`) to remove it — an unset quota means unlimited,
    /// the default.
    CacheQuota {
        bytes: Option<u64>,
        #[arg(long)]
        clear: bool,
    },
    /// Evicts thumbnail files oldest-first (excluding any pinned item's)
    /// until under the configured quota. A no-op if no quota is set or
    /// the cache is already under it.
    CacheEnforceQuota,
}

#[derive(Subcommand)]
pub enum DiagnosticsAction {
    /// Exports a redacted, aggregate-only diagnostic snapshot — no
    /// titles, paths, tags, or notes. Prints to stdout unless `--file`
    /// is given. (Named `--file`, not `--output`, since `--output`
    /// is already the global text/json/jsonl/table format flag.)
    Bundle {
        #[arg(long)]
        file: Option<PathBuf>,
    },
}

#[derive(Subcommand)]
pub enum LibraryAction {
    /// Register a folder to scan.
    Add {
        path: PathBuf,
        #[arg(long = "name")]
        display_name: Option<String>,
    },
    /// List registered folder roots.
    List,
    /// Unregister a folder root. Detaches its files (clears their local
    /// path) without deleting the items, favorites, ratings, or
    /// collection membership built on top of them.
    Remove {
        root_id: String,
        /// Required — this is a destructive-to-confirm operation with no
        /// interactive prompt yet.
        #[arg(long)]
        yes: bool,
    },
    /// Scan every enabled root, or one specific root/path.
    Scan {
        /// Must already be a registered root (see `library add`).
        #[arg(long)]
        path: Option<PathBuf>,
    },
    /// Summary of registered roots and indexed item count.
    Status,
}

#[derive(Subcommand)]
pub enum FavoriteAction {
    Add { item_id: String },
    Remove { item_id: String },
}

#[derive(Subcommand)]
pub enum CollectionAction {
    Create {
        name: String,
        #[arg(long)]
        description: Option<String>,
    },
    List,
    Add {
        item_id: String,
        #[arg(long = "to")]
        collection_id: String,
    },
    Remove {
        item_id: String,
        #[arg(long = "from")]
        collection_id: String,
    },
}

#[derive(Subcommand)]
pub enum ItemAction {
    Show {
        item_id: String,
    },
    /// Resolves what opening the item means (local path, resume
    /// position, page count, or chapter map, depending on media type)
    /// and, for video/audio, launches an external player unless
    /// `--no-launch` is set.
    Open {
        item_id: String,
        /// External player binary to launch for video/audio items.
        #[arg(long)]
        player: Option<String>,
        /// Resolve only — never spawn a player process.
        #[arg(long)]
        no_launch: bool,
    },
    /// Records a playback/reading position.
    Progress {
        item_id: String,
        /// A `domain::Progress` JSON object, e.g.
        /// `{"progress_type":"time_based","position_ms":5000,"duration_ms":10000}`.
        #[arg(long = "json")]
        progress_json: String,
        /// Overrides the auto-derived completion flag.
        #[arg(long)]
        completed: Option<bool>,
    },
    /// Lists a comic/manga item's pages.
    Pages {
        item_id: String,
    },
    /// Prints a story item's sanitized content.
    Read {
        item_id: String,
        /// Print only this chapter, by its index in the chapter map.
        #[arg(long)]
        chapter: Option<u32>,
    },
    /// Pins (or unpins) an item, exempting its cached thumbnails from
    /// quota-driven eviction — see `db cache-enforce-quota`.
    Pin {
        item_id: String,
        #[arg(long)]
        unpin: bool,
    },
}

/// Mirrors `plugin_host::PluginStatus` — kept separate so `plugin-host`
/// (a library crate) doesn't need a `clap` dependency just for this.
#[derive(Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum PluginStatusArg {
    Stable,
    Beta,
    Degraded,
    Disabled,
    Removed,
}

impl From<PluginStatusArg> for plugin_host::PluginStatus {
    fn from(value: PluginStatusArg) -> Self {
        match value {
            PluginStatusArg::Stable => plugin_host::PluginStatus::Stable,
            PluginStatusArg::Beta => plugin_host::PluginStatus::Beta,
            PluginStatusArg::Degraded => plugin_host::PluginStatus::Degraded,
            PluginStatusArg::Disabled => plugin_host::PluginStatus::Disabled,
            PluginStatusArg::Removed => plugin_host::PluginStatus::Removed,
        }
    }
}

#[derive(Subcommand)]
pub enum PluginAction {
    /// Parses and validates a plugin manifest YAML file, printing its
    /// permission summary (`docs/18`'s "Permissions UI" fields).
    Validate { manifest_path: PathBuf },
    /// Lists every plugin in the local registry.
    RegistryList,
    /// Adds a manifest to the local registry under a given status.
    RegistryAdd {
        manifest_path: PathBuf,
        #[arg(long, value_enum, default_value_t = PluginStatusArg::Beta)]
        status: PluginStatusArg,
    },
    /// Transitions an already-registered plugin's status.
    RegistrySetStatus {
        id: String,
        #[arg(long, value_enum)]
        status: PluginStatusArg,
    },
}
