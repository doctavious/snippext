use clap::Parser;
use snippext::SnippextResult;
use std::io::Write;
use crate::cmd::clear::ClearOpt;
use crate::cmd::{clear, Command, extract, init};
use crate::cmd::extract::{ExtractOpt};
use crate::cmd::init::InitOpt;

mod cmd;

// TODO priorities
// 1. refactor to subcommands
//      . init - generate config with prompts
//      . extract - extract snippets
//      . clear - clear snippets from targets
// 2. updating target files should read lines instead of whole file
// 3. add file name and path attributes
// 4. validate error messages
// 5. docs
// 6. publish to crates.io
// 7. create bin / script to download and install




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

    #[arg(long, help = "TODO: ...")]
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
    // TODO: implement and then use logger instead of println

    // TODO: add debug
    // if opt.debug {
    //     std::env::set_var("RUST_LOG", "debug");
    //     env_logger::init();
    // }

    // let mut builder = Builder::new();
    //
    // builder.format(|formatter, record| {
    //     writeln!(
    //         formatter,
    //         "{} [{}] ({}): {}",
    //         Local::now().format("%Y-%m-%d %H:%M:%S"),
    //         record.level(),
    //         record.target(),
    //         record.args()
    //     )
    // });
    //
    // if let Ok(var) = env::var("RUST_LOG") {
    //     builder.parse_filters(&var);
    // } else {
    //     // if no RUST_LOG provided, default to logging at the Info level
    //     builder.filter(None, LevelFilter::Info);
    //     // Filter extraneous html5ever not-implemented messages
    //     builder.filter(Some("html5ever"), LevelFilter::Error);
    // }
    //
    // builder.init();
}



fn prompt(name:&str) -> String {
    let mut line = String::new();
    print!("{}", name);
    std::io::stdout().flush().unwrap();
    std::io::stdin().read_line(&mut line).expect("Error: Could not read a line");

    return line.trim().to_string()
}


// TODO: method to build ClearSettings from ClearOpt/CLI args

#[cfg(test)]
mod tests {
    use std::collections::HashSet;
    use std::path::PathBuf;
    use crate::cmd::extract::ExtractOpt;


}
