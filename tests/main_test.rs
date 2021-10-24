use snippext::{run, SnippextSettings, SnippetSource};

use tempfile::tempdir;


use std::fs;
use std::path::{Path, PathBuf};
use snippext::error::SnippextError;

#[test]
fn should_successfully_extract_from_local_sources_directory() {
    let dir = tempdir().unwrap();

    run(SnippextSettings::new(
        vec![String::from("// ")],
        String::from("snippet::"),
        String::from("end::"),
        String::from("md"),
        String::from("{{snippet}}"),
        vec![SnippetSource::new_local(vec![String::from("./tests/samples/*")])],
        Some(dir.path().to_string_lossy().to_string()),
        None
    )).unwrap();

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
        fs::read_to_string(Path::new(&dir.path()).join("tests/samples/sample_file.rs/fn_1.md")).unwrap();
    let sample_fn_1_content_expected = r#"fn sample_fn_1() {

}
"#;
    assert_eq!(sample_fn_1_content_expected, sample_fn_1_content_actual);

    let sample_fn_2_content_actual =
        fs::read_to_string(Path::new(&dir.path()).join("tests/samples/sample_file.rs/fn_2.md")).unwrap();
    let sample_fn_2_content_expected = r#"fn sample_fn_2() {

}
"#;
    assert_eq!(sample_fn_2_content_expected, sample_fn_2_content_actual);

    dir.close().unwrap();
}

// TODO: test extracting from remote source


#[test]
fn should_successfully_extract_from_remote() {
    let dir = tempdir().unwrap();

    run(SnippextSettings::new(
        vec![String::from("// ")],
        String::from("snippet::"),
        String::from("end::"),
        String::from("md"),
        String::from("{{snippet}}"),
        vec![SnippetSource::new_remote(
            String::from("https://github.com/doctavious/snippext.git"),
            String::from("main"),
            None,
            Some(format!("{}/snippext/", dir.path().to_string_lossy())),
            vec![String::from("/tests/**/*")]
        )],
        Some(format!("{}/generated-snippets/", dir.path().to_string_lossy())),
        None
    )).unwrap();

    let main_content_actual =
        fs::read_to_string(Path::new(&dir.path()).join("generated-snippets/tests/samples/main.rs/main.md")).unwrap();
    let main_content_expected = r#"fn main() {

    println!("printing...")
}
"#;
    assert_eq!(main_content_expected, main_content_actual);

    dir.close().unwrap();
}

#[test]
fn should_successfully_extract_from_local_sources_file() {
    let dir = tempdir().unwrap();

    run(SnippextSettings::new(
        vec![String::from("# ")],
        String::from("snippet::"),
        String::from("end::"),
        String::from("md"),
        String::from("{{snippet}}"),
        vec![SnippetSource::new_local(vec![String::from("./tests/samples/custom_prefix.rb")])],
        Some(dir.path().to_string_lossy().to_string()),
        None,
    )).unwrap();

    let content =
        fs::read_to_string(Path::new(&dir.path()).join("tests/samples/custom_prefix.rb/ruby.md")).unwrap();

    assert_eq!("puts \"Hello, Ruby!\"\n", content);

    dir.close().unwrap();
}

#[test]
fn should_support_template_with_attributes() {
    let dir = tempdir().unwrap();

    run(SnippextSettings::new(
        vec![String::from("// ")],
        String::from("snippet::"),
        String::from("end::"),
        String::from("md"),
        String::from("```{{lang}}\n{{snippet}}```\n"),
        vec![SnippetSource::new_local(vec![String::from("./tests/samples/main.rs")])],
        Some(dir.path().to_string_lossy().to_string()),
        None,
    )).unwrap();

    let actual =
        fs::read_to_string(Path::new(&dir.path()).join("tests/samples/main.rs/main.md")).unwrap();
    let expected = r#"```rust
fn main() {

    println!("printing...")
}
```
"#;
    assert_eq!(expected, actual);

    dir.close().unwrap();
}

#[test]
fn should_treat_unknown_template_variables_as_empty_string() {
    let dir = tempdir().unwrap();

    run(SnippextSettings::new(
        vec![String::from("// ")],
        String::from("snippet::"),
        String::from("end::"),
        String::from("md"),
        String::from("```{{unknown}}\n{{snippet}}```\n"),
        vec![SnippetSource::new_local(vec![String::from("./tests/samples/main.rs")])],
        Some(dir.path().to_string_lossy().to_string()),
        None,
    )).unwrap();

    let actual =
        fs::read_to_string(Path::new(&dir.path()).join("tests/samples/main.rs/main.md")).unwrap();
    let expected = r#"```
fn main() {

    println!("printing...")
}
```
"#;
    assert_eq!(expected, actual);

    dir.close().unwrap();
}

#[test]
fn should_support_files_with_no_snippets() {
    let dir = tempdir().unwrap();

    run(SnippextSettings::new(
        vec![String::from("// ")],
        String::from("snippet::"),
        String::from("end::"),
        String::from("md"),
        String::from("```{{unknown}}\n{{snippet}}```\n"),
        vec![SnippetSource::new_local(vec![String::from("./tests/samples/no_snippets.rs")])],
        Some(dir.path().to_string_lossy().to_string()),
        None,
    )).unwrap();

    let files: Vec<PathBuf> = fs::read_dir(&dir).unwrap()
        .into_iter()
        .filter(|r| r.is_ok())
        .map(|r| r.unwrap().path())
        .filter(|r| r.is_dir())
        .collect();

    assert_eq!(0, files.len());

    dir.close().unwrap();
}

#[test]
fn invalid_glob() {
    let dir = tempdir().unwrap();

    let result = run(SnippextSettings::new(
        vec![String::from("// ")],
        String::from("snippet::"),
        String::from("end::"),
        String::from("md"),
        String::from("```{{unknown}}\n{{snippet}}```\n"),
        vec![SnippetSource::new_local(vec![String::from("[&")])],
        Some(dir.path().to_string_lossy().to_string()),
        None,
    ));

    dir.close().unwrap();

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
        vec![String::from("// ")],
        String::from("snippet::"),
        String::from("end::"),
        String::from("md"),
        String::from("```{{unknown}}\n{{snippet}}```\n"),
        vec![SnippetSource::new_local(vec![String::from("./tests/samples/*.md")])],
        Some(dir.path().to_string_lossy().to_string()),
        None,
    )).unwrap();

    let files: Vec<PathBuf> = fs::read_dir(&dir).unwrap()
        .into_iter()
        .filter(|r| r.is_ok())
        .map(|r| r.unwrap().path())
        .filter(|r| r.is_dir())
        .collect();

    assert_eq!(0, files.len());

    dir.close().unwrap();
}

