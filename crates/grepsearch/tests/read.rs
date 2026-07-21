use grepsearch::{EngineConfig, Error, SearchEngine};
use std::path::{Path, PathBuf};

fn fixture_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
}

fn engine() -> SearchEngine {
    SearchEngine::new(EngineConfig {
        root: fixture_root(),
        ..EngineConfig::default()
    })
    .unwrap()
}

#[test]
fn reads_an_exact_line_range() {
    let slice = engine().read_lines(Path::new("src/lib.rs"), 5, 6).unwrap();
    assert_eq!(slice.path, PathBuf::from("src/lib.rs"));
    assert_eq!(slice.start, 5);
    assert_eq!(slice.end, 6);
    assert_eq!(
        slice.content,
        "pub fn remove_user(name: &str) -> bool {\n    !name.is_empty()"
    );
}

#[test]
fn reads_the_first_line() {
    let slice = engine().read_lines(Path::new("src/lib.rs"), 1, 1).unwrap();
    assert_eq!(slice.content, "pub fn add_user(name: &str) -> bool {");
}

#[test]
fn end_past_eof_is_clamped() {
    let slice = engine()
        .read_lines(Path::new("src/lib.rs"), 7, 100)
        .unwrap();
    assert_eq!(slice.end, 8);
    assert_eq!(slice.content, "}\n// end of module");
}

#[test]
fn start_of_zero_is_rejected() {
    let err = engine()
        .read_lines(Path::new("src/lib.rs"), 0, 3)
        .unwrap_err();
    assert!(matches!(err, Error::InvalidRange { .. }));
}

#[test]
fn start_past_eof_is_rejected() {
    let err = engine()
        .read_lines(Path::new("src/lib.rs"), 50, 60)
        .unwrap_err();
    assert!(matches!(err, Error::InvalidRange { .. }));
}

#[test]
fn end_before_start_is_rejected() {
    let err = engine()
        .read_lines(Path::new("src/lib.rs"), 5, 2)
        .unwrap_err();
    assert!(matches!(err, Error::InvalidRange { .. }));
}

#[test]
fn missing_file_is_rejected() {
    let err = engine()
        .read_lines(Path::new("no-such-file.rs"), 1, 10)
        .unwrap_err();
    assert!(matches!(err, Error::NotFound(_)));
}

#[test]
fn empty_file_has_no_readable_lines() {
    let err = engine()
        .read_lines(Path::new("empty.txt"), 1, 1)
        .unwrap_err();
    assert!(matches!(err, Error::InvalidRange { .. }));
}
