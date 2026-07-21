//! Python bindings for the grepsearch retrieval library.
//!
//! The classes here are thin wrappers over the Rust types. Searches release
//! the interpreter lock while they run, so other Python threads keep making
//! progress during a walk.

use pyo3::IntoPyObjectExt;
use pyo3::exceptions::{PyFileNotFoundError, PyOSError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList};
use std::path::PathBuf;

fn map_err(err: grepsearch::Error) -> PyErr {
    match err {
        grepsearch::Error::Io(e) => PyOSError::new_err(e.to_string()),
        grepsearch::Error::NotFound(path) => {
            PyFileNotFoundError::new_err(path.display().to_string())
        }
        other => PyValueError::new_err(other.to_string()),
    }
}

fn json_err(err: serde_json::Error) -> PyErr {
    PyValueError::new_err(err.to_string())
}

fn value_to_py(py: Python<'_>, value: &serde_json::Value) -> PyResult<Py<PyAny>> {
    match value {
        serde_json::Value::Null => Ok(py.None()),
        serde_json::Value::Bool(b) => b.into_py_any(py),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                i.into_py_any(py)
            } else if let Some(u) = n.as_u64() {
                u.into_py_any(py)
            } else {
                n.as_f64().unwrap_or(f64::NAN).into_py_any(py)
            }
        }
        serde_json::Value::String(s) => s.into_py_any(py),
        serde_json::Value::Array(items) => {
            let list = PyList::empty(py);
            for item in items {
                list.append(value_to_py(py, item)?)?;
            }
            list.into_py_any(py)
        }
        serde_json::Value::Object(map) => {
            let dict = PyDict::new(py);
            for (key, item) in map {
                dict.set_item(key, value_to_py(py, item)?)?;
            }
            dict.into_py_any(py)
        }
    }
}

fn to_dict_impl<T: serde::Serialize>(py: Python<'_>, value: &T) -> PyResult<Py<PyAny>> {
    let json = serde_json::to_value(value).map_err(json_err)?;
    value_to_py(py, &json)
}

fn to_json_impl<T: serde::Serialize>(value: &T) -> PyResult<String> {
    serde_json::to_string(value).map_err(json_err)
}

/// The outcome of a content search, grouped by file.
#[pyclass(frozen, module = "grepsearch")]
pub struct GrepResult {
    inner: grepsearch::GrepResult,
}

#[pymethods]
impl GrepResult {
    /// Matches grouped by file, best files first.
    #[getter]
    fn files(&self) -> Vec<FileMatches> {
        self.inner
            .files
            .iter()
            .cloned()
            .map(|inner| FileMatches { inner })
            .collect()
    }

    /// Total number of matches returned across all files.
    #[getter]
    fn total_matches(&self) -> usize {
        self.inner.total_matches
    }

    /// True when a budget cap stopped the search early.
    #[getter]
    fn truncated(&self) -> bool {
        self.inner.truncated
    }

    /// The result as nested dicts and lists.
    fn to_dict(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        to_dict_impl(py, &self.inner)
    }

    /// The result serialized as a JSON string.
    fn to_json(&self) -> PyResult<String> {
        to_json_impl(&self.inner)
    }

    fn __repr__(&self) -> String {
        format!(
            "GrepResult(total_matches={}, files={}, truncated={})",
            self.inner.total_matches,
            self.inner.files.len(),
            self.inner.truncated
        )
    }
}

/// All returned matches within a single file.
#[pyclass(frozen, module = "grepsearch")]
pub struct FileMatches {
    inner: grepsearch::FileMatches,
}

#[pymethods]
impl FileMatches {
    /// Path of the file, relative to the engine root.
    #[getter]
    fn path(&self) -> String {
        self.inner.path.display().to_string()
    }

    /// The matches found in this file, in line order.
    #[getter]
    fn matches(&self) -> Vec<LineMatch> {
        self.inner
            .matches
            .iter()
            .cloned()
            .map(|inner| LineMatch { inner })
            .collect()
    }

    /// Number of matches returned for this file.
    #[getter]
    fn match_count(&self) -> usize {
        self.inner.match_count
    }

    /// The file matches as nested dicts and lists.
    fn to_dict(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        to_dict_impl(py, &self.inner)
    }

    /// The file matches serialized as a JSON string.
    fn to_json(&self) -> PyResult<String> {
        to_json_impl(&self.inner)
    }

    fn __repr__(&self) -> String {
        format!(
            "FileMatches(path={:?}, match_count={})",
            self.inner.path.display().to_string(),
            self.inner.match_count
        )
    }
}

