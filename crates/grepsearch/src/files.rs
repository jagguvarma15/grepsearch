use crate::engine::SearchEngine;
use crate::error::Error;
use crate::types::{FileHit, FileQuery};
use ignore::overrides::OverrideBuilder;

/// Finds files under the engine root whose paths match the query glob.
///
/// Matching uses gitignore style semantics, so a bare file glob such as
/// `*.rs` matches at any depth. Results are sorted by path and capped at
/// `max_results`.
pub(crate) fn find_files(engine: &SearchEngine, q: &FileQuery) -> Result<Vec<FileHit>, Error> {
    let mut overrides = OverrideBuilder::new(engine.root());
    overrides
        .add(&q.glob)
        .map_err(|e| Error::Glob(e.to_string()))?;
    let overrides = overrides.build().map_err(|e| Error::Glob(e.to_string()))?;

    let mut builder = engine.walk_builder(engine.root());
    builder.overrides(overrides);

    let mut hits = Vec::new();
    for entry in builder.build() {
        let Ok(entry) = entry else { continue };
        if !entry.file_type().is_some_and(|t| t.is_file()) {
            continue;
        }
        let metadata = entry.metadata().ok();
        hits.push(FileHit {
            path: engine.relativize(entry.path()),
            bytes: metadata.as_ref().map_or(0, std::fs::Metadata::len),
            modified: metadata
                .and_then(|m| m.modified().ok())
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|d| d.as_secs_f64()),
        });
    }
    hits.sort_by(|a, b| a.path.cmp(&b.path));
    hits.truncate(q.max_results);
    Ok(hits)
}
