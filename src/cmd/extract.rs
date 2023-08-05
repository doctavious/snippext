use std::collections::{HashMap, HashSet};
use std::fs;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::time::SystemTime;

use chrono::{DateTime, Utc};
use clap::Parser;
use config::{Config, Environment};
use filetime::{set_file_mtime, FileTime};
use glob::{glob, Pattern};
use regex::Regex;
use reqwest::blocking::Client;
use reqwest::header::{HeaderValue, EXPIRES, LAST_MODIFIED};
use tempfile::TempDir;
use tracing::{info, warn};
use url::Url;
use walkdir::WalkDir;

use crate::constants::{DEFAULT_SOURCE_FILES, DEFAULT_TEMPLATE_IDENTIFIER};
use crate::error::SnippextError;
use crate::sanitize::sanitize;
use crate::templates::SnippextTemplate;
use crate::types::{LinkFormat, Snippet, SnippetSource};
use crate::{git, SnippextResult, SnippextSettings};

#[derive(Clone, Debug, Parser)]
#[command()]
pub struct Args {
    #[arg(short, long, value_parser, help = "Config file to use")]
    pub config: Option<PathBuf>,

    #[arg(short, long, help = "flag to mark beginning of a snippet")]
    pub begin: Option<String>,

    #[arg(short, long, help = "flag to mark ending of a snippet")]
    pub end: Option<String>,

    #[arg(
        short = 'x',
        long,
        help = "extension for generated files. Defaults to txt when not specified."
    )]
    pub extension: Option<String>,

    // TODO: make vec default to ["// ", "<!-- "]
    // The tag::[] and end::[] directives should be placed after a line comment as defined by the language of the source file.
    // comment prefix
    #[arg(
        short = 'p',
        long,
        value_name = "PREFIXES",
        help = "Prefixes to use for comments"
    )]
    pub comment_prefixes: Option<Vec<String>>,

    #[arg(
        short,
        long,
        value_name = "DIR",
        help = "Directory where templates exists. File names act as keys"
    )]
    pub templates: Option<String>,

    #[arg(short, long, value_name = "URL", help = "")]
    pub repository_url: Option<String>,

    #[arg(
        short = 'B',
        long,
        requires = "repository_url",
        value_name = "BRANCH",
        help = ""
    )]
    pub repository_branch: Option<String>,

    #[arg(short = 'C', long, value_name = "COMMIT", help = "")]
    pub repository_commit: Option<String>,

    // #[arg(short = 'D', long, value_name = "DIRECTORY", help = "Directory remote repository is cloned into")]
    // pub repository_directory: Option<String>,
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
    // value_delimiter(',')
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

struct SnippetExtractionState {
    pub key: String,
    pub start_line: usize,
    pub lines: String,
    pub attributes: HashMap<String, String>,
}

impl SnippetExtractionState {
    fn append_line(&mut self, line: &str) {
        self.lines.push_str(line)
    }
}

struct SourceFile {
    pub full_path: PathBuf,
    pub relative_path: PathBuf,
}

pub fn execute(extract_opt: Args) -> SnippextResult<()> {
    let settings = build_settings(extract_opt)?;
    extract(settings)
}

pub fn extract(snippext_settings: SnippextSettings) -> SnippextResult<()> {
    validate_snippext_settings(&snippext_settings)?;

    let source_files = get_source_files(&snippext_settings)?;
    for source_file in source_files {
        let snippets = extract_snippets(
            &snippext_settings.comment_prefixes,
            snippext_settings.begin.to_owned(),
            snippext_settings.end.to_owned(),
            source_file.full_path.as_path(),
        )?;

        if snippets.is_empty() {
            continue;
        }

        if let Some(output_dir) = &snippext_settings.output_dir {
            println!("output directory {} / {}", output_dir, &snippets.len());
            for snippet in &snippets {
                let x: &[_] = &['.', '/'];
                let output_path = Path::new(output_dir.as_str())
                    .join(
                        source_file
                            .relative_path
                            .to_string_lossy()
                            .trim_start_matches(x),
                    )
                    .join(sanitize(snippet.identifier.to_owned()))
                    .with_extension("txt");

                fs::create_dir_all(output_path.parent().unwrap()).unwrap();
                let result = SnippextTemplate::render_template(snippet, &snippext_settings, None)?;
                fs::write(output_path, result).unwrap();
            }
        }

        if let Some(targets) = &snippext_settings.targets {
            for target in targets {
                for snippet in &snippets {
                    update_target_file_snippet(
                        Path::new(target).to_path_buf(),
                        &snippet,
                        &snippext_settings,
                    )?;
                }
            }
        }
    }

    Ok(())
}

