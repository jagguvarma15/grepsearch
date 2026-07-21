use grepsearch::{EngineConfig, Error, GrepQuery, SearchEngine};
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
fn literal_search_finds_exact_string() {
    let result = engine()
        .grep(&GrepQuery {
            literal: true,
            ..GrepQuery::new("fn authenticate")
        })
        .unwrap();
    assert_eq!(result.total_matches, 1);
    assert!(!result.truncated);
    assert_eq!(result.files.len(), 1);
    assert_eq!(result.files[0].path, PathBuf::from("src/main.rs"));
    assert_eq!(result.files[0].matches[0].line_number, 10);
    assert!(
        result.files[0].matches[0]
            .line
            .starts_with("fn authenticate")
    );
}

#[test]
fn regex_search_matches_pattern() {
    let result = engine().grep(&GrepQuery::new(r"fn \w+_user")).unwrap();
    assert_eq!(result.total_matches, 2);
    assert_eq!(result.files.len(), 1);
    assert_eq!(result.files[0].path, PathBuf::from("src/lib.rs"));
    let lines: Vec<usize> = result.files[0]
        .matches
        .iter()
        .map(|m| m.line_number)
        .collect();
    assert_eq!(lines, vec![1, 5]);
}

#[test]
fn no_match_returns_empty_result() {
    let result = engine().grep(&GrepQuery::new("zzz_not_present")).unwrap();
    assert!(result.files.is_empty());
    assert_eq!(result.total_matches, 0);
    assert!(!result.truncated);
}

#[test]
fn invalid_pattern_is_reported() {
    let err = engine().grep(&GrepQuery::new("(unclosed")).unwrap_err();
    assert!(matches!(err, Error::Pattern(_)));
}

#[test]
fn case_insensitive_search_ignores_case() {
    let sensitive = engine()
        .grep(&GrepQuery {
            literal: true,
            ..GrepQuery::new("AUTHENTICATE")
        })
        .unwrap();
    assert_eq!(sensitive.total_matches, 0);

    let insensitive = engine()
        .grep(&GrepQuery {
            literal: true,
            case_insensitive: true,
            ..GrepQuery::new("AUTHENTICATE")
        })
        .unwrap();
    assert_eq!(insensitive.total_matches, 5);
}

#[test]
fn before_context_at_file_start_is_empty() {
    let result = engine()
        .grep(&GrepQuery {
            literal: true,
            context_before: 3,
            ..GrepQuery::new("pub fn add_user")
        })
        .unwrap();
    let m = &result.files[0].matches[0];
    assert_eq!(m.line_number, 1);
    assert!(m.before.is_empty());
}

#[test]
fn after_context_at_file_end_is_empty() {
    let result = engine()
        .grep(&GrepQuery {
            literal: true,
            context_after: 3,
            ..GrepQuery::new("end of module")
        })
        .unwrap();
    let m = &result.files[0].matches[0];
    assert_eq!(m.line_number, 8);
    assert!(m.after.is_empty());
}

#[test]
fn context_lines_surround_the_match() {
    let result = engine()
        .grep(&GrepQuery {
            literal: true,
            context_before: 2,
            context_after: 1,
            ..GrepQuery::new("pub fn remove_user")
        })
        .unwrap();
    let m = &result.files[0].matches[0];
    assert_eq!(m.line_number, 5);
    assert_eq!(m.before, vec!["}".to_string(), String::new()]);
    assert_eq!(m.after, vec!["    !name.is_empty()".to_string()]);
}

#[test]
fn gitignored_files_are_skipped_by_default() {
    let result = engine()
        .grep(&GrepQuery {
            literal: true,
            ..GrepQuery::new("cached artifact")
        })
        .unwrap();
    assert_eq!(result.total_matches, 0);
}

#[test]
fn disabling_gitignore_surfaces_ignored_files() {
    let engine = SearchEngine::new(EngineConfig {
        root: fixture_root(),
        respect_gitignore: false,
        ..EngineConfig::default()
    })
    .unwrap();
    let result = engine
        .grep(&GrepQuery {
            literal: true,
            ..GrepQuery::new("cached artifact")
        })
        .unwrap();
    assert_eq!(result.total_matches, 1);
    assert_eq!(result.files[0].path, PathBuf::from("build/cache.txt"));
}

#[test]
fn hidden_files_are_skipped_by_default() {
    let result = engine()
        .grep(&GrepQuery {
            literal: true,
            ..GrepQuery::new("hidden entry")
        })
        .unwrap();
    assert_eq!(result.total_matches, 0);
}

