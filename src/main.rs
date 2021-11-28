use config::{Config, Environment, File, Source};
use snippext::{
    clear, ClearSettings,
    DEFAULT_SOURCE_FILES, DEFAULT_TEMPLATE_IDENTIFIER,
    extract,
    init, InitSettings,
    SnippetSource, SnippextResult, SnippextSettings, SnippextTemplate
};
use std::collections::HashMap;
use std::io::Write;
use structopt::StructOpt;

use std::path::PathBuf;

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

fn main() -> SnippextResult<()> {
    let opt: Opt = Opt::from_args();

    // TODO: add debug
    // if opt.debug {
    //     std::env::set_var("RUST_LOG", "debug");
    //     env_logger::init();
    // }

    match opt.cmd {
        Command::Init(initOpt) => {
            let init_settings = if !initOpt.default {
                None
            } else {
                Some(init_settings_from_prompt()?)
            };
            init(init_settings)
        }
        Command::Extract(extractOpt) => {
            let settings = build_settings(extractOpt)?;
            extract(settings)
        }
        Command::Clear(clearOpt) => {
            let settings = build_clear_settings(clearOpt)?;
            clear(settings)
        }
    }

}

fn init_settings_from_prompt() -> SnippextResult<SnippextSettings> {
    Ok(SnippextSettings {
        begin: "".to_string(),
        end: "".to_string(),
        extension: "".to_string(),
        comment_prefixes: Default::default(),
        templates: Default::default(),
        sources: vec![],
        output_dir: None,
        targets: None
    })
}

fn build_init_settings(opt: InitOpt) -> SnippextResult<InitSettings> {
    let mut s = Config::default();
    let mut settings: InitSettings = s.try_into()?;
    return Ok(settings);
}

fn build_clear_settings(opt: ClearOpt) -> SnippextResult<ClearSettings> {
    let mut s = Config::default();
    let mut settings: ClearSettings = s.try_into()?;
    return Ok(settings);
}