/// A single matched line with optional surrounding context.
#[pyclass(frozen, module = "grepsearch")]
pub struct LineMatch {
    inner: grepsearch::LineMatch,
}

#[pymethods]
impl LineMatch {
    /// 1-based line number of the match.
    #[getter]
    fn line_number(&self) -> usize {
        self.inner.line_number
    }

    /// The matched line, possibly truncated to the query line length cap.
    #[getter]
    fn line(&self) -> String {
        self.inner.line.clone()
    }

    /// Context lines immediately before the match.
    #[getter]
    fn before(&self) -> Vec<String> {
        self.inner.before.clone()
    }

    /// Context lines immediately after the match.
    #[getter]
    fn after(&self) -> Vec<String> {
        self.inner.after.clone()
    }

    /// Byte offsets of the matched line within the file.
    #[getter]
    fn byte_range(&self) -> (usize, usize) {
        self.inner.byte_range
    }

    /// The match as nested dicts and lists.
    fn to_dict(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        to_dict_impl(py, &self.inner)
    }

    /// The match serialized as a JSON string.
    fn to_json(&self) -> PyResult<String> {
        to_json_impl(&self.inner)
    }

    fn __repr__(&self) -> String {
        format!(
            "LineMatch(line_number={}, line={:?})",
            self.inner.line_number, self.inner.line
        )
    }
}

/// A contiguous range of lines read from a file.
#[pyclass(frozen, module = "grepsearch")]
pub struct FileSlice {
    inner: grepsearch::FileSlice,
}

#[pymethods]
impl FileSlice {
    /// Path of the file, relative to the engine root.
    #[getter]
    fn path(&self) -> String {
        self.inner.path.display().to_string()
    }

    /// 1-based first line of the slice.
    #[getter]
    fn start(&self) -> usize {
        self.inner.start
    }

    /// 1-based last line of the slice, inclusive.
    #[getter]
    fn end(&self) -> usize {
        self.inner.end
    }

    /// The requested lines joined with newlines.
    #[getter]
    fn content(&self) -> String {
        self.inner.content.clone()
    }

    /// The slice as nested dicts and lists.
    fn to_dict(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        to_dict_impl(py, &self.inner)
    }

    /// The slice serialized as a JSON string.
    fn to_json(&self) -> PyResult<String> {
        to_json_impl(&self.inner)
    }

    fn __repr__(&self) -> String {
        format!(
            "FileSlice(path={:?}, start={}, end={})",
            self.inner.path.display().to_string(),
            self.inner.start,
            self.inner.end
        )
    }
}

/// A file discovered by a file name search.
#[pyclass(frozen, module = "grepsearch")]
pub struct FileHit {
    inner: grepsearch::FileHit,
}

#[pymethods]
impl FileHit {
    /// Path of the file, relative to the engine root.
    #[getter]
    fn path(&self) -> String {
        self.inner.path.display().to_string()
    }

    /// Size of the file in bytes.
    #[getter]
    fn bytes(&self) -> u64 {
        self.inner.bytes
    }

    /// Last modification time as seconds since the unix epoch, or None.
    #[getter]
    fn modified(&self) -> Option<f64> {
        self.inner.modified
    }

    /// The hit as nested dicts and lists.
    fn to_dict(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        to_dict_impl(py, &self.inner)
    }

    /// The hit serialized as a JSON string.
    fn to_json(&self) -> PyResult<String> {
        to_json_impl(&self.inner)
    }

    fn __repr__(&self) -> String {
        format!(
            "FileHit(path={:?}, bytes={})",
            self.inner.path.display().to_string(),
            self.inner.bytes
        )
    }
}

/// An entry produced by a directory listing.
#[pyclass(frozen, module = "grepsearch")]
pub struct DirEntry {
    inner: grepsearch::DirEntry,
}

#[pymethods]
impl DirEntry {
    /// Path of the entry, relative to the engine root.
    #[getter]
    fn path(&self) -> String {
        self.inner.path.display().to_string()
    }

    /// True when the entry is a directory.
    #[getter]
    fn is_dir(&self) -> bool {
        self.inner.is_dir
    }

    /// Depth of the entry below the listed directory, starting at 1.
    #[getter]
    fn depth(&self) -> usize {
        self.inner.depth
    }

    /// The entry as nested dicts and lists.
    fn to_dict(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        to_dict_impl(py, &self.inner)
    }

