use crate::error::Error;
use crate::types::{DirEntry, FileHit, FileQuery, FileSlice, GrepQuery, GrepResult};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Configuration for a [`SearchEngine`].
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct EngineConfig {
    /// Directory that all searches are rooted at. Returned paths are
    /// relative to this directory.
    pub root: PathBuf,
    /// Honor gitignore, ignore, and git exclude files. Defaults to true.
    pub respect_gitignore: bool,
    /// Include hidden files and directories. Defaults to false.
    pub include_hidden: bool,
    /// Follow symbolic links while walking. Defaults to false.
    pub follow_symlinks: bool,
}

impl Default for EngineConfig {
    fn default() -> Self {
        Self {
            root: PathBuf::from("."),
            respect_gitignore: true,
            include_hidden: false,
            follow_symlinks: false,
        }
    }
}

/// The entry point for all retrieval primitives.
///
/// An engine is a thin, stateless handle over a root directory. It keeps no
/// index and no content cache; every query reads the live filesystem.
#[derive(Debug)]
pub struct SearchEngine {
    root: PathBuf,
    config: EngineConfig,
}

impl SearchEngine {
    /// Creates an engine rooted at `config.root`.
    ///
    /// Returns [`Error::NotFound`] when the root does not exist or is not a
    /// directory.
    pub fn new(config: EngineConfig) -> Result<Self, Error> {
        let root = config
            .root
            .canonicalize()
            .map_err(|_| Error::NotFound(config.root.clone()))?;
        if !root.is_dir() {
            return Err(Error::NotFound(config.root.clone()));
        }
        Ok(Self { root, config })
    }

    /// The canonicalized root directory of this engine.
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// The configuration this engine was created with.
    pub fn config(&self) -> &EngineConfig {
        &self.config
    }

    /// Searches file contents under the root for a pattern.
    pub fn grep(&self, q: &GrepQuery) -> Result<GrepResult, Error> {
        crate::search::grep(self, q)
    }

    /// Finds files under the root whose paths match a glob.
    pub fn find_files(&self, q: &FileQuery) -> Result<Vec<FileHit>, Error> {
        crate::files::find_files(self, q)
    }

    /// Reads an inclusive 1-based line range from a file.
    pub fn read_lines(&self, path: &Path, start: usize, end: usize) -> Result<FileSlice, Error> {
        crate::read::read_lines(self, path, start, end)
    }

    /// Lists a directory below the root, up to `max_depth` levels deep.
    pub fn list_dir(&self, path: &Path, max_depth: usize) -> Result<Vec<DirEntry>, Error> {
        crate::walk::list_dir(self, path, max_depth)
    }

    /// Builds a directory walker over `dir` that applies this engine's
    /// ignore, hidden file, and symlink settings.
    pub(crate) fn walk_builder(&self, dir: &Path) -> ignore::WalkBuilder {
        let mut builder = ignore::WalkBuilder::new(dir);
        builder
            .hidden(!self.config.include_hidden)
            .follow_links(self.config.follow_symlinks)
            .require_git(false);
        if !self.config.respect_gitignore {
            builder
                .git_ignore(false)
                .git_global(false)
                .git_exclude(false)
                .ignore(false)
                .parents(false);
        }
        builder
    }

    /// Resolves a caller supplied path against the engine root.
    pub(crate) fn resolve(&self, path: &Path) -> PathBuf {
        if path.is_absolute() {
            path.to_path_buf()
        } else {
            self.root.join(path)
        }
    }

    /// Makes a path relative to the engine root when possible.
    pub(crate) fn relativize(&self, path: &Path) -> PathBuf {
        path.strip_prefix(&self.root)
            .map(Path::to_path_buf)
            .unwrap_or_else(|_| path.to_path_buf())
    }
}
