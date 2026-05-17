//! Subsequence fuzzy filter for TUI lists (ROADMAP v2 §4.4).

/// `true` when all chars of `needle` appear in order in `haystack` (case-insensitive).
pub fn matches(haystack: &str, needle: &str) -> bool {
    if needle.is_empty() {
        return true;
    }
    let h: Vec<char> = haystack.to_lowercase().chars().collect();
    let n: Vec<char> = needle.to_lowercase().chars().collect();
    let mut i = 0usize;
    for &c in &n {
        while i < h.len() && h[i] != c {
            i += 1;
        }
        if i >= h.len() {
            return false;
        }
        i += 1;
    }
    true
}

/// Lower score = better match. Returns `None` if no match.
pub fn score(haystack: &str, needle: &str) -> Option<u32> {
    if needle.is_empty() {
        return Some(0);
    }
    let h: Vec<char> = haystack.to_lowercase().chars().collect();
    let n: Vec<char> = needle.to_lowercase().chars().collect();
    let mut idx = 0usize;
    let mut total_gap = 0u32;
    for &c in &n {
        let start = idx;
        while idx < h.len() && h[idx] != c {
            idx += 1;
        }
        if idx >= h.len() {
            return None;
        }
        total_gap += (idx - start) as u32;
        idx += 1;
    }
    Some(total_gap + haystack.len() as u32 / 10)
}
