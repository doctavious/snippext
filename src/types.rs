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

// TODO: might be better as an enum
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct SnippetSource {
    pub repository: Option<String>,
    pub repository_ref: Option<String>,
    pub cone_patterns: Option<Vec<String>>, // for sparse checkout. cone pattern sets
    pub files: Vec<String>,
    pub url: Option<String>,
}

impl SnippetSource {
    pub fn new_local(files: Vec<String>) -> Self {
        Self {
            repository: None,
            repository_ref: None,
            cone_patterns: None,
            files,
            url: None,
        }
    }

    pub fn new_git(
        repository: String,
        repository_ref: String,
        files: Vec<String>,
    ) -> Self {
        Self {
            repository: Some(repository),
            repository_ref: Some(repository_ref),
            cone_patterns: None,
            files,
            url: None,
        }
    }

    pub fn new_url(url: String) -> Self {
        Self {
            repository: None,
            repository_ref: None,
            cone_patterns: None,
            files: Vec::default(),
            url: Some(url),
        }
    }

    pub fn is_remote(&self) -> bool {
        self.repository.is_some()
    }

    pub fn is_url(&self) -> bool {
        self.url.is_some()
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
