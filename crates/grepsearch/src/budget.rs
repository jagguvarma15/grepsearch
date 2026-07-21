use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

/// Shared, thread safe accounting for the output budget of a single query.
///
/// Workers reserve capacity before storing anything. A failed reservation
/// marks the budget as truncated, which callers use to stop the walk early
/// and to set the truncated flag on the final result.
pub(crate) struct Budget {
    max_results: usize,
    max_bytes: usize,
    matches_used: AtomicUsize,
    bytes_used: AtomicUsize,
    truncated: AtomicBool,
}

impl Budget {
    pub(crate) fn new(max_results: usize, max_bytes: usize) -> Self {
        Self {
            max_results,
            max_bytes,
            matches_used: AtomicUsize::new(0),
            bytes_used: AtomicUsize::new(0),
            truncated: AtomicBool::new(false),
        }
    }

    /// Attempts to reserve room for `matches` additional matches and `bytes`
    /// additional bytes of line text. Returns false and marks the budget as
    /// truncated when either cap would be exceeded.
    pub(crate) fn try_reserve(&self, matches: usize, bytes: usize) -> bool {
        let prev_matches = self.matches_used.fetch_add(matches, Ordering::SeqCst);
        let prev_bytes = self.bytes_used.fetch_add(bytes, Ordering::SeqCst);
        if prev_matches + matches > self.max_results || prev_bytes + bytes > self.max_bytes {
            self.truncated.store(true, Ordering::SeqCst);
            return false;
        }
        true
    }

    /// True when a reservation has failed, meaning results were dropped.
    pub(crate) fn truncated(&self) -> bool {
        self.truncated.load(Ordering::SeqCst)
    }
}

/// Truncates `text` to at most `max_len` bytes, respecting char boundaries.
pub(crate) fn truncate_line(text: &str, max_len: usize) -> String {
    if text.len() <= max_len {
        return text.to_string();
    }
    let mut end = max_len;
    while end > 0 && !text.is_char_boundary(end) {
        end -= 1;
    }
    text[..end].to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reservations_within_caps_succeed() {
        let budget = Budget::new(2, 100);
        assert!(budget.try_reserve(1, 40));
        assert!(budget.try_reserve(1, 40));
        assert!(!budget.truncated());
    }

    #[test]
    fn exceeding_match_cap_marks_truncated() {
        let budget = Budget::new(1, 100);
        assert!(budget.try_reserve(1, 10));
        assert!(!budget.try_reserve(1, 10));
        assert!(budget.truncated());
    }

    #[test]
    fn exceeding_byte_cap_marks_truncated() {
        let budget = Budget::new(10, 50);
        assert!(budget.try_reserve(1, 30));
        assert!(!budget.try_reserve(1, 30));
        assert!(budget.truncated());
    }

    #[test]
    fn zero_cost_reservation_succeeds_at_cap() {
        let budget = Budget::new(1, 10);
        assert!(budget.try_reserve(1, 10));
        assert!(budget.try_reserve(0, 0));
        assert!(!budget.truncated());
    }

    #[test]
    fn short_lines_are_untouched() {
        assert_eq!(truncate_line("hello", 10), "hello");
        assert_eq!(truncate_line("hello", 5), "hello");
    }

    #[test]
    fn long_lines_are_cut_at_the_cap() {
        assert_eq!(truncate_line("hello world", 5), "hello");
        assert_eq!(truncate_line("hello", 0), "");
    }

    #[test]
    fn truncation_respects_char_boundaries() {
        let text = "ab\u{00e9}cd";
        let cut = truncate_line(text, 3);
        assert_eq!(cut, "ab");
        assert!(cut.len() <= 3);
    }
}
