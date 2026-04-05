//! Text formatting utilities
//!
//! This module provides helper functions for formatting and aligning text.

/// Format a cost value for display
///
/// - None -> "n/a"
/// - 0.0 -> "0"
/// - Whole numbers -> without decimals
/// - Others -> trimmed to significant decimals
pub fn format_cost(value: Option<f64>) -> String {
    match value {
        None => "n/a".to_string(),
        Some(0.0) => "0".to_string(),
        Some(v) if v.fract() == 0.0 => format!("{:.0}", v),
        Some(v) => format!("{:0.10}", v)
            .trim_end_matches('0')
            .trim_end_matches('.')
            .to_string(),
    }
}

/// Left-justify text to a given width
pub fn ljust(text: &str, width: usize) -> String {
    if text.len() >= width {
        text.to_string()
    } else {
        format!("{}{}", text, " ".repeat(width - text.len()))
    }
}

/// Right-justify text to a given width
pub fn rjust(text: &str, width: usize) -> String {
    if text.len() >= width {
        text.to_string()
    } else {
        format!("{}{}", " ".repeat(width - text.len()), text)
    }
}

/// Wrap usage text to fit within a width, breaking at commas
pub fn wrap_usage(text: &str, width: usize) -> Vec<String> {
    if text.len() <= width {
        return vec![text.to_string()];
    }

    let parts: Vec<&str> = text.split(',').map(|part| part.trim()).collect();
    let mut lines = Vec::new();
    let mut current = String::new();

    for part in parts {
        if current.is_empty() {
            current = part.to_string();
        } else if current.len() + 2 + part.len() <= width {
            current.push_str(", ");
            current.push_str(part);
        } else {
            lines.push(current);
            current = part.to_string();
        }
    }

    if !current.is_empty() {
        lines.push(current);
    }

    lines
}

/// Strip ANSI escape sequences from text
pub fn strip_ansi(text: &str) -> String {
    let mut result = String::with_capacity(text.len());
    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        if chars[i] == '\x1b' && i + 1 < chars.len() && chars[i + 1] == '[' {
            i += 2;
            while i < chars.len() && chars[i] != 'm' && !chars[i].is_ascii_alphabetic() {
                i += 1;
            }
            if i < chars.len() {
                i += 1;
            }
        } else {
            result.push(chars[i]);
            i += 1;
        }
    }

    result
}

/// Split a model ID into (provider, model_name) parts
pub fn split_model_id(model: &str) -> (String, String) {
    match model.split_once('/') {
        Some((provider, model_name)) => (provider.to_string(), model_name.to_string()),
        None => (String::new(), model.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_cost_should_handle_various_cases() {
        assert_eq!(format_cost(None), "n/a");
        assert_eq!(format_cost(Some(0.0)), "0");
        assert_eq!(format_cost(Some(5.0)), "5");
        assert_eq!(format_cost(Some(1.5)), "1.5");
        assert_eq!(format_cost(Some(1.500)), "1.5");
    }

    #[test]
    fn ljust_should_pad_correctly() {
        assert_eq!(ljust("test", 6), "test  ");
        assert_eq!(ljust("test", 4), "test");
        assert_eq!(ljust("testing", 4), "testing");
    }

    #[test]
    fn rjust_should_pad_correctly() {
        assert_eq!(rjust("test", 6), "  test");
        assert_eq!(rjust("test", 4), "test");
        assert_eq!(rjust("testing", 4), "testing");
    }

    #[test]
    fn wrap_usage_should_respect_width() {
        let result = wrap_usage("a, b, c", 10);
        assert_eq!(result, vec!["a, b, c"]);

        let result = wrap_usage("verylongitem1, verylongitem2", 10);
        assert!(result.len() > 1);
    }

    #[test]
    fn strip_ansi_should_remove_escape_sequences() {
        let colored = "\x1b[32mprovider/alpha\x1b[0m";
        assert_eq!(strip_ansi(colored), "provider/alpha");

        let plain = "provider/alpha";
        assert_eq!(strip_ansi(plain), "provider/alpha");
    }

    #[test]
    fn split_model_id_should_handle_various_formats() {
        assert_eq!(
            split_model_id("openai/gpt-4"),
            ("openai".to_string(), "gpt-4".to_string())
        );
        assert_eq!(
            split_model_id("provider/sub/model"),
            ("provider".to_string(), "sub/model".to_string())
        );
        assert_eq!(
            split_model_id("model-without-provider"),
            (String::new(), "model-without-provider".to_string())
        );
    }
}
