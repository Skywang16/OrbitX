/// Truncate string at UTF-8 boundary, never panic
pub fn truncate_at_char_boundary(s: &str, max_bytes: usize) -> &str {
    if s.len() <= max_bytes {
        return s;
    }
    
    let mut pos = max_bytes;
    while pos > 0 && !s.is_char_boundary(pos) {
        pos -= 1;
    }
    
    &s[..pos]
}

pub fn truncate_with_ellipsis(s: &str, max_bytes: usize) -> String {
    if s.len() <= max_bytes {
        return s.to_string();
    }
    
    format!("{}...", truncate_at_char_boundary(s, max_bytes))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ascii_truncation() {
        assert_eq!(truncate_at_char_boundary("hello world", 5), "hello");
    }

    #[test]
    fn test_utf8_boundary() {
        let text = "分析";
        assert_eq!(truncate_at_char_boundary(text, 1), "");
        assert_eq!(truncate_at_char_boundary(text, 2), "");
        assert_eq!(truncate_at_char_boundary(text, 3), "分");
        assert_eq!(truncate_at_char_boundary(text, 4), "分");
    }

    #[test]
    fn test_longer_than_max() {
        let text = "a".repeat(1000);
        assert_eq!(truncate_at_char_boundary(&text, 500).len(), 500);
    }

    #[test]
    fn test_empty_string() {
        assert_eq!(truncate_at_char_boundary("", 100), "");
    }

    #[test]
    fn test_exact_boundary() {
        assert_eq!(truncate_at_char_boundary("hello", 5), "hello");
    }

    #[test]
    fn test_ellipsis() {
        let text = "a".repeat(300);
        let result = truncate_with_ellipsis(&text, 200);
        assert!(result.ends_with("..."));
        assert!(result.len() <= 203);
    }
}
