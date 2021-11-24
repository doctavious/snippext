use std::collections::{HashMap, HashSet};
use snippext::{run, SnippetSource, SnippextSettings, SnippextTemplate};

use tempfile::tempdir;

use snippext::error::SnippextError;
use std::fs;
use std::path::{Path, PathBuf};

#[test]
fn should_successfully_extract_from_local_sources_directory() {
    let dir = tempdir().unwrap();

    run(SnippextSettings::new(
        HashSet::from([String::from("// ")]),
        String::from("snippet::"),
        String::from("end::"),
        String::from("md"),
        HashMap::from([
            ("default".to_string(), SnippextTemplate {
                content: String::from("{{snippet}}"),
                default: true
            }),
        ]),
        vec![SnippetSource::new_local(vec![String::from(
            "./tests/samples/*",
        )])],
        Some(dir.path().to_string_lossy().to_string()),
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
fn should_successfully_extract_from_remote() {
    let dir = tempdir().unwrap();

    run(SnippextSettings::new(
        HashSet::from([String::from("// ")]),
        String::from("snippet::"),
        String::from("end::"),
        String::from("md"),
        HashMap::from([
            ("default".to_string(), SnippextTemplate {
                content: String::from("{{snippet}}"),
                default: true
            }),
        ]),
        vec![SnippetSource::new_remote(
            String::from("https://github.com/doctavious/snippext.git"),
            String::from("main"),
            None,
            Some(format!("{}/snippext/", dir.path().to_string_lossy())),
            vec![String::from("/tests/**/*")],
        )],
        Some(format!(
            "{}/generated-snippets/",
            dir.path().to_string_lossy()
        )),
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

    run(SnippextSettings::new(
        HashSet::from([String::from("# ")]),
        String::from("snippet::"),
        String::from("end::"),
        String::from("md"),
        HashMap::from([
            ("default".to_string(), SnippextTemplate {
                content: String::from("{{snippet}}"),
                default: true
            }),
        ]),
        vec![SnippetSource::new_local(vec![String::from(
            "./tests/samples/custom_prefix.rb",
        )])],
        Some(dir.path().to_string_lossy().to_string()),
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

    run(SnippextSettings::new(
        HashSet::from([String::from("// "), String::from("<!-- ")]),
        String::from("snippet::"),
        String::from("end::"),
        String::from("md"),
        HashMap::from([
            ("default".to_string(), SnippextTemplate {
                content: String::from("{{snippet}}"),
                default: true
            }),
        ]),
        vec![SnippetSource::new_local(vec![String::from(
            "./tests/samples/*",
        )])],
        Some(dir.path().to_string_lossy().to_string()),
        Some(vec![Path::new(&dir.path())
            .join("./target.md")
            .to_string_lossy()
            .to_string()]),
    ))
    .unwrap();

    let actual = fs::read_to_string(Path::new(&dir.path()).join("./target.md")).unwrap();
    let expected = r#"This is some static content

<!-- snippet::main -->
fn main() {

    println!("printing...")
}
<!-- end::main -->

<!-- snippet::fn_1 -->
fn sample_fn_1() {

}
<!-- end::fn_1 -->
"#;
    assert_eq!(expected, actual);
}

#[test]
fn should_support_template_with_attributes() {
    let dir = tempdir().unwrap();

    run(SnippextSettings::new(
        HashSet::from([String::from("// ")]),
        String::from("snippet::"),
        String::from("end::"),
        String::from("md"),
        HashMap::from([
            ("default".to_string(), SnippextTemplate {
                content: String::from("```{{lang}}\n{{snippet}}```\n"),
                default: true
            }),
        ]),
        vec![SnippetSource::new_local(vec![String::from(
            "./tests/samples/main.rs",
        )])],
        Some(dir.path().to_string_lossy().to_string()),
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


    run(SnippextSettings::new(
        HashSet::from([String::from("// "), String::from("<!-- ")]),
        String::from("snippet::"),
        String::from("end::"),
        String::from("md"),
        HashMap::from([
            ("basic".to_string(), SnippextTemplate {
                content: String::from("{{snippet}}"),
                default: true
            }),
            ("code".to_string(), SnippextTemplate {
                content: String::from("```{{lang}}\n{{snippet}}```\n"),
                default: false
            }),
        ]),
        vec![SnippetSource::new_local(vec![String::from(
            "./tests/samples/main.rs",
        )])],
        None,
        Some(vec![Path::new(&dir.path())
            .join("./specify_template.md")
            .to_string_lossy()
            .to_string()]),

    ))
        .unwrap();

    let actual = fs::read_to_string(Path::new(&dir.path()).join("./specify_template.md")).unwrap();
    let expected = r#"Specify template
<!-- snippet::main[snippext_template=code] -->
```rust
fn main() {

    println!("printing...")
}
```
<!-- end::main -->
"#;
    assert_eq!(expected, actual);
}

#[test]
fn should_treat_unknown_template_variables_as_empty_string() {
    let dir = tempdir().unwrap();

    run(SnippextSettings::new(
        HashSet::from([String::from("// ")]),
        String::from("snippet::"),
        String::from("end::"),
        String::from("md"),
        HashMap::from([
            ("default".to_string(), SnippextTemplate {
                content: String::from("```{{unknown}}\n{{snippet}}```\n"),
                default: true
            }),
        ]),
        vec![SnippetSource::new_local(vec![String::from(
            "./tests/samples/main.rs",
        )])],
        Some(dir.path().to_string_lossy().to_string()),
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

    run(SnippextSettings::new(
        HashSet::from([String::from("// ")]),
        String::from("snippet::"),
        String::from("end::"),
        String::from("md"),
        HashMap::from([
            ("default".to_string(), SnippextTemplate {
                content: String::from("{{snippet}}"),
                default: true
            }),
        ]),
        vec![SnippetSource::new_local(vec![String::from(
            "./tests/samples/no_snippets.rs",
        )])],
        Some(dir.path().to_string_lossy().to_string()),
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

    let result = run(SnippextSettings::new(
        HashSet::from([String::from("// ")]),
        String::from("snippet::"),
        String::from("end::"),
        String::from("md"),
        HashMap::from([
            ("default".to_string(), SnippextTemplate {
                content: String::from("{{snippet}}"),
                default: true
            }),
        ]),
        vec![SnippetSource::new_local(vec![String::from("[&")])],
        Some(dir.path().to_string_lossy().to_string()),
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

    run(SnippextSettings::new(
        HashSet::from([String::from("// ")]),
        String::from("snippet::"),
        String::from("end::"),
        String::from("md"),
        HashMap::from([
            ("default".to_string(), SnippextTemplate {
                content: String::from("{{snippet}}"),
                default: true
            }),
        ]),
        vec![SnippetSource::new_local(vec![String::from(
            "./tests/samples/*.md",
        )])],
        Some(dir.path().to_string_lossy().to_string()),
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
