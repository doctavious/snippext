use std::collections::HashMap;
use std::fmt;
use std::path::PathBuf;

use clap::{Parser, ValueEnum};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Snippet {
    // The snippet name is sanitized to prevent malicious code to overwrite arbitrary files on your system.
    pub identifier: String,
    /// The path the snippet was read from.
    pub path: PathBuf,
    pub text: String,
    pub attributes: HashMap<String, Value>,
    pub start_line: usize,
    pub end_line: usize,
    pub source_link: Option<String>,
}

#[non_exhaustive]
#[remain::sorted]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum SnippetSource {
    Git {
        repository: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        branch: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        cone_patterns: Option<Vec<String>>, // for sparse checkout. cone pattern sets
        files: Vec<String>,
    },
    Local {
        files: Vec<String>,
    },
    Url(String),
}

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
    pub const VARIANTS: &'static [LinkFormat] = &[
        Self::AzureRepos,
        Self::BitBucket,
        Self::GitHub,
        Self::GitLab,
        Self::Gitea,
        Self::Gitee,
    ];

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

#[derive(Debug)]
pub struct MissingSnippet {
    pub key: String,
    pub line_number: u32,
    pub path: PathBuf,
}

#[non_exhaustive]
#[remain::sorted]
#[derive(Clone, Debug, Default, Deserialize, Serialize, ValueEnum)]
#[clap(rename_all = "lower")]
pub enum MissingSnippetsBehavior {
    Fail,
    #[default]
    Ignore,
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