use std::collections::HashSet;
use std::path::Path;

use crate::SnippextResult;

pub fn file_comments(extension: &str) -> SnippextResult<Vec<&'static str>> {
    match extension {
        "adoc" => Ok(vec!["//"]), // AsciiDoc
        "sh" => Ok(vec!["#"]),    // bash
        "c" => Ok(vec!["//"]),
        "cpp" => Ok(vec!["//"]),
        "cs" => Ok(vec!["//"]), // C#
        "css" => Ok(vec!["//"]),
        "ex" | "exs" => Ok(vec!["#"]), // Elixir
        "fs" => Ok(vec!["//"]),        // F#
        "go" => Ok(vec!["//"]),
        "h" | "hpp" => Ok(vec!["//"]),
        "hs" => Ok(vec!["//"]), // Haskell
        "html" => Ok(vec!["<!--"]),
        "java" => Ok(vec!["//"]),
        "js" => Ok(vec!["//"]),
        "json5" => Ok(vec!["//"]),
        "kt" => Ok(vec!["//"]),  // Kotlin
        "lsp" => Ok(vec![";;"]), // Lisp
        "lua" => Ok(vec!["--"]),
        "md" => Ok(vec!["<!--"]), // Markdown
        "m" => Ok(vec!["//"]),    // Objective-c
        "php" => Ok(vec!["//"]),
        "pl" => Ok(vec!["#"]), // Perl
        "py" => Ok(vec!["#"]), // Python

        // For RestructuredText its considered by some as bad practice to have text on same line
        // but thats what we have to work with.
        "rst" => Ok(vec![".."]), // ReStructuredText
        "rb" => Ok(vec!["#"]),   // Ruby
        "rs" => Ok(vec!["//"]),  // Rust
        "scala" => Ok(vec!["//"]),
        "sql" => Ok(vec!["--"]),
        "swift" => Ok(vec!["//"]),
        "tf" => Ok(vec!["#"]), // Terraform
        "toml" => Ok(vec!["#"]),
        "ts" => Ok(vec!["//"]), // TypeScript
        "vb" => Ok(vec!["'"]),
        "xml" => Ok(vec!["<!--"]),
        "yaml" | "yml" => Ok(vec!["#"]),
        _ => Ok(vec!["<!--", "#", "//"]),
    }
}

pub fn get_snippet_start_prefixes(
    extension: &str,
    prefix: &str,
) -> SnippextResult<HashSet<String>> {
    let mut prefixes = get_comment_prefixes(extension, prefix)?;

    if extension == "cs" {
        prefixes.insert("#region".into());
    }

    if extension == "vb" {
        prefixes.insert("#Region".into());
    }

    Ok(prefixes)
}

pub fn get_snippet_end_prefixes(extension: &str, prefix: &str) -> SnippextResult<HashSet<String>> {
    let mut prefixes = get_comment_prefixes(extension, prefix)?;

    // support C# and VB regions
    if extension == "cs" {
        prefixes.insert("#endregion".into());
    }

    if extension == "vb" {
        prefixes.insert("#End Region".into());
    }

    Ok(prefixes)
}

fn get_comment_prefixes(extension: &str, prefix: &str) -> SnippextResult<HashSet<String>> {
    let mut comments = HashSet::new();

    for comment in file_comments(extension)? {
        comments.insert(format!("{comment}{prefix}"));
        comments.insert(format!("{comment} {prefix}"));
    }

    Ok(comments)
}

pub fn extension_from_path(path: &Path) -> String {
    if let Some(ending) = path.extension() {
        ending.to_string_lossy().to_string()
    } else {
        "".into()
    }
}

pub fn extension(filename: &str) -> String {
    extension_from_path(Path::new(filename))
}
