use super::super::{FloatSource, color::ColorSource, syntax};
use super::operand;
use ferroui_designer_animation::{AnimationBrush, AnimationValue, BrushForm};
use ferroui_designer_core::{SourceSpan, TextPatch};
use syn::{Expr, ExprCall};

#[derive(Clone, Debug)]
pub(in crate::animation) struct SolidSource {
    color: ColorSource,
    opacity: OpacitySource,
}

#[derive(Clone, Debug)]
enum OpacitySource {
    Explicit(FloatSource),
    Default {
        method: SourceSpan,
        argument: SourceSpan,
        packed: bool,
    },
}

impl SolidSource {
    pub(super) fn read(
        call: &ExprCall,
        method: &str,
        resolve: &impl Fn(&Expr) -> Option<String>,
    ) -> Option<(AnimationBrush, Self)> {
        let color = call.args.first()?;
        let (value, opacity, source) = match (method, call.args.len()) {
            ("with_opacity", 2) | ("with_transforms", 4) => {
                if method == "with_transforms"
                    && (operand::optional(&call.args[2], resolve)?.is_some()
                        || operand::optional(&call.args[3], resolve)?.is_some())
                {
                    return None;
                }
                let opacity = syntax::float(&call.args[1])?;
                (
                    ColorSource::read_with(color, false, resolve)?,
                    opacity.value,
                    OpacitySource::Explicit(opacity),
                )
            }
            ("new" | "from_uint32", 1) => {
                let Expr::Path(factory) = call.func.as_ref() else {
                    return None;
                };
                let packed = method == "from_uint32";
                let value = if packed {
                    ColorSource::read_packed(color)?
                } else {
                    ColorSource::read_with(color, false, resolve)?
                };
                (
                    value,
                    1.0,
                    OpacitySource::Default {
                        method: syntax::span(&factory.path.segments.last()?.ident),
                        argument: syntax::span(color),
                        packed,
                    },
                )
            }
            _ => return None,
        };
        let brush = AnimationBrush::Solid {
            argb: value.0.argb8()?,
            opacity,
            form: BrushForm::Object,
        };
        Some((
            brush,
            Self {
                color: value.1,
                opacity: source,
            },
        ))
    }

    pub(super) fn plan(
        &self,
        old: [u8; 4],
        next: [u8; 4],
        opacity: f64,
        patches: &mut Vec<TextPatch>,
    ) -> Result<(), String> {
        self.color.plan(
            &AnimationValue::from_argb8(old),
            &AnimationValue::from_argb8(next),
            patches,
        )?;
        match self.opacity {
            OpacitySource::Explicit(source) => source.plan(opacity, patches),
            OpacitySource::Default {
                method,
                argument,
                packed,
            } if opacity != 1.0 => {
                patches.push(TextPatch {
                    span: method,
                    replacement: "with_opacity".into(),
                });
                if packed {
                    patches.push(TextPatch {
                        span: SourceSpan::new(argument.start, argument.start),
                        replacement: "::ferroui_base::media::Color::from_uint32(".into(),
                    });
                }
                patches.push(TextPatch {
                    span: SourceSpan::new(argument.end, argument.end),
                    replacement: format!("{}, {opacity:?}", if packed { ")" } else { "" }),
                });
            }
            _ => {}
        }
        Ok(())
    }
}
