use std::fs;
use std::io::BufRead;
use std::io::BufReader;
use std::path::PathBuf;
use clap::Parser;
use config::{Config, Environment, File};
use serde::{Deserialize, Serialize};
use crate::error::SnippextError;
use crate::SnippextResult;

#[derive(Clone, Debug, Parser)]
#[command()]
pub struct Args {
    #[arg(short, long, value_parser, help = "Config file to use")]
    pub config: Option<PathBuf>,

    #[arg(short, long, help = "flag to mark beginning of a snippet")]
    pub begin: Option<String>,

    #[arg(short, long, help = "flag to mark ending of a snippet")]
    pub end: Option<String>,

    // TODO: make vec default to ["// ", "<!-- "]
    // The tag::[] and end::[] directives should be placed after a line comment as defined by the language of the source file.
    // comment prefix
    #[arg(short = 'p', long, help = "Prefixes to use for comments")]
    pub comment_prefixes: Option<Vec<String>>,

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
    pub comment_prefixes: Vec<String>,
    pub targets: Option<Vec<String>>,
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

    if let Some(begin) = opt.begin {
        builder = builder.set_override("begin", begin)?;
    }

    if let Some(end) = opt.end {
        builder = builder.set_override("end", end)?;
    }

    if let Some(comment_prefixes) = opt.comment_prefixes {
        builder = builder.set_override("comment_prefixes", comment_prefixes)?;
    }

    if let Some(targets) = opt.targets {
        builder = builder.set_override("targets", targets)?;
    }

    let settings: ClearSettings = builder.build()?.try_deserialize()?;
    return Ok(settings);
}

// TODO: this probably goes in lib?
/// remove snippets from target files
pub fn clear(settings: ClearSettings) -> SnippextResult<()> {
    validate_clear_settings(&settings)?;

    clear_targets(
        settings.begin.as_str(),
        settings.end.as_str(),
        settings.comment_prefixes,
        settings.targets.unwrap(),
    )
}

// TODO: move write out or provide way to test
fn clear_targets(
    begin: &str,
    end: &str,
    comment_prefixes: Vec<String>,
    targets: Vec<String>,
) -> SnippextResult<()> {
    for target in targets {
        let f = fs::File::open(&target)?;
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

fn validate_clear_settings(settings: &ClearSettings) -> SnippextResult<()> {
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

    if settings.targets.is_none() {
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
    use crate::cmd::clear::ClearSettings;
    use crate::error::SnippextError;
    use std::fs;
    use std::io::Write;
    use tempfile::{NamedTempFile}; // TODO: why cant we use crate:error here?

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
        );

        super::clear_targets(
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
    fn clear_target_starting_with_snippet() {
        let mut target = NamedTempFile::new().unwrap();
        target.write(
            r#"# snippet::foo
# end::foo
"#
            .as_bytes(),
        );

        super::clear_targets(
            "snippet::",
            "end::",
            vec![String::from("# ")],
            vec![String::from(target.path().to_string_lossy())],
        );

        let actual = fs::read_to_string(target.path()).unwrap();
        assert_eq!("", actual);
    }

    #[test]
    fn clear_target_should_require_at_least_one_prefix() {
        let validation_result = super::clear(ClearSettings {
            begin: String::from("snippet::"),
            end: String::from("end::"),
            comment_prefixes: vec![],
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
    fn clear_target_should_require_non_empty_begin_and_end() {
        let validation_result = super::clear(ClearSettings {
            begin: String::from(""),
            end: String::from(""),
            comment_prefixes: vec![String::from("# ")],
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
    fn clear_target_should_require_targets_or_output_dir() {
        let validation_result = super::clear(ClearSettings {
            begin: String::from("snippet::"),
            end: String::from("end::"),
            comment_prefixes: vec![String::from("# ")],
            targets: None,
        });

        let error = validation_result.err().unwrap();
        match error {
            SnippextError::ValidationError(failures) => {
                assert_eq!(1, failures.len());
                assert_eq!(
                    String::from("Must specify targets"),
                    failures.get(0).unwrap().to_string()
                );
            }
            _ => {
                panic!("invalid SnippextError");
            }
        }
    }
}