#[test]
fn including_hidden_files_surfaces_them() {
    let engine = SearchEngine::new(EngineConfig {
        root: fixture_root(),
        include_hidden: true,
        ..EngineConfig::default()
    })
    .unwrap();
    let result = engine
        .grep(&GrepQuery {
            literal: true,
            ..GrepQuery::new("hidden entry")
        })
        .unwrap();
    assert_eq!(result.total_matches, 1);
    assert_eq!(result.files[0].path, PathBuf::from(".hidden.txt"));
}

#[test]
fn globs_restrict_the_searched_files() {
    let result = engine()
        .grep(&GrepQuery {
            literal: true,
            globs: vec!["*.rs".to_string()],
            ..GrepQuery::new("authenticate")
        })
        .unwrap();
    assert_eq!(result.files.len(), 1);
    assert_eq!(result.files[0].path, PathBuf::from("src/main.rs"));
    assert_eq!(result.total_matches, 2);
}

#[test]
fn negated_globs_exclude_files() {
    let result = engine()
        .grep(&GrepQuery {
            literal: true,
            globs: vec!["!*.md".to_string()],
            ..GrepQuery::new("authenticate")
        })
        .unwrap();
    let paths: Vec<&PathBuf> = result.files.iter().map(|f| &f.path).collect();
    assert!(!paths.contains(&&PathBuf::from("notes.md")));
    assert_eq!(result.total_matches, 3);
}

#[test]
fn binary_files_are_skipped() {
    let result = engine()
        .grep(&GrepQuery {
            literal: true,
            ..GrepQuery::new("binary payload")
        })
        .unwrap();
    assert_eq!(result.total_matches, 0);
}

#[test]
fn max_results_caps_matches_and_sets_truncated() {
    let result = engine()
        .grep(&GrepQuery {
            literal: true,
            max_results: 2,
            ..GrepQuery::new("authenticate")
        })
        .unwrap();
    assert!(result.total_matches <= 2);
    assert!(result.truncated);
}

#[test]
fn max_bytes_caps_output_and_sets_truncated() {
    let result = engine()
        .grep(&GrepQuery {
            literal: true,
            max_bytes: 20,
            ..GrepQuery::new("authenticate")
        })
        .unwrap();
    assert!(result.truncated);
    let returned: usize = result
        .files
        .iter()
        .flat_map(|f| f.matches.iter())
        .map(|m| m.line.len())
        .sum();
    assert!(returned <= 20);
}

#[test]
fn long_lines_are_truncated_to_max_line_len() {
    let result = engine()
        .grep(&GrepQuery {
            literal: true,
            globs: vec!["*.md".to_string()],
            max_line_len: 32,
            ..GrepQuery::new("authenticate")
        })
        .unwrap();
    assert!(result.total_matches > 0);
    for file in &result.files {
        for m in &file.matches {
            assert!(m.line.len() <= 32);
        }
    }
}

#[test]
fn results_are_ordered_by_match_count_then_depth() {
    let result = engine()
        .grep(&GrepQuery {
            literal: true,
            ..GrepQuery::new("authenticate")
        })
        .unwrap();
    let paths: Vec<PathBuf> = result.files.iter().map(|f| f.path.clone()).collect();
    assert_eq!(
        paths,
        vec![
            PathBuf::from("notes.md"),
            PathBuf::from("src/main.rs"),
            PathBuf::from("script.py"),
        ]
    );
    assert_eq!(result.total_matches, 5);
}

#[test]
fn missing_root_is_rejected() {
    let err = SearchEngine::new(EngineConfig {
        root: fixture_root().join("does-not-exist"),
        ..EngineConfig::default()
    })
    .unwrap_err();
    assert!(matches!(err, Error::NotFound(_)));
}

#[test]
fn gitignore_is_honored_outside_a_git_repository() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join(".gitignore"), "secret.txt\n").unwrap();
    std::fs::write(dir.path().join("secret.txt"), "token alpha\n").unwrap();
    std::fs::write(dir.path().join("visible.txt"), "token beta\n").unwrap();

    let engine = SearchEngine::new(EngineConfig {
        root: dir.path().to_path_buf(),
        ..EngineConfig::default()
    })
    .unwrap();
    let result = engine
        .grep(&GrepQuery {
            literal: true,
            ..GrepQuery::new("token")
        })
        .unwrap();
    assert_eq!(result.total_matches, 1);
    assert_eq!(result.files[0].path, PathBuf::from("visible.txt"));
}

#[test]
fn grep_result_serialization_is_stable() {
    let result = engine()
        .grep(&GrepQuery {
            literal: true,
            context_after: 1,
            ..GrepQuery::new("authenticate")
        })
        .unwrap();
    insta::assert_json_snapshot!("grep_authenticate", result);
}
