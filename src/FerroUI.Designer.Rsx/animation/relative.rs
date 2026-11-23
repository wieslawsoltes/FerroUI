use super::{number_source::FloatReader, variant::VariantSource, FloatSource};
use ferroui_designer_animation::{AnimationValue, RelativeUnit, ValueKind};
use ferroui_designer_core::TextPatch;
use syn::Expr;

const UNIT_TYPE: &str = "ferroui_base::RelativeUnit";

#[derive(Clone, Debug)]
pub(super) struct RelativeSource {
    first: FloatSource,
    second: Option<FloatSource>,
    unit: RelativeUnit,
    unit_variant: VariantSource,
}

impl RelativeSource {
    pub(super) fn read(
        expr: &Expr,
        kind: ValueKind,
        resolve: &impl Fn(&Expr) -> Option<String>,
        numbers: &FloatReader<'_>,
    ) -> Option<(AnimationValue, Self)> {
        let Expr::Call(call) = expr else { return None };
        if !call.attrs.is_empty() {
            return None;
        }
        let factory = resolve(&call.func)?;
        let args = call.args.iter().collect::<Vec<_>>();
        let (first, second, unit) = match (kind, factory.as_str(), args.as_slice()) {
            (ValueKind::RelativePoint, "ferroui_base::RelativePoint::new", [x, y, unit]) => {
                (numbers.read(x)?, Some(numbers.read(y)?), *unit)
            }
            (
                ValueKind::RelativePoint,
                "ferroui_base::RelativePoint::from_point",
                [Expr::Call(point), unit],
            ) if resolve(&point.func)?.as_str() == "ferroui_base::Point::new"
                && point.attrs.is_empty() && point.args.len() == 2 =>
            {
                (
                    numbers.read(&point.args[0])?,
                    Some(numbers.read(&point.args[1])?),
                    *unit,
                )
            }
            (ValueKind::RelativeScalar, "ferroui_base::RelativeScalar::new", [value, unit]) => {
                (numbers.read(value)?, None, *unit)
            }
            _ => return None,
        };
        let unit_path = unit;
        let unit = match resolve(unit_path)?.as_str() {
            "ferroui_base::RelativeUnit::Absolute" => RelativeUnit::Absolute,
            "ferroui_base::RelativeUnit::Relative" => RelativeUnit::Relative,
            _ => return None,
        };
        let value = match &second {
            Some(y) => AnimationValue::RelativePoint {
                x: first.value,
                y: y.value,
                unit,
            },
            None => AnimationValue::RelativeScalar {
                scalar: first.value,
                unit,
            },
        };
        value.validate().ok()?;
        let unit_variant = VariantSource::read(unit_path, UNIT_TYPE, unit_variant(unit), resolve)?;
        Some((
            value,
            Self {
                first,
                second,
                unit,
                unit_variant,
            },
        ))
    }

    pub(super) fn plan(
        &self,
        value: &AnimationValue,
        patches: &mut Vec<TextPatch>,
    ) -> Result<(), String> {
        let (first, second, unit) = match (&self.second, value) {
            (Some(y_source), AnimationValue::RelativePoint { x, y, unit }) => {
                (*x, Some((y_source, *y)), *unit)
            }
            (None, AnimationValue::RelativeScalar { scalar, unit }) => (*scalar, None, *unit),
            _ => return Err("Choose a value that matches the relative Rust property type".into()),
        };
        self.first.plan(first, patches);
        if let Some((source, value)) = second {
            source.plan(value, patches);
        }
        if self.unit != unit {
            self.unit_variant.plan(UNIT_TYPE, unit_variant(unit), patches);
        }
        Ok(())
    }
}

pub(super) fn literal(value: &AnimationValue) -> Option<String> {
    let qualified_unit = |unit| format!("::ferroui_base::RelativeUnit::{}", unit_variant(unit));
    match value {
        AnimationValue::RelativePoint { x, y, unit } => Some(format!(
            "::ferroui_base::RelativePoint::new({x:?}, {y:?}, {})",
            qualified_unit(*unit)
        )),
        AnimationValue::RelativeScalar { scalar, unit } => Some(format!(
            "::ferroui_base::RelativeScalar::new({scalar:?}, {})",
            qualified_unit(*unit)
        )),
        _ => None,
    }
}

fn unit_variant(unit: RelativeUnit) -> &'static str {
    match unit {
        RelativeUnit::Absolute => "Absolute",
        RelativeUnit::Relative => "Relative",
    }
}
