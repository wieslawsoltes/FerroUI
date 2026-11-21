use super::color_integer::ColorInteger;
use super::syntax::{path, span};
use ferroui_designer_animation::{AnimationValue, ValueKind};
use ferroui_designer_core::{SourceSpan, TextPatch};
use syn::{Expr, Lit};

#[derive(Clone, Debug)]
pub(super) enum ColorSource {
    BrushText(SourceSpan),
    Packed(ColorInteger),
    Components {
        parts: Vec<ColorInteger>,
        rgb: Option<RgbSource>,
    },
}

#[derive(Clone, Copy, Debug)]
pub(super) struct RgbSource {
    method: SourceSpan,
    insert_alpha: SourceSpan,
}

impl ColorSource {
    pub(super) fn read(expr: &Expr, brush: bool) -> Option<(AnimationValue, Self)> {
        Self::read_with(expr, brush, path)
    }

    pub(super) fn read_with(
        expr: &Expr,
        brush: bool,
        resolve: impl Fn(&Expr) -> Option<String>,
    ) -> Option<(AnimationValue, Self)> {
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
        if let Expr::Struct(color) = expr {
            return Self::read_struct(color, resolve);
        }
        let Expr::Call(call) = expr else {
            return None;
        };
        let factory = resolve(&call.func)?;
        let method = factory.strip_prefix("ferroui_base::media::Color::")?;
        if method == "from_uint32" && call.args.len() == 1 {
            let packed = ColorInteger::read(&call.args[0], "u32", u32::MAX)?;
            let value = AnimationValue::from_argb8(packed.value.to_be_bytes());
            return Some((value, Self::Packed(packed)));
        }
        let count = match method {
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
            .map(read_channel)
            .collect::<Option<Vec<_>>>()?;
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
        Some(Self::components(parts, rgb))
    }

    fn read_struct(
        color: &syn::ExprStruct,
        resolve: impl Fn(&Expr) -> Option<String>,
    ) -> Option<(AnimationValue, Self)> {
        if !color.attrs.is_empty()
            || color.fields.len() != 4
            || color.qself.is_some()
            || color.rest.is_some()
        {
            return None;
        }
        let name = Expr::Path(syn::ExprPath {
            attrs: Vec::new(),
            qself: None,
            path: color.path.clone(),
        });
        if resolve(&name)?.as_str() != "ferroui_base::media::Color" {
            return None;
        }
        let mut channels = Vec::with_capacity(4);
        for name in ["a", "r", "g", "b"] {
            let field = color.fields.iter().find(
                |field| matches!(&field.member, syn::Member::Named(member) if member == name),
            )?;
            if !field.attrs.is_empty() {
                return None;
            }
            channels.push(read_channel(&field.expr)?);
        }
        Some(Self::components(channels, None))
    }

    fn components(parts: Vec<ColorInteger>, rgb: Option<RgbSource>) -> (AnimationValue, Self) {
        let mut argb = [255; 4];
        for (value, part) in argb[4 - parts.len()..].iter_mut().zip(&parts) {
            *value = part.value as u8;
        }
        (
            AnimationValue::from_argb8(argb),
            Self::Components { parts, rgb },
        )
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
            Self::Packed(packed) => patches.extend(packed.patch(u32::from_be_bytes(bytes))),
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
                patches.extend(
                    parts
                        .iter()
                        .zip(&bytes[4 - parts.len()..])
                        .filter_map(|(part, &value)| part.patch(u32::from(value))),
                );
            }
        }
        Ok(())
    }
}

fn read_channel(expression: &Expr) -> Option<ColorInteger> {
    ColorInteger::read(expression, "u8", u32::from(u8::MAX))
}
