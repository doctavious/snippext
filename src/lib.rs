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
mod git;
mod sanitize;
mod unindent;

use glob::glob;

use regex::Regex;
use sanitize::sanitize;
use serde::{Deserialize, Serialize};

use crate::error::SnippextError;
use config::Source;
use handlebars::{no_escape, Handlebars};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::fs;
use std::fs::File;
use std::io::prelude::*;
use std::io::BufReader;
use std::path::{Path, PathBuf};
use unindent::unindent;

pub type SnippextResult<T> = core::result::Result<T, SnippextError>;

// TODO: this might not be needed
const DEFAULT_SNIPPEXT_CONFIG: &str = include_str!("./default_snippext_config.yaml");
const DEFAULT_CONFIG: &'static str = "snippext";
const DEFAULT_COMMENT_PREFIXES: &'static [&'static str] = &["// ", "# ", "<!-- "];
const DEFAULT_BEGIN: &'static str = "snippet::";
const DEFAULT_END: &'static str = "end::";
const DEFAULT_INCLUDE: &'static str = "snippet::include::";
// snippet::start::
// snippet::end::
const DEFAULT_REPLACE: &'static str = "snippet::replace::"; // TODO: do we want this?
const DEFAULT_TEMPLATE: &'static str = "{{snippet}}";
const DEFAULT_FILE_EXTENSION: &'static str = "md";
pub const DEFAULT_SOURCE_FILES: &'static str = "**";
const DEFAULT_OUTPUT_DIR: &'static str = "./snippets/";
const SNIPPEXT_TEMPLATE_ATTRIBUTE: &'static str = "snippext_template";
pub const DEFAULT_TEMPLATE_IDENTIFIER: &'static str = "default";

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
            attributes,
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
        target_attributes: Option<BTreeMap<String, String>>,
    ) -> SnippextResult<String> {
        let mut data = BTreeMap::new();
        if target_attributes.is_some() {
            data.append(&mut target_attributes.unwrap());
        }

        data.insert("snippet".to_string(), unindent(snippet.text.as_str()));
        for attribute in &snippet.attributes {
            data.insert(attribute.0.to_string(), attribute.1.to_string());
        }

        let template = get_template_by_id(data.get(SNIPPEXT_TEMPLATE_ATTRIBUTE), &snippext_settings).unwrap();
        return template.render(&data);
    }

    pub fn render(&self, data: &BTreeMap<String, String>) -> SnippextResult<String> {
        let mut hbs = Handlebars::new();
        hbs.register_escape_fn(no_escape);

        let rendered = hbs.render_template(self.content.as_str(), data)?;

        Ok(rendered)
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
            templates: HashMap::from([
                (String::from("default"), SnippextTemplate {
                    content: String::from(DEFAULT_TEMPLATE),
                    default: true,
                }),
            ]),
            sources: vec![SnippetSource::new_local(vec![String::from(
                DEFAULT_SOURCE_FILES,
            )])],
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
    pub fn new(
        comment_prefixes: HashSet<String>,
        begin: String,
        end: String,
        extension: String,
        templates: HashMap<String, SnippextTemplate>,
        sources: Vec<SnippetSource>,
        output_dir: Option<String>,
        targets: Option<Vec<String>>,
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
            files,
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
            files,
        }
    }

    pub fn is_remote(&self) -> bool {
        self.repository.is_some()
    }
}

pub fn run(snippext_settings: SnippextSettings) -> SnippextResult<()> {
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
                    .with_extension(snippext_settings.extension.as_str());

                fs::create_dir_all(output_path.parent().unwrap()).unwrap();
                let result = SnippextTemplate::render_template(snippet, &snippext_settings, None)?;
                fs::write(output_path, result).unwrap();
            }
        }

        if let Some(targets) = &snippext_settings.targets {
            for target in targets {
                for snippet in &snippets {
                    // update_target_file(
                    //     Path::new(target).to_path_buf(),
                    //     &snippext_settings.comment_prefixes,
                    //     // snippet.snippet_start_tag,
                    //     // snippet.snippet_end_tag,
                    //     snippext_settings.begin.to_owned() + snippet.identifier.as_str(),
                    //     snippext_settings.end.to_owned() + snippet.identifier.as_str(),
                    //     snippet.text.as_str(),
                    // );
                    update_target_file_snippet(
                        Path::new(target).to_path_buf(),
                        &snippet,
                        &snippext_settings
                    );
                }
            }
        }
    }

    Ok(())
}

