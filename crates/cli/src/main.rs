mod cli_args;
mod commands;
mod exit_code;

use clap::Parser;
use cli_args::{Cli, Command, DbAction};

fn main() {
    let cli = Cli::parse();

    let code = match cli.command {
        Command::Doctor => commands::doctor(cli.output, cli.quiet),
        Command::Db {
            action: DbAction::Check,
        } => commands::db_check(cli.output, cli.quiet),
    };

    std::process::exit(code.as_i32());
}
