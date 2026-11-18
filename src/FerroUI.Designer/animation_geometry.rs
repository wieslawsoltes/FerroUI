use crate::{DesignerSession, PlacementPlan};
use ferroui_base::layout::Layoutable;
use ferroui_controls::{Canvas, Control};
use ferroui_designer_animation::AnimationValue;
use ferroui_designer_core::{Edit, NodeId};
use ferroui_designer_tools::{Point, Rect};
use std::collections::{BTreeMap, BTreeSet};

impl DesignerSession {
    pub(crate) fn measured_animation_baseline(
        &self,
        node: NodeId,
        property: &str,
    ) -> Option<AnimationValue> {
        let element = self.preview.visual(node)?.cast::<Layoutable>()?;
        let value = match property {
            "Width" => element.bounds().width,
            "Height" => element.bounds().height,
            "Canvas.Left" => element.previous_arrange()?.x,
            "Canvas.Top" => element.previous_arrange()?.y,
            _ => return None,
        };
        value.is_finite().then_some(AnimationValue::Number(value))
    }

    /// Translate the existing selection in artboard coordinates using Studio's
    /// measured layout planners. The result carries the same geometry for hover and commit.
    pub fn plan_recorded_move(
        &self,
        nodes: &[NodeId],
        anchor: NodeId,
        grab: Option<Point>,
        destination: Point,
    ) -> Result<PlacementPlan, String> {
        if !(1..=128).contains(&nodes.len())
            || !nodes.contains(&anchor)
            || nodes.iter().collect::<BTreeSet<_>>().len() != nodes.len()
        {
            return Err(
                "Choose up to 128 distinct controls and a drag anchor in the selection".into(),
            );
        }
        let artboard = self.preview.host.clone().upcast();
        let measured = self
            .preview
            .bounds(anchor, &artboard)
            .ok_or("The drag anchor has no measured bounds")?;
        let grab = grab.unwrap_or(Point::new(measured.x, measured.y));
        let delta = Point::new(destination.x - grab.x, destination.y - grab.y);
        let edits = self.plan_selection_translation(nodes, delta)?;
        let edits = self.animation_geometry_edits(edits)?;
        let bounds = nodes
            .iter()
            .try_fold(measured, |bounds, &node| {
                self.preview
                    .bounds(node, &artboard)
                    .map(|measured| bounds.union(measured))
            })
            .ok_or("The selection no longer has measured bounds")?;
        Ok(PlacementPlan {
            edits,
            bounds: Some(Rect::new(
                bounds.x + delta.x,
                bounds.y + delta.y,
                bounds.width,
                bounds.height,
            )),
            description: "Record move at playhead".into(),
        })
    }

    pub(crate) fn animation_geometry_edits(&self, edits: Vec<Edit>) -> Result<Vec<Edit>, String> {
        let active = self
            .active_animation_program()
            .and_then(|id| {
                self.animation_programs()
                    .iter()
                    .find(|program| program.id == id && program.timeline.is_some())
            })
            .ok_or("Select a typed animation before recording geometry")?;
        let anchors = edits
            .iter()
            .filter_map(|edit| match edit {
                Edit::SetAttribute { node, name, .. } => match property_name(name) {
                    "Canvas.Left" => Some((*node, "Canvas.Right")),
                    "Canvas.Top" => Some((*node, "Canvas.Bottom")),
                    _ => None,
                },
                _ => None,
            })
            .collect::<BTreeSet<_>>();
        let mut recorded = Vec::with_capacity(edits.len());
        let mut values = BTreeMap::new();
        for edit in edits {
            match edit {
                Edit::SetAttribute { node, name, value } => {
                    let name = property_name(&name);
                    if !active.targets.contains(&node) {
                        return Err("Choose an animation whose selector targets the control before recording its geometry".into());
                    }
                    // One native selector shares its tracks across matching controls.
                    if values.insert(name.to_owned(), value.clone()).is_some_and(|previous| previous != value) {
                        return Err("These controls need different values on a shared track. Record their geometry in separate animations.".into());
                    }
                    self.validate_recorded_value(node, name, &value)?;
                    recorded.push(Edit::SetAttribute { node, name: name.into(), value });
                }
                // Animated leading coordinates override a trailing anchor while
                // sampled. Keep its base value available when the animation stops.
                Edit::RemoveAttribute { node, name }
                    if anchors.contains(&(node, property_name(&name)))
                        || self.has_leading_canvas_anchor(node, property_name(&name)) => {}
                _ => return Err("This geometry edit requires a structural source change. Finish recording before applying it.".into()),
            }
        }
        Ok(recorded)
    }

    fn has_leading_canvas_anchor(&self, node: NodeId, trailing: &str) -> bool {
        let leading = match trailing {
            "Canvas.Right" => Canvas::get_left,
            "Canvas.Bottom" => Canvas::get_top,
            _ => return false,
        };
        self.preview
            .visual(node)
            .and_then(|visual| visual.cast::<Control>())
            .is_some_and(|control| leading(&control).is_finite())
    }

    pub(crate) fn record_geometry(
        &mut self,
        program: NodeId,
        time: f64,
        label: &str,
        edits: Vec<Edit>,
    ) -> Result<(), String> {
        if self.active_animation_program() != Some(program)
            || self.time() != time
            || self.is_playing()
        {
            return Err(
                "Scrub to the requested program and time before recording measured geometry".into(),
            );
        }
        if edits.is_empty() {
            return Ok(());
        }
        let properties = edits
            .iter()
            .filter_map(|edit| match edit {
                Edit::SetAttribute { node, name, .. } => Some((*node, name.clone())),
                _ => None,
            })
            .collect::<Vec<_>>();
        self.record_at(program, time, properties, |session| {
            session.apply_visual(label, edits)
        })
    }
}

fn property_name(name: &str) -> &str {
    name.rsplit(':').next().unwrap_or(name)
}
