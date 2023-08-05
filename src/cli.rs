use clap::{Parser, Subcommand};
use crate::cmd::*;

// use constants that can also be used as defaults
// https://github.com/TeXitoi/structopt/issues/226

// TODO: add config.rs?

// TODO: environment variable fallback here or via config?
// should document it here regardless
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





