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
#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum SnippetSource {
    Local {
        files: Vec<String>,
    },
    Git {
        repository: String,
        reference: Option<String>,
        cone_patterns: Option<Vec<String>>, // for sparse checkout. cone pattern sets
        files: Vec<String>
    },
    Url(String)
}

impl SnippetSource {

    // TODO: add tests
    pub fn source_link(
        &self,
        snippet: &Snippet,
        settings: &SnippextSettings
    ) -> String {
        match self {
            SnippetSource::Local { .. } => {
                if let Some(link_format) = &settings.link_format {
                    let mut path = settings.url_prefix.to_owned().unwrap_or_default();
                    if !path.ends_with("/") {
                        path.push_str("/")
                    }

                    path.push_str(snippet.path.to_str().unwrap_or_default());

                    link_format.source_link(&path, &snippet)
                } else {
                    String::new()
                }
            }
            SnippetSource::Git { repository: url, .. } => {
                let url_str = url;
                if let Some(link_format) = &settings.link_format {
                    return link_format.source_link(&url_str, &snippet);
                } else {
                    let url = Url::from_str(url).ok();
                    if let Some(url) = url {
                        if let Some(link_format) = url.domain()
                            .and_then(|d| LinkFormat::from_domain(d)) {
                            return link_format.source_link(&url_str, &snippet);
                        }
                    }
                }

                return String::new();
            }
            SnippetSource::Url(url) => {
                url.to_string()
            }
        }
    }
}

#[non_exhaustive]
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

    // https://github.com/dotnet/sourcelink/blob/bf63e726a31d7bdb25b4589627cef44da0072174/docs/README.md#custom-content-urls
    // https://github.com/dotnet/sourcelink/blob/bf63e726a31d7bdb25b4589627cef44da0072174/src/SourceLink.Gitea/GetSourceLinkUrl.cs#L21
    // https://github.com/dotnet/sourcelink/blob/bf63e726a31d7bdb25b4589627cef44da0072174/src/SourceLink.AzureRepos.Git/GetSourceLinkUrl.cs#L33
    // https://github.com/dotnet/sourcelink/blob/bf63e726a31d7bdb25b4589627cef44da0072174/src/SourceLink.GitHub/GetSourceLinkUrl.cs#L24
    // https://github.com/dotnet/sourcelink/blob/bf63e726a31d7bdb25b4589627cef44da0072174/src/SourceLink.Bitbucket.Git/GetSourceLinkUrl.cs#L48


    // https://github.com/doctavious/snippext/blob/main/src/cli.rs
    // https://gitea.com/golovin/color-tomato-theme/src/branch/master/LICENSE.md
    // https://gitlab.com/vortex185330/frontend/vortex-frontend/-/blob/main/README.md?ref_type=heads
    // https://gitlab.com/gitlab-org/gitlab/-/blob/master/app/workers/auto_merge_process_worker.rb?ref_type=heads
    // https://bitbucket.org/MyCompany/MyProject/raw/28ebd306a7612e496c73ff142d132f92847b717d/*
    // https://gitee.com/goploy/goploy/blob/master/go.mod#L10-17

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
