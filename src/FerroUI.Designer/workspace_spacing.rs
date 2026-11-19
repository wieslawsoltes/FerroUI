//! The spacing subset of native layout helpers presented by external editors.
use crate::workspace_tools::parent;
use crate::{DesignerSession, LayoutEditPreview, LayoutHelperSnapshot};
use ferroui_designer_core::NodeId;
use ferroui_designer_tools::{
    Affine, Axis, LayoutGuideGeometry, LayoutGuideRole, LayoutHandleAction, Point,
};
use serde::{Deserialize, Serialize};

pub const SPACING_PROPERTIES: &[&str] = &[
    "Spacing",
    "ItemSpacing",
    "LineSpacing",
    "RowSpacing",
    "ColumnSpacing",
    "Padding",
    "Margin",
];

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Handle {
    id: String,
    owner: NodeId,
    owner_label: String,
    property: String,
    label: String,
    position: Point,
    axis: Point,
    value: f64,
    minimum: f64,
    maximum: Option<f64>,
    disabled_reason: Option<String>,
}

#[derive(Serialize)]
struct Band {
    points: Vec<Point>,
    filled: bool,
    padding: bool,
}

struct MeasuredHandle {
    view: Handle,
    snapshot: usize,
    index: usize,
}

pub(crate) struct SpacingModel {
    pub(crate) model: u64,
    snapshots: Vec<LayoutHelperSnapshot>,
    handles: Vec<MeasuredHandle>,
    bands: Vec<Band>,
}

#[derive(Deserialize)]
#[serde(untagged, deny_unknown_fields)]
pub(crate) enum Adjustment {
    Delta { delta: Point },
    Value { value: f64 },
}

impl SpacingModel {
    pub(crate) fn is_current(&self, session: &DesignerSession) -> bool {
        !self.snapshots.is_empty()
            && self
                .snapshots
                .iter()
                .all(|snapshot| snapshot.is_current(session))
    }

    pub(crate) fn capture(session: &DesignerSession, model: u64) -> Self {
        let mut result = Self {
            model,
            snapshots: Vec::new(),
            handles: Vec::new(),
            bands: Vec::new(),
        };
        let selection = session.selection();
        let [first, ..] = selection else {
            return result;
        };
        let selected_panel = match selection {
            [node] => session.layout_helper_snapshot(*node, None, &[]).ok(),
            _ => None,
        };
        if let Some(snapshot) = selected_panel {
            result.add(session, snapshot, true);
        }
        let parent_panel = parent(session, *first).ok().filter(|&panel| {
            selection
                .iter()
                .all(|&node| parent(session, node).ok() == Some(panel))
        });
        if let Some(snapshot) =
            parent_panel.and_then(|panel| session.layout_helper_snapshot(panel, None, &[]).ok())
        {
            result.add(session, snapshot, false);
        }
        result
    }

    fn add(&mut self, session: &DesignerSession, snapshot: LayoutHelperSnapshot, own: bool) {
        let snapshot_index = self.snapshots.len();
        let previous = self.handles.len();
        for (index, handle) in snapshot.scene.handles.iter().enumerate() {
            let (owner, property) = match &handle.action {
                LayoutHandleAction::PanelProperty { panel, name }
                    if SPACING_PROPERTIES.contains(&name.as_str()) =>
                {
                    (*panel, name)
                }
                LayoutHandleAction::ThicknessSide { node, property, .. }
                    if property == "Margin" || own && property == "Padding" =>
                {
                    (*node, property)
                }
                _ => continue,
            };
            let Ok(position) = snapshot.to_artboard.transform(handle.position) else {
                continue;
            };
            let local_axis = match handle.axis {
                Axis::Horizontal => Point::new(1.0, 0.0),
                Axis::Vertical => Point::new(0.0, 1.0),
            };
            let Ok(axis) = snapshot.handle_delta_to_artboard(index, local_axis) else {
                continue;
            };
            let Some(node) = session.document().ast().node(owner) else {
                continue;
            };
            let owner_label = node
                .xaml_name()
                .unwrap_or_else(|| node.local_name())
                .to_owned();
            self.handles.push(MeasuredHandle {
                view: Handle {
                    id: format!("{}:{}", snapshot.scene.panel, handle.id),
                    owner,
                    owner_label,
                    property: property.clone(),
                    label: handle.label.clone(),
                    position,
                    axis,
                    value: handle.value,
                    minimum: handle.minimum,
                    maximum: handle.maximum,
                    disabled_reason: handle.disabled_reason.clone(),
                },
                snapshot: snapshot_index,
                index,
            });
        }
        if self.handles.len() > previous {
            self.bands.extend(bands(&snapshot, own));
            self.snapshots.push(snapshot);
        }
    }

    pub(crate) fn catalog(&self, session: &DesignerSession) -> serde_json::Value {
        let handles = self
            .handles
            .iter()
            .map(|measure| &measure.view)
            .collect::<Vec<_>>();
        serde_json::json!({
            "model": self.model, "revision": session.document().revision(),
            "handles": handles, "bands": self.bands,
        })
    }

    pub(crate) fn plan(
        &self,
        session: &DesignerSession,
        id: &str,
        adjustment: Adjustment,
    ) -> Result<(Handle, LayoutEditPreview), String> {
        let entry = self
            .handles
            .iter()
            .find(|entry| entry.view.id == id)
            .ok_or("The spacing handle is no longer available")?;
        let snapshot = &self.snapshots[entry.snapshot];
        let preview = match adjustment {
            Adjustment::Delta { delta } => session.preview_layout_handle(
                snapshot,
                entry.index,
                ferroui_base::Point::new(delta.x, delta.y),
            ),
            Adjustment::Value { value } => {
                session.preview_layout_handle_value(snapshot, entry.index, value)
            }
        }?;
        if let Some(reason) = preview.rejection() {
            return Err(reason.to_owned());
        }
        let mut view = entry.view.clone();
        let moved = &preview.helpers.scene.handles[entry.index];
        view.value = moved.value;
        view.position = snapshot
            .to_artboard
            .transform(moved.position)
            .map_err(|error| error.to_string())?;
        Ok((view, preview))
    }
}

fn bands(snapshot: &LayoutHelperSnapshot, own: bool) -> impl Iterator<Item = Band> + '_ {
    snapshot.scene.primitives.iter().filter_map(move |guide| {
        let padding = guide.role == LayoutGuideRole::Padding;
        if guide.role != LayoutGuideRole::Spacing && !(padding && own) {
            return None;
        }
        let (corners, filled) = match guide.geometry {
            LayoutGuideGeometry::Line { start, end } => (vec![start, end], false),
            LayoutGuideGeometry::Rectangle { bounds, filled } => (
                vec![
                    Point::new(bounds.x, bounds.y),
                    Point::new(bounds.right(), bounds.y),
                    Point::new(bounds.right(), bounds.bottom()),
                    Point::new(bounds.x, bounds.bottom()),
                ],
                filled,
            ),
            LayoutGuideGeometry::Label { .. } => return None,
        };
        Some(Band {
            points: project(corners, snapshot.to_artboard)?,
            filled,
            padding,
        })
    })
}

fn project(corners: Vec<Point>, transform: Affine) -> Option<Vec<Point>> {
    corners
        .into_iter()
        .map(|corner| transform.transform(corner).ok())
        .collect()
}