    /// The entry serialized as a JSON string.
    fn to_json(&self) -> PyResult<String> {
        to_json_impl(&self.inner)
    }

    fn __repr__(&self) -> String {
        format!(
            "DirEntry(path={:?}, is_dir={})",
            self.inner.path.display().to_string(),
            self.inner.is_dir
        )
    }
}

/// Index-free search engine rooted at a directory.
#[pyclass(frozen, module = "grepsearch")]
pub struct SearchEngine {
    inner: grepsearch::SearchEngine,
}

#[pymethods]
impl SearchEngine {
    #[new]
    #[pyo3(signature = (
        root = PathBuf::from("."),
        *,
        respect_gitignore = true,
        include_hidden = false,
        follow_symlinks = false,
    ))]
    fn new(
        root: PathBuf,
        respect_gitignore: bool,
        include_hidden: bool,
        follow_symlinks: bool,
    ) -> PyResult<Self> {
        let config = grepsearch::EngineConfig {
            root,
            respect_gitignore,
            include_hidden,
            follow_symlinks,
        };
        let inner = grepsearch::SearchEngine::new(config).map_err(map_err)?;
        Ok(Self { inner })
    }

    /// The canonicalized root directory of this engine.
    #[getter]
    fn root(&self) -> String {
        self.inner.root().display().to_string()
    }

    /// Searches file contents under the root for a pattern.
    #[pyo3(signature = (
        pattern,
        *,
        literal = false,
        case_insensitive = false,
        globs = None,
        context_before = 0,
        context_after = 0,
        max_results = grepsearch::DEFAULT_MAX_RESULTS,
        max_bytes = grepsearch::DEFAULT_MAX_BYTES,
        max_line_len = grepsearch::DEFAULT_MAX_LINE_LEN,
    ))]
    #[allow(clippy::too_many_arguments)]
    fn grep(
        &self,
        py: Python<'_>,
        pattern: String,
        literal: bool,
        case_insensitive: bool,
        globs: Option<Vec<String>>,
        context_before: usize,
        context_after: usize,
        max_results: usize,
        max_bytes: usize,
        max_line_len: usize,
    ) -> PyResult<GrepResult> {
        let query = grepsearch::GrepQuery {
            pattern,
            literal,
            case_insensitive,
            globs: globs.unwrap_or_default(),
            context_before,
            context_after,
            max_results,
            max_bytes,
            max_line_len,
        };
        let inner = py.detach(|| self.inner.grep(&query)).map_err(map_err)?;
        Ok(GrepResult { inner })
    }

    /// Finds files under the root whose paths match a glob.
    #[pyo3(signature = (glob, *, max_results = grepsearch::DEFAULT_MAX_FILES))]
    fn find_files(
        &self,
        py: Python<'_>,
        glob: String,
        max_results: usize,
    ) -> PyResult<Vec<FileHit>> {
        let query = grepsearch::FileQuery { glob, max_results };
        let hits = py
            .detach(|| self.inner.find_files(&query))
            .map_err(map_err)?;
        Ok(hits.into_iter().map(|inner| FileHit { inner }).collect())
    }

    /// Reads an inclusive 1-based line range from a file.
    fn read_lines(
        &self,
        py: Python<'_>,
        path: PathBuf,
        start: usize,
        end: usize,
    ) -> PyResult<FileSlice> {
        let inner = py
            .detach(|| self.inner.read_lines(&path, start, end))
            .map_err(map_err)?;
        Ok(FileSlice { inner })
    }

    /// Lists a directory below the root, up to max_depth levels deep.
    #[pyo3(signature = (path = PathBuf::from("."), *, max_depth = 1))]
    fn list_dir(&self, py: Python<'_>, path: PathBuf, max_depth: usize) -> PyResult<Vec<DirEntry>> {
        let entries = py
            .detach(|| self.inner.list_dir(&path, max_depth))
            .map_err(map_err)?;
        Ok(entries
            .into_iter()
            .map(|inner| DirEntry { inner })
            .collect())
    }

    fn __repr__(&self) -> String {
        format!("SearchEngine(root={:?})", self.root())
    }
}

#[pymodule]
fn _native(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<SearchEngine>()?;
    m.add_class::<GrepResult>()?;
    m.add_class::<FileMatches>()?;
    m.add_class::<LineMatch>()?;
    m.add_class::<FileSlice>()?;
    m.add_class::<FileHit>()?;
    m.add_class::<DirEntry>()?;
    m.add("__version__", env!("CARGO_PKG_VERSION"))?;
    Ok(())
}
