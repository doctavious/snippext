use std::collections::HashMap;
use std::str::FromStr;

use handlebars::{no_escape, Handlebars};
use serde::{Deserialize, Serialize};
use url::Url;

use crate::constants::{
    DEFAULT_GIT_BRANCH, DEFAULT_TEMPLATE_IDENTIFIER, SNIPPEXT_TEMPLATE_ATTRIBUTE,
};
use crate::error::SnippextError;
use crate::settings::SnippextSettings;
use crate::types::{LinkFormat, Snippet, SnippetSource};
use crate::unindent::unindent;
use crate::{git, SnippextResult};

pub fn render_template(
    snippet: &Snippet,
    source: &SnippetSource,
    snippext_settings: &SnippextSettings,
    target_attributes: Option<HashMap<String, String>>,
) -> SnippextResult<String> {
    let mut data = HashMap::new();

    if let Some(target_attributes) = target_attributes {
        data.extend(target_attributes);
    }

    // TODO: do we want to make unindent optional?
    data.insert("snippet".to_string(), unindent(snippet.text.as_str()));
    data.insert(
        "source_path".to_string(),
        snippet.path.to_string_lossy().to_string(),
    );

    let source_link = build_source_link(
        snippet,
        source,
        snippext_settings.link_format,
        snippext_settings.source_link_prefix.as_ref(),
    );
    if let Some(source_link) = source_link {
        data.insert("source_links_enabled".to_string(), "true".to_string());
        data.insert(
            "source_link_prefix".to_string(),
            snippext_settings
                .source_link_prefix
                .to_owned()
                .unwrap_or_default(),
        );

        data.insert("source_link".to_string(), source_link);
    }

    for attribute in &snippet.attributes {
        data.insert(attribute.0.to_string(), attribute.1.to_string());
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

fn build_source_link(
    snippet: &Snippet,
    source: &SnippetSource,
    link_format: Option<LinkFormat>,
    source_link_prefix: Option<&String>,
) -> Option<String> {
    match source {
        SnippetSource::Local { .. } => {
            let link_format = link_format?;
            let mut path = String::new();
            if let Some(source_link_prefix) = source_link_prefix {
                path.push_str(source_link_prefix);
                if !path.ends_with("/") {
                    path.push('/');
                }
            }
            path.push_str(snippet.path.to_str().unwrap_or_default());
            Some(link_format.source_link(&path, &snippet))
        }
        SnippetSource::Git {
            repository,
            branch,
            ..
        } => {
            let url = Url::from_str(repository).expect("Git repository must be a valid URL");
            let link_format = link_format.or_else(|| {
                let domain = url.domain()?;
                LinkFormat::from_domain(domain)
            })?;

            let mut path = url.to_string().strip_suffix(".git")?.to_string();
            // TODO: would like to hoist this logic up as there is no reason this needs to run for
            // every file. We should determine it once and use it for ever snippet we generate
            let branch = if let Some(branch) = branch {
                branch.clone()
            } else {
                git::abbrev_ref(Some(&snippet.path.clone()))
                    .unwrap_or(DEFAULT_GIT_BRANCH.to_string())
            };
            path.push_str(
                format!(
                    "{}{}/{}",
                    link_format.blob_path_segment(),
                    branch,
                    &snippet.path.to_str().unwrap_or_default()
                )
                .as_str(),
            );

            Some(link_format.source_link(&path, &snippet))
        }
        SnippetSource::Url(url) => Some(url.to_string()),
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

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::path::PathBuf;

    use crate::templates::build_source_link;
    use crate::types::{LinkFormat, Snippet, SnippetSource};

    #[test]
    fn local_source_link_without_prefix() {
        let source = SnippetSource::Local {
            files: vec!["**".to_string()],
        };

        let snippet = Snippet {
            identifier: "example".to_string(),
            path: PathBuf::from("src/main.rs"),
            text: "{{snippet}}".to_string(),
            attributes: HashMap::new(),
            start_line: 1,
            end_line: 10,
        };

        let source_link = build_source_link(&snippet, &source, Some(LinkFormat::GitHub), None)
            .expect("Should build source link");

        assert_eq!("src/main.rs#L1-L10", source_link);
    }

    #[test]
    fn local_source_link_with_prefix() {
        let source = SnippetSource::Local {
            files: vec!["**".to_string()],
        };

        let snippet = Snippet {
            identifier: "example".to_string(),
            path: PathBuf::from("src/main.rs"),
            text: "{{snippet}}".to_string(),
            attributes: HashMap::new(),
            start_line: 1,
            end_line: 10,
        };

        let source_link = build_source_link(
            &snippet,
            &source,
            Some(LinkFormat::GitHub),
            Some(&"https://github.com/doctavious/snippext/blob/main/".to_string()),
        )
        .expect("Should build source link");

        assert_eq!(
            "https://github.com/doctavious/snippext/blob/main/src/main.rs#L1-L10",
            source_link
        );
    }

    #[test]
    fn local_source_without_link_format_should_not_build_source_link() {
        let source = SnippetSource::Local {
            files: vec!["**".to_string()],
        };

        let snippet = Snippet {
            identifier: "example".to_string(),
            path: PathBuf::from("src/main.rs"),
            text: "{{snippet}}".to_string(),
            attributes: HashMap::new(),
            start_line: 1,
            end_line: 10,
        };

        let source_link = build_source_link(&snippet, &source, None, None);

        assert!(source_link.is_none());
    }

    #[test]
    fn git_source_link() {
        let source = SnippetSource::Git {
            repository: "https://github.com/doctavious/snippext.git".to_string(),
            branch: Some("main".to_string()),
            cone_patterns: None,
            files: vec!["**".to_string()],
        };

        let snippet = Snippet {
            identifier: "example".to_string(),
            path: PathBuf::from("src/main.rs"),
            text: "{{snippet}}".to_string(),
            attributes: HashMap::new(),
            start_line: 1,
            end_line: 10,
        };

        let source_link = build_source_link(&snippet, &source, None, None)
            .expect("source link should be present");

        assert_eq!(
            "https://github.com/doctavious/snippext/blob/main/src/main.rs#L1-L10",
            source_link
        );
    }

    #[test]
    fn url_source_link() {
        let source = SnippetSource::Url (
            "https://gist.githubusercontent.com/seancarroll/94629074d8cb36e9f5a0bc47b72ba6a5/raw/e87bd099a28b3a5c8112145e227ee176b3169439/snippext_example.rs".into()
        );

        let snippet = Snippet {
            identifier: "example".to_string(),
            path: PathBuf::from("https://gist.githubusercontent.com/seancarroll/94629074d8cb36e9f5a0bc47b72ba6a5/raw/e87bd099a28b3a5c8112145e227ee176b3169439/snippext_example.rs"),
            text: "{{snippet}}".to_string(),
            attributes: HashMap::new(),
            start_line: 1,
            end_line: 10,
        };

        let source_link = build_source_link(&snippet, &source, None, Some(&String::new()))
            .expect("Should build source link");

        assert_eq!(
            "https://gist.githubusercontent.com/seancarroll/94629074d8cb36e9f5a0bc47b72ba6a5/raw/e87bd099a28b3a5c8112145e227ee176b3169439/snippext_example.rs",
            source_link
        );
    }
}
