use std::fs;
use std::path::{Path, PathBuf};

use indexmap::IndexMap;
use snippext::cmd::extract::extract;
use snippext::constants::{DEFAULT_END, DEFAULT_START, DEFAULT_TEMPLATE_IDENTIFIER};
use snippext::error::SnippextError;
use snippext::settings::SnippextSettings;
use snippext::types::{LinkFormat, MissingSnippetsBehavior, SnippetSource};
use tempfile::tempdir;
use tracing_test::traced_test;
use walkdir::WalkDir;

#[test]
#[traced_test]
fn should_successfully_extract_from_local_sources_directory() {
    let dir = tempdir().unwrap();

    extract(SnippextSettings::new(
        String::from(DEFAULT_START),
        String::from(DEFAULT_END),
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
        false,
        MissingSnippetsBehavior::default(),
        false,
    ))
    .unwrap();

    let main_content_actual =
        fs::read_to_string(Path::new(&dir.path()).join("tests/samples/main.rs/main_default.md"))
            .unwrap();
    let main_content_expected = r#"fn main() {

    println!("printing...")
}
"#;
    assert_eq!(main_content_expected, main_content_actual);

    let main_nested_content_actual =
        fs::read_to_string(Path::new(&dir.path()).join("tests/samples/main.rs/nested_default.md"))
            .unwrap();
    assert_eq!("println!(\"printing...\")\n", main_nested_content_actual);

    let sample_fn_1_content_actual = fs::read_to_string(
        Path::new(&dir.path()).join("tests/samples/sample_file.rs/fn_1_default.md"),
    )
    .unwrap();
    let sample_fn_1_content_expected = r#"fn sample_fn_1() {

}
"#;
    assert_eq!(sample_fn_1_content_expected, sample_fn_1_content_actual);

    let sample_fn_2_content_actual = fs::read_to_string(
        Path::new(&dir.path()).join("tests/samples/sample_file.rs/fn_2_default.md"),
    )
    .unwrap();
    let sample_fn_2_content_expected = r#"fn sample_fn_2() {

}
"#;
    assert_eq!(sample_fn_2_content_expected, sample_fn_2_content_actual);
}

#[test]
#[traced_test]
fn should_successfully_output_snippet_for_each_template() {
    let dir = tempdir().unwrap();

    extract(SnippextSettings::new(
        String::from(DEFAULT_START),
        String::from(DEFAULT_END),
        IndexMap::from([
            (
                DEFAULT_TEMPLATE_IDENTIFIER.to_string(),
                String::from("{{snippet}}"),
            ),
            ("another_template".to_string(), String::from("{{snippet}}")),
        ]),
        vec![SnippetSource::Local {
            files: vec![String::from("./tests/samples/main.rs")],
        }],
        Some(dir.path().to_string_lossy().to_string()),
        Some(String::from("md")),
        None,
        None,
        None,
        false,
        MissingSnippetsBehavior::default(),
        false,
    ))
    .unwrap();

    let count = WalkDir::new(Path::new(&dir.path()))
        .into_iter()
        .filter_map(Result::ok)
        .filter(|e| e.file_type().is_file())
        .count();

    assert_eq!(4, count);
}

