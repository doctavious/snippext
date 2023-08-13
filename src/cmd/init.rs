use std::collections::HashMap;
use std::fs;

use clap::Parser;
use inquire::{required, Confirm, Select, Text, Editor};
use serde::{Deserialize, Serialize};

use crate::constants::{
    DEFAULT_BEGIN, DEFAULT_END, DEFAULT_FILE_EXTENSION,
    DEFAULT_SNIPPEXT_CONFIG, DEFAULT_SOURCE_FILES, DEFAULT_TEMPLATE,
};
use crate::templates::SnippextTemplate;
use crate::types::{LinkFormat, SnippetSource};
use crate::{SnippextResult, SnippextSettings};

#[derive(Clone, Debug, Parser)]
#[command()]
pub struct Args {
    #[arg(long, help = "TODO: ...")]
    pub default: bool,
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

pub fn execute(init_opt: Args) -> SnippextResult<()> {
    let init_settings = if init_opt.default {
        None
    } else {
        Some(init_settings_from_prompt()?)
    };

    init(init_settings)
}

fn init_settings_from_prompt() -> SnippextResult<SnippextSettings> {
    // TODO: look at render config options

    // begin: "snippet::start::"
    // end: "snippet::end::"
    // extension: "md"
    // comment_prefixes:
    // - "// "
    //     - "# "
    //     - "<!-- "
    // templates:
    //     default:
    //     template: "{{snippet}}{{#if source_links_enabled}}\n{{source_link}}{{/if}}"
    // default: true
    // default_with_links:
    //     template: "{{snippet}}"
    // default: false
    // code:
    //     template: "```{{lang}}\n{{snippet}}```\n"
    // default: false
    // code_with_source_links:
    //     template: "```{{lang}}\n{{snippet}}```\n<a href='{url_prefix}{source_link}' title='Snippet source file'>snippet source</a>\n"
    // default: false
    // sources:
    // # extract from local files
    //     - files:
    // - "**"
    // output_dir: "./generated-snippets/"

    let begin = Text::new("Begin tag:")
        .with_default(DEFAULT_BEGIN)
        .with_help_message("")
        .prompt()?;

    let end = Text::new("End tag:")
        .with_default(DEFAULT_END)
        .with_help_message("")
        .prompt()?;

    let output_extension = Text::new("Extension:")
        .with_default(DEFAULT_FILE_EXTENSION)
        .with_help_message("File extension for generated snippet")
        .prompt()?;

    // TODO: support multiple templates (id / default / template)
    let mut templates: HashMap<String, SnippextTemplate> = HashMap::new();
    loop {
        let identifier = Text::new("Template identifier:")
            .with_validator(required!("This field is required"))
            .with_help_message("")
            .prompt()?;

        let template = Editor::new("Template content:")
            .with_predefined_text(DEFAULT_TEMPLATE)
            .with_help_message("")
            .prompt()?;

        // mark default? can we be smart of if already has a default then no need to ask.
        // if only one template then just mark that as default

        templates.insert(
            identifier,
            SnippextTemplate {
                content: template,
                default: false,
            },
        );

        let add_another_template = Confirm::new("Add another template?")
            .with_default(false)
            .with_help_message("")
            .prompt()?;

        if !add_another_template {
            break;
        }
    }

    // TODO: add if user wants to use default template?

    let mut sources: Vec<SnippetSource> = Vec::new();
    loop {
        let source_type = Select::new("Type of source?", vec!["local", "remote"]).prompt()?;

        if source_type.eq("local") {
            // TODO: loop or comma separated globs?
            let source_files = Text::new("Source files:")
                .with_default(DEFAULT_SOURCE_FILES)
                .with_help_message("Globs")
                .prompt()?;

            // sources.insert(SnippetSource::new_local())
        } else {
            // repository
            // branch
            // commit
            // directory
            // files

            // sources.insert(SnippetSource::new_remote())
        }

        // url

        let add_another_source = Confirm::new("Add another source?")
            .with_default(false)
            .with_help_message("")
            .prompt()?;

        if !add_another_source {
            break;
        }
    }

    let output_directory_prompt = Text::new("Output directory:")
        .with_help_message("Output directory to write generated snippets to.")
        .prompt()?;

    let output_dir = if output_directory_prompt.is_empty() {
        Some(output_directory_prompt)
    } else {
        None
    };

    let targets = Text::new("targets:")
        .with_default("**")
        .with_help_message(
            "Glob patterns that specify files/directories to be spliced with extracted snippets"
        )
        .prompt()?;

    let link_format = Select::new("Source link format", LinkFormat::VARIANTS.to_vec())
        .with_help_message(
            "Press escape to skip selection"
        )
        .prompt_skippable()?;

    let url_prefix = Text::new("URL Prefix")
        .with_help_message(
            ""
        )
        .prompt_skippable()?;

    Ok(SnippextSettings {
        begin,
        end,
        output_extension,
        templates,
        sources,
        output_dir,
        targets: Some(targets.split(",").map(|t| t.to_string()).collect()),
        link_format,
        url_prefix,
    })
}
