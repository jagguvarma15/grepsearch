# grepsearch

Index-free, grep-based retrieval primitives for AI agent tool loops. The engine is the ripgrep search core, compiled into a native Python extension, so every query runs at Rust speed with no subprocess overhead and no index to build or sync.

Grep-based retrieval works by running search tools in a reasoning loop instead of pre-embedding a corpus into a vector index. Every query reads the live filesystem, so results are never stale. This package provides the search side of that loop; your agent framework provides the reasoning.

## Installation

```sh
pip install grepsearch
```

Prebuilt wheels cover Linux (x86_64, aarch64), macOS (Apple silicon, Intel), and Windows (x64) for Python 3.9 and newer.

## Usage

```python
from grepsearch import SearchEngine

engine = SearchEngine("path/to/repo")

# Content search with budgets sized for a context window
result = engine.grep(
    "fn authenticate",
    literal=True,
    context_after=5,
    max_results=40,
    max_bytes=32_000,
)
for file in result.files:
    for match in file.matches:
        print(f"{file.path}:{match.line_number}: {match.line}")

# The result serializes straight into a tool output for a model
tool_output = result.to_json()

# Follow up around a promising hit
piece = engine.read_lines("src/auth.rs", 40, 80)

# Orient in the repository
for entry in engine.list_dir(".", max_depth=2):
    print(entry.path)

# Find files by name
hits = engine.find_files("**/*.toml")
```

Every grep result carries a `truncated` flag. When a budget cap stops the search early, the flag tells the caller that more matches may exist, which prevents an agent from mistaking a capped result for a complete one.

## Behavior

- Gitignore, ignore, and hidden file rules are honored by default; disable per engine with `respect_gitignore=False` or `include_hidden=True`
- Binary files are detected and skipped
- Searches run in parallel across files and release the interpreter lock while running
- Result ordering is deterministic: files with more matches first, then shallower paths, then lexicographic
- Errors map to familiar Python exceptions: `ValueError` for bad patterns, globs, and line ranges; `FileNotFoundError` for missing paths

## Rust

The same engine ships as a Rust crate for agents and CLIs built in Rust: https://crates.io/crates/grepsearch

## License

MIT
