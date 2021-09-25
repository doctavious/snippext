use structopt::StructOpt;
use std::path::{Path, PathBuf};

use std::io::prelude::*;
use std::io::BufReader;
use std::fs::File;
use walkdir::WalkDir;

use serde::{Serialize, Deserialize};
use std::error::Error;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Snippet {
    identifier: String,
    text: String,
    closed: bool,
}

impl Snippet {

    pub fn new(identifier: String) -> Snippet {
        Snippet {
            identifier,
            text: "".to_string(),
            closed: false
        }
    }

}



pub fn extract_snippets(begin_pattern: String, end_pattern: String, filename: String) -> Result<Vec<Snippet>, Box<dyn Error>> {
    let f = File::open(filename)?;
    let reader = BufReader::new(f);

    let mut snippets: Vec<Snippet> = Vec::new();
    for line in reader.lines() {
        let l = line?;

        let begin_ident = matches(&l, &begin_pattern);
        if begin_ident != "" {
            let snippet = Snippet::new(begin_ident);
            snippets.push(snippet);
            continue
        }

        let end_ident = matches(&l, &end_pattern);
        if end_ident != "" {
            for snippet in snippets.iter_mut() {
                if snippet.identifier == end_ident {
                    snippet.closed = true
                }
            }
			continue
		}
		for snippet in snippets.iter_mut() {
			// snippet := &snippets[i]
			if snippet.closed {
				continue
			}
			snippet.text = String::from(snippet.text.as_str()) + l.as_str() + "\n"
		}
    }

    Ok(snippets)
}

fn matches(s: &String, prefix: &String) -> String {
    let trimmed = s.trim();
    let len_diff = s.len() - trimmed.len();
    if trimmed.starts_with(prefix) {
        return s[prefix.len() + len_diff..].to_string();
    }
	return String::from("");
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
            .filter(|e| !e.file_type().is_dir()) {
                out.push(entry.path().to_path_buf());
        }
    }

    out
}


#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
