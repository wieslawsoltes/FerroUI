mod arguments;
mod operand;
mod stops;

use super::{color::ColorSource, syntax, FloatSource};
use arguments::Arguments;
use ferroui_designer_animation::{
    AnimationBrush, AnimationGradient, AnimationValue, BrushForm, GradientShape,
};
use ferroui_designer_core::{SourceSpan, TextPatch};
use stops::StopsSource;
use syn::{Expr, Type};

const IMMUTABLE: &str = "ferroui_base::media::immutable::Immutable";

#[derive(Clone, Debug)]
pub(super) enum BrushSource {
    Gradient(GradientSource),
    Solid {
        color: ColorSource,
        opacity: FloatSource,
    },
}

#[derive(Clone, Debug)]
pub(super) struct GradientSource {
    stops: StopsSource,
    constructor: Constructor,
}

#[derive(Clone, Debug)]
enum Constructor {
    Full(Box<Arguments>),
    Defaults {
        name: SourceSpan,
        after_stops: usize,
    },
}

impl BrushSource {
    pub(super) fn read(
        expression: &Expr,
        resolve: &impl Fn(&Expr) -> Option<String>,
    ) -> Option<(AnimationValue, Self)> {
        let operand = syntax::unparen(operand::optional(expression, resolve)??);
        let operand = match operand {
            Expr::Cast(cast)
                if cast.attrs.is_empty()
                    && (matches!(&*cast.ty, Type::Infer(_))
                        || syntax::rc_interface_cast(
                            &cast.ty,
                            "ferroui_base::media::IBrush",
                            resolve,
                        )) =>
            {
                syntax::unparen(&cast.expr)
            }
            _ => operand,
        };
        let Expr::Call(call) = syntax::unparen(operand::rc(operand, resolve)?) else {
            return None;
        };
        if !call.attrs.is_empty() {
            return None;
        }
        let resolved = resolve(&call.func)?;
        let (class, method) = resolved.strip_prefix(IMMUTABLE)?.rsplit_once("::")?;
        if (class, method) == ("SolidColorBrush", "with_opacity") && call.args.len() == 2 {
            let (value, color) = ColorSource::read_with(&call.args[0], false, resolve)?;
            let opacity = syntax::float(&call.args[1])?;
            let brush = AnimationBrush::Solid {
                argb: value.argb8()?,
                opacity: opacity.value,
                form: BrushForm::Object,
            };
            return Some((AnimationValue::Brush(brush), Self::Solid { color, opacity }));
        }
        let shape = GradientShape::defaults(class)?;
        let (values, stops) = StopsSource::read(call.args.first()?, resolve)?;
        let mut gradient = AnimationGradient::from_stops(shape, values);
        gradient.transform_origin = ferroui_base::RelativePoint::TOP_LEFT.into();
        let (gradient, constructor) = match method {
            "from_stops" if call.args.len() == 1 => {
                let Expr::Path(path) = call.func.as_ref() else {
                    return None;
                };
                let name = syntax::span(&path.path.segments.last()?.ident);
                let after_stops = syntax::span(call.args.first()?).end;
                (gradient, Constructor::Defaults { name, after_stops })
            }
            "new" | "with_radius" => {
                let shared_radius = if method == "with_radius" {
                    let Expr::Path(path) = call.func.as_ref() else {
                        return None;
                    };
                    Some(syntax::span(&path.path.segments.last()?.ident))
                } else {
                    None
                };
                let operands = call.args.iter().skip(1).collect::<Vec<_>>();
                let (gradient, args) =
                    Arguments::read(gradient, &operands, shared_radius, resolve)?;
                (gradient, Constructor::Full(Box::new(args)))
            }
            _ => return None,
        };
        let value = AnimationValue::Brush(AnimationBrush::gradient(gradient));
        value.validate().ok()?;
        Some((value, Self::Gradient(GradientSource { stops, constructor })))
    }

    pub(super) fn plan(
        &self,
        old: &AnimationBrush,
        new: &AnimationBrush,
        patches: &mut Vec<TextPatch>,
    ) -> Result<(), String> {
        if let (
            Self::Solid { color, opacity },
            AnimationBrush::Solid { argb: old, .. },
            AnimationBrush::Solid {
                argb: next,
                opacity: next_opacity,
                ..
            },
        ) = (self, old, new)
        {
            color.plan(
                &AnimationValue::from_argb8(*old),
                &AnimationValue::from_argb8(*next),
                patches,
            )?;
            opacity.plan(*next_opacity, patches);
            return Ok(());
        }
        let (Self::Gradient(source), Some(old), Some(new)) =
            (self, old.as_gradient(), new.as_gradient())
        else {
            return Err("Keep the Rust brush constructor's type when editing its key".into());
        };
        if old.shape.class_name() != new.shape.class_name() {
            return Err("Keep the Rust gradient constructor's shape when editing its key".into());
        }
        source.stops.plan(&old.stops, &new.stops, patches)?;
        match &source.constructor {
            Constructor::Full(args) => args.plan(old, new, patches),
            &Constructor::Defaults { name, after_stops } => {
                let mut geometry = new.clone();
                geometry.stops.clone_from(&old.stops);
                if geometry != *old {
                    patches.push(TextPatch {
                        span: name,
                        replacement: "new".into(),
                    });
                    patches.push(TextPatch {
                        span: SourceSpan::new(after_stops, after_stops),
                        replacement: format!(", {}", arguments::literal(new)?),
                    });
                }
                Ok(())
            }
        }
    }
}

pub(super) fn literal(brush: &AnimationBrush) -> Result<String, String> {
    let call = match brush {
        AnimationBrush::Gradient { value, .. } => {
            let class = value.shape.class_name();
            let stops = stops::literal(&value.stops);
            let args = arguments::literal(value)?;
            format!("::{IMMUTABLE}{class}::new({stops}, {args})")
        }
        AnimationBrush::Solid { argb, opacity, .. } => {
            let color = stops::color_literal(*argb);
            format!("::{IMMUTABLE}SolidColorBrush::with_opacity({color}, {opacity:?})")
        }
    };
    Ok(format!(
        "Some(::std::rc::Rc::new({call}) as ::std::rc::Rc<dyn ::ferroui_base::media::IBrush>)"
    ))
}