/// find appropriate Snippext Template using the following rules
///
/// 1. template by id. None if not found
/// If id not provided
/// if only one template provided use it
/// if more than one template find the default one
fn get_template_by_id<'a>(id: Option<&String>, snippext_settings: &'a SnippextSettings) -> Option<&'a SnippextTemplate> {
    return if let Some(identifier) = id {
        if let Some(template) = snippext_settings.templates.get(identifier) {
            Some(template)
        } else {
            None
        }
    } else {
        // could probably turn this into a match expression with match guards
        if snippext_settings.templates.len() == 1 {
            return Some(snippext_settings.templates.values().next().unwrap());
        }

        if snippext_settings.templates.len() > 1 {
            let t = snippext_settings.templates
                .iter()
                .find(|t| t.1.default);
            return if let Some(template) = t {
                Some(template.1)
            } else {
                // we validate that we should always have one default template
                // so should never get here. Should we assert instead?
                None
            }
        }

        // we validate that we have at least one template so should never get here.
        // should we assert instead?
        None
    }
}


// TODO: This should probably read lines instead of entire file content
//       currently we cant have the same snippet multiple times in the same file
// TODO: should look for same comment prefixes?
pub fn update_target_file(
    source: PathBuf,
    snippet_prefixes: &Vec<String>,
    snippet_start: String,
    snippet_end: String,
    content: &str,
) -> SnippextResult<()> {
    let mut source_content = fs::read_to_string(source.to_path_buf())?;
    update_target_string(
        &mut source_content,
        snippet_prefixes,
        snippet_start,
        snippet_end,
        content,
    )?;
    fs::write(source.to_path_buf(), source_content)?;
    Ok(())
}

pub fn update_target_file_snippet(
    source: PathBuf,
    snippet: &Snippet,
    snippet_settings: &SnippextSettings,
) -> SnippextResult<()> {
    let mut source_content = fs::read_to_string(source.to_path_buf())?;
    update_target_string_snippet(
        &mut source_content,
        snippet,
        snippet_settings,
    )?;
    fs::write(source.to_path_buf(), source_content)?;
    Ok(())
}

// TODO: clean up
// TODO: add appropriate error handling like not finding end of a snippet
pub fn update_target_string(
    source: &mut String,
    snippet_prefixes: &Vec<String>,
    snippet_start: String,
    snippet_end: String,
    content: &str,
) -> SnippextResult<()> {
    for prefix in snippet_prefixes {
        if let Some(snippet_start_index) =
            source.find(String::from(prefix.as_str().to_owned() + snippet_start.as_str()).as_str())
        {
            // TODO: extract attribute from snippet
            // TODO: should find/use template
            if let Some(snippet_start_tag_end_index) = source[snippet_start_index..].find("\n") {
                let content_starting_index = snippet_start_index + snippet_start_tag_end_index;
                let end_index = source
                    .find(String::from(prefix.as_str().to_owned() + snippet_end.as_str()).as_str())
                    .unwrap_or(source.len());
                source.replace_range(
                    content_starting_index..end_index,
                    format!("\n{}", content).as_str(),
                );
            }
        }
    }
    Ok(())
}

