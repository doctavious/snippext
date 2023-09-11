use std::fs;

use clap::{Parser, ValueEnum};
use indexmap::IndexMap;
use inquire::validator::{ErrorMessage, StringValidator, Validation};
use inquire::{Confirm, CustomUserError, Editor, Select, Text};
use tracing::warn;

use crate::constants::{
    DEFAULT_END, DEFAULT_GIT_BRANCH, DEFAULT_OUTPUT_FILE_EXTENSION, DEFAULT_SNIPPEXT_CONFIG,
    DEFAULT_SOURCE_FILES, DEFAULT_START, DEFAULT_TEMPLATE, DEFAULT_TEMPLATE_IDENTIFIER,
};
use crate::error::SnippextError;
use crate::types::{LinkFormat, MissingSnippetsBehavior, SnippetSource};
use crate::{SnippextResult, SnippextSettings};

/// Initialize a Snippext configuration file which contains options for extracting snippets
/// from target files as well as options for splicing snippets into target files.
#[derive(Clone, Debug, Parser)]
#[command()]
pub struct Args {
    /// Initializes Snippext config using the default Snippext configuration
    #[arg(long)]
    pub default: bool,
}

pub fn execute(init_opt: Args) -> SnippextResult<()> {
    let content = if init_opt.default {
        DEFAULT_SNIPPEXT_CONFIG.to_string()
    } else {
        serde_yaml::to_string(&init_settings_from_prompt()?)?
    };

    fs::write("./snippext.yaml", content)?;
    Ok(())
}

