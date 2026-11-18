use crate::document::DesignDocument;
use crate::projects::Projects;
use crate::protocol::Source;
use crate::renderer::Renderer;
use ferroui_designer_core::{Edit, Node, NodeId};
use ferroui_designer_language::Range;
use ferroui_designer_tools::{
    ComponentAction, ComponentCatalog, ComponentKind, plan_component_action,
    plan_component_action_in_context, visible_component_resources,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Deserialize)]
pub(crate) struct ResourceRequest {
    pub(crate) frame: u64,
    #[serde(flatten)]
    pub(crate) source: Source,
    pub(crate) target: Option<NodeId>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ResourceRow {
    id: NodeId,
    name: String,
    type_name: String,
    scope: String,
    range: Range,
    usages: usize,
    visible: bool,
    properties: Vec<String>,
}

pub(crate) fn list(
    input: &ResourceRequest,
    renderer: Option<&Renderer>,
    projects: &Projects,
) -> Result<Vec<ResourceRow>, String> {
    let design = DesignDocument::parse(&input.source, projects.source(&input.source)?)?;
    if design.rsx.is_some() {
        return Ok(Vec::new());
    }
    let catalog =
        ComponentCatalog::from_document(&design.xaml).map_err(|error| error.to_string())?;
    let visible: BTreeSet<_> = if let Some(target) = input.target {
        let entries = match renderer.and_then(|renderer| renderer.stage(input.frame).ok()) {
            Some(stage) => stage.session.borrow().visible_component_resources(target),
            None => {
                visible_component_resources(&design.xaml, target).map_err(|error| error.to_string())
            }
        }?;
        entries.into_iter().map(|entry| entry.node).collect()
    } else {
        BTreeSet::new()
    };
    let target = input.target.and_then(|id| design.xaml.ast().node(id));
    catalog
        .entries
        .into_iter()
        .filter(|entry| {
            entry.key.is_some()
                || matches!(
                    entry.kind,
                    ComponentKind::ResourceDictionary
                        | ComponentKind::Style
                        | ComponentKind::ControlTheme
                        | ComponentKind::ControlTemplate
                        | ComponentKind::DataTemplate
                )
        })
        .map(|entry| {
            let node = design
                .xaml
                .ast()
                .node(entry.node)
                .expect("catalog source node");
            let is_visible = visible.contains(&entry.node);
            let properties = target
                .filter(|_| is_visible)
                .map_or_else(Vec::new, |target| {
                    compatible_properties(&design, node, target)
                });
            let owner = entry.owner.and_then(|id| design.xaml.ast().node(id));
            Ok(ResourceRow {
                id: entry.node,
                name: entry.display_name,
                type_name: node.local_name().into(),
                scope: owner
                    .map_or("Document", |owner| {
                        owner.xaml_name().unwrap_or(owner.local_name())
                    })
                    .into(),
                range: design.range(node.name_span)?,
                usages: entry.usages,
                visible: is_visible,
                properties,
            })
        })
        .collect()
}

pub(crate) fn apply(
    design: &DesignDocument<'_>,
    renderer: Option<&Renderer>,
    frame: u64,
    node: NodeId,
    resource: NodeId,
    property: &str,
    dynamic: bool,
) -> Result<Vec<Edit>, String> {
    if design.rsx.is_some() {
        return Err("XAML resource references are authored in XAML documents".into());
    }
    let target = design
        .xaml
        .ast()
        .node(node)
        .ok_or("Select a source element")?;
    let definition = design
        .xaml
        .ast()
        .node(resource)
        .ok_or("The resource declaration changed")?;
    if !compatible_properties(design, definition, target)
        .iter()
        .any(|name| name == property)
    {
        return Err("Choose a compatible property on the selected control".into());
    }
    let action = ComponentAction::ApplyResource {
        resource,
        targets: vec![node],
        property: property.into(),
        dynamic,
    };
    let context = renderer
        .and_then(|renderer| renderer.stage(frame).ok())
        .and_then(|stage| stage.session.borrow().component_resource_context(node));
    let edits = match context {
        Some(context) => plan_component_action_in_context(&design.xaml, &action, &context),
        None => plan_component_action(&design.xaml, &action),
    };
    edits.map_err(|error| error.to_string())
}

fn compatible_properties(
    design: &DesignDocument<'_>,
    resource: &Node,
    target: &Node,
) -> Vec<String> {
    let kind = resource.local_name();
    design.schema()
        .properties(target.local_name(), target.namespace_uri.as_deref())
        .into_values()
        .filter(|info| !info.attached && !info.read_only && !info.event)
        .filter(|info| {
            crate::edits::editable_attribute(design, target, &info.name)
                || target.attribute_info(&info.name).is_none()
        })
        .filter(|info| match kind {
            "SolidColorBrush"
            | "LinearGradientBrush"
            | "RadialGradientBrush"
            | "ImageBrush"
            | "DrawingBrush"
            | "VisualBrush" => info.type_name.contains("IBrush"),
            "Color" => info.type_name == "Color",
            "String" => {
                matches!(info.type_name.as_str(), "String" | "Option<String>")
                    || matches!(info.name.as_str(), "Content" | "Header")
            }
            "Double" => info.type_name == "f64",
            "Int32" => info.type_name == "i32",
            "Boolean" => info.type_name == "bool",
            "Thickness" => info.type_name == "Thickness",
            "CornerRadius" => info.type_name == "CornerRadius",
            "ControlTheme" => info.type_name.contains("Ref<ControlTheme>"),
            "ControlTemplate" => info.type_name.contains("IControlTemplate"),
            "DataTemplate" | "TreeDataTemplate" => info.type_name.contains("IDataTemplate"),
            _ => false,
        })
        .map(|info| info.name)
        .collect()
}
