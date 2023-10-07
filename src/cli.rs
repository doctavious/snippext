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
#[allow(clippy::large_enum_variant)]
#[remain::sorted]
#[derive(Subcommand, Debug)]
pub enum Command {
    /// Clear snippets from target files
    Clear(clear::Args),
    /// Extract snippets from sources and splice into target files / render to output directory
    Extract(extract::Args),
    /// Initialize Snippext configuration
    Init(init::Args),
}
