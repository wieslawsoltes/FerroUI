//! Shared drop planning for Studio, embedded designers and automation.
use crate::{DesignerSession, LayoutEditPreview, OutlinePlacement};
use ferroui_base::Point;
use ferroui_designer_core::{Edit, NodeId};
use ferroui_designer_tools::Rect;

pub enum PlacementOrigin<'a> {
    Asset(&'a str),
    Node { node: NodeId, grab: Option<Point> },
    Nodes { nodes: &'a [NodeId], anchor: NodeId, grab: Option<Point> },
}

pub enum PlacementTarget {
    Outline {
        node: NodeId,
        position: OutlinePlacement,
    },
    Canvas(Point),
}

pub struct PlacementPlan {
    pub edits: Vec<Edit>,
    pub bounds: Option<Rect>,
    pub description: String,
}

impl DesignerSession {
    pub fn plan_resize_handle(&self, node: NodeId, handle: u8, dx: f64, dy: f64) -> Result<Vec<Edit>, String> {
        self.validate_resize_handle(handle, dx, dy)?;
        let host = self.preview.host.clone().upcast();
        let before = self.preview.bounds(node, &host).ok_or("The selected control has no measured bounds")?;
        let after = crate::affine_manipulation::resize(before, dx, dy, handle);
        crate::workspace_tools::resize_edits(self, node, before, after).map(|(edits, _)| edits)
    }

    pub(crate) fn validate_resize_handle(&self, handle: u8, dx: f64, dy: f64) -> Result<(), String> {
        if handle > 7 || ![dx, dy].iter().all(|value| value.is_finite() && value.abs() <= 6144.0) || !self.can_manipulate() {
            return Err("Resize a current preview control with a valid handle and displacement".into());
        }
        Ok(())
    }

    /// Resolve a gesture against the current native layout and return a source transaction.
    pub fn plan_placement(
        &self,
        origin: PlacementOrigin<'_>,
        target: PlacementTarget,
    ) -> Result<PlacementPlan, String> {
        match target {
            PlacementTarget::Outline { node, position } => {
                let (edits, description) = match origin {
                    PlacementOrigin::Asset(source) => (
                        self.plan_outline_insert(source, node, position)?,
                        "Insert control",
                    ),
                    PlacementOrigin::Node { node: moving, .. } => (
                        self.plan_outline_move(moving, node, position)?,
                        "Move control",
                    ),
                    PlacementOrigin::Nodes { nodes, .. } => (
                        self.plan_outline_move_many(nodes, node, position)?,
                        "Move selected controls",
                    ),
                };
                Ok(PlacementPlan {
                    edits,
                    bounds: None,
                    description: description.into(),
                })
            }
            PlacementTarget::Canvas(point) => match origin {
                PlacementOrigin::Asset(source) => {
                    self.measured_placement(&self.preview_layout_insert_nearby(source, point)?)
                }
                PlacementOrigin::Node { node, grab } => {
                    let artboard = self.preview.host.clone().upcast();
                    let bounds = self
                        .preview
                        .bounds(node, &artboard)
                        .ok_or("The control has no measured bounds")?;
                    let grab = grab.unwrap_or_else(|| Point::new(bounds.x, bounds.y));
                    let delta = Point::new(point.x - grab.x, point.y - grab.y);
                    if let Some(preview) = self.preview_layout_move(&[node], point, delta)? {
                        return self.measured_placement(&preview);
                    }
                    let edits = self.plan_artboard_translation(node, point, delta)?;
                    let landing = Rect::new(
                        bounds.x + delta.x,
                        bounds.y + delta.y,
                        bounds.width,
                        bounds.height,
                    );
                    Ok(PlacementPlan {
                        edits,
                        bounds: Some(landing),
                        description: "Move control".into(),
                    })
                }
                PlacementOrigin::Nodes { nodes, anchor, grab } => {
                    if !nodes.contains(&anchor) { return Err("Start the drag on a selected control".into()); }
                    let artboard = self.preview.host.clone().upcast();
                    let bounds = self.preview.bounds(anchor, &artboard).ok_or("The dragged selection has no measured bounds")?;
                    let grab = grab.unwrap_or_else(|| Point::new(bounds.x, bounds.y));
                    let delta = Point::new(point.x - grab.x, point.y - grab.y);
                    if let Some(preview) = self.preview_layout_move(nodes, point, delta)? {
                        return self.measured_placement(&preview);
                    }
                    let target = crate::workspace_layout_helpers::layout_target(self, point, nodes)
                        .filter(|target| self.is_free_canvas(*target)).ok_or("Drop the selection into a supported layout container")?;
                    if !nodes.iter().all(|node| crate::workspace_tools::parent(self, *node).ok() == Some(target)) {
                        return Err("Choose a Canvas that owns the full selection".into());
                    }
                    let edits = self.plan_selection_translation(nodes, ferroui_designer_tools::Point::new(delta.x, delta.y))?;
                    let bounds = self.canvas_selection_bounds(nodes)?;
                    Ok(PlacementPlan { edits,
                        bounds: Some(Rect::new(bounds.x + delta.x, bounds.y + delta.y, bounds.width, bounds.height)),
                        description: "Move selected controls".into() })
                }
            },
        }
    }

    pub(crate) fn canvas_selection_bounds(&self, nodes: &[NodeId]) -> Result<ferroui_base::Rect, String> {
        let artboard = self.preview.host.clone().upcast();
        let mut bounds = nodes.iter().map(|node| self.preview.bounds(*node, &artboard));
        let first = bounds.next().flatten().ok_or("The selection has no measured bounds")?;
        bounds.try_fold(first, |union, rect| rect.map(|rect| union.union(rect)))
            .ok_or_else(|| "The selection has no measured bounds".into())
    }

    fn measured_placement(&self, preview: &LayoutEditPreview) -> Result<PlacementPlan, String> {
        let edits = preview.source_edits(self)?;
        let helpers = &preview.helpers;
        let bounds = helpers
            .scene
            .placement
            .as_ref()
            .map(|placement| helpers.to_artboard.bounds(placement.bounds))
            .transpose()
            .map_err(|error| error.to_string())?;
        Ok(PlacementPlan {
            edits,
            bounds,
            description: preview.description().into(),
        })
    }
}
