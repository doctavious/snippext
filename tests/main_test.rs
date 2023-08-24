use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use indexmap::IndexMap;

use snippext::cmd::extract::extract;
use snippext::error::SnippextError;
use snippext::settings::SnippextSettings;
use snippext::types::{LinkFormat, SnippetSource};
use tempfile::tempdir;
use tracing_test::traced_test;
use snippext::constants::DEFAULT_TEMPLATE_IDENTIFIER;

#[test]
#[traced_test]
fn should_successfully_extract_from_local_sources_directory() {
    let dir = tempdir().unwrap();

    extract(SnippextSettings::new(
        String::from("snippet::start::"),
        String::from("snippet::end::"),
        IndexMap::from([(
            DEFAULT_TEMPLATE_IDENTIFIER.to_string(),
            String::from("{{snippet}}"),
        )]),
        vec![SnippetSource::Local {
            files: vec![String::from("./tests/samples/*")],
        }],
        Some(dir.path().to_string_lossy().to_string()),
        Some(String::from("md")),
        None,
        None,
        None,
    ))
    .unwrap();

    let main_content_actual =
        fs::read_to_string(Path::new(&dir.path()).join("tests/samples/main.rs/main.md")).unwrap();
    let main_content_expected = r#"fn main() {

    println!("printing...")
}
"#;
    assert_eq!(main_content_expected, main_content_actual);

    let main_nested_content_actual =
        fs::read_to_string(Path::new(&dir.path()).join("tests/samples/main.rs/nested.md")).unwrap();
    assert_eq!("println!(\"printing...\")\n", main_nested_content_actual);

    let sample_fn_1_content_actual =
        fs::read_to_string(Path::new(&dir.path()).join("tests/samples/sample_file.rs/fn_1.md"))
            .unwrap();
    let sample_fn_1_content_expected = r#"fn sample_fn_1() {

}
"#;
    assert_eq!(sample_fn_1_content_expected, sample_fn_1_content_actual);

    let sample_fn_2_content_actual =
        fs::read_to_string(Path::new(&dir.path()).join("tests/samples/sample_file.rs/fn_2.md"))
            .unwrap();
    let sample_fn_2_content_expected = r#"fn sample_fn_2() {

}
"#;
    assert_eq!(sample_fn_2_content_expected, sample_fn_2_content_actual);
}

#[test]
fn error_when_extracting_from_unavailable_remote() {
    let dir = tempdir().unwrap();

    let result = extract(SnippextSettings::new(
        String::from("snippet::start::"),
        String::from("snippet::end::"),
        IndexMap::from([(
            DEFAULT_TEMPLATE_IDENTIFIER.to_string(),
            String::from("{{snippet}}"),
        )]),
        vec![SnippetSource::Git {
            repository: String::from("https://some_bad_url_that_doesnt_exist.blah/not_found.git"),
            reference: Some(String::from("main")),
            cone_patterns: None,
            files: vec![String::from("/tests/**/*")],
        }],
        Some(format!(
            "{}/generated-snippets/",
            dir.path().to_string_lossy()
        )),
        Some(String::from("md")),
        None,
        None,
        None,
    ));

    assert!(result.is_err());
}

#[test]
fn should_error_when_snippet_is_not_closed() {
    let dir = tempdir().unwrap();

    let result = extract(SnippextSettings::new(
        String::from("snippet::start::"),
        String::from("snippet::end::"),
        IndexMap::from([(
            DEFAULT_TEMPLATE_IDENTIFIER.to_string(),
            String::from("{{snippet}}"),
        )]),
        vec![SnippetSource::Local {
            files: vec![String::from("./tests/snippet_left_open.rs")],
        }],
        Some(dir.path().to_string_lossy().to_string()),
        Some(String::from("md")),
        None,
        None,
        None,
    ));

    assert!(result.is_err());
}

