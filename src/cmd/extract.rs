use std::collections::hash_map::Entry;
use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::time::SystemTime;
use std::{env, fs};

use chrono::DateTime;
use clap::ArgAction::SetTrue;
use clap::Parser;
use config::{Config, Environment, FileFormat};
use filetime::{set_file_mtime, FileTime};
use glob::{glob, Pattern};
use regex::Regex;
use reqwest::blocking::Client;
use reqwest::header::{HeaderValue, EXPIRES, LAST_MODIFIED};
use serde_json::{json, Value};
use tracing::warn;
use url::Url;
use walkdir::WalkDir;

use crate::constants::{
    DEFAULT_GIT_BRANCH, DEFAULT_OUTPUT_FILE_EXTENSION, DEFAULT_SNIPPEXT_CONFIG,
    DEFAULT_SOURCE_FILES, DEFAULT_TEMPLATE_IDENTIFIER, SNIPPEXT,
};
use crate::error::SnippextError;
use crate::files::{SnippextComment, SnippextComments};
use crate::sanitize::sanitize;
use crate::templates::render_template;
use crate::types::{LinkFormat, MissingSnippet, MissingSnippetsBehavior, Snippet, SnippetSource};
use crate::{files, git, SnippextResult, SnippextSettings};

/// Extracts snippets from source files and outputs and/or splices them into target files.
#[derive(Clone, Debug, Parser)]
#[command()]
pub struct Args {
    /// Config file to use. If not provided the default Snippext configuration will be used
    #[arg(short, long, value_parser)]
    pub config: Option<PathBuf>,

    /// Prefix that marks the start of a snippet. If provided overrides start defined in configuration
    #[arg(short = 'S', long)]
    pub start: Option<String>,

    /// Prefix that marks the ending of a snippet. If provided overrides end defined in configuration
    #[arg(short, long)]
    pub end: Option<String>,

    /// Directory where Snippext templates exists used to render snippets.
    /// File names act as template identifiers which can be used in target files to pick which template
    /// should be used to render snippet.
    #[arg(short, long, value_name = "DIR")]
    pub templates: Option<String>,

    /// The repository to clone from
    #[arg(long, value_name = "REPO")]
    pub repository_url: Option<String>,

    /// Branch name to use during git clone
    #[arg(long, requires = "repository_url", value_name = "BRANCH")]
    pub repository_branch: Option<String>,

    /// A list of directories, space separated, to be included in the sparse checkout
    #[arg(long, requires = "repository_url", value_name = "PATTERN")]
    pub repository_cone_patterns: Option<Vec<String>>,

    /// Directory in which the generated snippet files be will output to. This is required unless
    /// `targets` is provided. Generated snippets will be rendered with the default template
    #[arg(short, long, value_name = "DIR", required_unless_present = "targets")]
    pub output_dir: Option<String>,

    /// Extension for generated files written to the output directory.
    /// Defaults to `md` when not specified.
    #[arg(short = 'x', long)]
    pub output_extension: Option<String>,

    /// List of glob patters, separated by spaces, that contain the files to be spliced
    /// with the code snippets.
    #[arg(
        short = 'T',
        long,
        required_unless_present = "output_dir",
        value_delimiter = ' '
    )]
    pub targets: Vec<String>,

    /// List of glob patterns, separated by space, to look for snippets. Not applicable for
    /// URL sources. Defaults to `**`.
    #[arg(short, long, value_delimiter = ' ')]
    pub sources: Vec<String>,

    /// List of URLs, separated by space, to download and extract snippets from.
    /// URLs must return raw text in order for snippets to be successfully extracted.
    #[arg(long, value_delimiter = ' ')]
    pub url_sources: Vec<String>,

    /// Defines the format of snippet source links that appear under each snippet.
    /// Source links for local sources will not be included if not specified.
    /// If not provided For git sources links will attempt to determine based on git repository url.
    #[arg(short = 'l', long, value_name = "FORMAT", value_enum, ignore_case = true)]
    pub link_format: Option<LinkFormat>,

    /// String that will prefix all local snippet source links. This is useful when markdown
    /// files are hosted on a site that is not co-located with the source code files.
    #[arg(long, value_name = "PREFIX")]
    pub source_link_prefix: Option<String>,

    /// Flag that determines whether source links will be omitted from being rendered
    #[arg(long, action = SetTrue)]
    pub omit_source_links: Option<bool>,

    /// Defined behavior for what to do when missing snippets are present.
    #[arg(short, long, value_name = "BEHAVIOR", value_enum, ignore_case = true)]
    pub missing_snippets_behavior: Option<MissingSnippetsBehavior>,

    /// Flag that determines whether nested snippet comments are included in parent snippets
    #[arg(long, action = SetTrue)]
    pub retain_nested_snippet_comments: Option<bool>,

    /// Flag that determines whether source file language should be autodetected. Language
    /// autodetect is used to set `lang` attribute that can be used in snippet templates.
    #[arg(long, action = SetTrue)]
    pub disable_language_autodetect: Option<bool>,

    /// Flag that determines whether ellipsis should be added to gaps when `select_lines` attribute
    /// is used to render snippets.
    #[arg(long, action = SetTrue)]
    pub selected_lines_include_ellipses: Option<bool>

}

struct SnippetExtractionState {
    pub key: String,
    pub start_line: usize,
    pub lines: String,
    pub attributes: HashMap<String, Value>,
    pub retain_nested_comments: bool,
}

impl SnippetExtractionState {
    fn append_line(&mut self, line: &str) {
        self.lines.push_str(line)
    }
}

#[derive(Debug)]
struct SourceFile {
    pub full_path: PathBuf,
    pub relative_path: PathBuf,
    pub source_link: SourceLink,
}

#[derive(Debug)]
struct SourceLink {
    source_link: String,
    link_format: Option<LinkFormat>,
}

