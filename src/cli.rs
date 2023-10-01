use clap::{Parser, Subcommand};

use crate::cmd::*;

/// Snippext is a CLI use to extract snippets from source files and merge into your documentation.
#[derive(Parser, Debug)]
#[command(about, version, author)]
pub struct Args {

    #[command(subcommand)]
    pub cmd: Command,

    /// Print debugging information
    #[arg(long)]
    pub debug: bool,
}

/// Snippext CLI Commands
#[remain::sorted]
#[derive(Subcommand, Debug)]
pub enum Command {
    Clear(clear::Args),
    Extract(extract::Args),
    Init(init::Args),
}
