use super::syntax::span;
use crate::literal::string_literal;
use ferroui_designer_core::{SourceSpan, TextPatch};
use syn::{Expr, Lit};

/// A string operand whose Rust quoting and enclosing call survive source edits.
#[derive(Clone, Debug)]
pub(super) struct TextSource {
    literal: SourceSpan,
    spelling: String,
}

impl TextSource {
    pub(super) fn read(expression: &Expr) -> Option<(String, Self)> {
        let Expr::Lit(literal) = expression else {
            return None;
        };
        let Lit::Str(text) = &literal.lit else {
            return None;
        };
        if !literal.attrs.is_empty() {
            return None;
        }
        Some((
            text.value(),
            Self {
                literal: span(text),
                spelling: text.token().to_string(),
            },
        ))
    }

    pub(super) fn parse_call(
        expression: &Expr,
        resolve: &impl Fn(&Expr) -> Option<String>,
        factories: &[&str],
    ) -> Option<(String, Self)> {
        let call = match expression {
            Expr::MethodCall(method)
                if method.attrs.is_empty()
                    && method.turbofish.is_none()
                    && ((method.method == "unwrap" && method.args.is_empty())
                        || (method.method == "expect" && method.args.len() == 1)) =>
            {
                method.receiver.as_ref()
            }
            Expr::Try(operation) if operation.attrs.is_empty() => operation.expr.as_ref(),
            _ => return None,
        };
        let Expr::Call(call) = call else { return None };
        if !call.attrs.is_empty()
            || call.args.len() != 1
            || !factories.contains(&resolve(&call.func)?.as_str())
        {
            return None;
        }
        Self::read(call.args.first()?)
    }

    pub(super) fn plan(&self, text: &str, patches: &mut Vec<TextPatch>) {
        patches.push(TextPatch {
            span: self.literal,
            replacement: string_literal(&self.spelling, text),
        });
    }
}
