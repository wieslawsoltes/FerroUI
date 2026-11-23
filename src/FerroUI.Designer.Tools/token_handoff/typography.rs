use super::{
    font,
    references::Tokens,
    scalars::{append_note, dimension, finite, literal_number},
    ValueResult,
};
use crate::components::{escape, framework, key_attribute, FRAMEWORK, XAML};
use ferroui_designer_core::{Document, MarkupValue, Node, NodeKind};
use serde_json::{json, Value};
use std::{collections::BTreeMap, fmt::Write};

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
    if !targets_text_block(document, theme)
        || theme.attributes.iter().any(|attribute| {
            !attribute.is_namespace()
                && Some(attribute) != key_attribute(theme)
                && attribute.name != "TargetType"
        })
    {
        return Err(
            "Export typography from a keyed TextBlock ControlTheme with no other theme attributes"
                .into(),
        );
    }

    let mut setters = BTreeMap::new();
    for child in theme
        .children
        .iter()
        .filter_map(|id| document.ast().node(*id))
    {
        if trivia(child) {
            continue;
        }
        if child.local_name() != "Setter"
            || !framework(child)
            || child.attributes.iter().any(|attribute| {
                !attribute.is_namespace()
                    && !matches!(attribute.name.as_str(), "Property" | "Value")
            })
            || child
                .children
                .iter()
                .filter_map(|id| document.ast().node(*id))
                .any(|child| !trivia(child))
        {
            return Err("Typography export requires direct literal Setter elements".into());
        }
        let property = child
            .attribute("Property")
            .ok_or("A Setter needs a Property")?;
        let value = child
            .attribute("Value")
            .ok_or("A Setter needs a literal Value")?
            .trim();
        if !SETTERS.contains(&property) || setters.insert(property, value).is_some() {
            return Err(format!(
                "Unexpected or repeated typography setter {property}"
            ));
        }
    }
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

fn targets_text_block(document: &Document, theme: &Node) -> bool {
    let Some(target) = theme.attribute("TargetType") else {
        return false;
    };
    let namespaces = document.ast().namespaces(theme.id);
    let target = match MarkupValue::parse(target.trim()) {
        Ok(MarkupValue::Literal(target)) => target,
        Ok(MarkupValue::Extension(extension)) => {
            let Some((prefix, "Type")) = extension.name.split_once(':') else {
                return false;
            };
            let [argument] = extension.arguments.as_slice() else {
                return false;
            };
            if namespaces.get(prefix).map(String::as_str) != Some(XAML) || argument.name.is_some() {
                return false;
            }
            let MarkupValue::Literal(target) = &argument.value else {
                return false;
            };
            target.clone()
        }
        Err(_) => return false,
    };
    let (prefix, name) = target.split_once(':').unwrap_or(("", &target));
    name == "TextBlock"
        && matches!(
            namespaces.get(prefix).map(String::as_str),
            Some(
                FRAMEWORK
                    | "using:FerroUI.Controls"
                    | "clr-namespace:FerroUI.Controls;assembly=FerroUI.Controls"
            )
        )
}

fn trivia(node: &Node) -> bool {
    node.kind == NodeKind::Comment
        || matches!(node.kind, NodeKind::Text | NodeKind::CData)
            && node
                .text
                .as_deref()
                .is_some_and(|text| text.trim().is_empty())
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
        let key = escape(key);
        let mut theme = format!("    <ControlTheme x:Key=\"{key}\" TargetType=\"TextBlock\">\n");
        let values = [
            escape(&self.family),
            self.size.to_string(),
            self.weight.to_string(),
            self.spacing.to_string(),
            self.line_height.to_string(),
        ];
        for (property, value) in SETTERS.into_iter().zip(values) {
            let _ = writeln!(
                theme,
                "        <Setter Property=\"{property}\" Value=\"{value}\"/>"
            );
        }
        theme.push_str("    </ControlTheme>\n");
        theme
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
