use snippext::{run, SnippextSettings, SnippetSource, SnippextResult};
use structopt::StructOpt;
use config::{Config, File, Environment, Source};


use std::path::PathBuf;


// static DEFAULT_CONFIG: &'static str = include_str!("default_snippext.yaml");

// split into subcommands??
// 1. generate - output to dir
// 2. write - write to target files
// 3. clean - clean up generate or files

// use constants that can also be used as defaults
// https://github.com/TeXitoi/structopt/issues/226

fn main() -> SnippextResult<()> {
    let opt: Opt = Opt::from_args();

    // TODO: add debug
    // if opt.debug {
    //     std::env::set_var("RUST_LOG", "debug");
    //     env_logger::init();
    // }

    // https://stackoverflow.com/questions/27244465/merge-two-hashmaps-in-rust
    // Precedence of options
    // If you specify an option by using one of the environment variables described in this topic,
    // it overrides any value loaded from a profile in the configuration file.
    // If you specify an option by using a parameter on the AWS CLI command line, it overrides any
    // value from either the corresponding environment variable or a profile in the configuration file.


    let settings = build_settings(opt)?;

    return run(settings);
}

fn build_settings(opt: Opt) -> SnippextResult<SnippextSettings> {
    let mut s = Config::default();

    if let Some(config) = opt.config {
        s.merge(File::from(config)).unwrap();
    } else {
        // TODO: use constant
        s.merge(File::with_name("snippext").required(false)).unwrap();
    }

    // TODO: this can probably come from structopt?
    s.merge(Environment::with_prefix("snippext")).unwrap();

    if let Some(begin) = opt.begin {
        s.set("begin", begin);
    }

    if let Some(end) = opt.end {
        s.set("end", end);
    }

    if let Some(extension) = opt.extension {
        s.set("extension", extension);
    }

    if let Some(comment_prefixes) = opt.comment_prefixes {
        s.set("comment_prefixes", comment_prefixes);
    }

    if let Some(template) = opt.template {
        s.set("template", template);
    }

    if let Some(output_dir) = opt.output_dir {
        s.set("output_dir", output_dir);
    }

    if let Some(targets) = opt.targets {
        s.set("targets", targets);
    }

    let mut settings: SnippextSettings = s.try_into()?;

    let snippet_source= if let Some(repo_url) = opt.repository_url {
        SnippetSource::new_remote(
            repo_url.to_string(),
            opt.repository_branch.unwrap(),
            opt.repository_commit.clone(),
            opt.repository_directory.clone(),
            opt.sources.unwrap_or(Vec::new())
        )
    } else {
        SnippetSource::new_local(opt.sources.unwrap_or(Vec::new()))
    };

    // TODO: should this override or merge?
    settings.sources.push(snippet_source);

    return Ok(settings);
}

// TODO: environment variable fallback here or via config?
// should document it here regardless
#[derive(Clone, StructOpt, Debug)]
#[structopt(about = "TODO: add some details")]
struct Opt {

    #[structopt(
        short,
        long,
        parse(from_os_str),
        help = "Config file to use"
    )]
    config: Option<PathBuf>,

    #[structopt(
        short,
        long,
        help = "flag to mark beginning of a snippet"
    )]
    begin: Option<String>,

    #[structopt(
        short = "end",
        long,
        help = "flag to mark ending of a snippet"
    )]
    end: Option<String>,

    #[structopt(
        short = "x",
        long,
        help = "extension for generated files"
    )]
    extension: Option<String>,

    // TODO: make vec default to ["// ", "<!-- "]
    // The tag::[] and end::[] directives should be placed after a line comment as defined by the language of the source file.
    // comment prefix
    #[structopt(
        short = "p",
        long,
        help = "Prefixes to use for comments"
    )]
    comment_prefixes: Option<Vec<String>>,

    #[structopt(
        short,
        long,
        help = ""
    )]
    template: Option<String>,

    #[structopt(
        short,
        long,
        help = ""
    )]
    repository_url: Option<String>,

    #[structopt(
        short = "B",
        long,
        requires = "repository_url",
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
        short,
        long,
        help = "TODO: ..."
    )]
    sources: Option<Vec<String>>,
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use crate::{Opt};

    #[test]
    fn default_config_file() {
        let opt = Opt {
            config: None,
            begin: None,
            end: None,
            extension: None,
            comment_prefixes: None,
            template: None,
            repository_url: None,
            repository_branch: None,
            repository_commit: None,
            repository_directory: None,
            output_dir: None,
            targets: None,
            sources: Some(vec![])
        };

        let settings = super::build_settings(opt).unwrap();
        println!("{:?}", settings);
    }

    #[test]
    fn verify_cli_args() {
        let opt = Opt {
            config: None,
            begin: Some(String::from("snippext::")),
            end: Some(String::from("finish::")),
            extension: Some(String::from("txt")),
            comment_prefixes: Some(vec![String::from("# ")]),
            template: Some(String::from("````\n{{snippet}}\n```")),
            repository_url: Some(String::from("https://github.com/doctavious/snippext.git")),
            repository_branch: Some(String::from("main")),
            repository_commit: Some(String::from("1883d49216b34baed67629c363b40da3ead770b8")),
            repository_directory: Some(String::from("docs")),
            sources: Some(vec![String::from("**/*.rs")]),
            output_dir: Some(String::from("./snppext/")),
            targets: Some(vec![String::from("README.md")]),
        };

        let settings = super::build_settings(opt).unwrap();

        assert_eq!("snippext::", settings.begin);
        assert_eq!("finish::", settings.end);
        assert_eq!("txt", settings.extension);
        assert_eq!(vec![String::from("# ")], settings.comment_prefixes);
        assert_eq!("````\n{{snippet}}\n```", settings.template);
        assert_eq!(Some(String::from("./snppext/")), settings.output_dir);
        assert_eq!(Some(vec![String::from("README.md")]), settings.targets);

        assert_eq!(2, settings.sources.len());
        let source = settings.sources.get(1).unwrap();
        assert_eq!(Some(String::from("https://github.com/doctavious/snippext.git")), source.repository);
        assert_eq!(Some(String::from("main")), source.branch);
        assert_eq!(Some(String::from("1883d49216b34baed67629c363b40da3ead770b8")), source.commit);
        assert_eq!(Some(String::from("docs")), source.directory);
        assert_eq!(vec![String::from("**/*.rs")], source.files);
    }

    #[test]
    fn support_overrides() {
        dotenv::from_path("./tests/.env.test").unwrap();

        let opt = Opt {
            config: Some(PathBuf::from("./tests/custom_snippext.yaml")),
            begin: None,
            end: None,
            extension: Some(String::from("txt")),
            comment_prefixes: None,
            template: None,
            repository_url: None,
            repository_branch: None,
            repository_commit: None,
            repository_directory: None,
            output_dir: None,
            targets: None,
            sources: None
        };

        let settings = super::build_settings(opt).unwrap();
        // env overrides config
        assert_eq!(Some(String::from("./generated-snippets/")), settings.output_dir);
        // cli arg overrides env
        assert_eq!("txt", settings.extension);
    }


}