/// returns a list of validation failures
fn validate_snippext_settings(settings: &SnippextSettings) -> SnippextResult<()> {
    let mut failures = Vec::new();

    if settings.begin.is_empty() {
        failures.push(String::from("begin must not be an empty string"));
    }

    if settings.end.is_empty() {
        failures.push(String::from("end must not be an empty string"));
    }

    if settings.comment_prefixes.is_empty() {
        failures.push(String::from("comment_prefixes must not be empty"));
    }

    if settings.templates.is_empty() {
        failures.push(String::from("templates must not be empty"));
    } else {
        let mut default_templates = 0;
        for (i, template) in settings.templates.iter().enumerate() {
            if template.0.is_empty() {
                failures.push(format!(
                    "templates[{}].identifier must not be an empty string",
                    i
                ));
            }

            if template.1.content.is_empty() {
                failures.push(format!(
                    "templates[{}].content must not be an empty string",
                    i
                ));
            }

            if template.1.default {
                default_templates = default_templates + 1;
            }
        }

        if settings.templates.len() > 1 && default_templates == 0 {
            failures.push(String::from(
                "When multiple templates are defined one must be marked default",
            ));
        }

        if default_templates > 1 {
            failures.push(String::from(
                "templates must have only one marked as default",
            ));
        }
    }

    if settings.extension.is_empty() {
        failures.push(String::from("extension must not be an empty string"));
    }

    if settings.sources.is_empty() {
        failures.push(String::from("sources must not be empty"));
    } else {
        for (i, source) in settings.sources.iter().enumerate() {
            if source.files.is_empty() {
                failures.push(format!("sources[{}].files must not be empty", i));
            }

            if (source.repository.is_none() || source.repository.as_ref().unwrap() == "")
                && (source.cone_patterns.is_some()
                    || source.branch.is_some()
                    || source.commit.is_some())
            {
                failures.push(format!("sources[{}] specifies branch, commit, cone_patterns without specifying repository", i));
            }
        }
    }

    if settings.output_dir.is_none() && settings.targets.is_none() {
        failures.push(String::from("output_dir or targets is required"));
    }

    return if !failures.is_empty() {
        Err(SnippextError::ValidationError(failures))
    } else {
        Ok(())
    };
}

// TODO: we should instead be collecting snippets
// TODO: This code is absolute shit. Clean this up
// Need both the absolute path and the relative path so that for when we output generated files
// we only include relative directories within the output directory.
fn get_source_files(settings: &SnippextSettings) -> SnippextResult<Vec<SourceFile>> {
    let mut source_files: Vec<SourceFile> = Vec::new();

    for source in &settings.sources {
        if source.is_remote() {
            let repo = source.repository.as_ref().unwrap();
            let download_dir = TempDir::new()?.into_path().to_string_lossy().to_string();

            git::checkout_files(
                &repo,
                source.branch.clone(),
                source.cone_patterns.clone(),
                &download_dir,
            )?;

            let dir_length = download_dir.len();
            let patterns = source
                .files
                .iter()
                .map(|f| Pattern::new(f))
                .filter_map(|p| p.ok())
                .collect::<Vec<Pattern>>();

            for entry in WalkDir::new(&download_dir)
                .into_iter()
                .filter_map(|e| e.ok())
            {
                let path = &entry.path().to_string_lossy().to_string();
                let relative_path_str = &path[dir_length..];

                if entry.path().is_file() && patterns.iter().any(|p| p.matches(relative_path_str)) {
                    source_files.push(SourceFile {
                        full_path: entry.path().to_path_buf(),
                        relative_path: PathBuf::from(relative_path_str),
                    });
                }
            }
        } else if let Some(url) = &source.url {
            let path = download_url(url)?;
            source_files.push(SourceFile {
                full_path: path.clone(),
                relative_path: path.clone(),
            });
        } else {
            for file in &source.files {
                let paths = match glob(file.as_str()) {
                    Ok(paths) => paths,
                    Err(error) => {
                        return Err(SnippextError::GlobPatternError(format!(
                            "Glob pattern error for `{}`. {}",
                            file, error.msg
                        )))
                    }
                };

                for entry in paths {
                    let path = entry.unwrap();
                    let relative_path = if let Ok(prefix) = path.clone().strip_prefix(file.as_str())
                    {
                        prefix.to_path_buf()
                    } else {
                        path.clone()
                    };

                    if !path.is_dir() {
                        source_files.push(SourceFile {
                            full_path: path.clone(),
                            relative_path,
                        });
                    }
                }
            }
        }
    }

    Ok(source_files)
}

