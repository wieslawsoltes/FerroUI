//! Checked f64 operands and patches at their authored use sites.

use super::syntax::{authored_path, span, unparen};
use crate::local_rust::RustSourceScope;
use ferroui_designer_core::{SourceSpan, TextPatch};
use syn::{Expr, Lit, Path, UnOp};

pub(super) struct FloatReader<'a>(pub(super) &'a dyn Fn(&Expr) -> Option<f64>);

#[derive(Clone, Debug)]
pub(crate) struct FloatSource {
    pub(crate) value: f64,
    pub(super) literal: SourceSpan,
    minus: Option<SourceSpan>,
    explicit_type: bool,
    qualifiers: Option<Box<[SourceSpan]>>,
}

pub(super) fn constant_value(path: &Expr, scope: RustSourceScope<'_>) -> Option<f64> {
    let initializer = scope.typed_constant_initializer(
        path,
        &["f64", "core::primitive::f64", "std::primitive::f64"],
    )?;
    FloatReader::LITERALS
        .read(initializer)
        .map(|literal| literal.value)
}

impl FloatReader<'_> {
    pub(super) const LITERALS: Self = Self(&|_| None);

    pub(super) fn read(&self, expr: &Expr) -> Option<FloatSource> {
        let (expr, minus) = match unparen(expr) {
            Expr::Unary(unary) if unary.attrs.is_empty() && matches!(unary.op, UnOp::Neg(_)) => {
                (unparen(&unary.expr), Some(span(&unary.op)))
            }
            expr => (expr, None),
        };
        let (value, literal, explicit_type, qualifiers) = match expr {
            Expr::Lit(value) if value.attrs.is_empty() => {
                let Lit::Float(literal) = &value.lit else {
                    return None;
                };
                let suffix = literal.suffix();
                if !matches!(suffix, "" | "f64") {
                    return None;
                }
                (
                    literal.base10_parse::<f64>().ok()?,
                    span(literal),
                    suffix == "f64",
                    None,
                )
            }
            Expr::Path(path) if path.attrs.is_empty() => {
                authored_path(expr)?;
                let value = (self.0)(expr)?;
                let (literal, qualifiers) = path_spans(&path.path)?;
                (value, literal, true, qualifiers)
            }
            _ => return None,
        };
        value.is_finite().then_some(FloatSource {
            value: if minus.is_some() { -value } else { value },
            literal,
            minus,
            explicit_type,
            qualifiers,
        })
    }
}

impl FloatSource {
    pub(crate) fn plan(&self, value: f64, patches: &mut Vec<TextPatch>) {
        if self.value == value {
            return;
        }
        let prefix = if self.minus.is_none() && value.is_sign_negative() {
            "-"
        } else {
            ""
        };
        let suffix = if self.explicit_type { "f64" } else { "" };
        patches.push(TextPatch {
            span: self.literal,
            replacement: format!("{prefix}{:?}{suffix}", value.abs()),
        });
        if let Some(qualifiers) = &self.qualifiers {
            patches.extend(qualifiers.iter().map(|&span| TextPatch {
                span,
                replacement: String::new(),
            }));
        }
        if let Some(minus) = self.minus.filter(|_| !value.is_sign_negative()) {
            patches.push(TextPatch {
                span: minus,
                replacement: String::new(),
            });
        }
    }
}

fn path_spans(path: &Path) -> Option<(SourceSpan, Option<Box<[SourceSpan]>>)> {
    // Replace adjacent path tokens together. Separate patches keep any comments
    // between path segments at their original positions.
    let mut spans: Vec<SourceSpan> = Vec::new();
    let mut push = |span: SourceSpan| {
        if let Some(last) = spans.last_mut().filter(|last| last.end == span.start) {
            last.end = span.end;
        } else {
            spans.push(span);
        }
    };
    if let Some(leading) = &path.leading_colon {
        push(span(leading));
    }
    for segment in path.segments.pairs() {
        push(span(&segment.value().ident));
        if let Some(separator) = segment.punct() {
            push(span(separator));
        }
    }
    let literal = spans.pop()?;
    Some((
        literal,
        (!spans.is_empty()).then(|| spans.into_boxed_slice()),
    ))
}
