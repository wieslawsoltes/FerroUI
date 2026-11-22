use super::{
    super::{FloatSource, syntax},
    operand::{self, GeometrySource, Leaf},
};
use ferroui_designer_animation::{
    AnimationGradient, AnimationValue, GradientDistance, GradientPoint, GradientShape,
    GradientSpread,
};
use ferroui_designer_core::{SourceSpan, TextPatch};
use syn::Expr;

#[derive(Clone, Debug)]
pub(super) struct Arguments {
    opacity: FloatSource,
    spread: SourceSpan,
    origin: GeometrySource,
    transform: Leaf,
    relative_transform: Leaf,
    geometry: Vec<GeometrySource>,
}

impl Arguments {
    pub(super) fn read(
        mut value: AnimationGradient,
        operands: &[&Expr],
        resolve: &impl Fn(&Expr) -> Option<String>,
    ) -> Option<(AnimationGradient, Self)> {
        let [
            opacity,
            transform,
            origin,
            spread,
            geometry @ ..,
            relative_transform,
        ] = operands
        else {
            return None;
        };
        let defaults = value.shape.fields();
        if defaults.len() != geometry.len() {
            return None;
        }
        let opacity = syntax::float(opacity)?;
        value.opacity = opacity.value;
        value.spread = GradientSpread::parse(
            resolve(spread)?.strip_prefix("ferroui_base::media::GradientSpreadMethod::")?,
        )?;
        let Expr::Path(spread) = spread else {
            return None;
        };
        let spread = syntax::span(&spread.path.segments.last()?.ident);
        let (transform_origin, origin) =
            GeometrySource::read(origin, &value.transform_origin.value(), resolve)?;
        value.transform_origin = point(&transform_origin)?;
        value.transform = operand::transform(transform, resolve)?;
        value.relative_transform = operand::transform(relative_transform, resolve)?;
        let (values, geometry): (Vec<_>, Vec<_>) = defaults
            .into_iter()
            .zip(geometry)
            .map(|((_, default), expr)| GeometrySource::read(expr, &default, resolve))
            .collect::<Option<Vec<_>>>()?
            .into_iter()
            .unzip();
        value.shape = shape(value.shape, &values)?;
        let source = Self {
            opacity,
            spread,
            origin,
            transform: Leaf::read(transform)?,
            relative_transform: Leaf::read(relative_transform)?,
            geometry,
        };
        Some((value, source))
    }

    pub(super) fn plan(
        &self,
        original: &AnimationGradient,
        next: &AnimationGradient,
        patches: &mut Vec<TextPatch>,
    ) -> Result<(), String> {
        self.opacity.plan(next.opacity, patches);
        if original.spread != next.spread {
            patches.push(TextPatch {
                span: self.spread,
                replacement: next.spread.name().into(),
            });
        }
        self.origin.plan(
            &original.transform_origin.value(),
            &next.transform_origin.value(),
            patches,
        )?;
        if original.transform != next.transform {
            self.transform
                .plan(operand::transform_literal(&next.transform), patches);
        }
        if original.relative_transform != next.relative_transform {
            self.relative_transform.plan(
                operand::transform_literal(&next.relative_transform),
                patches,
            );
        }
        for ((source, (_, old)), (_, next)) in self
            .geometry
            .iter()
            .zip(original.shape.fields())
            .zip(next.shape.fields())
        {
            source.plan(&old, &next, patches)?;
        }
        Ok(())
    }
}

pub(super) fn literal(gradient: &AnimationGradient) -> Result<String, String> {
    let mut values = vec![
        format!("{:?}", gradient.opacity),
        operand::transform_literal(&gradient.transform),
        operand::geometry_literal(&gradient.transform_origin.value())?,
        format!(
            "::ferroui_base::media::GradientSpreadMethod::{}",
            gradient.spread.name()
        ),
    ];
    for (_, value) in gradient.shape.fields() {
        values.push(operand::geometry_literal(&value)?);
    }
    values.push(operand::transform_literal(&gradient.relative_transform));
    Ok(values.join(", "))
}

fn shape(default: GradientShape, values: &[AnimationValue]) -> Option<GradientShape> {
    Some(match (default, values) {
        (GradientShape::Linear { .. }, [start, end]) => GradientShape::Linear {
            start: point(start)?,
            end: point(end)?,
        },
        (GradientShape::Radial { .. }, [center, origin, radius_x, radius_y]) => {
            GradientShape::Radial {
                center: point(center)?,
                origin: point(origin)?,
                radius_x: distance(radius_x)?,
                radius_y: distance(radius_y)?,
            }
        }
        (GradientShape::Conic { .. }, [center, AnimationValue::Number(angle)]) => {
            GradientShape::Conic {
                center: point(center)?,
                angle: *angle,
            }
        }
        _ => return None,
    })
}

fn point(value: &AnimationValue) -> Option<GradientPoint> {
    let AnimationValue::RelativePoint { x, y, unit } = *value else {
        return None;
    };
    Some(GradientPoint { x, y, unit })
}

fn distance(value: &AnimationValue) -> Option<GradientDistance> {
    let AnimationValue::RelativeScalar { scalar, unit } = *value else {
        return None;
    };
    Some(GradientDistance { scalar, unit })
}
