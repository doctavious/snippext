use snippext::{run, SnippextSettings, SnippetSource};

use tempfile::tempdir;
use std::fs::File;
use std::io::{self, Write};
use std::fs;
use std::path::Path;

// TODO: begin / end cant be empty

#[test]
fn test() {
    let dir = tempdir().unwrap();

    let result = run(SnippextSettings::new(
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

#[test]
fn test_custom_prefix() {
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
fn test_custom_template() {
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

// TODO: add test where var is not in context

// TODO: add test where no snippets found

// TODO: add test for invalid glob

// TODO: add test for glob that returns no files

// TODO: test required args/flags
