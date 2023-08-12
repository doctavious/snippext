use std::collections::HashSet;
use std::path::Path;
use crate::error::SnippextError;
use crate::files::FileType::*;
use crate::SnippextResult;

#[derive(Debug)]
pub enum FileType {
    AsciiDoc,
    Bash,
    C,
    CPP,
    CSharp,
    CSS,
    Elixir,
    FSharp,
    Go,
    Haskell,
    HTML,
    Java,
    JavaScript,
    JSON5,
    Kotlin,
    Lisp,
    Lua,
    Markdown,
    ObjectiveC,
    PHP,
    Perl,
    Python,
    ReStructuredText,
    Ruby,
    Rust,
    Scala,
    SQL,
    Swift,
    Terraform,
    TOML,
    TypeScript,
    VB,
    XML,
    YAML,
    Unknown
}


pub fn file_type(filename: &str) -> SnippextResult<Vec<FileType>> {
    match extension(filename)?.as_str() {
        "adoc" => Ok(vec![AsciiDoc]),
        "sh" => Ok(vec![Bash]),
        "c" => Ok(vec![C]),
        "cpp" => Ok(vec![CPP]),
        "cs" => Ok(vec![CSharp]),
        "css" => Ok(vec![CSS]),
        "ex" | "exs" => Ok(vec![Elixir]),
        "fs" => Ok(vec![FSharp]),
        "go" => Ok(vec![Go]),
        "h" | "hpp" => Ok(vec![C, CPP, ObjectiveC]),
        "hs" => Ok(vec![Haskell]),
        "html" => Ok(vec![HTML]),
        "java" => Ok(vec![Java]),
        "js" => Ok(vec![JavaScript]),
        "json5" => Ok(vec![JSON5]),
        "kt" => Ok(vec![Kotlin]),
        "lsp" => Ok(vec![Lisp]),
        "lua" => Ok(vec![Lua]),
        "md" => Ok(vec![Markdown]),
        "m" => Ok(vec![ObjectiveC]),
        "php" => Ok(vec![PHP]),
        "pl" => Ok(vec![Perl]),
        "py" => Ok(vec![Python]),

        // For RestructuredText its considered by some as bad practice to have text on same line
        // but thats what we have to work with.
        "rst" => Ok(vec![ReStructuredText]),
        "rb" => Ok(vec![Ruby]),
        "rs" => Ok(vec![Rust]),
        "scala" => Ok(vec![Scala]),
        "sql" => Ok(vec![SQL]),
        "swift" => Ok(vec![Swift]),
        "tf" => Ok(vec![Terraform]),
        "toml" => Ok(vec![TOML]),
        "ts" => Ok(vec![TypeScript]),
        "vb" => Ok(vec![VB]),
        "xml" => Ok(vec![XML]),
        "yaml" | "yml" => Ok(vec![YAML]),
        _ => Ok(vec![Unknown]),
    }
}


impl FileType {
    fn create_snippet_comments(&self, tag: &String) -> Vec<String> {
        let mut tag_prefixes = Vec::default();
        for comment in self.file_comments() {
            tag_prefixes.push(format!("{comment}{tag}"));
            tag_prefixes.push(format!("{comment} {tag}"));
        }
        tag_prefixes
    }

    // regions are not comments so perhaps another word is more appropriate
    pub fn snippet_start_comments(&self, tag: String) -> Vec<String> {
        match self {
            CSharp => {
                let mut prefixes = self.create_snippet_comments(&tag);
                prefixes.push("#region".into());
                prefixes
            },
            VB => {
                let mut prefixes = self.create_snippet_comments(&tag);
                prefixes.push("#Region".into());
                prefixes
            },
            _ => self.create_snippet_comments(&tag)
        }
    }

    pub fn snippet_end_comments(&self, tag: String) -> Vec<String> {
        match self {
            CSharp => {
                let mut prefixes = self.create_snippet_comments(&tag);
                prefixes.push(format!("#endregion"));
                prefixes
            },
            VB => {
                let mut prefixes = self.create_snippet_comments(&tag);
                prefixes.push(format!("#End Region"));
                prefixes
            },
            _ => self.create_snippet_comments(&tag)
        }
    }

