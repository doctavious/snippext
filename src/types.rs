use std::collections::HashMap;
use std::fmt;
use std::path::PathBuf;
use std::str::FromStr;

use clap::{Parser, ValueEnum};
use serde::{Deserialize, Serialize};
use url::Url;
use crate::settings::SnippextSettings;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Snippet {
    // The snippet name is sanitized to prevent malicious code to overwrite arbitrary files on your system.
    pub identifier: String,
    /// The path the snippet was read from.
    pub path: PathBuf,
    pub text: String,
    pub attributes: HashMap<String, String>,
    pub start_line: usize,
    pub end_line: usize,
}

impl Snippet {
    pub fn new(
        identifier: String,
        path: PathBuf,
        text: String,
        attributes: HashMap<String, String>,
        start_line: usize,
        end_line: usize,
    ) -> Self {
        Self {
            identifier,
            path,
            text,
            attributes,
            start_line,
            end_line,
        }
    }
}

// SourceLocation

// SnippetSource
// path: string

#[non_exhaustive]
#[remain::sorted]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum SnippetSource {
    Git {
        repository: String,
        reference: Option<String>,
        cone_patterns: Option<Vec<String>>, // for sparse checkout. cone pattern sets
        files: Vec<String>
    },
    Local {
        files: Vec<String>,
    },
    Url(String)
}

#[non_exhaustive]
#[remain::sorted]
#[derive(Clone, Copy, Debug, Deserialize, Parser, Serialize, ValueEnum)]
#[clap(rename_all = "lowercase")]
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

    pub fn source_link(
        &self,
        url: &String,
        snippet: &Snippet
    ) -> String {
        match self {
            LinkFormat::AzureRepos => format!(
                "{}&line={}&lineEnd={}",
                url, snippet.start_line, snippet.end_line
            ),
            LinkFormat::BitBucket => {
                format!("{}#lines={}:{}", url, snippet.start_line, snippet.end_line)
            }
            LinkFormat::GitHub => format!("{}#L{}-L{}", url, snippet.start_line, snippet.end_line),
            LinkFormat::GitLab => format!("{}#L{}-{}", url, snippet.start_line, snippet.end_line),
            LinkFormat::Gitea => format!("{}#L{}-L{}", url, snippet.start_line, snippet.end_line),
            LinkFormat::Gitee => format!("{}#L{}-{}", url, snippet.start_line, snippet.end_line),
        }
    }

    pub fn from_domain(domain: &str) -> Option<Self> {
        match domain.split('.').next()? {
            "azure" => Some(LinkFormat::AzureRepos),
            "bitbucket" => Some(LinkFormat::BitBucket),
            "github" => Some(LinkFormat::GitHub),
            "gitlab" => Some(LinkFormat::GitLab),
            "gitea" => Some(LinkFormat::Gitea),
            "gitee" => Some(LinkFormat::Gitee),
            _ => None
        }
    }

    // TODO: not a fan that this is separate from generally building source link
    // Would prefer something with better cohesion but not sure what that looks like
    // TODO: figure out appropriate value for Azure Repos. See
    // https://github.com/dotnet/sourcelink/blob/bf63e726a31d7bdb25b4589627cef44da0072174/src/SourceLink.AzureRepos.Git/GetSourceLinkUrl.cs#L33
    pub fn blob_path_segment(&self) -> &str {
        match self {
            LinkFormat::AzureRepos => "",
            LinkFormat::BitBucket => "/raw/",
            LinkFormat::GitHub => "/blob/",
            LinkFormat::GitLab => "/-/blob/",
            LinkFormat::Gitea => "-/blob/",
            LinkFormat::Gitee => "/blob/",
        }
    }
}

impl fmt::Display for LinkFormat {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            LinkFormat::AzureRepos => write!(f, "azure"),
            LinkFormat::BitBucket => write!(f, "bitbucket"),
            LinkFormat::GitHub => write!(f, "github"),
            LinkFormat::GitLab => write!(f, "gitlab"),
            LinkFormat::Gitea => write!(f, "gitea"),
            LinkFormat::Gitee => write!(f, "gitee"),

        }
    }
}
