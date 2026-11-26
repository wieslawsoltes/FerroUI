//! Checked f64 operands and patches at their authored use sites.

use super::constant_source::{ConstantReader, ConstantType, ConstantValue};
use super::operand_source::OperandSource;
use super::syntax::{span, unparen};
use ferroui_designer_core::{SourceSpan, TextPatch};
use syn::{Expr, Lit, UnOp};

#[derive(Clone, Debug)]
pub(crate) struct FloatSource {
    pub(crate) value: f64,
    source: OperandSource,
    minus: Option<SourceSpan>,
    explicit_type: bool,
}

impl FloatSource {
    pub(super) fn read(expr: &Expr, constants: &ConstantReader<'_>) -> Option<Self> {
        let (expr, minus) = match unparen(expr) {
            Expr::Unary(unary) if unary.attrs.is_empty() && matches!(unary.op, UnOp::Neg(_)) => {
                (unparen(&unary.expr), Some(span(&unary.op)))
            }
            expr => (expr, None),
        };
        let (value, source, explicit_type) = match expr {
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
                    OperandSource::literal(span(literal)),
                    suffix == "f64",
                )
            }
            Expr::Path(_) => {
                let source = OperandSource::reference(expr)?;
                let ConstantValue::Float(value) = constants.get(expr, ConstantType::F64)? else {
                    return None;
                };
                (value, source, true)
            }
            _ => return None,
        };
        value.is_finite().then_some(FloatSource {
            value: if minus.is_some() { -value } else { value },
            source,
            minus,
            explicit_type,
        })
    }

    pub(super) fn anchor(&self) -> usize {
        self.source.anchor()
    }

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
        self.source
            .plan(format!("{prefix}{:?}{suffix}", value.abs()), patches);
        if let Some(minus) = self.minus.filter(|_| !value.is_sign_negative()) {
            patches.push(TextPatch {
                span: minus,
                replacement: String::new(),
            });
        }
    }
}