pub fn update_target_string_snippet(
    source: &mut String,
    snippet: &Snippet,
    snippet_settings: &SnippextSettings
) -> SnippextResult<()> {
    for prefix in &snippet_settings.comment_prefixes {
        // TODO: create helper method for building prefix+being+ident string
        if let Some(snippet_start_index) =
        source.find(String::from(prefix.as_str().to_owned() + snippet_settings.begin.as_str() + snippet.identifier.as_str()).as_str())
        {
            // TODO: extract attribute from snippet
            // TODO: should find/use template
            if let Some(snippet_start_tag_end_index) = source[snippet_start_index..].find("\n") {
                let snippet_include_start = &source[snippet_start_index..snippet_start_index + snippet_start_tag_end_index];
                let mut attributes = BTreeMap::new();
                let last_square_bracket_pos = snippet_include_start.rfind('[');
                if let Some(last_square_bracket_pos) = last_square_bracket_pos {
                    // TODO: make regex const?
                    // TODO: extract to fn
                    let re = Regex::new("\\[([^]]+)]").unwrap();
                    let captured_kv = re.captures(snippet_include_start);
                    if captured_kv.is_some() {
                        for kv in captured_kv.unwrap().get(1).unwrap().as_str().split(",") {
                            let parts: Vec<&str> = kv.split("=").collect();
                            println!("found attribute [{:?}]", parts);
                            if parts.len() == 2 {
                                attributes.insert(
                                    parts.get(0).unwrap().to_string(),
                                    parts.get(1).unwrap().to_string(),
                                );
                            }
                        }
                    }
                }

                let result = SnippextTemplate::render_template(snippet, snippet_settings, Some(attributes))?;
                let content_starting_index = snippet_start_index + snippet_start_tag_end_index;
                let end_index = source
                    .find(String::from(prefix.as_str().to_owned() + snippet_settings.end.as_str() + snippet.identifier.as_str()).as_str())
                    .unwrap_or(source.len());
                source.replace_range(
                    content_starting_index..end_index,
                    format!("\n{}", result).as_str(),
                );

                println!("source: [{}]", source);
            }
        }
    }
    Ok(())
}

