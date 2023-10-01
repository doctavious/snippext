#![doc(html_root_url = "https://docs.rs/snippext")]
#![doc(issue_tracker_base_url = "https://github.com/doctavious/snippext/issues/")]
#![warn(missing_docs)]
#![deny(rustdoc::broken_intra_doc_links)]

pub mod cli;
pub mod cmd;
pub mod constants;
pub mod error;
mod files;
pub mod git;
pub mod sanitize;
pub mod settings;
pub mod templates;
pub mod types;
pub mod unindent;

use crate::error::SnippextError;
use crate::settings::SnippextSettings;

pub type SnippextResult<T> = Result<T, SnippextError>;
