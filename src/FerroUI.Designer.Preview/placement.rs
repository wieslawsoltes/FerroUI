use crate::protocol::{DragOrigin, DropHint, DropPosition, DropTarget, Placement};
use crate::renderer::{Renderer, Stage};
use crate::toolbox;
use ferroui_base::Point;
use ferroui_designer::{raster_asset_fragment, OutlinePlacement, PlacementOrigin, PlacementTarget};
use ferroui_designer_core::Edit;

impl Renderer {
    pub(crate) fn stage(&self, frame: u64) -> Result<&Stage, String> {
        self.stage
            .as_ref()
            .filter(|stage| stage.token == frame)
            .ok_or_else(|| {
                "The rendered layout changed. Wait for the current preview and try again.".into()
            })
    }
}

impl Stage {
    pub(crate) fn plan(&self, placement: &Placement, recording: bool) -> Result<(Vec<Edit>, DropHint), String> {
        let session = self.session.borrow();
        let image;
        let origin = match &placement.origin {
            DragOrigin::Asset { control } => {
                let asset = toolbox::assets().iter().chain(&self.project_assets)
                    .find(|asset| asset.id == *control)
                    .ok_or("Choose a control from the current Toolbox")?;
                PlacementOrigin::Asset(&asset.source)
            }
            DragOrigin::Image { source } => {
                session.validate_raster_asset(source)?;
                image = raster_asset_fragment(source)?;
                PlacementOrigin::Asset(&image)
            }
            DragOrigin::Node { node, grab } => PlacementOrigin::Node {
                node: *node,
                grab: grab.map(|point| Point::new(point.x, point.y)),
            },
            DragOrigin::Nodes { nodes, anchor, grab } => {
                let distinct: std::collections::BTreeSet<_> = nodes.iter().copied().collect();
                if !(2..=128).contains(&nodes.len()) || distinct.len() != nodes.len() || !distinct.contains(anchor) {
                    return Err("Drag two to 128 distinct controls from the current selection".into());
                }
                PlacementOrigin::Nodes {
                    nodes, anchor: *anchor,
                    grab: grab.map(|point| Point::new(point.x, point.y)),
                }
            }
        };
        let target = match placement.target {
            DropTarget::Outline { node, position } => {
                let position = match position {
                    DropPosition::Inside => OutlinePlacement::Inside,
                    DropPosition::Before => OutlinePlacement::Before,
                    DropPosition::After => OutlinePlacement::After,
                };
                PlacementTarget::Outline { node, position }
            }
            DropTarget::Canvas { x, y } => {
                if !(0.0..=f64::from(self.width)).contains(&x)
                    || !(0.0..=f64::from(self.height)).contains(&y)
                {
                    return Err("Drop inside the visible artboard".into());
                }
                PlacementTarget::Canvas(Point::new(x, y))
            }
        };
        let as_layout_point = |point: Point| ferroui_designer_tools::Point::new(point.x, point.y);
        let plan = match (recording, origin, target) {
            (true, PlacementOrigin::Node { node, grab }, PlacementTarget::Canvas(point)) => {
                session.plan_recorded_move(&[node], node, grab.map(as_layout_point), as_layout_point(point))?
            }
            (true, PlacementOrigin::Nodes { nodes, anchor, grab }, PlacementTarget::Canvas(point)) => {
                session.plan_recorded_move(nodes, anchor, grab.map(as_layout_point), as_layout_point(point))?
            }
            (true, _, _) => return Err("Choose a canvas move to record".into()),
            (false, origin, target) => session.plan_placement(origin, target)?,
        };
        if let Some(policy) = &self.placement_policy {
            policy.validate(&placement.origin, session.document(), &plan.edits)?;
        }
        Ok((
            plan.edits,
            DropHint {
                bounds: plan.bounds,
                description: plan.description,
            },
        ))
    }
}
