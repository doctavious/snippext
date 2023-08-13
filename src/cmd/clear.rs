use std::collections::{HashMap, HashSet};
use std::collections::hash_map::Entry;
use std::fs;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;

use clap::Parser;
use config::{Config, Environment, File};
use serde::{Deserialize, Serialize};

use crate::error::SnippextError;
use crate::{files, SnippextResult};
use crate::cmd::is_line_snippet;

#[derive(Clone, Debug, Parser)]
#[command()]
pub struct Args {
    #[arg(short, long, value_parser, help = "Config file to use")]
    pub config: Option<PathBuf>,

    #[arg(short, long, help = "Flag to mark beginning of a snippet")]
    pub begin: Option<String>,

    #[arg(short, long, help = "Flag to mark ending of a snippet")]
    pub end: Option<String>,

    // globs
    #[arg(
        short = 'T',
        long,
        required_unless_present = "output_dir",
        help = "The local directories that contain the files to be spliced with the code snippets."
    )]
    pub targets: Option<Vec<String>>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct ClearSettings {
    pub begin: String,
    pub end: String,
    pub targets: Vec<String>,
}

/// Removes snippets from target files
pub fn execute(clear_opt: Args) -> SnippextResult<()> {
    let settings = build_clear_settings(clear_opt)?;
    clear(settings)
}

fn build_clear_settings(opt: Args) -> SnippextResult<ClearSettings> {
    let mut builder = Config::builder();

    if let Some(config) = opt.config {
        builder = builder.add_source(File::from(config));
    } else {
        // TODO: use constant
        builder = builder.add_source(File::with_name("snippext").required(false));
    }

    builder = builder.add_source(Environment::with_prefix("snippext"));
    builder = builder.set_override_option("begin", opt.begin)?
        .set_override_option("end", opt.end)?
        .set_override_option("targets",opt.targets)?;

    let settings: ClearSettings = builder.build()?.try_deserialize()?;
    return Ok(settings);
}

/// remove snippets from target files
pub fn clear(settings: ClearSettings) -> SnippextResult<()> {
    validate_clear_settings(&settings)?;

    let mut cache: HashMap<String, (HashSet<String>, HashSet<String>)> = HashMap::new();
    for target in settings.targets {
        let extension = files::extension(target.as_str());
        let (snippet_start_prefixes, snippet_end_prefixes) = match cache.entry(extension.clone()) {
            Entry::Occupied(entry) => entry.into_mut(),
            Entry::Vacant(entry) => entry.insert((
                files::get_snippet_start_prefixes(
                    extension.as_str().clone(),
                    settings.begin.as_str())?,
                files::get_snippet_end_prefixes(
                    extension.clone().as_str(),
                    settings.end.as_str())?
            ))
        };

        let f = fs::File::open(&target)?;
        let reader = BufReader::new(f);

        let mut omit = false;
        let mut new_lines: Vec<String> = Vec::new();
        // https://github.com/temporalio/snipsync/blob/891805910946cca06de074a77cec27bffdfc4cc9/src/Sync.js#L372
        for line in reader.lines() {
            let l = line?;

            if is_line_snippet(l.as_str(), &snippet_end_prefixes).is_some() {
                omit = false;
            }

            if !omit {
                new_lines.push(l.clone());
            }

            if is_line_snippet(l.as_str(), &snippet_start_prefixes).is_some() {
                omit = true;
            }
        }

        let new_content = new_lines
            .into_iter()
            .fold(String::new(), |content, s| content + s.as_str() + "\n");
        fs::write(&target, new_content.as_bytes())?;
    }

    Ok(())
}

fn validate_clear_settings(settings: &ClearSettings) -> SnippextResult<()> {
    let mut failures = vec![];

    if settings.begin.is_empty() {
        failures.push("begin must not be empty".to_string())
    }

    if settings.end.is_empty() {
        failures.push("end must not be empty".to_string())
    }

    if settings.targets.is_empty() {
        failures.push("Must specify targets".to_string())
    }

    return if failures.is_empty() {
        Ok(())
    } else {
        Err(SnippextError::ValidationError(failures))
    };
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::io::Write;

    use tempfile::NamedTempFile;

    use crate::cmd::clear::ClearSettings;
    use crate::error::SnippextError; // TODO: why cant we use crate:error here?

    #[test]
    fn clear_target() {
        let mut target = NamedTempFile::new().unwrap();
        target.write(
            r#"# Some content
# snippet::foo
foo
# end::foo

More content
"#
            .as_bytes(),
        ).unwrap();

        super::clear(ClearSettings {
            begin: "snippet::".to_string(),
            end: "end::".to_string(),
            targets: vec![String::from(target.path().to_string_lossy())],
        }).unwrap();

        let actual = fs::read_to_string(target.path()).unwrap();
        let expected = r#"# Some content
# snippet::foo
# end::foo

More content
"#;
        assert_eq!(expected, actual);
    }

    #[test]
    fn clear_target_starting_with_snippet() {
        let mut target = NamedTempFile::new().unwrap();
        target.write(
            r#"# snippet::foo
# end::foo
"#
            .as_bytes(),
        ).unwrap();

        super::clear(ClearSettings {
            begin: "snippet::".to_string(),
            end: "end::".to_string(),
            targets: vec![String::from(target.path().to_string_lossy())],
        }).unwrap();

        let actual = fs::read_to_string(target.path()).unwrap();
        let expected = r#"# snippet::foo
# end::foo
"#;
        assert_eq!(expected, actual);
    }

    #[test]
    fn clear_target_should_require_non_empty_begin_and_end() {
        let validation_result = super::clear(ClearSettings {
            begin: String::from(""),
            end: String::from(""),
            targets: vec!["".to_string()],
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
}
