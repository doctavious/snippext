use std::collections::hash_map::Entry;
use std::collections::{HashMap, HashSet};
use std::{env, fs};
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::time::SystemTime;

use chrono::DateTime;
use clap::Parser;
use config::{Config, Environment, FileFormat};
use filetime::{set_file_mtime, FileTime};
use glob::{glob, Pattern};
use regex::Regex;
use reqwest::blocking::Client;
use reqwest::header::{HeaderValue, EXPIRES, LAST_MODIFIED};
use serde_json::json;
use tracing::warn;
use url::Url;
use walkdir::WalkDir;

use crate::cmd::is_line_snippet;
use crate::constants::{
    DEFAULT_OUTPUT_FILE_EXTENSION, DEFAULT_SOURCE_FILES, DEFAULT_TEMPLATE_IDENTIFIER,
};
use crate::error::SnippextError;
use crate::sanitize::sanitize;
use crate::templates::SnippextTemplate;
use crate::types::{LinkFormat, Snippet, SnippetSource};
use crate::{files, git, SnippextResult, SnippextSettings};

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
        short,
        long,
        value_name = "DIR",
        help = "Directory where templates exists. File names act as keys"
    )]
    pub templates: Option<String>,

    #[arg(long, value_name = "URL", help = "")]
    pub repository_url: Option<String>,

    #[arg(long, requires = "repository_url", value_name = "REF", help = "")]
    pub repository_ref: Option<String>,

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

    #[arg(
        short = 'x',
        long,
        help = "Extension for generated files. Defaults to txt when not specified."
    )]
    pub output_extension: Option<String>,

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
    // default to ** or if none and recusively walk everything
    #[arg(short, long, help = "TODO: ...")]
    pub sources: Vec<String>,

    /// Urls to files to be included as snippets.
    /// Each url will be accessible using the file name as a key.
    /// Any snippets within the files will be extracted and accessible as individual keyed snippets.
    #[arg(
        long,
        help = "URLs to be included in snippets. URL must return raw text in order for snippets to\
                be successfully extracted."
    )]
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

    let mut cache = HashMap::new();
    let source_files = get_source_files(&snippext_settings)?;
    for source_file in source_files {
        let snippets = extract_snippets(
            source_file.full_path.as_path(),
            &snippext_settings,
            &mut cache,
        )?;

        if snippets.is_empty() {
            continue;
        }

        if let Some(output_dir) = &snippext_settings.output_dir {
            let extension = snippext_settings
                .output_extension
                .as_deref()
                .unwrap_or(DEFAULT_OUTPUT_FILE_EXTENSION);
            for (_, snippet) in &snippets {
                let trim_chars: &[_] = &['.', '/'];
                let output_path = Path::new(output_dir.as_str())
                    .join(
                        source_file
                            .relative_path
                            .to_string_lossy()
                            .trim_start_matches(trim_chars),
                    )
                    .join(sanitize(snippet.identifier.to_owned()))
                    .with_extension(extension);

                fs::create_dir_all(output_path.parent().unwrap()).unwrap();
                let result = SnippextTemplate::render_template(snippet, &snippext_settings, None)?;
                fs::write(output_path, result).unwrap();
            }
        }

        if let Some(targets) = &snippext_settings.targets {
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
                    process_target_file(entry.unwrap(), &snippets, &snippext_settings, &mut cache)?;
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

    if settings.sources.is_empty() {
        failures.push(String::from("sources must not be empty"));
    } else {
        for (i, source) in settings.sources.iter().enumerate() {

            if !source.is_url() {
                if source.files.is_empty() {
                    failures.push(format!("sources[{}].files must not be empty", i));
                }
            }

            if (source.repository.is_none() || source.repository.as_ref().unwrap() == "")
                && (source.cone_patterns.is_some() || source.repository_ref.is_some())
            {
                failures.push(format!(
                    "sources[{}] specifies ref, cone_patterns without specifying repository",
                    i
                ));
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

// TODO: we should instead be collecting snippets
// TODO: This code is absolute shit. Clean this up
// Need both the absolute path and the relative path so that for when we output generated files
// we only include relative directories within the output directory.
fn get_source_files(settings: &SnippextSettings) -> SnippextResult<Vec<SourceFile>> {
    let mut source_files: Vec<SourceFile> = Vec::new();

    for source in &settings.sources {
        if source.is_remote() {
            let repo = source.repository.as_ref().unwrap();
            let repo_name = Path::new(repo)
                .file_stem()
                .ok_or(SnippextError::GeneralError(format!("Could not get repository name from {}", &repo)))?;
            let download_dir = get_download_directory()?.join(repo_name);
            // dont need this second check but being safe
            if download_dir.exists() && download_dir.starts_with(std::env::temp_dir()) {
                fs::remove_dir_all(&download_dir)?;
            }
            fs::create_dir_all(&download_dir)?;
            git::checkout_files(
                &repo,
                source.repository_ref.clone(),
                source.cone_patterns.clone(),
                &download_dir,
            )?;

            let dir_length = download_dir.to_string_lossy().len();
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

fn get_download_directory() -> SnippextResult<PathBuf> {
    let snippext_dir = env::temp_dir().join("snippext");
    if !snippext_dir.exists() {
        fs::create_dir(&snippext_dir)?;
    }

    Ok(snippext_dir)
}

fn url_to_path(url_string: &String) -> SnippextResult<PathBuf> {
    let invalid_chars = [
        '/', '\\', '?', '*', ':', '|', '"', '<', '>', ',', ';', '=', ' ', '.'
    ];

    let url = Url::from_str(url_string.as_str())?;
    let path: String = url.path()
        .chars()
        .map(|c| if invalid_chars.contains(&c) { '_'} else {c})
        .collect();

    Ok(PathBuf::from(url.authority()).join(path))
}

fn download_url(url: &String) -> SnippextResult<PathBuf> {
    let url_file_path = url_to_path(url)?;
    let download_path = get_download_directory()?.join(url_file_path);
    let parent_dirs = download_path
        .parent()
        .ok_or(SnippextError::GeneralError("could not create download directory".into()))?;
    fs::create_dir_all(parent_dirs)?;

    if let Ok(file_metadata) = download_path.metadata() {
        let file_modified = file_metadata.modified().ok();
        if file_modified.is_some_and(|t| t > SystemTime::now()) {
            return Ok(download_path);
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
                return Ok(download_path);
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

    Ok(download_path)
}

// Wed, 16 Aug 2023 22:40:19 GMT
fn header_to_systemtime(header_value: Option<&HeaderValue>) -> Option<SystemTime> {
    let header_value_str = header_value?.to_str().ok()?;
    let date_time = DateTime::parse_from_rfc2822(&header_value_str).ok()?;
    Some(date_time.into())
}

fn extract_snippets(
    path: &Path,
    settings: &SnippextSettings,
    cache: &mut HashMap<String, (HashSet<String>, HashSet<String>)>,
) -> SnippextResult<HashMap<String, Snippet>> {
    println!("extracting from {:?}", path);
    let f = File::open(path)?;
    let reader = BufReader::new(f);

    let mut current_line_number = 0;
    let mut state = Vec::new();
    let mut snippets = HashMap::new();
    let extension = files::extension_from_path(path);

    let (snippet_start_prefixes, snippet_end_prefixes) = match cache.entry(extension.clone()) {
        Entry::Occupied(entry) => entry.into_mut(),
        Entry::Vacant(entry) => entry.insert((
            files::get_snippet_start_prefixes(extension.as_str().clone(), settings.begin.as_str())?,
            files::get_snippet_end_prefixes(extension.clone().as_str(), settings.end.as_str())?,
        )),
    };

    for line in reader.lines() {
        current_line_number += 1;
        let l = line?;
        let current_line = l.trim();

        if let Some(start_prefix) = is_line_snippet(current_line, &snippet_start_prefixes) {
            let mut attributes = HashMap::from([
                ("path".to_string(), path.to_string_lossy().to_string()),
                (
                    "filename".to_string(),
                    path.file_name().unwrap().to_string_lossy().to_string(),
                ),
            ]);

            let Ok((key, snippet_attributes)) = extract_id_and_attributes(current_line, &start_prefix) else {
                // TODO: error
                continue;
            };

            if let Some(snippet_attributes) = snippet_attributes {
                attributes.extend(snippet_attributes);
            }

            state.push(SnippetExtractionState {
                key,
                start_line: current_line_number,
                lines: String::new(),
                attributes,
            });

            continue;
        }

        // currently not in snippet
        if state.is_empty() {
            continue;
        }

        if let Some(_) = is_line_snippet(current_line, &snippet_end_prefixes) {
            if let Some(state) = state.pop() {
                let id = state.key;
                let old_value = snippets.insert(
                    id.clone(),
                    Snippet::new(
                        id.clone(),
                        path.to_path_buf(),
                        state.lines,
                        state.attributes,
                        state.start_line,
                        current_line_number,
                    ),
                );

                if old_value.is_some() {
                    warn!("multiple snippets with id {} found", id.clone());
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
            &path.to_string_lossy(),
            &snippet.start_line
        )));
    }

    println!("{:?}", snippets);
    Ok(snippets)
}

fn extract_id_and_attributes(
    line: &str,
    begin: &String,
) -> SnippextResult<(String, Option<HashMap<String, String>>)> {
    let re = Regex::new(format!("{begin}[ ]*(?P<key>[\\w-]*)(?P<attributes>\\[[^]]+])?").as_str())
        .unwrap();
    let captures = re.captures(line);
    if let Some(capture_groups) = captures {
        let Some(key) = capture_groups.name("key") else {
            return Err(SnippextError::GeneralError(format!("could not extract key from {}", line)));
        };

        let attributes = if let Some(match_attributes) = capture_groups.name("attributes") {
            let mut attributes = HashMap::new();
            let trim_ends: &[_] = &['[', ']'];
            let parts: Vec<&str> = match_attributes
                .as_str()
                .trim_matches(trim_ends)
                .split("=")
                .collect();
            if parts.len() == 2 {
                attributes.insert(
                    parts.get(0).unwrap().to_string(),
                    parts.get(1).unwrap().to_string(),
                );
            }
            Some(attributes)
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

struct MissingSnippet<'a> {
    key: String,
    line_number: u32,
    path: &'a PathBuf,
}

fn process_target_file(
    target: PathBuf,
    snippets: &HashMap<String, Snippet>,
    settings: &SnippextSettings,
    cache: &mut HashMap<String, (HashSet<String>, HashSet<String>)>,
) -> SnippextResult<()> {
    let mut new_file_lines = Vec::new();
    let mut updated = false;
    let mut in_current_snippet = None;
    let mut line_number = 0;
    let mut missing_snippets = Vec::new();
    let extension = files::extension_from_path(target.as_path());
    let (snippet_start_prefixes, snippet_end_prefixes) = match cache.entry(extension.clone()) {
        Entry::Occupied(entry) => entry.into_mut(),
        Entry::Vacant(entry) => entry.insert((
            files::get_snippet_start_prefixes(extension.as_str().clone(), settings.begin.as_str())?,
            files::get_snippet_end_prefixes(extension.clone().as_str(), settings.end.as_str())?,
        )),
    };

    let f = File::open(&target)?;
    let reader = BufReader::new(f);
    for line in reader.lines() {
        line_number = line_number + 1;
        let line = line?;
        let current_line = line.trim();

        if in_current_snippet.is_some() {
            if is_line_snippet(current_line, &snippet_end_prefixes).is_some() {
                new_file_lines.push(line.clone());
                in_current_snippet = None;
            }

            continue;
        }

        new_file_lines.push(line.clone());

        if is_line_snippet(current_line, &snippet_start_prefixes).is_none() {
            continue;
        }

        // TODO: log error
        let Ok((key, attributes)) = extract_id_and_attributes(current_line, &settings.begin) else {
            warn!("Failed to extract id/attributes from snippet. File {} line number {}",
                target.to_string_lossy(),
                line_number
            );
            continue;
        };

        let Some(snippet) = snippets.get(&key) else {
            missing_snippets.push(MissingSnippet {
                key,
                line_number,
                path: &target,
            });
            continue;
        };

        let result = SnippextTemplate::render_template(&snippet, &settings, attributes)?;

        let result_lines: Vec<String> = result.lines().map(|s| s.to_string()).collect();

        new_file_lines.extend(result_lines);
        updated = true;
        in_current_snippet = Some(key);
    }

    if let Some(in_current_snippet) = in_current_snippet {
        return Err(SnippextError::GeneralError(format!(
            "Expected to find end of snippet {}",
            in_current_snippet
        )));
    }

    // TODO: error if fail when missing snippets is true and missing snippets exist
    // log($"WARN: The source file:{missing.File} includes a key {missing.Key}, however the snippet is missing. Make sure that the snippet is defined.");
    // https://github.com/SimonCropp/MarkdownSnippets/blob/1a148e6b8a1054e7ccf8cffaa2280944d9dca1c7/src/MarkdownSnippets/MissingSnippetsException.cs#L4

    if updated {
        fs::write(target.to_path_buf(), new_file_lines.join("\n"))?;
    }

    Ok(())
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

    builder = builder
        .add_source(Environment::with_prefix("snippext"))
        .set_override_option("begin", opt.begin)?
        .set_override_option("end", opt.end)?
        .set_override_option("output_dir", opt.output_dir)?
        .set_override_option("output_extension", opt.output_extension)?;

    if !opt.targets.is_empty() {
        builder = builder.set_override("targets", opt.targets)?;
    }

    if let Some(link_format) = opt.link_format {
        builder = builder.set_override("link_format", link_format.to_string())?;
    }

    if let Some(url_prefix) = opt.url_prefix {
        builder = builder.set_override("url_prefix", url_prefix)?;
    }

    println!("{:?}", opt.templates);
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

            templates.insert(
                file_name.to_string_lossy().to_string(),
                SnippextTemplate {
                    content,
                    default: file_name == DEFAULT_TEMPLATE_IDENTIFIER,
                },
            );
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

        let source = SnippetSource::new_git(
            repo_url.to_string(),
            opt.repository_ref.unwrap(),
            source_files,
        );
        snippet_sources.push(source);
    } else if !opt.sources.is_empty() {
        snippet_sources.push(SnippetSource::new_local(opt.sources));
    }

    for url_source in opt.url_sources {
        snippet_sources.push(SnippetSource::new_url(url_source));
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
    use std::collections::HashMap;
    use std::fs;
    use std::path::{Path, PathBuf};
    use tempfile::tempdir;

    use super::Args;
    use crate::error::SnippextError;
    use crate::settings::SnippextSettings;
    use crate::templates::SnippextTemplate;
    use crate::types::SnippetSource;

    // #[test]
    // fn default_config_file() {
    //     let opt = Args {
    //         config: None,
    //         begin: None,
    //         end: None,
    //         extension: None,
    //         comment_prefixes: None,
    //         templates: None,
    //         repository_url: None,
    //         repository_ref: None,
    //         output_dir: None,
    //         targets: Vec::default(),
    //         sources: Vec::default(),
    //         url_sources: Vec::default(),
    //         link_format: None,
    //         url_prefix: None,
    //     };
    //
    //     let settings = super::build_settings(opt).unwrap();
    //     // TODO: add asserts
    //     info!("{:?}", settings);
    // }

    #[test]
    fn verify_cli_args() {
        let opt = Args {
            config: None,
            begin: Some(String::from("snippext::begin::")),
            end: Some(String::from("finish::")),
            templates: Some(String::from("./tests/templates")),
            repository_url: Some(String::from("https://github.com/doctavious/snippext.git")),
            repository_ref: Some(String::from("main")),
            sources: vec![String::from("**/*.rs")],
            url_sources: Vec::default(),
            output_dir: Some(String::from("./snippext/")),
            output_extension: Some(String::from("txt")),
            targets: vec![String::from("README.md")],
            link_format: None,
            url_prefix: None,
        };

        let settings = super::build_settings(opt).unwrap();

        assert_eq!("snippext::begin::", settings.begin);
        assert_eq!("finish::", settings.end);
        assert_eq!(Some("txt".into()), settings.output_extension);

        println!("{:?}", settings.templates);
        assert_eq!(1, settings.templates.len());

        let default_template = settings.templates.get("default").unwrap();
        assert_eq!("````\n{{snippet}}\n```", default_template.content);
        assert!(default_template.default);
        assert_eq!(Some(String::from("./snippext/")), settings.output_dir);
        assert_eq!(Some(vec![String::from("README.md")]), settings.targets);

        assert_eq!(1, settings.sources.len());
        let source = settings.sources.get(0).unwrap();
        assert_eq!(
            Some(String::from("https://github.com/doctavious/snippext.git")),
            source.repository
        );
        assert_eq!(Some(String::from("main")), source.repository_ref);
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
            templates: None,
            repository_url: None,
            repository_ref: None,
            output_dir: None,
            output_extension: Some(String::from("txt")),
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
        assert_eq!(Some("txt".into()), settings.output_extension);
    }

    // https://users.rust-lang.org/t/whats-the-rust-way-to-unit-test-for-an-error/23677/2
    #[test]
    fn strings_must_not_be_empty() {
        let settings = SnippextSettings::new(
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
            Some(String::from("")),
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
    fn at_least_one_template_is_required() {
        let settings = SnippextSettings::new(
            String::from("snippet::start::"),
            String::from("snippet::end::"),
            HashMap::new(),
            vec![SnippetSource::new_local(vec![String::from("**")])],
            Some(String::from("./snippets/")),
            Some(String::from("md")),
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
            String::from("snippet::start::"),
            String::from("snippet::end::"),
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
            Some(String::from("md")),
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
            String::from("snippet::start::"),
            String::from("snippet::end::"),
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
            Some(String::from("md")),
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
    fn sources_must_not_be_empty() {
        let settings = SnippextSettings::new(
            String::from("snippet::start::"),
            String::from("snippet::end::"),
            HashMap::from([(
                "default".to_string(),
                SnippextTemplate {
                    content: String::from("{{snippet}}"),
                    default: true,
                },
            )]),
            vec![],
            Some(String::from("./snippets/")),
            Some(String::from("md")),
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
    fn local_sources_must_have_at_least_one_files_entry() {
        let settings = SnippextSettings::new(
            String::from("snippet::start::"),
            String::from("snippet::end::"),
            HashMap::from([(
                "default".to_string(),
                SnippextTemplate {
                    content: String::from("{{snippet}}"),
                    default: true,
                },
            )]),
            vec![SnippetSource::new_local(vec![])],
            Some(String::from("./snippets/")),
            Some(String::from("md")),
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
            String::from("snippet::start::"),
            String::from("snippet::end::"),
            HashMap::from([(
                "default".to_string(),
                SnippextTemplate {
                    content: String::from("{{snippet}}"),
                    default: true,
                },
            )]),
            vec![SnippetSource {
                repository: None,
                repository_ref: Some(String::from("branch")),
                cone_patterns: None,
                files: vec![String::from("**")],
                url: None,
            }],
            Some(String::from("./snippets/")),
            Some(String::from("md")),
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
                    String::from(
                        "sources[0] specifies ref, cone_patterns without specifying repository"
                    ),
                    failures.get(0).unwrap().to_string()
                );
            }
            _ => {
                panic!("invalid SnippextError");
            }
        }
    }

    #[test]
    fn should_successfully_extract_from_url() {
        let dir = tempdir().unwrap();
        let target = Path::new(&dir.path()).join("./target.md");
        fs::copy(
            Path::new("./tests/targets/target.md"),
            &target,
        ).unwrap();

        let settings = SnippextSettings::new(
            String::from("snippet::start::"),
            String::from("snippet::end::"),
            HashMap::from([(
                "default".to_string(),
                SnippextTemplate {
                    content: String::from("{{snippet}}"),
                    default: true,
                },
            )]),
            vec![SnippetSource::new_url("https://gist.githubusercontent.com/seancarroll/94629074d8cb36e9f5a0bc47b72ba6a5/raw/e87bd099a28b3a5c8112145e227ee176b3169439/snippext_example.rs".into())],
            None,
            None,
            Some(vec![target.to_string_lossy().to_string()]),
            None,
            None,
        );

        super::extract(settings).expect("Should extract from URL");

        // /var/folders/jm/1m24fjf96xv_458bclbpd1xh0000gn/T/snippext/https___gist.githubusercontent.com_seancarroll_94629074d8cb36e9f5a0bc47b72ba6a5_raw_e87bd099a28b3a5c8112145e227ee176b3169439_snippext_example.rs
        // https___gist.github.com_seancarroll_94629074d8cb36e9f5a0bc47b72ba6a5
        let actual =
            fs::read_to_string(target)
                .unwrap();

        let expected = r#"This is some static content

<!-- snippet::start::main -->
fn main() {
    println!("Hello, World!");
}
<!-- snippet::end::main -->

<!-- snippet::start::fn_1 -->
some content
<!-- snippet::end::fn_1 -->"#;
        assert_eq!(
            expected,
            actual
        );
    }
}