fn download_url(url: &String) -> SnippextResult<PathBuf> {
    let url_file_path = Url::from_str(url.as_str())?.to_file_path().map_err(|_| {
        SnippextError::GeneralError(format!("failed to convert url {} to file path", url))
    })?;

    if let Ok(file_metadata) = url_file_path.metadata() {
        let file_modified = file_metadata.modified().ok();
        if file_modified.is_some_and(|t| t > SystemTime::now()) {
            return Ok(url_file_path);
        }
    }

    let client = Client::new();
    let head = client.head(url).send()?;
    if !head.status().is_success() {
        // TODO: log
        return Err(SnippextError::GeneralError(format!(
            "Failed to download details from {}",
            url
        )));
    }

    if let Ok(file_metadata) = url_file_path.metadata() {
        if let Ok(file_created) = file_metadata.created() {
            let web_modified = header_to_systemtime(head.headers().get(LAST_MODIFIED));
            if web_modified.is_some_and(|t| t < file_created) {
                return Ok(url_file_path);
            }
        }

        fs::remove_file(&url_file_path)?;
    }

    let mut response = client.get(url).send()?;
    if response.status().is_success() {
        let mut file = File::create(&url_file_path)?;
        response.copy_to(&mut file)?;

        // TODO: do we need to do this? Would this be different then head?
        let web_expiration = header_to_systemtime(response.headers().get(EXPIRES));
        if let Some(expires) = web_expiration {
            set_file_mtime(&url_file_path, FileTime::from(expires))?;
        }
    }

    Ok(url_file_path)
}

fn header_to_systemtime(header_value: Option<&HeaderValue>) -> Option<SystemTime> {
    let header_value_str = header_value?.to_str().ok()?;
    let date_time: DateTime<Utc> = chrono::DateTime::from_str(header_value_str).ok()?;
    Some(date_time.into())
}

