use clap::{Parser, Subcommand};

use crate::cmd::*;

/// Snippext is a CLI use to extract snippets from source files and merge into your documentation.
#[derive(Parser, Debug)]
#[command(about, version, author)]
pub struct Args {
    #[command(subcommand)]
    pub cmd: Command,

    #[arg(long, help = "")]
    pub debug: bool,
}

#[remain::sorted]
#[derive(Subcommand, Debug)]
pub enum Command {
    Clear(clear::Args),
    Extract(extract::Args),
    Init(init::Args),
}