impl SourceLink {
    pub fn new_local(
        path: &str,
        source_link_prefix: Option<&str>,
        link_format: Option<LinkFormat>,
    ) -> Self {
        let mut source_link = String::new();
        if let Some(source_link_prefix) = source_link_prefix {
            source_link.push_str(source_link_prefix);
            if !source_link.ends_with("/") {
                source_link.push('/');
            }
        }
        source_link.push_str(path);
        Self {
            source_link,
            link_format,
        }
    }

    pub fn new_git(
        repository: &str,
        branch: &str,
        path: &str,
        link_format: Option<LinkFormat>,
    ) -> Self {
        let mut source_link = repository.trim_end_matches(".git").to_string();
        if !source_link.ends_with("/") {
            source_link.push('/');
        }

        if let Some(link_format) = link_format {
            let blob_path = match link_format {
                LinkFormat::AzureRepos => "",
                LinkFormat::BitBucket => "raw/",
                LinkFormat::GitHub => "blob/",
                LinkFormat::GitLab => "-/blob/",
                LinkFormat::Gitea => "-/blob/",
                LinkFormat::Gitee => "blob/",
            };
            source_link.push_str(blob_path);
        }

        source_link.push_str(branch);
        source_link.push_str(path);

        Self {
            source_link,
            link_format,
        }
    }

    pub fn new_url(url: String) -> Self {
        Self {
            source_link: url,
            link_format: None,
        }
    }

    pub fn append_lines(&self, start_line: usize, end_line: usize) -> String {
        if let Some(link_format) = self.link_format {
            match link_format {
                LinkFormat::AzureRepos => {
                    format!(
                        "{}&line={}&lineEnd={}",
                        self.source_link, start_line, end_line
                    )
                }
                LinkFormat::BitBucket => {
                    format!("{}#lines={}:{}", self.source_link, start_line, end_line)
                }
                LinkFormat::GitHub => format!("{}#L{}-L{}", self.source_link, start_line, end_line),
                LinkFormat::GitLab => format!("{}#L{}-{}", self.source_link, start_line, end_line),
                LinkFormat::Gitea => format!("{}#L{}-L{}", self.source_link, start_line, end_line),
                LinkFormat::Gitee => format!("{}#L{}-{}", self.source_link, start_line, end_line),
            }
        } else {
            self.source_link.clone()
        }
    }
}

/// Entry point for `extract` CLI command
pub fn execute(extract_opt: Args) -> SnippextResult<()> {
    let settings = build_settings(extract_opt)?;
    extract(settings)
}

pub fn extract(snippext_settings: SnippextSettings) -> SnippextResult<()> {
    validate_snippext_settings(&snippext_settings)?;

    let trim_chars: &[_] = &['.', '/'];
    let extension = snippext_settings
        .output_extension
        .as_deref()
        .unwrap_or(DEFAULT_OUTPUT_FILE_EXTENSION);

    let mut snippets = HashMap::new();
    let mut snippet_ids = HashSet::new();
    // file / snippet prefix cache
    let mut cache = HashMap::new();

    for source in &snippext_settings.sources {
        let extracted_snippets =
            extract_snippets(source, &snippext_settings, &mut cache, &mut snippet_ids)?;

        if let Some(output_dir) = &snippext_settings.output_dir {
            let base_path = Path::new(output_dir.as_str());
            for (_, snippet) in &extracted_snippets {
                for identifier in snippext_settings.templates.keys() {
                    let output_path = base_path
                        .join(
                            snippet
                                .path
                                .to_string_lossy()
                                .trim_start_matches(trim_chars),
                        )
                        .join(format!(
                            "{}_{}",
                            sanitize(snippet.identifier.to_owned()),
                            identifier
                        ))
                        .with_extension(extension);

                    fs::create_dir_all(output_path.parent().unwrap()).unwrap();
                    let result = render_template(
                        Some(identifier),
                        snippet,
                        &snippext_settings,
                        None
                    )?;
                    fs::write(output_path, result).unwrap();
                }
            }
        }

        snippets.extend(extracted_snippets);
    }

    if let Some(targets) = &snippext_settings.targets {
        let mut missing_snippets = Vec::new();
        for target in targets {
            let globs = match glob(target.as_str()) {
                Ok(paths) => paths,
                Err(error) => {
                    return Err(SnippextError::GlobPatternError(format!(
                        "Glob pattern error for `{}`. {}",
                        target, error.msg
                    )))
                }
            };

            for entry in globs {
                let path = entry.unwrap();
                let target_file_missing_snippets =
                    process_target_file(path.as_path(), &snippets, &snippext_settings, &mut cache)?;
                missing_snippets.extend(target_file_missing_snippets);
            }
        }

        match snippext_settings.missing_snippets_behavior {
            MissingSnippetsBehavior::Fail => {
                return Err(SnippextError::MissingSnippetsError(missing_snippets));
            }
            MissingSnippetsBehavior::Warn => {
                for missing_snippet in missing_snippets {
                    warn!(
                        "Snippet {} missing in {:?} at line {}",
                        &missing_snippet.key, &missing_snippet.path, &missing_snippet.line_number
                    )
                }
            }
            MissingSnippetsBehavior::Ignore => {
                // do nothing
            }
        }
    }

    Ok(())
}

