//! Index-free, grep-based retrieval primitives for agent tool loops.
//!
//! This crate wraps the ripgrep engine crates behind a structured,
//! serializable API. A caller, typically an LLM tool loop, gets typed
//! results it can format into tool outputs and decides for itself how to
//! chain the primitives together.
//!
//! # Design principles
//!
//! - Index-free and always fresh: every query reads the live filesystem.
//!   There is no persisted index, no content cache, and no sync step.
//! - Structured results: all queries and results are plain serde types,
//!   never pre-formatted strings.
//! - Parallel by default: content search fans out across files using a
//!   parallel, gitignore-aware directory walker.
//! - Budget-aware: results feed a context window, so every primitive
//!   supports hard caps on match counts, total bytes, and line length, and
//!   reports when a cap was hit.
//! - Ignore-aware by default: gitignore, ignore, and hidden file rules are
//!   honored out of the box and can be disabled per engine.
//! - Composable primitives, not a pipeline: content search, file search,
//!   targeted reads, and directory listing are sharp, independent tools.
//!
//! # Example
//!
//! ```no_run
//! use grepsearch::{EngineConfig, GrepQuery, SearchEngine};
//!
//! # fn main() -> Result<(), grepsearch::Error> {
//! let engine = SearchEngine::new(EngineConfig {
//!     root: ".".into(),
//!     ..EngineConfig::default()
//! })?;
//! let hits = engine.grep(&GrepQuery {
//!     pattern: "fn authenticate".into(),
//!     literal: true,
//!     context_after: 5,
//!     max_results: 40,
//!     max_bytes: 32_000,
//!     ..GrepQuery::default()
//! })?;
//! for file in &hits.files {
//!     for m in &file.matches {
//!         println!("{}:{}: {}", file.path.display(), m.line_number, m.line);
//!     }
//! }
//! # Ok(())
//! # }
//! ```

#![warn(missing_docs)]

mod budget;
mod engine;
mod error;
mod files;
mod read;
mod search;
mod types;
mod walk;

pub use engine::{EngineConfig, SearchEngine};
pub use error::Error;
pub use types::{
    DEFAULT_MAX_BYTES, DEFAULT_MAX_FILES, DEFAULT_MAX_LINE_LEN, DEFAULT_MAX_RESULTS, DirEntry,
    FileHit, FileMatches, FileQuery, FileSlice, GrepQuery, GrepResult, LineMatch,
};
