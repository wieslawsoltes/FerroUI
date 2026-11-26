use super::{
    references::Tokens,
    scalars::{dimension, literal_number},
    ValueResult,
};
use serde_json::{json, Value};

const CAPS: [(&str, &str); 3] = [("butt", "Flat"), ("round", "Round"), ("square", "Square")];

pub(super) enum Stroke {
    Solid,
    Dashed,
    Dotted,
    Pattern { pixels: Vec<f64>, cap: &'static str },
}

impl Stroke {
    pub(super) fn import<'a>(
        tokens: &Tokens<'a>,
        value: &'a Value,
        pixels_per_rem: Option<f64>,
        note: &mut Option<String>,
    ) -> ValueResult<Self> {
        if let Some(style) = value.as_str() {
            return match style {
                "solid" => Ok(Self::Solid),
                "dashed" => Ok(Self::Dashed),
                "dotted" => Ok(Self::Dotted),
                _ => Err(format!(
                    "FerroUI Shape has no single-stroke representation for {style}"
                )),
            };
        }
        let dashes = tokens.property(&value["dashArray"])?;
        let dashes = dashes
            .as_array()
            .ok_or("A stroke pattern needs a dashArray")?;
        let pixels = dashes
            .iter()
            .map(|value| {
                dimension(
                    tokens,
                    tokens.typed_property(value, "dimension")?,
                    pixels_per_rem,
                    note,
                )
            })
            .collect::<ValueResult<_>>()?;
        let cap = tokens.property(&value["lineCap"])?.as_str();
        let (cap, _) = CAPS
            .iter()
            .find(|(token, _)| Some(*token) == cap)
            .ok_or("Choose a butt, round or square lineCap")?;
        Self::pattern(pixels, cap)
    }

    pub(super) fn from_xaml(
        dashes: Option<&str>,
        cap: &str,
        width: Option<f64>,
    ) -> ValueResult<Self> {
        let (token_cap, _) = CAPS
            .iter()
            .find(|(_, xaml)| xaml.eq_ignore_ascii_case(cap))
            .ok_or("Export a Flat, Round or Square StrokeLineCap")?;
        let values = if let Some(dashes) = dashes {
            let mut values = Vec::new();
            for part in dashes.split(',') {
                if part.trim().is_empty() {
                    return Err("Define literal StrokeDashArray intervals".into());
                }
                for value in part.split_whitespace() {
                    values.push(literal_number(value)?);
                }
            }
            values
        } else {
            Vec::new()
        };
        match (token_cap, values.as_slice(), width) {
            (&"butt", [], _) => Ok(Self::Solid),
            (&"butt", [2.0, 2.0], None) => Ok(Self::Dashed),
            (&"round", [0.0, 2.0], None) => Ok(Self::Dotted),
            (_, [], _) => Self::pattern(values, token_cap),
            (_, _, Some(width)) if width > 0.0 => Self::pattern(
                values.into_iter().map(|relative| relative * width).collect(),
                token_cap,
            ),
            _ => Err("A custom dash pattern needs a positive literal StrokeThickness to recover logical pixels".into()),
        }
    }

    fn pattern(pixels: Vec<f64>, cap: &'static str) -> ValueResult<Self> {
        check_lengths(&pixels)?;
        Ok(Self::Pattern { pixels, cap })
    }

    pub(super) fn requires_width(&self) -> bool {
        matches!(self, Self::Pattern { pixels, .. } if !pixels.is_empty())
    }

    pub(super) fn properties(
        &self,
        width: Option<f64>,
    ) -> ValueResult<[(&'static str, String); 3]> {
        let (dashes, cap) = match self {
            Self::Solid => ("{x:Null}".into(), "Flat"),
            Self::Dashed => ("2,2".into(), "Flat"),
            Self::Dotted => ("0,2".into(), "Round"),
            Self::Pattern { pixels, cap } => {
                let cap = CAPS
                    .iter()
                    .find(|(token, _)| token == cap)
                    .expect("checked line cap")
                    .1;
                if pixels.is_empty() {
                    ("{x:Null}".into(), cap)
                } else {
                    let width = width
                        .filter(|width| *width > 0.0)
                        .ok_or("A custom dash pattern needs a positive stroke width")?;
                    let relative: Vec<_> = pixels.iter().map(|pixels| pixels / width).collect();
                    check_lengths(&relative)?;
                    (numbers(&relative), cap)
                }
            }
        };
        Ok([
            ("StrokeDashArray", dashes),
            ("StrokeDashOffset", "0".into()),
            ("StrokeLineCap", cap.into()),
        ])
    }

    pub(super) fn display(&self) -> String {
        match self {
            Self::Solid => "solid".into(),
            Self::Dashed => "dashed".into(),
            Self::Dotted => "dotted".into(),
            Self::Pattern { pixels, cap } if pixels.is_empty() => format!("solid · {cap} cap"),
            Self::Pattern { pixels, cap } => format!("{} px · {cap} cap", numbers(pixels)),
        }
    }

    pub(super) fn token_value(&self) -> Value {
        match self {
            Self::Solid => json!("solid"),
            Self::Dashed => json!("dashed"),
            Self::Dotted => json!("dotted"),
            Self::Pattern { pixels, cap } => {
                let dashes: Vec<_> = pixels
                    .iter()
                    .map(|value| json!({"value":value,"unit":"px"}))
                    .collect();
                json!({"dashArray":dashes,"lineCap":cap})
            }
        }
    }
}

fn numbers(values: &[f64]) -> String {
    values
        .iter()
        .map(f64::to_string)
        .collect::<Vec<_>>()
        .join(",")
}

fn check_lengths(values: &[f64]) -> ValueResult<()> {
    let length: f64 = values.iter().sum();
    if values
        .iter()
        .any(|value| !value.is_finite() || *value < 0.0)
        || (!values.is_empty() && (!length.is_finite() || length <= 0.0))
    {
        return Err("Dash intervals must be nonnegative and have a finite positive total".into());
    }
    Ok(())
}
