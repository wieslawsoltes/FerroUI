use super::syntax::{path, span};
use ferroui_designer_animation::{AnimationValue, ValueKind};
use ferroui_designer_core::{SourceSpan, TextPatch};
use syn::{Expr, Lit};

#[derive(Clone, Debug)]
pub(super) enum ColorSource {
    BrushText(SourceSpan),
    Components {
        parts: Vec<ByteSource>,
        rgb: Option<RgbSource>,
    },
}

#[derive(Clone, Copy, Debug)]
pub(super) struct RgbSource {
    method: SourceSpan,
    insert_alpha: SourceSpan,
}

#[derive(Clone, Copy, Debug)]
pub(super) struct ByteSource {
    value: u8,
    span: SourceSpan,
    suffixed: bool,
}

impl ColorSource {
    pub(super) fn read(expr: &Expr, brush: bool) -> Option<(AnimationValue, Self)> {
        Self::read_with(expr, brush, path)
    }

    pub(super) fn read_with(expr: &Expr, brush: bool, resolve: impl Fn(&Expr) -> Option<String>) -> Option<(AnimationValue, Self)> {
        if let (
            true,
            Expr::Lit(syn::ExprLit {
                lit: Lit::Str(text),
                ..
            }),
        ) = (brush, expr)
        {
            let value = AnimationValue::parse(&text.value(), ValueKind::Color).ok()?;
            return Some((value, Self::BrushText(span(text))));
        }
        let Expr::Call(call) = expr else { return None };
        let count = match resolve(&call.func)?.strip_prefix("ferroui_base::media::Color::")? {
            "new" | "from_argb" => 4,
            "from_rgb" => 3,
            _ => return None,
        };
        if call.args.len() != count {
            return None;
        }
        let parts = call
            .args
            .iter()
            .map(ByteSource::read)
            .collect::<Option<Vec<_>>>()?;
        let mut argb = [255; 4];
        for (value, part) in argb[4 - count..].iter_mut().zip(&parts) {
            *value = part.value;
        }
        let rgb = if count == 3 {
            let Expr::Path(factory) = call.func.as_ref() else {
                return None;
            };
            let insert = call.paren_token.span.open().byte_range().end;
            Some(RgbSource {
                method: span(&factory.path.segments.last()?.ident),
                insert_alpha: SourceSpan {
                    start: insert,
                    end: insert,
                },
            })
        } else {
            None
        };
        Some((
            AnimationValue::from_argb8(argb),
            Self::Components { parts, rgb },
        ))
    }

    pub(super) fn plan(
        &self,
        previous: &AnimationValue,
        value: &AnimationValue,
        patches: &mut Vec<TextPatch>,
    ) -> Result<(), String> {
        if previous == value {
            return Ok(());
        }
        let bytes = value.argb8().ok_or("Enter a hexadecimal color value")?;
        match self {
            Self::BrushText(span) => patches.push(TextPatch {
                span: *span,
                replacement: format!("{:?}", value.to_xaml()),
            }),
            Self::Components { parts, rgb } => {
                if let Some(rgb) = rgb.filter(|_| bytes[0] != 255) {
                    patches.push(TextPatch {
                        span: rgb.method,
                        replacement: "from_argb".into(),
                    });
                    patches.push(TextPatch {
                        span: rgb.insert_alpha,
                        replacement: format!("{}, ", bytes[0]),
                    });
                }
                for (part, value) in parts.iter().zip(&bytes[4 - parts.len()..]) {
                    if part.value != *value {
                        let suffix = if part.suffixed { "u8" } else { "" };
                        patches.push(TextPatch {
                            span: part.span,
                            replacement: format!("{value}{suffix}"),
                        });
                    }
                }
            }
        }
        Ok(())
    }
}

impl ByteSource {
    fn read(expr: &Expr) -> Option<Self> {
        let Expr::Lit(literal) = expr else {
            return None;
        };
        let Lit::Int(byte) = &literal.lit else {
            return None;
        };
        match byte.suffix() {
            "" | "u8" => Some(Self {
                value: byte.base10_parse().ok()?,
                span: span(byte),
                suffixed: byte.suffix() == "u8",
            }),
            _ => None,
        }
    }
}