pub fn extract_snippets(
    comment_prefixes: &HashSet<String>,
    begin_pattern: String,
    end_pattern: String,
    filename: &Path,
) -> SnippextResult<Vec<Snippet>> {
    let f = File::open(filename)?;
    let reader = BufReader::new(f);

    let mut snippets: Vec<Snippet> = Vec::new();
    for line in reader.lines() {
        let l = line?;

        let begin_ident = matches(&l, comment_prefixes, &begin_pattern);
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
                                parts.get(1).unwrap().to_string(),
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
fn get_filenames(settings: &SnippextSettings) -> SnippextResult<Vec<SourceFile>> {
    let mut source_files: Vec<SourceFile> = Vec::new();

    for source in &settings.sources {
        if source.is_remote() {
            git::checkout_files(
                source.repository.clone().unwrap(),
                source.branch.clone(),
                source.cone_patterns.clone(),
                source.directory.clone(),
            );
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
                failures.push(format!("templates[{}].identifier must not be an empty string", i));
            }

            if template.1.content.is_empty() {
                failures.push(format!("templates[{}].content must not be an empty string", i));
            }

            if template.1.default {
                default_templates = default_templates + 1;
            }
        }

        if settings.templates.len() > 1 && default_templates == 0 {
            failures.push(String::from("When multiple templates are defined one must be marked default"));
        }

        if default_templates > 1 {
            failures.push(String::from("templates must have only one marked as default"));
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

pub struct InitSettings {
    pub default: bool,
}

fn init(settings: InitSettings) -> SnippextResult<()> {
    return if settings.default {
        fs::write("./snippext.yaml", DEFAULT_SNIPPEXT_CONFIG)?;
        Ok(())
    } else {
        Ok(())
    }
}

pub struct CleanSettings {
    pub begin: String,
    pub end: String,
    pub comment_prefixes: Vec<String>,
    pub output_dir: Option<String>,
    pub targets: Option<Vec<String>>,
}

fn validate_clean_settings(settings: &CleanSettings) -> SnippextResult<()> {
    let mut failures = vec![];

    if settings.begin.is_empty() {
        failures.push("begin must not be empty".to_string())
    }

    if settings.end.is_empty() {
        failures.push("end must not be empty".to_string())
    }

    if settings.comment_prefixes.is_empty() {
        failures.push("Must provide at least one comment prefix".to_string())
    }

    if settings.targets.is_none() && settings.output_dir.is_none() {
        failures.push("Must specify targets or output_dir".to_string())
    }

    return if failures.is_empty() {
        Ok(())
    } else {
        Err(SnippextError::ValidationError(failures))
    };
}

fn clean(settings: CleanSettings) -> SnippextResult<()> {
    validate_clean_settings(&settings)?;

    if let Some(targets) = settings.targets {
        clean_targets(
            settings.begin.as_str(),
            settings.end.as_str(),
            settings.comment_prefixes,
            targets,
        );
    }

    // if let Some(output_dir) = settings.output_dir {
    //     fs::remove_dir_all(output_dir)?;
    // }

    Ok(())
}

// TODO: move write out or provide way to test
fn clean_targets(
    begin: &str,
    end: &str,
    comment_prefixes: Vec<String>,
    targets: Vec<String>,
) -> SnippextResult<()> {
    for target in targets {
        let mut f = File::open(&target)?;
        let reader = BufReader::new(f);

        let mut omit = false;
        let mut new_lines: Vec<String> = Vec::new();
        // https://github.com/temporalio/snipsync/blob/891805910946cca06de074a77cec27bffdfc4cc9/src/Sync.js#L372
        for line in reader.lines() {
            let l = line?;

            for prefix in &comment_prefixes {
                if l.contains(String::from(prefix.to_owned() + begin).as_str()) {
                    omit = true;
                    break;
                }
                if !omit {
                    new_lines.push(l.clone());
                }
                if l.contains(String::from(prefix.to_owned() + end).as_str()) {
                    omit = false;
                }
            }
        }

        let new_content = new_lines
            .into_iter()
            .fold(String::new(), |content, s| content + s.as_str() + "\n");
        fs::write(&target, new_content.as_bytes())?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use std::collections::{HashMap, HashSet};
    use crate::error::SnippextError;
    use crate::{CleanSettings, SnippetSource, SnippextSettings, SnippextTemplate};
    use std::fs;
    use std::io::Write;
    use tempfile::{tempdir, NamedTempFile};

    // https://users.rust-lang.org/t/whats-the-rust-way-to-unit-test-for-an-error/23677/2
    #[test]
    fn strings_must_not_be_empty() {
        let settings = SnippextSettings::new(
            HashSet::from([String::from("# ")]),
            String::from(""),
            String::from(""),
            String::from(""),
            HashMap::from([
                              ("".to_string(), SnippextTemplate {
                                  content: "".to_string(),
                                  default: false
                              })
                ]),
            vec![SnippetSource::new_local(vec![String::from("**")])],
            Some(String::from("./snippets/")),
            None,
        );

        let validation_result = super::run(settings);
        let error = validation_result.err().unwrap();
        match error {
            SnippextError::ValidationError(failures) => {
                println!("{:?}", failures);
                assert_eq!(5, failures.len());
                assert!(failures.contains(&String::from("begin must not be an empty string")));
                assert!(failures.contains(&String::from("end must not be an empty string")));
                assert!(failures.contains(&String::from("templates[0].identifier must not be an empty string")));
                assert!(failures.contains(&String::from("templates[0].content must not be an empty string")));
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
        );

        let validation_result = super::run(settings);
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
                ("first".to_string(), SnippextTemplate {
                    content: String::from("{{snippet}}"),
                    default: false
                }),
                ("second".to_string(), SnippextTemplate {
                    content: String::from("{{snippet}}"),
                    default: false
                }),
            ]),
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
                ("first".to_string(), SnippextTemplate {
                    content: String::from("{{snippet}}"),
                    default: true
                }),
                ("second".to_string(), SnippextTemplate {
                    content: String::from("{{snippet}}"),
                    default: true
                }),
            ]),
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
            HashMap::from([
                ("default".to_string(), SnippextTemplate {
                    content: String::from("{{snippet}}"),
                    default: true
                })
            ]),
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
            HashMap::from([
                ("default".to_string(), SnippextTemplate {
                    content: String::from("{{snippet}}"),
                    default: true
                })
            ]),
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
            HashMap::from([
                ("default".to_string(), SnippextTemplate {
                    content: String::from("{{snippet}}"),
                    default: true
                })
            ]),
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
            HashMap::from([
                ("default".to_string(), SnippextTemplate {
                    content: String::from("{{snippet}}"),
                    default: true
                })
            ]),
            vec![SnippetSource {
                repository: None,
                branch: Some(String::from("branch")),
                commit: Some(String::from("commit")),
                cone_patterns: None,
                directory: None,
                files: vec![String::from("**")],
            }],
            Some(String::from("./snippets/")),
            None,
        );

        let validation_result = super::run(settings);
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

    #[test]
    fn update_target() {
        let mut source = r#"Some content
# snippet::foo
foo
# end::foo
"#
        .to_string();

        super::update_target_string(
            &mut source,
            &vec![String::from("# ")],
            String::from("snippet::foo"),
            String::from("end::foo"),
            "\nbar\n",
        );

        // TODO: assert something
        println!("{}", source);
    }

    #[test]
    fn clean_target() {
        let mut target = NamedTempFile::new().unwrap();
        target.write(
            r#"# Some content
# snippet::foo
foo
# end::foo

More content
"#
            .as_bytes(),
        );

        super::clean_targets(
            "snippet::",
            "end::",
            vec![String::from("# ")],
            vec![String::from(target.path().to_string_lossy())],
        );

        let actual = fs::read_to_string(target.path()).unwrap();
        let expected = r#"# Some content

More content
"#;
        assert_eq!(expected, actual);
    }

    #[test]
    fn clean_target_starting_with_snippet() {
        let mut target = NamedTempFile::new().unwrap();
        target.write(
            r#"# snippet::foo
# end::foo
"#
            .as_bytes(),
        );

        super::clean_targets(
            "snippet::",
            "end::",
            vec![String::from("# ")],
            vec![String::from(target.path().to_string_lossy())],
        );

        let actual = fs::read_to_string(target.path()).unwrap();
        assert_eq!("", actual);
    }

    #[test]
    fn clean_target_should_require_at_least_one_prefix() {
        let validation_result = super::clean(CleanSettings {
            begin: String::from("snippet::"),
            end: String::from("end::"),
            comment_prefixes: vec![],
            output_dir: None,
            targets: Some(vec!["".to_string()]),
        });

        let error = validation_result.err().unwrap();
        match error {
            SnippextError::ValidationError(failures) => {
                assert_eq!(1, failures.len());
                assert_eq!(
                    String::from("Must provide at least one comment prefix"),
                    failures.get(0).unwrap().to_string()
                )
            }
            _ => {
                panic!("invalid SnippextError");
            }
        }
    }

    #[test]
    fn clean_target_should_require_non_empty_begin_and_end() {
        let validation_result = super::clean(CleanSettings {
            begin: String::from(""),
            end: String::from(""),
            comment_prefixes: vec![String::from("# ")],
            output_dir: None,
            targets: Some(vec!["".to_string()]),
        });

        let error = validation_result.err().unwrap();
        match error {
            SnippextError::ValidationError(failures) => {
                assert_eq!(2, failures.len());
                assert_eq!(
                    String::from("begin must not be empty"),
                    failures.get(0).unwrap().to_string()
                );
                assert_eq!(
                    String::from("end must not be empty"),
                    failures.get(1).unwrap().to_string()
                );
            }
            _ => {
                panic!("invalid SnippextError");
            }
        }
    }

    #[test]
    fn clean_target_should_require_targets_or_output_dir() {
        let validation_result = super::clean(CleanSettings {
            begin: String::from("snippet::"),
            end: String::from("end::"),
            comment_prefixes: vec![String::from("# ")],
            output_dir: None,
            targets: None,
        });

        let error = validation_result.err().unwrap();
        match error {
            SnippextError::ValidationError(failures) => {
                assert_eq!(1, failures.len());
                assert_eq!(
                    String::from("Must specify targets or output_dir"),
                    failures.get(0).unwrap().to_string()
                );
            }
            _ => {
                panic!("invalid SnippextError");
            }
        }
    }
}
