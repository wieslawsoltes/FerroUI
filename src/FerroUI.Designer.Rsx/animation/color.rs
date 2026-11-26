use super::color_integer::ColorInteger;
use super::color_text::TextColor;
use super::constant_source::{ConstantReader, ConstantType, ConstantValue};
use super::operand_source::OperandSource;
use super::syntax::{span, unparen};
use ferroui_designer_animation::AnimationValue;
use ferroui_designer_core::{SourceSpan, TextPatch};
use syn::{Expr, ExprCall};

#[derive(Clone, Debug)]
pub(super) enum ColorSource {
    Text(TextColor),
    Reference(OperandSource),
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
    pub(super) fn read(
        expr: &Expr,
        brush: bool,
        resolve: &impl Fn(&Expr) -> Option<String>,
        constants: &ConstantReader<'_>,
    ) -> Option<(AnimationValue, Self)> {
        let expr = unparen(expr);
        if let Some((value, source)) = TextColor::read(expr, brush, resolve) {
            return Some((value, Self::Text(source)));
        }
        if let Expr::Path(_) = expr {
            let source = OperandSource::reference(expr)?;
            let ConstantValue::Color(bytes) = constants.get(expr, ConstantType::Color)? else {
                return None;
            };
            return Some((AnimationValue::from_argb8(bytes), Self::Reference(source)));
        }
        if let Expr::Struct(color) = expr {
            return Self::read_struct(color, resolve, constants);
        }
        let Expr::Call(call) = expr else {
            return None;
        };
        if !call.attrs.is_empty() {
            return None;
        }
        let factory = resolve(&call.func)?;
        if let Some(argument) = packed_argument(call, &factory) {
            return Self::read_packed(argument, constants);
        }
        let method = color_method(&factory)?;
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
            .map(|argument| ColorInteger::read(argument, ConstantType::U8, constants))
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

    pub(super) fn read_packed(
        argument: &Expr,
        constants: &ConstantReader<'_>,
    ) -> Option<(AnimationValue, Self)> {
        let packed = ColorInteger::read(argument, ConstantType::U32, constants)?;
        let value = AnimationValue::from_argb8(packed.value().to_be_bytes());
        Some((value, Self::Packed(packed)))
    }

    fn read_struct(
        color: &syn::ExprStruct,
        resolve: &impl Fn(&Expr) -> Option<String>,
        constants: &ConstantReader<'_>,
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
        if !ConstantType::Color.accepts(&resolve(&name)?) {
            return None;
        }
        let mut channels = Vec::with_capacity(4);
        for name in ["a", "r", "g", "b"] {
            let field = color.fields.iter().find(
                |field| matches!(&field.member, syn::Member::Named(member) if member == name),
            )?;
            if !field.attrs.is_empty() || field.colon_token.is_none() {
                return None;
            }
            channels.push(ColorInteger::read(
                &field.expr,
                ConstantType::U8,
                constants,
            )?);
        }
        Some(Self::components(channels, None))
    }

    fn components(parts: Vec<ColorInteger>, rgb: Option<RgbSource>) -> (AnimationValue, Self) {
        let mut argb = [255; 4];
        for (value, part) in argb[4 - parts.len()..].iter_mut().zip(&parts) {
            *value = part.value() as u8;
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
            Self::Text(text) => patches.push(text.patch(value, bytes)),
            Self::Reference(source) => source.plan(rust_literal(bytes), patches),
            Self::Packed(packed) => packed.plan(u32::from_be_bytes(bytes), patches),
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
                for (part, &value) in parts.iter().zip(&bytes[4 - parts.len()..]) {
                    part.plan(u32::from(value), patches);
                }
            }
        }
        Ok(())
    }
}

pub(super) fn rust_literal([a, r, g, b]: [u8; 4]) -> String {
    format!("::ferroui_base::media::Color::from_argb({a}, {r}, {g}, {b})")
}

fn color_method(factory: &str) -> Option<&str> {
    factory.strip_prefix("ferroui_base::media::Color::")
}

pub(super) fn packed_argument<'a>(call: &'a ExprCall, factory: &str) -> Option<&'a Expr> {
    (color_method(factory) == Some("from_uint32") && call.attrs.is_empty() && call.args.len() == 1)
        .then(|| &call.args[0])
}
