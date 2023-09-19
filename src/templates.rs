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
    target_attributes: Option<HashMap<String, Value>>,
) -> SnippextResult<String> {
    let mut data: HashMap<String, Value> = HashMap::new();
    data.insert(
        "omit_source_link".to_string(),
        Value::Bool(snippext_settings.omit_source_links),
    );

    data.extend(snippet.attributes.clone());
    if let Some(target_attributes) = target_attributes {
        data.extend(target_attributes);
    }

    let snippet_content = if let Some(selected_lines) = data.get("selected_lines") {
        let selected_numbers = selected_lines
            .as_array()
            .ok_or(SnippextError::GeneralError(
                "selected_lines must be an array".to_string(),
            ))?;

        let snippet_content_lines: Vec<&str> = snippet.text.as_str().lines().collect();
        let mut new_lines = Vec::new();
        for selected_number in selected_numbers {
            let sn = selected_number.as_str().ok_or(SnippextError::GeneralError(
                "select_lines values must be strings".to_string(),
            ))?;

            let include_lines = if sn.contains("-") {
                let line_nums: Vec<&str> = sn.split("-").collect();
                [
                    line_nums[0].parse::<usize>()? - 1,
                    line_nums[1].parse::<usize>()?,
                ]
            } else {
                let num = sn.trim().parse::<usize>()?;
                [num - 1, num]
            };

            new_lines.extend_from_slice(&snippet_content_lines[include_lines[0]..include_lines[1]]);
        }

        new_lines.iter().fold(String::new(), |mut a, b| {
            a.push_str(b);
            a.push_str("\n");
            a
        })
    } else {
        snippet.text.clone()
    };

    // TODO: do we want to make unindent optional?
    data.insert(
        "snippet".to_string(),
        Value::String(unindent(snippet_content.as_str())),
    );
    data.insert(
        "source_path".to_string(),
        Value::String(snippet.path.to_string_lossy().to_string()),
    );

    data.insert(
        "source_link_prefix".to_string(),
        Value::String(
            snippext_settings
                .source_link_prefix
                .to_owned()
                .unwrap_or_default(),
        ),
    );

    if let Some(source_link) = &snippet.source_link {
        data.insert(
            "source_link".to_string(),
            Value::String(source_link.to_string()),
        );
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
            _ => Err(SnippextError::TemplateNotFound(String::from(format!(
                "{} has wrong type",
                template_identifier.to_string()
            )))),
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
