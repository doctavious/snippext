use std::collections::HashSet;

pub mod clear;
pub mod extract;
pub mod init;

// TODO: find a place for this
pub(crate) fn is_line_snippet(line: &str, prefixes: &HashSet<String>) -> Option<String> {
    for prefix in prefixes {
        if line.starts_with(prefix) {
            return Some(prefix.to_string());
        }
    }
    None
}
