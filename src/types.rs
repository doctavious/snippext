use std::collections::HashMap;
use std::fmt;
use std::path::PathBuf;

use clap::{Parser, ValueEnum};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Snippet {
    // The snippet name is sanitized to prevent malicious code to overwrite arbitrary files on your system.
    /// Snippet identifier
    pub identifier: String,
    /// The path the snippet was read from.
    pub path: PathBuf,
    /// Snippet content
    pub text: String,
    /// Per-snippet configuration attributes
    pub attributes: HashMap<String, Value>,
    /// Line the snippets starts on within the source file
    pub start_line: usize,
    /// Line the snippet ends on within the source file
    pub end_line: usize,
    /// Link to the source file the snippet is defined in
    pub source_link: Option<String>,
}

/// Where to look for source snippets. Remote sources will be downloaded
#[non_exhaustive]
#[remain::sorted]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum SnippetSource {
    /// Snippet source that comes from a remote Git repository
    Git {
        /// Git repository to clone
        repository: String,
        /// Git branch to clone
        #[serde(skip_serializing_if = "Option::is_none")]
        branch: Option<String>,
        /// Patterns to use as part of a sparse-checkout
        #[serde(skip_serializing_if = "Option::is_none")]
        cone_patterns: Option<Vec<String>>, // for sparse checkout. cone pattern sets
        /// Glob patterns used to identify files to extract source snippets from
        files: Vec<String>,
    },
    /// Snippet source that comes from local files
    Local {
        /// Glob patterns used to identify files to extract source snippets from
        files: Vec<String>,
    },
    /// Snippet source that comes from a URL
    Url(String),
}

/// Defines the format of snippet source links that appear under each snippet.
#[non_exhaustive]
#[remain::sorted]
#[derive(Clone, Copy, Debug, Deserialize, Parser, Serialize, ValueEnum)]
#[clap(rename_all = "lower")]
pub enum LinkFormat {
    AzureRepos,
    BitBucket,
    Gitea,
    Gitee,
    GitHub,
    GitLab,
}

impl LinkFormat {
    /// All possible Link Format variants
    pub const VARIANTS: &'static [LinkFormat] = &[
        Self::AzureRepos,
        Self::BitBucket,
        Self::GitHub,
        Self::GitLab,
        Self::Gitea,
        Self::Gitee,
    ];

    /// Determine LinkFormat based on URL domain
    pub fn from_domain(domain: &str) -> Option<Self> {
        match domain.split('.').next()? {
            "azure" => Some(LinkFormat::AzureRepos),
            "bitbucket" => Some(LinkFormat::BitBucket),
            "github" => Some(LinkFormat::GitHub),
            "gitlab" => Some(LinkFormat::GitLab),
            "gitea" => Some(LinkFormat::Gitea),
            "gitee" => Some(LinkFormat::Gitee),
            _ => None,
        }
    }
}

impl fmt::Display for LinkFormat {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        // TODO: This logic is used when converting to config-rs Config and their current version
        // 0.13.3 doesnt ignore case when checking strings against variants so it will fail if
        // string does not match variants exactly
        match self {
            LinkFormat::AzureRepos => write!(f, "AzureRepos"),
            LinkFormat::BitBucket => write!(f, "BitBucket"),
            LinkFormat::GitHub => write!(f, "GitHub"),
            LinkFormat::GitLab => write!(f, "GitLab"),
            LinkFormat::Gitea => write!(f, "Gitea"),
            LinkFormat::Gitee => write!(f, "Gitee"),
        }
    }
}

/// Details for missing snippets
#[derive(Debug)]
pub struct MissingSnippet {
    /// The snippet identifier that was not found
    pub key: String,
    /// The line number in the target file for the snippet that was not found in source
    pub line_number: u32,
    /// The source file the missing snippet is in.
    pub path: PathBuf,
}

/// Defines how Snippext should behave when there is snippets in target files that do not match
/// any source snippet.
#[non_exhaustive]
#[remain::sorted]
#[derive(Clone, Debug, Default, Deserialize, Serialize, ValueEnum)]
#[clap(rename_all = "lower")]
pub enum MissingSnippetsBehavior {
    /// Cause extract to fail if any missing snippets are found providing details to users about each
    Fail,
    /// Do nothing if missing snippets are found
    #[default]
    Ignore,
    /// Log a warning for each missing snippet
    Warn,
}

impl fmt::Display for MissingSnippetsBehavior {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        // TODO: This logic is used when converting to config-rs Config and their current version
        // 0.13.3 doesnt ignore case when checking strings against variants so it will fail if
        // string does not match variants exactly
        match self {
            MissingSnippetsBehavior::Fail => write!(f, "Fail"),
            MissingSnippetsBehavior::Ignore => write!(f, "Ignore"),
            MissingSnippetsBehavior::Warn => write!(f, "Warn"),
        }
    }
}