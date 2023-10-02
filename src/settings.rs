use std::fs;
use std::path::Path;

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use crate::constants::{
    DEFAULT_END, DEFAULT_OUTPUT_DIR, DEFAULT_OUTPUT_FILE_EXTENSION, DEFAULT_SOURCE_FILES,
    DEFAULT_START, DEFAULT_TEMPLATE, DEFAULT_TEMPLATE_IDENTIFIER,
};
use crate::types::{LinkFormat, MissingSnippetsBehavior, SnippetSource};
use crate::SnippextResult;

const fn _default_true() -> bool { true }

/// Snippext configuration settings
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SnippextSettings {
    /// Prefix that marks the start of a snippet.
    pub start: String,
    /// Prefix that marks the end of a snippet.
    pub end: String,
    /// Templates used to render Snippets
    pub templates: IndexMap<String, String>,
    /// Defines where source snippets should be extracted from
    pub sources: Vec<SnippetSource>,
    /// Directory in which the generated snippet files be will output to
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_dir: Option<String>,
    /// Extension for generated files written to the output directory.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_extension: Option<String>,
    /// List of glob patters that contain the files to be spliced with the code snippets.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub targets: Option<Vec<String>>,
    /// Defines the format of snippet source links that appear under each snippet.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub link_format: Option<LinkFormat>,
    /// String that will prefix all local snippet source links.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_link_prefix: Option<String>,
    /// Determines whether source links will be omitted from being rendered
    #[serde(default)]
    pub omit_source_links: bool,
    /// Defined behavior for what to do when missing snippets are present.
    #[serde(default)]
    pub missing_snippets_behavior: MissingSnippetsBehavior,
    /// Determines whether nested snippet comments are included in parent snippets
    #[serde(default)]
    pub retain_nested_snippet_comments: bool,
    /// Determines whether source file language should be autodetected.
    #[serde(default = "_default_true")]
    pub enable_autodetect_language: bool,
    /// determines whether ellipsis should be added to gaps when `select_lines` attribute is used
    /// to render snippets.
    #[serde(default)]
    pub selected_lines_include_ellipses: bool,
}

impl Default for SnippextSettings {
    /// Create default SnippextSettings which will have the following
    /// start: [`DEFAULT_START`]
    /// end: [`DEFAULT_END`]
    /// extension: [`DEFAULT_FILE_EXTENSION`]
    /// template: [`DEFAULT_TEMPLATE`]
    /// sources: all files via [`DEFAULT_SOURCE_FILES`] glob
    /// output_dir: [`DEFAULT_OUTPUT_DIR`]
    /// missing_snippets_behavior: [`MissingSnippetsBehavior::default()`]
    /// enable_autodetect_language: true
    fn default() -> Self {
        Self {
            start: String::from(DEFAULT_START),
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
            omit_source_links: false,
            missing_snippets_behavior: MissingSnippetsBehavior::default(),
            retain_nested_snippet_comments: false,
            enable_autodetect_language: true,
            selected_lines_include_ellipses: false,
        }
    }
}

impl SnippextSettings {
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
}
