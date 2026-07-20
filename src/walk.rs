use crate::engine::SearchEngine;
use crate::error::Error;
use crate::types::DirEntry;
use std::path::Path;

/// Lists entries below a directory, up to `max_depth` levels deep.
///
/// The listing honors the engine's ignore and hidden file settings, so an
/// agent orienting itself in a repository sees the same files a search
/// would. Entries are sorted by path; the listed directory itself is not
/// included.
pub(crate) fn list_dir(
    engine: &SearchEngine,
    path: &Path,
    max_depth: usize,
) -> Result<Vec<DirEntry>, Error> {
    let abs = engine.resolve(path);
    if !abs.is_dir() {
        return Err(Error::NotFound(path.to_path_buf()));
    }
    let mut builder = engine.walk_builder(&abs);
    builder.max_depth(Some(max_depth));

    let mut entries = Vec::new();
    for entry in builder.build() {
        let Ok(entry) = entry else { continue };
        if entry.depth() == 0 {
            continue;
        }
        entries.push(DirEntry {
            path: engine.relativize(entry.path()),
            is_dir: entry.file_type().is_some_and(|t| t.is_dir()),
            depth: entry.depth(),
        });
    }
    entries.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(entries)
}
