mod export;
pub(super) use export::export;

use super::{
    references::{Resolved, Tokens},
    scalars::{append_note, color, dimension, finite, hex_color},
    stroke::Stroke,
    theme, ValueResult,
};
use crate::components::FRAMEWORK;
use ferroui_designer_core::{Document, Node};
use serde_json::{json, Value};

pub(super) const METADATA: &str = "outline";
const SHAPES: &[&str] = &[
    "Shape",
    "Arc",
    "Ellipse",
    "Line",
    "Path",
    "Polygon",
    "Polyline",
    "Rectangle",
    "Sector",
];
const STYLE_DEFAULTS: [(&str, &str); 3] = [
    ("StrokeDashArray", "{x:Null}"),
    ("StrokeDashOffset", "0"),
    ("StrokeLineCap", "Flat"),
];
pub(super) struct Outline {
    target: &'static str,
    brush: Option<String>,
    width: Option<f64>,
    stroke: Stroke,
    omitted_defaults: Vec<&'static str>,
}

pub(super) fn target(document: &Document, node: &Node) -> Option<&'static str> {
    let (namespace, name) = theme::target(document, node)?;
    match namespace.as_str() {
        FRAMEWORK => target_name(&name),
        "using:FerroUI.Controls" | "clr-namespace:FerroUI.Controls;assembly=FerroUI.Controls"
            if name == "Border" =>
        {
            Some("Border")
        }
        "using:FerroUI.Controls.Shapes"
        | "clr-namespace:FerroUI.Controls.Shapes;assembly=FerroUI.Controls" => {
            SHAPES.iter().copied().find(|target| *target == name)
        }
        _ => None,
    }
}

fn target_name(name: &str) -> Option<&'static str> {
    if name == "Border" {
        Some("Border")
    } else {
        SHAPES.iter().copied().find(|target| *target == name)
    }
}

pub(super) fn import<'a>(
    tokens: &Tokens<'a>,
    resolved: &Resolved<'a>,
    kind: &str,
    pixels_per_rem: Option<f64>,
    note: &mut Option<String>,
) -> ValueResult<Outline> {
    let value = resolved.value;
    let (brush, width, stroke) = if kind == "border" {
        let brush = color(
            tokens,
            tokens.typed_property(&value["color"], "color")?,
            note,
        )?;
        let width = dimension(
            tokens,
            tokens.typed_property(&value["width"], "dimension")?,
            pixels_per_rem,
            note,
        )?;
        (
            Some(brush),
            Some(nonnegative(width)?),
            tokens.typed_property(&value["style"], "strokeStyle")?,
        )
    } else {
        let width = resolved
            .outline
            .and_then(|metadata| metadata.get("strokeWidth"))
            .map(|width| finite(width).and_then(nonnegative))
            .transpose()?;
        (None, width, value)
    };
    let stroke = Stroke::import(tokens, stroke, pixels_per_rem, note)?;
    let default = if brush.is_some() && matches!(stroke, Stroke::Solid) {
        "Border"
    } else {
        "Shape"
    };
    let target = match resolved
        .outline
        .and_then(|metadata| metadata.get("targetType"))
    {
        Some(target) => target
            .as_str()
            .and_then(target_name)
            .ok_or("Choose a FerroUI Border or Shape target")?,
        None => default,
    };
    if target == "Border" && (brush.is_none() || !matches!(stroke, Stroke::Solid)) {
        return Err(
            "Border supports a solid border token. Use a Shape for stroke styles and dash patterns"
                .into(),
        );
    }
    let width = if brush.is_none() && stroke.requires_width() && width.is_none() {
        append_note(note, "Uses a 1 px stroke to preserve absolute dash lengths");
        Some(1.0)
    } else {
        width
    };
    if target != "Border" {
        append_note(note, &format!("Apply this theme to {target}"));
    }
    let mut omitted_defaults = Vec::new();
    if let Some(omitted) = resolved
        .outline
        .and_then(|metadata| metadata.get("omitDefaultSetters"))
    {
        for property in omitted
            .as_array()
            .ok_or("Expected omitted outline setter names")?
        {
            let (name, _) = STYLE_DEFAULTS
                .iter()
                .find(|(name, _)| Some(*name) == property.as_str())
                .ok_or("Only the default stroke pattern, offset and cap can be omitted")?;
            omitted_defaults.push(*name);
        }
    }
    Ok(Outline {
        target,
        brush,
        width,
        stroke,
        omitted_defaults,
    })
}

fn nonnegative(width: f64) -> ValueResult<f64> {
    if width >= 0.0 {
        Ok(width)
    } else {
        Err("Stroke width must be nonnegative".into())
    }
}

impl Outline {
    pub(super) fn kind(&self) -> &'static str {
        if self.brush.is_some() {
            "border"
        } else {
            "strokeStyle"
        }
    }

    pub(super) fn summary(&self) -> String {
        let mut value = format!("{} · {}", self.target, self.stroke.display());
        if let Some(width) = self.width {
            value.push_str(&format!(" · {width} px wide"));
        }
        if let Some(brush) = &self.brush {
            value.push_str(&format!(" · {brush}"));
        }
        value
    }

    pub(super) fn xaml(&self, key: &str) -> ValueResult<String> {
        let mut setters = Vec::new();
        let border = self.target == "Border";
        if let Some(brush) = &self.brush {
            setters.push((if border { "BorderBrush" } else { "Stroke" }, brush.clone()));
        }
        if let Some(width) = self.width {
            setters.push((
                if border {
                    "BorderThickness"
                } else {
                    "StrokeThickness"
                },
                width.to_string(),
            ));
        }
        if !border {
            setters.extend(self.stroke.properties(self.width)?.into_iter().filter(
                |(name, value)| {
                    !self.omitted_defaults.contains(name)
                        || !STYLE_DEFAULTS.contains(&(*name, value.as_str()))
                },
            ));
        }
        Ok(theme::write(key, self.target, setters))
    }

    pub(super) fn token_value(&self) -> ValueResult<Value> {
        let style = self.stroke.token_value();
        if let (Some(brush), Some(width)) = (&self.brush, self.width) {
            Ok(
                json!({"color":hex_color(brush)?, "width":{"value":width,"unit":"px"}, "style":style}),
            )
        } else {
            Ok(style)
        }
    }

    pub(super) fn metadata(&self) -> Value {
        let mut metadata = json!({"targetType":self.target});
        if let Some(width) = self.width.filter(|_| self.brush.is_none()) {
            metadata["strokeWidth"] = json!(width);
        }
        if !self.omitted_defaults.is_empty() {
            metadata["omitDefaultSetters"] = json!(self.omitted_defaults);
        }
        metadata
    }
}
