use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Default cap on the number of matches returned by a grep query.
pub const DEFAULT_MAX_RESULTS: usize = 100;

/// Default cap on the total bytes of line text returned by a grep query.
pub const DEFAULT_MAX_BYTES: usize = 65_536;

/// Default cap on the length of a single returned line, in bytes.
pub const DEFAULT_MAX_LINE_LEN: usize = 512;

/// Default cap on the number of files returned by a file query.
pub const DEFAULT_MAX_FILES: usize = 1_000;

/// A content search request executed by [`SearchEngine::grep`](crate::SearchEngine::grep).
///
/// All budget fields are hard caps. When any cap is hit the result is marked
/// as truncated so the caller knows the picture is incomplete.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct GrepQuery {
    /// Pattern to search for. Interpreted as a regular expression unless
    /// `literal` is set.
    pub pattern: String,
    /// Treat `pattern` as a fixed string instead of a regular expression.
    pub literal: bool,
    /// Match case-insensitively.
    pub case_insensitive: bool,
    /// Restrict the search to paths matching these globs, using gitignore
    /// style semantics. A leading `!` negates a glob.
    pub globs: Vec<String>,
    /// Lines of context to include before each match.
    pub context_before: usize,
    /// Lines of context to include after each match.
    pub context_after: usize,
    /// Hard cap on the number of matches returned across all files.
    pub max_results: usize,
    /// Hard cap on the total bytes of matched and context line text returned.
    pub max_bytes: usize,
    /// Truncate matched and context lines longer than this many bytes.
    pub max_line_len: usize,
}

impl Default for GrepQuery {
    fn default() -> Self {
        Self {
            pattern: String::new(),
            literal: false,
            case_insensitive: false,
            globs: Vec::new(),
            context_before: 0,
            context_after: 0,
            max_results: DEFAULT_MAX_RESULTS,
            max_bytes: DEFAULT_MAX_BYTES,
            max_line_len: DEFAULT_MAX_LINE_LEN,
        }
    }
}

impl GrepQuery {
    /// Creates a query for `pattern` with default options and budgets.
    pub fn new(pattern: impl Into<String>) -> Self {
        Self {
            pattern: pattern.into(),
            ..Self::default()
        }
    }
}

/// A file name search request executed by
/// [`SearchEngine::find_files`](crate::SearchEngine::find_files).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct FileQuery {
    /// Glob to match against paths relative to the engine root, using
    /// gitignore style semantics.
    pub glob: String,
    /// Hard cap on the number of files returned.
    pub max_results: usize,
}

impl Default for FileQuery {
    fn default() -> Self {
        Self {
            glob: String::from("**"),
            max_results: DEFAULT_MAX_FILES,
        }
    }
}

impl FileQuery {
    /// Creates a query for `glob` with the default result cap.
    pub fn new(glob: impl Into<String>) -> Self {
        Self {
            glob: glob.into(),
            ..Self::default()
        }
    }
}

/// The outcome of a content search, grouped by file.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GrepResult {
    /// Matches grouped by file, ordered by descending match count, then by
    /// ascending path depth, then by path.
    pub files: Vec<FileMatches>,
    /// Total number of matches returned across all files.
    pub total_matches: usize,
    /// True when a budget cap stopped the search before it completed, meaning
    /// more matches may exist than were returned.
    pub truncated: bool,
}

/// All returned matches within a single file.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FileMatches {
    /// Path of the file, relative to the engine root.
    pub path: PathBuf,
    /// The matches found in this file, in line order.
    pub matches: Vec<LineMatch>,
    /// Number of matches returned for this file.
    pub match_count: usize,
}

/// A single matched line with optional surrounding context.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LineMatch {
    /// 1-based line number of the match.
    pub line_number: usize,
    /// The matched line, without its trailing line terminator and possibly
    /// truncated to the query's maximum line length.
    pub line: String,
    /// Context lines immediately before the match, oldest first.
    pub before: Vec<String>,
    /// Context lines immediately after the match, closest first.
    pub after: Vec<String>,
    /// Byte offsets of the matched line within the file, as a half open
    /// start and end pair.
    pub byte_range: (usize, usize),
}

/// A contiguous range of lines read from a file.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FileSlice {
    /// Path of the file, relative to the engine root.
    pub path: PathBuf,
    /// 1-based first line of the slice.
    pub start: usize,
    /// 1-based last line of the slice, inclusive. May be smaller than the
    /// requested end when the file is shorter.
    pub end: usize,
    /// The requested lines joined with newlines.
    pub content: String,
}

/// A file discovered by a file name search.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FileHit {
    /// Path of the file, relative to the engine root.
    pub path: PathBuf,
    /// Size of the file in bytes.
    pub bytes: u64,
    /// Last modification time as seconds since the unix epoch, when
    /// available.
    pub modified: Option<f64>,
}

/// An entry produced by a directory listing.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DirEntry {
    /// Path of the entry, relative to the engine root.
    pub path: PathBuf,
    /// True when the entry is a directory.
    pub is_dir: bool,
    /// Depth of the entry below the listed directory, starting at 1.
    pub depth: usize,
}
