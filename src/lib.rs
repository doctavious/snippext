#![doc(html_root_url = "https://docs.rs/snippext")]
#![doc(issue_tracker_base_url = "https://github.com/doctavious/snippext/issues/")]
#![warn(missing_docs)]
#![deny(rustdoc::broken_intra_doc_links)]

//! TODO: add docs for snippext lib
//! [short sentence explaining what it is]
//! [more detailed explanation]
//! [at least one code example that users can copy/paste to try it]
//! [even more advanced explanations if necessary]

pub mod error;
pub mod git;
pub mod unindent;
pub mod sanitize;
pub mod cmd;

use std::collections::{HashMap, HashSet};
use std::default::Default;
use std::fs::{File};
use std::{fmt, fs};
use std::io::prelude::*;
use std::io::BufReader;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::time::SystemTime;

use chrono::{DateTime, Utc};
use clap::{Parser, ValueEnum};
use clap::builder::PossibleValue;
use filetime::{set_file_mtime, FileTime};
use glob::glob;
use handlebars::{no_escape, Handlebars};
use regex::Regex;
use reqwest::blocking::Client;
use reqwest::header::{EXPIRES, HeaderValue, LAST_MODIFIED};
use serde::{Deserialize, Serialize};
use url::Url;

use crate::error::SnippextError;
use crate::unindent::unindent;
use crate::sanitize::sanitize;


pub type SnippextResult<T> = Result<T, SnippextError>;

// TODO: this might not be needed
pub const DEFAULT_SNIPPEXT_CONFIG: &str = include_str!("./default_snippext_config.yaml");
pub const DEFAULT_CONFIG: &'static str = "snippext";
pub const DEFAULT_COMMENT_PREFIXES: &'static [&'static str] = &["// ", "# ", "<!-- "];
pub const DEFAULT_BEGIN: &'static str = "snippet::";
pub const DEFAULT_END: &'static str = "end::";
pub const DEFAULT_INCLUDE: &'static str = "snippet::include::";
// snippet::start::
// snippet::end::
const DEFAULT_REPLACE: &'static str = "snippet::replace::"; // TODO: do we want this?
pub const DEFAULT_TEMPLATE: &'static str = "{{snippet}}";
pub const DEFAULT_FILE_EXTENSION: &'static str = "md";
pub const DEFAULT_SOURCE_FILES: &'static str = "**";
pub const DEFAULT_OUTPUT_DIR: &'static str = "./snippets/";
const SNIPPEXT_TEMPLATE_ATTRIBUTE: &'static str = "snippext_template";
pub const DEFAULT_TEMPLATE_IDENTIFIER: &'static str = "default";

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Snippet {
    // snippet_start_tag
    // snippet_end_tag

    // The snippet name is sanitized to prevent malicious code to overwrite arbitrary files on your system.
    pub identifier: String,
    /// The path the snippet was read from.
    pub path: PathBuf,
    pub text: String,
    pub attributes: HashMap<String, String>,
    pub start_line: usize,
    pub end_line: usize,

}

impl Snippet {
    pub fn new(
        identifier: String,
        path: PathBuf,
        text: String,
        attributes: HashMap<String, String>,
        start_line: usize,
        end_line: usize,
    ) -> Self {
        Self {
            identifier,
            path,
            text,
            attributes,
            start_line,
            end_line,
        }
    }

