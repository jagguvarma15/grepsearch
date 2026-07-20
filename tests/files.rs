use grepsearch::{EngineConfig, Error, FileQuery, SearchEngine};
use std::path::PathBuf;

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
fn glob_finds_files_at_any_depth() {
    let hits = engine().find_files(&FileQuery::new("*.rs")).unwrap();
    let paths: Vec<PathBuf> = hits.iter().map(|h| h.path.clone()).collect();
    assert_eq!(
        paths,
        vec![PathBuf::from("src/lib.rs"), PathBuf::from("src/main.rs")]
    );
}

#[test]
fn recursive_glob_finds_a_specific_file() {
    let hits = engine().find_files(&FileQuery::new("**/main.rs")).unwrap();
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].path, PathBuf::from("src/main.rs"));
}

#[test]
fn glob_matching_nothing_returns_empty() {
    let hits = engine().find_files(&FileQuery::new("*.zig")).unwrap();
    assert!(hits.is_empty());
}

#[test]
fn invalid_glob_is_reported() {
    let err = engine().find_files(&FileQuery::new("a{b")).unwrap_err();
    assert!(matches!(err, Error::Glob(_)));
}

#[test]
fn max_results_caps_the_file_list() {
    let hits = engine()
        .find_files(&FileQuery {
            glob: String::from("**"),
            max_results: 2,
        })
        .unwrap();
    assert_eq!(hits.len(), 2);
}

#[test]
fn file_metadata_is_populated() {
    let hits = engine().find_files(&FileQuery::new("notes.md")).unwrap();
    assert_eq!(hits.len(), 1);
    assert!(hits[0].bytes > 0);
    assert!(hits[0].modified.is_some());
}

#[test]
fn gitignored_files_are_not_listed_by_default() {
    let hits = engine().find_files(&FileQuery::new("cache.txt")).unwrap();
    assert!(hits.is_empty());
}

#[test]
fn disabling_gitignore_lists_ignored_files() {
    let engine = SearchEngine::new(EngineConfig {
        root: fixture_root(),
        respect_gitignore: false,
        ..EngineConfig::default()
    })
    .unwrap();
    let hits = engine.find_files(&FileQuery::new("cache.txt")).unwrap();
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].path, PathBuf::from("build/cache.txt"));
}
