use std::collections::HashMap;

use handlebars::{no_escape, Handlebars};
use serde::{Deserialize, Serialize};

use crate::constants::SNIPPEXT_TEMPLATE_ATTRIBUTE;
use crate::error::SnippextError;
use crate::settings::SnippextSettings;
use crate::types::{LinkFormat, Snippet};
use crate::unindent::unindent;
use crate::SnippextResult;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SnippextTemplate {
    pub content: String,
    pub default: bool,
}

impl SnippextTemplate {
    pub fn render_template(
        snippet: &Snippet,
        snippext_settings: &SnippextSettings,
        target_attributes: Option<HashMap<String, String>>,
    ) -> SnippextResult<String> {
        let mut data = HashMap::new();

        if let Some(target_attributes) = target_attributes {
            data.extend(target_attributes);
        }

        // TODO: do we want to make unindent optional?
        data.insert("snippet".to_string(), unindent(snippet.text.as_str()));
        // https://github.com/temporalio/snipsync/blob/fef6170acacc6dd351c4ab5784cccaafa80d93d5/src/Sync.js#L68
        // https://github.com/SimonCropp/MarkdownSnippets/blob/fae28ec759089641d3bf89a90211776de97d8899/src/MarkdownSnippets/Processing/SnippetMarkdownHandling.cs#L62
        // <a href='{url_prefix}{source_link}' title='Snippet source file'>snippet source</a>
        if let Some(link_format) = &snippext_settings.link_format {
            data.insert("source_links_enabled".to_string(), "true".to_string());

            let url_prefix = snippext_settings.url_prefix.to_owned().unwrap_or_default();
            data.insert("url_prefix".to_string(), url_prefix.clone());
            let source_link =
                SnippextTemplate::build_source_link(&snippet, link_format, url_prefix);

            // TODO: do we want to add a sup tag here or in the template?
            // I think template which means we should also move everything but the actual
            // href to the template
            data.insert(
                "source_link".to_string(),
                format!(
                    "<a href='{}' title='Snippet source file'>source</a>",
                    source_link
                ),
            );
        }

        for attribute in &snippet.attributes {
            data.insert(attribute.0.to_string(), attribute.1.to_string());
        }

        let template = get_template(data.get(SNIPPEXT_TEMPLATE_ATTRIBUTE), snippext_settings)?;
        return template.render(&data);
    }

    fn render(&self, data: &HashMap<String, String>) -> SnippextResult<String> {
        let mut hbs = Handlebars::new();
        hbs.register_escape_fn(no_escape);

        let rendered = hbs.render_template(self.content.as_str(), data)?;

        Ok(rendered)
    }

    fn build_source_link(
        snippet: &Snippet,
        link_format: &LinkFormat,
        url_prefix: String,
    ) -> String {
        let mut path = url_prefix;
        if !path.ends_with("/") {
            path.push_str("/")
        }

        path.push_str(snippet.path.to_str().unwrap_or_default());

        match link_format {
            LinkFormat::GitHub => format!("{}#L{}-L{}", path, snippet.start_line, snippet.end_line),
            LinkFormat::GitLab => format!("{}#L{}-{}", path, snippet.start_line, snippet.end_line),
            LinkFormat::BitBucket => {
                format!("{}#lines={}:{}", path, snippet.start_line, snippet.end_line)
            }
            LinkFormat::Gitea => format!("{}#L{}-L{}", path, snippet.start_line, snippet.end_line),
            LinkFormat::TFS => format!(
                "{}&line={}&lineEnd={}",
                path, snippet.start_line, snippet.end_line
            ),
        }
    }
}

/// find appropriate Snippext Template using the following rules
///
/// 1. template by id. None if not found
/// If id not provided
/// if only one template provided use it
/// if more than one template find the default one
fn get_template<'a>(
    id: Option<&String>,
    snippext_settings: &'a SnippextSettings,
) -> SnippextResult<&'a SnippextTemplate> {
    return if let Some(identifier) = id {
        if let Some(template) = snippext_settings.templates.get(identifier) {
            Ok(template)
        } else {
            Err(SnippextError::TemplateNotFound(String::from(format!(
                "{} does not exist",
                identifier
            ))))
        }
    } else {
        // could probably turn this into a match expression with match guards
        if snippext_settings.templates.len() == 1 {
            return Ok(snippext_settings.templates.values().next().unwrap());
        }

        if snippext_settings.templates.len() > 1 {
            let default_template = snippext_settings.templates.iter().find(|t| t.1.default);
            return if let Some(template) = default_template {
                Ok(template.1)
            } else {
                // we validate that we should always have one default template
                // so should never get here. Should we assert instead?
                Err(SnippextError::TemplateNotFound(String::from(
                    "No default template found",
                )))
            };
        }

        // we validate that we have at least one template so should never get here.
        // should we assert instead?
        Err(SnippextError::TemplateNotFound(String::from(
            "No templates found",
        )))
    };
}