    pub fn create_tag(&self, prefix: String, tag: String) -> String {
        prefix + tag.as_str() + self.identifier.as_str()
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SnippextTemplate {
    pub content: String,
    pub default: bool,
}

impl SnippextTemplate {
    pub fn render_template(
        snippet: &Snippet,
        snippext_settings: &SnippextSettings,
        target_attributes: Option<HashMap<String, String>>,
    ) -> SnippextResult<String> {
        let mut data = HashMap::new();
        if target_attributes.is_some() {
            data.extend(target_attributes.unwrap());
        }

        // TODO: do we want to make unindent optional?
        data.insert("snippet".to_string(), unindent(snippet.text.as_str()));
        // https://github.com/temporalio/snipsync/blob/fef6170acacc6dd351c4ab5784cccaafa80d93d5/src/Sync.js#L68
        // https://github.com/SimonCropp/MarkdownSnippets/blob/fae28ec759089641d3bf89a90211776de97d8899/src/MarkdownSnippets/Processing/SnippetMarkdownHandling.cs#L62
        // <a href='{url_prefix}{source_link}' title='Snippet source file'>snippet source</a>
        if let Some(link_format) = &snippext_settings.link_format {
            data.insert("source_links_enabled".to_string(), "true".to_string());

            let source_link = SnippextTemplate::build_source_link(
                &snippet,
                link_format,
                snippext_settings.url_prefix.to_owned().unwrap_or_default(),
            );

            data.insert(
                "source_link".to_string(),
                format!(
                    "<a href='{}' title='Snippet source file'>source</a>",
                    source_link
                ),
            );
        }

        for attribute in &snippet.attributes {
            data.insert(attribute.0.to_string(), attribute.1.to_string());
        }

        let template = get_template(data.get(SNIPPEXT_TEMPLATE_ATTRIBUTE), snippext_settings)?;
        return template.render(&data);
    }

    pub fn render(&self, data: &HashMap<String, String>) -> SnippextResult<String> {
        let mut hbs = Handlebars::new();
        hbs.register_escape_fn(no_escape);

        let rendered = hbs.render_template(self.content.as_str(), data)?;

        Ok(rendered)
    }

    fn build_source_link(snippet: &Snippet, link_format: &LinkFormat, url_prefix: String) -> String {
        let mut path = url_prefix;
        if !path.ends_with("/") {
            path.push_str("/")
        }

        path.push_str(snippet.path.to_str().unwrap_or_default());

        match link_format {
            LinkFormat::GitHub => format!("{}#L{}-L{}", path, snippet.start_line, snippet.end_line),
            LinkFormat::GitLab => format!("{}#L{}-{}", path, snippet.start_line, snippet.end_line),
            LinkFormat::BitBucket => {
                format!("{}#lines={}:{}", path, snippet.start_line, snippet.end_line)
            }
            LinkFormat::Gitea => format!("{}#L{}-L{}", path, snippet.start_line, snippet.end_line),
            LinkFormat::TFS => format!(
                "{}&line={}&lineEnd={}",
                path, snippet.start_line, snippet.end_line
            ),
        }
    }
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct SnippextSettings {
    pub begin: String,
    pub end: String,
    pub extension: String,
    pub comment_prefixes: HashSet<String>,
    pub templates: HashMap<String, SnippextTemplate>,
    pub sources: Vec<SnippetSource>,
    pub output_dir: Option<String>,
    pub targets: Option<Vec<String>>,
    pub link_format: Option<LinkFormat>,
    pub url_prefix: Option<String>,
}

impl SnippextSettings {
    /// Create default SnippextSettings which will have the following
    /// begin: [`DEFAULT_BEGIN`]
    /// end: [`DEFAULT_END`]
    /// extension: [`DEFAULT_FILE_EXTENSION`]
    /// comment_prefixes: [`DEFAULT_COMMENT_PREFIXES`]
    /// template: [`DEFAULT_TEMPLATE`]
    /// sources: all files via [`DEFAULT_SOURCE_FILES`] glob
    /// output_dir: [`DEFAULT_OUTPUT_DIR`]
    pub fn default() -> Self {
        Self {
            begin: String::from(DEFAULT_BEGIN),
            end: String::from(DEFAULT_END),
            extension: String::from(DEFAULT_FILE_EXTENSION),
            comment_prefixes: DEFAULT_COMMENT_PREFIXES
                .into_iter()
                .map(|s| s.to_string())
                .collect(),
            templates: HashMap::from([(
                String::from("default"),
                SnippextTemplate {
                    content: String::from(DEFAULT_TEMPLATE),
                    default: true,
                },
            )]),
            sources: vec![SnippetSource::new_local(vec![String::from(
                DEFAULT_SOURCE_FILES,
            )])],
            output_dir: Some(String::from(DEFAULT_OUTPUT_DIR)),
            targets: None,
            link_format: None,
            url_prefix: None,
        }
    }

    /// Create SnippextSettings from config file
    ///
    /// # Arguments
    ///
    /// * `path` - Path of config file
    pub fn from_config<S: AsRef<Path>>(path: S) -> SnippextResult<Self> {
        let content = fs::read_to_string(path)?;
        let settings = serde_json::from_str(content.as_str())?;
        Ok(settings)
    }

    // TODO: <S: Into<String>>
    pub fn new(
        comment_prefixes: HashSet<String>,
        begin: String,
        end: String,
        extension: String,
        templates: HashMap<String, SnippextTemplate>,
        sources: Vec<SnippetSource>,
        output_dir: Option<String>,
        targets: Option<Vec<String>>,
        link_format: Option<LinkFormat>,
        url_prefix: Option<String>,
    ) -> Self {
        Self {
            begin,
            end,
            extension,
            comment_prefixes,
            templates,
            sources,
            output_dir,
            targets,
            link_format,
            url_prefix,
        }
    }
}


// TODO: might be better as an enum
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct SnippetSource {
    pub repository: Option<String>,
    pub branch: Option<String>,
    pub commit: Option<String>,
    pub cone_patterns: Option<Vec<String>>, // for sparse checkout. cone pattern sets
    pub directory: Option<String>,
    pub files: Vec<String>,
    pub url: Option<String>,
}

impl SnippetSource {
    pub fn new_local(files: Vec<String>) -> Self {
        Self {
            repository: None,
            branch: None,
            commit: None,
            cone_patterns: None,
            directory: None,
            files,
            url: None,
        }
    }

    pub fn new_git(
        repository: String,
        branch: String,
        commit: Option<String>,
        directory: Option<String>,
        files: Vec<String>,
    ) -> Self {
        Self {
            repository: Some(repository),
            branch: Some(branch),
            commit,
            cone_patterns: None,
            directory,
            files,
            url: None,
        }
    }

    pub fn new_url(url: String) -> Self {
        Self {
            repository: None,
            branch: None,
            commit: None,
            cone_patterns: None,
            directory: None,
            files: Vec::default(),
            url: Some(url),
        }
    }

    pub fn is_remote(&self) -> bool {
        self.repository.is_some()
    }

    pub fn is_url(&self) -> bool {
        self.url.is_some()
    }
}

#[derive(Clone, Debug, Deserialize, Parser, Serialize)]
pub enum LinkFormat {
    GitHub,
    GitLab,
    Gitea,
    BitBucket,
    TFS,
}

impl ValueEnum for LinkFormat {
    fn value_variants<'a>() -> &'a [Self] {
        &[
            LinkFormat::GitHub,
            LinkFormat::GitLab,
            LinkFormat::Gitea,
            LinkFormat::BitBucket,
            LinkFormat::TFS,
        ]
    }

    fn to_possible_value(&self) -> Option<PossibleValue> {
        Some(match self {
            LinkFormat::GitHub => PossibleValue::new("github"),
            LinkFormat::GitLab => PossibleValue::new("gitlab"),
            LinkFormat::Gitea => PossibleValue::new("gitea"),
            LinkFormat::BitBucket => PossibleValue::new("bitbucket"),
            LinkFormat::TFS => PossibleValue::new("tfs"),
        })
    }
}

impl fmt::Display for LinkFormat {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            LinkFormat::GitHub => write!(f, "github"),
            LinkFormat::GitLab => write!(f, "gitlab"),
            LinkFormat::Gitea => write!(f, "gitea"),
            LinkFormat::BitBucket => write!(f, "bitbucket"),
            LinkFormat::TFS => write!(f, "tfs"),
        }
    }
}

/// find appropriate Snippext Template using the following rules
///
/// 1. template by id. None if not found
/// If id not provided
/// if only one template provided use it
/// if more than one template find the default one
fn get_template<'a>(
    id: Option<&String>,
    snippext_settings: &'a SnippextSettings,
) -> SnippextResult<&'a SnippextTemplate> {
    return if let Some(identifier) = id {
        if let Some(template) = snippext_settings.templates.get(identifier) {
            Ok(template)
        } else {
            Err(SnippextError::TemplateNotFound(String::from(format!(
                "{} does not exist",
                identifier
            ))))
        }
    } else {
        // could probably turn this into a match expression with match guards
        if snippext_settings.templates.len() == 1 {
            return Ok(snippext_settings.templates.values().next().unwrap());
        }

        if snippext_settings.templates.len() > 1 {
            let default_template = snippext_settings.templates.iter().find(|t| t.1.default);
            return if let Some(template) = default_template {
                Ok(template.1)
            } else {
                // we validate that we should always have one default template
                // so should never get here. Should we assert instead?
                Err(SnippextError::TemplateNotFound(String::from(
                    "No default template found",
                )))
            };
        }

        // we validate that we have at least one template so should never get here.
        // should we assert instead?
        Err(SnippextError::TemplateNotFound(String::from(
            "No templates found",
        )))
    };
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