fn extract_snippets(
    comment_prefixes: &HashSet<String>,
    begin_pattern: String,
    end_pattern: String,
    path: &Path,
) -> SnippextResult<Vec<Snippet>> {
    let f = File::open(path)?;
    let reader = BufReader::new(f);

    let mut state = Vec::default();
    let mut current_line_number = 0;
    let mut snippets: Vec<Snippet> = Vec::new();

    for line in reader.lines() {
        current_line_number += 1;
        let l = line?;

        let begin_ident = matches(&l, comment_prefixes, &begin_pattern);
        if let Some(begin_ident) = begin_ident {
            let mut attributes = HashMap::from([
                ("path".to_string(), path.to_string_lossy().to_string()),
                (
                    "filename".to_string(),
                    path.file_name().unwrap().to_string_lossy().to_string(),
                ),
            ]);
            // TODO: I feel like this is the long hard way to do this...
            let last_square_bracket_pos = begin_ident.rfind('[');
            if let Some(last_square_bracket_pos) = last_square_bracket_pos {
                let identifier = &begin_ident.as_str()[..last_square_bracket_pos];
                attributes.extend(extract_attributes(begin_ident.as_str()));
                state.push(SnippetExtractionState {
                    key: identifier.to_string(),
                    start_line: current_line_number,
                    lines: String::new(),
                    attributes,
                });
            } else {
                state.push(SnippetExtractionState {
                    key: begin_ident,
                    start_line: current_line_number,
                    lines: String::new(),
                    attributes,
                });
            }

            continue;
        }

        // currently not in snippet
        if state.is_empty() {
            continue;
        }

        let end_ident = matches(&l, &comment_prefixes, &end_pattern);
        if end_ident.is_some() {
            if let Some(state) = state.pop() {
                snippets.push(Snippet::new(
                    state.key,
                    path.to_path_buf(),
                    state.lines,
                    state.attributes,
                    state.start_line,
                    current_line_number,
                ));
            }
        } else {
            for e in state.iter_mut() {
                e.append_line((l.clone() + "\n").as_str())
            }
        }
    }

    if !state.is_empty() {
        let snippet = state.pop().unwrap();
        return Err(SnippextError::GeneralError(format!(
            "Snippet '{}' was not closed in file {} starting at line {}",
            &snippet.key,
            &path.to_string_lossy(),
            &snippet.start_line
        )));
    }
    println!("snippets extracted from {:?}", &path.to_string_lossy());
    Ok(snippets)
}

// (?<pair>(?<key>.+?)(?:=)(?<value>[^=]+)(?:,|$))
/// Extract comma separated key value parts from source string
/// format [k=v,k2=v2]
fn extract_attributes(source: &str) -> HashMap<String, String> {
    let mut attributes = HashMap::new();
    let re = Regex::new("\\[([^]]+)]").unwrap();
    let captured_kv = re.captures(source);
    if captured_kv.is_some() {
        for kv in captured_kv.unwrap().get(1).unwrap().as_str().split(",") {
            let parts: Vec<&str> = kv.split("=").collect();
            if parts.len() == 2 {
                attributes.insert(
                    parts.get(0).unwrap().to_string(),
                    parts.get(1).unwrap().to_string(),
                );
            }
        }
    }

    attributes
}

// TODO: This should probably read lines instead of entire file content
//       currently we cant have the same snippet multiple times in the same file
// TODO: should look for same comment prefixes?
pub fn update_target_file_snippet(
    source: PathBuf,
    snippet: &Snippet,
    snippet_settings: &SnippextSettings,
) -> SnippextResult<()> {
    let mut source_content = fs::read_to_string(source.to_path_buf())?;
    update_target_string_snippet(&mut source_content, snippet, snippet_settings)?;
    fs::write(source.to_path_buf(), source_content)?;
    Ok(())
}

pub fn update_target_string_snippet(
    source: &mut String,
    snippet: &Snippet,
    snippet_settings: &SnippextSettings,
) -> SnippextResult<()> {
    for prefix in &snippet_settings.comment_prefixes {
        // TODO: create helper method for building prefix+being+ident string
        if let Some(snippet_start_index) = source.find(
            String::from(
                prefix.as_str().to_owned()
                    + snippet_settings.begin.as_str()
                    + snippet.identifier.as_str(),
            )
            .as_str(),
        ) {
            // TODO: extract attribute from snippet
            // TODO: should find/use template
            if let Some(snippet_start_tag_end_index) = source[snippet_start_index..].find("\n") {
                let snippet_include_start =
                    &source[snippet_start_index..snippet_start_index + snippet_start_tag_end_index];
                let attributes = if snippet_include_start.rfind('[').is_some() {
                    Some(extract_attributes(snippet_include_start))
                } else {
                    None
                };

                let result =
                    SnippextTemplate::render_template(snippet, snippet_settings, attributes)?;
                let content_starting_index = snippet_start_index + snippet_start_tag_end_index;
                let end_index = source
                    .find(
                        String::from(
                            prefix.as_str().to_owned()
                                + snippet_settings.end.as_str()
                                + snippet.identifier.as_str(),
                        )
                        .as_str(),
                    )
                    .unwrap_or(source.len());
                source.replace_range(
                    content_starting_index..end_index,
                    format!("\n{}", result).as_str(),
                );
            }
        }
    }
    Ok(())
}

