use super::super::{
    scalars::{hex_color, literal_number},
    theme, ValueResult,
};
use super::{nonnegative, Outline, Stroke, STYLE_DEFAULTS};
use crate::components::XAML;
use ferroui_designer_core::{Document, MarkupValue, Node};

const SHAPE_SETTERS: &[&str] = &[
    "Stroke",
    "StrokeThickness",
    "StrokeDashArray",
    "StrokeDashOffset",
    "StrokeLineCap",
];
const BORDER_SETTERS: &[&str] = &["BorderBrush", "BorderThickness"];

pub(in crate::token_handoff) fn export(
    document: &Document,
    node: &Node,
    target: &'static str,
) -> ValueResult<Outline> {
    if target == "Border" {
        let setters = theme::literal_setters(document, node, BORDER_SETTERS)?;
        let brush = *setters
            .get("BorderBrush")
            .ok_or("Define a literal BorderBrush")?;
        let thickness = setters
            .get("BorderThickness")
            .ok_or("Define a uniform BorderThickness")?;
        let widths = thickness
            .split(',')
            .map(literal_number)
            .collect::<ValueResult<Vec<_>>>()?;
        let [width, rest @ ..] = widths.as_slice() else {
            return Err("Define a uniform BorderThickness".into());
        };
        if !matches!(widths.len(), 1 | 2 | 4) || rest.iter().any(|other| other != width) {
            return Err("A DTCG border needs uniform BorderThickness".into());
        }
        hex_color(brush)?;
        return Ok(Outline {
            target,
            brush: Some(brush.into()),
            width: Some(nonnegative(*width)?),
            stroke: Stroke::Solid,
            omitted_defaults: Vec::new(),
        });
    }
    let setters = theme::literal_setters(document, node, SHAPE_SETTERS)?;
    let brush = setters.get("Stroke").copied();
    let width = setters
        .get("StrokeThickness")
        .map(|width| literal_number(width).and_then(nonnegative))
        .transpose()?;
    if let Some(brush) = brush {
        hex_color(brush)?;
        if width.is_none() {
            return Err(
                "Define a literal StrokeThickness to export the border color and width".into(),
            );
        }
    } else if !setters.contains_key("StrokeDashArray") && !setters.contains_key("StrokeLineCap") {
        return Err(
            "Define a literal StrokeDashArray or StrokeLineCap to export a stroke style".into(),
        );
    }
    if setters
        .get("StrokeDashOffset")
        .is_some_and(|offset| literal_number(offset).ok() != Some(0.0))
    {
        return Err("DTCG stroke styles require a zero dash offset".into());
    }
    let dashes = setters
        .get("StrokeDashArray")
        .copied()
        .filter(|value| !null(document, node, value));
    let stroke = Stroke::from_xaml(
        dashes,
        setters.get("StrokeLineCap").copied().unwrap_or("Flat"),
        width,
    )?;
    let omitted_defaults = STYLE_DEFAULTS
        .into_iter()
        .filter_map(|(property, _)| (!setters.contains_key(property)).then_some(property))
        .collect();
    Ok(Outline {
        target,
        brush: brush.map(str::to_owned),
        width,
        stroke,
        omitted_defaults,
    })
}

fn null(document: &Document, node: &Node, value: &str) -> bool {
    let Ok(MarkupValue::Extension(extension)) = MarkupValue::parse(value) else {
        return false;
    };
    let Some((prefix, "Null")) = extension.name.split_once(':') else {
        return false;
    };
    extension.arguments.is_empty()
        && document
            .ast()
            .namespaces(node.id)
            .get(prefix)
            .map(String::as_str)
            == Some(XAML)
}
