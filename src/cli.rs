use clap::{Parser, Subcommand};

use crate::cmd::*;

#[derive(Parser, Debug)]
#[command(about, version, author)]
pub struct Args {
    #[command(subcommand)]
    pub cmd: Command,

    #[arg(long, help = "")]
    pub debug: bool,
}

#[derive(Subcommand, Debug)]
pub enum Command {
    Clear(clear::Args),
    Extract(extract::Args),
    Init(init::Args),
}
