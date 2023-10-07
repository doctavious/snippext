use std::collections::HashMap;

use handlebars::{no_escape, Handlebars};
use indexmap::IndexSet;
use serde_json::Value;

use crate::constants::{DEFAULT_TEMPLATE_IDENTIFIER, SNIPPEXT_TEMPLATE_ATTRIBUTE};
use crate::error::SnippextError;
use crate::settings::SnippextSettings;
use crate::types::Snippet;
use crate::unindent::unindent;
use crate::{files, unindent, SnippextResult};

pub(crate) fn render_template(
    identifier: Option<&String>,
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
        let mut sns = IndexSet::new();
        for selected_number in selected_numbers {
            let sn = selected_number.as_str().ok_or(SnippextError::GeneralError(
                "select_lines values must be strings".to_string(),
            ))?;

            let include_lines = if sn.contains('-') {
                let line_nums: Vec<&str> = sn.split('-').collect();
                [
                    line_nums[0].parse::<usize>()? - 1,
                    line_nums[1].parse::<usize>()?,
                ]
            } else {
                let num = sn.trim().parse::<usize>()?;
                [num - 1, num]
            };

            sns.insert(include_lines);
        }

        let selected_lines_include_ellipses = data
            .get("selected_lines_include_ellipses")
            .and_then(|v| v.as_bool())
            .unwrap_or(snippext_settings.selected_lines_include_ellipses);

        if selected_lines_include_ellipses {
            let extension = &files::extension_from_path(&snippet.path);
            let ellipsis = if files::is_text_file(extension.as_str()) {
                String::from("...")
            } else {
                files::file_comments(extension)[0].0.to_owned() + " ..."
            };

            // if line 1 (index 0) isn't selected add ellipsis to start
            if sns[0][0] != 0 {
                new_lines.push(ellipsis.clone())
            }

            for (idx, sn) in sns.iter().enumerate() {
                // if there is a gap between selected lines or ranges then add an ellipsis
                let content_lines = &snippet_content_lines[sn[0]..sn[1]];
                // we subtract 1 from the end of the previous range because its an exclusive range.
                if idx > 0 && sn[0] - (sns[idx - 1][1] - 1) > 1 {
                    let spaces = unindent::count_spaces_string(content_lines[0]).unwrap_or(0);
                    let ellipsis_comment = format!("{}{}", " ".repeat(spaces), ellipsis);
                    new_lines.push(ellipsis_comment);
                }
                let a: Vec<String> = content_lines.iter().map(|l| l.to_string()).collect();
                new_lines.extend(a);
            }

            // if we didnt highlight the last line add an ellipsis at the end.
            if sns
                .last()
                .is_some_and(|sn| sn[1] != snippet_content_lines.len())
            {
                new_lines.push(ellipsis)
            }
        } else {
            for sn in sns {
                let a: Vec<String> = snippet_content_lines[sn[0]..sn[1]]
                    .iter()
                    .map(|l| l.to_string())
                    .collect();
                new_lines.extend(a);
            }
        }

        new_lines.iter().fold(String::new(), |mut a, b| {
            a.push_str(b);
            a.push('\n');
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

    let template = get_template(identifier, &data, snippext_settings)?;
    render(template, &data)
}

fn render(content: &str, data: &HashMap<String, Value>) -> SnippextResult<String> {
    let mut hbs = Handlebars::new();
    hbs.register_escape_fn(no_escape);

    let rendered = hbs.render_template(content, data)?;

    Ok(rendered)
}

// TODO: clean up
fn get_template<'a>(
    identifier: Option<&String>,
    data: &HashMap<String, Value>,
    snippext_settings: &'a SnippextSettings,
) -> SnippextResult<&'a String> {
    if let Some(identifier) = identifier {
        return if let Some(template) = snippext_settings.templates.get(identifier) {
            Ok(template)
        } else {
            Err(SnippextError::TemplateNotFound(format!(
                "{} does not exist",
                identifier
            )))
        };
    }
    return if let Some(template_identifier) = data.get(SNIPPEXT_TEMPLATE_ATTRIBUTE) {
        match template_identifier {
            Value::String(identifier) => {
                if let Some(template) = snippext_settings.templates.get(identifier) {
                    Ok(template)
                } else {
                    Err(SnippextError::TemplateNotFound(format!(
                        "{} does not exist",
                        identifier
                    )))
                }
            }
            _ => Err(SnippextError::TemplateNotFound(format!(
                "{} has wrong type",
                template_identifier.to_string()
            ))),
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