/// returns a list of validation failures
fn validate_snippext_settings(settings: &SnippextSettings) -> SnippextResult<()> {
    let mut failures = Vec::new();

    if settings.start.is_empty() {
        failures.push(String::from("start must not be an empty string"));
    }

    if settings.end.is_empty() {
        failures.push(String::from("end must not be an empty string"));
    }

    if settings.templates.is_empty() {
        failures.push(String::from("templates must not be empty"));
    } else {
        let mut has_default_template = false;
        for (i, template) in settings.templates.iter().enumerate() {
            if template.0.is_empty() {
                failures.push(format!(
                    "templates[{}] identifier must not be an empty string",
                    i
                ));
                continue;
            }

            if template.1.is_empty() {
                failures.push(format!(
                    "templates[{}] template must not be an empty string",
                    template.0
                ));
            }

            if template.0 == DEFAULT_TEMPLATE_IDENTIFIER {
                has_default_template = true
            }
        }

        if !has_default_template {
            failures.push(String::from("Must have one template named 'default'"));
        }
    }

    if settings.sources.is_empty() {
        failures.push(String::from("sources must not be empty"));
    } else {
        for (i, source) in settings.sources.iter().enumerate() {
            match source {
                SnippetSource::Local { files } => {
                    if files.is_empty() {
                        failures.push(format!("sources[{}].files must not be empty", i));
                    }
                }
                SnippetSource::Git {
                    repository: url,
                    files,
                    ..
                } => {
                    if url == "" {
                        failures.push(format!("sources[{}].url must not be empty", i));
                    }

                    if files.is_empty() {
                        failures.push(format!("sources[{}].files must not be empty", i));
                    }
                }
                _ => {}
            }
        }
    }

    if settings.output_dir.is_none() && settings.targets.is_none() {
        failures.push(String::from("output_dir or targets is required"));
    }

    if settings
        .output_extension
        .as_ref()
        .is_some_and(|e| e.is_empty())
    {
        failures.push(String::from("output_extension must not be an empty string"));
    }

    return if !failures.is_empty() {
        Err(SnippextError::ValidationError(failures))
    } else {
        Ok(())
    };
}

fn extract_snippets(
    source: &SnippetSource,
    settings: &SnippextSettings,
    cache: &mut HashMap<String, SnippextComments>,
    snippet_ids: &mut HashSet<String>,
) -> SnippextResult<HashMap<String, Snippet>> {
    let mut snippets = HashMap::new();
    match source {
        SnippetSource::Local { files } => {
            for file in files {
                let paths = glob(file.as_str()).map_err(|e| {
                    SnippextError::GlobPatternError(format!(
                        "Glob pattern error for `{}`. {}",
                        file, e.msg
                    ))
                })?;

                for entry in paths {
                    let path = entry.unwrap();
                    if !path.is_dir() {
                        let source_file = SourceFile {
                            full_path: path.clone(),
                            relative_path: path.clone(),
                            source_link: SourceLink::new_local(
                                path.to_string_lossy().as_ref(),
                                settings.source_link_prefix.as_deref(),
                                settings.link_format,
                            ),
                        };

                        let extracted_snippets =
                            extract_snippets_from_file(source_file, settings, cache, snippet_ids)?;

                        snippets.extend(extracted_snippets);
                    }
                }
            }
        }
        SnippetSource::Git {
            repository,
            branch,
            cone_patterns,
            files,
        } => {
            let repository_url =
                Url::from_str(repository).expect("Git repository must be a valid URL");
            let repo_name =
                Path::new(repository)
                    .file_stem()
                    .ok_or(SnippextError::GeneralError(format!(
                        "Could not get repository name from {}",
                        &repository
                    )))?;
            let download_dir = get_download_directory()?.join(repo_name);

            // dont need this second check but being safe
            if download_dir.exists() && download_dir.starts_with(std::env::temp_dir()) {
                fs::remove_dir_all(&download_dir)?;
            }

            fs::create_dir_all(&download_dir)?;
            git::checkout_files(
                &repository,
                branch.clone(),
                cone_patterns.clone(),
                &download_dir,
            )?;

            let dir_length = download_dir.to_string_lossy().len();
            let patterns = files
                .iter()
                .map(|f| Pattern::new(f))
                .filter_map(|p| p.ok())
                .collect::<Vec<Pattern>>();

            let branch = if let Some(branch) = branch {
                branch.clone()
            } else {
                git::abbrev_ref(Some(&download_dir)).unwrap_or(DEFAULT_GIT_BRANCH.to_string())
            };

            let link_format = settings.link_format.or_else(|| {
                let domain = repository_url.domain()?;
                LinkFormat::from_domain(domain)
            });

            for entry in WalkDir::new(&download_dir)
                .into_iter()
                .filter_map(Result::ok)
                .filter(|e| e.file_type().is_file())
            {
                let path = &entry.path().to_string_lossy().to_string();
                let relative_path_str = &path[dir_length..];

                if patterns.iter().any(|p| p.matches(relative_path_str)) {
                    let source_file = SourceFile {
                        full_path: entry.path().to_path_buf(),
                        relative_path: PathBuf::from(relative_path_str),
                        source_link: SourceLink::new_git(
                            repository,
                            branch.as_str(),
                            relative_path_str,
                            link_format,
                        ),
                    };

                    let extracted_snippets =
                        extract_snippets_from_file(source_file, settings, cache, snippet_ids)?;

                    snippets.extend(extracted_snippets);
                }
            }
        }
        SnippetSource::Url(url) => {
            let source_file = get_source_file_from_url(url)?;
            let extracted_snippets =
                extract_snippets_from_file(source_file, settings, cache, snippet_ids)?;

            snippets.extend(extracted_snippets);
        }
    }

    Ok(snippets)
}

fn get_download_directory() -> SnippextResult<PathBuf> {
    let snippext_dir = env::temp_dir().join("snippext");
    if !snippext_dir.exists() {
        fs::create_dir(&snippext_dir)?;
    }

    Ok(snippext_dir)
}

const INVALID_DIR_CHARS: [char; 14] = [
    '/', '\\', '?', '*', ':', '|', '"', '<', '>', ',', ';', '=', ' ', '.',
];

fn url_to_path(url_string: &String) -> SnippextResult<PathBuf> {
    let url = Url::from_str(url_string.as_str())?;
    let path: String = url
        .path()
        .chars()
        .map(|c| {
            if INVALID_DIR_CHARS.contains(&c) {
                '_'
            } else {
                c
            }
        })
        .collect();

    Ok(PathBuf::from(url.authority()).join(path))
}

