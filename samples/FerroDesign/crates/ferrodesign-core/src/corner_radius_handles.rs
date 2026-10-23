//! Shared local geometry and validated independent rectangle-radius editing.
//! Camera transforms and minimum screen-space handle insets belong to the UI.
use crate::{DocumentError, Editor, Node, NodeId, NodeKind, Point};

/// A radius handle's canonical corner and linear drag direction. The location
/// uses the effective clamped radius; pointer deltas edit the authored radius.
/// Keeping these separate prevents a click on a clamped/inset handle from
/// silently rewriting the layer's original value or its variable binding.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CornerRadiusHandle {
    pub corner: usize,
    pub vertex: Point,
    pub position: Point,
    /// Local displacement corresponding to a one-unit authored radius change.
    pub local_per_radius: Point,
    pub authored_radius: f64,
    pub effective_radius: f64,
}

/// At most four box handles, 128 polygon handles or 256 star handles. Straight,
/// reversing, zero-area and nonfinite geometry has no editable corner handle.
/// Polygon fillet clamping is shared with `polygon_corner_path`, not inferred
/// from the exported cubics or approximated independently by the editor.
pub fn corner_radius_handles(node: &Node) -> Vec<CornerRadiusHandle> {
    if !crate::supports_corner_radius(node.kind)
        || !node.width.is_finite()
        || !node.height.is_finite()
        || node.width <= 0.0
        || node.height <= 0.0
        || !node.corner_radius.is_finite()
        || node.corner_radius < 0.0
    {
        return Vec::new();
    }
    if matches!(node.kind, NodeKind::Polygon | NodeKind::Star) {
        if node.kind == NodeKind::Star
            && (!node.star_inner_radius.is_finite() || node.star_inner_radius <= 0.0)
        {
            return Vec::new();
        }
        let points = node.polygon_points();
        if points.iter().any(|point| !finite(*point)) {
            return Vec::new();
        }
        let (corners, scale) = crate::polygon_corners::fillet_corners(&points, node.corner_radius);
        return points
            .into_iter()
            .zip(corners)
            .enumerate()
            .filter_map(|(corner, (vertex, plan))| {
                if plan.turn == 0.0 {
                    return None;
                }
                let normal = Point::new(-plan.incoming.y, plan.incoming.x);
                let local_per_radius =
                    normal * plan.turn.signum() - plan.incoming * plan.tangent_factor;
                let effective_radius = plan.radius * scale;
                let position = vertex + local_per_radius * effective_radius;
                (finite(position) && finite(local_per_radius)).then_some(CornerRadiusHandle {
                    corner,
                    vertex,
                    position,
                    local_per_radius,
                    authored_radius: node.corner_radius,
                    effective_radius,
                })
            })
            .collect();
    }
    let authored = node.corner_radii.unwrap_or([node.corner_radius; 4]);
    if authored
        .iter()
        .any(|radius| !radius.is_finite() || *radius < 0.0)
    {
        return Vec::new();
    }
    let effective = crate::normalized_corner_radii(node.width, node.height, authored);
    let directions = [
        Point::new(1.0, 1.0),
        Point::new(-1.0, 1.0),
        Point::new(-1.0, -1.0),
        Point::new(1.0, -1.0),
    ];
    node.local_bounds()
        .corners()
        .into_iter()
        .enumerate()
        .map(|(corner, vertex)| {
            let local_per_radius = directions[corner];
            CornerRadiusHandle {
                corner,
                vertex,
                position: vertex + local_per_radius * effective[corner],
                local_per_radius,
                authored_radius: authored[corner],
                effective_radius: effective[corner],
            }
        })
        .collect()
}

fn finite(point: Point) -> bool {
    point.x.is_finite() && point.y.is_finite()
}

impl Editor {
    /// Edit one Rectangle corner atomically. Other kinds are left unchanged:
    /// procedural polygon/star geometry has one authored uniform radius.
    /// The uniform source value is retained when independent radii are enabled;
    /// a literal corner edit detaches its uniform binding and marks the existing
    /// component override shared by uniform/independent rectangle radii.
    pub fn set_individual_corner_radius(
        &mut self,
        ids: &[NodeId],
        corner: usize,
        radius: f64,
    ) -> Result<usize, DocumentError> {
        if corner >= 4 || !radius.is_finite() || radius < 0.0 {
            return Err(DocumentError::Invalid(
                "Choose a rectangle corner and a finite nonnegative radius.".into(),
            ));
        }
        let mut changed = std::collections::BTreeSet::new();
        for &id in ids {
            let node = self
                .document
                .nodes
                .get(&id)
                .ok_or(DocumentError::MissingNode(id))?;
            if node.kind != NodeKind::Rectangle || self.document.effective_locked(id) {
                continue;
            }
            let mut radii = node.corner_radii.unwrap_or([node.corner_radius; 4]);
            radii[corner] = radius;
            if node.corner_radii != Some(radii)
                || node.variable_bindings.contains_key("corner_radius")
            {
                changed.insert(id);
            }
        }
        if changed.is_empty() {
            return Ok(0);
        }
        let ids: Vec<_> = changed.into_iter().collect();
        self.update_nodes(&ids, "Individual corner radius", |node| {
            node.corner_radii.get_or_insert([node.corner_radius; 4])[corner] = radius;
            node.variable_bindings.remove("corner_radius");
            node.overridden_properties.insert("corner_radius".into());
        });
        Ok(ids.len())
    }
}

#[cfg(test)]
#[path = "corner_radius_handles_tests.rs"]
mod tests;