// TODO: return tuple (prefix and identifier) or struct?
// Might not be necessary depending on how we want to enable doctavious
fn matches(s: &str, comment_prefixes: &HashSet<String>, pattern: &str) -> Option<String> {
    let trimmed = s.trim();
    let len_diff = s.len() - trimmed.len();
    for comment_prefix in comment_prefixes {
        let prefix = String::from(comment_prefix.as_str()) + pattern;
        if trimmed.starts_with(&prefix) {
            return Some(s[prefix.len() + len_diff..].to_string());
        }
    }
    None
}

// https://stackoverflow.com/questions/27244465/merge-two-hashmaps-in-rust
// Precedence of options
// If you specify an option by using one of the environment variables described in this topic,
// it overrides any value loaded from a profile in the configuration file.
// If you specify an option by using a parameter on the AWS CLI command line, it overrides any
// value from either the corresponding environment variable or a profile in the configuration file.
// TODO: update fn to build_snippext_settings which should be extract settings?
fn build_settings(opt: Args) -> SnippextResult<SnippextSettings> {
    let mut builder = Config::builder();

    if let Some(config) = opt.config {
        builder = builder.add_source(config::File::from(config));
    } else {
        // TODO: use constant
        builder = builder.add_source(config::File::with_name("snippext").required(false));
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
            return Err(SnippextError::GeneralError(format!(
                "Template {} does not exist",
                template
            )));
        }

        if !templates_path.is_dir() {
            return Err(SnippextError::GeneralError(format!(
                "Template {} should be a directory",
                template
            )));
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
                },
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
            // opt.repository_directory.clone(),
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
    use std::collections::{HashMap, HashSet};
    use std::path::PathBuf;

    use tracing::info;

    use super::Args;
    use crate::error::SnippextError;
    use crate::settings::SnippextSettings;
    use crate::templates::SnippextTemplate;
    use crate::types::SnippetSource;

    #[test]
    fn default_config_file() {
        let opt = Args {
            config: None,
            begin: None,
            end: None,
            extension: None,
            comment_prefixes: None,
            templates: None,
            repository_url: None,
            repository_branch: None,
            repository_commit: None,
            // repository_directory: None,
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
        let opt = Args {
            config: None,
            begin: Some(String::from("snippext::begin::")),
            end: Some(String::from("finish::")),
            extension: Some(String::from("txt")),
            comment_prefixes: Some(vec![String::from("# ")]),
            templates: Some(String::from("./tests/templates")),
            repository_url: Some(String::from("https://github.com/doctavious/snippext.git")),
            repository_branch: Some(String::from("main")),
            repository_commit: Some(String::from("1883d49216b34baed67629c363b40da3ead770b8")),
            // repository_directory: Some(String::from("docs")),
            sources: vec![String::from("**/*.rs")],
            url_sources: Vec::default(),
            output_dir: Some(String::from("./snippext/")),
            targets: vec![String::from("README.md")],
            link_format: None,
            url_prefix: None,
        };

        let settings = super::build_settings(opt).unwrap();

        assert_eq!("snippext::begin::", settings.begin);
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
        // assert_eq!(Some(String::from("docs")), source.directory);
        assert_eq!(vec![String::from("**/*.rs")], source.files);
    }

    #[test]
    fn support_overrides() {
        dotenv::from_path("./tests/.env.test").unwrap();

        let opt = Args {
            config: Some(PathBuf::from("./tests/custom_snippext.yaml")),
            begin: None,
            end: None,
            extension: Some(String::from("txt")),
            comment_prefixes: None,
            templates: None,
            repository_url: None,
            repository_branch: None,
            repository_commit: None,
            // repository_directory: None,
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

    // https://users.rust-lang.org/t/whats-the-rust-way-to-unit-test-for-an-error/23677/2
    #[test]
    fn strings_must_not_be_empty() {
        let settings = SnippextSettings::new(
            HashSet::from([String::from("# ")]),
            String::from(""),
            String::from(""),
            String::from(""),
            HashMap::from([(
                "".to_string(),
                SnippextTemplate {
                    content: "".to_string(),
                    default: false,
                },
            )]),
            vec![SnippetSource::new_local(vec![String::from("**")])],
            Some(String::from("./snippets/")),
            None,
            None,
            None,
        );

        let validation_result = super::extract(settings);
        let error = validation_result.err().unwrap();
        match error {
            SnippextError::ValidationError(failures) => {
                assert_eq!(5, failures.len());
                assert!(failures.contains(&String::from("begin must not be an empty string")));
                assert!(failures.contains(&String::from("end must not be an empty string")));
                assert!(failures.contains(&String::from(
                    "templates[0].identifier must not be an empty string"
                )));
                assert!(failures.contains(&String::from(
                    "templates[0].content must not be an empty string"
                )));
                assert!(failures.contains(&String::from("extension must not be an empty string")));
            }
            _ => {
                panic!("invalid SnippextError");
            }
        }
    }

    #[test]
    fn at_least_one_template_is_required() {
        let settings = SnippextSettings::new(
            HashSet::from([String::from("# ")]),
            String::from("snippet::start::"),
            String::from("snippet::end::"),
            String::from("md"),
            HashMap::new(),
            vec![SnippetSource::new_local(vec![String::from("**")])],
            Some(String::from("./snippets/")),
            None,
            None,
            None,
        );

        let validation_result = super::extract(settings);
        let error = validation_result.err().unwrap();
        match error {
            SnippextError::ValidationError(failures) => {
                assert_eq!(1, failures.len());
                assert_eq!(
                    String::from("templates must not be empty"),
                    failures.get(0).unwrap().to_string()
                );
            }
            _ => {
                panic!("invalid snippextError");
            }
        }
    }

    #[test]
    fn one_template_must_be_marked_default_when_multiple_templates_exist() {
        let settings = SnippextSettings::new(
            HashSet::from([String::from("# ")]),
            String::from("snippet::start::"),
            String::from("snippet::end::"),
            String::from("md"),
            HashMap::from([
                (
                    "first".to_string(),
                    SnippextTemplate {
                        content: String::from("{{snippet}}"),
                        default: false,
                    },
                ),
                (
                    "second".to_string(),
                    SnippextTemplate {
                        content: String::from("{{snippet}}"),
                        default: false,
                    },
                ),
            ]),
            vec![SnippetSource::new_local(vec![String::from("**")])],
            Some(String::from("./snippets/")),
            None,
            None,
            None,
        );

        let validation_result = super::extract(settings);
        let error = validation_result.err().unwrap();
        match error {
            SnippextError::ValidationError(failures) => {
                assert_eq!(1, failures.len());
                assert_eq!(
                    String::from("When multiple templates are defined one must be marked default"),
                    failures.get(0).unwrap().to_string()
                );
            }
            _ => {
                panic!("invalid snippextError");
            }
        }
    }

    #[test]
    fn multiple_default_templates_cannot_exist() {
        let settings = SnippextSettings::new(
            HashSet::from([String::from("# ")]),
            String::from("snippet::start::"),
            String::from("snippet::end::"),
            String::from("md"),
            HashMap::from([
                (
                    "first".to_string(),
                    SnippextTemplate {
                        content: String::from("{{snippet}}"),
                        default: true,
                    },
                ),
                (
                    "second".to_string(),
                    SnippextTemplate {
                        content: String::from("{{snippet}}"),
                        default: true,
                    },
                ),
            ]),
            vec![SnippetSource::new_local(vec![String::from("**")])],
            Some(String::from("./snippets/")),
            None,
            None,
            None,
        );

        let validation_result = super::extract(settings);
        let error = validation_result.err().unwrap();
        match error {
            SnippextError::ValidationError(failures) => {
                assert_eq!(1, failures.len());
                assert_eq!(
                    String::from("templates must have only one marked as default"),
                    failures.get(0).unwrap().to_string()
                );
            }
            _ => {
                panic!("invalid snippextError");
            }
        }
    }

    #[test]
    fn at_least_one_comment_prefix_is_required() {
        let settings = SnippextSettings::new(
            HashSet::new(),
            String::from("snippet::start::"),
            String::from("snippet::end::"),
            String::from("md"),
            HashMap::from([(
                "default".to_string(),
                SnippextTemplate {
                    content: String::from("{{snippet}}"),
                    default: true,
                },
            )]),
            vec![SnippetSource::new_local(vec![String::from("**")])],
            Some(String::from("./snippets/")),
            None,
            None,
            None,
        );

        let validation_result = super::extract(settings);
        let error = validation_result.err().unwrap();
        match error {
            SnippextError::ValidationError(failures) => {
                assert_eq!(1, failures.len());
                assert_eq!(
                    String::from("comment_prefixes must not be empty"),
                    failures.get(0).unwrap().to_string()
                );
            }
            _ => {
                panic!("invalid snippextError");
            }
        }
    }

    #[test]
    fn sources_must_not_be_empty() {
        let settings = SnippextSettings::new(
            HashSet::from([String::from("# ")]),
            String::from("snippet::start::"),
            String::from("snippet::end::"),
            String::from("md"),
            HashMap::from([(
                "default".to_string(),
                SnippextTemplate {
                    content: String::from("{{snippet}}"),
                    default: true,
                },
            )]),
            vec![],
            Some(String::from("./snippets/")),
            None,
            None,
            None,
        );

        let validation_result = super::extract(settings);
        let error = validation_result.err().unwrap();
        match error {
            SnippextError::ValidationError(failures) => {
                assert_eq!(1, failures.len());
                assert_eq!(
                    String::from("sources must not be empty"),
                    failures.get(0).unwrap().to_string()
                );
            }
            _ => {
                panic!("invalid snippextError");
            }
        }
    }

    #[test]
    fn sources_must_have_at_least_one_files_entry() {
        let settings = SnippextSettings::new(
            HashSet::from([String::from("# ")]),
            String::from("snippet::start::"),
            String::from("snippet::end::"),
            String::from("md"),
            HashMap::from([(
                "default".to_string(),
                SnippextTemplate {
                    content: String::from("{{snippet}}"),
                    default: true,
                },
            )]),
            vec![SnippetSource::new_local(vec![])],
            Some(String::from("./snippets/")),
            None,
            None,
            None,
        );

        let validation_result = super::extract(settings);
        let error = validation_result.err().unwrap();

        match error {
            SnippextError::ValidationError(failures) => {
                assert_eq!(1, failures.len());
                assert_eq!(
                    String::from("sources[0].files must not be empty"),
                    failures.get(0).unwrap().to_string()
                );
            }
            _ => {
                panic!("invalid SnippextError");
            }
        }
    }

    #[test]
    fn repository_must_be_provided_if_other_remote_sources_are_provided() {
        let settings = SnippextSettings::new(
            HashSet::from([String::from("# ")]),
            String::from("snippet::start::"),
            String::from("snippet::end::"),
            String::from("md"),
            HashMap::from([(
                "default".to_string(),
                SnippextTemplate {
                    content: String::from("{{snippet}}"),
                    default: true,
                },
            )]),
            vec![SnippetSource {
                repository: None,
                branch: Some(String::from("branch")),
                commit: Some(String::from("commit")),
                cone_patterns: None,
                files: vec![String::from("**")],
                url: None,
            }],
            Some(String::from("./snippets/")),
            None,
            None,
            None,
        );

        let validation_result = super::extract(settings);
        let error = validation_result.err().unwrap();
        match error {
            SnippextError::ValidationError(failures) => {
                assert_eq!(1, failures.len());
                assert_eq!(
                    String::from("sources[0] specifies branch, commit, cone_patterns without specifying repository"),
                    failures.get(0).unwrap().to_string()
                );
            }
            _ => {
                panic!("invalid SnippextError");
            }
        }
    }
}
