use crate::document::{schema, DesignDocument};
use crate::protocol::{EditRequest, EditResult, Operation, MAX_SOURCE_BYTES};
use crate::renderer::Renderer;
use ferroui_designer_core::{Edit, Node, NodeId, NodeKind, TextPatch};
use ferroui_designer_language::TextEdit;
use ferroui_designer_rsx::apply_rsx;
use ferroui_designer_tools::{ExpressionPolicy, PropertyRegistry};

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

pub(crate) fn edit(request: &EditRequest, renderer: Option<&Renderer>) -> Result<EditResult, String> {
    let design = DesignDocument::parse(&request.document)?;
    let mut candidate = design.xaml.snapshot_without_history();
    let mut selected = None;
    let patches = match &request.operation {
        Operation::ApplyCounterpart { source } => {
            if source.len() > MAX_SOURCE_BYTES {
                return Err("Designer source exceeds 1 MiB".into());
            }
            match &design.rsx {
                Some(rsx) => rsx.apply_xaml(source)?,
                None => apply_rsx(&design.xaml, source)?,
            }
        }
        Operation::SetProperty { node, name, value } => {
            let target = editable_node(&design, *node)?;
            if let Some((i, mapped)) = design.rsx_node(*node) {
                if mapped.properties.contains_key(name) {
                    return editor_edits(
                        &design,
                        vec![design.rsx.as_ref().unwrap().set_property(i, name, value)?],
                    );
                }
            }
            if !editable_attribute(&design, target, name) && !addable(&design, target, name) {
                return Err(format!("{name} is unavailable in the property inspector"));
            }
            let edits = PropertyRegistry::built_in()
                .plan(
                    &candidate,
                    &[*node],
                    name,
                    Some(value),
                    ExpressionPolicy::PreserveExpressions,
                )
                .map_err(|e| e.to_string())?;
            let change = candidate
                .apply("Set property", edits)
                .map_err(|e| e.to_string())?;
            translated_patches(&design, candidate.source(), change.patches)?
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
            if !design.editable_structure() {
                return Err("Edit this RSX structure in Rust to preserve its expressions and comments".into());
            }
            let stage = renderer.ok_or("Render the design before placing controls")?.stage(*frame)?;
            let session = stage.session.borrow();
            if session.document().source() != design.xaml.source()
                || session.document_uri() != request.document.uri
            {
                return Err("The source changed during the drag. Start again with the current preview.".into());
            }
            drop(session);
            let (edits, hint) = stage.plan(placement)?;
            let change = candidate.apply(hint.description, edits).map_err(|error| error.to_string())?;
            if !change.is_empty() {
                selected = crate::edit_selection::dropped_node(&candidate, &change, &placement.origin);
            }
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

fn editor_edits(
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
    Ok(EditResult { edits, selection: None })
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
                        && schema()
                            .property(node.local_name(), node.namespace_uri.as_deref(), name)
                            .is_none_or(|info| !info.read_only && !info.event)
                }
        })
}

fn addable(design: &DesignDocument<'_>, node: &Node, name: &str) -> bool {
    COMMON_PROPERTIES.contains(&name)
        && design.editable_structure()
        && node.kind == NodeKind::Element
        && node.attribute_info(name).is_none()
        && !(name == "Name" && node.xaml_name().is_some())
        && design
            .rsx_node(node.id)
            .is_none_or(|(_, node)| node.supports_literal(name))
        && schema()
            .property(node.local_name(), node.namespace_uri.as_deref(), name)
            .is_some_and(|info| !info.read_only && !info.event)
        && (name != "Content" || node.children.is_empty())
}

pub(crate) fn addable_properties(design: &DesignDocument<'_>, node: &Node) -> Vec<String> {
    COMMON_PROPERTIES
        .iter()
        .filter(|name| addable(design, node, name))
        .map(|name| (*name).into())
        .collect()
}
