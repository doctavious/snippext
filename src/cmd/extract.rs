use std::collections::HashMap;
use clap::Parser;
use std::path::PathBuf;
use config::{Config, Environment, File};
use snippext::{
    DEFAULT_SOURCE_FILES, DEFAULT_TEMPLATE_IDENTIFIER,
    extract, SnippetSource, SnippextResult, SnippextSettings, SnippextTemplate
};


#[derive(Clone, Debug, Parser)]
#[command()]
pub struct ExtractOpt {
    #[arg(short, long, value_parser, help = "Config file to use")]
    pub config: Option<PathBuf>,

    #[arg(short, long, help = "flag to mark beginning of a snippet")]
    pub begin: Option<String>,

    #[arg(short, long, help = "flag to mark ending of a snippet")]
    pub end: Option<String>,

    #[arg(short = 'x', long, help = "extension for generated files")]
    pub extension: Option<String>,

    // TODO: make vec default to ["// ", "<!-- "]
    // The tag::[] and end::[] directives should be placed after a line comment as defined by the language of the source file.
    // comment prefix
    #[arg(short = 'p', long, help = "Prefixes to use for comments")]
    pub comment_prefixes: Option<Vec<String>>,

    // TODO: update?
    #[arg(short, long, help = "")]
    pub template: Option<String>,

    #[arg(short, long, help = "")]
    pub repository_url: Option<String>,

    #[arg(short = 'B', long, requires = "repository_url", help = "")]
    pub repository_branch: Option<String>,

    #[arg(short = 'C', long, help = "")]
    pub repository_commit: Option<String>,

    #[arg(short = 'D', long, help = "Directory remote repository is cloned into")]
    pub repository_directory: Option<String>,

    // TODO: require if for output_dir an targets. one must be provided.
    #[arg(
        short,
        long,
        required_unless_present = "targets",
        help = "directory in which the files will be generated"
    )]
    pub output_dir: Option<String>,

    // globs
    #[arg(
        short = 'T',
        long,
        required_unless_present = "output_dir",
        help = "The local directories that contain the files to be spliced with the code snippets."
    )]
    pub targets: Option<Vec<String>>,

    // TODO: write to target files instead of output directory

    // aka files
    // list of globs and default to all??
    // default to **
    #[arg(short, long, help = "TODO: ...")]
    pub sources: Option<Vec<String>>,
}

pub fn execute(extract_opt: ExtractOpt) -> SnippextResult<()> {
    let settings = build_settings(extract_opt)?;
    extract(settings)
}

// https://stackoverflow.com/questions/27244465/merge-two-hashmaps-in-rust
// Precedence of options
// If you specify an option by using one of the environment variables described in this topic,
// it overrides any value loaded from a profile in the configuration file.
// If you specify an option by using a parameter on the AWS CLI command line, it overrides any
// value from either the corresponding environment variable or a profile in the configuration file.
// TODO: update fn to build_snippext_settings which should be extract settings?
fn build_settings(opt: ExtractOpt) -> SnippextResult<SnippextSettings> {
    let mut builder = Config::builder();

    if let Some(config) = opt.config {
        builder = builder.add_source(File::from(config));
    } else {
        // TODO: use constant
        builder = builder.add_source(File::with_name("snippext").required(false));
    }

    builder = builder.add_source(Environment::with_prefix("snippext"));

    if let Some(begin) = opt.begin {
        builder = builder.set_override("begin", begin)?;
    }

    if let Some(end) = opt.end {
        builder = builder.set_override("end", end)?;
    }

    if let Some(extension) = opt.extension {
        builder = builder.set_override("extension", extension)?;
    }

    if let Some(comment_prefixes) = opt.comment_prefixes {
        builder = builder.set_override("comment_prefixes", comment_prefixes)?;
    }

    // if let Some(template) = opt.template {
    //     s.set("template", template);
    // }

    if let Some(output_dir) = opt.output_dir {
        builder = builder.set_override("output_dir", output_dir)?;
    }

    if let Some(targets) = opt.targets {
        builder = builder.set_override("targets", targets)?;
    }

    let mut settings: SnippextSettings = builder.build()?.try_deserialize()?;

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


#[cfg(test)]
mod tests {
    use std::collections::HashSet;
    use std::path::PathBuf;
    use crate::ExtractOpt;

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
        // TODO: add asserts
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
