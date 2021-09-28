use snippext::extract;
use structopt::StructOpt;

fn main() {
    let opt: Opt = Opt::from_args();

    // TODO: add debug
    // if opt.debug {
    //     std::env::set_var("RUST_LOG", "debug");
    //     env_logger::init();
    // }

    extract(
        opt.comment_prefix.to_owned(),
        opt.begin.to_owned(),
        opt.end.to_owned(),
        opt.output_dir.to_owned(),
        opt.extension.to_owned(),
        opt.sources
    )
}


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
        short,
        long,
        default_value = "./snippets/",
        help = "directory in which the files will be generated"
    )]
    output_dir: String,

    #[structopt(
        short = "x",
        long,
        default_value = ".md",
        help = "extension for generated files"
    )]
    extension: String,

    // default to current directory
    sources: Vec<String>,

    // TODO: excludes
    // TODO: includes


    // TODO: make vec default to ["// ", "<!-- "]
    // The tag::[] and end::[] directives should be placed after a line comment as defined by the language of the source file.
    // comment prefix
    #[structopt(
        short,
        long,
        default_value = "// ",
        help = ""
    )]
    comment_prefix: String,
}
