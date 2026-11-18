use crate::protocol::{DragOrigin, DropHint, DropPosition, DropTarget, Placement};
use crate::renderer::{Renderer, Stage};
use crate::toolbox;
use ferroui_base::Point;
use ferroui_designer::{LayoutEditPreview, OutlinePlacement};
use ferroui_designer_core::Edit;
use ferroui_designer_tools::Rect;

impl Renderer {
    pub(crate) fn stage(&self, frame: u64) -> Result<&Stage, String> {
        self.stage.as_ref().filter(|stage| stage.token == frame)
            .ok_or_else(|| "The rendered layout changed. Wait for the current preview and try again.".into())
    }
}

impl Stage {
    pub(crate) fn plan(&self, placement: &Placement) -> Result<(Vec<Edit>, DropHint), String> {
        match placement.target {
            DropTarget::Outline { node, position } => self.outline(&placement.origin, node, position),
            DropTarget::Canvas { x, y } => self.canvas(&placement.origin, Point::new(x, y)),
        }
    }

    fn outline(&self, origin: &DragOrigin, target: u64, position: DropPosition) -> Result<(Vec<Edit>, DropHint), String> {
        let position = match position {
            DropPosition::Inside => OutlinePlacement::Inside,
            DropPosition::Before => OutlinePlacement::Before,
            DropPosition::After => OutlinePlacement::After,
        };
        let (edits, description) = match origin {
            DragOrigin::Asset { control } => {
                let asset = toolbox::find(control)?;
                (self.session.plan_outline_insert(&asset.source, target, position)?, format!("Insert {}", asset.title))
            }
            DragOrigin::Node { node, .. } => (self.session.plan_outline_move(*node, target, position)?, "Move control".into()),
        };
        Ok((edits, DropHint { bounds: None, description }))
    }

    fn canvas(&self, origin: &DragOrigin, position: Point) -> Result<(Vec<Edit>, DropHint), String> {
        if !(0.0..=f64::from(self.width)).contains(&position.x)
            || !(0.0..=f64::from(self.height)).contains(&position.y)
        {
            return Err("Drop inside the visible artboard".into());
        }
        match origin {
            DragOrigin::Asset { control } => {
                let asset = toolbox::find(control)?;
                self.measured(&self.session.preview_layout_insert(&asset.source, position)?)
            }
            DragOrigin::Node { node, grab } => {
                let bounds = self.bounds.get(node).ok_or("The control has no measured bounds")?;
                let (x, y) = grab.map_or((bounds.x, bounds.y), |point| (point.x, point.y));
                let delta = Point::new(position.x - x, position.y - y);
                if let Some(preview) = self.session.preview_layout_move(&[*node], position, delta)? {
                    return self.measured(&preview);
                }
                let edits = self.session.plan_artboard_translation(*node, position, delta)?;
                let landing = Rect::new(bounds.x + delta.x, bounds.y + delta.y, bounds.width, bounds.height);
                Ok((edits, DropHint { bounds: Some(landing), description: "Move control".into() }))
            }
        }
    }

    fn measured(&self, preview: &LayoutEditPreview) -> Result<(Vec<Edit>, DropHint), String> {
        let edits = preview.source_edits(&self.session)?;
        let helpers = &preview.helpers;
        let bounds = helpers.scene.placement.as_ref()
            .map(|placement| helpers.to_artboard.bounds(placement.bounds))
            .transpose().map_err(|error| error.to_string())?;
        Ok((edits, DropHint { bounds, description: preview.description().into() }))
    }
}
