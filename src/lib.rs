#![doc(html_root_url = "https://docs.rs/snippext")]
#![doc(issue_tracker_base_url = "https://github.com/doctavious/snippext/issues/")]
#![warn(missing_docs)]
#![deny(rustdoc::broken_intra_doc_links)]


//! TODO: add docs for snippext lib
//! [short sentence explaining what it is]
//! [more detailed explanation]
//! [at least one code example that users can copy/paste to try it]
//! [even more advanced explanations if necessary]

mod sanitize;
mod unindent;
pub mod error;
mod git;

use glob::{glob, Paths, PatternError};
use git2::{build::CheckoutBuilder, Cred, Error as GitError, RemoteCallbacks, Repository};
use lazy_static::lazy_static;
use regex::Regex;
use sanitize::sanitize;
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::fs::File;
use std::{fs, env};
use std::io::prelude::*;
use std::io::BufReader;
use std::path::{Path, PathBuf};
use std::collections::{HashMap, BTreeMap};
use unindent::unindent;
use handlebars::{Handlebars, no_escape};
use config::{Source, Value, ConfigError};
use crate::error::SnippextError;

pub type SnippextResult<T> = core::result::Result<T, SnippextError>;

// TODO: this might not be needed
const DEFAULT_CONFIG: &'static str = "snippext";
const DEFAULT_COMMENT_PREFIXES: &'static [&'static str] = &["// ", "# ", "<!-- "];
const DEFAULT_BEGIN: &'static str = "snippet::";
const DEFAULT_END: &'static str = "end::";
const DEFAULT_TEMPLATE: &'static str = "{{snippet}}";
const DEFAULT_FILE_EXTENSION: &'static str = "md";
const DEFAULT_SOURCE_FILES: &'static str = "**";
const DEFAULT_OUTPUT_DIR: &'static str = "./snippets/";


#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Snippet {
    // snippet_start_tag
    // snippet_end_tag

    // The snippet name is sanitized to prevent malicious code to overwrite arbitrary files on your system.
    pub identifier: String,
    pub text: String,
    pub closed: bool,
    pub attributes: HashMap<String, String>,
}

impl Snippet {
    pub fn new(identifier: String, attributes: HashMap<String, String>) -> Self {
        Self {
            identifier,
            text: "".to_string(),
            closed: false,
            attributes
        }
    }
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct SnippextSettings {
    pub begin: String,
    pub end: String,
    pub extension: String,
    pub comment_prefixes: Vec<String>,
    pub template: String,
    pub sources: Vec<SnippetSource>,
    pub output_dir: Option<String>,
    pub targets: Option<Vec<String>>,
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
            comment_prefixes: DEFAULT_COMMENT_PREFIXES.into_iter().map(|s| s.to_string()).collect(),
            template: String::from(DEFAULT_TEMPLATE),
            sources: vec![SnippetSource::new_local(vec![String::from(DEFAULT_SOURCE_FILES)])],
            output_dir: Some(String::from(DEFAULT_OUTPUT_DIR)),
            targets: None,
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
    pub fn new (
        comment_prefixes: Vec<String>,
        begin: String,
        end: String,
        extension: String,
        template: String,
        sources: Vec<SnippetSource>,
        output_dir: Option<String>,
        targets: Option<Vec<String>>,
    ) -> Self {
        Self {
            begin,
            end,
            extension,
            comment_prefixes,
            template,
            sources,
            output_dir,
            targets,
        }
    }
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct SnippetSource {
    pub repository: Option<String>,
    pub branch: Option<String>,
    pub commit: Option<String>,
    pub cone_patterns: Option<Vec<String>>, // for sparse checkout. cone pattern sets
    pub directory: Option<String>,
    pub files: Vec<String>,
}

impl SnippetSource {
    pub fn new_local(files: Vec<String>) -> Self {
        Self {
            repository: None,
            branch: None,
            commit: None,
            cone_patterns: None,
            directory: None,
            files
        }
    }

    pub fn new_remote(
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
            files
        }
    }

    pub fn is_remote(&self) -> bool {
        self.repository.is_some()
    }
}

pub fn run(snippext_settings: SnippextSettings) -> SnippextResult<()>
{
    validate_settings(&snippext_settings)?;

    let source_files = get_filenames(snippext_settings.sources)?;

    // TODO: move this?
    let mut hbs = Handlebars::new();
    hbs.register_escape_fn(no_escape);

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

            for snippet in snippets {
                let x: &[_] = &['.', '/'];
                let output_path = Path::new(output_dir.as_str())
                    .join(source_file.relative_path.to_string_lossy().trim_start_matches(x))
                    .join(sanitize(snippet.identifier))
                    .with_extension(snippext_settings.extension.as_str());

                fs::create_dir_all(output_path.parent().unwrap()).unwrap();

                let mut data = BTreeMap::new();
                data.insert("snippet".to_string(), unindent(snippet.text.as_str()));
                for attribute in snippet.attributes {
                    data.insert(attribute.0, attribute.1);
                }
                let result = hbs.render_template(snippext_settings.template.as_str(), &data).unwrap();
                fs::write(output_path, result).unwrap();
            }
        }

