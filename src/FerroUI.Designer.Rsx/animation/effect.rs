use super::syntax::{path, span};
use crate::literal::string_literal;
use ferroui_designer_animation::AnimationEffect;
use ferroui_designer_core::{SourceSpan, TextPatch};
use syn::{Expr, Lit};

#[derive(Clone, Debug)]
pub(super) struct EffectSource {
    span: SourceSpan,
    spelling: String,
}

impl EffectSource {
    pub(super) fn read(
        expression: &Expr,
        resolve: &impl Fn(&Expr) -> Option<String>,
    ) -> Option<(AnimationEffect, Self)> {
        let (inner, wrapped) = match expression {
            Expr::Call(call)
                if call.attrs.is_empty()
                    && call.args.len() == 1
                    && matches!(
                        path(&call.func)?.as_str(),
                        "Some" | "std::option::Option::Some" | "core::option::Option::Some"
                    ) =>
            {
                (call.args.first()?, true)
            }
            _ => (expression, false),
        };
        let Expr::MethodCall(method) = inner else {
            return None;
        };
        let returns_effect = (method.method == "unwrap" && method.args.is_empty())
            || (method.method == "expect" && method.args.len() == 1);
        let returns_option = !wrapped && method.method == "ok" && method.args.is_empty();
        if !method.attrs.is_empty()
            || method.turbofish.is_some()
            || !(returns_effect || returns_option)
        {
            return None;
        }
        let Expr::Call(call) = method.receiver.as_ref() else {
            return None;
        };
        if !call.attrs.is_empty()
            || call.args.len() != 1
            || resolve(&call.func)?.as_str() != "ferroui_base::media::Effect::parse"
        {
            return None;
        }
        let Expr::Lit(literal) = call.args.first()? else {
            return None;
        };
        let Lit::Str(text) = &literal.lit else {
            return None;
        };
        if !literal.attrs.is_empty() {
            return None;
        }
        let value = AnimationEffect::parse_native(&text.value()).ok()?;
        Some((
            value,
            Self {
                span: span(text),
                spelling: text.token().to_string(),
            },
        ))
    }

    pub(super) fn patch(&self, text: &str) -> TextPatch {
        TextPatch {
            span: self.span,
            replacement: string_literal(&self.spelling, text),
        }
    }
}
