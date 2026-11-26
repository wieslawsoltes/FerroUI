use super::{
    font,
    references::Tokens,
    scalars::{append_note, dimension, finite, literal_number},
    theme, ValueResult,
};
use crate::components::FRAMEWORK;
use ferroui_designer_core::{Document, Node};
use serde_json::{json, Value};

const SETTERS: [&str; 5] = [
    "FontFamily",
    "FontSize",
    "FontWeight",
    "LetterSpacing",
    "LineHeight",
];

pub(super) struct Typography {
    family: String,
    size: f64,
    weight: u16,
    spacing: f64,
    line_height: f64,
}

pub(super) fn import<'a>(
    tokens: &Tokens<'a>,
    value: &'a Value,
    pixels_per_rem: Option<f64>,
    note: &mut Option<String>,
) -> ValueResult<Typography> {
    if !value.is_object() {
        return Err("A typography token needs an object with five typographic values".into());
    }
    let field = |name: &str, kind| {
        let field = value.get(name).ok_or_else(|| format!("Missing {name}"))?;
        tokens.typed_property(field, kind)
    };
    let family = font::import_family(field("fontFamily", "fontFamily")?)?;
    let weight = font::import_weight(field("fontWeight", "fontWeight")?)?;
    let mut length = |name| dimension(tokens, field(name, "dimension")?, pixels_per_rem, note);
    let size = positive(length("fontSize")?, "Font size")?;
    let spacing = length("letterSpacing")?;
    let multiplier = positive(
        finite(field("lineHeight", "number")?)?,
        "Line-height multiplier",
    )?;
    let line_height = positive(size * multiplier, "Line height")?;
    append_note(
        note,
        "Converted the line-height multiplier to XAML logical pixels",
    );
    Ok(Typography {
        family,
        size,
        weight,
        spacing,
        line_height,
    })
}

pub(super) fn export(document: &Document, theme: &Node) -> ValueResult<Typography> {
    if !theme::target(document, theme).is_some_and(|(namespace, name)| {
        name == "TextBlock"
            && matches!(
                namespace.as_str(),
                FRAMEWORK
                    | "using:FerroUI.Controls"
                    | "clr-namespace:FerroUI.Controls;assembly=FerroUI.Controls"
            )
    }) {
        return Err("Export typography from a keyed TextBlock ControlTheme".into());
    }
    let setters = theme::literal_setters(document, theme, &SETTERS)?;
    let [Some(family), Some(size), Some(weight), Some(spacing), Some(line_height)] =
        SETTERS.map(|name| setters.get(name).copied())
    else {
        return Err("Define FontFamily, FontSize, FontWeight, LetterSpacing and LineHeight to export typography".into());
    };
    Ok(Typography {
        family: family.into(),
        size: positive(literal_number(size)?, "Font size")?,
        weight: font::export_weight(weight)?,
        spacing: literal_number(spacing)?,
        line_height: positive(literal_number(line_height)?, "Line height")?,
    })
}

fn positive(value: f64, name: &str) -> ValueResult<f64> {
    if value.is_finite() && value > 0.0 {
        return Ok(value);
    }
    Err(format!("{name} must be positive and finite"))
}

impl Typography {
    pub(super) fn summary(&self) -> String {
        format!(
            "{} · {} px · weight {} · {} px line height",
            self.family, self.size, self.weight, self.line_height
        )
    }

    pub(super) fn xaml(&self, key: &str) -> String {
        let values = [
            self.family.clone(),
            self.size.to_string(),
            self.weight.to_string(),
            self.spacing.to_string(),
            self.line_height.to_string(),
        ];
        theme::write(key, "TextBlock", SETTERS.into_iter().zip(values))
    }

    pub(super) fn token_value(&self) -> ValueResult<Value> {
        Ok(json!({
            "fontFamily": font::export_family(&self.family)?,
            "fontSize": {"value": self.size, "unit": "px"},
            "fontWeight": self.weight,
            "letterSpacing": {"value": self.spacing, "unit": "px"},
            "lineHeight": positive(self.line_height / self.size, "Line-height multiplier")?,
        }))
    }
}
