//! Plan one new layout container around an authored sibling run.
use crate::layout::single_content;
use crate::layout_attributes::{ensure_layout_literal, framework_namespace, layout_attribute};
use crate::{plan_layout_attributes, require, LayoutContext, Rect, Result, ToolError};
use ferroui_designer_core::{Edit, NodeId, NodeKind};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[path = "layout_wrap_slots.rs"]
mod slots;
#[path = "layout_wrap_source.rs"]
mod source;
#[cfg(test)]
#[path = "layout_wrap_tests.rs"]
mod tests;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WrapKind {
    VerticalStack,
    HorizontalStack,
    WrapPanel,
    Grid,
    Canvas,
    Border,
}

impl WrapKind {
    pub const ALL: [Self; 6] = [
        Self::VerticalStack,
        Self::HorizontalStack,
        Self::WrapPanel,
        Self::Grid,
        Self::Canvas,
        Self::Border,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::VerticalStack => "Vertical StackPanel",
            Self::HorizontalStack => "Horizontal StackPanel",
            Self::WrapPanel => "WrapPanel",
            Self::Grid => "Grid",
            Self::Canvas => "Canvas (keep positions)",
            Self::Border => "Border",
        }
    }

    pub fn detail(self) -> &'static str {
        match self {
            Self::VerticalStack => "Arrange children from top to bottom",
            Self::HorizontalStack => "Arrange children from left to right",
            Self::WrapPanel => "Flow children onto new lines",
            Self::Grid => "Start with a shared cell that can be divided into tracks",
            Self::Canvas => "Keep the measured layout positions inside the new Canvas",
            Self::Border => "Add a border and padding around one control",
        }
    }

    pub fn control(self) -> &'static str {
        match self {
            Self::VerticalStack | Self::HorizontalStack => "StackPanel",
            Self::WrapPanel => "WrapPanel",
            Self::Grid => "Grid",
            Self::Canvas => "Canvas",
            Self::Border => "Border",
        }
    }
}

/// A checked placement and property plan in the original document coordinates.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SelectionWrap {
    pub kind: WrapKind,
    pub parent: NodeId,
    pub container: NodeId,
    pub children: Vec<NodeId>,
    pub wrapper_properties: Vec<(String, String)>,
    pub guarded_properties: Vec<&'static str>,
    pub(crate) edits: Vec<Edit>,
}

impl SelectionWrap {
    pub fn child_edits(&self) -> &[Edit] {
        &self.edits
    }
}

