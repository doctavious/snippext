pub mod clear;
pub mod extract;
pub mod init;

use clap::{Parser, Subcommand};

use crate::cmd::clear::ClearOpt;
use crate::cmd::extract::ExtractOpt;
use crate::cmd::init::InitOpt;


#[derive(Subcommand, Debug)]
pub enum Command {
    Init(InitOpt),
    Extract(ExtractOpt),
    Clear(ClearOpt),
}
