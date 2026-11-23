use super::effect_constructor::ConstructorSource;
use super::number_source::FloatReader;
use super::syntax::{span, unparen};
use crate::literal::string_literal;
use ferroui_designer_animation::{AnimationEffect, EffectForm};
use ferroui_designer_core::{SourceSpan, TextPatch};
use syn::{Expr, Lit};

#[derive(Clone, Debug)]
pub(super) enum EffectSource {
    Literal { span: SourceSpan, spelling: String, project_effects: bool },
    Object(ConstructorSource),
}

impl EffectSource {
    pub(super) fn read(
        expression: &Expr,
        resolve: &impl Fn(&Expr) -> Option<String>,
        project_effects: bool,
        numbers: &FloatReader<'_>,
    ) -> Option<(AnimationEffect, Self)> {
        let expression = unparen(expression);
        let (inner, wrapped) = match expression {
            Expr::Call(call)
                if call.attrs.is_empty()
                    && call.args.len() == 1
                    && matches!(
                        resolve(&call.func).as_deref(),
                        Some("Some" | "std::option::Option::Some" | "core::option::Option::Some")
                    ) =>
            {
                (call.args.first()?, true)
            }
            _ => (expression, false),
        };
        let Expr::MethodCall(method) = unparen(inner) else {
            let (value, source) = ConstructorSource::read(inner, resolve, numbers)?;
            return Some((value, Self::Object(source)));
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
        let Expr::Call(call) = unparen(&method.receiver) else {
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
        let value = parse(&text.value(), project_effects).ok()?;
        Some((
            value,
            Self::Literal {
                span: span(text),
                spelling: text.token().to_string(),
                project_effects,
            },
        ))
    }

    pub(super) fn plan(
        &self,
        old: &AnimationEffect,
        text: &str,
        patches: &mut Vec<TextPatch>,
    ) -> Result<(), String> {
        match self {
            Self::Object(source) => source.plan(old, &parse(text, false)?, patches),
            Self::Literal { span, spelling, project_effects } => {
                let next = parse(text, *project_effects)?;
                let form = next.form();
                let literal = next.with_form(EffectForm::Literal);
                if literal.form() != EffectForm::Literal {
                    return Err(
                        "Use a native effect constructor for direction or independent opacity"
                            .into(),
                    );
                }
                if old != &literal {
                    let value = if form == EffectForm::Literal {
                        text.to_owned()
                    } else {
                        literal.to_xaml()
                    };
                    patches.push(TextPatch {
                        span: *span,
                        replacement: string_literal(spelling, &value),
                    });
                }
                Ok(())
            }
        }
    }
}

pub(super) fn parse(text: &str, project_effects: bool) -> Result<AnimationEffect, String> {
    let parsed = if project_effects {
        AnimationEffect::parse_for_project_host(text)
    } else {
        AnimationEffect::parse_native(text)
    };
    parsed.map_err(|error| error.to_string())
}
