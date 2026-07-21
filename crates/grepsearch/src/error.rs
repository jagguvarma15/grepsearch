use std::path::PathBuf;

/// Errors returned by the search primitives.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// An underlying I/O operation failed.
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    /// A search pattern failed to compile.
    #[error("invalid pattern: {0}")]
    Pattern(String),

    /// A glob expression failed to parse.
    #[error("invalid glob: {0}")]
    Glob(String),

    /// A requested path does not exist or has the wrong type.
    #[error("not found: {0}")]
    NotFound(PathBuf),

    /// A requested line range is invalid for the target file.
    #[error("invalid line range {start}..={end}: {reason}")]
    InvalidRange {
        /// First requested line, 1-based.
        start: usize,
        /// Last requested line, inclusive.
        end: usize,
        /// Why the range was rejected.
        reason: String,
    },
}
