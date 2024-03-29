use std::fs;
use std::path::{Path, PathBuf};

use indexmap::IndexMap;
use snippext::cmd::extract::extract;
use snippext::constants::{DEFAULT_SNIPPEXT_CONFIG, DEFAULT_TEMPLATE, DEFAULT_TEMPLATE_IDENTIFIER};
use snippext::error::SnippextError;
use snippext::settings::SnippextSettings;
use snippext::types::{LinkFormat, SnippetSource};
use tempfile::tempdir;
use tracing_test::traced_test;
use walkdir::WalkDir;

#[test]
fn should_deserialize_default_config_to_snippext_settings() {
    let settings: Result<SnippextSettings, serde_yaml::Error> =
        serde_yaml::from_str(DEFAULT_SNIPPEXT_CONFIG);
    assert!(settings.is_ok());
}

#[test]
#[traced_test]
fn should_successfully_extract_from_local_sources_directory() {
    let dir = tempdir().unwrap();

    extract(SnippextSettings {
        templates: IndexMap::from([(
            DEFAULT_TEMPLATE_IDENTIFIER.to_string(),
            String::from("{{snippet}}"),
        )]),
        sources: vec![SnippetSource::Local {
            files: vec![String::from("./tests/samples/*")],
        }],
        output_dir: Some(dir.path().to_string_lossy().to_string()),
        output_extension: Some(String::from("md")),
        ..Default::default()
    })
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
fn should_successfully_extract_from_local_sources_directory_with_default_template() {
    let dir = tempdir().unwrap();

    extract(SnippextSettings {
        templates: IndexMap::from([(
            DEFAULT_TEMPLATE_IDENTIFIER.to_string(),
            DEFAULT_TEMPLATE.to_string(),
        )]),
        sources: vec![SnippetSource::Local {
            files: vec![String::from("./tests/samples/*")],
        }],
        output_dir: Some(dir.path().to_string_lossy().to_string()),
        output_extension: Some(String::from("md")),
        ..Default::default()
    })
    .unwrap();

    let main_content_actual =
        fs::read_to_string(Path::new(&dir.path()).join("tests/samples/main.rs/main_default.md"))
            .unwrap();
    let main_content_expected = r#"```rust
fn main() {

    println!("printing...")
}
```
<a href='tests/samples/main.rs' title='Snippet source file'>snippet source</a>
"#;
    assert_eq!(main_content_expected, main_content_actual);
}

#[test]
#[traced_test]
fn should_successfully_output_snippet_for_each_template() {
    let dir = tempdir().unwrap();

    extract(SnippextSettings {
        templates: IndexMap::from([
            (
                DEFAULT_TEMPLATE_IDENTIFIER.to_string(),
                String::from("```{{lang}}\n{{snippet}}```"),
            ),
            ("another_template".to_string(), String::from("{{snippet}}")),
        ]),
        sources: vec![SnippetSource::Local {
            files: vec![String::from("./tests/samples/main.rs")],
        }],
        output_dir: Some(dir.path().to_string_lossy().to_string()),
        output_extension: Some(String::from("md")),
        ..Default::default()
    })
    .unwrap();

    let count = WalkDir::new(Path::new(&dir.path()))
        .into_iter()
        .filter_map(Result::ok)
        .filter(|e| e.file_type().is_file())
        .count();

    assert_eq!(4, count);

    let main_default_content_actual =
        fs::read_to_string(Path::new(&dir.path()).join("tests/samples/main.rs/main_default.md"))
            .unwrap();
    let main_default_content_expected = r#"```rust
fn main() {

    println!("printing...")
}
```"#;
    assert_eq!(main_default_content_expected, main_default_content_actual);

    let main_raw_content_actual = fs::read_to_string(
        Path::new(&dir.path()).join("tests/samples/main.rs/main_another_template.md"),
    )
    .unwrap();
    let main_raw_content_expected = r#"fn main() {

    println!("printing...")
}
"#;
    assert_eq!(main_raw_content_expected, main_raw_content_actual);
}

#[test]
fn error_when_extracting_from_unavailable_remote() {
    let dir = tempdir().unwrap();

    let result = extract(SnippextSettings {
        templates: IndexMap::from([(
            DEFAULT_TEMPLATE_IDENTIFIER.to_string(),
            String::from("{{snippet}}"),
        )]),
        sources: vec![SnippetSource::Git {
            repository: String::from("https://some_bad_url_that_doesnt_exist.blah/not_found.git"),
            branch: Some(String::from("main")),
            cone_patterns: None,
            files: vec![String::from("/tests/**/*")],
        }],
        output_dir: Some(format!(
            "{}/generated-snippets/",
            dir.path().to_string_lossy()
        )),
        output_extension: Some(String::from("md")),
        ..Default::default()
    });

    assert!(result.is_err());
}

#[test]
fn should_error_when_snippet_is_not_closed() {
    let dir = tempdir().unwrap();

    let result = extract(SnippextSettings {
        templates: IndexMap::from([(
            DEFAULT_TEMPLATE_IDENTIFIER.to_string(),
            String::from("{{snippet}}"),
        )]),
        sources: vec![SnippetSource::Local {
            files: vec![String::from("./tests/snippet_left_open.rs")],
        }],
        output_dir: Some(dir.path().to_string_lossy().to_string()),
        output_extension: Some(String::from("md")),
        ..Default::default()
    });

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

    extract(SnippextSettings {
        templates: IndexMap::from([(
            DEFAULT_TEMPLATE_IDENTIFIER.to_string(),
            String::from("{{snippet}}{{source_link}}"),
        )]),
        sources: vec![SnippetSource::Git {
            repository: String::from("https://github.com/doctavious/snippext.git"),
            branch: Some(String::from("main")),
            cone_patterns: None,
            files: vec![String::from("/tests/samples/*")],
        }],
        output_dir: Some(format!(
            "{}/generated-snippets/",
            dir.path().to_string_lossy()
        )),
        output_extension: Some(String::from("md")),
        ..Default::default()
    })
    .unwrap();

    let main_content_actual = fs::read_to_string(
        Path::new(&dir.path()).join("generated-snippets/tests/samples/main.rs/main_default.md"),
    )
    .unwrap();
    let main_content_expected = r#"fn main() {

    println!("printing...")
}
https://github.com/doctavious/snippext/blob/main/tests/samples/main.rs#L1-L8"#;
    assert_eq!(main_content_expected, main_content_actual);
}

fn should_successfully_extract_from_remote_without_branch_provided() {
    let dir = tempdir().unwrap();

    extract(SnippextSettings {
        templates: IndexMap::from([(
            DEFAULT_TEMPLATE_IDENTIFIER.to_string(),
            String::from("{{snippet}}"),
        )]),
        sources: vec![SnippetSource::Git {
            repository: String::from("https://github.com/doctavious/snippext.git"),
            branch: None,
            cone_patterns: None,
            files: vec![String::from("/tests/samples/*")],
        }],
        output_dir: Some(format!(
            "{}/generated-snippets/",
            dir.path().to_string_lossy()
        )),
        output_extension: Some(String::from("md")),
        ..Default::default()
    })
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

    extract(SnippextSettings {
        templates: IndexMap::from([(
            DEFAULT_TEMPLATE_IDENTIFIER.to_string(),
            String::from("{{snippet}}{{source_link}}"),
        )]),
        sources: vec![SnippetSource::Url {
            url: "https://gist.githubusercontent.com/seancarroll/94629074d8cb36e9f5a0bc47b72ba6a5/raw/2b9d5db6482c7ff90a0cf3689d2a36b99e77d189/snippext_example.rs".into()
        }],
        output_dir: Some(format!(
            "{}/generated-snippets/",
            dir.path().to_string_lossy()
        )),
        output_extension: Some(String::from("md")),
        ..Default::default()
    })
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

    extract(SnippextSettings {
        templates: IndexMap::from([(
            DEFAULT_TEMPLATE_IDENTIFIER.to_string(),
            String::from("{{snippet}}"),
        )]),
        sources: vec![SnippetSource::Local {
            files: vec![String::from("./tests/samples/custom_prefix.rb")],
        }],
        output_dir: Some(dir.path().to_string_lossy().to_string()),
        output_extension: Some(String::from("md")),
        ..Default::default()
    })
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

    extract(SnippextSettings {
        templates: IndexMap::from([(
            DEFAULT_TEMPLATE_IDENTIFIER.to_string(),
            String::from("{{snippet}}"),
        )]),
        sources: vec![SnippetSource::Local {
            files: vec![String::from("./tests/samples/*")],
        }],
        output_dir: Some(dir.path().to_string_lossy().to_string()),
        output_extension: Some(String::from("md")),
        targets: Some(vec![target.to_string_lossy().to_string()]),
        ..Default::default()
    })
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

    extract(SnippextSettings {
        templates: IndexMap::from([(
            DEFAULT_TEMPLATE_IDENTIFIER.to_string(),
            String::from("{{snippet}}"),
        )]),
        sources: vec![SnippetSource::Local {
            files: vec![String::from("./tests/samples/main.rs")],
        }],
        output_dir: Some(dir.path().to_string_lossy().to_string()),
        output_extension: Some(String::from("md")),
        targets: Some(vec![target.to_string_lossy().to_string()]),
        ..Default::default()
    })
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

    extract(SnippextSettings {
        templates: IndexMap::from([(
            DEFAULT_TEMPLATE_IDENTIFIER.to_string(),
            String::from("```{{lang}}\n{{snippet}}```\n"),
        )]),
        sources: vec![SnippetSource::Local {
            files: vec![String::from("./tests/samples/main.rs")],
        }],
        output_dir: Some(dir.path().to_string_lossy().to_string()),
        output_extension: Some(String::from("md")),
        ..Default::default()
    })
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

    extract(SnippextSettings {
        templates: IndexMap::from([
            (
                DEFAULT_TEMPLATE_IDENTIFIER.to_string(),
                String::from("{{snippet}}"),
            ),
            (
                "code".to_string(),
                String::from("```{{lang}}\n{{snippet}}```\n"),
            ),
        ]),
        sources: vec![SnippetSource::Local {
            files: vec![String::from("./tests/samples/main.rs")],
        }],
        targets: Some(vec![target.to_string_lossy().to_string()]),
        ..Default::default()
    })
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

    extract(SnippextSettings {
        templates: IndexMap::from([
            (
                DEFAULT_TEMPLATE_IDENTIFIER.to_string(),
                String::from("{{snippet}}"),
            ),
            (
                "code".to_string(),
                String::from("```{{lang}}\n{{snippet}}```\n"),
            ),
        ]),
        sources: vec![SnippetSource::Local {
            files: vec![String::from("./tests/samples/main.rs")],
        }],
        targets: Some(vec![target.to_string_lossy().to_string()]),
        ..Default::default()
    })
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
fn should_support_selected_lines_with_ellipsis() {
    let dir = tempdir().unwrap();
    let target = Path::new(&dir.path()).join("selected_lines_with_ellipsis.md");
    fs::copy(
        Path::new("./tests/targets/selected_lines_with_ellipsis.md"),
        &target,
    )
    .unwrap();

    extract(SnippextSettings {
        templates: IndexMap::from([
            (
                DEFAULT_TEMPLATE_IDENTIFIER.to_string(),
                String::from("{{snippet}}"),
            ),
            (
                "code".to_string(),
                String::from("```{{lang}}\n{{snippet}}```\n"),
            ),
        ]),
        sources: vec![SnippetSource::Local {
            files: vec![String::from("./tests/samples/main.rs")],
        }],
        targets: Some(vec![target.to_string_lossy().to_string()]),
        ..Default::default()
    })
    .unwrap();

    let actual = fs::read_to_string(target).unwrap();
    let expected = r#"Select Lines
<!-- snippet::start main { "template": "code", "selected_lines": ["1", "3-4"], "selected_lines_include_ellipses": true } -->
```rust
fn main() {
    // ...
    println!("printing...")
}
```
<!-- snippet::end -->"#;
    assert_eq!(expected, actual);
}

#[test]
fn should_treat_unknown_template_variables_as_empty_string() {
    let dir = tempdir().unwrap();

    extract(SnippextSettings {
        templates: IndexMap::from([(
            DEFAULT_TEMPLATE_IDENTIFIER.to_string(),
            String::from("```{{unknown}}\n{{snippet}}```\n"),
        )]),
        sources: vec![SnippetSource::Local {
            files: vec![String::from("./tests/samples/main.rs")],
        }],
        output_dir: Some(dir.path().to_string_lossy().to_string()),
        output_extension: Some(String::from("md")),
        ..Default::default()
    })
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

    extract(SnippextSettings {
        templates: IndexMap::from([(
            DEFAULT_TEMPLATE_IDENTIFIER.to_string(),
            String::from("{{snippet}}"),
        )]),
        sources: vec![SnippetSource::Local {
            files: vec![String::from("./tests/samples/no_snippets.rs")],
        }],
        output_dir: Some(dir.path().to_string_lossy().to_string()),
        output_extension: Some(String::from("md")),
        ..Default::default()
    })
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

    let result = extract(SnippextSettings {
        templates: IndexMap::from([(
            DEFAULT_TEMPLATE_IDENTIFIER.to_string(),
            String::from("{{snippet}}"),
        )]),
        sources: vec![SnippetSource::Local {
            files: vec![String::from("[&")],
        }],
        output_dir: Some(dir.path().to_string_lossy().to_string()),
        output_extension: Some(String::from("md")),
        ..Default::default()
    });

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

    extract(SnippextSettings {
        templates: IndexMap::from([(
            DEFAULT_TEMPLATE_IDENTIFIER.to_string(),
            String::from("{{snippet}}"),
        )]),
        sources: vec![SnippetSource::Local {
            files: vec![String::from("./tests/samples/*.md")],
        }],
        output_dir: Some(dir.path().to_string_lossy().to_string()),
        output_extension: Some(String::from("md")),
        ..Default::default()
    })
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

    extract(SnippextSettings {
        templates: IndexMap::from([(
            DEFAULT_TEMPLATE_IDENTIFIER.to_string(),
            String::from(
                "```{{snippet}}```{{#unless omit_source_link}}\n{{source_link}}{{/unless}}",
            ),
        )]),
        sources: vec![SnippetSource::Local {
            files: vec![String::from("./tests/samples/custom_prefix.rb")],
        }],
        output_dir: Some(dir.path().to_string_lossy().to_string()),
        output_extension: Some(String::from("md")),
        link_format: Some(LinkFormat::GitHub),
        ..Default::default()
    })
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

    extract(SnippextSettings {
        templates: IndexMap::from([(
            DEFAULT_TEMPLATE_IDENTIFIER.to_string(),
            String::from(
                "```{{snippet}}```{{#unless omit_source_link}}\n{{source_link}}{{/unless}}",
            ),
        )]),
        sources: vec![SnippetSource::Local {
            files: vec![String::from("./tests/samples/custom_prefix.rb")],
        }],
        output_dir: Some(dir.path().to_string_lossy().to_string()),
        output_extension: Some(String::from("md")),
        link_format: Some(LinkFormat::GitHub),
        source_link_prefix: Some("http://github.com/foo".into()),
        ..Default::default()
    })
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

    extract(SnippextSettings {
        templates: IndexMap::from([(
            DEFAULT_TEMPLATE_IDENTIFIER.to_string(),
            String::from(
                "```{{snippet}}```{{#unless omit_source_link}}\n{{source_link}}{{/unless}}",
            ),
        )]),
        sources: vec![SnippetSource::Local {
            files: vec![String::from("./tests/samples/custom_prefix.rb")],
        }],
        output_dir: Some(dir.path().to_string_lossy().to_string()),
        output_extension: Some(String::from("md")),
        link_format: Some(LinkFormat::GitHub),
        omit_source_links: true,
        ..Default::default()
    })
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

    extract(SnippextSettings {
        templates: IndexMap::from([(
            DEFAULT_TEMPLATE_IDENTIFIER.to_string(),
            String::from(
                "```{{snippet}}```{{#unless omit_source_link}}\n{{source_link}}{{/unless}}",
            ),
        )]),
        sources: vec![SnippetSource::Local {
            files: vec![String::from("./tests/main.cs")],
        }],
        output_dir: Some(dir.path().to_string_lossy().to_string()),
        output_extension: Some(String::from("md")),
        link_format: Some(LinkFormat::GitHub),
        ..Default::default()
    })
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

    extract(SnippextSettings {
        templates: IndexMap::from([(
            DEFAULT_TEMPLATE_IDENTIFIER.to_string(),
            String::from("```{{lang}}\n{{snippet}}```\n"),
        )]),
        sources: vec![SnippetSource::Local {
            files: vec![String::from("./tests/samples/main.rs")],
        }],
        output_dir: Some(dir.path().to_string_lossy().to_string()),
        retain_nested_snippet_comments: true,
        output_extension: Some(String::from("md")),
        ..Default::default()
    })
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

#[test]
fn should_retain_nested_snippet_comments_for_individual_snippets() {
    let dir = tempdir().unwrap();

    extract(SnippextSettings {
        templates: IndexMap::from([(
            DEFAULT_TEMPLATE_IDENTIFIER.to_string(),
            String::from("```{{lang}}\n{{snippet}}```\n"),
        )]),
        sources: vec![SnippetSource::Local {
            files: vec![String::from("./tests/samples/retain_nested_comments.rs")],
        }],
        output_dir: Some(dir.path().to_string_lossy().to_string()),
        retain_nested_snippet_comments: false,
        output_extension: Some(String::from("md")),
        ..Default::default()
    })
    .unwrap();

    let retain_actual = fs::read_to_string(
        Path::new(&dir.path()).join("tests/samples/retain_nested_comments.rs/retain_default.md"),
    )
    .unwrap();
    let retain_expected = r#"```rust
fn main() {

    // snippet::start loop { "retain_nested_snippet_comments": false }
    for n in 1..10 {
        // snippet::start print
        println!("printing...")
        // snippet::end
    }
    // snippet::end
}
```
"#;
    assert_eq!(retain_expected, retain_actual);

    let loop_actual = fs::read_to_string(
        Path::new(&dir.path()).join("tests/samples/retain_nested_comments.rs/loop_default.md"),
    )
    .unwrap();
    let loop_expected = r#"```rust
for n in 1..10 {
    println!("printing...")
}
```
"#;
    assert_eq!(loop_expected, loop_actual);
}

#[test]
fn should_allow_custom_highlighted_attributes() {
    let dir = tempdir().unwrap();
    let target = Path::new(&dir.path()).join("highlighted_lines.md");
    fs::copy(Path::new("./tests/targets/highlighted_lines.md"), &target).unwrap();

    extract(SnippextSettings {
        templates: IndexMap::from([(
            DEFAULT_TEMPLATE_IDENTIFIER.to_string(),
            String::from("```{{lang}} hl_lines=\"{{highlighted_lines}}\"\n{{snippet}}```\n"),
        )]),
        sources: vec![SnippetSource::Local {
            files: vec![String::from("./tests/samples/main.rs")],
        }],
        targets: Some(vec![target.to_string_lossy().to_string()]),
        ..Default::default()
    })
    .unwrap();

    let actual = fs::read_to_string(target).unwrap();
    let expected = r#"Highlighted Lines