pub struct SnippetExtractionState {
    pub key: String,
    pub start_line: usize,
    pub lines: String,
    pub attributes: HashMap<String, String>,
}

impl SnippetExtractionState {
    pub fn append_line(&mut self, line: &str) {
        self.lines.push_str(line)
    }
}

pub fn extract(snippext_settings: SnippextSettings) -> SnippextResult<()> {
    validate_snippext_settings(&snippext_settings)?;

    let source_files = get_filenames(&snippext_settings)?;
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

        // TODO: print to stdout if output_dir and targets are both none?
        // if &snippext_settings.output_dir.is_none() && &snippext_settings.targets.is_none() {
        //
        // }

        // TODO: output_dir optional
        // TODO: targets
        // TODO: stdout if neither is provided
        if let Some(output_dir) = &snippext_settings.output_dir {
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

pub fn extract_snippets(
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
                ("filename".to_string(), path.file_name().unwrap().to_string_lossy().to_string())
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

struct SourceFile {
    pub full_path: PathBuf,
    pub relative_path: PathBuf,
}



// TODO: we should instead be collecting snippets
// TODO: This code is absolute shit. Clean this up
// Need both the absolute path and the relative path so that for when we output generated files
// we only include relative directories within the output directory.
fn get_filenames(settings: &SnippextSettings) -> SnippextResult<Vec<SourceFile>> {
    let mut source_files: Vec<SourceFile> = Vec::new();

    for source in &settings.sources {
        if source.is_remote() {
            git::checkout_files(
                source.repository.clone().unwrap(),
                source.branch.clone(),
                source.cone_patterns.clone(),
                source.directory.clone(),
            )?;
        }

        if let Some(url) = &source.url {
            let path = download_url(url)?;
            // source_files.append(path.to_string_lossy().to_string());
            // TODO: what to do with path
        }

        for file in &source.files {
            let (g, d) = if let Some(dir) = source.directory.clone() {
                // TODO: encapsulate this somewhere
                let x: &[_] = &['.', '/'];
                (
                    format!(
                        "{}/{}",
                        dir.trim_end_matches('/'),
                        file.clone().trim_start_matches(x)
                    ),
                    dir,
                )
            } else {
                (file.clone(), file.clone())
            };

            let paths = match glob(g.as_str()) {
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
                let relative_path = if let Ok(prefix) = path.clone().strip_prefix(d.clone()) {
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

    Ok(source_files)
}

fn download_url(url: &String) -> SnippextResult<PathBuf> {
    let url_file_path = Url::from_str(url.as_str())?
        .to_file_path()
        .map_err(|_| SnippextError::GeneralError(format!("failed to convert url {} to file path", url)))?;

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
        return Err(SnippextError::GeneralError(format!("Failed to download details from {}", url)));
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

    // TODO: should we output to stdout instead?
    if settings.output_dir.is_none() && settings.targets.is_none() {
        failures.push(String::from("output_dir or targets is required"));
    }

    return if !failures.is_empty() {
        Err(SnippextError::ValidationError(failures))
    } else {
        Ok(())
    };
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct InitSettings {
    pub default: bool,
}

/// Configure Snippext settings
pub fn init(settings: Option<SnippextSettings>) -> SnippextResult<()> {
    let content = if let Some(settings) = settings {
        serde_yaml::to_string(&settings)?
    } else {
        DEFAULT_SNIPPEXT_CONFIG.to_string()
    };

    fs::write("./snippext.yaml", content)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::error::SnippextError;
    use crate::{SnippetSource, SnippextSettings, SnippextTemplate};
    use std::collections::{HashMap, HashSet};
    use std::fs;
    use std::io::Write;
    use tempfile::{tempdir, NamedTempFile};
    use tracing::info;

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
            String::from("snippet::"),
            String::from("end::"),
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
            String::from("snippet::"),
            String::from("end::"),
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
            String::from("snippet::"),
            String::from("end::"),
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
            String::from("snippet::"),
            String::from("end::"),
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
            String::from("snippet::"),
            String::from("end::"),
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
            String::from("snippet::"),
            String::from("end::"),
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
            String::from("snippet::"),
            String::from("end::"),
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
                directory: None,
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
