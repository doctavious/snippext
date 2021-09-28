mod sanitize;
mod unindent;

use sanitize::sanitize;
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::fs::File;
use std::fs;
use std::io::prelude::*;
use std::io::BufReader;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;
use std::collections::HashMap;
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
    pub fn new(identifier: String) -> Snippet {
        Snippet {
            identifier,
            text: "".to_string(),
            closed: false,
            attributes: HashMap::new(),
        }
    }
}

// TODO: return result
pub fn extract(
    comment_prefix: String,
    begin: String,
    end: String,
    output_dir: String,
    extension: String,
    sources: Vec<String>
)
{
    let filenames = get_filenames(sources);
    for filename in filenames {
        let snippets = extract_snippets(
            comment_prefix.to_owned(),
            begin.to_owned(),
            end.to_owned(),
            filename.as_path()
        ).unwrap();

        for snippet in snippets {

            let x: &[_] = &['.', '/'];
            let output_path = Path::new(output_dir.as_str())
                .join(filename.as_path().to_string_lossy().trim_start_matches(x))
                .join(sanitize(snippet.identifier))
                .with_extension(extension.as_str());

            fs::create_dir_all(output_path.parent().unwrap()).unwrap();

            // TODO: support custom template
            // TODO: should we include a comment that the file is generated?

            fs::write(output_path, unindent(snippet.text.as_str())).unwrap();
        }
    }
}

pub fn extract_snippets(
    comment_prefix: String,
    begin_pattern: String,
    end_pattern: String,
    filename: &Path,
) -> Result<Vec<Snippet>, Box<dyn Error>> {
    let f = File::open(filename)?;
    let reader = BufReader::new(f);

    let mut snippets: Vec<Snippet> = Vec::new();
    for line in reader.lines() {
        let l = line?;

        let begin_ident = matches(&l, String::from(comment_prefix.as_str()) + &begin_pattern);
        if begin_ident != "" {
            let snippet = Snippet::new(begin_ident);
            snippets.push(snippet);
            continue;
        }

        let end_ident = matches(&l, String::from(comment_prefix.as_str()) + &end_pattern);
        if end_ident != "" {
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
        let path = Path::new(&source);
        if !path.is_dir() {
            out.push(path.to_path_buf())
        }

        for entry in WalkDir::new(&source)
            .into_iter()
            .filter_map(Result::ok)
            .filter(|e| !e.file_type().is_dir())
        {
            out.push(entry.path().to_path_buf());
        }
    }
    out
}

fn matches(s: &String, prefix: String) -> String {
    let trimmed = s.trim();
    let len_diff = s.len() - trimmed.len();
    if trimmed.starts_with(&prefix) {
        // don't include attributes, starting with '['
        return s[prefix.len() + len_diff..].chars().take_while(|&c| c != '[').collect();
    }
    return String::from("");
}


