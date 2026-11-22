use super::{
    gradient_geometry::{brush_type, properties},
    native_type,
    references::{Resolved, Tokens},
    scalars::{append_note, color, finite, hex_color, literal_number},
    ValueResult,
};
use crate::components::{escape, key_attribute};
use ferroui_designer_core::{Document, Node, NodeKind};
use serde_json::{json, Value};
use std::{collections::BTreeMap, fmt::Write};

const MEDIA: &str = "FerroUI.Media";

pub(super) struct ImportedGradient {
    pub kind: &'static str,
    properties: BTreeMap<String, String>,
    stops: Vec<(f64, String)>,
}

impl ImportedGradient {
    pub fn summary(&self) -> String {
        summary(self.stops.len())
    }

    pub fn xaml(&self, key: &str) -> String {
        let kind = self.kind;
        let mut source = format!("    <{kind} x:Key=\"{}\"", escape(key));
        for (name, value) in &self.properties {
            let _ = write!(source, " {name}=\"{}\"", escape(value));
        }
        source.push_str(">\n");
        for (offset, color) in &self.stops {
            let _ = writeln!(
                source,
                "        <GradientStop Offset=\"{offset}\" Color=\"{color}\"/>"
            );
        }
        let _ = writeln!(source, "    </{kind}>");
        source
    }
}

pub(super) fn import<'a>(
    tokens: &Tokens<'a>,
    value: &Resolved<'a>,
    preferred_kind: Option<&str>,
    note: &mut Option<String>,
) -> ValueResult<ImportedGradient> {
    let kind = brush_type(
        preferred_kind
            .or(value.xaml_type)
            .unwrap_or("LinearGradientBrush"),
    )?;
    let properties = if let Some(metadata) = value.gradient_properties {
        let original = brush_type(value.xaml_type.unwrap_or(kind))?;
        let attributes = metadata
            .as_object()
            .ok_or("Gradient geometry metadata must contain literal property values")?;
        let attributes = attributes
            .iter()
            .map(|(name, value)| {
                let value = value
                    .as_str()
                    .ok_or("Gradient geometry metadata must contain literal property values")?;
                Ok::<_, String>((name.as_str(), value))
            })
            .collect::<ValueResult<Vec<_>>>()?;
        let mut values = properties(original, attributes.into_iter())?;
        if original == kind {
            append_note(note, "Retained FerroUI gradient properties");
        } else {
            values.retain(|name, _| matches!(name.as_str(), "Opacity" | "SpreadMethod"));
            values.extend(default_geometry(kind, note));
            append_note(
                note,
                &format!("Used {kind} geometry in place of {original} geometry"),
            );
        }
        values
    } else {
        default_geometry(kind, note)
    };
    let values = value
        .value
        .as_array()
        .filter(|stops| !stops.is_empty())
        .ok_or("A gradient needs a list of at least one stop")?;
    let mut stops = Vec::with_capacity(values.len());
    let mut clamped = false;
    for (index, entry) in values.iter().enumerate() {
        let stop = import_stop(tokens, entry, note)
            .map_err(|message| format!("Gradient stop {} — {message}", index + 1))?;
        clamped |= !(0.0..=1.0).contains(&stop.0);
        stops.push((stop.0.clamp(0.0, 1.0), stop.1));
    }
    if clamped {
        append_note(
            note,
            "Clamped stop positions to the DTCG range of zero to one",
        );
    }
    Ok(ImportedGradient {
        kind,
        properties,
        stops,
    })
}

fn default_geometry(kind: &str, note: &mut Option<String>) -> BTreeMap<String, String> {
    if kind == "LinearGradientBrush" {
        append_note(note, "Mapped to a horizontal linear gradient");
        BTreeMap::from([
            ("StartPoint".into(), "0%,0%".into()),
            ("EndPoint".into(), "100%,0%".into()),
        ])
    } else {
        append_note(note, "Using the selected brush's default geometry");
        BTreeMap::new()
    }
}

