use super::{color::ColorSource, number_source::FloatReader, syntax, FloatSource};
use ferroui_designer_animation::{AnimationEffect, AnimationValue, EffectForm};
use ferroui_designer_core::TextPatch;
use syn::Expr;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Kind {
    Blur,
    Offset,
    Directional,
}
#[derive(Clone, Debug)]
pub(super) struct ConstructorSource {
    kind: Kind,
    numbers: Vec<FloatSource>,
    color: Option<ColorSource>,
}

impl ConstructorSource {
    pub(super) fn read(
        expression: &Expr,
        resolve: &impl Fn(&Expr) -> Option<String>,
        numbers: &FloatReader<'_>,
    ) -> Option<(AnimationEffect, Self)> {
        let expression = syntax::unparen(expression);
        let expression = match expression {
            Expr::Cast(cast) if cast.attrs.is_empty()
                && syntax::rc_interface_cast(&cast.ty, "ferroui_base::media::IEffect", resolve) => {
                syntax::unparen(&cast.expr)
            }
            _ => expression,
        };
        let Expr::Call(rc) = expression else {
            return None;
        };
        if !rc.attrs.is_empty()
            || rc.args.len() != 1
            || !matches!(
                resolve(&rc.func)?.as_str(),
                "std::rc::Rc::new" | "alloc::rc::Rc::new"
            )
        {
            return None;
        }
        let Expr::Call(call) = syntax::unparen(rc.args.first()?) else {
            return None;
        };
        if !call.attrs.is_empty() {
            return None;
        }
        let form = EffectForm::Object;
        let name = resolve(&call.func)?;
        let class = name
            .strip_prefix("ferroui_base::media::")?
            .strip_suffix("::new")?;
        if class == "ImmutableBlurEffect" && call.args.len() == 1 {
            let radius = numbers.read(call.args.first()?)?;
            let value = AnimationEffect::Blur {
                radius: radius.value,
                form,
            };
            return Some((
                value,
                Self {
                    kind: Kind::Blur,
                    numbers: vec![radius],
                    color: None,
                },
            ));
        }
        let kind = match class {
            "ImmutableDropShadowEffect" => Kind::Offset,
            "ImmutableDropShadowDirectionEffect" => Kind::Directional,
            _ => return None,
        };
        if call.args.len() != 5 {
            return None;
        }
        let numbers = [0, 1, 2, 4]
            .into_iter()
            .map(|index| numbers.read(&call.args[index]))
            .collect::<Option<Vec<_>>>()?;
        let (color, source) = ColorSource::read_with(&call.args[3], false, resolve)?;
        let argb = color.argb8()?;
        let a = numbers[0].value;
        let b = numbers[1].value;
        let blur_radius = numbers[2].value;
        let opacity = numbers[3].value;
        let value = match kind {
            Kind::Offset => AnimationEffect::DropShadow {
                offset_x: a,
                offset_y: b,
                blur_radius,
                argb,
                opacity,
                form,
            },
            Kind::Directional => AnimationEffect::DirectionalDropShadow {
                direction: a,
                shadow_depth: b,
                blur_radius,
                argb,
                opacity,
                form,
            },
            Kind::Blur => unreachable!(),
        };
        Some((
            value,
            Self {
                kind,
                numbers,
                color: Some(source),
            },
        ))
    }

    pub(super) fn plan(
        &self,
        old: &AnimationEffect,
        next: &AnimationEffect,
        patches: &mut Vec<TextPatch>,
    ) -> Result<(), String> {
        let (kind, parts, color) = fields(next)?;
        if self.kind != kind {
            return Err("Keep the Rust effect constructor's type when editing its key".into());
        }
        if let (Some(source), Some(color), Some(old)) = (&self.color, color, fields(old)?.2) {
            source.plan(
                &AnimationValue::from_argb8(old),
                &AnimationValue::from_argb8(color),
                patches,
            )?;
        }
        for (source, value) in self.numbers.iter().zip(parts) {
            source.plan(value, patches);
        }
        Ok(())
    }
}

pub(super) fn literal(effect: &AnimationEffect) -> Result<String, String> {
    if effect.form() == EffectForm::Literal {
        return Ok(format!(
            "::ferroui_base::media::Effect::parse({:?}).unwrap()",
            effect.to_xaml()
        ));
    }
    let (kind, args, color) = fields(effect)?;
    let mut fields = args
        .into_iter()
        .map(|n| format!("{n:?}"))
        .collect::<Vec<_>>();
    if let Some([a, r, g, b]) = color {
        fields.insert(
            3,
            format!("::ferroui_base::media::Color::from_argb({a}, {r}, {g}, {b})"),
        );
    }
    let name = match kind {
        Kind::Blur => "ImmutableBlurEffect",
        Kind::Offset => "ImmutableDropShadowEffect",
        Kind::Directional => "ImmutableDropShadowDirectionEffect",
    };
    Ok(format!("::std::rc::Rc::new(::ferroui_base::media::{name}::new({})) as ::std::rc::Rc<dyn ::ferroui_base::media::IEffect>", fields.join(", ")))
}

type ConstructorFields = (Kind, Vec<f64>, Option<[u8; 4]>);

fn fields(effect: &AnimationEffect) -> Result<ConstructorFields, String> {
    Ok(match *effect {
        AnimationEffect::Blur { radius, .. } => (Kind::Blur, vec![radius], None),
        AnimationEffect::DropShadow {
            offset_x,
            offset_y,
            blur_radius,
            opacity,
            argb,
            ..
        } => (
            Kind::Offset,
            vec![offset_x, offset_y, blur_radius, opacity],
            Some(argb),
        ),
        AnimationEffect::DirectionalDropShadow {
            direction,
            shadow_depth,
            blur_radius,
            opacity,
            argb,
            ..
        } => (
            Kind::Directional,
            vec![direction, shadow_depth, blur_radius, opacity],
            Some(argb),
        ),
        AnimationEffect::Registered { .. } => {
            return Err("Use Effect::parse with a registered shader literal".into());
        }
    })
}
