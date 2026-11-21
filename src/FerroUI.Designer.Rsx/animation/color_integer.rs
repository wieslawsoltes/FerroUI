use super::syntax::span;
use ferroui_designer_core::{SourceSpan, TextPatch};
use syn::{Expr, Lit};

/// An unsigned channel or packed ARGB literal, with its authored spelling.
#[derive(Clone, Debug)]
pub(super) struct ColorInteger {
    pub(super) value: u32,
    span: SourceSpan,
    spelling: String,
    suffix: &'static str,
}

impl ColorInteger {
    pub(super) fn read(
        mut expression: &Expr,
        integer_type: &'static str,
        max: u32,
    ) -> Option<Self> {
        let literal = loop {
            match expression {
                Expr::Paren(group) if group.attrs.is_empty() => expression = &group.expr,
                Expr::Group(group) if group.attrs.is_empty() => expression = &group.expr,
                Expr::Lit(literal) if literal.attrs.is_empty() => break &literal.lit,
                _ => return None,
            }
        };
        let Lit::Int(literal) = literal else {
            return None;
        };
        let suffix = match literal.suffix() {
            "" => "",
            suffix if suffix == integer_type => integer_type,
            _ => return None,
        };
        let value = literal.base10_parse().ok().filter(|value| *value <= max)?;
        Some(Self {
            value,
            span: span(literal),
            spelling: literal.to_string(),
            suffix,
        })
    }

    pub(super) fn patch(&self, value: u32) -> Option<TextPatch> {
        (self.value != value).then(|| TextPatch {
            span: self.span,
            replacement: self.replacement(value),
        })
    }

    fn replacement(&self, value: u32) -> String {
        let spelling = &self.spelling[..self.spelling.len() - self.suffix.len()];
        let (prefix, digits, output) = if let Some(digits) = spelling.strip_prefix("0x") {
            let uppercase = digits.bytes().any(|byte| byte.is_ascii_uppercase())
                && !digits.bytes().any(|byte| byte.is_ascii_lowercase());
            let output = if uppercase {
                format!("{value:X}")
            } else {
                format!("{value:x}")
            };
            ("0x", digits, output)
        } else if let Some(digits) = spelling.strip_prefix("0o") {
            ("0o", digits, format!("{value:o}"))
        } else if let Some(digits) = spelling.strip_prefix("0b") {
            ("0b", digits, format!("{value:b}"))
        } else {
            ("", spelling, value.to_string())
        };
        let magnitude = digits.trim_end_matches('_');
        if prefix.is_empty() && !magnitude.starts_with('0') && !magnitude.contains('_') {
            let separators = &digits[magnitude.len()..];
            return format!("{output}{separators}{}", self.suffix);
        }
        // Align new digits from the right so changing their width keeps the
        // existing group separators and the placement of the type suffix.
        let width = digits.bytes().filter(|&byte| byte != b'_').count();
        let output = format!("{output:0>width$}");
        let mut replacement = String::with_capacity(self.spelling.len().max(output.len()));
        replacement.push_str(prefix);
        let leading = digits.len() - digits.trim_start_matches('_').len();
        replacement.push_str(&digits[..leading]);
        let extra = output.len() - width;
        replacement.push_str(&output[..extra]);
        let mut position = extra;
        for digit in digits[leading..].bytes() {
            if digit == b'_' {
                replacement.push('_');
            } else {
                replacement.push(char::from(output.as_bytes()[position]));
                position += 1;
            }
        }
        replacement.push_str(self.suffix);
        replacement
    }
}