fn import_stop<'a>(
    tokens: &Tokens<'a>,
    value: &'a Value,
    note: &mut Option<String>,
) -> ValueResult<(f64, String)> {
    let mut stop = tokens.typed_property(value, "gradient")?;
    let mut followed = Vec::new();
    // A gradient token can name a single stop by wrapping it in a singleton list.
    while let Some(list) = stop.as_array() {
        let [entry] = list.as_slice() else {
            return Err("A gradient stop reference must resolve to a single gradient stop".into());
        };
        let identity = stop as *const Value;
        if followed.contains(&identity) || followed.len() >= 128 {
            return Err("Circular or overlong gradient stop reference".into());
        }
        followed.push(identity);
        stop = tokens.typed_property(entry, "gradient")?;
    }
    if !stop.is_object() {
        return Err("Expected a single gradient stop object".into());
    }
    let position = finite(tokens.typed_property(&stop["position"], "number")?)?;
    let tint = color(
        tokens,
        tokens.typed_property(&stop["color"], "color")?,
        note,
    )?;
    Ok((position, tint))
}

pub(super) struct ExportedGradient {
    pub kind: &'static str,
    pub properties: BTreeMap<String, String>,
    pub stops: Vec<Value>,
}

impl ExportedGradient {
    pub fn summary(&self) -> String {
        summary(self.stops.len())
    }
}

pub(super) fn export(document: &Document, node: &Node) -> ValueResult<ExportedGradient> {
    let kind = brush_type(node.local_name())?;
    let properties = properties(
        kind,
        node.attributes
            .iter()
            .filter(|attribute| {
                !attribute.is_namespace() && Some(*attribute) != key_attribute(node)
            })
            .map(|attribute| (attribute.name.as_str(), attribute.value.as_str())),
    )?;
    let children = literal_children(document, node)?;
    let children = match children.as_slice() {
        [child]
            if child.kind == NodeKind::PropertyElement
                && native_type(child, MEDIA)
                && (child.local_name() == "GradientBrush.GradientStops"
                    || child.local_name() == format!("{kind}.GradientStops"))
                && child
                    .attributes
                    .iter()
                    .all(|attribute| attribute.is_namespace()) =>
        {
            literal_children(document, child)?
        }
        _ => children,
    };
    if children.is_empty() {
        return Err("A gradient needs at least one literal stop".into());
    }
    let stops = children
        .iter()
        .enumerate()
        .map(|(index, child)| {
            export_stop(document, child)
                .map_err(|message| format!("Gradient stop {} — {message}", index + 1))
        })
        .collect::<ValueResult<Vec<_>>>()?;
    Ok(ExportedGradient {
        kind,
        properties,
        stops,
    })
}

fn export_stop(document: &Document, node: &Node) -> ValueResult<Value> {
    if node.kind != NodeKind::Element
        || !native_type(node, MEDIA)
        || node.local_name() != "GradientStop"
        || node.attributes.iter().any(|attribute| {
            !attribute.is_namespace() && !matches!(attribute.name.as_str(), "Color" | "Offset")
        })
        || !literal_children(document, node)?.is_empty()
    {
        return Err("Use a GradientStop with literal Color and Offset attributes".into());
    }
    let tint = hex_color(
        node.attribute("Color")
            .ok_or("The stop needs a literal Color")?,
    )?;
    let position = literal_number(node.attribute("Offset").unwrap_or("0"))?;
    if !(0.0..=1.0).contains(&position) {
        return Err("This XAML offset is outside the DTCG range of zero to one".into());
    }
    Ok(json!({"color":tint,"position":position}))
}

fn literal_children<'a>(document: &'a Document, node: &Node) -> ValueResult<Vec<&'a Node>> {
    let mut children = Vec::new();
    for child in node
        .children
        .iter()
        .filter_map(|id| document.ast().node(*id))
    {
        match child.kind {
            NodeKind::Element | NodeKind::PropertyElement => children.push(child),
            NodeKind::Comment => {}
            NodeKind::Text | NodeKind::CData
                if child.text.as_deref().unwrap_or_default().trim().is_empty() => {}
            _ => return Err("Use literal gradient elements without additional content".into()),
        }
    }
    Ok(children)
}

fn summary(count: usize) -> String {
    format!(
        "{count} gradient {}",
        if count == 1 { "stop" } else { "stops" }
    )
}
