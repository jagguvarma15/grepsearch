use crate::engine::SearchEngine;
use crate::error::Error;
use crate::types::FileSlice;
use std::path::Path;

/// Reads an inclusive, 1-based line range from a file under the engine root.
///
/// The end of the range is clamped to the last line of the file. Requesting a
/// start past the end of the file, a start of zero, or an end before the
/// start is an error.
pub(crate) fn read_lines(
    engine: &SearchEngine,
    path: &Path,
    start: usize,
    end: usize,
) -> Result<FileSlice, Error> {
    let abs = engine.resolve(path);
    if !abs.is_file() {
        return Err(Error::NotFound(path.to_path_buf()));
    }
    if start == 0 {
        return Err(Error::InvalidRange {
            start,
            end,
            reason: String::from("line numbers are 1-based"),
        });
    }
    if end < start {
        return Err(Error::InvalidRange {
            start,
            end,
            reason: String::from("end is before start"),
        });
    }
    let raw = std::fs::read(&abs)?;
    let text = String::from_utf8_lossy(&raw);
    let lines: Vec<&str> = text.lines().collect();
    if start > lines.len() {
        return Err(Error::InvalidRange {
            start,
            end,
            reason: format!("file has {} lines", lines.len()),
        });
    }
    let end = end.min(lines.len());
    Ok(FileSlice {
        path: engine.relativize(&abs),
        start,
        end,
        content: lines[start - 1..end].join("\n"),
    })
}
