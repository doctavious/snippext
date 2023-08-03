use clap::Parser;
use tracing::{Level};
use tracing_subscriber;

use snippext::cmd::{clear, extract, init, Command};
use snippext::SnippextResult;


// TODO priorities
// 1. refactor to subcommands
//      . init - generate config with prompts
//      . extract - extract snippets
//      . clear - clear snippets from targets
// 2. updating target files should read lines instead of whole file - done
// 3. add file name and path attributes - done
// 4. we should keep snippet name when writing out snippet to taret file
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

// use constants that can also be used as defaults
// https://github.com/TeXitoi/structopt/issues/226

// TODO: add config.rs?

// TODO: environment variable fallback here or via config?
// should document it here regardless
#[derive(Parser, Debug)]
#[command(about, version, author)]
struct Opt {
    #[command(subcommand)]
    cmd: Command,

    #[arg(long, help = "")]
    pub debug: bool,
}

fn main() -> SnippextResult<()> {
    let opt: Opt = Opt::parse();

    init_logger(opt.debug);

    match opt.cmd {
        Command::Init(init_opt) => init::execute(init_opt),
        Command::Extract(extract_opt) => extract::execute(extract_opt),
        Command::Clear(clear_opt) => clear::execute(clear_opt),
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
