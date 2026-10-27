//! Renderer-independent design-time features for panel layout. Coordinates use the same
//! measured space as `LayoutContext`; the host owns transforms, pixels, input and transactions.
use crate::layout::{
    authored_children, built_in_placement, ensure_literal, number, single_content, thickness,
};
use crate::{
    Axis, GridLength, GridTrackChange, LayoutAction, LayoutContext, Point, PropertyRegistry, Rect,
    Result, ToolError, require,
};
use ferroui_designer_core::{Edit, NodeId};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LayoutHelperRequest {
    pub panel: NodeId,
    pub pointer: Option<Point>,
    /// Existing nodes being moved. Placement boundaries still reference the original AST.
    pub dragged_nodes: Vec<NodeId>,
    /// Selected direct children whose resolved margins should be visualized and edited.
    #[serde(default)]
    pub selected_nodes: Vec<NodeId>,
}
impl LayoutHelperRequest {
    pub fn new(panel: NodeId) -> Self {
        Self {
            panel,
            pointer: None,
            dragged_nodes: Vec::new(),
            selected_nodes: Vec::new(),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum LayoutGuideRole {
    Container,
    Content,
    Track,
    Flow,
    Spacing,
    Padding,
    Relation,
    Placement,
    Rejected,
    Measurement,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum LayoutGuideGeometry {
    Line {
        start: Point,
        end: Point,
    },
    Rectangle {
        bounds: Rect,
        filled: bool,
    },
    /// `position` is the center anchor of a screen-sized label, not its top-left corner.
    Label {
        position: Point,
        text: String,
    },
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LayoutGuide {
    pub role: LayoutGuideRole,
    pub geometry: LayoutGuideGeometry,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum LayoutHandleAction {
    PanelProperty {
        panel: NodeId,
        name: String,
    },
    ThicknessSide {
        node: NodeId,
        property: String,
        side: usize,
        values: [f64; 4],
    },
    /// Values are measured extents. Star weights scale proportionally; Auto deliberately becomes px.
    GridTrack {
        panel: NodeId,
        axis: Axis,
        index: usize,
        original: GridLength,
        measured_extent: f64,
    },
    Custom {
        key: String,
    },
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LayoutHandle {
    pub id: String,
    pub label: String,
    pub position: Point,
    pub axis: Axis,
    pub value: f64,
    pub minimum: f64,
    pub maximum: Option<f64>,
    /// +1 for leading/positive-axis handles, -1 for trailing padding handles.
    pub direction: f64,
    pub action: LayoutHandleAction,
    pub disabled_reason: Option<String>,
}
impl LayoutHandle {
    /// Convert a panel-local pointer displacement to a validated, clamped draft value.
    pub fn value_after_delta(&self, delta: Point) -> Result<f64> {
        delta.validate()?;
        let distance = match self.axis {
            Axis::Horizontal => delta.x,
            Axis::Vertical => delta.y,
        };
        let value = self.value + distance * self.direction;
        require(value.is_finite(), "Layout helper value overflow")?;
        Ok(value
            .max(self.minimum)
            .min(self.maximum.unwrap_or(f64::MAX)))
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LayoutPlacement {
    /// Actual Children/Content/Child AST container, not necessarily the visual panel node.
    pub container: NodeId,
    /// Element insertion boundary in the original AST, before removing `dragged_nodes`.
    pub index: usize,
    pub attributes: Vec<(String, String)>,
    pub bounds: Rect,
    pub description: String,
    pub rejection: Option<String>,
}
impl LayoutPlacement {
    pub fn rejected(
        context: &LayoutContext<'_>,
        panel: NodeId,
        reason: impl Into<String>,
    ) -> Result<Self> {
        let reason = reason.into();
        Ok(Self {
            container: context.content_container(panel)?,
            index: 0,
            attributes: Vec::new(),
            bounds: context.bounds(panel)?,
            description: reason.clone(),
            rejection: Some(reason),
        })
    }
    pub fn validate(&self) -> Result<()> {
        self.bounds.validate()
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LayoutHelperScene {
    pub panel: NodeId,
    pub title: String,
    pub primitives: Vec<LayoutGuide>,
    pub handles: Vec<LayoutHandle>,
    pub placement: Option<LayoutPlacement>,
}
impl LayoutHelperScene {
    pub fn generic(context: &LayoutContext<'_>, request: &LayoutHelperRequest) -> Result<Self> {
        let element = context.element(request.panel)?;
        let title = element.xaml_name().map_or_else(
            || element.local_name().to_owned(),
            |name| format!("{name} · {}", element.local_name()),
        );
        let mut scene = Self {
            panel: request.panel,
            title,
            primitives: Vec::new(),
            handles: Vec::new(),
            placement: None,
        };
        scene.rectangle(
            context.bounds(request.panel)?,
            LayoutGuideRole::Container,
            false,
        );
        Ok(scene)
    }
    pub fn validate(&self) -> Result<()> {
        require(
            self.primitives.len() <= 16_384 && self.handles.len() <= 2_048,
            "Layout helper provider exceeded the scene budget",
        )?;
        for guide in &self.primitives {
            match &guide.geometry {
                LayoutGuideGeometry::Line { start, end } => {
                    start.validate()?;
                    end.validate()?;
                }
                LayoutGuideGeometry::Rectangle { bounds, .. } => bounds.validate()?,
                LayoutGuideGeometry::Label { position, .. } => position.validate()?,
            }
        }
        let mut ids = BTreeSet::new();
        for handle in &self.handles {
            handle.position.validate()?;
            require(
                ids.insert(handle.id.as_str()),
                "Layout helper IDs must be unique within a panel",
            )?;
            require(
                handle.value.is_finite()
                    && handle.minimum.is_finite()
                    && handle.direction.is_finite()
                    && handle.direction.abs() == 1.0
                    && handle
                        .maximum
                        .is_none_or(|max| max.is_finite() && max >= handle.minimum),
                "Invalid layout helper range",
            )?;
        }
        if let Some(placement) = &self.placement {
            placement.validate()?;
        }
        Ok(())
    }
    pub fn line(&mut self, start: Point, end: Point, role: LayoutGuideRole) {
        self.primitives.push(LayoutGuide {
            role,
            geometry: LayoutGuideGeometry::Line { start, end },
        });
    }
    pub fn rectangle(&mut self, bounds: Rect, role: LayoutGuideRole, filled: bool) {
        self.primitives.push(LayoutGuide {
            role,
            geometry: LayoutGuideGeometry::Rectangle { bounds, filled },
        });
    }
    pub fn label(&mut self, position: Point, text: impl Into<String>, role: LayoutGuideRole) {
        self.primitives.push(LayoutGuide {
            role,
            geometry: LayoutGuideGeometry::Label {
                position,
                text: text.into(),
            },
        });
    }
}

pub(crate) fn plan_standard_helper(
    context: &LayoutContext<'_>,
    action: &LayoutHandleAction,
    value: f64,
    plan: impl FnOnce(&LayoutAction) -> Result<Vec<Edit>>,
) -> Result<Vec<Edit>> {
    require(value.is_finite(), "Layout helper values must be finite")?;
    match action {
        LayoutHandleAction::PanelProperty { panel, name } => {
            require(value >= 0.0, "Spacing must be nonnegative")?;
            plan(&LayoutAction::PanelProperty {
                panel: *panel,
                name: name.clone(),
                value: value.to_string(),
            })
        }
        LayoutHandleAction::ThicknessSide {
            node,
            property,
            side,
            values,
        } => {
            require(
                *side < 4 && values.iter().all(|v| v.is_finite()),
                "Invalid thickness helper",
            )?;
            require(
                property == "Margin" || property == "Padding",
                "Thickness helpers edit Margin or Padding",
            )?;
            if property == "Padding" {
                require(value >= 0.0, "Padding must be nonnegative")?;
            }
            let mut components = *values;
            components[*side] = value;
            let text = components
                .into_iter()
                .map(|value| value.to_string())
                .collect::<Vec<_>>()
                .join(",");
            PropertyRegistry::built_in().plan(
                context.document,
                &[*node],
                property,
                Some(&text),
                crate::ExpressionPolicy::PreserveExpressions,
            )
        }
        LayoutHandleAction::GridTrack {
            panel,
            axis,
            index,
            original,
            measured_extent,
        } => {
            require(value >= 0.0, "Grid track length must be nonnegative")?;
            let length = match original {
                GridLength::Star(weight) => {
                    require(
                        *measured_extent > 0.0 && *weight > 0.0,
                        "A zero-width star track cannot be scaled; edit its unit explicitly",
                    )?;
                    GridLength::Star(weight * value / measured_extent)
                }
                GridLength::Auto | GridLength::Pixels(_) => GridLength::Pixels(value),
            };
            crate::plan_grid_track_change(
                context.document,
                *panel,
                *axis,
                GridTrackChange::Set {
                    index: *index,
                    length,
                },
            )
        }
        LayoutHandleAction::Custom { .. } => Err(ToolError(
            "Register a helper planner for this custom intent".into(),
        )),
    }
}

pub(crate) fn compact(value: f64) -> String {
    let rounded = (value * 100.0).round() / 100.0;
    if rounded == 0.0 {
        "0".into()
    } else {
        rounded.to_string()
    }
}
fn property_number(context: &LayoutContext<'_>, node: NodeId, name: &str) -> f64 {
    context
        .property(node, name)
        .and_then(|value| number(value).ok())
        .unwrap_or(0.0)
}
fn writable(context: &LayoutContext<'_>, node: NodeId, name: &str) -> Option<String> {
    ensure_literal(context, node, name)
        .err()
        .map(|error| error.to_string())
}
fn scalar_handle(
    scene: &mut LayoutHelperScene,
    context: &LayoutContext<'_>,
    name: &str,
    position: Point,
    axis: Axis,
) {
    if scene.handles.iter().any(|handle| handle.id == name) {
        return;
    }
    scene.handles.push(LayoutHandle {
        id: name.into(),
        label: name.into(),
        position,
        axis,
        value: property_number(context, scene.panel, name),
        minimum: 0.0,
        maximum: None,
        direction: 1.0,
        action: LayoutHandleAction::PanelProperty {
            panel: scene.panel,
            name: name.into(),
        },
        disabled_reason: writable(context, scene.panel, name),
    });
}
fn flow_axis(context: &LayoutContext<'_>, panel: NodeId, name: &str) -> (bool, bool) {
    let horizontal = if name == "FlexPanel" {
        context
            .property(panel, "Direction")
            .unwrap_or("Row")
            .starts_with("Row")
    } else {
        context
            .property(panel, "Orientation")
            .unwrap_or(if name == "WrapPanel" {
                "Horizontal"
            } else {
                "Vertical"
            })
            == "Horizontal"
    };
    let reversed = if name == "FlexPanel" {
        context
            .property(panel, "Direction")
            .is_some_and(|v| v.ends_with("Reverse"))
    } else {
        name == "ReversibleStackPanel"
            && context
                .property(panel, "ReverseOrder")
                .is_some_and(|v| v.eq_ignore_ascii_case("True"))
    };
    (horizontal, reversed)
}

pub(crate) fn built_in_helpers(
    context: &LayoutContext<'_>,
    request: &LayoutHelperRequest,
    name: &str,
    managed: bool,
) -> Result<LayoutHelperScene> {
    let mut scene = LayoutHelperScene::generic(context, request)?;
    let panel = request.panel;
    let bounds = context.bounds(panel)?;
    if managed {
        scene.label(
            bounds.center(),
            "Generated items · edit the item source or template",
            LayoutGuideRole::Rejected,
        );
        if request.pointer.is_some() {
            scene.placement = Some(LayoutPlacement::rejected(
                context,
                panel,
                "This panel owns generated runtime content",
            )?);
        }
        return Ok(scene);
    }
    let children = authored_children(context, panel)?
        .into_iter()
        .filter(|id| !request.dragged_nodes.contains(id))
        .filter(|id| context.is_visible(*id).unwrap_or(false))
        .collect::<Vec<_>>();
    match name {
        "Grid" => grid_helpers(context, &mut scene)?,
        "StackPanel" | "ReversibleStackPanel" | "WrapPanel" | "FlexPanel" => {
            flow_helpers(context, &mut scene, name, &children)?
        }
        "UniformGrid" => uniform_helpers(context, &mut scene, &children)?,
        "DockPanel" => dock_helpers(context, &mut scene, &children)?,
        "RelativePanel" => relative_helpers(context, &mut scene, &children)?,
        "Canvas" => {
            scene.line(
                Point::new(bounds.x, bounds.y),
                Point::new(bounds.x + (bounds.width * 0.2).min(48.0), bounds.y),
                LayoutGuideRole::Flow,
            );
            scene.line(
                Point::new(bounds.x, bounds.y),
                Point::new(bounds.x, bounds.y + (bounds.height * 0.2).min(48.0)),
                LayoutGuideRole::Flow,
            );
            if let Some(pointer) = request.pointer {
                scene.label(
                    Point::new(pointer.x + 24.0, pointer.y - 14.0),
                    format!(
                        "{}, {}",
                        compact(pointer.x - bounds.x),
                        compact(pointer.y - bounds.y)
                    ),
                    LayoutGuideRole::Measurement,
                );
            }
        }
        _ => {}
    }
    // Margin and padding are distinct measured surfaces, with handles only where the property exists.
    if matches!(
        name,
        "Border"
            | "ContentControl"
            | "UserControl"
            | "Window"
            | "ScrollViewer"
            | "Button"
            | "ToggleButton"
    ) {
        thickness_helpers(context, &mut scene, panel, "Padding", bounds)?;
    }
    for node in request.selected_nodes.iter().copied().take(64) {
        if context.parent(node).ok() != Some(panel) {
            continue;
        }
        let Some(child_bounds) = context.bounds.get(&node).copied() else {
            continue;
        };
        margin_helpers(context, &mut scene, node, child_bounds)?;
    }
    if single_content(name) {
        let occupied = !children.is_empty()
            || ["Content", "Child"].iter().any(|name| {
                context
                    .element(panel)
                    .ok()
                    .and_then(|node| node.attribute(name))
                    .is_some_and(|value| !value.is_empty())
            });
        scene.label(
            bounds.center(),
            if occupied {
                "Content occupied · select the inner layout"
            } else {
                "Drop content here"
            },
            if occupied {
                LayoutGuideRole::Content
            } else {
                LayoutGuideRole::Placement
            },
        );
    }
    if let Some(position) = request.pointer {
        let placement = built_in_placement(context, panel, position, name, &request.dragged_nodes)
            .or_else(|reason| LayoutPlacement::rejected(context, panel, reason.to_string()))?;
        let role = if placement.rejection.is_some() {
            LayoutGuideRole::Rejected
        } else {
            LayoutGuideRole::Placement
        };
        scene.rectangle(placement.bounds, role, true);
        scene.label(
            Point::new(placement.bounds.center().x, placement.bounds.y - 12.0),
            placement.description.clone(),
            role,
        );
        scene.placement = Some(placement);
    }
    scene.validate()?;
    Ok(scene)
}

fn thickness_helpers(
    context: &LayoutContext<'_>,
    scene: &mut LayoutHelperScene,
    node: NodeId,
    property: &str,
    bounds: Rect,
) -> Result<()> {
    let Some(value) = context.property(node, property) else {
        return Ok(());
    };
    let Ok(values) = thickness(value) else {
        return Ok(());
    };
    if property == "Padding" && values.iter().any(|v| *v < 0.0) {
        return Ok(());
    }
    let role = if property == "Padding" {
        LayoutGuideRole::Padding
    } else {
        LayoutGuideRole::Spacing
    };
    let [left, top, right, bottom] = values;
    let left = left.max(0.0).min(bounds.width);
    let right = right.max(0.0).min(bounds.width);
    let top = top.max(0.0).min(bounds.height);
    let bottom = bottom.max(0.0).min(bounds.height);
    let bands = [
        Rect::new(bounds.x, bounds.y, left, bounds.height),
        Rect::new(bounds.x, bounds.y, bounds.width, top),
        Rect::new(bounds.right() - right, bounds.y, right, bounds.height),
        Rect::new(bounds.x, bounds.bottom() - bottom, bounds.width, bottom),
    ];
    for (side, band) in bands.into_iter().enumerate() {
        if values[side] > 0.0 {
            scene.rectangle(band, role, true);
            scene.label(band.center(), compact(values[side]), role);
        }
        let (position, axis, direction) = match side {
            0 => (
                Point::new(bounds.x + left, bounds.center().y),
                Axis::Horizontal,
                1.0,
            ),
            1 => (
                Point::new(bounds.center().x, bounds.y + top),
                Axis::Vertical,
                1.0,
            ),
            2 => (
                Point::new(bounds.right() - right, bounds.center().y),
                Axis::Horizontal,
                -1.0,
            ),
            _ => (
                Point::new(bounds.center().x, bounds.bottom() - bottom),
                Axis::Vertical,
                -1.0,
            ),
        };
        scene.handles.push(LayoutHandle {
            id: format!("{property}.{side}"),
            label: format!("{property} {}", ["left", "top", "right", "bottom"][side]),
            position,
            axis,
            value: values[side],
            minimum: if property == "Padding" {
                0.0
            } else {
                -f64::MAX
            },
            maximum: None,
            direction,
            action: LayoutHandleAction::ThicknessSide {
                node,
                property: property.into(),
                side,
                values,
            },
            disabled_reason: writable(context, node, property),
        });
    }
    let content = Rect::new(
        bounds.x + left,
        bounds.y + top,
        (bounds.width - left - right).max(0.0),
        (bounds.height - top - bottom).max(0.0),
    );
    scene.rectangle(content, LayoutGuideRole::Content, false);
    Ok(())
}

fn margin_helpers(
    context: &LayoutContext<'_>,
    scene: &mut LayoutHelperScene,
    node: NodeId,
    bounds: Rect,
) -> Result<()> {
    let layout = context.resolved_layout(node)?;
    let values = layout.margin;
    let outer = layout.slot.unwrap_or(Rect::new(
        bounds.x - values[0],
        bounds.y - values[1],
        (bounds.width + values[0] + values[2]).max(0.0),
        (bounds.height + values[1] + values[3]).max(0.0),
    ));
    scene.rectangle(outer, LayoutGuideRole::Spacing, false);
    for (side, value) in values.into_iter().enumerate() {
        let (start, end, axis, direction) = match side {
            0 => (
                Point::new(bounds.x - value, bounds.center().y),
                Point::new(bounds.x, bounds.center().y),
                Axis::Horizontal,
                1.0,
            ),
            1 => (
                Point::new(bounds.center().x, bounds.y - value),
                Point::new(bounds.center().x, bounds.y),
                Axis::Vertical,
                1.0,
            ),
            2 => (
                Point::new(bounds.right(), bounds.center().y),
                Point::new(bounds.right() + value, bounds.center().y),
                Axis::Horizontal,
                1.0,
            ),
            _ => (
                Point::new(bounds.center().x, bounds.bottom()),
                Point::new(bounds.center().x, bounds.bottom() + value),
                Axis::Vertical,
                1.0,
            ),
        };
        scene.line(start, end, LayoutGuideRole::Spacing);
        let midpoint = Point::new((start.x + end.x) * 0.5, (start.y + end.y) * 0.5);
        if value != 0.0 {
            scene.label(midpoint, compact(value), LayoutGuideRole::Spacing);
        }
        scene.handles.push(LayoutHandle {
            id: format!("Margin.{node}.{side}"),
            label: format!("Margin {}", ["left", "top", "right", "bottom"][side]),
            position: midpoint,
            axis,
            value,
            minimum: -f64::MAX,
            maximum: None,
            direction,
            action: LayoutHandleAction::ThicknessSide {
                node,
                property: "Margin".into(),
                side,
                values,
            },
            disabled_reason: writable(context, node, "Margin"),
        });
    }
    Ok(())
}

fn grid_helpers(context: &LayoutContext<'_>, scene: &mut LayoutHelperScene) -> Result<()> {
    let bounds = context.bounds(scene.panel)?;
    let lines = context.grid_boundaries(scene.panel)?;
    for (axis, positions, spacing, caption) in [
        (Axis::Horizontal, &lines.columns, "ColumnSpacing", "C"),
        (Axis::Vertical, &lines.rows, "RowSpacing", "R"),
    ] {
        let definitions = crate::grid_track_values(context.document, scene.panel, axis);
        let gap = property_number(context, scene.panel, spacing);
        for (index, pair) in positions.windows(2).enumerate().take(512) {
            let extent = (pair[1]
                - pair[0]
                - if index + 2 < positions.len() {
                    gap
                } else {
                    0.0
                })
            .max(0.0);
            let (start, end, label, handle_position) = match axis {
                Axis::Horizontal => (
                    Point::new(bounds.x + pair[1], bounds.y),
                    Point::new(bounds.x + pair[1], bounds.bottom()),
                    Point::new(bounds.x + pair[0] + extent * 0.5, bounds.y + 11.0),
                    Point::new(bounds.x + pair[0] + extent, bounds.y + 24.0),
                ),
                Axis::Vertical => (
                    Point::new(bounds.x, bounds.y + pair[1]),
                    Point::new(bounds.right(), bounds.y + pair[1]),
                    Point::new(bounds.x + 24.0, bounds.y + pair[0] + extent * 0.5),
                    Point::new(bounds.x + 12.0, bounds.y + pair[0] + extent),
                ),
            };
            scene.line(start, end, LayoutGuideRole::Track);
            let lexical = definitions.as_ref().ok().and_then(|all| all.get(index));
            let original = lexical.and_then(|value| GridLength::parse(value).ok());
            scene.label(
                label,
                format!(
                    "{caption}{} · {}",
                    index + 1,
                    lexical.map_or_else(|| compact(extent), String::clone)
                ),
                LayoutGuideRole::Track,
            );
            if index < 128 {
                let mut disabled = definitions.as_ref().err().map(ToString::to_string);
                if definitions
                    .as_ref()
                    .is_ok_and(|all| all.len() != positions.len() - 1)
                {
                    disabled=Some("Grid definitions are supplied by a style or runtime factory; edit that source".into());
                }
                if original.is_none() {
                    disabled = Some("This Grid track is bound or object-valued".into());
                }
                if matches!(original,Some(GridLength::Star(weight)) if weight==0.0 || extent==0.0) {
                    disabled = Some("A zero-width star track needs an explicit unit edit".into());
                }
                scene.handles.push(LayoutHandle {
                    id: format!("{caption}{index}"),
                    label: format!(
                        "{} {}",
                        if axis == Axis::Horizontal {
                            "Column"
                        } else {
                            "Row"
                        },
                        index + 1
                    ),
                    position: handle_position,
                    axis,
                    value: extent,
                    minimum: 0.0,
                    maximum: None,
                    direction: 1.0,
                    action: LayoutHandleAction::GridTrack {
                        panel: scene.panel,
                        axis,
                        index,
                        original: original.unwrap_or(GridLength::Pixels(extent)),
                        measured_extent: extent,
                    },
                    disabled_reason: disabled,
                });
            }
        }
        if positions.len() > 2 {
            let position = match axis {
                Axis::Horizontal => {
                    Point::new(bounds.x + positions[1] - gap * 0.5, bounds.bottom() - 12.0)
                }
                Axis::Vertical => {
                    Point::new(bounds.right() - 12.0, bounds.y + positions[1] - gap * 0.5)
                }
            };
            scalar_handle(scene, context, spacing, position, axis);
        }
    }
    Ok(())
}

fn flow_helpers(
    context: &LayoutContext<'_>,
    scene: &mut LayoutHelperScene,
    name: &str,
    children: &[NodeId],
) -> Result<()> {
    let bounds = context.bounds(scene.panel)?;
    let (horizontal, reversed) = flow_axis(context, scene.panel, name);
    let axis = if horizontal {
        Axis::Horizontal
    } else {
        Axis::Vertical
    };
    let mut ordered = children.to_vec();
    if name == "FlexPanel" {
        ordered.sort_by_key(|id| {
            context
                .property(*id, "Flex.Order")
                .and_then(|s| s.parse::<i64>().ok())
                .unwrap_or(0)
        });
    }
    let mut start = Point::new(bounds.x + 8.0, bounds.y + 8.0);
    let mut end = if horizontal {
        Point::new((bounds.right() - 8.0).max(start.x), start.y)
    } else {
        Point::new(start.x, (bounds.bottom() - 8.0).max(start.y))
    };
    if reversed {
        std::mem::swap(&mut start, &mut end);
    }
    scene.line(start, end, LayoutGuideRole::Flow);
    let spacing = match name {
        "WrapPanel" => "ItemSpacing",
        "FlexPanel" => {
            if horizontal {
                "ColumnSpacing"
            } else {
                "RowSpacing"
            }
        }
        _ => "Spacing",
    };
    let cross_spacing = if name == "WrapPanel" {
        "LineSpacing"
    } else if horizontal {
        "RowSpacing"
    } else {
        "ColumnSpacing"
    };
    let mut emitted_gap = false;
    let mut emitted_cross = false;
    for pair in ordered.windows(2).take(512) {
        let (Some(a), Some(b)) = (
            context.bounds.get(&pair[0]).copied(),
            context.bounds.get(&pair[1]).copied(),
        ) else {
            continue;
        };
        a.validate()?;
        b.validate()?;
        let same_line = if horizontal {
            a.y < b.bottom() && b.y < a.bottom()
        } else {
            a.x < b.right() && b.x < a.right()
        };
        let gap = if same_line {
            if horizontal {
                let (left, right) = if a.x <= b.x { (a, b) } else { (b, a) };
                Rect::new(
                    left.right(),
                    a.y.max(b.y),
                    (right.x - left.right()).max(0.0),
                    (a.bottom().min(b.bottom()) - a.y.max(b.y)).max(0.0),
                )
            } else {
                let (top, bottom) = if a.y <= b.y { (a, b) } else { (b, a) };
                Rect::new(
                    a.x.max(b.x),
                    top.bottom(),
                    (a.right().min(b.right()) - a.x.max(b.x)).max(0.0),
                    (bottom.y - top.bottom()).max(0.0),
                )
            }
        } else if horizontal {
            let (top, bottom) = if a.y <= b.y { (a, b) } else { (b, a) };
            Rect::new(
                bounds.x,
                top.bottom(),
                bounds.width,
                (bottom.y - top.bottom()).max(0.0),
            )
        } else {
            let (left, right) = if a.x <= b.x { (a, b) } else { (b, a) };
            Rect::new(
                left.right(),
                bounds.y,
                (right.x - left.right()).max(0.0),
                bounds.height,
            )
        };
        scene.rectangle(gap, LayoutGuideRole::Spacing, true);
        let distance = if same_line {
            if horizontal { gap.width } else { gap.height }
        } else if horizontal {
            gap.height
        } else {
            gap.width
        };
        scene.label(gap.center(), compact(distance), LayoutGuideRole::Spacing);
        if same_line && !emitted_gap {
            scalar_handle(scene, context, spacing, gap.center(), axis);
            emitted_gap = true;
        } else if !same_line && !emitted_cross && matches!(name, "WrapPanel" | "FlexPanel") {
            scalar_handle(
                scene,
                context,
                cross_spacing,
                gap.center(),
                if horizontal {
                    Axis::Vertical
                } else {
                    Axis::Horizontal
                },
            );
            emitted_cross = true;
        }
    }
    if !emitted_gap {
        scalar_handle(
            scene,
            context,
            spacing,
            Point::new(bounds.center().x, bounds.center().y),
            axis,
        );
    }
    Ok(())
}

fn uniform_dimensions(
    context: &LayoutContext<'_>,
    panel: NodeId,
    count: usize,
) -> Result<(usize, usize, usize)> {
    let mut rows = crate::layout::integer(context.property(panel, "Rows").unwrap_or("0"))?;
    let mut columns = crate::layout::integer(context.property(panel, "Columns").unwrap_or("0"))?;
    let mut first = crate::layout::integer(context.property(panel, "FirstColumn").unwrap_or("0"))?;
    if first >= columns {
        first = 0;
    }
    let count = count
        .max(1)
        .checked_add(first)
        .ok_or_else(|| ToolError("UniformGrid count overflow".into()))?;
    if rows == 0 && columns == 0 {
        columns = (count as f64).sqrt().ceil() as usize;
        rows = columns;
    } else if columns == 0 {
        columns = count.div_ceil(rows);
    } else if rows == 0 {
        rows = count.div_ceil(columns);
    }
    Ok((rows, columns, first))
}
fn uniform_helpers(
    context: &LayoutContext<'_>,
    scene: &mut LayoutHelperScene,
    children: &[NodeId],
) -> Result<()> {
    let bounds = context.bounds(scene.panel)?;
    let (rows, columns, first) = uniform_dimensions(context, scene.panel, children.len())?;
    let gap_x = property_number(context, scene.panel, "ColumnSpacing");
    let gap_y = property_number(context, scene.panel, "RowSpacing");
    require(
        gap_x >= 0.0 && gap_y >= 0.0,
        "UniformGrid spacing must be nonnegative",
    )?;
    let width = ((bounds.width - columns.saturating_sub(1) as f64 * gap_x) / columns.max(1) as f64)
        .max(0.0);
    let height =
        ((bounds.height - rows.saturating_sub(1) as f64 * gap_y) / rows.max(1) as f64).max(0.0);
    for index in 0..rows.saturating_mul(columns).min(512) {
        let row = index / columns.max(1);
        let column = index % columns.max(1);
        let cell = Rect::new(
            bounds.x + column as f64 * (width + gap_x),
            bounds.y + row as f64 * (height + gap_y),
            width,
            height,
        );
        scene.rectangle(cell, LayoutGuideRole::Track, false);
        if index < first {
            scene.label(cell.center(), "Reserved", LayoutGuideRole::Content);
        } else if index < first + children.len() {
            scene.label(
                Point::new(cell.x + 12.0, cell.y + 10.0),
                (index - first + 1).to_string(),
                LayoutGuideRole::Track,
            );
        }
    }
    if columns > 1 {
        scalar_handle(
            scene,
            context,
            "ColumnSpacing",
            Point::new(bounds.x + width + gap_x * 0.5, bounds.center().y),
            Axis::Horizontal,
        );
    }
    if rows > 1 {
        scalar_handle(
            scene,
            context,
            "RowSpacing",
            Point::new(bounds.center().x, bounds.y + height + gap_y * 0.5),
            Axis::Vertical,
        );
    }
    Ok(())
}
fn dock_helpers(
    context: &LayoutContext<'_>,
    scene: &mut LayoutHelperScene,
    children: &[NodeId],
) -> Result<()> {
    for (index, child) in children.iter().enumerate().take(512) {
        let Some(bounds) = context.bounds.get(child).copied() else {
            continue;
        };
        let fill = index + 1 == children.len()
            && context
                .property(scene.panel, "LastChildFill")
                .is_none_or(|v| !v.eq_ignore_ascii_case("False"));
        scene.rectangle(bounds, LayoutGuideRole::Content, false);
        scene.label(
            bounds.center(),
            if fill {
                "Fill".to_owned()
            } else {
                format!(
                    "{} · {}",
                    index + 1,
                    context.property(*child, "DockPanel.Dock").unwrap_or("Left")
                )
            },
            LayoutGuideRole::Track,
        );
    }
    let bounds = context.bounds(scene.panel)?;
    scalar_handle(
        scene,
        context,
        "HorizontalSpacing",
        Point::new(bounds.center().x, bounds.y + 12.0),
        Axis::Horizontal,
    );
    scalar_handle(
        scene,
        context,
        "VerticalSpacing",
        Point::new(bounds.x + 12.0, bounds.center().y),
        Axis::Vertical,
    );
    Ok(())
}
fn relative_helpers(
    context: &LayoutContext<'_>,
    scene: &mut LayoutHelperScene,
    children: &[NodeId],
) -> Result<()> {
    let names = children
        .iter()
        .filter_map(|id| {
            context
                .element(*id)
                .ok()
                .and_then(|node| node.xaml_name().map(|name| (name.to_owned(), *id)))
        })
        .collect::<BTreeMap<_, _>>();
    for child in children.iter().take(512) {
        let Some(bounds) = context.bounds.get(child).copied() else {
            continue;
        };
        for property in [
            "RelativePanel.LeftOf",
            "RelativePanel.RightOf",
            "RelativePanel.Above",
            "RelativePanel.Below",
            "RelativePanel.AlignLeftWith",
            "RelativePanel.AlignRightWith",
            "RelativePanel.AlignTopWith",
            "RelativePanel.AlignBottomWith",
            "RelativePanel.AlignHorizontalCenterWith",
            "RelativePanel.AlignVerticalCenterWith",
        ] {
            if let Some(target) = context
                .property(*child, property)
                .and_then(|name| names.get(name))
                .and_then(|target| context.bounds.get(target))
            {
                scene.line(bounds.center(), target.center(), LayoutGuideRole::Relation);
                scene.label(
                    Point::new(
                        (bounds.center().x + target.center().x) * 0.5,
                        (bounds.center().y + target.center().y) * 0.5,
                    ),
                    property.trim_start_matches("RelativePanel."),
                    LayoutGuideRole::Relation,
                );
            }
        }
    }
    Ok(())
}

#[cfg(test)]
#[path = "layout_helpers_tests.rs"]
mod tests;