<!-- snippet::start main { "highlighted_lines": "1 3-5" } -->
```rust hl_lines="1 3-5"
fn main() {

    println!("printing...")
}
```
<!-- snippet::end -->"#;
    assert_eq!(expected, actual);
}

#[test]
fn should_autodetect_language() {
    let dir = tempdir().unwrap();

    extract(SnippextSettings {
        templates: IndexMap::from([(
            DEFAULT_TEMPLATE_IDENTIFIER.to_string(),
            String::from("```{{lang}}\n{{snippet}}```\n"),
        )]),
        sources: vec![SnippetSource::Local {
            files: vec![String::from("./tests/samples/custom_prefix.rb")],
        }],
        output_dir: Some(dir.path().to_string_lossy().to_string()),
        retain_nested_snippet_comments: true,
        output_extension: Some(String::from("md")),
        ..Default::default()
    })
    .unwrap();

    let actual = fs::read_to_string(
        Path::new(&dir.path()).join("tests/samples/custom_prefix.rb/ruby_default.md"),
    )
    .unwrap();
    let expected = r#"```ruby
puts "Hello, Ruby!"
```
"#;
    assert_eq!(expected, actual);
}

#[test]
fn should_support_disabling_autodetect_language() {
    let dir = tempdir().unwrap();

    extract(SnippextSettings {
        templates: IndexMap::from([(
            DEFAULT_TEMPLATE_IDENTIFIER.to_string(),
            String::from("```{{lang}}\n{{snippet}}```\n"),
        )]),
        sources: vec![SnippetSource::Local {
            files: vec![String::from("./tests/samples/custom_prefix.rb")],
        }],
        output_dir: Some(dir.path().to_string_lossy().to_string()),
        retain_nested_snippet_comments: true,
        output_extension: Some(String::from("md")),
        enable_autodetect_language: false,
        ..Default::default()
    })
    .unwrap();

    let actual = fs::read_to_string(
        Path::new(&dir.path()).join("tests/samples/custom_prefix.rb/ruby_default.md"),
    )
    .unwrap();
    let expected = r#"```
puts "Hello, Ruby!"
```
"#;
    assert_eq!(expected, actual);
}
