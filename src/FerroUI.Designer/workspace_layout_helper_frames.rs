//! Compose child layout features through native presenters without changing authored units.
//! A Viewbox or a control template may scale/rotate the child's real layout parent. Margin
//! geometry belongs to that parent, while the surrounding panel's other helpers stay in the
//! panel frame. Each composed grip retains its own input frame for the captured gesture.
//! Filled margin regions remain filled when their composed edges are axis-aligned. Rotated
//! or sheared regions use their four actual edges, avoiding a misleading bounding-box fill.
use crate::DesignerSession;
use crate::workspace_tools::{Measurement, measure, parent};
use ferroui_designer_core::NodeId;
use ferroui_designer_tools::{
    Affine, LayoutGuideGeometry, LayoutGuideRole, LayoutHelperScene, Point,
};
use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug)]
pub(crate) struct HelperHandleFrame {
    pub(crate) to_artboard: Affine,
    pub(crate) to_panel: Affine,
}

pub(crate) fn transform_vector(transform: Affine, vector: Point) -> Result<Point, String> {
    let origin = transform
        .transform(Point::new(0.0, 0.0))
        .map_err(|error| error.to_string())?;
    let end = transform
        .transform(vector)
        .map_err(|error| error.to_string())?;
    Ok(Point::new(end.x - origin.x, end.y - origin.y))
}

pub(crate) fn append_native_margins(
    session: &DesignerSession,
    panel: NodeId,
    measured: &Measurement,
    scene: &mut LayoutHelperScene,
) -> Result<BTreeMap<String, HelperHandleFrame>, String> {
    let mut frames = BTreeMap::new();
    let mut presenter_measurements: Vec<Measurement> = Vec::new();
    for node in session.selection().iter().copied().take(64) {
        if parent(session, node).ok() != Some(panel) {
            continue;
        }
        let Some(visual) = session
            .preview
            .visual(node)
            .filter(|visual| visual.is_visible())
        else {
            continue;
        };
        let Some(layout_parent) = visual.visual_parent() else {
            continue;
        };
        // Most panels directly own their children. Reuse the existing measurement in that
        // common case, and measure each distinct presenter once for multi-selection.
        let child_measurement = if layout_parent.ptr_eq(&measured.visual) {
            measured
        } else {
            let index = match presenter_measurements
                .iter()
                .position(|measurement| measurement.visual.ptr_eq(&layout_parent))
            {
                Some(index) => index,
                None => {
                    let measurement = match measure(session, panel, Some(node)) {
                        Ok(measurement) => measurement,
                        Err(reason) => {
                            if let Some(bounds) = measured.bounds.get(&node) {
                                scene.label(
                                    bounds.center(),
                                    format!("Margin unavailable: {reason}"),
                                    LayoutGuideRole::Rejected,
                                );
                            }
                            continue;
                        }
                    };
                    presenter_measurements.push(measurement);
                    presenter_measurements.len() - 1
                }
            };
            &presenter_measurements[index]
        };
        if !child_measurement.bounds.contains_key(&node) {
            continue;
        }
        let to_panel = child_measurement
            .to_artboard
            .then(measured.to_parent)
            .map_err(|error| error.to_string())?;
        let mut margins = LayoutHelperScene {
            panel,
            title: String::new(),
            primitives: Vec::new(),
            handles: Vec::new(),
            placement: None,
        };
        margins
            .append_margin_helpers(&child_measurement.context(session.document()), node)
            .map_err(|error| error.to_string())?;
        append_primitives(scene, margins.primitives, to_panel)?;
        for mut handle in margins.handles {
            handle.position = to_panel
                .transform(handle.position)
                .map_err(|error| error.to_string())?;
            frames.insert(
                handle.id.clone(),
                HelperHandleFrame {
                    to_artboard: child_measurement.to_artboard,
                    to_panel,
                },
            );
            scene.handles.push(handle);
        }
    }
    Ok(frames)
}

fn append_primitives(
    scene: &mut LayoutHelperScene,
    guides: Vec<ferroui_designer_tools::LayoutGuide>,
    transform: Affine,
) -> Result<(), String> {
    let epsilon = 1e-12
        * transform
            .m11
            .abs()
            .max(transform.m12.abs())
            .max(transform.m21.abs())
            .max(transform.m22.abs())
            .max(1.0);
    let axis_aligned = (transform.m12.abs() <= epsilon && transform.m21.abs() <= epsilon)
        || (transform.m11.abs() <= epsilon && transform.m22.abs() <= epsilon);
    let point = |point| {
        transform
            .transform(point)
            .map_err(|error| error.to_string())
    };
    for guide in guides {
        match guide.geometry {
            LayoutGuideGeometry::Line { start, end } => {
                scene.line(point(start)?, point(end)?, guide.role);
            }
            LayoutGuideGeometry::Rectangle { bounds, filled } => {
                if filled && axis_aligned {
                    scene.rectangle(
                        transform
                            .bounds(bounds)
                            .map_err(|error| error.to_string())?,
                        guide.role,
                        true,
                    );
                } else {
                    // The shared scene carries rectangles rather than arbitrary filled
                    // polygons. Keep exact transformed edges for non-axis-aligned regions.
                    let corners = [
                        point(Point::new(bounds.x, bounds.y))?,
                        point(Point::new(bounds.right(), bounds.y))?,
                        point(Point::new(bounds.right(), bounds.bottom()))?,
                        point(Point::new(bounds.x, bounds.bottom()))?,
                    ];
                    for index in 0..4 {
                        scene.line(corners[index], corners[(index + 1) % 4], guide.role);
                    }
                }
            }
            LayoutGuideGeometry::Label { position, text } => {
                scene.label(point(position)?, text, guide.role);
            }
        }
    }
    Ok(())
}

#[cfg(test)]
#[path = "workspace_layout_helper_frames_tests.rs"]
mod tests;
