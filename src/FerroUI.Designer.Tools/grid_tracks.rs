//! Structural Grid edits preserve the authored definition representation and child cells.
//! A host can preview these commands without mutating either the document or live controls.
use crate::{Axis, GridLength, Result, ToolError, error, require};
use ferroui_designer_core::{Document, Edit, Node, NodeId, NodeKind};
use std::collections::BTreeMap;

/// Indices refer to the current axis. Insertion is before `index`, including `len()` to append.
#[derive(Clone, Debug, PartialEq)]
pub enum GridTrackChange {
    Set { index: usize, length: GridLength },
    Insert { index: usize, length: GridLength },
    Remove { index: usize },
}

/// Effective placement along one axis, supplied by a native host after resolving styles.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GridTrackPlacement {
    pub index: usize,
    pub span: usize,
}

struct AxisInfo {
    collection: &'static str,
    definition: &'static str,
    length: &'static str,
    position: &'static str,
    span: &'static str,
}

impl AxisInfo {
    fn new(axis: Axis) -> Self {
        match axis {
            Axis::Horizontal => Self {
                collection: "ColumnDefinitions",
                definition: "ColumnDefinition",
                length: "Width",
                position: "Grid.Column",
                span: "Grid.ColumnSpan",
            },
            Axis::Vertical => Self {
                collection: "RowDefinitions",
                definition: "RowDefinition",
                length: "Height",
                position: "Grid.Row",
                span: "Grid.RowSpan",
            },
        }
    }
}

struct Definitions<'a> {
    owner: &'a Node,
    collection: Option<&'a Node>,
    nodes: Vec<&'a Node>,
    values: Vec<String>,
    attribute: String,
}

fn framework_namespace(namespace: Option<&str>) -> bool {
    matches!(
        namespace,
        None | Some(
            "https://github.com/ferroui"
                | "using:FerroUI.Controls"
                | "clr-namespace:FerroUI.Controls;assembly=FerroUI.Controls"
        )
    )
}

fn attribute<'a>(
    document: &Document,
    node: &'a Node,
    name: &str,
) -> Result<Option<&'a ferroui_designer_core::Attribute>> {
    let mut attributes = node.attributes.iter().filter(|attribute| {
        attribute.name.rsplit(':').next() == Some(name)
            && if name.contains('.') && !attribute.name.contains(':') {
                framework_namespace(
                    document
                        .ast()
                        .namespaces(node.id)
                        .get("")
                        .map(String::as_str),
                )
            } else {
                framework_namespace(attribute.namespace_uri.as_deref())
            }
    });
    let result = attributes.next();
    require(
        attributes.next().is_none(),
        format!("{name} is declared more than once"),
    )?;
    Ok(result)
}

fn property_elements<'a>(document: &'a Document, node: &Node, name: &str) -> Vec<&'a Node> {
    node.children
        .iter()
        .filter_map(|id| document.ast().node(*id))
        .filter(|child| {
            child.kind == NodeKind::PropertyElement
                && framework_namespace(child.namespace_uri.as_deref())
                && (child.local_name() == name || child.local_name().ends_with(&format!(".{name}")))
        })
        .collect()
}

fn definitions<'a>(
    document: &'a Document,
    panel: NodeId,
    axis: &AxisInfo,
) -> Result<Definitions<'a>> {
    require(document.is_valid(), "Grid editing requires valid XAML")?;
    let owner = document
        .ast()
        .node(panel)
        .ok_or_else(|| error("The Grid no longer exists"))?;
    require(
        owner.local_name() == "Grid" && framework_namespace(owner.namespace_uri.as_deref()),
        "This editor accepts framework Grid definitions; register an adapter for a custom panel",
    )?;
    let authored = attribute(document, owner, axis.collection)?;
    let collections = property_elements(document, owner, axis.collection);
    require(
        collections.len() <= 1 && (authored.is_none() || collections.is_empty()),
        "Grid definitions have more than one source; resolve the XAML before editing",
    )?;
    let collection = collections.first().copied();
    let mut nodes = Vec::new();
    let mut values = if let Some(authored) = authored {
        require(
            !authored.value.trim_start().starts_with('{'),
            "Grid definitions are bound; edit their source",
        )?;
        authored
            .value
            .split(',')
            .map(str::trim)
            .map(str::to_owned)
            .collect::<Vec<_>>()
    } else if let Some(collection) = collection {
        for child in document.ast().element_children(collection.id) {
            let child = document
                .ast()
                .node(child)
                .ok_or_else(|| error("Missing Grid definition"))?;
            require(
                child.local_name() == axis.definition
                    && framework_namespace(child.namespace_uri.as_deref()),
                "The Grid contains a custom definition; edit it in XAML",
            )?;
            nodes.push(child);
        }
        nodes
            .iter()
            .map(|node| {
                if !property_elements(document, node, axis.length).is_empty() {
                    Ok("(object value)".to_owned())
                } else {
                    Ok(attribute(document, node, axis.length)?
                        .map_or("*", |value| value.value.as_str())
                        .to_owned())
                }
            })
            .collect::<Result<Vec<_>>>()?
    } else {
        Vec::new()
    };
    // Empty definitions have one implicit star track in the native layout engine.
    if values.is_empty() || values.iter().all(|value| value.is_empty()) {
        values = vec!["*".into()];
    }
    Ok(Definitions {
        owner,
        collection,
        nodes,
        values,
        attribute: authored.map_or_else(|| axis.collection.to_owned(), |value| value.name.clone()),
    })
}