pub fn plan_selection_wrap(
    context: &LayoutContext<'_>,
    parent: NodeId,
    selection: &[NodeId],
    kind: WrapKind,
) -> Result<SelectionWrap> {
    require(
        context.document.is_valid(),
        "Repair the source before wrapping controls",
    )?;
    require(
        !selection.is_empty(),
        "Select one or more authored controls to wrap",
    )?;
    let owner = context.element(parent)?;
    let name = owner.local_name();
    require(
        framework_namespace(owner.namespace_uri.as_deref())
            && (matches!(
                name,
                "Canvas"
                    | "Grid"
                    | "StackPanel"
                    | "ReversibleStackPanel"
                    | "WrapPanel"
                    | "UniformGrid"
                    | "FlexPanel"
                    | "DockPanel"
                    | "Panel"
            ) || single_content(name)),
        "This container has no supported wrapping rule",
    )?;
    let container = context.content_container(parent)?;
    let children = sibling_run(context, container, selection)?;
    require(
        kind != WrapKind::Border || children.len() == 1,
        "A Border accepts one child",
    )?;
    require(
        kind != WrapKind::Canvas || name == "Canvas",
        "Keeping positions requires a Canvas parent",
    )?;
    let mut plan = SelectionWrap {
        kind,
        parent,
        container,
        children,
        wrapper_properties: Vec::new(),
        guarded_properties: slots::retired_properties(name, kind).collect(),
        edits: Vec::new(),
    };
    match kind {
        WrapKind::HorizontalStack => plan.set("Orientation", "Horizontal"),
        WrapKind::VerticalStack => plan.set("Orientation", "Vertical"),
        _ => {}
    }
    slots::place(context, name, &mut plan)?;
    if matches!(name, "Canvas" | "Grid" | "Panel") {
        let value = slots::shared_property(
            context,
            &plan.children,
            "ZIndex",
            "0",
            slots::signed_integer,
        )?;
        if value != 0 {
            plan.set("ZIndex", value);
        }
    }
    for &node in &plan.children {
        require(
            context.is_visible(node)?,
            "Show all selected controls before wrapping them",
        )?;
        for property in slots::retired_properties(name, kind) {
            let target = context.element(node)?;
            ensure_layout_literal(context.document, target, property)?;
            if let Some(attribute) = layout_attribute(context.document, target, property)? {
                let inner_canvas_anchor =
                    kind == WrapKind::Canvas && matches!(property, "Canvas.Left" | "Canvas.Top");
                if !inner_canvas_anchor {
                    plan.edits.push(Edit::RemoveAttribute {
                        node,
                        name: attribute.name.clone(),
                    });
                }
            }
        }
    }
    if kind == WrapKind::Canvas {
        let (bounds, child_slots) = slots::canvas_bounds(context, &plan.children)?;
        for (node, rect) in child_slots {
            plan.edits.extend(plan_layout_attributes(
                context.document,
                node,
                &[
                    ("Canvas.Left".into(), (rect.x - bounds.x).to_string()),
                    ("Canvas.Top".into(), (rect.y - bounds.y).to_string()),
                ],
            )?);
        }
    }
    if matches!(name, "Canvas" | "Grid" | "Panel") {
        plan.guarded_properties.push("ZIndex");
    }
    Ok(plan)
}

impl SelectionWrap {
    fn set(&mut self, name: &str, value: impl ToString) {
        self.wrapper_properties
            .push((name.into(), value.to_string()));
    }
}

fn sibling_run(
    context: &LayoutContext<'_>,
    container: NodeId,
    selection: &[NodeId],
) -> Result<Vec<NodeId>> {
    let selected = selection.iter().copied().collect::<BTreeSet<_>>();
    require(
        selected.len() == selection.len(),
        "Select distinct authored siblings",
    )?;
    let ast = context.document.ast();
    let siblings = ast.element_children(container);
    let Some(first) = siblings.iter().position(|id| selected.contains(id)) else {
        return Err(ToolError(
            "Select children in the same authored layout slot".into(),
        ));
    };
    let run = siblings
        .get(first..first + selected.len())
        .ok_or_else(|| ToolError("Select consecutive siblings in the same layout slot".into()))?;
    require(
        run.iter().all(|id| {
            selected.contains(id)
                && ast
                    .node(*id)
                    .is_some_and(|node| node.kind == NodeKind::Element)
        }),
        "Select consecutive controls in the same authored layout slot",
    )?;
    let owner = context.element(container)?;
    require(
        !owner
            .children
            .iter()
            .filter_map(|id| ast.node(*id))
            .any(|node| {
                matches!(node.kind, NodeKind::Text | NodeKind::CData)
                    && node
                        .text
                        .as_ref()
                        .is_some_and(|text| !text.trim().is_empty())
            }),
        "This slot mixes controls with authored text. Wrap it in source.",
    )?;
    Ok(run.to_vec())
}

fn enclosing_bounds(values: impl Iterator<Item = Rect>) -> Result<Rect> {
    let bounds = values
        .reduce(|a, b| {
            let left = a.x.min(b.x);
            let top = a.y.min(b.y);
            Rect::new(
                left,
                top,
                a.right().max(b.right()) - left,
                a.bottom().max(b.bottom()) - top,
            )
        })
        .ok_or_else(|| ToolError("The selected bounds are unavailable".into()))?;
    bounds.validate()?;
    Ok(bounds)
}
