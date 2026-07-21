import json
from pathlib import Path

import pytest

import grepsearch

FIXTURES = Path(__file__).resolve().parents[2] / "grepsearch" / "tests" / "fixtures"


@pytest.fixture
def engine():
    return grepsearch.SearchEngine(str(FIXTURES))


def test_version_is_exposed():
    assert grepsearch.__version__


def test_literal_search_finds_exact_string(engine):
    result = engine.grep("fn authenticate", literal=True)
    assert result.total_matches == 1
    assert not result.truncated
    assert result.files[0].path == "src/main.rs"
    assert result.files[0].matches[0].line_number == 10
    assert result.files[0].matches[0].line.startswith("fn authenticate")


def test_regex_search_matches_pattern(engine):
    result = engine.grep(r"fn \w+_user")
    assert result.total_matches == 2
    assert result.files[0].path == "src/lib.rs"
    assert [m.line_number for m in result.files[0].matches] == [1, 5]


def test_no_match_returns_empty_result(engine):
    result = engine.grep("zzz_not_present")
    assert result.files == []
    assert result.total_matches == 0
    assert not result.truncated


def test_invalid_pattern_raises_value_error(engine):
    with pytest.raises(ValueError):
        engine.grep("(unclosed")


def test_case_insensitive_search(engine):
    assert engine.grep("AUTHENTICATE", literal=True).total_matches == 0
    insensitive = engine.grep("AUTHENTICATE", literal=True, case_insensitive=True)
    assert insensitive.total_matches == 5


def test_context_lines_surround_the_match(engine):
    result = engine.grep(
        "pub fn remove_user", literal=True, context_before=2, context_after=1
    )
    match = result.files[0].matches[0]
    assert match.line_number == 5
    assert match.before == ["}", ""]
    assert match.after == ["    !name.is_empty()"]


def test_gitignored_files_are_skipped_by_default(engine):
    assert engine.grep("cached artifact", literal=True).total_matches == 0


def test_disabling_gitignore_surfaces_ignored_files():
    engine = grepsearch.SearchEngine(str(FIXTURES), respect_gitignore=False)
    result = engine.grep("cached artifact", literal=True)
    assert result.total_matches == 1
    assert result.files[0].path == "build/cache.txt"


def test_hidden_files_appear_only_when_requested():
    default_engine = grepsearch.SearchEngine(str(FIXTURES))
    assert default_engine.grep("hidden entry", literal=True).total_matches == 0
    hidden_engine = grepsearch.SearchEngine(str(FIXTURES), include_hidden=True)
    assert hidden_engine.grep("hidden entry", literal=True).total_matches == 1


def test_globs_restrict_the_searched_files(engine):
    result = engine.grep("authenticate", literal=True, globs=["*.rs"])
    assert [f.path for f in result.files] == ["src/main.rs"]
    assert result.total_matches == 2


def test_budget_cap_sets_truncated(engine):
    result = engine.grep("authenticate", literal=True, max_results=2)
    assert result.total_matches <= 2
    assert result.truncated


def test_long_lines_are_truncated(engine):
    result = engine.grep("authenticate", literal=True, globs=["*.md"], max_line_len=32)
    assert result.total_matches > 0
    for file in result.files:
        for match in file.matches:
            assert len(match.line.encode()) <= 32


def test_result_ordering_is_deterministic(engine):
    result = engine.grep("authenticate", literal=True)
    assert [f.path for f in result.files] == ["notes.md", "src/main.rs", "script.py"]


def test_grep_to_dict_and_to_json(engine):
    result = engine.grep("fn authenticate", literal=True)
    data = result.to_dict()
    assert data["total_matches"] == 1
    assert data["files"][0]["path"] == "src/main.rs"
    assert data == json.loads(result.to_json())


def test_missing_root_raises_file_not_found():
    with pytest.raises(FileNotFoundError):
        grepsearch.SearchEngine(str(FIXTURES / "does-not-exist"))


def test_find_files(engine):
    hits = engine.find_files("*.rs")
    assert [h.path for h in hits] == ["src/lib.rs", "src/main.rs"]
    assert all(h.bytes > 0 for h in hits)
    assert all(h.modified is None or h.modified > 0 for h in hits)


def test_find_files_empty_and_capped(engine):
    assert engine.find_files("*.zig") == []
    assert len(engine.find_files("**", max_results=2)) == 2


def test_invalid_glob_raises_value_error(engine):
    with pytest.raises(ValueError):
        engine.find_files("a{b")


def test_read_lines(engine):
    piece = engine.read_lines("src/lib.rs", 5, 6)
    assert piece.path == "src/lib.rs"
    assert piece.start == 5
    assert piece.end == 6
    assert piece.content == "pub fn remove_user(name: &str) -> bool {\n    !name.is_empty()"


def test_read_lines_clamps_end(engine):
    piece = engine.read_lines("src/lib.rs", 7, 100)
    assert piece.end == 8


def test_read_lines_errors(engine):
    with pytest.raises(ValueError):
        engine.read_lines("src/lib.rs", 0, 3)
    with pytest.raises(ValueError):
        engine.read_lines("src/lib.rs", 50, 60)
    with pytest.raises(FileNotFoundError):
        engine.read_lines("no-such-file.rs", 1, 10)


def test_list_dir(engine):
    entries = engine.list_dir(".", max_depth=1)
    assert [e.path for e in entries] == [
        "binary.bin",
        "empty.txt",
        "notes.md",
        "script.py",
        "src",
    ]
    src = entries[-1]
    assert src.is_dir
    assert src.depth == 1


def test_list_dir_missing_raises_file_not_found(engine):
    with pytest.raises(FileNotFoundError):
        engine.list_dir("no-such-dir")


def test_reprs_are_informative(engine):
    result = engine.grep("fn authenticate", literal=True)
    assert "GrepResult" in repr(result)
    assert "FileMatches" in repr(result.files[0])
    assert "LineMatch" in repr(result.files[0].matches[0])
    assert "SearchEngine" in repr(engine)


def test_find_files_comparison_uses_lists(engine):
    hits = engine.find_files("notes.md")
    assert len(hits) == 1
    assert hits[0].to_dict()["path"] == "notes.md"
