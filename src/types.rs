use std::collections::HashMap;
use std::fmt;
use std::path::PathBuf;

use clap::{Parser, ValueEnum};
use serde::{Deserialize, Serialize};

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

#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum SnippetSource {
    Local {
        files: Vec<String>,
    },
    Git {
        url: String,
        reference: Option<String>,
        cone_patterns: Option<Vec<String>>, // for sparse checkout. cone pattern sets
        files: Vec<String>
    },
    Url(String)
}

impl SnippetSource {

    // TODO: add tests
    pub fn source_link(
        self,
        snippet: &Snippet,
        link_format: &LinkFormat,
        url_prefix: String
    ) -> String {

        match self {
            SnippetSource::Local { .. } => {
                let mut path = url_prefix;
                if !path.ends_with("/") {
                    path.push_str("/")
                }

                path.push_str(snippet.path.to_str().unwrap_or_default());

                link_format.source_link(&path, &snippet)
            }
            SnippetSource::Git { url, .. } => {
                link_format.source_link(&url, &snippet)
            }
            SnippetSource::Url(url) => {
                url
            }
        }
    }
}

#[derive(Clone, Debug, Deserialize, Parser, Serialize, ValueEnum)]
#[clap(rename_all = "lowercase")]
pub enum LinkFormat {
    GitHub,
    GitLab,
    Gitea,
    BitBucket,
    TFS,
}

impl LinkFormat {
    pub const VARIANTS: &'static [LinkFormat] = &[
        Self::GitHub,
        Self::GitLab,
        Self::Gitea,
        Self::BitBucket,
        Self::TFS,
    ];

    pub fn source_link(&self, url: &String, snippet: &Snippet) -> String {
        match self {
            LinkFormat::GitHub => format!("{}#L{}-L{}", url, snippet.start_line, snippet.end_line),
            LinkFormat::GitLab => format!("{}#L{}-{}", url, snippet.start_line, snippet.end_line),
            LinkFormat::BitBucket => {
                format!("{}#lines={}:{}", url, snippet.start_line, snippet.end_line)
            }
            LinkFormat::Gitea => format!("{}#L{}-L{}", url, snippet.start_line, snippet.end_line),
            LinkFormat::TFS => format!(
                "{}&line={}&lineEnd={}",
                url, snippet.start_line, snippet.end_line
            ),
        }
    }
}

impl fmt::Display for LinkFormat {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            LinkFormat::GitHub => write!(f, "github"),
            LinkFormat::GitLab => write!(f, "gitlab"),
            LinkFormat::Gitea => write!(f, "gitea"),
            LinkFormat::BitBucket => write!(f, "bitbucket"),
            LinkFormat::TFS => write!(f, "tfs"),
        }
    }
}
