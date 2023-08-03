use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use clap::Parser;
use config::{Config, Environment, File};
use tracing::{info, warn};
use crate::{
    extract, LinkFormat, SnippetSource, SnippextResult, SnippextSettings, SnippextTemplate,
    DEFAULT_SOURCE_FILES, DEFAULT_TEMPLATE_IDENTIFIER,
};
use crate::error::SnippextError;

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
    #[arg(short = 'p', long, value_name = "PREFIXES", help = "Prefixes to use for comments")]
    pub comment_prefixes: Option<Vec<String>>,

    #[arg(short, long, value_name = "DIR", help = "Directory where templates exists. File names act as keys")]
    pub templates: Option<String>,

    #[arg(short, long, value_name = "URL",  help = "")]
    pub repository_url: Option<String>,

    #[arg(short = 'B', long, requires = "repository_url", value_name = "BRANCH", help = "")]
    pub repository_branch: Option<String>,

    #[arg(short = 'C', long, value_name = "COMMIT", help = "")]
    pub repository_commit: Option<String>,

    #[arg(short = 'D', long, value_name = "DIRECTORY", help = "Directory remote repository is cloned into")]
    pub repository_directory: Option<String>,

    #[arg(
        short,
        long,
        required_unless_present = "targets",
        help = "Directory in which the generated snippet files be will output to. Is required unless \
        targets is provided."
    )]
    pub output_dir: Option<String>,

    // globs
    #[arg(
        short = 'T',
        long,
        required_unless_present = "output_dir",
        help = "The local directories that contain the files to be spliced with the code snippets."
    )]
    pub targets: Vec<String>,

    // TODO: write to target files instead of output directory

    // aka files
    // list of globs and default to all??
    // default to **
    #[arg(short, long, help = "TODO: ...")]
    pub sources: Vec<String>,


    /// Urls to files to be included as snippets.
    /// Each url will be accessible using the file name as a key.
    /// Any snippets within the files will be extracted and accessible as individual keyed snippets.
    #[arg(long, help = "TODO: ...")]
    pub url_sources: Vec<String>,

    // value_parser
    #[arg(
        short = 'l',
        long,
        value_name = "FORMAT",
        value_enum,
        help = "Defines the format of snippet source links that appear under each snippet. Links \
        will not be included if not specified."
    )]
    pub link_format: Option<LinkFormat>,

    #[arg(
        long,
        help = "Allows string to be defined that will prefix all snippet source links. This is useful \
        when markdown files are hosted on a site that is not co-located with the source code files."
    )]
    pub url_prefix: Option<String>,
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

    if let Some(output_dir) = opt.output_dir {
        builder = builder.set_override("output_dir", output_dir)?;
    }

    if !opt.targets.is_empty() {
        builder = builder.set_override("targets", opt.targets)?;
    }

    if let Some(link_format) = opt.link_format {
        builder = builder.set_override("link_format", link_format.to_string())?;
    }

    if let Some(url_prefix) = opt.url_prefix {
        builder = builder.set_override("url_prefix", url_prefix)?;
    }

    let mut settings: SnippextSettings = builder.build()?.try_deserialize()?;

    if let Some(template) = opt.templates {
        let templates_path = Path::new(template.as_str());
        info!("template path {:?}", templates_path);
        if !templates_path.exists() {
            return Err(SnippextError::GeneralError(format!("Template {} does not exist", template)));
        }

        if !templates_path.is_dir() {
            return Err(SnippextError::GeneralError(format!("Template {} should be a directory", template)));
        }

        let mut templates = HashMap::new();
        for entry in fs::read_dir(templates_path)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                warn!("Skipping template {} because it's a directory", template);
                continue;
            }
            let Ok(content) = fs::read_to_string(&path) else {
                warn!("unable to read template file {:?}", &path);
                continue;
            };

            let Some(file_name) = path.file_stem() else {
                warn!("Unable to get file stem for {:?}", &path);
                continue;
            };

            templates.insert(
                file_name.to_string_lossy().into(),
                SnippextTemplate {
                    content,
                    default: file_name == DEFAULT_TEMPLATE_IDENTIFIER,
                }
            );
        }

        settings.templates = templates;
    }

    let mut snippet_sources = Vec::new();
    if let Some(repo_url) = opt.repository_url {
        let source_files = if opt.sources.is_empty() {
            vec![DEFAULT_SOURCE_FILES.into()]
        } else {
            opt.sources
        };

        let source = SnippetSource::new_git(
            repo_url.to_string(),
            opt.repository_branch.unwrap(),
            opt.repository_commit.clone(),
            opt.repository_directory.clone(),
            source_files,
        );
        snippet_sources.push(source);
    } else if !opt.sources.is_empty() {
        snippet_sources.push(SnippetSource::new_local(opt.sources));
    }

    for url_source in opt.url_sources {
        snippet_sources.push(SnippetSource::new_url(url_source));
    }

    settings.sources = snippet_sources;
    return Ok(settings);
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;
    use std::path::PathBuf;
    use tracing::info;
    use crate::cmd::ExtractOpt;

    #[test]
    fn default_config_file() {
        let opt = ExtractOpt {
            config: None,
            begin: None,
            end: None,
            extension: None,
            comment_prefixes: None,
            templates: None,
            repository_url: None,
            repository_branch: None,
            repository_commit: None,
            repository_directory: None,
            output_dir: None,
            targets: Vec::default(),
            sources: Vec::default(),
            url_sources: Vec::default(),
            link_format: None,
            url_prefix: None,
        };

        let settings = super::build_settings(opt).unwrap();
        // TODO: add asserts
        info!("{:?}", settings);
    }

    #[test]
    fn verify_cli_args() {
        let opt = ExtractOpt {
            config: None,
            begin: Some(String::from("snippext::")),
            end: Some(String::from("finish::")),
            extension: Some(String::from("txt")),
            comment_prefixes: Some(vec![String::from("# ")]),
            templates: Some(String::from("./tests/templates")),
            repository_url: Some(String::from("https://github.com/doctavious/snippext.git")),
            repository_branch: Some(String::from("main")),
            repository_commit: Some(String::from("1883d49216b34baed67629c363b40da3ead770b8")),
            repository_directory: Some(String::from("docs")),
            sources: vec![String::from("**/*.rs")],
            url_sources: Vec::default(),
            output_dir: Some(String::from("./snippext/")),
            targets: vec![String::from("README.md")],
            link_format: None,
            url_prefix: None,
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
            templates: None,
            repository_url: None,
            repository_branch: None,
            repository_commit: None,
            repository_directory: None,
            output_dir: None,
            targets: Vec::default(),
            sources: Vec::default(),
            url_sources: Vec::default(),
            link_format: None,
            url_prefix: None,
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
