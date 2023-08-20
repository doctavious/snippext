use std::collections::HashMap;
use std::str::FromStr;

use handlebars::{no_escape, Handlebars};
use serde::{Deserialize, Serialize};
use url::Url;

use crate::constants::SNIPPEXT_TEMPLATE_ATTRIBUTE;
use crate::error::SnippextError;
use crate::settings::SnippextSettings;
use crate::types::{LinkFormat, Snippet, SnippetSource};
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
        data.insert("source_path".to_string(), snippet.path.to_string_lossy().to_string());

        let source_link = build_source_link(
            snippet,
            source,
            snippext_settings.link_format,
            snippext_settings.url_prefix.as_ref()
        );
        if let Some(source_link) = source_link {
            data.insert("source_links_enabled".to_string(), "true".to_string());
            data.insert("url_prefix".to_string(), snippext_settings.url_prefix.to_owned().unwrap_or_default());

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
}

fn build_source_link(
    snippet: &Snippet,
    source: &SnippetSource,
    link_format: Option<LinkFormat>,
    url_prefix: Option<&String>
) -> Option<String> {
    match source {
        SnippetSource::Local { .. } => {
            let link_format = link_format?;
            let mut path = String::new();
            if let Some(url_prefix) = url_prefix {
                path.push_str(url_prefix);
                if !path.ends_with("/") {
                    path.push('/');
                }
            }
            path.push_str(snippet.path.to_str().unwrap_or_default());
            Some(link_format.source_link(&path,  &snippet))
        }
        SnippetSource::Git { repository, reference, .. } => {
            let url = Url::from_str(repository).expect("Git repository must be a valid URL");
            let link_format = link_format.or_else(|| {
                let domain = url.domain()?;
                LinkFormat::from_domain(domain)
            })?;

            let mut path = url.to_string().strip_suffix(".git")?.to_string();
            path.push_str(
                format!("{}{}/{}",
                    link_format.blob_path_segment(),
                    reference.as_deref().unwrap_or("main"),
                    &snippet.path.to_str().unwrap_or_default()
                ).as_str()
            );
            // path.push_str(&snippet.path.to_str().unwrap_or_default());

            Some(link_format.source_link(
                &path,
                &snippet
            ))
        }
        SnippetSource::Url(url) => {
            Some(url.to_string())
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

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::path::PathBuf;
    use crate::templates::build_source_link;
    use crate::types::{LinkFormat, Snippet, SnippetSource};

    #[test]
    fn local_source_link() {
        let source = SnippetSource::Local {
            files: vec!["**".to_string()]
        };

        let snippet = Snippet::new(
            "example".to_string(),
            PathBuf::from("src/main.rs"),
            "{{snippet}}".to_string(),
            HashMap::new(),
            0,
            10,
        );

        let source_link = build_source_link(
            &snippet,
            &source,
            Some(LinkFormat::GitHub),
            None,
        ).expect("Should build source link");

        assert_eq!("src/main.rs#L0-L10", source_link);
    }

    #[test]
    fn local_source_link_with_prefix() {
        let source = SnippetSource::Local {
            files: vec!["**".to_string()]
        };

        let snippet = Snippet::new(
            "example".to_string(),
            PathBuf::from("src/main.rs"),
            "{{snippet}}".to_string(),
            HashMap::new(),
            1,
            10,
        );

        let source_link = build_source_link(
            &snippet,
            &source,
            Some(LinkFormat::GitHub),
            Some(&"https://github.com/doctavious/snippext/blob/main/".to_string())
        ).expect("Should build source link");

        assert_eq!("https://github.com/doctavious/snippext/blob/main/src/main.rs#L1-L10", source_link);
    }

    #[test]
    fn local_source_without_link_format_should_not_build_source_link() {
        let source = SnippetSource::Local {
            files: vec!["**".to_string()]
        };

        let snippet = Snippet::new(
            "example".to_string(),
            PathBuf::from("src/main.rs"),
            "{{snippet}}".to_string(),
            HashMap::new(),
            1,
            10,
        );

        let source_link = build_source_link(
            &snippet,
            &source,
            None,
            None,
        );

        assert!(source_link.is_none());
    }

    #[test]
    fn git_source_link() {
        let source = SnippetSource::Git {
            repository: "https://github.com/doctavious/snippext.git".to_string(),
            reference: Some("main".to_string()),
            cone_patterns: None,
            files: vec!["**".to_string()]
        };

        let snippet = Snippet::new(
            "example".to_string(),
            PathBuf::from("src/main.rs"),
            "{{snippet}}".to_string(),
            HashMap::new(),
            1,
            10,
        );

        let source_link = build_source_link(
            &snippet,
            &source,
            None,
            None,
        ).expect("source link should be present");

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

        let snippet = Snippet::new(
            "example".to_string(),
            PathBuf::from("https://gist.githubusercontent.com/seancarroll/94629074d8cb36e9f5a0bc47b72ba6a5/raw/e87bd099a28b3a5c8112145e227ee176b3169439/snippext_example.rs"),
            "{{snippet}}".to_string(),
            HashMap::new(),
            0,
            10,
        );

        let source_link = build_source_link(
            &snippet,
            &source,
            None,
            Some(&String::new())
        ).expect("Should build source link");

        assert_eq!(
            "https://gist.githubusercontent.com/seancarroll/94629074d8cb36e9f5a0bc47b72ba6a5/raw/e87bd099a28b3a5c8112145e227ee176b3169439/snippext_example.rs",
            source_link
        );
    }
}