fn get_source_file_from_url(url: &String) -> SnippextResult<SourceFile> {
    let url_file_path = url_to_path(url)?;
    let download_path = get_download_directory()?.join(&url_file_path);
    let parent_dirs = download_path.parent().ok_or(SnippextError::GeneralError(
        "could not create download directory".into(),
    ))?;
    fs::create_dir_all(parent_dirs)?;

    if let Ok(file_metadata) = download_path.metadata() {
        let file_modified = file_metadata.modified().ok();
        if file_modified.is_some_and(|t| t > SystemTime::now()) {
            return Ok(SourceFile {
                full_path: download_path,
                relative_path: url_file_path,
                source_link: SourceLink::new_url(url.to_string()),
            });
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

    if let Ok(file_metadata) = download_path.metadata() {
        if let Ok(file_created) = file_metadata.created() {
            let web_modified = header_to_systemtime(head.headers().get(LAST_MODIFIED));
            if web_modified.is_some_and(|t| t < file_created) {
                return Ok(SourceFile {
                    full_path: download_path,
                    relative_path: url_file_path,
                    source_link: SourceLink::new_url(url.to_string()),
                });
            }
        }

        fs::remove_file(&download_path)?;
    }

    let mut response = client.get(url).send()?;
    if response.status().is_success() {
        let mut file = File::create(&download_path)?;

        response.copy_to(&mut file)?;

        let web_expiration = header_to_systemtime(response.headers().get(EXPIRES));
        if let Some(expires) = web_expiration {
            set_file_mtime(&download_path, FileTime::from(expires))?;
        }
    }

    return Ok(SourceFile {
        full_path: download_path,
        relative_path: url_file_path,
        source_link: SourceLink::new_url(url.to_string()),
    });
}

// Wed, 16 Aug 2023 22:40:19 GMT
fn header_to_systemtime(header_value: Option<&HeaderValue>) -> Option<SystemTime> {
    let header_value_str = header_value?.to_str().ok()?;
    let date_time = DateTime::parse_from_rfc2822(&header_value_str).ok()?;
    Some(date_time.into())
}

fn extract_snippets_from_file(
    source_file: SourceFile,
    settings: &SnippextSettings,
    cache: &mut HashMap<String, SnippextComments>,
    snippet_ids: &mut HashSet<String>,
) -> SnippextResult<HashMap<String, Snippet>> {
    let f = File::open(&source_file.full_path)?;
    let reader = BufReader::new(f);

    let mut current_line_number = 0;
    let mut state: Vec<SnippetExtractionState> = Vec::new();
    let mut snippets = HashMap::new();
    let extension = files::extension_from_path(&source_file.full_path);

    let snippet_comments = match cache.entry(extension.clone()) {
        Entry::Occupied(entry) => entry.into_mut(),
        Entry::Vacant(entry) => entry.insert(SnippextComments::new(
            extension.as_str().clone(),
            settings.start.as_str(),
            settings.end.as_str(),
        )),
    };


    let language = if settings.enable_autodetect_language {
        match hyperpolyglot::detect(&source_file.full_path) {
            Ok(detection) => {
                detection.and_then(|x| Some(x.language().to_ascii_lowercase()))
            }
            Err(_) => {
                warn!("failed to detect language for file {}", &source_file.full_path.to_string_lossy());
                None
            }
        }
    } else {
        None
    };

    for line in reader.lines() {
        current_line_number += 1;
        let l = line?;
        let current_line = l.trim();

        if let Some(comment) = snippet_comments.is_line_start_snippet(current_line) {
            let mut attributes = HashMap::from([
                (
                    "path".to_string(),
                    Value::String(source_file.relative_path.to_string_lossy().to_string()),
                ),
                (
                    "filename".to_string(),
                    Value::String(
                        source_file
                            .full_path
                            .file_name()
                            .unwrap()
                            .to_string_lossy()
                            .to_string(),
                    ),
                ),
            ]);

            if let Some(language) = &language {
                attributes.insert("lang".to_string(), Value::String(language.clone()));
            }

            let Ok((key, snippet_attributes)) = extract_id_and_attributes(current_line, comment) else {
                // TODO: error
                continue;
            };

            if let Some(snippet_attributes) = snippet_attributes {
                attributes.extend(snippet_attributes);
            }

            let retain_nested_comments = attributes.get("retain_nested_snippet_comments")
                .and_then(|v| v.as_bool())
                .unwrap_or(settings.retain_nested_snippet_comments);

            if !state.is_empty() {
                for app_state in state.iter_mut() {
                    if app_state.retain_nested_comments {
                        app_state.append_line((l.clone() + "\n").as_str());
                    }
                }
            }

            state.push(SnippetExtractionState {
                key,
                start_line: current_line_number,
                lines: String::new(),
                attributes,
                retain_nested_comments
            });

            continue;
        }

        // currently not in snippet
        if state.is_empty() {
            continue;
        }

        if snippet_comments.is_line_end_snippet(current_line).is_some() {
            if let Some(snippet_extraction_state) = state.pop() {
                let id = snippet_extraction_state.key;

                snippets.insert(
                    id.clone(),
                    Snippet {
                        identifier: id.clone(),
                        path: source_file.relative_path.to_owned(),
                        text: snippet_extraction_state.lines,
                        attributes: snippet_extraction_state.attributes,
                        start_line: snippet_extraction_state.start_line,
                        end_line: current_line_number,
                        source_link: Some(
                            source_file
                                .source_link
                                .append_lines(snippet_extraction_state.start_line, current_line_number),
                        ),
                    },
                );

                let new_id = snippet_ids.insert(id.clone());
                if !new_id {
                    warn!("multiple snippets with id {} found", id.clone());
                }

                for app_state in state.iter_mut() {
                    if app_state.retain_nested_comments {
                        app_state.append_line((l.clone() + "\n").as_str());
                    }
                }
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
            &source_file.relative_path.to_string_lossy(),
            &snippet.start_line
        )));
    }

    Ok(snippets)
}

fn extract_id_and_attributes(
    line: &str,
    comment: &SnippextComment,
) -> SnippextResult<(String, Option<HashMap<String, Value>>)> {
    let regex_close = comment
        .start_close
        .as_ref()
        .and_then(|s| Some(format!("({}|$)", s)))
        .unwrap_or("$".to_string());

    let format = format!(
        "{}[ ]*(?P<key>[\\S]*)(?P<attributes>.*?){}",
        comment.start, regex_close
    );

    let re = Regex::new(&format).unwrap();
    let captures = re.captures(line);
    if let Some(capture_groups) = captures {
        let Some(key) = capture_groups.name("key") else {
            return Err(SnippextError::GeneralError(format!("could not extract key from {}", line)));
        };

        let attributes = if let Some(match_attributes) = capture_groups.name("attributes") {
            let attributes_str = match_attributes.as_str().trim();
            if attributes_str == "" {
                None
            } else {
                Some(serde_json::from_str(attributes_str)?)
            }
        } else {
            None
        };

        return Ok((key.as_str().to_string(), attributes));
    }

    Err(SnippextError::GeneralError(format!(
        "could not extract snippet details from {}",
        line
    )))
}

fn process_target_file(
    target: &Path,
    snippets: &HashMap<String, Snippet>,
    settings: &SnippextSettings,
    cache: &mut HashMap<String, SnippextComments>,
) -> SnippextResult<Vec<MissingSnippet>> {
    let mut new_file_lines = Vec::new();
    let mut updated = false;
    let mut in_current_snippet = None;
    let mut line_number = 0;
    let mut missing_snippets = Vec::new();
    let extension = files::extension_from_path(target);
    let snippet_comments = match cache.entry(extension.clone()) {
        Entry::Occupied(entry) => entry.into_mut(),
        Entry::Vacant(entry) => entry.insert(SnippextComments::new(
            extension.as_str().clone(),
            settings.start.as_str(),
            settings.end.as_str(),
        )),
    };

    let f = File::open(&target)?;
    let reader = BufReader::new(f);
    for line in reader.lines() {
        line_number = line_number + 1;
        let line = line?;
        let current_line = line.trim();

        if in_current_snippet.is_some() {
            if snippet_comments.is_line_end_snippet(current_line).is_some() {
                new_file_lines.push(line.clone());
                in_current_snippet = None;
            }

            continue;
        }

        new_file_lines.push(line.clone());

        let snippet_comment = snippet_comments.is_line_start_snippet(current_line);
        if snippet_comment.is_none() {
            continue;
        }

        let Ok((key, attributes)) = extract_id_and_attributes(current_line, snippet_comment.unwrap()) else {
            warn!("Failed to extract id/attributes from snippet. File {} line number {}",
                target.to_string_lossy(),
                line_number
            );
            continue;
        };

        let mut found = false;
        if let Some(snippet) = find_snippet(snippets, &key) {
            found = true;
            let result = render_template(None, &snippet, &settings, attributes)?;

            let result_lines: Vec<String> = result.lines().map(|s| s.to_string()).collect();
            new_file_lines.extend(result_lines);
            updated = true;
            in_current_snippet = Some(key.clone());
        }

        if !found {
            missing_snippets.push(MissingSnippet {
                key: key.clone(),
                line_number,
                path: target.to_owned(),
            });
        }
    }

    if let Some(in_current_snippet) = in_current_snippet {
        return Err(SnippextError::GeneralError(format!(
            "Expected to find end of snippet {}",
            in_current_snippet
        )));
    }

    if updated {
        fs::write(target.to_path_buf(), new_file_lines.join("\n"))?;
    }

    Ok(missing_snippets)
}

fn find_snippet(snippets: &HashMap<String, Snippet>, key: &String) -> Option<Snippet> {
    let snippet = snippets.get(key);
    if snippet.is_some() {
        return snippet.cloned();
    }

    if key.starts_with("http") {
        let source = get_source_file_from_url(key);
        match source {
            Ok(s) => {
                if let Ok(content) = fs::read_to_string(&s.full_path) {
                    let line_count = content.lines().count();
                    // TODO: I would like this build source link the same way in all spots
                    return Some(Snippet {
                        identifier: key.to_string(),
                        path: s.full_path.clone(),
                        text: content,
                        attributes: Default::default(),
                        start_line: 1,
                        end_line: line_count,
                        source_link: Some(key.to_string()),
                    });
                }
            }
            Err(e) => {
                warn!("Failed to download snippet with error {}", e);
                return None;
            }
        }
    }

    // TODO: only read from path if it was a source file
    if let Ok(content) = fs::read_to_string(key) {
        let line_count = content.lines().count();
        return Some(Snippet {
            identifier: key.to_string(),
            path: PathBuf::from(key),
            text: content,
            attributes: Default::default(),
            start_line: 1,
            end_line: line_count,
            source_link: Some(key.to_string()),
        });
    }

    return None;
}

fn build_settings(opt: Args) -> SnippextResult<SnippextSettings> {
    let mut builder = Config::builder();

    if let Some(config) = opt.config {
        builder = builder.add_source(config::File::from(config));
    } else {
        builder = builder
            .add_source(config::File::from_str(
                DEFAULT_SNIPPEXT_CONFIG,
                FileFormat::Yaml,
            ))
            .add_source(config::File::with_name(SNIPPEXT).required(false));
    }

    builder = builder
        .add_source(Environment::with_prefix(SNIPPEXT))
        .set_override_option("start", opt.start)?
        .set_override_option("end", opt.end)?
        .set_override_option("output_dir", opt.output_dir)?
        .set_override_option("output_extension", opt.output_extension)?
        .set_override_option("omit_source_links", opt.omit_source_links)?
        .set_override_option("retain_nested_snippet_comments", opt.retain_nested_snippet_comments)?
        .set_override_option("selected_lines_include_ellipses", opt.selected_lines_include_ellipses)?;

    if !opt.targets.is_empty() {
        builder = builder.set_override("targets", opt.targets)?;
    }

    if let Some(link_format) = opt.link_format {
        builder = builder.set_override("link_format", link_format.to_string())?;
    }

    if let Some(source_link_prefix) = opt.source_link_prefix {
        builder = builder.set_override("source_link_prefix", source_link_prefix)?;
    }

    if let Some(missing_snippets_behavior) = opt.missing_snippets_behavior {
        builder = builder.set_override(
            "missing_snippets_behavior",
            missing_snippets_behavior.to_string(),
        )?;
    }

    if opt.disable_language_autodetect.is_some_and(|disabled| disabled) {
        builder = builder.set_override("enable_autodetect_language", false)?;
    }


    if let Some(template) = opt.templates {
        let templates_path = Path::new(template.as_str());
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

            templates.insert(file_name.to_string_lossy().to_string(), content);
        }

        // might be a better way to do this but works for now
        let templates_json = serde_json::to_string(&json!({
            "templates": templates
        }))?;

        builder = builder.add_source(config::File::from_str(
            templates_json.as_str(),
            FileFormat::Json,
        ));
    }

    let mut snippet_sources = Vec::new();
    if let Some(repo_url) = opt.repository_url {
        let source_files = if opt.sources.is_empty() {
            vec![DEFAULT_SOURCE_FILES.into()]
        } else {
            opt.sources
        };

        let source = SnippetSource::Git {
            repository: repo_url.to_string(),
            branch: opt.repository_branch,
            cone_patterns: opt.repository_cone_patterns,
            files: source_files,
        };

        snippet_sources.push(source);
    } else if !opt.sources.is_empty() {
        snippet_sources.push(SnippetSource::Local { files: opt.sources });
    }

    for url_source in opt.url_sources {
        snippet_sources.push(SnippetSource::Url(url_source));
    }

    // might be a better way to do this but works for now
    let sources_json = serde_json::to_string(&json!({
        "sources": snippet_sources
    }))?;
    builder = builder.add_source(config::File::from_str(
        sources_json.as_str(),
        FileFormat::Json,
    ));

    let settings: SnippextSettings = builder.build()?.try_deserialize()?;

    return Ok(settings);
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::{Path, PathBuf};

    use indexmap::IndexMap;
    use tempfile::tempdir;
    use tracing_test::traced_test;

    use super::Args;
    use crate::cmd::extract::{MissingSnippetsBehavior, SourceLink};
    use crate::constants::{DEFAULT_START, DEFAULT_TEMPLATE_IDENTIFIER};
    use crate::error::SnippextError;
    use crate::settings::SnippextSettings;
    use crate::types::{LinkFormat, SnippetSource};

    #[test]
    fn verify_cli_args() {
        let args = Args {
            config: None,
            start: Some(String::from(DEFAULT_START)),
            end: Some(String::from("finish::")),
            templates: Some(String::from("./tests/templates")),
            repository_url: Some(String::from("https://github.com/doctavious/snippext.git")),
            repository_branch: Some(String::from("main")),
            repository_cone_patterns: None,
            sources: vec![String::from("**/*.rs")],
            url_sources: Vec::default(),
            output_dir: Some(String::from("./snippext/")),
            output_extension: Some(String::from("txt")),
            targets: vec![String::from("README.md")],
            link_format: Some(LinkFormat::GitHub),
            source_link_prefix: None,
            omit_source_links: None,
            missing_snippets_behavior: Some(MissingSnippetsBehavior::Warn),
            retain_nested_snippet_comments: None,
            disable_language_autodetect: None,
            selected_lines_include_ellipses: None,
        };

        let settings = super::build_settings(args).unwrap();

        assert_eq!("snippet::start", settings.start);
        assert_eq!("finish::", settings.end);
        assert_eq!(Some("txt".into()), settings.output_extension);
        assert_eq!(2, settings.templates.len());

        let default_template = settings.templates.get(DEFAULT_TEMPLATE_IDENTIFIER).unwrap();
        assert_eq!("```\n{{snippet}}\n```", default_template);
        assert_eq!(Some(String::from("./snippext/")), settings.output_dir);
        assert_eq!(Some(vec![String::from("README.md")]), settings.targets);

        assert_eq!(1, settings.sources.len());
        let source = settings.sources.get(0).unwrap();
        match source {
            SnippetSource::Git {
                repository: url,
                branch: reference,
                files,
                ..
            } => {
                assert_eq!(
                    String::from("https://github.com/doctavious/snippext.git"),
                    *url
                );
                assert_eq!(Some(String::from("main")), *reference);
                // assert_eq!(Some(String::from("docs")), source.directory);
                assert_eq!(vec![String::from("**/*.rs")], *files);
            }
            _ => {
                panic!("SnippetSource should be Git")
            }
        }
    }

    #[test]
    fn support_overrides() {
        dotenv::from_path("./tests/.env.test").unwrap();

        let opt = Args {
            config: Some(PathBuf::from("./tests/custom_snippext.yaml")),
            start: None,
            end: None,
            templates: None,
            repository_url: None,
            repository_branch: None,
            repository_cone_patterns: None,
            output_dir: None,
            output_extension: Some(String::from("txt")),
            targets: Vec::default(),
            sources: Vec::default(),
            url_sources: Vec::default(),
            link_format: None,
            source_link_prefix: None,
            omit_source_links: Some(true),
            missing_snippets_behavior: None,
            retain_nested_snippet_comments: None,
            disable_language_autodetect: None,
            selected_lines_include_ellipses: None,
        };

        let settings = super::build_settings(opt).unwrap();
        // env overrides config
        assert_eq!(
            Some(String::from("./generated-snippets/")),
            settings.output_dir
        );
        // cli arg overrides env
        assert_eq!(Some("txt".into()), settings.output_extension);
        assert_eq!(true, settings.omit_source_links);
    }

    // https://users.rust-lang.org/t/whats-the-rust-way-to-unit-test-for-an-error/23677/2
    #[test]
    fn strings_must_not_be_empty() {
        let settings = SnippextSettings {
            start: String::from(""),
            end: String::from(""),
            templates: IndexMap::from([("".to_string(), "".to_string())]),
            sources: vec![SnippetSource::Local {
                files: vec![String::from("**")],
            }],
            output_dir: Some(String::from("./snippets/")),
            output_extension: Some(String::from("")),
            ..Default::default()
        };

        let validation_result = super::extract(settings);
        let error = validation_result.err().unwrap();
        match error {
            SnippextError::ValidationError(failures) => {
                assert_eq!(5, failures.len());
                assert!(failures.contains(&String::from("start must not be an empty string")));
                assert!(failures.contains(&String::from("end must not be an empty string")));
                assert!(failures.contains(&String::from(
                    "templates[0] identifier must not be an empty string"
                )));
                assert!(failures.contains(&String::from("Must have one template named 'default'")));
                assert!(failures.contains(&String::from(
                    "output_extension must not be an empty string"
                )));
            }
            _ => {
                panic!("invalid SnippextError");
            }
        }
    }

    #[test]
    fn template_content_must_not_be_empty() {
        let settings = SnippextSettings {
            start: String::from(""),
            end: String::from(""),
            templates: IndexMap::from([("default".to_string(), "".to_string())]),
            sources: vec![SnippetSource::Local {
                files: vec![String::from("**")],
            }],
            output_dir: Some(String::from("./snippets/")),
            ..Default::default()
        };

        let validation_result = super::extract(settings);
        let error = validation_result.err().unwrap();
        match error {
            SnippextError::ValidationError(failures) => {
                assert_eq!(3, failures.len());
                assert!(failures.contains(&String::from("start must not be an empty string")));
                assert!(failures.contains(&String::from("end must not be an empty string")));
                assert!(failures.contains(&String::from(
                    "templates[default] template must not be an empty string"
                )));
            }
            _ => {
                panic!("invalid SnippextError");
            }
        }
    }

    #[test]
    fn at_least_one_template_is_required() {
        let settings = SnippextSettings {
            templates: IndexMap::new(),
            sources: vec![SnippetSource::Local {
                files: vec![String::from("**")],
            }],
            output_dir: Some(String::from("./snippets/")),
            output_extension: Some(String::from("md")),
            ..Default::default()
        };

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
    fn one_template_must_be_named_default() {
        let settings = SnippextSettings {
            templates: IndexMap::from([
                ("first".to_string(), String::from("{{snippet}}")),
                ("second".to_string(), String::from("{{snippet}}")),
            ]),
            sources: vec![SnippetSource::Local {
                files: vec![String::from("**")],
            }],
            output_dir: Some(String::from("./snippets/")),
            output_extension: Some(String::from("md")),
            ..Default::default()
        };

        let validation_result = super::extract(settings);
        let error = validation_result.err().unwrap();
        match error {
            SnippextError::ValidationError(failures) => {
                assert_eq!(1, failures.len());
                assert_eq!(
                    String::from("Must have one template named 'default'"),
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
        let settings = SnippextSettings {
            templates: IndexMap::from([(
                DEFAULT_TEMPLATE_IDENTIFIER.to_string(),
                String::from("{{snippet}}"),
            )]),
            sources: vec![],
            output_dir: Some(String::from("./snippets/")),
            output_extension: Some(String::from("md")),
            ..Default::default()
        };

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
    fn local_sources_must_have_at_least_one_files_entry() {
        let settings = SnippextSettings {
            templates: IndexMap::from([(
                DEFAULT_TEMPLATE_IDENTIFIER.to_string(),
                String::from("{{snippet}}"),
            )]),
            sources: vec![SnippetSource::Local { files: vec![] }],
            output_dir: Some(String::from("./snippets/")),
            output_extension: Some(String::from("md")),
            ..Default::default()
        };

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
    fn should_return_error_when_missing_snippets_behavior_is_fail_no_snippets() {
        let settings = SnippextSettings {
            templates: IndexMap::from([(
                DEFAULT_TEMPLATE_IDENTIFIER.to_string(),
                String::from("{{snippet}}"),
            )]),
            sources: vec![SnippetSource::Local {
                files: vec!["./tests/samples/no_snippets.rs".into()],
            }],
            output_dir: Some(String::from("./snippets/")),
            output_extension: Some(String::from("md")),
            targets: Some(vec!["./tests/targets/specify_template.md".into()]),
            missing_snippets_behavior: MissingSnippetsBehavior::Fail,
            ..Default::default()
        };

        let validation_result = super::extract(settings);
        let error = validation_result.err().unwrap();

        match error {
            SnippextError::MissingSnippetsError(missing_snippets) => {
                assert_eq!(1, missing_snippets.len());

                let missing = missing_snippets.get(0).unwrap();
                assert_eq!("main", missing.key);
                assert_eq!(
                    "tests/targets/specify_template.md",
                    missing.path.to_string_lossy()
                );
                assert_eq!(2, missing.line_number);
            }
            _ => {
                panic!("invalid SnippextError");
            }
        }
    }

    #[test]
    fn should_return_error_when_missing_snippets_behavior_is_fail_multiple_snippets() {
        let dir = tempdir().unwrap();
        let target = Path::new(&dir.path()).join("target.md");
        fs::copy(Path::new("./tests/targets/target.md"), &target).unwrap();

        let settings = SnippextSettings {
            templates: IndexMap::from([(
                DEFAULT_TEMPLATE_IDENTIFIER.to_string(),
                String::from("{{snippet}}"),
            )]),
            sources: vec![SnippetSource::Local {
                files: vec!["./tests/samples/main.rs".into()],
            }],
            output_dir: Some(String::from("./snippets/")),
            output_extension: Some(String::from("md")),
            targets: Some(vec![target.to_string_lossy().to_string()]),
            missing_snippets_behavior: MissingSnippetsBehavior::Fail,
            ..Default::default()
        };

        let validation_result = super::extract(settings);
        let error = validation_result.err().unwrap();

        match error {
            SnippextError::MissingSnippetsError(missing_snippets) => {
                assert_eq!(1, missing_snippets.len());

                let missing = missing_snippets.get(0).unwrap();
                assert_eq!("fn_1", missing.key);
                assert_eq!(target.to_string_lossy(), missing.path.to_string_lossy());
                assert_eq!(6, missing.line_number);
            }
            _ => {
                panic!("invalid SnippextError");
            }
        }
    }

    #[test]
    #[traced_test]
    fn should_log_when_missing_snippets_behavior_is_warning() {
        let settings = SnippextSettings {
            templates: IndexMap::from([(
                DEFAULT_TEMPLATE_IDENTIFIER.to_string(),
                String::from("{{snippet}}"),
            )]),
            sources: vec![SnippetSource::Local {
                files: vec!["./tests/samples/no_snippets.rs".into()],
            }],
            output_dir: Some(String::from("./snippets/")),
            output_extension: Some(String::from("md")),
            targets: Some(vec!["./tests/targets/specify_template.md".into()]),
            missing_snippets_behavior: MissingSnippetsBehavior::Warn,
            ..Default::default()
        };

        let validation_result = super::extract(settings);

        assert!(validation_result.is_ok());
        assert!(logs_contain(
            "Snippet main missing in \"tests/targets/specify_template.md\" at line 2"
        ));
    }

    #[test]
    fn should_successfully_extract_from_url() {
        let dir = tempdir().unwrap();
        let target = Path::new(&dir.path()).join("./target.md");
        fs::copy(Path::new("./tests/targets/target.md"), &target).unwrap();

        let settings = SnippextSettings {
            templates: IndexMap::from([(
                DEFAULT_TEMPLATE_IDENTIFIER.to_string(),
                String::from("{{snippet}}"),
            )]),
            sources: vec![SnippetSource::Url("https://gist.githubusercontent.com/seancarroll/94629074d8cb36e9f5a0bc47b72ba6a5/raw/2b9d5db6482c7ff90a0cf3689d2a36b99e77d189/snippext_example.rs".into())],
            targets: Some(vec![target.to_string_lossy().to_string()]),
            ..Default::default()
        };

        super::extract(settings).expect("Should extract from URL");

        let actual = fs::read_to_string(target).unwrap();
        let expected = r#"This is some static content

<!-- snippet::start main -->
fn main() {
    println!("Hello, World!");
}
<!-- snippet::end -->

<!-- snippet::start fn_1 -->
some content
<!-- snippet::end -->"#;
        assert_eq!(expected, actual);
    }

    #[test]
    fn should_successfully_include_url_snippet() {
        let dir = tempdir().unwrap();
        let target = Path::new(&dir.path()).join("./url_snippet.md");
        fs::copy(Path::new("./tests/targets/url_snippet.md"), &target).unwrap();

        let settings = SnippextSettings {
            templates: IndexMap::from([(
                DEFAULT_TEMPLATE_IDENTIFIER.to_string(),
                String::from("{{snippet}}"),
            )]),
            sources: vec![SnippetSource::Local {
                files: vec!["./tests/samples/**".into()],
            }],
            targets: Some(vec![target.to_string_lossy().to_string()]),
            ..Default::default()
        };

        super::extract(settings).expect("Should extract from URL");

        let actual = fs::read_to_string(target).unwrap();
        let expected = r#"This snippet comes from a url
<!-- snippet::start https://raw.githubusercontent.com/doctavious/snippext/main/LICENSE -->
MIT License

Copyright (c) 2021 Doctavious

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
<!-- snippet::end -->"#;
        assert_eq!(expected, actual);
    }

    #[test]
    fn should_successfully_include_file_snippet() {
        let dir = tempdir().unwrap();
        let target = Path::new(&dir.path()).join("./file_snippet.md");
        fs::copy(Path::new("./tests/targets/file_snippet.md"), &target).unwrap();

        let settings = SnippextSettings {
            templates: IndexMap::from([(
                DEFAULT_TEMPLATE_IDENTIFIER.to_string(),
                String::from("{{snippet}}"),
            )]),
            sources: vec![SnippetSource::Local {
                files: vec!["./tests/samples/**".into()],
            }],
            targets: Some(vec![target.to_string_lossy().to_string()]),
            ..Default::default()
        };

        super::extract(settings).expect("Should extract from URL");

        let actual = fs::read_to_string(target).unwrap();
        let expected = r#"This snippet comes from a file
<!-- snippet::start LICENSE -->
MIT License

Copyright (c) 2021 Doctavious

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
<!-- snippet::end -->"#;
        assert_eq!(expected, actual);
    }

    #[test]
    fn local_source_link_without_prefix() {
        let source_link =
            SourceLink::new_local("src/main.rs".into(), None, Some(LinkFormat::GitHub));

        let source_link_str = source_link.append_lines(1, 10);

        assert_eq!("src/main.rs#L1-L10", source_link_str);
    }

    #[test]
    fn local_source_without_link_format_should_return_path() {
        let source_link = SourceLink::new_local("src/main.rs".into(), None, None);

        let source_link_str = source_link.append_lines(1, 10);
        assert_eq!("src/main.rs", source_link_str);
    }

    #[test]
    fn local_source_link_with_prefix() {
        let source_link = SourceLink::new_local(
            "src/main.rs".into(),
            Some("https://github.com/doctavious/snippext/blob/main/"),
            Some(LinkFormat::GitHub),
        );

        let source_link_str = source_link.append_lines(1, 10);
        assert_eq!(
            "https://github.com/doctavious/snippext/blob/main/src/main.rs#L1-L10",
            source_link_str
        );
    }
}
