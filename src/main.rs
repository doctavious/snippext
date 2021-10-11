use snippext::{run, SnippetSettings};
use structopt::StructOpt;
use config::{ConfigError, Config, File, Environment, Source, Value};
use std::env;
use std::collections::HashMap;

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

    let mut s = Config::default();
    // Start off by merging in the "default" configurations
    // s.merge(File::with_name("config/default"))?;
    // TODO: add defaults

    // TODO: use constant
    s.merge(File::with_name("snippet.yaml").required(false)).unwrap();

    // TODO: this can probably come from structopt?
    s.merge(Environment::with_prefix("snippext")).unwrap();

    // TODO: add any command line args
    // TODO: test that this works
    s.merge(opt).unwrap();

    let settings: SnippetSettings = s.try_into().unwrap();

    run(settings);

    // run(SnippetSettings::new(
    //     opt.comment_prefixes.0,
    //     opt.begin.to_owned(),
    //     opt.end.to_owned(),
    //     opt.output_dir,
    //     opt.extension.to_owned(),
    //     opt.template,
    //     opt.sources)
    // )
}

// https://github.com/viperproject/prusti-dev/blob/22a4eb83ef91391d9a91e6b3246ddf951b8eb251/prusti-common/src/config/commandline.rs#L97
impl Source for Opt {
    fn clone_into_box(&self) -> Box<dyn Source + Send + Sync> {
        Box::new((*self).clone())
    }

    fn collect(&self) -> Result<HashMap<String, Value>, ConfigError> {
        let mut m = HashMap::new();
        let uri: String = "command line".into();

        m.insert(String::from("begin"), Value::new(Some(&uri), self.begin.to_string()));
        // m.insert("end", ValueKind::String(&self.end));
        // m.insert("extension", ValueKind::String(&self.extension));
        m.insert(String::from("comment_prefixes"), Value::new(Some(&uri), self.comment_prefixes.0.clone()));
        // m.insert("begin", &self.begin);
        // m.insert("begin", &self.begin);

        // TODO: I dont think we can have automatic defaults on structopt as we wont be able to
        // properly determine if they were provided and if they should values should be overwritten

        Ok(m)
    }
}

// split into subcommands??
// 1. generate - output to dir
// 2. write - write to target files
// 3. clean - clean up generate or files

// use constants that can also be used as defaults
// https://github.com/TeXitoi/structopt/issues/226

// Only way I know how to get structopt default value to work with Vec is to use a struct
#[derive(Clone, Debug, PartialEq)]
struct CommentPrefixes(Vec<String>);

impl std::str::FromStr for CommentPrefixes {
    type Err = Box<dyn std::error::Error>;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(CommentPrefixes(s.split(",").map(|x| x.trim().to_owned()).collect()))
    }
}

// TODO: environment variable fallback
#[derive(Clone, StructOpt, Debug)]
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
