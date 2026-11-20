use crate::document::DesignDocument;
use crate::projects::Projects;
use crate::property_fields::{can_add_property, registry};
use crate::protocol::{EditRequest, EditResult, Operation, MAX_SOURCE_BYTES};
use crate::renderer::Renderer;
use ferroui_designer_core::{Edit, Node, NodeId, NodeKind, TextPatch};
use ferroui_designer_language::{TextEdit, plan_event_handler, routed_events};
use ferroui_designer_tools::ExpressionPolicy;

const COMMON_PROPERTIES: &[&str] = &[
    "Name",
    "Width",
    "Height",
    "Margin",
    "Padding",
    "Spacing",
    "Background",
    "Foreground",
    "Text",
    "Content",
    "FontSize",
    "Orientation",
    "HorizontalAlignment",
    "VerticalAlignment",
    "Grid.Row",
    "Grid.Column",
    "Canvas.Left",
    "Canvas.Top",
];

pub(crate) fn edit(request: &EditRequest, renderer: Option<&Renderer>, projects: &Projects) -> Result<EditResult, String> {
    let design = DesignDocument::parse(&request.document, projects.source(&request.document)?)?;
    let mut candidate = design.xaml.snapshot_without_history();
    let mut selected = None;
    let patches = match &request.operation {
        Operation::ApplyCounterpart { source } => {
            if source.len() > MAX_SOURCE_BYTES {
                return Err("Designer source exceeds 1 MiB".into());
            }
            match &design.rsx {
                Some(rsx) => rsx.apply_xaml(source)?,
                None => design.rsx_catalog().apply_rsx(&design.xaml, source)?,
            }
        }
        Operation::SetProperty { node, name, .. } | Operation::ResetProperty { node, name } => {
            let target = editable_node(&design, *node)?;
            let value = match &request.operation {
                Operation::SetProperty { value, .. } => Some(value.as_str()),
                _ => None,
            };
            let writable = editable_attribute(&design, target, name)
                || value.is_some() && can_add_property(&design, target, name);
            if !writable {
                return Err(format!("{name} is unavailable in the property inspector"));
            }
            if let Some((i, _)) = design.rsx_node(*node) {
                let rsx = design.rsx.as_ref().unwrap();
                return if name == "Name" {
                    rust_name_edit(&design, &[*node], value)
                } else {
                    rust_edit(&design, rsx.edit_property(i, name, value)?)
                };
            }
            let edits = registry()
                .plan(
                    &candidate,
                    &[*node],
                    name,
                    value,
                    ExpressionPolicy::PreserveExpressions,
                )
                .map_err(|e| e.to_string())?;
            let change = candidate
                .apply("Set property", edits)
                .map_err(|e| e.to_string())?;
            translated_patches(&design, candidate.source(), change.patches)?
        }
        Operation::SetEvent { node, name, value } => {
            let target = editable_node(&design, *node)?;
            if let Some((i, _)) = design.rsx_node(*node) {
                if !routed_events(design.schema(), &design.xaml, target).contains_key(name) {
                    return Err(format!("{name} is unavailable for this control"));
                }
                let rsx = design.rsx.as_ref().unwrap();
                return rust_edit(&design, rsx.set_event(i, name, value.as_deref())?);
            }
            let edit = plan_event_handler(design.schema(), &candidate, target, name, value.as_deref())?;
            candidate.apply("Edit event handler", vec![edit]).map_err(|error| error.to_string())?.patches
        }
        Operation::ApplyResource { frame, node, resource, property, dynamic } => {
            let edits = crate::resources::apply(&design, renderer, *frame, *node, *resource, property, *dynamic)?;
            candidate.apply("Apply resource", edits).map_err(|error| error.to_string())?.patches
        }
        Operation::RemoveNode { node } => {
            let target = editable_node(&design, *node)?;
            if !can_remove(&design, target) {
                return Err("Select a removable child control".into());
            }
            let change = candidate
                .apply("Remove control", vec![Edit::RemoveNode { node: *node }])
                .map_err(|e| e.to_string())?;
            translated_patches(&design, candidate.source(), change.patches)?
        }
        Operation::Place { frame, placement } => {
            let stage = renderer.ok_or("Render the design before placing controls")?.stage(*frame)?;
            let session = stage.session.borrow();
            if session.document().source() != design.xaml.source()
                || session.document_uri() != request.document.uri
            {
                return Err("The source changed during the drag. Start again with the current preview.".into());
            }
            drop(session);
            let (edits, hint) = stage.plan(placement, false)?;
            let change = candidate.apply(hint.description, edits.clone()).map_err(|error| error.to_string())?;
            if !change.is_empty() {
                selected = crate::edit_selection::dropped_node(&candidate, &change, &placement.origin);
            }
            if design.rsx.is_some() {
                selected.map_or_else(|| Ok(Vec::new()), |node| {
                    crate::rsx_placement::patches(&design, &candidate, node, &placement.origin, &edits)
                })?
            } else { change.patches }
        }
        Operation::Resize { frame, node, handle, dx, dy } => {
            let stage = renderer.ok_or("Render the design before resizing")?.stage(*frame)?;
            let session = stage.session.borrow();
            if session.document().source() != design.xaml.source() || session.document_uri() != request.document.uri {
                return Err("The source changed during the resize".into());
            }
            let edits = session.plan_resize_handle(*node, *handle, *dx, *dy)?;
            if !edits.is_empty() { selected = Some(*node); }
            let change = candidate.apply("Resize control", edits).map_err(|error| error.to_string())?;
            translated_patches(&design, candidate.source(), change.patches)?
        }
        Operation::Apply { label, edits } => {
            if !design.editable_structure() {
                return Err("Author this Rust block directly to preserve its expressions and comments".into());
            }
            if edits.is_empty() || edits.len() > 4096 || label.len() > 256 {
                return Err("Supply a label and between 1 and 4096 source edits".into());
            }
            if edits.iter().any(|edit| matches!(edit, Edit::ReplaceSource { .. } | Edit::Text { .. })) {
                return Err("Use the versioned source buffer tool to write raw source".into());
            }
            let change = candidate.apply(label, edits.clone()).map_err(|error| error.to_string())?;
            translated_patches(&design, candidate.source(), change.patches)?
        }
    };
    let selection = selected.and_then(|node| crate::edit_selection::authored_range(&design, &candidate, &patches, node));
    let mut result = editor_edits(&design, patches)?;
    result.selection = selection;
    Ok(result)
}

