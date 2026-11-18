//! Shared drop planning for Studio, embedded designers and automation.
use crate::{DesignerSession, LayoutEditPreview, OutlinePlacement};
use ferroui_base::Point;
use ferroui_designer_core::{Edit, NodeId};
use ferroui_designer_tools::Rect;

pub enum PlacementOrigin<'a> {
    Asset(&'a str),
    Node { node: NodeId, grab: Option<Point> },
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
                };
                Ok(PlacementPlan {
                    edits,
                    bounds: None,
                    description: description.into(),
                })
            }
            PlacementTarget::Canvas(point) => match origin {
                PlacementOrigin::Asset(source) => {
                    self.measured_placement(&self.preview_layout_insert(source, point)?)
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
            },
        }
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