// https://stackoverflow.com/questions/27244465/merge-two-hashmaps-in-rust
// Precedence of options
// If you specify an option by using one of the environment variables described in this topic,
// it overrides any value loaded from a profile in the configuration file.
// If you specify an option by using a parameter on the AWS CLI command line, it overrides any
// value from either the corresponding environment variable or a profile in the configuration file.
// TODO: update fn to build_snippext_settings which should be extract settings?
fn build_settings(opt: ExtractOpt) -> SnippextResult<SnippextSettings> {
    let mut s = Config::default();

    if let Some(config) = opt.config {
        s.merge(File::from(config)).unwrap();
    } else {
        // TODO: use constant
        s.merge(File::with_name("snippext").required(false))
            .unwrap();
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

    // if let Some(template) = opt.template {
    //     s.set("template", template);
    // }

    if let Some(output_dir) = opt.output_dir {
        s.set("output_dir", output_dir);
    }

    if let Some(targets) = opt.targets {
        s.set("targets", targets);
    }

    let mut settings: SnippextSettings = s.try_into()?;

    if let Some(template) = opt.template {
        settings.templates = HashMap::from([(
            String::from(DEFAULT_TEMPLATE_IDENTIFIER),
            SnippextTemplate {
                content: template,
                default: true,
            },
        )]);
    }

    if let Some(repo_url) = opt.repository_url {
        let source = SnippetSource::new_remote(
            repo_url.to_string(),
            opt.repository_branch.unwrap(),
            opt.repository_commit.clone(),
            opt.repository_directory.clone(),
            opt.sources
                .unwrap_or(vec![String::from(DEFAULT_SOURCE_FILES)]),
        );
        settings.sources = vec![source];
    } else if let Some(sources) = opt.sources {
        let source = SnippetSource::new_local(sources);
        settings.sources = vec![source];
    }

    // TODO: should this override or merge?
    // settings.sources.push(snippet_source);

    return Ok(settings);
}

#[derive(StructOpt, Debug)]
enum Command {
    Init(InitOpt),
    Extract(ExtractOpt),
    Clear(ClearOpt),
}

// TODO: environment variable fallback here or via config?
// should document it here regardless
#[derive(StructOpt, Debug)]
#[structopt(about = "TODO: add some details")]
struct Opt {

    #[structopt(subcommand)]
    cmd: Command,
}

#[derive(Clone, StructOpt, Debug)]
#[structopt(about = "TODO: add some details")]
struct InitOpt {
    #[structopt(long, help = "TODO: ...")]
    pub default: bool,
}

#[derive(Clone, StructOpt, Debug)]
#[structopt(about = "TODO: add some details")]
struct ExtractOpt {
    #[structopt(short, long, parse(from_os_str), help = "Config file to use")]
    config: Option<PathBuf>,

    #[structopt(short, long, help = "flag to mark beginning of a snippet")]
    begin: Option<String>,

    #[structopt(short = "end", long, help = "flag to mark ending of a snippet")]
    end: Option<String>,

    #[structopt(short = "x", long, help = "extension for generated files")]
    extension: Option<String>,

    // TODO: make vec default to ["// ", "<!-- "]
    // The tag::[] and end::[] directives should be placed after a line comment as defined by the language of the source file.
    // comment prefix
    #[structopt(short = "p", long, help = "Prefixes to use for comments")]
    comment_prefixes: Option<Vec<String>>,

    #[structopt(short, long, help = "")]
    template: Option<String>,

    #[structopt(short, long, help = "")]
    repository_url: Option<String>,

    #[structopt(short = "B", long, requires = "repository_url", help = "")]
    repository_branch: Option<String>,

    #[structopt(short = "C", long, help = "")]
    repository_commit: Option<String>,

    #[structopt(short = "D", long, help = "Directory remote repository is cloned into")]
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
    #[structopt(short, long, help = "TODO: ...")]
    sources: Option<Vec<String>>,
}

#[derive(Clone, StructOpt, Debug)]
#[structopt(about = "TODO: add some details")]
struct ClearOpt {
    #[structopt(short, long, parse(from_os_str), help = "Config file to use")]
    config: Option<PathBuf>,

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
    use crate::{ExtractOpt, Opt};
    use std::collections::HashSet;
    use std::path::PathBuf;

    #[test]
    fn default_config_file() {
        let opt = ExtractOpt {
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
            sources: Some(vec![]),
        };

        let settings = super::build_settings(opt).unwrap();
        println!("{:?}", settings);
    }

    #[test]
    fn verify_cli_args() {
        let opt = ExtractOpt {
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
            output_dir: Some(String::from("./snippext/")),
            targets: Some(vec![String::from("README.md")]),
        };

        let settings = super::build_settings(opt).unwrap();

        assert_eq!("snippext::", settings.begin);
        assert_eq!("finish::", settings.end);
        assert_eq!("txt", settings.extension);
        assert_eq!(
            HashSet::from([String::from("# ")]),
            settings.comment_prefixes
        );

        assert_eq!(1, settings.templates.len());
        assert_eq!("default", settings.templates.keys().next().unwrap());
        let template = settings.templates.values().next().unwrap();
        assert_eq!("````\n{{snippet}}\n```", template.content);
        assert!(template.default);
        assert_eq!(Some(String::from("./snippext/")), settings.output_dir);
        assert_eq!(Some(vec![String::from("README.md")]), settings.targets);

        assert_eq!(1, settings.sources.len());
        let source = settings.sources.get(0).unwrap();
        assert_eq!(
            Some(String::from("https://github.com/doctavious/snippext.git")),
            source.repository
        );
        assert_eq!(Some(String::from("main")), source.branch);
        assert_eq!(
            Some(String::from("1883d49216b34baed67629c363b40da3ead770b8")),
            source.commit
        );
        assert_eq!(Some(String::from("docs")), source.directory);
        assert_eq!(vec![String::from("**/*.rs")], source.files);
    }

    #[test]
    fn support_overrides() {
        dotenv::from_path("./tests/.env.test").unwrap();

        let opt = ExtractOpt {
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
            sources: None,
        };

        let settings = super::build_settings(opt).unwrap();
        // env overrides config
        assert_eq!(
            Some(String::from("./generated-snippets/")),
            settings.output_dir
        );
        // cli arg overrides env
        assert_eq!("txt", settings.extension);
    }
}
