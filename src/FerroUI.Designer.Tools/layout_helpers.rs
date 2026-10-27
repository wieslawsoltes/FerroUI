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
use std::collections::BTreeSet;

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
            )?;
            crate::plan_layout_attributes(context.document, *node, &[(property.clone(), text)])
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
#[path = "layout_helper_builtins.rs"]
mod builtins;
pub(crate) use builtins::built_in_helpers;

#[cfg(test)]
#[path = "layout_helpers_tests.rs"]
mod tests;