#[test]
fn should_successfully_extract_from_remote() {
    let dir = tempdir().unwrap();

    extract(SnippextSettings::new(
        String::from("snippet::start::"),
        String::from("snippet::end::"),
        IndexMap::from([(
            DEFAULT_TEMPLATE_IDENTIFIER.to_string(),
            String::from("{{snippet}}"),
        )]),
        vec![SnippetSource::Git {
            repository: String::from("https://github.com/doctavious/snippext.git"),
            reference: Some(String::from("main")),
            cone_patterns: None,
            files: vec![String::from("/tests/samples/*")],
        }],
        Some(format!(
            "{}/generated-snippets/",
            dir.path().to_string_lossy()
        )),
        Some(String::from("md")),
        None,
        None,
        None,
    ))
    .unwrap();

    let main_content_actual = fs::read_to_string(
        Path::new(&dir.path()).join("generated-snippets/tests/samples/main.rs/main.md"),
    )
    .unwrap();
    let main_content_expected = r#"fn main() {

    println!("printing...")
}
"#;
    assert_eq!(main_content_expected, main_content_actual);
}

#[test]
fn should_successfully_extract_from_local_sources_file() {
    let dir = tempdir().unwrap();

    extract(SnippextSettings::new(
        String::from("snippet::start::"),
        String::from("snippet::end::"),
        IndexMap::from([(
            DEFAULT_TEMPLATE_IDENTIFIER.to_string(),
            String::from("{{snippet}}"),
        )]),
        vec![SnippetSource::Local {
            files: vec![String::from("./tests/samples/custom_prefix.rb")],
        }],
        Some(dir.path().to_string_lossy().to_string()),
        Some(String::from("md")),
        None,
        None,
        None,
    ))
    .unwrap();

    let content =
        fs::read_to_string(Path::new(&dir.path()).join("tests/samples/custom_prefix.rb/ruby.md"))
            .unwrap();

    assert_eq!("puts \"Hello, Ruby!\"\n", content);
}

#[test]
fn should_update_specified_targets() {
    let dir = tempdir().unwrap();

    fs::copy(
        Path::new("./tests/targets/target.md"),
        Path::new(&dir.path()).join("./target.md"),
    )
    .unwrap();

    extract(SnippextSettings::new(
        String::from("snippet::start::"),
        String::from("snippet::end::"),
        IndexMap::from([(
            DEFAULT_TEMPLATE_IDENTIFIER.to_string(),
            String::from("{{snippet}}"),
        )]),
        vec![SnippetSource::Local {
            files: vec![String::from("./tests/samples/*")],
        }],
        Some(dir.path().to_string_lossy().to_string()),
        Some(String::from("md")),
        Some(vec![Path::new(&dir.path())
            .join("./target.md")
            .to_string_lossy()
            .to_string()]),
        None,
        None,
    ))
    .unwrap();

    let actual = fs::read_to_string(Path::new(&dir.path()).join("./target.md")).unwrap();
    let expected = r#"This is some static content

<!-- snippet::start::main -->
fn main() {

    println!("printing...")
}
<!-- snippet::end::main -->

<!-- snippet::start::fn_1 -->
fn sample_fn_1() {

}
<!-- snippet::end::fn_1 -->"#;
    assert_eq!(expected, actual);
}

#[test]
fn should_keep_default_content_in_target_when_snippet_key_is_not_found() {
    let dir = tempdir().unwrap();

    fs::copy(
        Path::new("./tests/targets/target.md"),
        Path::new(&dir.path()).join("./target.md"),
    )
    .unwrap();

    extract(SnippextSettings::new(
        String::from("snippet::start::"),
        String::from("snippet::end::"),
        IndexMap::from([(
            DEFAULT_TEMPLATE_IDENTIFIER.to_string(),
            String::from("{{snippet}}"),
        )]),
        vec![SnippetSource::Local {
            files: vec![String::from("./tests/samples/main.rs")],
        }],
        Some(dir.path().to_string_lossy().to_string()),
        Some(String::from("md")),
        Some(vec![Path::new(&dir.path())
            .join("./target.md")
            .to_string_lossy()
            .to_string()]),
        None,
        None,
    ))
    .unwrap();

    let actual = fs::read_to_string(Path::new(&dir.path()).join("./target.md")).unwrap();
    let expected = r#"This is some static content

<!-- snippet::start::main -->
fn main() {

    println!("printing...")
}
<!-- snippet::end::main -->

<!-- snippet::start::fn_1 -->
some content
<!-- snippet::end::fn_1 -->"#;
    assert_eq!(expected, actual);
}

