use super::{syntax, text_source::TextSource};
use crate::blocks::keep_comments;
use ferroui_designer_animation::AnimationTransformList;
use ferroui_designer_core::{SourceSpan, TextPatch};
use syn::{Expr, Type, spanned::Spanned};

const PARSE: &[&str] = &["ferroui_base::media::transformation::TransformOperations::parse"];
const IDENTITY: &str = "ferroui_base::media::transformation::TransformOperations::identity";

#[derive(Clone, Debug)]
pub(super) enum TransformListSource {
    Text(TextSource),
    Empty {
        span: SourceSpan,
        comments: String,
        constructor: bool,
    },
}

impl TransformListSource {
    pub(super) fn read(
        expression: &Expr,
        resolve: &impl Fn(&Expr) -> Option<String>,
    ) -> Option<(AnimationTransformList, Self)> {
        if let Some((text, source)) = TextSource::read(expression) {
            return Some((
                AnimationTransformList::parse(&text).ok()?,
                Self::Text(source),
            ));
        }
        if matches!(
            resolve(expression).as_deref(),
            Some("None" | "std::option::Option::None" | "core::option::Option::None")
        ) {
            return Self::empty(expression, false);
        }
        let value = match expression {
            Expr::Call(call)
                if call.attrs.is_empty()
                    && call.args.len() == 1
                    && matches!(
                        resolve(&call.func).as_deref(),
                        Some("Some" | "std::option::Option::Some" | "core::option::Option::Some")
                    ) =>
            {
                let Expr::Cast(value) = call.args.first()? else {
                    return None;
                };
                if !value.attrs.is_empty() || !transform_cast(&value.ty, resolve) {
                    return None;
                }
                value.expr.as_ref()
            }
            _ => expression,
        };
        if let Some((text, source)) = TextSource::parse_call(value, resolve, PARSE) {
            return Some((
                AnimationTransformList::parse(&text).ok()?,
                Self::Text(source),
            ));
        }
        let Expr::Call(call) = value else { return None };
        if !call.attrs.is_empty()
            || !call.args.is_empty()
            || resolve(&call.func)?.as_str() != IDENTITY
        {
            return None;
        }
        Self::empty(value, true)
    }

    fn empty(expression: &Expr, constructor: bool) -> Option<(AnimationTransformList, Self)> {
        let comments = keep_comments(&expression.span().source_text()?).ok()?;
        Some((
            AnimationTransformList::parse("none").ok()?,
            Self::Empty {
                span: syntax::span(expression),
                comments,
                constructor,
            },
        ))
    }

    pub(super) fn plan(&self, value: &str, patches: &mut Vec<TextPatch>) {
        match self {
            Self::Text(source) => source.plan(value, patches),
            Self::Empty {
                span,
                comments,
                constructor,
            } => {
                let literal = format!("{value:?}");
                let expression = if *constructor {
                    format!(
                        "::ferroui_base::media::transformation::TransformOperations::parse({literal}).unwrap()"
                    )
                } else {
                    literal
                };
                patches.push(TextPatch {
                    span: *span,
                    replacement: format!("{comments}{expression}"),
                });
            }
        }
    }
}

fn transform_cast(ty: &Type, resolve: &impl Fn(&Expr) -> Option<String>) -> bool {
    if matches!(ty, Type::Infer(_)) {
        return true;
    }
    syntax::rc_interface_cast(ty, "ferroui_base::media::ITransform", &|path| {
        Some(match resolve(path)?.as_str() {
            "Rc" => "std::rc::Rc".into(),
            "ITransform" => "ferroui_base::media::ITransform".into(),
            other => other.into(),
        })
    })
}
