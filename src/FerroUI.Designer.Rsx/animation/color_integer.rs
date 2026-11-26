use super::constant_source::{ConstantReader, ConstantType, ConstantValue};
use super::operand_source::OperandSource;
use super::syntax::{span, unparen};
use ferroui_designer_core::{SourceSpan, TextPatch};
use syn::{Expr, Lit};

/// An unsigned channel or packed ARGB literal, with its authored spelling.
#[derive(Clone, Debug)]
pub(super) struct ColorInteger {
    literal: IntegerLiteral,
    source: OperandSource,
}

#[derive(Clone, Debug)]
pub(super) struct IntegerLiteral {
    value: u32,
    spelling: String,
    suffix: &'static str,
}

impl ColorInteger {
    pub(super) fn read(
        expression: &Expr,
        ty: ConstantType,
        constants: &ConstantReader<'_>,
    ) -> Option<Self> {
        let expression = unparen(expression);
        if let Some((literal, span)) = IntegerLiteral::read(expression, ty) {
            return Some(Self {
                literal,
                source: OperandSource::literal(span),
            });
        }
        let source = OperandSource::reference(expression)?;
        let ConstantValue::Integer(literal) = constants.get(expression, ty)? else {
            return None;
        };
        Some(Self { literal, source })
    }

    pub(super) fn value(&self) -> u32 {
        self.literal.value
    }

    pub(super) fn plan(&self, value: u32, patches: &mut Vec<TextPatch>) {
        if self.value() != value {
            self.source.plan(self.literal.replacement(value), patches);
        }
    }
}

impl IntegerLiteral {
    pub(super) fn read(expression: &Expr, ty: ConstantType) -> Option<(Self, SourceSpan)> {
        let (integer_type, max) = match ty {
            ConstantType::U8 => ("u8", u32::from(u8::MAX)),
            ConstantType::U32 => ("u32", u32::MAX),
            _ => return None,
        };
        let Expr::Lit(expression) = unparen(expression) else {
            return None;
        };
        if !expression.attrs.is_empty() {
            return None;
        }
        let Lit::Int(literal) = &expression.lit else {
            return None;
        };
        let suffix = match literal.suffix() {
            "" => "",
            suffix if suffix == integer_type => integer_type,
            _ => return None,
        };
        let value = literal.base10_parse().ok().filter(|value| *value <= max)?;
        Some((
            Self {
                value,
                spelling: literal.to_string(),
                suffix,
            },
            span(literal),
        ))
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
