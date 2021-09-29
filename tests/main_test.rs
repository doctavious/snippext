use snippext::extract;

use tempfile::tempdir;
use std::fs::File;
use std::io::{self, Write};
use walkdir::WalkDir;
use std::fs;
use std::path::Path;

//TODO: begin / end cant be empty

#[test]
fn test() {

    // Create a directory inside of `std::env::temp_dir()`.
    let dir = tempdir().unwrap();

    extract(
        String::from("// "),
        String::from("snippet::"),
        String::from("end::"),
        dir.path().to_string_lossy().to_string(),
        String::from("md"),
        String::from("{{snippet}}"),
        vec![String::from("./tests/samples")],
    );

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

    extract(
        String::from("# "),
        String::from("snippet::"),
        String::from("end::"),
        dir.path().to_string_lossy().to_string(),
        String::from("md"),
        String::from("{{snippet}}"),
        vec![String::from("./tests/samples/custom_prefix.rb")]
    );

    let content =
        fs::read_to_string(Path::new(&dir.path()).join("tests/samples/custom_prefix.rb/ruby.md")).unwrap();

    assert_eq!("puts \"Hello, Ruby!\"\n", content);

    dir.close().unwrap();
}
