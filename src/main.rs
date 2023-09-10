use clap::Parser;
use snippext::cli::Command;
use snippext::cmd::{clear, extract, init};
use snippext::{cli, SnippextResult};
use tracing::Level;
use tracing_subscriber;

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
    let level = if debug { Level::DEBUG } else { Level::INFO };

    tracing_subscriber::fmt().with_max_level(level).init();
}

#[cfg(test)]
mod tests {}
