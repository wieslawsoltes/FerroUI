//! Typed brush operations for hosts of Studio's source-preserving object draft.
use super::object_draft::{object_attributes, OBJECT_PRESETS};
use crate::PropertyObjectDraft;
use ferroui_base::{media::Color, RelativePoint, RelativeScalar};
use ferroui_designer_core::{Document, MarkupValue, Node, NodeId, NodeKind};
use ferroui_designer_tools::{component_property_attribute, component_property_object};
use serde::{Deserialize, Serialize};

const NS: &str = "https://github.com/ferroui";
const BRUSHES: [&str; 4] = [
    "SolidColorBrush",
    "LinearGradientBrush",
    "RadialGradientBrush",
    "ConicGradientBrush",
];

#[derive(Serialize)]
pub(crate) struct BrushDraft {
    source: String,
    brush: BrushObject,
    stops: Vec<BrushObject>,
    presets: [&'static str; 4],
}

#[derive(Serialize)]
struct BrushObject {
    node: NodeId,
    properties: Vec<BrushProperty>,
    kind: String,
}

#[derive(Serialize)]
struct BrushProperty {
    name: &'static str,
    kind: &'static str,
    value: Option<String>,
}

#[derive(Deserialize)]
#[serde(tag = "edit", rename_all = "snake_case", deny_unknown_fields)]
pub(crate) enum BrushChange {
    Preset {
        kind: String,
    },
    Set {
        node: NodeId,
        property: String,
        value: String,
    },
    InsertStop,
    RemoveStop {
        node: NodeId,
    },
    MoveStop {
        node: NodeId,
        direction: i8,
    },
}

impl BrushDraft {
    pub(crate) fn open(document: &Document, node: NodeId, property: &str) -> Result<Self, String> {
        if let Some(source) = component_property_object(document, node, property)
            .map_err(|error| error.to_string())?
        {
            return model(source);
        }
        let mut draft = preset(BRUSHES[0])?;
        if let Some(attribute) = component_property_attribute(document, node, property)
            .map_err(|error| error.to_string())?
        {
            let value = attribute.value.as_str();
            Color::parse(value).map_err(|_| "Choose a literal brush property. Source bindings and resources can be edited through their existing actions.")?;
            let root = root(&draft)?.id;
            draft.set_attribute(root, "Color", Some(value))?;
        }
        model(draft.source())
    }