pub(crate) fn rust_name_edit(
    design: &DesignDocument<'_>,
    nodes: &[NodeId],
    value: Option<&str>,
) -> Result<EditResult, String> {
    let projection = design
        .rsx
        .as_ref()
        .ok_or("Open the current RSX invocation")?;
    let targets = nodes
        .iter()
        .map(|node| {
            design
                .rsx_node(*node)
                .map(|(index, _)| index)
                .ok_or("Choose a mapped Rust control")
        })
        .collect::<Result<Vec<_>, _>>()?;
    let patches = projection.edit_control_names(&targets, value)?;
    let ranges = crate::edit_selection::rsx_control_ranges(design, &patches, nodes)?;
    let mut result = editor_edits(design, patches)?;
    result.retained_selection = ranges;
    Ok(result)
}

fn rust_edit(design: &DesignDocument<'_>, patch: TextPatch) -> Result<EditResult, String> {
    let mut candidate = design.source.to_owned();
    candidate.replace_range(patch.span.range(), &patch.replacement);
    if candidate.len() > MAX_SOURCE_BYTES {
        return Err("Designer source exceeds 1 MiB".into());
    }
    design.rsx_catalog().project(&candidate, design.rsx.as_ref().unwrap().invocation.start)?;
    editor_edits(design, vec![patch])
}

fn translated_patches(
    design: &DesignDocument<'_>,
    draft: &str,
    patches: Vec<TextPatch>,
) -> Result<Vec<TextPatch>, String> {
    match &design.rsx {
        Some(rsx) => rsx.apply_xaml(draft),
        None => Ok(patches),
    }
}

pub(crate) fn editor_edits(
    design: &DesignDocument<'_>,
    patches: Vec<TextPatch>,
) -> Result<EditResult, String> {
    let final_size = patches
        .iter()
        .fold(design.source.len() as i64, |size, patch| {
            size - patch.span.len() as i64 + patch.replacement.len() as i64
        });
    if final_size > MAX_SOURCE_BYTES as i64 {
        return Err("Designer source exceeds 1 MiB".into());
    }
    let edits = patches
        .into_iter()
        .filter(|patch| design.source.get(patch.span.range()) != Some(patch.replacement.as_str()))
        .map(|patch| {
            Ok(TextEdit {
                range: design.range(patch.span)?,
                new_text: patch.replacement,
            })
        })
        .collect::<Result<_, String>>()?;
    Ok(EditResult { edits, selection: None, timeline_pose: None, timeline_program_key: None, timeline_keys: None, retained_selection: Vec::new() })
}

fn editable_node<'a>(design: &'a DesignDocument<'_>, id: NodeId) -> Result<&'a Node, String> {
    design
        .xaml
        .ast()
        .node(id)
        .filter(|node| node.kind == NodeKind::Element)
        .ok_or_else(|| "Select a source control".into())
}

pub(crate) fn can_remove(design: &DesignDocument<'_>, node: &Node) -> bool {
    design.editable_structure() && node.kind == NodeKind::Element && node.parent.is_some()
}

pub(crate) fn editable_attribute(design: &DesignDocument<'_>, node: &Node, name: &str) -> bool {
    if let Some((_, mapped)) = design.rsx_node(node.id) {
        return mapped.properties.contains_key(name);
    }
    node.kind == NodeKind::Element
        && node.attribute_info(name).is_some_and(|attr| {
            !attr.is_namespace()
                && (!attr.value.starts_with('{') || attr.value.starts_with("{}"))
                && if attr.namespace_uri.as_deref()
                    == Some("http://schemas.microsoft.com/winfx/2006/xaml")
                {
                    name.ends_with(":Name")
                } else {
                    attr.namespace_uri.is_none()
                        && design.schema()
                            .property(node.local_name(), node.namespace_uri.as_deref(), name)
                            .is_none_or(|info| !info.read_only && !info.event)
                }
        })
}

pub(crate) fn addable_properties(design: &DesignDocument<'_>, node: &Node) -> Vec<String> {
    COMMON_PROPERTIES
        .iter()
        .filter(|name| can_add_property(design, node, name))
        .map(|name| (*name).into())
        .collect()
}
