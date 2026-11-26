use crate::protocol::{DragOrigin, DropTarget, Placement};
use crate::renderer::Stage;
use ferroui_base::Point;
use ferroui_designer::{CanvasSnapAdjustment, CanvasSnapQuery, PlacementOrigin};
use ferroui_designer_core::NodeId;
use ferroui_designer_tools::{CanvasGuideSettings, SnapIndex, CANVAS_MIN_ZOOM};
use serde::Deserialize;

#[derive(Deserialize)]
pub(crate) struct Request {
    pub(crate) frame: u64,
    zoom: f64,
    settings: CanvasGuideSettings,
    gesture: Gesture,
}

#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
enum Gesture {
    Place {
        placement: Placement,
        recording: bool,
    },
    Resize {
        node: NodeId,
        handle: u8,
        dx: f64,
        dy: f64,
    },
}

pub(crate) struct PreparedSnap {
    frame: u64,
    excluded: Vec<NodeId>,
    settings: CanvasGuideSettings,
    index: SnapIndex,
}

impl Request {
    pub(crate) fn resolve(&self, stage: &Stage) -> Result<CanvasSnapAdjustment, String> {
        if !(CANVAS_MIN_ZOOM..=8.0).contains(&self.zoom) {
            return Err("Choose a canvas zoom between 1% and 800%".into());
        }
        self.settings.validate()?;
        if !self.settings.snapping().enabled() {
            return Ok(CanvasSnapAdjustment::default());
        }
        let mut excluded = match &self.gesture {
            Gesture::Place { placement, .. } => match &placement.origin {
                DragOrigin::Node { node, .. } => vec![*node],
                DragOrigin::Nodes { nodes, anchor, .. } => {
                    if !(2..=128).contains(&nodes.len()) || !nodes.contains(anchor) {
                        return Err("Choose a valid canvas selection and drop anchor".into());
                    }
                    nodes.clone()
                }
                _ => Vec::new(),
            },
            Gesture::Resize { node, .. } => vec![*node],
        };
        excluded.sort_unstable();
        let session = stage.session.borrow();
        if !session.can_manipulate() {
            return Err("Render the current design before snapping its controls".into());
        }
        let mut cache = stage.canvas_snap.borrow_mut();
        if !cache.as_ref().is_some_and(|entry| {
            entry.frame == self.frame
                && entry.excluded == excluded
                && entry.settings == self.settings
        }) {
            let index = session.prepare_canvas_snap(&self.settings, &excluded)?;
            *cache = Some(PreparedSnap {
                frame: self.frame,
                excluded,
                settings: self.settings.clone(),
                index,
            });
        }
        let query = CanvasSnapQuery::new(
            &cache
                .as_ref()
                .expect("The current frame prepared its snap index")
                .index,
            &self.settings,
            self.zoom,
        )?;
        match &self.gesture {
            Gesture::Place {
                placement,
                recording,
            } => {
                let DropTarget::Canvas { x, y } = placement.target else {
                    return Ok(CanvasSnapAdjustment::default());
                };
                if !inside(stage, x, y) {
                    return Err("Drop inside the visible artboard".into());
                }
                let origin = match &placement.origin {
                    DragOrigin::Node { node, grab } => PlacementOrigin::Node {
                        node: *node,
                        grab: grab.map(point),
                    },
                    DragOrigin::Nodes {
                        nodes,
                        anchor,
                        grab,
                    } => PlacementOrigin::Nodes {
                        nodes,
                        anchor: *anchor,
                        grab: grab.map(point),
                    },
                    _ => PlacementOrigin::Asset(""),
                };
                let adjusted =
                    session.snap_canvas_placement(&origin, Point::new(x, y), &query, *recording)?;
                Ok(
                    if inside(stage, x + adjusted.delta.x, y + adjusted.delta.y) {
                        adjusted
                    } else {
                        CanvasSnapAdjustment::default()
                    },
                )
            }
            Gesture::Resize {
                node,
                handle,
                dx,
                dy,
            } => session.snap_canvas_resize(*node, *handle, *dx, *dy, &query),
        }
    }
}

fn inside(stage: &Stage, x: f64, y: f64) -> bool {
    (0.0..=f64::from(stage.width)).contains(&x) && (0.0..=f64::from(stage.height)).contains(&y)
}

fn point(point: ferroui_designer_tools::Point) -> Point {
    Point::new(point.x, point.y)
}
