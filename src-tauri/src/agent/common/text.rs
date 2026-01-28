//! Text utilities for safe string operations.

/// Truncate a string to a maximum number of characters, respecting UTF-8 boundaries.
///
/// If the string is longer than `max_chars`, it will be truncated and an ellipsis (…) appended.
#[inline]
pub fn truncate_chars(s: &str, max_chars: usize) -> String {
    let char_count = s.chars().count();
    if char_count <= max_chars {
        s.to_string()
    } else {
        let mut result: String = s.chars().take(max_chars).collect();
        result.push('…');
        result
    }
}

/// Truncate a string to a maximum number of characters without adding ellipsis.
#[inline]
pub fn truncate_chars_no_ellipsis(s: &str, max_chars: usize) -> String {
    s.chars().take(max_chars).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ascii() {
        assert_eq!(truncate_chars("hello world", 5), "hello…");
        assert_eq!(truncate_chars("hello", 10), "hello");
    }

    #[test]
    fn test_chinese() {
        assert_eq!(truncate_chars("你好世界", 2), "你好…");
        assert_eq!(truncate_chars("你好", 5), "你好");
    }

    #[test]
    fn test_mixed() {
        assert_eq!(truncate_chars("hello你好", 6), "hello你…");
    }

    #[test]
    fn test_empty() {
        assert_eq!(truncate_chars("", 5), "");
    }
}
