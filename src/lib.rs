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

use glob::glob;
use git2::{build::CheckoutBuilder, Cred, Error as GitError, RemoteCallbacks, Repository};
use regex::Regex;
use sanitize::sanitize;
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::fs::File;
use std::{fs, env};
use std::io::prelude::*;
use std::io::BufReader;
use std::path::{Path, PathBuf};
use std::collections::HashMap;
use tera::{Context, Tera};
use unindent::unindent;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Snippet {
    // The snippet name is sanitized to prevent malicious code to overwrite arbitrary files on your system.
    pub identifier: String,
    pub text: String,
    pub closed: bool,
    pub attributes: HashMap<String, String>,
}

impl Snippet {
    pub fn new(identifier: String, attributes: HashMap<String, String>) -> Snippet {
        Snippet {
            identifier,
            text: "".to_string(),
            closed: false,
            attributes
        }
    }
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct SnippetSettings {
    pub begin: String,
    pub end: String,
    pub extension: String,
    pub comment_prefixes: Vec<String>,
    pub template: String,
    pub sources: SnippetSource,
    pub output_dir: Option<String>,
    pub targets: Option<Vec<String>>,
}

impl SnippetSettings {

    // TODO: add default
    pub fn default() -> Self {
        Self {
            begin: String::from(""),
            end: String::from(""),
            extension: String::from(""),
            comment_prefixes: vec![],
            template: String::from(""),
            sources: SnippetSource::new_local(vec![]),
            output_dir: None,
            targets: None,
        }
    }

    pub fn new (
        comment_prefixes: Vec<String>,
        begin: String,
        end: String,
        output_dir: Option<String>,
        extension: String,
        template: String,
        sources: Vec<String>
    ) -> Self {
        Self {
            begin,
            end,
            extension,
            comment_prefixes,
            template,
            sources: SnippetSource::new_local(sources),
            output_dir,
            targets: None,
        }
    }
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct SnippetSource {
    pub repository: Option<String>,
    pub branch: Option<String>,
    // TODO: rename commit....
    pub starting_point: Option<String>,
    pub directory: Option<String>, // default to "."
    pub files: Vec<String>,
}

impl SnippetSource {
    pub fn new_local(sources: Vec<String>) -> Self {
        Self {
            repository: None,
            branch: None,
            starting_point: None,
            directory: None,
            files: sources
        }
    }

    pub fn new_remote(
        repository: String,
        branch: String,
        starting_point: Option<String>,
        directory: Option<String>,
        files: Vec<String>,
    ) -> Self {
        Self {
            repository: Some(repository),
            branch: Some(branch),
            starting_point,
            directory,
            files
        }
    }
}

// TODO: return result
pub fn run(snippet_settings: SnippetSettings)
{
    let filenames = get_filenames(snippet_settings.sources.files);
    for filename in filenames {
        let snippets = extract_snippets(
            &snippet_settings.comment_prefixes,
            snippet_settings.begin.to_owned(),
            snippet_settings.end.to_owned(),
            filename.as_path()
        ).unwrap();

        // TODO: output_dir optional
        let output_dir = snippet_settings.output_dir.as_ref().unwrap();
        for snippet in snippets {
            let x: &[_] = &['.', '/'];
            let output_path = Path::new(output_dir.as_str())
                .join(filename.as_path().to_string_lossy().trim_start_matches(x))
                .join(sanitize(snippet.identifier))
                .with_extension(snippet_settings.extension.as_str());

            fs::create_dir_all(output_path.parent().unwrap()).unwrap();

            let mut context = Context::new();
            context.insert("snippet", unindent(snippet.text.as_str()).as_str());
            for attribute in snippet.attributes {
                context.insert(&attribute.0.to_string(), &attribute.1.to_string());
            }

            let result = Tera::one_off(snippet_settings.template.as_str(), &context, false).unwrap();
            fs::write(output_path, result).unwrap();
        }
    }
}


pub fn extract_snippets(
    comment_prefixes: &Vec<String>,
    begin_pattern: String,
    end_pattern: String,
    filename: &Path,
) -> Result<Vec<Snippet>, Box<dyn Error>> {
    let f = File::open(filename)?;
    let reader = BufReader::new(f);

    let mut snippets: Vec<Snippet> = Vec::new();
    for line in reader.lines() {
        let l = line?;

        let begin_ident = matches(&l, &comment_prefixes, &begin_pattern);
        if !begin_ident.is_empty() {
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
        if !end_ident.is_empty() {
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

// if an entry is a directory all files from directory will be listed.
fn get_filenames(sources: Vec<String>) -> Vec<PathBuf> {
    let mut out: Vec<PathBuf> = Vec::new();

    for source in sources {
        // TODO: do we want to print failures and continue rather than unwrap?
        for entry in glob(&source).unwrap() {
            let path = entry.unwrap();
            if !path.is_dir() {
                out.push(path);
            }
        }
    }
    out
}

fn matches(s: &str, comment_prefixes: &[String], pattern: &str) -> String {
    let trimmed = s.trim();
    let len_diff = s.len() - trimmed.len();
    for comment_prefix in comment_prefixes {
        let prefix = String::from(comment_prefix.as_str()) + pattern;
        if trimmed.starts_with(&prefix) {
            return s[prefix.len() + len_diff..].to_string();
        }
    }
    String::from("")
}

// TODO: Do we need to allow users to specify path to clone to and path of ssh creds?
// sparse clone / depth 1?
// git2-rs doesnt appear to support sparse checkout, yet, because lib2git doesnt
fn git_clone(remote: &str) {
    // HTTP clone
    let repo = match Repository::clone(remote, "/path/to/a/repo") {
        Ok(repo) => repo,
        Err(e) => panic!("failed to clone: {}", e),
    };

    // SSH clone
    // Prepare callbacks.
    let mut callbacks = RemoteCallbacks::new();
    callbacks.credentials(|_url, username_from_url, _allowed_types| {
        Cred::ssh_key(
            username_from_url.unwrap(),
            None,
            std::path::Path::new(&format!("{}/.ssh/id_rsa", env::var("HOME").unwrap())),
            None,
        )
    });

    // Prepare fetch options.
    let mut fo = git2::FetchOptions::new();
    fo.remote_callbacks(callbacks);

    // Prepare builder.
    let mut builder = git2::build::RepoBuilder::new();
    builder.fetch_options(fo);

    // let mut checkout_builder = CheckoutBuilder::new()

    // Clone the project.
    builder.clone(
        "git@github.com:rust-lang/git2-rs.git",
        Path::new("/tmp/git2-rs"),
    );
}

