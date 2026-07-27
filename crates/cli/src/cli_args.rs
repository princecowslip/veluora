use std::path::PathBuf;

use clap::{Parser, Subcommand, ValueEnum};

/// The `veloura` command-line interface. See `docs/10-cli.md` for the
/// full, eventual command tree — this milestone implements the
/// foundational subset (`doctor`, `db check`) plus the global options
/// every future command will share.
#[derive(Parser)]
#[command(name = "veloura", version, about = "Veloura command-line interface")]
pub struct Cli {
    /// Path to a config file. Reserved: Milestone A does not yet parse a
    /// config file, so this is accepted but currently unused.
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
}

#[derive(Subcommand)]
pub enum DbAction {
    /// Verify the database is reachable and migrations are applied.
    Check,
}
