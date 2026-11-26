use super::{
    references::Tokens,
    scalars::{finite, literal_number},
    ValueResult,
};
use serde_json::Value;

pub(super) const PROPERTIES: [&str; 4] = [
    "ControlPointX1",
    "ControlPointY1",
    "ControlPointX2",
    "ControlPointY2",
];

pub(super) fn import<'a>(tokens: &Tokens<'a>, value: &'a Value) -> ValueResult<String> {
    let values = value
        .as_array()
        .ok_or("A cubic Bézier value needs four control-point coordinates")?;
    let points = values
        .iter()
        .map(|value| finite(tokens.typed_property(value, "number")?));
    Ok(format(validate(points)?))
}

pub(super) fn export(value: &str) -> ValueResult<([f64; 4], String)> {
    let mut coordinates = Vec::new();
    for segment in value.split(',') {
        if segment.trim().is_empty() {
            return Err("A KeySpline needs four literal control-point coordinates".into());
        }
        coordinates.extend(segment.split_whitespace().map(literal_number));
    }
    let points = validate(coordinates.into_iter())?;
    Ok((points, format(points)))
}

fn validate(points: impl Iterator<Item = ValueResult<f64>>) -> ValueResult<[f64; 4]> {
    let [x1, y1, x2, y2]: [f64; 4] = points
        .collect::<ValueResult<Vec<_>>>()?
        .try_into()
        .map_err(|_| "A cubic Bézier value needs four control-point coordinates")?;
    if !(0.0..=1.0).contains(&x1) || !(0.0..=1.0).contains(&x2) {
        return Err(
            "The X coordinates of a cubic Bézier curve must be between zero and one".into(),
        );
    }
    Ok([x1, y1, x2, y2])
}

fn format([x1, y1, x2, y2]: [f64; 4]) -> String {
    format!("{x1},{y1},{x2},{y2}")
}
