use crate::document::{schema, DesignDocument};
use crate::protocol::{EditRequest, Operation, MAX_SOURCE_BYTES};
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

pub(crate) fn edit(request: &EditRequest) -> Result<Vec<TextEdit>, String> {
    let design = DesignDocument::parse(&request.document)?;
    let mut candidate = design.xaml.snapshot_without_history();
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
        Operation::InsertChild { node, control } => {
            let parent = editable_node(&design, *node)?;
            if !can_insert(&design, parent) {
                return Err("Select a panel that accepts children".into());
            }
            let source = match control.as_str() {
                "Button" => r#"<Button xmlns="https://github.com/ferroui" Content="Button"/>"#,
                "TextBlock" => r#"<TextBlock xmlns="https://github.com/ferroui" Text="Text"/>"#,
                "StackPanel" => r#"<StackPanel xmlns="https://github.com/ferroui"/>"#,
                "Border" => r#"<Border xmlns="https://github.com/ferroui" Padding="12"/>"#,
                _ => return Err("Unknown toolbox control".into()),
            };
            let index = candidate.ast().element_children(*node).len();
            let change = candidate
                .apply(
                    "Insert control",
                    vec![Edit::InsertChild {
                        parent: *node,
                        index,
                        source: source.into(),
                    }],
                )
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
    };
    editor_edits(&design, patches)
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
) -> Result<Vec<TextEdit>, String> {
    let final_size = patches
        .iter()
        .fold(design.source.len() as i64, |size, patch| {
            size - patch.span.len() as i64 + patch.replacement.len() as i64
        });
    if final_size > MAX_SOURCE_BYTES as i64 {
        return Err("Designer source exceeds 1 MiB".into());
    }
    patches
        .into_iter()
        .filter(|patch| design.source.get(patch.span.range()) != Some(patch.replacement.as_str()))
        .map(|patch| {
            Ok(TextEdit {
                range: design.range(patch.span)?,
                new_text: patch.replacement,
            })
        })
        .collect()
}

fn editable_node<'a>(design: &'a DesignDocument<'_>, id: NodeId) -> Result<&'a Node, String> {
    design
        .xaml
        .ast()
        .node(id)
        .filter(|node| node.kind == NodeKind::Element)
        .ok_or_else(|| "Select a source control".into())
}

pub(crate) fn can_insert(design: &DesignDocument<'_>, node: &Node) -> bool {
    if !design.editable_structure() || node.kind != NodeKind::Element {
        return false;
    }
    schema()
        .type_info(node.local_name(), node.namespace_uri.as_deref())
        .is_some_and(|info| {
            info.namespace == ferroui_designer_language::FRAMEWORK_NAMESPACE
                && matches!(
                    info.name.as_str(),
                    "Grid" | "StackPanel" | "Canvas" | "WrapPanel" | "DockPanel" | "UniformGrid"
                )
        })
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
