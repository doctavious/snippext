use crate::{ClearOpt, ExtractOpt, InitOpt};
use clap::{Parser, Subcommand};

pub mod clear;
pub mod extract;
pub mod init;

#[derive(Subcommand, Debug)]
pub enum Command {
    Init(InitOpt),
    Extract(ExtractOpt),
    Clear(ClearOpt),
}