    pub fn file_comments(&self) -> Vec<&'static str> {
        match self {
            AsciiDoc => vec!["//"], // AsciiDoc
            Bash => vec!["#"], // bash
            C => vec!["//"],
            CPP => vec!["//"],
            CSharp => vec!["//"], // C#
            CSS => vec!["//"],
            Elixir => vec!["#"], // Elixir
            FSharp => vec!["//"], // F#
            Go => vec!["//"],
            Haskell => vec!["//"], // Haskell
            HTML => vec!["<!--"],
            Java => vec!["//"],
            JavaScript => vec!["//"],
            JSON5 => vec!["//"],
            Kotlin => vec!["//"], // Kotlin
            Lisp => vec![";;"], // Lisp
            Lua => vec!["--"],
            Markdown => vec!["<!--"], // Markdown
            ObjectiveC => vec!["//"], // Objective-c
            PHP => vec!["//"],
            Perl => vec!["#"], // Perl
            Python => vec!["#"], // Python

            // For RestructuredText its considered by some as bad practice to have text on same line
            // but thats what we have to work with.
            ReStructuredText => vec![".."], // ReStructuredText
            Ruby => vec!["#"], // Ruby
            Rust => vec!["//"], // Rust
            Scala => vec!["//"],
            SQL => vec!["--"],
            Swift => vec!["//"],
            Terraform => vec!["#"], // Terraform
            TOML => vec!["#"],
            TypeScript => vec!["//"], // TypeScript
            VB => vec!["'"],
            XML => vec!["<!--"],
            YAML => vec!["#"],
            _ => vec!["<!--", "#", "//" ],
        }
    }
}


pub fn file_comments(extension: &str) -> SnippextResult<Vec<&'static str>> {
    match extension {
        "adoc" => Ok(vec!["//"]), // AsciiDoc
        "sh" => Ok(vec!["#"]), // bash
        "c" => Ok(vec!["//"]),
        "cpp" => Ok(vec!["//"]),
        "cs" => Ok(vec!["//"]), // C#
        "css" => Ok(vec!["//"]),
        "ex" | "exs" => Ok(vec!["#"]), // Elixir
        "fs" => Ok(vec!["//"]), // F#
        "go" => Ok(vec!["//"]),
        "h" | "hpp" => Ok(vec!["//"]),
        "hs" => Ok(vec!["//"]), // Haskell
        "html" => Ok(vec!["<!--"]),
        "java" => Ok(vec!["//"]),
        "js" => Ok(vec!["//"]),
        "json5" => Ok(vec!["//"]),
        "kt" => Ok(vec!["//"]), // Kotlin
        "lsp" => Ok(vec![";;"]), // Lisp
        "lua" => Ok(vec!["--"]),
        "md" => Ok(vec!["<!--"]), // Markdown
        "m" => Ok(vec!["//"]), // Objective-c
        "php" => Ok(vec!["//"]),
        "pl" => Ok(vec!["#"]), // Perl
        "py" => Ok(vec!["#"]), // Python

        // For RestructuredText its considered by some as bad practice to have text on same line
        // but thats what we have to work with.
        "rst" => Ok(vec![".."]), // ReStructuredText
        "rb" => Ok(vec!["#"]), // Ruby
        "rs" => Ok(vec!["//"]), // Rust
        "scala" => Ok(vec!["//"]),
        "sql" => Ok(vec!["--"]),
        "swift" => Ok(vec!["//"]),
        "tf" => Ok(vec!["#"]), // Terraform
        "toml" => Ok(vec!["#"]),
        "ts" => Ok(vec!["//"]), // TypeScript
        "vb" => Ok(vec!["'"]),
        "xml" => Ok(vec!["<!--"]),
        "yaml" | "yml" => Ok(vec!["#"]),
        _ => Ok(vec!["<!--", "#", "//" ]),
    }
}

pub fn get_snippet_start_prefixes(extension: &str, tag: &str) -> SnippextResult<HashSet<String>> {
    let mut prefixes = get_comment_prefixes(extension, tag)?;

    if extension == "cs" {
        prefixes.insert("#region".into());
    }

    if extension == "vb" {
        prefixes.insert("#Region".into());
    }

    Ok(prefixes)
}


pub fn get_snippet_end_prefixes(extension: &str, tag: &str) -> SnippextResult<HashSet<String>> {
    let mut prefixes = get_comment_prefixes(extension, tag)?;

    // support C# and VB regions
    if extension == "cs" {
        prefixes.insert("#endregion".into());
    }

    if extension == "vb" {
        prefixes.insert("#End Region".into());
    }

    Ok(prefixes)
}

fn get_comment_prefixes(extension: &str, tag: &str) -> SnippextResult<HashSet<String>> {
    let mut comments = HashSet::new();

    for comment in file_comments(extension)? {
        comments.insert(format!("{comment}{tag}"));
        comments.insert(format!("{comment} {tag}"));
    }

    Ok(comments)
}

pub fn extension_from_path(path: &Path) -> SnippextResult<String> {
    if let Some(ending) = path.extension() {
        Ok(ending.to_string_lossy().to_string())
    } else {
        Err(SnippextError::GeneralError(
            format!("Could not determine file extension for {}", path.to_string_lossy()),
        ))
    }
}

pub fn extension(filename: &str) -> SnippextResult<String> {
    extension_from_path(Path::new(filename))
}


pub fn parse_filename(filename: &Path) -> SnippextResult<&str> {
    filename
        .to_str()
        .ok_or_else(|| SnippextError::GeneralError(format!("Invalid filename {}", filename.to_string_lossy())))
}