#[test]
fn should_support_template_with_attributes() {
    let dir = tempdir().unwrap();

    extract(SnippextSettings::new(
        String::from("snippet::start::"),
        String::from("snippet::end::"),
        IndexMap::from([(
            DEFAULT_TEMPLATE_IDENTIFIER.to_string(),
            String::from("```{{lang}}\n{{snippet}}```\n"),
        )]),
        vec![SnippetSource::Local {
            files: vec![String::from("./tests/samples/main.rs")],
        }],
        Some(dir.path().to_string_lossy().to_string()),
        Some(String::from("md")),
        None,
        None,
        None,
    ))
    .unwrap();

    let actual =
        fs::read_to_string(Path::new(&dir.path()).join("tests/samples/main.rs/main.md")).unwrap();
    let expected = r#"```rust
fn main() {

    println!("printing...")
}
```
"#;
    assert_eq!(expected, actual);
}

#[test]
fn support_target_snippet_specifies_template() {
    let dir = tempdir().unwrap();

    fs::copy(
        Path::new("./tests/targets/specify_template.md"),
        Path::new(&dir.path()).join("./specify_template.md"),
    )
    .unwrap();

    extract(SnippextSettings::new(
        String::from("snippet::start::"),
        String::from("snippet::end::"),
        IndexMap::from([
            (
                DEFAULT_TEMPLATE_IDENTIFIER.to_string(),
                String::from("{{snippet}}"),
            ),
            (
                "code".to_string(),
                String::from("```{{lang}}\n{{snippet}}```\n"),
            ),
        ]),
        vec![SnippetSource::Local {
            files: vec![String::from("./tests/samples/main.rs")],
        }],
        None,
        None,
        Some(vec![Path::new(&dir.path())
            .join("./specify_template.md")
            .to_string_lossy()
            .to_string()]),
        None,
        None,
    ))
    .unwrap();

    let actual = fs::read_to_string(Path::new(&dir.path()).join("./specify_template.md")).unwrap();
    let expected = r#"Specify template
<!-- snippet::start::main[template=code] -->
```rust
fn main() {

    println!("printing...")
}
```
<!-- snippet::end::main -->"#;
    assert_eq!(expected, actual);
}

#[test]
fn should_treat_unknown_template_variables_as_empty_string() {
    let dir = tempdir().unwrap();

    extract(SnippextSettings::new(
        String::from("snippet::start::"),
        String::from("snippet::end::"),
        IndexMap::from([(
            DEFAULT_TEMPLATE_IDENTIFIER.to_string(),
            String::from("```{{unknown}}\n{{snippet}}```\n"),
        )]),
        vec![SnippetSource::Local {
            files: vec![String::from("./tests/samples/main.rs")],
        }],
        Some(dir.path().to_string_lossy().to_string()),
        Some(String::from("md")),
        None,
        None,
        None,
    ))
    .unwrap();

    let actual =
        fs::read_to_string(Path::new(&dir.path()).join("tests/samples/main.rs/main.md")).unwrap();
    let expected = r#"```
fn main() {

    println!("printing...")
}
```
"#;
    assert_eq!(expected, actual);
}

#[test]
fn should_support_files_with_no_snippets() {
    let dir = tempdir().unwrap();

    extract(SnippextSettings::new(
        String::from("snippet::start::"),
        String::from("snippet::end::"),
        IndexMap::from([(
            DEFAULT_TEMPLATE_IDENTIFIER.to_string(),
            String::from("{{snippet}}"),
        )]),
        vec![SnippetSource::Local {
            files: vec![String::from("./tests/samples/no_snippets.rs")],
        }],
        Some(dir.path().to_string_lossy().to_string()),
        Some(String::from("md")),
        None,
        None,
        None,
    ))
    .unwrap();

    let files: Vec<PathBuf> = fs::read_dir(&dir)
        .unwrap()
        .into_iter()
        .filter(|r| r.is_ok())
        .map(|r| r.unwrap().path())
        .filter(|r| r.is_dir())
        .collect();

    assert_eq!(0, files.len());
}

#[test]
fn invalid_glob() {
    let dir = tempdir().unwrap();

    let result = extract(SnippextSettings::new(
        String::from("snippet::start::"),
        String::from("snippet::end::"),
        IndexMap::from([(
            DEFAULT_TEMPLATE_IDENTIFIER.to_string(),
            String::from("{{snippet}}"),
        )]),
        vec![SnippetSource::Local {
            files: vec![String::from("[&")],
        }],
        Some(dir.path().to_string_lossy().to_string()),
        Some(String::from("md")),
        None,
        None,
        None,
    ));

    assert!(result.is_err());
    match result.unwrap_err() {
        SnippextError::GlobPatternError(error) => {
            assert_eq!("Glob pattern error for `[&`. invalid range pattern", error);
        }
        _ => {
            panic!("invalid SnippextError");
        }
    }
}

#[test]
fn glob_returns_no_files() {
    let dir = tempdir().unwrap();

    extract(SnippextSettings::new(
        String::from("snippet::start::"),
        String::from("snippet::end::"),
        IndexMap::from([(
            DEFAULT_TEMPLATE_IDENTIFIER.to_string(),
            String::from("{{snippet}}"),
        )]),
        vec![SnippetSource::Local {
            files: vec![String::from("./tests/samples/*.md")],
        }],
        Some(dir.path().to_string_lossy().to_string()),
        Some(String::from("md")),
        None,
        None,
        None,
    ))
    .unwrap();

    let files: Vec<PathBuf> = fs::read_dir(&dir)
        .unwrap()
        .into_iter()
        .filter(|r| r.is_ok())
        .map(|r| r.unwrap().path())
        .filter(|r| r.is_dir())
        .collect();

    assert_eq!(0, files.len());
}

#[test]
fn support_source_links() {
    let dir = tempdir().unwrap();

    extract(SnippextSettings::new(
        String::from("snippet::start::"),
        String::from("snippet::end::"),
        IndexMap::from([(
            DEFAULT_TEMPLATE_IDENTIFIER.to_string(),
            String::from(
                    "```{{snippet}}```{{#if source_links_enabled}}\n{{source_link}}{{/if}}",
                ),
        )]),
        vec![SnippetSource::Local {
            files: vec![String::from("./tests/samples/custom_prefix.rb")],
        }],
        Some(dir.path().to_string_lossy().to_string()),
        Some(String::from("md")),
        None,
        Some(LinkFormat::GitHub),
        None,
    ))
    .unwrap();

    let content =
        fs::read_to_string(Path::new(&dir.path()).join("tests/samples/custom_prefix.rb/ruby.md"))
            .unwrap();

    assert_eq!(
        "```puts \"Hello, Ruby!\"\n```\ntests/samples/custom_prefix.rb#L2-L4",
        content
    );
}