fn init_settings_from_prompt() -> SnippextResult<SnippextSettings> {
    let start = Text::new("Start prefix:")
        .with_default(DEFAULT_START)
        .with_validator(NotEmptyValidator::default())
        .with_help_message("Prefix that marks the start of a snippet")
        .prompt()?;

    let end = Text::new("End prefix:")
        .with_default(DEFAULT_END)
        .with_validator(NotEmptyValidator::default())
        .with_help_message("Prefix that marks the ending of a snippet")
        .prompt()?;

    let default_config: SnippextSettings = serde_yaml::from_str(DEFAULT_SNIPPEXT_CONFIG)
        .expect("Should be able to deserialize default snippext config");

    let mut use_default_templates_message =
        String::from("Default templates include the following:\n\n");
    for (key, value) in &default_config.templates {
        use_default_templates_message.push_str(format!("{}:\n{}\n\n", key, value).as_str());
    }

    let use_default_templates = Confirm::new("Use default templates?")
        .with_default(true)
        .with_help_message(use_default_templates_message.as_str())
        .prompt()?;

    let mut templates: IndexMap<String, String> = IndexMap::new();
    if use_default_templates {
        templates.extend(default_config.templates);
    } else {
        loop {
            let (identifier, template) = if templates.is_empty() {
                let content = Editor::new("Default template content:")
                    .with_predefined_text(DEFAULT_TEMPLATE)
                    // .with_help_message("")
                    .prompt()?;
                (DEFAULT_TEMPLATE_IDENTIFIER.to_string(), content)
            } else {
                let identifier = Text::new("Template identifier:")
                    .with_validator(NotEmptyValidator::default())
                    .with_help_message(
                        "Identifier used to determine which template to use \
                        when rendering snippet in target files",
                    )
                    .prompt()?;

                let content = Editor::new("Template content:").prompt()?;

                (identifier, content)
            };

            templates.insert(identifier, template);

            let add_another_template = Confirm::new("Add another template?")
                .with_default(false)
                .prompt()?;

            if !add_another_template {
                break;
            }
        }
    }

    let mut sources: Vec<SnippetSource> = Vec::new();
    loop {
        // TODO: get variations from SnippetSource
        let source_type = Select::new("Type of source?", vec!["local", "git", "url"]).prompt()?;

        match source_type {
            "git" => {
                let repo = Text::new("Repository:")
                    .with_validator(NotEmptyValidator::default())
                    .with_help_message("The repository to clone from")
                    .prompt()?;

                let repository_branch = Text::new("Branch:")
                    .with_default(DEFAULT_GIT_BRANCH)
                    .with_help_message("Branch name to use during git clone")
                    .prompt()?;

                let cone_patterns_prompt = Text::new("Cone Patterns:")
                    .with_help_message(
                        "A list of directories, space separated, to be \
                        included in the sparse checkout",
                    )
                    .prompt_skippable()?;

                let cone_patterns = if let Some(cone_patterns) = cone_patterns_prompt {
                    Some(cone_patterns.split(" ").map(|s| s.to_string()).collect())
                } else {
                    None
                };

                let source_files_prompt = Text::new("Source files:")
                    .with_default(DEFAULT_SOURCE_FILES)
                    .with_help_message(
                        "List of glob patterns, separated by space, to look \
                        for snippets. Not applicable for URL sources.",
                    )
                    .prompt()?;

                let source_files = source_files_prompt
                    .split(' ')
                    .filter(|x| !x.is_empty())
                    .map(|s| s.to_string())
                    .collect();

                sources.push(SnippetSource::Git {
                    repository: repo,
                    branch: Some(repository_branch),
                    cone_patterns,
                    files: source_files,
                });
            }
            "local" => {
                let source_files_prompt = Text::new("Source files:")
                    .with_default(DEFAULT_SOURCE_FILES)
                    .with_help_message(
                        "List of glob patterns, separated by space, to look \
                        for snippets. Not applicable for URL sources.",
                    )
                    .prompt()?;

                let source_files = source_files_prompt
                    .split(' ')
                    .filter(|x| !x.is_empty())
                    .map(|s| s.to_string())
                    .collect();

                sources.push(SnippetSource::Local {
                    files: source_files,
                });
            }
            "url" => {
                let url = Text::new("URL:")
                    .with_validator(NotEmptyValidator::default())
                    .with_help_message("URL to content that should be included as snippets")
                    .prompt()?;
                sources.push(SnippetSource::Url(url));
            }
            _ => {
                warn!("Invalid source type {}", source_type);
            }
        }

        let add_another_source = Confirm::new("Add another source?")
            .with_default(false)
            .prompt()?;

        if !add_another_source {
            break;
        }
    }

    let output_directory_prompt = Text::new("Output directory:")
        .with_help_message("Output directory to write generated snippets to.")
        .prompt()?
        .trim()
        .to_string();

    let output_dir = if !output_directory_prompt.is_empty() {
        Some(output_directory_prompt)
    } else {
        None
    };

    let output_extension = if output_dir.is_some() {
        Some(
            Text::new("Output Extension:")
                .with_default(DEFAULT_OUTPUT_FILE_EXTENSION)
                .with_help_message("File extension for generated snippets")
                .prompt()?,
        )
    } else {
        None
    };

    let targets = Text::new("targets:")
        .with_default("**")
        .with_help_message(
            "Glob patterns that specify files/directories to be spliced with extracted snippets",
        )
        .prompt()?;

    let link_format = Select::new("Source link format", LinkFormat::VARIANTS.to_vec())
        .with_help_message("Press escape to skip selection")
        .prompt_skippable()?;

    let source_link_prefix_prompt = Text::new("Source Link Prefix")
        .with_help_message("String that will prefix all local snippet source links. This is useful \
                when markdown files are hosted on a site that is not co-located with the source code files.")
        .prompt_skippable()?;

    let source_link_prefix = if let Some(source_link_prefix_prompt) = source_link_prefix_prompt {
        let source_link_prefix = source_link_prefix_prompt.trim();
        if source_link_prefix.is_empty() {
            None
        } else {
            Some(source_link_prefix.to_string())
        }
    } else {
        None
    };

    let missing_snippets_behavior = Select::new(
        "Missing Snippet Behavior?",
        MissingSnippetsBehavior::value_variants().to_vec(),
    )
    .prompt()?;

    Ok(SnippextSettings {
        start,
        end,
        output_extension,
        templates,
        sources,
        output_dir,
        targets: Some(targets.split(",").map(|t| t.to_string()).collect()),
        link_format,
        source_link_prefix,
        omit_source_links: false,
        missing_snippets_behavior: MissingSnippetsBehavior::from_str(
            &missing_snippets_behavior.to_string(),
            true,
        )
        .map_err(SnippextError::GeneralError)?,
    })
}

#[derive(Clone, Default)]
struct NotEmptyValidator {}

/// Similar to ValueRequiredValidator but trims strings
impl StringValidator for NotEmptyValidator {
    fn validate(&self, input: &str) -> Result<Validation, CustomUserError> {
        Ok(if input.trim().is_empty() {
            Validation::Invalid(ErrorMessage::Custom("Response cannot be empty".to_string()))
        } else {
            Validation::Valid
        })
    }
}
