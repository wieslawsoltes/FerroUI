use super::{syntax, FloatSource};
use ferroui_designer_animation::{AnimationValue, RelativeUnit, ValueKind};
use ferroui_designer_core::{SourceSpan, TextPatch};
use syn::Expr;

#[derive(Clone, Debug)]
pub(super) struct RelativeSource {
    first: FloatSource,
    second: Option<FloatSource>,
    unit: RelativeUnit,
    unit_variant: SourceSpan,
}

impl RelativeSource {
    pub(super) fn read(expr: &Expr, kind: ValueKind) -> Option<(AnimationValue, Self)> {
        let Expr::Call(call) = expr else { return None };
        let factory = syntax::path(&call.func)?;
        let args = call.args.iter().collect::<Vec<_>>();
        let (first, second, unit) = match (kind, factory.as_str(), args.as_slice()) {
            (ValueKind::RelativePoint, "ferroui_base::RelativePoint::new", [x, y, unit]) => {
                (syntax::float(x)?, Some(syntax::float(y)?), *unit)
            }
            (
                ValueKind::RelativePoint,
                "ferroui_base::RelativePoint::from_point",
                [Expr::Call(point), unit],
            ) if syntax::path(&point.func)?.as_str() == "ferroui_base::Point::new"
                && point.args.len() == 2 =>
            {
                (
                    syntax::float(&point.args[0])?,
                    Some(syntax::float(&point.args[1])?),
                    *unit,
                )
            }
            (ValueKind::RelativeScalar, "ferroui_base::RelativeScalar::new", [value, unit]) => {
                (syntax::float(value)?, None, *unit)
            }
            _ => return None,
        };
        let Expr::Path(unit_path) = unit else {
            return None;
        };
        let unit = match syntax::path(unit)?.as_str() {
            "ferroui_base::RelativeUnit::Absolute" => RelativeUnit::Absolute,
            "ferroui_base::RelativeUnit::Relative" => RelativeUnit::Relative,
            _ => return None,
        };
        let value = match second {
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
        let unit_variant = syntax::span(&unit_path.path.segments.last()?.ident);
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
        let (first, second, unit) = match (self.second, value) {
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
            patches.push(TextPatch {
                span: self.unit_variant,
                replacement: unit_variant(unit).into(),
            });
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
