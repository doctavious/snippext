use crate::{ClearOpt, ExtractOpt, InitOpt};
use clap::{ArgMatches, Parser, Subcommand};

pub mod clear;
pub mod init;
pub mod extract;

#[derive(Subcommand, Debug)]
pub enum Command {
    Init(InitOpt),
    Extract(ExtractOpt),
    Clear(ClearOpt),
}
