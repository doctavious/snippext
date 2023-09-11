use std::collections::HashMap;

use handlebars::{no_escape, Handlebars};
use serde_json::Value;

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
    let mut data: HashMap<String, Value> = HashMap::new();
    for attribute in &snippet.attributes {
        data.insert(attribute.0.to_string(), Value::String(attribute.1.to_string()));
    }

    if let Some(target_attributes) = target_attributes {
        data.extend(target_attributes
            .iter()
            .map(|e| (e.0.to_string(), Value::String(e.1.to_string())))
        );
    }

    // TODO: do we want to make unindent optional?
    data.insert("snippet".to_string(), Value::String(unindent(snippet.text.as_str())));
    data.insert(
        "source_path".to_string(),
        Value::String(snippet.path.to_string_lossy().to_string()),
    );

    if snippext_settings.omit_source_links {
        data.insert("omit_source_link".to_string(), Value::Bool(true));
    } else {
        // TODO: hate this
        let omit_source_link = if let Some(omit) = data.get("omit_source_link") {
            match omit {
                Value::Bool(b) => b.clone(),
                Value::String(s) => s.parse::<bool>().is_ok(),
                _ => false
            }
        } else {
            false
        };

        data.insert("omit_source_link".to_string(), Value::Bool(omit_source_link));
        data.insert(
            "source_link_prefix".to_string(),
            Value::String(snippext_settings
                .source_link_prefix
                .to_owned()
                .unwrap_or_default()
            ),
        );

        if let Some(source_link) = &snippet.source_link {
            data.insert("source_link".to_string(), Value::String(source_link.to_string()));
        }
    }

    let template = get_template(&data, snippext_settings)?;
    return render(template, &data);
}

fn render(content: &String, data: &HashMap<String, Value>) -> SnippextResult<String> {
    let mut hbs = Handlebars::new();
    hbs.register_escape_fn(no_escape);

    let rendered = hbs.render_template(content.as_str(), data)?;

    Ok(rendered)
}

fn get_template<'a>(
    data: &HashMap<String, Value>,
    snippext_settings: &'a SnippextSettings,
) -> SnippextResult<&'a String> {
    return if let Some(template_identifier) = data.get(SNIPPEXT_TEMPLATE_ATTRIBUTE) {
        match template_identifier {
            Value::String(identifier) => {
                if let Some(template) = snippext_settings.templates.get(identifier) {
                    Ok(template)
                } else {
                    Err(SnippextError::TemplateNotFound(String::from(format!(
                        "{} does not exist",
                        identifier
                    ))))
                }
            }
            _ => {
                Err(SnippextError::TemplateNotFound(String::from(format!(
                    "{} has wrong type",
                    template_identifier.to_string()
                ))))
            }
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