        if let Some(targets) = &snippext_settings.targets {

        }
    }

    Ok(())
}

pub fn update_target_file(
    source: PathBuf,
    snippet_start: &str,
    snippet_end: &str,
    content: &str
) -> SnippextResult<()> {
    let mut source_content = fs::read_to_string(source.to_path_buf())?;
    update_target_string(&mut source_content, snippet_start, snippet_end, content)?;
    fs::write(source.to_path_buf(), source_content)?;
    Ok(())
}

pub fn update_target_string(
    source: &mut String,
    snippet_start: &str,
    snippet_end: &str,
    content: &str
) -> SnippextResult<()> {
    let snippet_start_index = source.find(snippet_start).ok_or(SnippextError::SnippetNotFound())?;
    let content_starting_index = snippet_start_index + snippet_start.len();
    let end_index = source.find(snippet_end).unwrap_or(source.len());
    source.replace_range(content_starting_index..end_index, content);
    Ok(())
}

pub fn extract_snippets(
    comment_prefixes: &Vec<String>,
    begin_pattern: String,
    end_pattern: String,
    filename: &Path,
) -> SnippextResult<Vec<Snippet>> {
    let f = File::open(filename)?;
    let reader = BufReader::new(f);

    let mut snippets: Vec<Snippet> = Vec::new();
    for line in reader.lines() {
        let l = line?;

        let begin_ident = matches(&l, &comment_prefixes, &begin_pattern);
        if let Some(begin_ident) = begin_ident {
            // TODO: I feel like this is the long hard way to do this...
            let mut attributes = HashMap::new();
            let last_square_bracket_pos = begin_ident.rfind('[');
            if let Some(last_square_bracket_pos) = last_square_bracket_pos {
                let identifier = &begin_ident.as_str()[..last_square_bracket_pos];
                let re = Regex::new("\\[([^]]+)]").unwrap();
                let captured_kv = re.captures(begin_ident.as_str());
                if captured_kv.is_some() {
                    for kv in captured_kv.unwrap().get(1).unwrap().as_str().split(",") {
                        let parts: Vec<&str> = kv.split("=").collect();
                        if parts.len() == 2 {
                            attributes.insert(
                                parts.get(0).unwrap().to_string(),
                                parts.get(1).unwrap().to_string()
                            );
                        }
                    }
                }

                let snippet = Snippet::new(identifier.to_string(), attributes);
                snippets.push(snippet);
            } else {
                let snippet = Snippet::new(begin_ident, attributes);
                snippets.push(snippet);
            }

            continue;
        }

        let end_ident = matches(&l, &comment_prefixes, &end_pattern);
        if let Some(end_ident) = end_ident {
            for snippet in snippets.iter_mut() {
                if snippet.identifier == end_ident {
                    snippet.closed = true
                }
            }
            continue;
        }
        for snippet in snippets.iter_mut() {
            if snippet.closed {
                continue;
            }
            snippet.text = String::from(snippet.text.as_str()) + l.as_str() + "\n"
        }
    }

    Ok(snippets)
}

struct SourceFile {
    pub full_path: PathBuf,
    pub relative_path: PathBuf,
}

// TODO: This code is absolute shit. Clean this up
// Need both the absolute path and the relative path so that for when we output generated files
// we only include relative directories within the output directory.
fn get_filenames(sources: Vec<SnippetSource>) -> SnippextResult<Vec<SourceFile>> {
    let mut out: Vec<PathBuf> = Vec::new();
    let mut source_files: Vec<SourceFile> = Vec::new();

    for source in sources {

        // let dir = if let Some(dir) = source.directory.clone() {
        //     dir
        // } else {
        //     String::from("./")
        // };

        if source.is_remote() {
            git::checkout_files(
                source.repository.unwrap(),
                source.branch,
                source.cone_patterns,
                source.directory.clone()
            );
        }

        for file in source.files {
            let (g, d) = if let Some(dir) = source.directory.clone() {
                let x: &[_] = &['.', '/'];
                (
                    format!("{}/{}", dir.trim_end_matches('/'), file.clone().trim_start_matches(x)),
                    dir
                )
            } else {
                (file.clone(), file.clone())
            };

            let paths = match glob(g.as_str()) {
                Ok(paths) => paths,
                Err(error) => {
                    return Err(SnippextError::GlobPatternError(format!("Glob pattern error for `{}`. {}", file, error.msg)))
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
                    // out.push(path);
                    source_files.push(SourceFile {
                        full_path: path.clone(),
                        relative_path
                    });
                }
            }
        }
    }

    Ok(source_files)
}

