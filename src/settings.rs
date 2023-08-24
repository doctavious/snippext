use std::collections::HashMap;
use std::fs;
use std::path::Path;
use indexmap::IndexMap;

use serde::{Deserialize, Serialize};

use crate::constants::{DEFAULT_BEGIN, DEFAULT_END, DEFAULT_OUTPUT_DIR, DEFAULT_OUTPUT_FILE_EXTENSION, DEFAULT_SOURCE_FILES, DEFAULT_TEMPLATE, DEFAULT_TEMPLATE_IDENTIFIER};
use crate::types::{LinkFormat, SnippetSource};
use crate::SnippextResult;

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct SnippextSettings {
    pub begin: String,
    pub end: String,
    pub templates: IndexMap<String, String>,
    pub sources: Vec<SnippetSource>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_dir: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_extension: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub targets: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub link_format: Option<LinkFormat>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_link_prefix: Option<String>,
}

impl SnippextSettings {
    /// Create default SnippextSettings which will have the following
    /// begin: [`DEFAULT_BEGIN`]
    /// end: [`DEFAULT_END`]
    /// extension: [`DEFAULT_FILE_EXTENSION`]
    /// template: [`DEFAULT_TEMPLATE`]
    /// sources: all files via [`DEFAULT_SOURCE_FILES`] glob
    /// output_dir: [`DEFAULT_OUTPUT_DIR`]
    pub fn default() -> Self {
        Self {
            begin: String::from(DEFAULT_BEGIN),
            end: String::from(DEFAULT_END),
            templates: IndexMap::from([(
                String::from(DEFAULT_TEMPLATE_IDENTIFIER),
                DEFAULT_TEMPLATE.to_string(),
            )]),
            sources: vec![SnippetSource::Local {
                files: vec![String::from(DEFAULT_SOURCE_FILES)],
            }],
            output_dir: Some(String::from(DEFAULT_OUTPUT_DIR)),
            output_extension: Some(String::from(DEFAULT_OUTPUT_FILE_EXTENSION)),
            targets: None,
            link_format: None,
            source_link_prefix: None,
        }
    }

    /// Create SnippextSettings from config file
    ///
    /// # Arguments
    ///
    /// * `path` - Path of config file
    pub fn from_config<S: AsRef<Path>>(path: S) -> SnippextResult<Self> {
        let content = fs::read_to_string(path)?;
        let settings = serde_json::from_str(content.as_str())?;
        Ok(settings)
    }

    // TODO: <S: Into<String>>
    pub fn new(
        begin: String,
        end: String,
        templates: IndexMap<String, String>,
        sources: Vec<SnippetSource>,
        output_dir: Option<String>,
        output_extension: Option<String>,
        targets: Option<Vec<String>>,
        link_format: Option<LinkFormat>,
        source_link_prefix: Option<String>,
    ) -> Self {
        Self {
            begin,
            end,
            templates,
            sources,
            output_dir,
            output_extension,
            targets,
            link_format,
            source_link_prefix,
        }
    }
}