/// Returns lexical lengths, including bindings on individual definitions. Inspectors can
/// display such values without replacing an expression or discarding definition metadata.
pub fn grid_track_values(document: &Document, panel: NodeId, axis: Axis) -> Result<Vec<String>> {
    Ok(definitions(document, panel, &AxisInfo::new(axis))?.values)
}

/// Plans one atomic definition/cell edit. Inserts expand a child spanning the new track;
/// removals shrink spans and move children in a removed track to its surviving neighbour.
/// Bound cell positions are rejected because their future effective index is not knowable.
pub fn plan_grid_track_change(
    document: &Document,
    panel: NodeId,
    axis: Axis,
    change: GridTrackChange,
) -> Result<Vec<Edit>> {
    plan_change(document, panel, axis, change, None)
}

/// Native variant: every authored child must have a measured placement. Hosts must reject
/// active bindings before supplying these values, as a new local value would detach them.
pub fn plan_grid_track_change_with_placements(
    document: &Document,
    panel: NodeId,
    axis: Axis,
    change: GridTrackChange,
    placements: &BTreeMap<NodeId, GridTrackPlacement>,
) -> Result<Vec<Edit>> {
    plan_change(document, panel, axis, change, Some(placements))
}

fn plan_change(
    document: &Document,
    panel: NodeId,
    axis: Axis,
    change: GridTrackChange,
    placements: Option<&BTreeMap<NodeId, GridTrackPlacement>>,
) -> Result<Vec<Edit>> {
    let info = AxisInfo::new(axis);
    let definitions = definitions(document, panel, &info)?;
    let count = definitions.values.len();
    let mut values = definitions.values.clone();
    let (index, insertion, length) = match &change {
        GridTrackChange::Set { index, length } => {
            require(*index < count, "Grid track index is out of range")?;
            length.validate()?;
            require(
                GridLength::parse(&values[*index]).is_ok(),
                "This track is bound or object-valued; edit its source",
            )?;
            if GridLength::parse(&values[*index])? == *length {
                return Ok(Vec::new());
            }
            values[*index] = length.to_xaml();
            (*index, None, Some(*length))
        }
        GridTrackChange::Insert { index, length } => {
            require(*index <= count, "Grid insertion index is out of range")?;
            require(
                count < 4096,
                "The layout editor supports at most 4096 tracks per axis",
            )?;
            length.validate()?;
            values.insert(*index, length.to_xaml());
            (*index, Some(true), Some(*length))
        }
        GridTrackChange::Remove { index } => {
            require(*index < count, "Grid track index is out of range")?;
            require(count > 1, "Keep at least one track in each Grid axis")?;
            values.remove(*index);
            (*index, Some(false), None)
        }
    };
    let mut edits = Vec::new();
    if let Some(collection) = definitions.collection {
        if definitions.nodes.is_empty() {
            // Materialize the implicit track alongside the inserted one in the existing
            // property element, retaining its comments and namespace context.
            for (index, value) in values.iter().enumerate() {
                edits.push(Edit::InsertChild {
                    parent: collection.id,
                    index,
                    source: definition_source(&definitions, &info, value),
                });
            }
        } else {
            match change {
                GridTrackChange::Set { .. } => {
                    let node = definitions.nodes[index];
                    require(
                        property_elements(document, node, info.length).is_empty(),
                        "This track has an object value; edit its XAML",
                    )?;
                    edits.push(Edit::SetAttribute {
                        node: node.id,
                        name: attribute(document, node, info.length)?
                            .map_or_else(|| info.length.to_owned(), |value| value.name.clone()),
                        value: length.unwrap().to_xaml(),
                    });
                }
                GridTrackChange::Insert { .. } => edits.push(Edit::InsertChild {
                    parent: collection.id,
                    index,
                    source: definition_source(&definitions, &info, &length.unwrap().to_xaml()),
                }),
                GridTrackChange::Remove { .. } => edits.push(Edit::RemoveNode {
                    node: definitions.nodes[index].id,
                }),
            }
        }
    } else {
        // Compact attribute syntax cannot contain binding/object values for individual tracks.
        for value in &values {
            GridLength::parse(value)?;
        }
        edits.push(Edit::SetAttribute {
            node: panel,
            name: definitions.attribute.clone(),
            value: values.join(","),
        });
    }
    if let Some(insert) = insertion {
        for child in content_children(document, definitions.owner)? {
            let authored_position = integer_property(document, child, info.position, 0)?;
            let authored_span = integer_property(document, child, info.span, 1)?;
            let (position, span) = if let Some(placements) = placements {
                let resolved = placements
                    .get(&child.id)
                    .ok_or_else(|| error("A Grid child has no resolved placement"))?;
                (resolved.index, resolved.span)
            } else {
                (authored_position, authored_span)
            };
            require(span > 0, "Grid spans must be positive")?;
            // Native Grid clamps out-of-range positions/spans; preserve those effective cells.
            let start = position.min(count - 1);
            let end = start.saturating_add(span).min(count);
            let (new_start, new_span) = if insert {
                (
                    start + usize::from(start >= index),
                    end - start + usize::from(start < index && end > index),
                )
            } else {
                let new_start = (start - usize::from(start > index)).min(count - 2);
                (
                    new_start,
                    (end - start - usize::from(start <= index && index < end)).max(1),
                )
            };
            for (property, old, new) in [
                (info.position, position, new_start),
                (info.span, span, new_span),
            ] {
                if old != new {
                    let name = attached_name(document, child, property, &mut edits)?;
                    edits.push(Edit::SetAttribute {
                        node: child.id,
                        name,
                        value: new.to_string(),
                    });
                }
            }
        }
    }
    let mut candidate = document.clone();
    candidate
        .apply("Validate Grid track change", edits.clone())
        .map_err(error)?;
    Ok(edits)
}

