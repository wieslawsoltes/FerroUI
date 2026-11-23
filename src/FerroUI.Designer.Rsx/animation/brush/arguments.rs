use super::{
    super::{constant_source::ConstantReader, variant::VariantSource, FloatSource},
    operand::{self, GeometrySource, Leaf},
};
use ferroui_designer_animation::{
    AnimationGradient, AnimationValue, GradientDistance, GradientPoint, GradientShape,
    GradientSpread, RelativeUnit,
};
use ferroui_designer_core::{SourceSpan, TextPatch};
use syn::Expr;

const SPREAD_TYPE: &str = "ferroui_base::media::GradientSpreadMethod";

#[derive(Clone, Debug)]
pub(super) struct Arguments {
    opacity: FloatSource,
    spread: VariantSource,
    origin: GeometrySource,
    transform: Leaf,
    geometry: Vec<GeometrySource>,
    trailing: TrailingArguments,
}

#[derive(Clone, Debug)]
enum TrailingArguments {
    RelativeTransform(Leaf),
    SharedRadius {
        constructor: SourceSpan,
        radius: FloatSource,
        expression: Leaf,
    },
}

impl Arguments {
    pub(super) fn read(
        mut value: AnimationGradient,
        operands: &[&Expr],
        shared_radius: Option<SourceSpan>,
        resolve: &impl Fn(&Expr) -> Option<String>,
        constants: &ConstantReader<'_>,
    ) -> Option<(AnimationGradient, Self)> {
        let [opacity, transform, origin, spread, geometry @ .., last] = operands else {
            return None;
        };
        let mut defaults = value.shape.fields();
        if shared_radius.is_some() {
            if !matches!(value.shape, GradientShape::Radial { .. }) {
                return None;
            }
            defaults.truncate(2);
        }
        if geometry.len() != defaults.len() {
            return None;
        }
        let opacity = constants.float(opacity)?;
        value.opacity = opacity.value;
        value.spread = GradientSpread::parse(
            resolve(spread)?.strip_prefix("ferroui_base::media::GradientSpreadMethod::")?,
        )?;
        let spread = VariantSource::read(spread, SPREAD_TYPE, value.spread.name(), resolve)?;
        let (transform_origin, origin) =
            GeometrySource::read(origin, &value.transform_origin.value(), resolve, constants)?;
        value.transform_origin = point(&transform_origin)?;
        value.transform = operand::transform(transform, resolve, constants)?;
        let (mut values, geometry): (Vec<_>, Vec<_>) = defaults
            .into_iter()
            .zip(geometry)
            .map(|((_, default), expr)| GeometrySource::read(expr, &default, resolve, constants))
            .collect::<Option<Vec<_>>>()?
            .into_iter()
            .unzip();
        let trailing = if let Some(constructor) = shared_radius {
            let radius = constants.float(last)?;
            let value = GradientDistance {
                scalar: radius.value,
                unit: RelativeUnit::Relative,
            }
            .value();
            values.extend([value.clone(), value]);
            TrailingArguments::SharedRadius {
                constructor,
                radius,
                expression: Leaf::read(last)?,
            }
        } else {
            value.relative_transform = operand::transform(last, resolve, constants)?;
            TrailingArguments::RelativeTransform(Leaf::read(last)?)
        };
        value.shape = shape(value.shape, &values)?;
        let source = Self {
            opacity,
            spread,
            origin,
            transform: Leaf::read(transform)?,
            geometry,
            trailing,
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
            self.spread.plan(SPREAD_TYPE, next.spread.name(), patches);
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
        for ((source, (_, old)), (_, next)) in self
            .geometry
            .iter()
            .zip(original.shape.fields())
            .zip(next.shape.fields())
        {
            source.plan(&old, &next, patches)?;
        }
        match &self.trailing {
            TrailingArguments::RelativeTransform(source) => {
                if original.relative_transform != next.relative_transform {
                    source.plan(
                        operand::transform_literal(&next.relative_transform),
                        patches,
                    );
                }
            }
            TrailingArguments::SharedRadius {
                constructor,
                radius,
                expression,
            } => {
                let GradientShape::Radial {
                    radius_x, radius_y, ..
                } = next.shape
                else {
                    return Err(
                        "Keep the Rust gradient constructor's shape when editing its key".into(),
                    );
                };
                if radius_x == radius_y
                    && radius_x.unit == RelativeUnit::Relative
                    && next.relative_transform.is_none()
                {
                    radius.plan(radius_x.scalar, patches);
                } else {
                    patches.push(TextPatch {
                        span: *constructor,
                        replacement: "new".into(),
                    });
                    let arguments = [
                        operand::geometry_literal(&radius_x.value())?,
                        operand::geometry_literal(&radius_y.value())?,
                        operand::transform_literal(&next.relative_transform),
                    ];
                    expression.plan(arguments.join(", "), patches);
                }
            }
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
