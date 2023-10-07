use std::path::Path;

pub type CommentLexicalTokens = (&'static str, Option<&'static str>);

pub const HTML_COMMENT: CommentLexicalTokens = ("<!--", Some("-->"));
pub const LISP_COMMENT: CommentLexicalTokens = (";;", None);
pub const DASH_COMMENT: CommentLexicalTokens = ("--", None);
pub const POUND_COMMENT: CommentLexicalTokens = ("#", None);
pub const SLASH_COMMENT: CommentLexicalTokens = ("//", None);
pub const RESTRUCTUREDTEXT_COMMENT: CommentLexicalTokens = ("..", None);
pub const VB_COMMENT: CommentLexicalTokens = ("'", None);

pub struct SnippextComment {
    pub start: String,
    pub start_close: Option<String>,
    pub end: String,
}

pub struct SnippextComments {
    comments: Vec<SnippextComment>,
}

impl SnippextComments {
    pub fn new(extension: &str, start: &str, end: &str) -> Self {
        Self {
            comments: get_snippet_comments(extension, start, end),
        }
    }

    pub fn is_line_start_snippet(&self, line: &str) -> Option<&SnippextComment> {
        self.comments.iter().find(|&comment| line.starts_with(comment.start.as_str()))
    }

    pub fn is_line_end_snippet(&self, line: &str) -> Option<&SnippextComment> {
        self.comments.iter().find(|&comment| line.starts_with(comment.end.as_str()))
    }
}

pub fn get_snippet_comments(extension: &str, start: &str, end: &str) -> Vec<SnippextComment> {
    let mut snippet_comments = Vec::new();
    for comment in file_comments(extension) {
        let start_close = comment.1.map(str::to_string);
        snippet_comments.push(SnippextComment {
            start: format!("{}{}", comment.0, start.to_string()),
            start_close: start_close.clone(),
            end: format!("{}{}", comment.0, end.to_string()),
        });
        snippet_comments.push(SnippextComment {
            start: format!("{} {}", comment.0, start.to_string()),
            start_close: start_close.clone(),
            end: format!("{} {}", comment.0, end.to_string()),
        });
    }

    if extension == "cs" {
        snippet_comments.push(SnippextComment {
            start: "#region".into(),
            start_close: None,
            end: "#endregion".into(),
        });
    }

    if extension == "vb" {
        snippet_comments.push(SnippextComment {
            start: "#Region".into(),
            start_close: None,
            end: "#End Region".into(),
        });
    }

    snippet_comments
}

// TODO: given we have hyperpolyglot we an tie this to language instead of just file extension
// this would allow us to make this more robust
pub fn file_comments(extension: &str) -> Vec<CommentLexicalTokens> {
    match extension {
        "adoc" => vec![SLASH_COMMENT], // AsciiDoc
        "sh" => vec![POUND_COMMENT],   // bash
        "c" => vec![SLASH_COMMENT],
        "cpp" => vec![SLASH_COMMENT],
        "cs" => vec![SLASH_COMMENT], // C#
        "css" => vec![SLASH_COMMENT],
        "ex" | "exs" => vec![POUND_COMMENT], // Elixir
        "fs" => vec![SLASH_COMMENT],         // F#
        "go" => vec![SLASH_COMMENT],
        "h" | "hpp" => vec![SLASH_COMMENT],
        "hs" => vec![SLASH_COMMENT], // Haskell
        "html" => vec![HTML_COMMENT],
        "java" => vec![SLASH_COMMENT],
        "js" => vec![SLASH_COMMENT],
        "mjs" => vec![SLASH_COMMENT],
        "cjs" => vec![SLASH_COMMENT],

        "json5" => vec![SLASH_COMMENT],
        "kt" => vec![SLASH_COMMENT], // Kotlin
        "lsp" => vec![LISP_COMMENT], // Lisp
        "lua" => vec![DASH_COMMENT],
        "md" => vec![HTML_COMMENT], // Markdown
        "m" => vec![SLASH_COMMENT], // Objective-c
        "php" => vec![SLASH_COMMENT],
        "pl" => vec![POUND_COMMENT], // Perl
        "py" => vec![POUND_COMMENT], // Python

        // For RestructuredText its considered by some as bad practice to have text on same line
        // but thats what we have to work with.
        "rst" => vec![RESTRUCTUREDTEXT_COMMENT], // ReStructuredText
        "rb" => vec![POUND_COMMENT],             // Ruby
        "rs" => vec![SLASH_COMMENT],             // Rust
        "scala" => vec![SLASH_COMMENT],
        "sql" => vec![DASH_COMMENT],
        "swift" => vec![SLASH_COMMENT],
        "tf" => vec![POUND_COMMENT], // Terraform
        "toml" => vec![POUND_COMMENT],
        "ts" => vec![SLASH_COMMENT], // TypeScript
        "vb" => vec![VB_COMMENT],
        "xml" => vec![HTML_COMMENT],
        "yaml" | "yml" => vec![POUND_COMMENT],
        _ => vec![HTML_COMMENT, POUND_COMMENT, SLASH_COMMENT],
    }
}

const TEXT_FILES: [&str; 4] = ["", "adoc", "md", "txt"];
pub(crate) fn is_text_file(extension: &str) -> bool {
    TEXT_FILES.contains(&extension)
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
