use super::syntax::span;
use crate::blocks::keep_comments;
use crate::literal::string_literal;
use ferroui_base::media::{Color, KnownColors};
use ferroui_designer_animation::AnimationValue;
use ferroui_designer_core::{SourceSpan, TextPatch};
use syn::{spanned::Spanned, Expr, Lit, LitStr};

#[derive(Clone, Debug)]
pub(super) enum TextColor {
    String { span: SourceSpan, spelling: String },
    Named { span: SourceSpan, comments: String },
}

impl TextColor {
    pub(super) fn read(
        expression: &Expr,
        brush: bool,
        resolve: &impl Fn(&Expr) -> Option<String>,
    ) -> Option<(AnimationValue, Self)> {
        let value = match expression {
            Expr::Lit(_) if brush => return Self::literal(string(expression)?),
            Expr::Path(path) if path.attrs.is_empty() => {
                let qualified = resolve(expression)?;
                let name = qualified.strip_prefix("ferroui_base::media::Colors::")?;
                let color = KnownColors::from_constant_name(name)?.to_color();
                let comments = keep_comments(&path.span().source_text()?).ok()?;
                return Some((
                    animation_value(color),
                    Self::Named {
                        span: span(path),
                        comments,
                    },
                ));
            }
            Expr::Try(value) if value.attrs.is_empty() => value.expr.as_ref(),
            Expr::MethodCall(method)
                if method.attrs.is_empty()
                    && method.turbofish.is_none()
                    && (method.method == "unwrap" && method.args.is_empty()
                        || method.method == "expect" && method.args.len() == 1) =>
            {
                method.receiver.as_ref()
            }
            _ => return None,
        };
        let Expr::Call(call) = value else {
            return None;
        };
        if !call.attrs.is_empty()
            || !matches!(
                resolve(&call.func)?.as_str(),
                "ferroui_base::media::Color::parse" | "ferroui_base::media::Color::try_parse"
            )
            || call.args.len() != 1
        {
            return None;
        }
        Self::literal(string(&call.args[0])?)
    }

    fn literal(text: &LitStr) -> Option<(AnimationValue, Self)> {
        let color = Color::try_parse(&text.value())?;
        Some((
            animation_value(color),
            Self::String {
                span: span(text),
                spelling: text.token().to_string(),
            },
        ))
    }

    pub(super) fn patch(&self, value: &AnimationValue, bytes: [u8; 4]) -> TextPatch {
        match self {
            Self::String { span, spelling } => TextPatch {
                span: *span,
                replacement: string_literal(spelling, &value.to_xaml()),
            },
            Self::Named { span, comments } => {
                let packed = u32::from_be_bytes(bytes);
                TextPatch {
                    span: *span,
                    replacement: format!(
                        "{comments}::ferroui_base::media::Color::from_uint32(0x{packed:08X})"
                    ),
                }
            }
        }
    }
}

fn string(expression: &Expr) -> Option<&LitStr> {
    let Expr::Lit(literal) = expression else {
        return None;
    };
    let Lit::Str(text) = &literal.lit else {
        return None;
    };
    literal.attrs.is_empty().then_some(text)
}

fn animation_value(color: Color) -> AnimationValue {
    AnimationValue::from_argb8(color.to_uint32().to_be_bytes())
}
