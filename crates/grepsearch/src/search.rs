use crate::budget::{Budget, truncate_line};
use crate::engine::SearchEngine;
use crate::error::Error;
use crate::types::{FileMatches, GrepQuery, GrepResult, LineMatch};
use grep::regex::{RegexMatcher, RegexMatcherBuilder};
use grep::searcher::{
    BinaryDetection, Searcher, SearcherBuilder, Sink, SinkContext, SinkContextKind, SinkMatch,
};
use ignore::WalkState;
use ignore::overrides::OverrideBuilder;
use std::sync::Mutex;

/// Runs a content search across all files under the engine root.
///
/// Files are walked in parallel. Each worker reserves budget capacity before
/// keeping a match, so caps hold across threads and the walk stops early once
/// the budget is exhausted.
pub(crate) fn grep(engine: &SearchEngine, q: &GrepQuery) -> Result<GrepResult, Error> {
    let matcher = build_matcher(q)?;
    let budget = Budget::new(q.max_results, q.max_bytes);
    let collected: Mutex<Vec<FileMatches>> = Mutex::new(Vec::new());

    let mut builder = engine.walk_builder(engine.root());
    if !q.globs.is_empty() {
        let mut overrides = OverrideBuilder::new(engine.root());
        for glob in &q.globs {
            overrides
                .add(glob)
                .map_err(|e| Error::Glob(e.to_string()))?;
        }
        builder.overrides(overrides.build().map_err(|e| Error::Glob(e.to_string()))?);
    }

    let matcher_ref = &matcher;
    let budget_ref = &budget;
    let collected_ref = &collected;
    builder.build_parallel().run(|| {
        let mut searcher = SearcherBuilder::new()
            .line_number(true)
            .before_context(q.context_before)
            .after_context(q.context_after)
            .binary_detection(BinaryDetection::quit(0))
            .build();
        Box::new(move |entry| {
            if budget_ref.truncated() {
                return WalkState::Quit;
            }
            let entry = match entry {
                Ok(entry) => entry,
                Err(_) => return WalkState::Continue,
            };
            if !entry.file_type().is_some_and(|t| t.is_file()) {
                return WalkState::Continue;
            }
            let mut sink = MatchSink {
                budget: budget_ref,
                max_line_len: q.max_line_len,
                matches: Vec::new(),
                pending_before: Vec::new(),
            };
            // Unreadable files are skipped rather than failing the query.
            let _ = searcher.search_path(matcher_ref, entry.path(), &mut sink);
            if !sink.matches.is_empty() {
                let file = FileMatches {
                    path: engine.relativize(entry.path()),
                    match_count: sink.matches.len(),
                    matches: sink.matches,
                };
                collected_ref.lock().unwrap().push(file);
            }
            if budget_ref.truncated() {
                WalkState::Quit
            } else {
                WalkState::Continue
            }
        })
    });

    let mut files = collected.into_inner().unwrap();
    sort_files(&mut files);
    let total_matches = files.iter().map(|f| f.match_count).sum();
    Ok(GrepResult {
        files,
        total_matches,
        truncated: budget.truncated(),
    })
}

fn build_matcher(q: &GrepQuery) -> Result<RegexMatcher, Error> {
    let mut builder = RegexMatcherBuilder::new();
    builder.case_insensitive(q.case_insensitive);
    let result = if q.literal {
        builder.build_literals(&[q.pattern.as_str()])
    } else {
        builder.build(&q.pattern)
    };
    result.map_err(|e| Error::Pattern(e.to_string()))
}

/// Deterministic result order: files with more matches first, then shallower
/// paths, then lexicographic by path.
fn sort_files(files: &mut [FileMatches]) {
    files.sort_by(|a, b| {
        b.match_count
            .cmp(&a.match_count)
            .then_with(|| {
                a.path
                    .components()
                    .count()
                    .cmp(&b.path.components().count())
            })
            .then_with(|| a.path.cmp(&b.path))
    });
}

/// Collects matches and context lines for a single file, charging every
/// stored line against the shared budget before keeping it.
struct MatchSink<'a> {
    budget: &'a Budget,
    max_line_len: usize,
    matches: Vec<LineMatch>,
    pending_before: Vec<String>,
}

impl Sink for MatchSink<'_> {
    type Error = std::io::Error;

    fn matched(&mut self, _searcher: &Searcher, mat: &SinkMatch<'_>) -> Result<bool, Self::Error> {
        let raw = mat.bytes();
        let text = String::from_utf8_lossy(raw);
        let line = truncate_line(text.trim_end_matches(['\r', '\n']), self.max_line_len);
        let before = std::mem::take(&mut self.pending_before);
        let cost = line.len() + before.iter().map(String::len).sum::<usize>();
        if !self.budget.try_reserve(1, cost) {
            // Budget exhausted; stop searching this file. Matches collected
            // so far are kept.
            return Ok(false);
        }
        let offset = mat.absolute_byte_offset() as usize;
        self.matches.push(LineMatch {
            line_number: mat.line_number().unwrap_or(0) as usize,
            line,
            before,
            after: Vec::new(),
            byte_range: (offset, offset + raw.len()),
        });
        Ok(true)
    }

    fn context(
        &mut self,
        _searcher: &Searcher,
        context: &SinkContext<'_>,
    ) -> Result<bool, Self::Error> {
        let text = String::from_utf8_lossy(context.bytes());
        let line = truncate_line(text.trim_end_matches(['\r', '\n']), self.max_line_len);
        match context.kind() {
            SinkContextKind::Before => {
                // Charged against the budget when the owning match arrives.
                self.pending_before.push(line);
            }
            SinkContextKind::After => {
                if let Some(last) = self.matches.last_mut() {
                    if !self.budget.try_reserve(0, line.len()) {
                        return Ok(false);
                    }
                    last.after.push(line);
                }
            }
            SinkContextKind::Other => {}
        }
        Ok(true)
    }

    fn context_break(&mut self, _searcher: &Searcher) -> Result<bool, Self::Error> {
        self.pending_before.clear();
        Ok(true)
    }
}
