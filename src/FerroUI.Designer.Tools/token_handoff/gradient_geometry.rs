use super::{scalars::literal_number, ValueResult};
use std::collections::BTreeMap;

pub(super) const TYPES: [&str; 3] = [
    "LinearGradientBrush",
    "RadialGradientBrush",
    "ConicGradientBrush",
];

pub(super) fn brush_type(name: &str) -> ValueResult<&'static str> {
    TYPES.into_iter().find(|kind| *kind == name).ok_or_else(|| {
        "Map a gradient to LinearGradientBrush, RadialGradientBrush, or ConicGradientBrush".into()
    })
}

pub(super) fn properties<'a>(
    kind: &str,
    values: impl Iterator<Item = (&'a str, &'a str)>,
) -> ValueResult<BTreeMap<String, String>> {
    values
        .map(|(property, value)| {
            let parsed = validate_property(kind, property, value)
                .map_err(|message| format!("{property} — {message}"))?;
            Ok((property.into(), parsed))
        })
        .collect()
}

fn validate_property(kind: &str, property: &str, value: &str) -> ValueResult<String> {
    match (kind, property) {
        (_, "SpreadMethod") => ["Pad", "Reflect", "Repeat"]
            .into_iter()
            .find(|method| method.eq_ignore_ascii_case(value.trim()))
            .map(str::to_owned)
            .ok_or_else(|| "Choose Pad, Reflect, or Repeat".into()),
        (_, "Opacity") => {
            let opacity = literal_number(value)?;
            if !(0.0..=1.0).contains(&opacity) {
                return Err("Opacity must be between zero and one".into());
            }
            Ok(opacity.to_string())
        }
        ("LinearGradientBrush", "StartPoint" | "EndPoint")
        | ("RadialGradientBrush", "Center" | "GradientOrigin")
        | ("ConicGradientBrush", "Center") => relative_point(value),
        ("RadialGradientBrush", "RadiusX" | "RadiusY") => relative_scalar(value),
        ("ConicGradientBrush", "Angle") => Ok(literal_number(value)?.to_string()),
        _ => Err(format!(
            "This authored property has no gradient mapping for {kind}"
        )),
    }
}

fn relative_point(value: &str) -> ValueResult<String> {
    let points = if value.contains(',') {
        value.split(',').map(str::trim).collect::<Vec<_>>()
    } else {
        value.split_whitespace().collect()
    };
    let [x, y] = points.as_slice() else {
        return Err("A point needs two literal coordinates".into());
    };
    if x.ends_with('%') != y.ends_with('%') {
        return Err("Use the same unit in both coordinates".into());
    }
    Ok(format!("{},{}", relative_scalar(x)?, relative_scalar(y)?))
}

fn relative_scalar(value: &str) -> ValueResult<String> {
    let value = value.trim();
    match value.strip_suffix('%') {
        Some(relative) => Ok(format!("{}%", literal_number(relative)?)),
        None => Ok(literal_number(value)?.to_string()),
    }
}
