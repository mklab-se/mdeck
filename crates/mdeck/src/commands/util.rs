//! Small shared helpers for CLI commands.

/// Truncate `s` to at most `max_chars` characters (not bytes), appending
/// `...` when something was cut. Safe for multi-byte text (å, ä, ö, —, emoji).
pub fn truncate_chars(s: &str, max_chars: usize) -> String {
    if s.chars().count() <= max_chars {
        return s.to_string();
    }
    let keep = max_chars.saturating_sub(3);
    let mut out: String = s.chars().take(keep).collect();
    out.push_str("...");
    out
}

/// The longest prefix of `s` that is at most `max_bytes` long and ends on a
/// char boundary. Never panics, unlike `&s[..max_bytes]`.
pub fn truncate_bytes(s: &str, max_bytes: usize) -> &str {
    if s.len() <= max_bytes {
        return s;
    }
    let mut end = max_bytes;
    while end > 0 && !s.is_char_boundary(end) {
        end -= 1;
    }
    &s[..end]
}

/// Number of digits to zero-pad slide numbers with: at least two, more when
/// the deck has 100+ slides so files still sort correctly.
pub fn slide_number_width(count: usize) -> usize {
    count.to_string().len().max(2)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn truncate_chars_ascii() {
        assert_eq!(truncate_chars("short", 10), "short");
        assert_eq!(truncate_chars("a very long string here", 10), "a very ...");
        assert_eq!(truncate_chars("exactly10!", 10), "exactly10!");
    }

    #[test]
    fn truncate_chars_swedish() {
        // Each of å ä ö is 2 bytes; a byte slice at 10 would panic mid-char
        let s = "Räksmörgås är gott på sommaren";
        assert_eq!(truncate_chars(s, 10), "Räksmör...");
        assert_eq!(truncate_chars("åäö", 3), "åäö");
    }

    #[test]
    fn truncate_chars_em_dash_and_emoji() {
        let s = "Plan — build — ship 🚀🚀🚀 and celebrate";
        let t = truncate_chars(s, 12);
        assert_eq!(t, "Plan — bu...");
        assert_eq!(t.chars().count(), 12);
        let e = "🚀🚀🚀🚀🚀🚀";
        assert_eq!(truncate_chars(e, 5), "🚀🚀...");
    }

    #[test]
    fn truncate_chars_tiny_max() {
        assert_eq!(truncate_chars("hello", 3), "...");
        assert_eq!(truncate_chars("hello", 0), "...");
    }

    #[test]
    fn truncate_bytes_respects_char_boundaries() {
        let s = "Räksmörgås";
        // "Rä" is 3 bytes; cutting at 2 would split 'ä'
        assert_eq!(truncate_bytes(s, 2), "R");
        assert_eq!(truncate_bytes(s, 3), "Rä");
        assert_eq!(truncate_bytes(s, 100), s);
        assert_eq!(truncate_bytes("—emoji🚀", 3), "—");
        assert_eq!(truncate_bytes("—emoji🚀", 4), "—e");
        assert_eq!(truncate_bytes("🚀", 3), "");
        assert_eq!(truncate_bytes("", 5), "");
    }

    #[test]
    fn truncate_bytes_never_exceeds_limit() {
        let s = "åäö—🚀 mixed text åäö";
        for max in 0..=s.len() {
            let t = truncate_bytes(s, max);
            assert!(t.len() <= max);
            assert!(s.starts_with(t));
        }
    }

    #[test]
    fn slide_number_width_grows_with_count() {
        assert_eq!(slide_number_width(1), 2);
        assert_eq!(slide_number_width(9), 2);
        assert_eq!(slide_number_width(99), 2);
        assert_eq!(slide_number_width(100), 3);
        assert_eq!(slide_number_width(999), 3);
        assert_eq!(slide_number_width(1000), 4);
    }
}
