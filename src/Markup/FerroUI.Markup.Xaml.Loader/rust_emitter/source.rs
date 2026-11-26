//! Text helpers of the emitter: literals and identifiers.

/// `text` as a Rust string literal (with the quotes). Printable ASCII is
/// written as it is; the quote, the backslash and the usual control
/// characters use their short escapes; everything else is written as
/// `\u{..}`, so the literal is plain ASCII whatever the document holds.
pub fn rust_string_literal(text: &str) -> String {
    let mut literal = String::with_capacity(text.len() + 2);
    literal.push('"');
    for character in text.chars() {
        match character {
            '"' => literal.push_str("\\\""),
            '\\' => literal.push_str("\\\\"),
            '\n' => literal.push_str("\\n"),
            '\r' => literal.push_str("\\r"),
            '\t' => literal.push_str("\\t"),
            '\0' => literal.push_str("\\0"),
            ' '..='~' => literal.push(character),
            other => literal.push_str(&format!("\\u{{{:x}}}", u32::from(other))),
        }
    }
    literal.push('"');
    literal
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plain_text_is_quoted() {
        assert_eq!(rust_string_literal("Hello"), "\"Hello\"");
        assert_eq!(rust_string_literal(""), "\"\"");
        assert_eq!(rust_string_literal("a b~{}"), "\"a b~{}\"");
    }

    #[test]
    fn quotes_and_backslashes_are_escaped() {
        assert_eq!(rust_string_literal("say \"hi\""), "\"say \\\"hi\\\"\"");
        assert_eq!(rust_string_literal("a\\b"), "\"a\\\\b\"");
    }

    #[test]
    fn control_characters_are_escaped() {
        assert_eq!(rust_string_literal("a\nb\r\tc\0"), "\"a\\nb\\r\\tc\\0\"");
        assert_eq!(rust_string_literal("\u{1}"), "\"\\u{1}\"");
        assert_eq!(rust_string_literal("\u{7f}"), "\"\\u{7f}\"");
    }

    #[test]
    fn other_characters_use_the_unicode_escape() {
        assert_eq!(rust_string_literal("\u{e9}"), "\"\\u{e9}\"");
        assert_eq!(rust_string_literal("\u{1f600}"), "\"\\u{1f600}\"");
    }
}
