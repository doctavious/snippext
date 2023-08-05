#![doc(html_root_url = "https://docs.rs/snippext")]
#![doc(issue_tracker_base_url = "https://github.com/doctavious/snippext/issues/")]
#![warn(missing_docs)]
#![deny(rustdoc::broken_intra_doc_links)]

//! TODO: add docs for snippext lib
//! [short sentence explaining what it is]
//! [more detailed explanation]
//! [at least one code example that users can copy/paste to try it]
//! [even more advanced explanations if necessary]

pub mod cli;
pub mod cmd;
pub mod constants;
pub mod error;
pub mod git;
pub mod sanitize;
pub mod settings;
pub mod templates;
pub mod types;
pub mod unindent;

use crate::error::SnippextError;
use crate::settings::SnippextSettings;

pub type SnippextResult<T> = Result<T, SnippextError>;
