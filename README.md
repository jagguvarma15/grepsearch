# grepsearch

Index-free, grep-based retrieval primitives for agent tool loops, built in Rust on the ripgrep engine crates.

Grep-based retrieval works by running search tools in a reasoning loop instead of pre-embedding a corpus into a vector index. This crate provides the search side of that loop: sharp, fast, composable primitives that a caller, typically an LLM tool loop, invokes to explore a codebase or corpus. There is no LLM loop, no embeddings, and no network access in this crate; the orchestration layer that consumes it lives elsewhere.

## Design principles

- Index-free and always fresh. No persisted index, no build or sync step. Every query reads the live filesystem, so results can never be stale.
- Structured results. All queries and results are typed serde structs, never pre-formatted strings. The caller decides how to render them.
- Parallel by default. Content search fans out across files using the ignore crate's parallel walker.
- Budget-aware. Results feed a context window, so every search supports hard caps on match counts, total bytes, and line length, and reports when a cap was hit.
- Ignore-aware by default. Gitignore, ignore, and hidden file rules are honored out of the box, with per-engine overrides to disable them.
- Composable primitives, not a pipeline. The crate provides search, find, read, and list; the agent orchestrates them. No fixed retrieval strategy is baked in.

## The primitives

| Primitive | Purpose |
| --- | --- |
| `grep` | Content search with regex or literal patterns, context lines, glob restriction, and output budgets |
| `find_files` | File and path search by glob, with gitignore style semantics |
| `read_lines` | Targeted read of an exact 1-based line range from a file |
| `list_dir` | Ignore-aware, depth-limited directory listing for orienting in a repository |

## Usage

```rust
use grepsearch::{EngineConfig, GrepQuery, SearchEngine};

let engine = SearchEngine::new(EngineConfig {
    root: ".".into(),
    ..EngineConfig::default()
})?;

let hits = engine.grep(&GrepQuery {
    pattern: "fn authenticate".into(),
    literal: true,
    context_after: 5,
    max_results: 40,
    max_bytes: 32_000,
    ..GrepQuery::default()
})?;

// Results serialize directly into a tool output for a model.
let json = serde_json::to_string(&hits)?;
```

The model reads the result, decides to `read_lines` around a promising hit or `grep` a refined pattern, and loops. That loop is out of scope for this crate; the guarantee here is that the primitives are fast, structured, budgeted, and ignore-aware.

Every grep result carries a `truncated` flag. When a budget cap stops the search early, the flag tells the caller that more matches may exist, which prevents an agent from mistaking a capped result for a complete one.

## Command line interface

A small development CLI ships behind the `cli` feature for exercising the library by hand:

```sh
cargo run --features cli -- grep "fn main" --glob "*.rs" -A 2
cargo run --features cli -- find "**/*.toml"
cargo run --features cli -- read src/lib.rs 1 40
cargo run --features cli -- ls src --depth 2
```

Every subcommand accepts `--json` to dump the structured result instead of plain text.

## Development

```sh
cargo test --all-features
cargo clippy --all-targets --all-features -- -D warnings
cargo fmt --check
```

Integration tests run against the fixture repository in `tests/fixtures`, which includes a gitignored directory, a hidden file, an empty file, and a binary file to pin down the boundary behavior.

## Non-goals

- No embeddings, vectors, or similarity search
- No LLM calls, agent loop, or prompt handling
- No network access of any kind
- No persistent index or content cache
- No reranking models; ordering is simple and deterministic

## License

MIT
