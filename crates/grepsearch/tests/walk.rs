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
fn shallow_listing_shows_top_level_entries() {
    let entries = engine().list_dir(Path::new("."), 1).unwrap();
    let paths: Vec<PathBuf> = entries.iter().map(|e| e.path.clone()).collect();
    assert_eq!(
        paths,
        vec![
            PathBuf::from("binary.bin"),
            PathBuf::from("empty.txt"),
            PathBuf::from("notes.md"),
            PathBuf::from("script.py"),
            PathBuf::from("src"),
        ]
    );
    let src = entries.iter().find(|e| e.path.ends_with("src")).unwrap();
    assert!(src.is_dir);
    assert_eq!(src.depth, 1);
}

#[test]
fn deeper_listing_includes_nested_files() {
    let entries = engine().list_dir(Path::new("."), 2).unwrap();
    let paths: Vec<PathBuf> = entries.iter().map(|e| e.path.clone()).collect();
    assert!(paths.contains(&PathBuf::from("src/lib.rs")));
    assert!(paths.contains(&PathBuf::from("src/main.rs")));
    let nested = entries
        .iter()
        .find(|e| e.path == Path::new("src/main.rs"))
        .unwrap();
    assert_eq!(nested.depth, 2);
    assert!(!nested.is_dir);
}

#[test]
fn listing_a_subdirectory_works() {
    let entries = engine().list_dir(Path::new("src"), 1).unwrap();
    let paths: Vec<PathBuf> = entries.iter().map(|e| e.path.clone()).collect();
    assert_eq!(
        paths,
        vec![PathBuf::from("src/lib.rs"), PathBuf::from("src/main.rs")]
    );
}

#[test]
fn gitignored_directories_are_hidden_from_listings() {
    let entries = engine().list_dir(Path::new("."), 2).unwrap();
    assert!(entries.iter().all(|e| !e.path.starts_with("build")));
}

#[test]
fn disabling_gitignore_reveals_ignored_directories() {
    let engine = SearchEngine::new(EngineConfig {
        root: fixture_root(),
        respect_gitignore: false,
        ..EngineConfig::default()
    })
    .unwrap();
    let entries = engine.list_dir(Path::new("."), 2).unwrap();
    let paths: Vec<PathBuf> = entries.iter().map(|e| e.path.clone()).collect();
    assert!(paths.contains(&PathBuf::from("build")));
    assert!(paths.contains(&PathBuf::from("build/cache.txt")));
}

#[test]
fn hidden_files_appear_only_when_requested() {
    let default_entries = engine().list_dir(Path::new("."), 1).unwrap();
    assert!(
        default_entries
            .iter()
            .all(|e| !e.path.ends_with(".hidden.txt"))
    );

    let engine = SearchEngine::new(EngineConfig {
        root: fixture_root(),
        include_hidden: true,
        ..EngineConfig::default()
    })
    .unwrap();
    let entries = engine.list_dir(Path::new("."), 1).unwrap();
    let paths: Vec<PathBuf> = entries.iter().map(|e| e.path.clone()).collect();
    assert!(paths.contains(&PathBuf::from(".hidden.txt")));
    assert!(paths.contains(&PathBuf::from(".gitignore")));
}

#[test]
fn missing_directory_is_rejected() {
    let err = engine().list_dir(Path::new("no-such-dir"), 1).unwrap_err();
    assert!(matches!(err, Error::NotFound(_)));
}

#[test]
fn listing_a_file_is_rejected() {
    let err = engine().list_dir(Path::new("notes.md"), 1).unwrap_err();
    assert!(matches!(err, Error::NotFound(_)));
}