#[test]
fn source_links_should_support_prefix() {
    let dir = tempdir().unwrap();

    extract(SnippextSettings::new(
        String::from("snippet::start::"),
        String::from("snippet::end::"),
        IndexMap::from([(
            DEFAULT_TEMPLATE_IDENTIFIER.to_string(),
            String::from(
                    "```{{snippet}}```{{#if source_links_enabled}}\n{{source_link}}{{/if}}",
                ),
        )]),
        vec![SnippetSource::Local {
            files: vec![String::from("./tests/samples/custom_prefix.rb")],
        }],
        Some(dir.path().to_string_lossy().to_string()),
        Some(String::from("md")),
        None,
        Some(LinkFormat::GitHub),
        Some("http://github.com/foo".into()),
    ))
    .unwrap();

    let content =
        fs::read_to_string(Path::new(&dir.path()).join("tests/samples/custom_prefix.rb/ruby.md"))
            .unwrap();

    assert_eq!(
        "```puts \"Hello, Ruby!\"\n```\nhttp://github.com/foo/tests/samples/custom_prefix.rb#L2-L4",
        content
    );
}

#[test]
fn support_csharp_regions() {
    let dir = tempdir().unwrap();

    extract(SnippextSettings::new(
        String::from("snippet::start::"),
        String::from("snippet::end::"),
        IndexMap::from([(
            DEFAULT_TEMPLATE_IDENTIFIER.to_string(),
            String::from(
                    "```{{snippet}}```{{#if source_links_enabled}}\n{{source_link}}{{/if}}",
                ),
        )]),
        vec![SnippetSource::Local {
            files: vec![String::from("./tests/main.cs")],
        }],
        Some(dir.path().to_string_lossy().to_string()),
        Some(String::from("md")),
        None,
        Some(LinkFormat::GitHub),
        None,
    ))
    .unwrap();

    let content =
        fs::read_to_string(Path::new(&dir.path()).join("tests/main.cs/console.md")).unwrap();

    assert_eq!(
        "```Console.WriteLine(\"Hello World!\");\n```\ntests/main.cs#L1-L3",
        content
    );
}
