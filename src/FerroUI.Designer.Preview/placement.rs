use crate::protocol::{DragOrigin, DropHint, DropPosition, DropTarget, Placement};
use crate::renderer::{Renderer, Stage};
use crate::toolbox;
use ferroui_base::Point;
use ferroui_designer::{OutlinePlacement, PlacementOrigin, PlacementTarget};
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
    pub(crate) fn plan(&self, placement: &Placement) -> Result<(Vec<Edit>, DropHint), String> {
        let origin = match &placement.origin {
            DragOrigin::Asset { control } => {
                PlacementOrigin::Asset(&toolbox::find(control)?.source)
            }
            DragOrigin::Node { node, grab } => PlacementOrigin::Node {
                node: *node,
                grab: grab.map(|point| Point::new(point.x, point.y)),
            },
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
        let plan = self.session.borrow().plan_placement(origin, target)?;
        Ok((
            plan.edits,
            DropHint {
                bounds: plan.bounds,
                description: plan.description,
            },
        ))
    }
}
