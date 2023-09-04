use std::collections::HashMap;

use handlebars::{no_escape, Handlebars};

use crate::constants::{DEFAULT_TEMPLATE_IDENTIFIER, SNIPPEXT_TEMPLATE_ATTRIBUTE};
use crate::error::SnippextError;
use crate::settings::SnippextSettings;
use crate::types::Snippet;
use crate::unindent::unindent;
use crate::SnippextResult;

pub fn render_template(
    snippet: &Snippet,
    snippext_settings: &SnippextSettings,
    target_attributes: Option<HashMap<String, String>>,
) -> SnippextResult<String> {
    let mut data = HashMap::new();
    for attribute in &snippet.attributes {
        data.insert(attribute.0.to_string(), attribute.1.to_string());
    }

    if let Some(target_attributes) = target_attributes {
        data.extend(target_attributes);
    }

    // TODO: do we want to make unindent optional?
    data.insert("snippet".to_string(), unindent(snippet.text.as_str()));
    data.insert(
        "source_path".to_string(),
        snippet.path.to_string_lossy().to_string(),
    );

    if let Some(source_link) = &snippet.source_link {
        if !data.contains_key("source_links_enabled") {
            data.insert("source_links_enabled".to_string(), "true".to_string());
        }

        data.insert(
            "source_link_prefix".to_string(),
            snippext_settings
                .source_link_prefix
                .to_owned()
                .unwrap_or_default(),
        );

        data.insert("source_link".to_string(), source_link.to_string());
    }

    let template = get_template(data.get(SNIPPEXT_TEMPLATE_ATTRIBUTE), snippext_settings)?;
    return render(template, &data);
}

fn render(content: &String, data: &HashMap<String, String>) -> SnippextResult<String> {
    let mut hbs = Handlebars::new();
    hbs.register_escape_fn(no_escape);

    let rendered = hbs.render_template(content.as_str(), data)?;

    Ok(rendered)
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
) -> SnippextResult<&'a String> {
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
        let default_template = snippext_settings
            .templates
            .iter()
            .find(|t| t.0 == DEFAULT_TEMPLATE_IDENTIFIER);
        return if let Some(template) = default_template {
            Ok(template.1)
        } else {
            // we validate that we should always have one default template
            // so should never get here. Should we assert instead?
            Err(SnippextError::TemplateNotFound(String::from(
                "No default template found",
            )))
        };
    };
}