// TODO: return tuple (prefix and identifier) or struct?
// Might not be necessary depending on how we want to enable doctavious
fn matches(s: &str, comment_prefixes: &[String], pattern: &str) -> Option<String> {
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
fn validate_settings(settings: &SnippextSettings) -> SnippextResult<()> {
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

    if settings.template.is_empty() {
        failures.push(String::from("template must not be an empty string"));
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
                && (source.cone_patterns.is_some() || source.branch.is_some() || source.commit.is_some()) {
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
    }
}

#[cfg(test)]
mod tests {
    use crate::error::SnippextError;
    use crate::{SnippextSettings, SnippetSource};

    // https://users.rust-lang.org/t/whats-the-rust-way-to-unit-test-for-an-error/23677/2
    #[test]
    fn strings_must_not_be_empty() {
        let settings = SnippextSettings::new(
            vec![String::from("# ")],
            String::from(""),
            String::from(""),
            String::from(""),
            String::from(""),
            vec![SnippetSource::new_local(vec![String::from("**")])],
            Some(String::from("./snippets/")),
            None,
        );

        let validation_result = super::run(settings);
        let error = validation_result.err().unwrap();
        match error {
            SnippextError::ValidationError(failures) => {
                assert_eq!(4, failures.len());
                assert!(failures.contains(&String::from("begin must not be an empty string")));
                assert!(failures.contains(&String::from("end must not be an empty string")));
                assert!(failures.contains(&String::from("template must not be an empty string")));
                assert!(failures.contains(&String::from("extension must not be an empty string")));
            }
            _ => {
                panic!("invalid SnippextError");
            }
        }
    }


    #[test]
    fn at_least_one_comment_prefix_is_required() {
        let settings = SnippextSettings::new(
            vec![],
            String::from("snippet::"),
            String::from("end::"),
            String::from("md"),
            String::from("{{snippet}}"),
            vec![SnippetSource::new_local(vec![String::from("**")])],
            Some(String::from("./snippets/")),
            None,
        );

        let validation_result = super::run(settings);
        let error = validation_result.err().unwrap();
        match error {
            SnippextError::ValidationError(failures) => {
                assert_eq!(1, failures.len());
                assert_eq!(
                    String::from("comment_prefixes must not be empty"),
                    failures.get(0).unwrap().to_string()
                )
            }
            _ => {
                panic!("invalid snippextError");
            }
        }
    }

    #[test]
    fn sources_must_not_be_empty() {
        let settings = SnippextSettings::new(
            vec![String::from("# ")],
            String::from("snippet::"),
            String::from("end::"),
            String::from("md"),
            String::from("{{snippet}}"),
            vec![],
            Some(String::from("./snippets/")),
            None,
        );

        let validation_result = super::run(settings);
        let error = validation_result.err().unwrap();
        match error {
            SnippextError::ValidationError(failures) => {
                assert_eq!(1, failures.len());
                assert_eq!(
                    String::from("sources must not be empty"),
                    failures.get(0).unwrap().to_string()
                )
            }
            _ => {
                panic!("invalid snippextError");
            }
        }
    }

    #[test]
    fn sources_must_have_at_least_one_files_entry() {
        let settings = SnippextSettings::new(
            vec![String::from("# ")],
            String::from("snippet::"),
            String::from("end::"),
            String::from("md"),
            String::from("{{snippet}}"),
            vec![SnippetSource::new_local(vec![])],
            Some(String::from("./snippets/")),
            None,
        );

        let validation_result = super::run(settings);
        let error = validation_result.err().unwrap();

        match error {
            SnippextError::ValidationError(failures) => {
                assert_eq!(1, failures.len());
                assert_eq!(
                    String::from("sources[0].files must not be empty"),
                    failures.get(0).unwrap().to_string()
                )
            }
            _ => {
                panic!("invalid SnippextError");
            }
        }
    }

    #[test]
    fn repository_must_be_provided_if_other_remote_sources_are_provided() {
        let settings = SnippextSettings::new(
            vec![String::from("# ")],
            String::from("snippet::"),
            String::from("end::"),
            String::from("md"),
            String::from("{{snippet}}"),
            vec![SnippetSource {
                repository: None,
                branch: Some(String::from("branch")),
                commit: Some(String::from("commit")),
                cone_patterns: None,
                directory: None,
                files: vec![String::from("**")]
            }],
            Some(String::from("./snippets/")),
            None,
        );

        let validation_result = super::run(settings);
        let error = validation_result.err().unwrap();
        println!("{:?}", error);
        match error {
            SnippextError::ValidationError(failures) => {
                assert_eq!(1, failures.len());
                assert_eq!(
                    String::from("sources[0] specifies branch, commit, cone_patterns without specifying repository"),
                    failures.get(0).unwrap().to_string()
                )
            }
            _ => {
                panic!("invalid SnippextError");
            }
        }
    }

    #[test]
    fn update_target() {
        let mut source = r#"Some content
snippet::foo
foo
end::foo
"#.to_string();

        super::update_target_string(
            &mut source,
            "snippet::foo",
            "end::foo",
            "\nbar\n",
        );

        println!("{}", source);
    }
}
