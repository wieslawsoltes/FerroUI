//! Translate the old parent's authored placement into the new wrapper's slot.
use super::{enclosing_bounds, SelectionWrap, WrapKind};
use crate::layout::integer;
use crate::layout_attributes::ensure_layout_literal;
use crate::{require, LayoutContext, Rect, Result, ToolError};
use ferroui_designer_core::NodeId;

const CANVAS: &[&str] = &["Canvas.Left", "Canvas.Top", "Canvas.Right", "Canvas.Bottom"];
const GRID: &[&str] = &["Grid.Row", "Grid.Column", "Grid.RowSpan", "Grid.ColumnSpan"];
const DOCK: &[&str] = &["DockPanel.Dock"];
const FLEX: &[&str] = &["Flex.Order"];

pub(super) fn retired_properties(
    parent: &str,
    kind: WrapKind,
) -> impl Iterator<Item = &'static str> {
    let old = match parent {
        "Canvas" => CANVAS,
        "Grid" => GRID,
        "DockPanel" => DOCK,
        "FlexPanel" => FLEX,
        _ => &[],
    };
    // Grid coordinates ignored by the original parent would become active.
    let newly_active = if kind == WrapKind::Grid && parent != "Grid" {
        GRID
    } else {
        &[]
    };
    old.iter().chain(newly_active).copied()
}

pub(super) fn place(
    context: &LayoutContext<'_>,
    parent: &str,
    plan: &mut SelectionWrap,
) -> Result<()> {
    match parent {
        "Canvas" => {
            let (bounds, _) = canvas_bounds(context, &plan.children)?;
            plan.set("Canvas.Left", bounds.x);
            plan.set("Canvas.Top", bounds.y);
            if matches!(plan.kind, WrapKind::Canvas | WrapKind::WrapPanel) {
                plan.set("Width", bounds.width);
            }
            if plan.kind == WrapKind::Canvas {
                plan.set("Height", bounds.height);
            }
        }
        "Grid" => {
            for (&name, fallback) in GRID.iter().zip(["0", "0", "1", "1"]) {
                let value = shared_property(context, &plan.children, name, fallback, integer)?;
                if value.to_string() != fallback {
                    plan.set(name, value);
                }
            }
        }
        "DockPanel" => {
            let dock = shared_property(context, &plan.children, DOCK[0], "Left", |side| {
                ["Left", "Top", "Right", "Bottom"]
                    .into_iter()
                    .find(|value| value.eq_ignore_ascii_case(side))
                    .ok_or_else(|| ToolError("Choose a supported DockPanel placement".into()))
            })?;
            if dock != "Left" {
                plan.set(DOCK[0], dock);
            }
        }
        "FlexPanel" => {
            let order = shared_property(context, &plan.children, FLEX[0], "0", signed_integer)?;
            if order != 0 {
                plan.set(FLEX[0], order);
            }
        }
        _ => {}
    }
    Ok(())
}

pub(super) fn signed_integer(value: &str) -> Result<i32> {
    value
        .trim()
        .parse::<i32>()
        .map_err(|_| ToolError("The selected layout order is invalid".into()))
}

pub(super) fn shared_property<T: PartialEq>(
    context: &LayoutContext<'_>,
    children: &[NodeId],
    property: &str,
    fallback: &str,
    parse: impl Fn(&str) -> Result<T>,
) -> Result<T> {
    let mut shared = None;
    for &node in children {
        ensure_layout_literal(context.document, context.element(node)?, property)?;
        let value = parse(context.property(node, property).unwrap_or(fallback))?;
        require(
            shared.as_ref().is_none_or(|previous| previous == &value),
            format!("Select controls that share the same {property} before wrapping"),
        )?;
        shared = Some(value);
    }
    shared.ok_or_else(|| ToolError("Select a control to wrap".into()))
}

pub(super) fn canvas_bounds(
    context: &LayoutContext<'_>,
    children: &[NodeId],
) -> Result<(Rect, Vec<(NodeId, Rect)>)> {
    let slots = children
        .iter()
        .map(|&node| {
            let slot = context.resolved_layout(node)?.slot.ok_or_else(|| {
                ToolError("Wait for the selected Canvas children to be arranged".into())
            })?;
            Ok((node, slot))
        })
        .collect::<Result<Vec<_>>>()?;
    Ok((
        enclosing_bounds(slots.iter().map(|(_, slot)| *slot))?,
        slots,
    ))
}