fn attached_name(
    document: &Document,
    node: &Node,
    property: &str,
    edits: &mut Vec<Edit>,
) -> Result<String> {
    if let Some(value) = attribute(document, node, property)? {
        return Ok(value.name.clone());
    }
    let namespaces = document.ast().namespaces(node.id);
    if framework_namespace(namespaces.get("").map(String::as_str)) {
        return Ok(property.to_owned());
    }
    if let Some((prefix, _)) = namespaces
        .iter()
        .find(|(prefix, uri)| !prefix.is_empty() && framework_namespace(Some(uri.as_str())))
    {
        return Ok(format!("{prefix}:{property}"));
    }
    let mut prefix = "ui".to_owned();
    let mut suffix = 1;
    while namespaces.contains_key(&prefix) {
        prefix = format!("ui{suffix}");
        suffix += 1;
    }
    let name = format!("xmlns:{prefix}");
    if !edits.iter().any(|edit| matches!(edit, Edit::SetAttribute { node: id, name: existing, .. } if *id == node.id && existing == &name)) {
        edits.push(Edit::SetAttribute { node: node.id, name, value: "https://github.com/ferroui".into() });
    }
    Ok(format!("{prefix}:{property}"))
}

fn definition_source(definitions: &Definitions<'_>, info: &AxisInfo, value: &str) -> String {
    let prefix = definitions
        .collection
        .unwrap_or(definitions.owner)
        .name
        .rsplit_once(':')
        .map(|(prefix, _)| prefix);
    let name = prefix.map_or_else(
        || info.definition.to_owned(),
        |prefix| format!("{prefix}:{}", info.definition),
    );
    format!("<{name} {}=\"{value}\" />", info.length)
}

fn content_children<'a>(document: &'a Document, grid: &Node) -> Result<Vec<&'a Node>> {
    let collections = property_elements(document, grid, "Children");
    require(
        collections.len() <= 1,
        "The Grid has more than one Children collection",
    )?;
    let mut children = Vec::new();
    for id in document.ast().element_children(grid.id) {
        if let Some(node) = document.ast().node(id) {
            if node.kind == NodeKind::Element {
                children.push(node);
            } else if collections
                .first()
                .is_some_and(|collection| collection.id == id)
            {
                children.extend(
                    document
                        .ast()
                        .element_children(id)
                        .into_iter()
                        .filter_map(|id| document.ast().node(id))
                        .filter(|node| node.kind == NodeKind::Element),
                );
            }
        }
    }
    Ok(children)
}

fn integer_property(document: &Document, node: &Node, name: &str, default: usize) -> Result<usize> {
    require(
        property_elements(document, node, name).is_empty(),
        format!("{name} has an object value; edit its source before changing tracks"),
    )?;
    match attribute(document, node, name)? {
        None => Ok(default),
        Some(value) => value.value.trim().parse().map_err(|_| {
            ToolError(format!(
                "{name} is bound or invalid; edit its source before changing tracks"
            ))
        }),
    }
}

#[cfg(test)]
#[path = "grid_tracks_tests.rs"]
mod tests;
