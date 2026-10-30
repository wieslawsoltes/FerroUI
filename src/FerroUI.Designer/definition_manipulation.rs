//! Nonvisual template declarations own a single visual layout root. Their measured
//! coordinate space comes from that root's real native parent, while every edit and
//! helper handle continues to address the original source node.
use crate::DesignerSession;
use ferroui_base::{Ref, Visual};
use ferroui_designer_core::{Document, Edit, NodeId, NodeKind};
use ferroui_designer_tools::{
    LayoutAction, LayoutAdapter, LayoutCapabilities, LayoutContext, LayoutHelperRequest,
    LayoutHelperScene, LayoutPlacement, LayoutRegistry, Point, ToolError,
};

const TEMPLATE_NAMESPACES: [&str; 3] = [
    "https://github.com/ferroui",
    "using:FerroUI.Markup.Xaml.Templates",
    "clr-namespace:FerroUI.Markup.Xaml.Templates;assembly=FerroUI.Markup.Xaml",
];
const TEMPLATE_TYPES: [&str; 3] = ["ControlTemplate", "DataTemplate", "TreeDataTemplate"];

pub(crate) fn layout_registry() -> LayoutRegistry {
    let mut registry = LayoutRegistry::built_in();
    for namespace in TEMPLATE_NAMESPACES {
        for name in TEMPLATE_TYPES {
            registry.register(Box::new(TemplateRootLayout {
                type_name: format!("{{{namespace}}}{name}"),
            }));
        }
    }
    registry
}

pub(crate) fn is_template_owner(document: &Document, owner: NodeId) -> bool {
    document.ast().node(owner).is_some_and(|node| {
        node.kind == NodeKind::Element
            && TEMPLATE_TYPES.contains(&node.local_name())
            && node
                .namespace_uri
                .as_deref()
                .is_some_and(|uri| TEMPLATE_NAMESPACES.contains(&uri))
    })
}

fn source_parent(document: &Document, node: NodeId) -> Option<NodeId> {
    let mut parent = document.ast().node(node)?.parent;
    while let Some(id) = parent {
        let node = document.ast().node(id)?;
        if node.kind != NodeKind::PropertyElement {
            return Some(id);
        }
        parent = node.parent;
    }
    None
}

/// Geometry-only bridge. The returned native presenter is deliberately never entered
/// in the source identity map and can never become an authoring or selection target.
pub(crate) fn native_layout_parent(
    session: &DesignerSession,
    owner: NodeId,
    child: Option<NodeId>,
) -> Option<Ref<Visual>> {
    if !is_template_owner(session.document(), owner) {
        return None;
    }
    let resolve = |child| {
        if source_parent(session.document(), child) != Some(owner) {
            return None;
        }
        session.preview.visual(child)?.visual_parent()
    };
    if let Some(child) = child {
        return resolve(child);
    }
    for id in session.document().ast().element_children(owner) {
        let node = session.document().ast().node(id)?;
        if node.kind == NodeKind::PropertyElement {
            for id in session.document().ast().element_children(id) {
                if let Some(parent) = resolve(id) {
                    return Some(parent);
                }
            }
        } else if let Some(parent) = resolve(id) {
            return Some(parent);
        }
    }
    None
}

struct TemplateRootLayout {
    type_name: String,
}
impl LayoutAdapter for TemplateRootLayout {
    fn panel_type(&self) -> &str {
        &self.type_name
    }
    fn capabilities(&self) -> LayoutCapabilities {
        LayoutCapabilities {
            move_items: true,
            resize_items: true,
            reorder: false,
            drop_items: false,
            generated_items: false,
        }
    }
    fn panel_properties(&self) -> &[&str] {
        &[]
    }
    fn plan(
        &self,
        context: &LayoutContext<'_>,
        action: &LayoutAction,
    ) -> ferroui_designer_tools::Result<Vec<Edit>> {
        let node = match action {
            LayoutAction::Move { node, .. } | LayoutAction::Resize { node, .. } => *node,
            _ => {
                return Err(ToolError(
                    "Edit the visual layout inside this template; its single root is not a panel collection".into(),
                ));
            }
        };
        let owner = source_parent(context.document, node)
            .filter(|owner| is_template_owner(context.document, *owner))
            .ok_or_else(|| ToolError("Select this template's visual root".into()))?;
        context.bounds(owner)?;
        context.bounds(node)?;
        // Reuse the established content-slot margin/size equations, including alignment
        // slack, resolved margins and expression protection. The registry validates the
        // combined transaction after this provider returns its source edits.
        LayoutRegistry::built_in()
            .adapter_exact("Border")
            .expect("built-in content layout")
            .plan(context, action)
    }
    fn design_helpers(
        &self,
        context: &LayoutContext<'_>,
        request: &LayoutHelperRequest,
    ) -> ferroui_designer_tools::Result<LayoutHelperScene> {
        let mut scene = LayoutHelperScene::generic(context, request)?;
        scene.title = "Template layout root".into();
        for node in &request.selected_nodes {
            if source_parent(context.document, *node) == Some(request.panel)
                && context.bounds.contains_key(node)
            {
                scene.append_margin_helpers(context, *node)?;
            }
        }
        if let Some(pointer) = request.pointer {
            scene.placement =
                self.placement(context, request.panel, pointer, &request.dragged_nodes)?;
        }
        Ok(scene)
    }
    fn placement(
        &self,
        context: &LayoutContext<'_>,
        panel: NodeId,
        _position: Point,
        _dragged: &[NodeId],
    ) -> ferroui_designer_tools::Result<Option<LayoutPlacement>> {
        LayoutPlacement::rejected(
            context,
            panel,
            "Drop into a panel or content holder inside the template",
        )
        .map(Some)
    }
}

#[cfg(test)]
#[path = "definition_manipulation_tests.rs"]
mod tests;
