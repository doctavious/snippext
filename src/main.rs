use clap::{Parser};
use tracing::{Level};
use tracing_subscriber;

use snippext::cmd::{clear, extract, init};
use snippext::{cli, SnippextResult};
use snippext::cli::Command;


// TODO priorities
// 1. refactor to subcommands
//      . init - generate config with prompts
//      . extract - extract snippets
//      . clear - clear snippets from targets
// 2. updating target files should read lines instead of whole file - done
// 3. add file name and path attributes - done
// 4. we should keep snippet name when writing out snippet to target file
// 5. validate error messages
// 6. docs
// 7. publish to crates.io
// 8. create bin / script to download and install

// static DEFAULT_CONFIG: &'static str = include_str!("default_snippext.yaml");

// split into subcommands?? does extract combine generate and write?
// 1. generate - output to dir
// 2. write - write to target files
// 3. clear - clear up generate or files
// 4. init - generate config file


fn main() -> SnippextResult<()> {
    let opt = cli::Args::parse();

    init_logger(opt.debug);

    match opt.cmd {
        Command::Init(args) => init::execute(args),
        Command::Extract(args) => extract::execute(args),
        Command::Clear(args) => clear::execute(args),
    }
}

fn init_logger(debug: bool) {
    let level = if debug {
        Level::DEBUG
    } else {
        Level::INFO
    };

    tracing_subscriber::fmt().with_max_level(level).init();
}

#[cfg(test)]
mod tests {

}