    pub(crate) fn change(source: &str, change: BrushChange) -> Result<Self, String> {
        if let BrushChange::Preset { kind } = change {
            return model(preset(&kind)?.source());
        }
        let mut draft = PropertyObjectDraft::new(source)?;
        let root = root(&draft)?;
        let (container, stops) = stop_nodes(draft.document(), root)?;
        match change {
            BrushChange::Set {
                node,
                property,
                value,
            } => {
                let target = if node == root.id {
                    root
                } else {
                    stop(&stops, node)?
                };
                let kind = object_attributes(target.local_name())
                    .iter()
                    .find_map(|(name, kind)| (*name == property).then_some(*kind))
                    .ok_or("Choose a typed property of this brush or stop")?;
                validate(kind, &property, &value)?;
                draft.set_attribute(node, &property, Some(&value))?;
            }
            BrushChange::InsertStop => {
                if root.local_name() == BRUSHES[0] {
                    return Err("Choose a gradient before inserting stops".into());
                }
                let prefix = root
                    .name
                    .split_once(':')
                    .map(|(prefix, _)| format!("{prefix}:"))
                    .unwrap_or_default();
                let source = format!("<{prefix}GradientStop Color=\"White\" Offset=\"0.5\" />");
                draft.insert(Some(container), &source)?;
            }
            BrushChange::RemoveStop { node } => {
                stop(&stops, node)?;
                draft.remove(node)?;
            }
            BrushChange::MoveStop { node, direction } => {
                let index = stops
                    .iter()
                    .position(|stop| stop.id == node)
                    .ok_or("Choose a gradient stop")?;
                let delta = isize::from(direction);
                let next = index
                    .checked_add_signed(delta)
                    .filter(|next| matches!(delta, -1 | 1) && *next < stops.len())
                    .ok_or("The gradient stop is already at the edge of the collection")?;
                let siblings = draft.document().ast().element_children(container);
                let at = |id| siblings.iter().position(|sibling| *sibling == id).unwrap() as isize;
                draft.move_by(node, at(stops[next].id) - at(node))?;
            }
            BrushChange::Preset { .. } => {
                unreachable!("Preset handled before parsing the current draft")
            }
        }
        model(draft.source())
    }
}

fn model(source: String) -> Result<BrushDraft, String> {
    // Each request starts from the detached source. Publish node IDs from that
    // same parser state, since the editing draft reconciles surviving IDs.
    let draft = PropertyObjectDraft::new(&source)?;
    let document = draft.document();
    let root = root(&draft)?;
    let (_, stops) = stop_nodes(document, root)?;
    Ok(BrushDraft {
        source,
        brush: describe(document, root)?,
        stops: stops
            .into_iter()
            .map(|node| describe(document, node))
            .collect::<Result<_, _>>()?,
        presets: BRUSHES,
    })
}

fn preset(kind: &str) -> Result<PropertyObjectDraft, String> {
    if !BRUSHES.contains(&kind) {
        return Err("Choose a supported brush type".into());
    }
    let source = OBJECT_PRESETS
        .iter()
        .map(|(_, source)| source)
        .find(|source| {
            source
                .strip_prefix('<')
                .and_then(|source| source.split_whitespace().next())
                == Some(kind)
        })
        .ok_or("The Studio brush preset is unavailable")?;
    PropertyObjectDraft::new(source)
}

fn root(draft: &PropertyObjectDraft) -> Result<&Node, String> {
    let document = draft.document();
    let children = document.ast().element_children(document.ast().root());
    let [id] = children.as_slice() else {
        return Err("Choose one authored brush object".into());
    };
    let node = document.ast().node(*id).expect("Indexed draft child");
    if node.namespace_uri.as_deref() != Some(NS) || !BRUSHES.contains(&node.local_name()) {
        return Err("Choose an authored FerroUI solid or gradient brush".into());
    }
    Ok(node)
}

fn stop_nodes<'a>(
    document: &'a Document,
    root: &'a Node,
) -> Result<(NodeId, Vec<&'a Node>), String> {
    let children = document.ast().element_children(root.id);
    let property = format!("{}.GradientStops", root.local_name());
    let framework_stop = |node: &Node| {
        node.local_name() == "GradientStop" && node.namespace_uri.as_deref() == Some(NS)
    };
    let containers: Vec<_> = children
        .iter()
        .filter_map(|id| document.ast().node(*id))
        .filter(|node| {
            node.kind == NodeKind::PropertyElement
                && node.namespace_uri.as_deref() == Some(NS)
                && (node.local_name() == property
                    || node.local_name() == "GradientBrush.GradientStops")
        })
        .collect();
    if containers.len() > 1 {
        return Err("The gradient has multiple stop collections".into());
    }
    let parent = containers.first().map_or(root.id, |node| node.id);
    if parent != root.id
        && children
            .iter()
            .filter_map(|id| document.ast().node(*id))
            .any(framework_stop)
    {
        return Err("The gradient uses both direct and explicit stop collections".into());
    }
    let stops = document
        .ast()
        .element_children(parent)
        .into_iter()
        .filter_map(|id| document.ast().node(id))
        .filter(|node| framework_stop(node))
        .collect();
    Ok((parent, stops))
}

fn stop<'a>(nodes: &[&'a Node], node: NodeId) -> Result<&'a Node, String> {
    nodes
        .iter()
        .copied()
        .find(|entry| entry.id == node)
        .ok_or_else(|| "Choose a gradient stop".into())
}

fn describe(document: &Document, node: &Node) -> Result<BrushObject, String> {
    let properties = object_attributes(node.local_name())
        .iter()
        .map(|(name, kind)| {
            let value = component_property_attribute(document, node.id, name)
                .map_err(|error| error.to_string())?
                .map(|attribute| attribute.value.clone());
            Ok(BrushProperty { name, kind, value })
        })
        .collect::<Result<_, String>>()?;
    Ok(BrushObject {
        node: node.id,
        properties,
        kind: node.local_name().into(),
    })
}

fn validate(kind: &str, property: &str, value: &str) -> Result<(), String> {
    let value = value.trim();
    if value.starts_with('{') {
        return MarkupValue::parse(value).map(|_| ());
    }
    let valid = match kind {
        "Color" => Color::parse(value).is_ok(),
        "RelativePoint" => RelativePoint::parse(value)
            .is_ok_and(|point| point.point.x.is_finite() && point.point.y.is_finite()),
        "RelativeScalar" => RelativeScalar::parse(value)
            .is_ok_and(|radius| radius.scalar.is_finite() && radius.scalar >= 0.0),
        "GradientSpreadMethod" => {
            value.eq_ignore_ascii_case("pad")
                || value.eq_ignore_ascii_case("repeat")
                || value.eq_ignore_ascii_case("reflect")
        }
        "f64" => value.parse::<f64>().is_ok_and(|number| {
            number.is_finite()
                && (!matches!(property, "Offset" | "Opacity") || (0.0..=1.0).contains(&number))
        }),
        _ => false,
    };
    if valid {
        Ok(())
    } else {
        Err(format!("Enter a valid {property} value"))
    }
}
