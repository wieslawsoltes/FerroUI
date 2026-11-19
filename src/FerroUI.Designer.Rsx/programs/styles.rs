use crate::source_edits::{insert_field, line_indent, line_prefix};
use ferroui_designer_core::{SourceSpan, TextPatch};
use syn::{Expr, spanned::Spanned};

/// Existing array delimiters belong to the author, including commas around opaque members.
pub(crate) enum StyleSlot {
    Missing,
    Array {
        close: usize,
        comma_before: Option<usize>,
    },
    Unavailable,
}

impl StyleSlot {
    pub(crate) fn from_expression(expression: &Expr) -> Self {
        let Expr::Array(array) = expression else {
            return Self::Unavailable;
        };
        let close = array.bracket_token.span.close().byte_range().start;
        let comma_before = if array.elems.trailing_punct() {
            None
        } else {
            array.elems.last().map(|item| item.span().byte_range().end)
        };
        Self::Array {
            close,
            comma_before,
        }
    }

    pub(super) fn insert(
        &self,
        source: &str,
        open: usize,
        rule: impl FnOnce(&str, &str) -> String,
    ) -> Result<Vec<TextPatch>, String> {
        let eol = if source.contains("\r\n") {
            "\r\n"
        } else {
            "\n"
        };
        match *self {
            Self::Missing => {
                let field_indent = format!("{}    ", line_indent(source, open));
                let rule_indent = format!("{field_indent}    ");
                let rule = rule(&rule_indent, eol);
                let field = format!("styles: [{eol}{rule_indent}{rule},{eol}{field_indent}]");
                Ok(vec![insert_field(source, open, &field)])
            }
            Self::Array {
                close,
                comma_before,
            } => {
                let mut patches = Vec::new();
                let prefix = line_prefix(source, Some(close));
                let closing_indent = line_indent(source, close);
                let rule_indent = format!("{closing_indent}    ");
                let rule = rule(&rule_indent, eol);
                let offset = if prefix.is_empty() {
                    close
                } else {
                    close - closing_indent.len()
                };
                let comma = if let Some(comma) = comma_before {
                    if comma == offset {
                        ","
                    } else {
                        patches.push(TextPatch {
                            span: SourceSpan::new(comma, comma),
                            replacement: ",".into(),
                        });
                        ""
                    }
                } else {
                    ""
                };
                let replacement = if prefix.is_empty() {
                    format!("{comma} {rule}, ")
                } else {
                    format!("{comma}{rule_indent}{rule},{eol}")
                };
                patches.push(TextPatch {
                    span: SourceSpan::new(offset, offset),
                    replacement,
                });
                Ok(patches)
            }
            Self::Unavailable => Err(
                "Choose a control with an inline styles array, or add the animation in Rust source"
                    .into(),
            ),
        }
    }
}
