use snippext::{run, SnippetSettings};
use structopt::StructOpt;

fn main() {
    let opt: Opt = Opt::from_args();

    // TODO: add debug
    // if opt.debug {
    //     std::env::set_var("RUST_LOG", "debug");
    //     env_logger::init();
    // }


    // TODO: create settings by merging
    // config / default config
    // command line args
    // environment vars

    run(SnippetSettings::new(
        opt.comment_prefixes.0,
        opt.begin.to_owned(),
        opt.end.to_owned(),
        opt.output_dir,
        opt.extension.to_owned(),
        opt.template,
        opt.sources)
    )
}

// split into subcommands??
// 1. generate - output to dir
// 2. write - write to target files
// 3. clean - clean up generate or files

// use constants that can also be used as defaults
// https://github.com/TeXitoi/structopt/issues/226

// Only way I know how to get structopt default value to work with Vec is to use a struct
#[derive(Debug, PartialEq)]
struct CommentPrefixes(Vec<String>);

impl std::str::FromStr for CommentPrefixes {
    type Err = Box<dyn std::error::Error>;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(CommentPrefixes(s.split(",").map(|x| x.trim().to_owned()).collect()))
    }
}

// TODO: environment variable fallback
#[derive(StructOpt, Debug)]
#[structopt(about = "TODO: add some details")]
struct Opt {

    #[structopt(
        short,
        long,
        default_value = "snippet::",
        help = "flag to mark beginning of a snippet"
    )]
    begin: String,

    #[structopt(
        short = "end",
        long,
        default_value = "end::",
        help = "flag to mark ending of a snippet"
    )]
    end: String,

    #[structopt(
        short = "x",
        long,
        default_value = ".md",
        help = "extension for generated files"
    )]
    extension: String,

    // TODO: make vec default to ["// ", "<!-- "]
    // The tag::[] and end::[] directives should be placed after a line comment as defined by the language of the source file.
    // comment prefix
    #[structopt(
        short,
        long,
        default_value = "// , <!-- ",
        help = ""
    )]
    comment_prefixes: CommentPrefixes,

    #[structopt(
        short,
        long,
        default_value = "{{snippet}}",
        help = ""
    )]
    template: String,

    #[structopt(
        short,
        long,
        help = ""
    )]
    repository_url: Option<String>,

    #[structopt(
        short = "B",
        long,
        help = ""
    )]
    repository_branch: Option<String>,

    #[structopt(
        short = "C",
        long,
        help = ""
    )]
    repository_commit: Option<String>,

    #[structopt(
        short = "D",
        long,
        help = "Directory remote repository is cloned into"
    )]
    repository_directory: Option<String>,

    // TODO: require if for output_dir an targets. one must be provided.

    #[structopt(
        short,
        long,
        // default_value = "./snippets/",
        required_unless = "targets",
        help = "directory in which the files will be generated"
    )]
    output_dir: Option<String>,

    // globs
    #[structopt(
        short = "T",
        long,
        required_unless = "output_dir",
        help = "The local directories that contain the files to be spliced with the code snippets."
    )]
    targets: Option<Vec<String>>,

    // TODO: write to target files instead of output directory

    // aka files
    // list of globs and default to all??
    // default to **
    #[structopt(
        help = "TODO: ..."
    )]
    sources: Vec<String>,
}