#[test]
fn error_when_extracting_from_unavailable_remote() {
    let dir = tempdir().unwrap();

    let result = extract(SnippextSettings::new(
        String::from(DEFAULT_START),
        String::from(DEFAULT_END),
        IndexMap::from([(
            DEFAULT_TEMPLATE_IDENTIFIER.to_string(),
            String::from("{{snippet}}"),
        )]),
        vec![SnippetSource::Git {
            repository: String::from("https://some_bad_url_that_doesnt_exist.blah/not_found.git"),
            branch: Some(String::from("main")),
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
        false,
        MissingSnippetsBehavior::default(),
        false,
    ));

    assert!(result.is_err());
}

#[test]
fn should_error_when_snippet_is_not_closed() {
    let dir = tempdir().unwrap();

    let result = extract(SnippextSettings::new(
        String::from(DEFAULT_START),
        String::from(DEFAULT_END),
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
        false,
        MissingSnippetsBehavior::default(),
        false,
    ));

    assert!(result.is_err());
}

#[test]
fn test_git_sources() {
    // these tests need to be run sequentially to prevent race condition when performing a git clone
    should_successfully_extract_from_remote_git_repository();
    should_successfully_extract_from_remote_without_branch_provided();
}

fn should_successfully_extract_from_remote_git_repository() {
    let dir = tempdir().unwrap();

    extract(SnippextSettings::new(
        String::from(DEFAULT_START),
        String::from(DEFAULT_END),
        IndexMap::from([(
            DEFAULT_TEMPLATE_IDENTIFIER.to_string(),
            String::from("{{snippet}}{{source_link}}"),
        )]),
        vec![SnippetSource::Git {
            repository: String::from("https://github.com/doctavious/snippext.git"),
            branch: Some(String::from("tests")),
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
        false,
        MissingSnippetsBehavior::default(),
        false,
    ))
    .unwrap();

    let main_content_actual = fs::read_to_string(
        Path::new(&dir.path()).join("generated-snippets/tests/samples/main.rs/main_default.md"),
    )
    .unwrap();
    let main_content_expected = r#"fn main() {

    println!("printing...")
}
https://github.com/doctavious/snippext/blob/tests/tests/samples/main.rs#L1-L8"#;
    assert_eq!(main_content_expected, main_content_actual);
}

fn should_successfully_extract_from_remote_without_branch_provided() {
    let dir = tempdir().unwrap();

    extract(SnippextSettings::new(
        String::from(DEFAULT_START),
        String::from(DEFAULT_END),
        IndexMap::from([(
            DEFAULT_TEMPLATE_IDENTIFIER.to_string(),
            String::from("{{snippet}}"),
        )]),
        vec![SnippetSource::Git {
            repository: String::from("https://github.com/doctavious/snippext.git"),
            branch: None,
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
        false,
        MissingSnippetsBehavior::default(),
        false,
    ))
    .unwrap();

    let main_content_actual = fs::read_to_string(
        Path::new(&dir.path()).join("generated-snippets/tests/samples/main.rs/main_default.md"),
    )
    .unwrap();

    let main_content_expected = r#"fn main() {

    println!("printing...")
}
"#;
    assert_eq!(main_content_expected, main_content_actual);
}

#[test]
fn url_source_link() {
    let dir = tempdir().unwrap();

    extract(SnippextSettings::new(
        String::from(DEFAULT_START),
        String::from(DEFAULT_END),
        IndexMap::from([(
            DEFAULT_TEMPLATE_IDENTIFIER.to_string(),
            String::from("{{snippet}}{{source_link}}"),
        )]),
        vec![SnippetSource::Url (
            "https://gist.githubusercontent.com/seancarroll/94629074d8cb36e9f5a0bc47b72ba6a5/raw/2b9d5db6482c7ff90a0cf3689d2a36b99e77d189/snippext_example.rs".into()
        )],
        Some(format!(
            "{}/generated-snippets/",
            dir.path().to_string_lossy()
        )),
        Some(String::from("md")),
        None,
        None,
        None,
        false,
        MissingSnippetsBehavior::default(),
        false,
    ))
        .unwrap();

    let actual = fs::read_to_string(
        Path::new(&dir.path())
            .join("generated-snippets")
            .join("gist.githubusercontent.com")
            .join("_seancarroll_94629074d8cb36e9f5a0bc47b72ba6a5_raw_2b9d5db6482c7ff90a0cf3689d2a36b99e77d189_snippext_example_rs")
            .join("main_default.md"),
    ).unwrap();

    let expected = r#"fn main() {
    println!("Hello, World!");
}
https://gist.githubusercontent.com/seancarroll/94629074d8cb36e9f5a0bc47b72ba6a5/raw/2b9d5db6482c7ff90a0cf3689d2a36b99e77d189/snippext_example.rs"#;

    assert_eq!(actual, expected);
}

#[test]
fn should_successfully_extract_from_local_sources_file() {
    let dir = tempdir().unwrap();

    extract(SnippextSettings::new(
        String::from(DEFAULT_START),
        String::from(DEFAULT_END),
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
        false,
        MissingSnippetsBehavior::default(),
        false,
    ))
    .unwrap();

    let content = fs::read_to_string(
        Path::new(&dir.path()).join("tests/samples/custom_prefix.rb/ruby_default.md"),
    )
    .unwrap();

    assert_eq!("puts \"Hello, Ruby!\"\n", content);
}

#[test]
fn should_update_specified_targets() {
    let dir = tempdir().unwrap();
    let target = Path::new(&dir.path()).join("target.md");
    fs::copy(Path::new("./tests/targets/target.md"), &target).unwrap();

    extract(SnippextSettings::new(
        String::from(DEFAULT_START),
        String::from(DEFAULT_END),
        IndexMap::from([(
            DEFAULT_TEMPLATE_IDENTIFIER.to_string(),
            String::from("{{snippet}}"),
        )]),
        vec![SnippetSource::Local {
            files: vec![String::from("./tests/samples/*")],
        }],
        Some(dir.path().to_string_lossy().to_string()),
        Some(String::from("md")),
        Some(vec![target.to_string_lossy().to_string()]),
        None,
        None,
        false,
        MissingSnippetsBehavior::default(),
        false,
    ))
    .unwrap();

    let actual = fs::read_to_string(Path::new(&dir.path()).join("./target.md")).unwrap();
    let expected = r#"This is some static content

<!-- snippet::start main -->
fn main() {

    println!("printing...")
}
<!-- snippet::end -->

<!-- snippet::start fn_1 -->
fn sample_fn_1() {

}
<!-- snippet::end -->"#;
    assert_eq!(expected, actual);
}

#[test]
fn should_keep_default_content_in_target_when_snippet_key_is_not_found() {
    let dir = tempdir().unwrap();
    let target = Path::new(&dir.path()).join("target.md");
    fs::copy(Path::new("./tests/targets/target.md"), &target).unwrap();

    extract(SnippextSettings::new(
        String::from(DEFAULT_START),
        String::from(DEFAULT_END),
        IndexMap::from([(
            DEFAULT_TEMPLATE_IDENTIFIER.to_string(),
            String::from("{{snippet}}"),
        )]),
        vec![SnippetSource::Local {
            files: vec![String::from("./tests/samples/main.rs")],
        }],
        Some(dir.path().to_string_lossy().to_string()),
        Some(String::from("md")),
        Some(vec![target.to_string_lossy().to_string()]),
        None,
        None,
        false,
        MissingSnippetsBehavior::default(),
        false,
    ))
    .unwrap();

    let actual = fs::read_to_string(Path::new(&dir.path()).join("./target.md")).unwrap();
    let expected = r#"This is some static content

<!-- snippet::start main -->
fn main() {

    println!("printing...")
}
<!-- snippet::end -->

<!-- snippet::start fn_1 -->
some content
<!-- snippet::end -->"#;
    assert_eq!(expected, actual);
}

#[test]
fn should_support_template_with_attributes() {
    let dir = tempdir().unwrap();

    extract(SnippextSettings::new(
        String::from(DEFAULT_START),
        String::from(DEFAULT_END),
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
        false,
        MissingSnippetsBehavior::default(),
        false,
    ))
    .unwrap();

    let actual =
        fs::read_to_string(Path::new(&dir.path()).join("tests/samples/main.rs/main_default.md"))
            .unwrap();
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
    let target = Path::new(&dir.path()).join("specify_template.md");
    fs::copy(Path::new("./tests/targets/specify_template.md"), &target).unwrap();

    extract(SnippextSettings::new(
        String::from(DEFAULT_START),
        String::from(DEFAULT_END),
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
        Some(vec![target.to_string_lossy().to_string()]),
        None,
        None,
        false,
        MissingSnippetsBehavior::default(),
        false,
    ))
    .unwrap();

    let actual = fs::read_to_string(target).unwrap();
    let expected = r#"Specify template
<!-- snippet::start main { "template": "code" } -->
```rust
fn main() {

    println!("printing...")
}
```
<!-- snippet::end -->"#;
    assert_eq!(expected, actual);
}

#[test]
fn should_support_selected_lines() {
    let dir = tempdir().unwrap();
    let target = Path::new(&dir.path()).join("selected_lines.md");
    fs::copy(Path::new("./tests/targets/selected_lines.md"), &target).unwrap();

    extract(SnippextSettings::new(
        String::from(DEFAULT_START),
        String::from(DEFAULT_END),
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
        Some(vec![target.to_string_lossy().to_string()]),
        None,
        None,
        false,
        MissingSnippetsBehavior::default(),
        false,
    ))
    .unwrap();

    let actual = fs::read_to_string(target).unwrap();
    let expected = r#"Select Lines
<!-- snippet::start main { "template": "code", "selected_lines": ["1", "3-4"] } -->
```rust
fn main() {
    println!("printing...")
}
```
<!-- snippet::end -->"#;
    assert_eq!(expected, actual);
}

#[test]
fn should_treat_unknown_template_variables_as_empty_string() {
    let dir = tempdir().unwrap();

    extract(SnippextSettings::new(
        String::from(DEFAULT_START),
        String::from(DEFAULT_END),
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
        false,
        MissingSnippetsBehavior::default(),
        false,
    ))
    .unwrap();

    let actual =
        fs::read_to_string(Path::new(&dir.path()).join("tests/samples/main.rs/main_default.md"))
            .unwrap();
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
        String::from(DEFAULT_START),
        String::from(DEFAULT_END),
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
        false,
        MissingSnippetsBehavior::default(),
        false,
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
        String::from(DEFAULT_START),
        String::from(DEFAULT_END),
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
        false,
        MissingSnippetsBehavior::default(),
        false,
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
        String::from(DEFAULT_START),
        String::from(DEFAULT_END),
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
        false,
        MissingSnippetsBehavior::default(),
        false,
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
        String::from(DEFAULT_START),
        String::from(DEFAULT_END),
        IndexMap::from([(
            DEFAULT_TEMPLATE_IDENTIFIER.to_string(),
            String::from(
                "```{{snippet}}```{{#unless omit_source_link}}\n{{source_link}}{{/unless}}",
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
        false,
        MissingSnippetsBehavior::default(),
        false,
    ))
    .unwrap();

    let content = fs::read_to_string(
        Path::new(&dir.path()).join("tests/samples/custom_prefix.rb/ruby_default.md"),
    )
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
        String::from(DEFAULT_START),
        String::from(DEFAULT_END),
        IndexMap::from([(
            DEFAULT_TEMPLATE_IDENTIFIER.to_string(),
            String::from(
                "```{{snippet}}```{{#unless omit_source_link}}\n{{source_link}}{{/unless}}",
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
        false,
        MissingSnippetsBehavior::default(),
        false,
    ))
    .unwrap();

    let content = fs::read_to_string(
        Path::new(&dir.path()).join("tests/samples/custom_prefix.rb/ruby_default.md"),
    )
    .unwrap();

    assert_eq!(
        "```puts \"Hello, Ruby!\"\n```\nhttp://github.com/foo/tests/samples/custom_prefix.rb#L2-L4",
        content
    );
}

#[test]
fn omit_source_links() {
    let dir = tempdir().unwrap();

    extract(SnippextSettings::new(
        String::from(DEFAULT_START),
        String::from(DEFAULT_END),
        IndexMap::from([(
            DEFAULT_TEMPLATE_IDENTIFIER.to_string(),
            String::from(
                "```{{snippet}}```{{#unless omit_source_link}}\n{{source_link}}{{/unless}}",
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
        true,
        MissingSnippetsBehavior::default(),
        false,
    ))
    .unwrap();

    let content = fs::read_to_string(
        Path::new(&dir.path()).join("tests/samples/custom_prefix.rb/ruby_default.md"),
    )
    .unwrap();

    assert_eq!("```puts \"Hello, Ruby!\"\n```", content);
}

#[test]
fn support_csharp_regions() {
    let dir = tempdir().unwrap();

    extract(SnippextSettings::new(
        String::from(DEFAULT_START),
        String::from(DEFAULT_END),
        IndexMap::from([(
            DEFAULT_TEMPLATE_IDENTIFIER.to_string(),
            String::from(
                "```{{snippet}}```{{#unless omit_source_link}}\n{{source_link}}{{/unless}}",
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
        false,
        MissingSnippetsBehavior::default(),
        false,
    ))
    .unwrap();

    let content =
        fs::read_to_string(Path::new(&dir.path()).join("tests/main.cs/console_default.md"))
            .unwrap();

    assert_eq!(
        "```Console.WriteLine(\"Hello World!\");\n```\ntests/main.cs#L1-L3",
        content
    );
}

#[test]
fn should_retain_nested_snippet_comments() {
    let dir = tempdir().unwrap();

    extract(SnippextSettings::new(
        String::from(DEFAULT_START),
        String::from(DEFAULT_END),
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
        false,
        MissingSnippetsBehavior::default(),
        true,
    ))
        .unwrap();

    let actual =
        fs::read_to_string(Path::new(&dir.path()).join("tests/samples/main.rs/main_default.md"))
            .unwrap();
    let expected = r#"```rust
fn main() {

    // snippet::start nested
    println!("printing...")
    // snippet::end
}
```
"#;
    assert_eq!(expected, actual);
}