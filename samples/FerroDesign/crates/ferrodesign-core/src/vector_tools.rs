//! Direct curve bending and segment erasing for the vector editor.
//!
//! This module is a child of vector_edit so it shares the same exact topology,
//! subdivision and constraint implementation as point editing.

use super::*;
use crate::{Editor, NodeId};

impl VectorPath {
    /// Pulls the point at `t` on an edge to `position` while keeping its anchors
    /// fixed. The two cubic controls move by the least combined squared
    /// displacement satisfying that point constraint. Neighboring Smooth and
    /// Symmetric tangents follow their respective endpoint handle.
    ///
    /// Lines and quadratics are first degree elevated without changing shape.
    /// A closing line becomes an explicit cubic followed by Close; the edge's
    /// command index remains stable for subsequent updates in the same gesture.
    /// A constrained single-anchor loop is declined because its two linked
    /// controls cannot generally satisfy an arbitrary independent bend.
    pub fn bend_segment(&mut self, segment: usize, t: f64, position: Point) -> bool {
        if !finite(position) || !t.is_finite() || !(0.0 < t && t < 1.0) {
            return false;
        }
        let Some(topology) = Topology::new(self) else {
            return false;
        };
        let Some(edge) = topology.edges.iter().find(|edge| edge.segment == segment) else {
            return false;
        };
        if edge.start_anchor == Some(edge.end_anchor)
            && (matches!(self.segments[segment], PathSegment::Close)
                || self.anchor_mode(edge.end_anchor) != Some(AnchorMode::Corner))
        {
            return false;
        }
        let delta = position - edge.curve.evaluate(t);
        if !finite(delta) || delta == Point::default() {
            return false;
        }
        let first_weight = 3.0 * (1.0 - t) * (1.0 - t) * t;
        let second_weight = 3.0 * (1.0 - t) * t * t;
        let denominator = first_weight * first_weight + second_weight * second_weight;
        if denominator <= f64::MIN_POSITIVE || !denominator.is_finite() {
            return false;
        }
        let (control1, control2, point) = edge.curve.as_cubic();
        let first = control1 + delta * (first_weight / denominator);
        let second = control2 + delta * (second_weight / denominator);
        if !finite(first) || !finite(second) {
            return false;
        }
        let mut candidate = self.clone();
        if matches!(candidate.segments[segment], PathSegment::Close) {
            candidate.segments.insert(segment + 1, PathSegment::Close);
            candidate.anchor_modes = self
                .canonical_anchor_modes(&topology)
                .into_iter()
                .map(|(anchor, mode)| {
                    (if anchor > segment { anchor + 1 } else { anchor }, mode)
                })
                .collect();
        }
        candidate.segments[segment] = PathSegment::CubicTo {
            control1,
            control2,
            point,
        };
        let first_id = PathPointId { segment, kind: PathPointKind::Control1 };
        let second_id = PathPointId { segment, kind: PathPointKind::Control2 };
        if !candidate.move_edit_point(first_id, first)
            || !candidate.move_edit_point(second_id, second)
            || !candidate.validate_anchor_modes()
        {
            return false;
        }
        *self = candidate;
        true
    }

    /// Erases an edge without reconnecting across the removed span. An open
    /// contour splits into two open contours; a closed contour becomes open.
    /// Surviving Bézier controls and anchor modes stay attached to their
    /// original vertices, and isolated endpoint anchors remain editable.
    ///
    /// This changes the command stream, not paint properties: the document's
    /// existing SVG fill rule still applies to any resulting open contours.
    pub fn erase_segment(&mut self, segment: usize) -> bool {
        let Some(topology) = Topology::new(self) else {
            return false;
        };
        let Some(edge) = topology.edges.iter().find(|edge| edge.segment == segment) else {
            return false;
        };
        let Some(mut contour) = Contour::for_anchor(self, edge.end_anchor) else {
            return false;
        };
        let Some(index) = segment.checked_sub(contour.start + 1) else {
            return false;
        };
        if index >= contour.edges.len() {
            // Close following an already explicit closing curve has no edge
            // geometry of its own; only actual drawn edges can be erased.
            return false;
        }
        if !contour.closed {
            self.segments[segment] = PathSegment::MoveTo {
                point: edge.curve.points[edge.curve.degree],
            };
            return true;
        }
        let next = (index + 1) % contour.points.len();
        contour.points.rotate_left(next);
        contour.anchor_commands.rotate_left(next);
        contour.edges.rotate_left(next);
        contour.edges.pop();
        contour.closed = false;
        let replacement = contour.commands();
        self.replace_contour(&contour, replacement);
        true
    }
}

impl Editor {
    /// Pointer-frequency bend operation. Enclose a gesture in a transaction to
    /// retain one undo item, just as for anchor and handle dragging.
    pub fn bend_vector_segment(
        &mut self,
        id: NodeId,
        segment: usize,
        t: f64,
        world_position: Point,
    ) -> bool {
        let Some(inverse) = self.document.world_transform(id).and_then(Transform::inverse) else {
            return false;
        };
        self.edit_vector_path(id, "Bend vector segment", |path| {
            path.bend_segment(segment, t, inverse.apply(world_position)).then_some(())
        })
        .is_some()
    }

    pub fn erase_vector_segment(&mut self, id: NodeId, segment: usize) -> bool {
        self.edit_vector_path(id, "Erase vector segment", |path| {
            path.erase_segment(segment).then_some(())
        })
        .is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(x: f64, y: f64) -> Point { Point::new(x, y) }

    fn assert_close(actual: Point, expected: Point) {
        assert!(actual.distance(expected) < 1e-7, "{actual:?} != {expected:?}");
    }

    fn open_curve() -> VectorPath {
        VectorPath {
            segments: vec![
                PathSegment::MoveTo { point: p(0.0, 0.0) },
                PathSegment::CubicTo {
                    control1: p(30.0, 10.0), control2: p(80.0, 120.0), point: p(100.0, 100.0),
                },
                PathSegment::CubicTo {
                    control1: p(150.0, 70.0), control2: p(170.0, 130.0), point: p(200.0, 50.0),
                },
            ],
            ..VectorPath::default()
        }
    }

    #[test]
    fn bend_passes_through_pointer_with_fixed_anchors_and_minimum_control_displacement() {
        for t in [0.03, 0.25, 0.5, 0.83, 0.97] {
            let mut path = open_curve();
            let before = path.clone();
            let original = Curve::from_segment(Point::default(), &before.segments[1]).unwrap();
            let delta = p(-18.0, 26.0);
            let target = original.evaluate(t) + delta;
            assert!(path.bend_segment(1, t, target));
            let result = Curve::from_segment(Point::default(), &path.segments[1]).unwrap();
            assert_close(result.evaluate(t), target);
            assert_eq!(result.points[0], original.points[0]);
            assert_eq!(result.points[3], original.points[3]);
            let d1 = result.points[1] - original.points[1];
            let d2 = result.points[2] - original.points[2];
            // The minimum-norm solution has displacements in the ratio of
            // the two Bernstein weights, including away from the midpoint.
            assert_close(d1 * t, d2 * (1.0 - t));
            assert_eq!(path.segments[0], before.segments[0]);
            assert_eq!(path.segments[2], before.segments[2]);
        }
    }

    #[test]
    fn bending_line_and_quadratic_elevates_degree_and_preserves_other_contours() {
        for edge in [
            PathSegment::LineTo { point: p(100.0, 20.0) },
            PathSegment::QuadraticTo { control: p(50.0, 100.0), point: p(100.0, 20.0) },
        ] {
            let mut path = VectorPath {
                segments: vec![
                    PathSegment::MoveTo { point: Point::default() }, edge,
                    PathSegment::MoveTo { point: p(400.0, 300.0) },
                ],
                ..VectorPath::default()
            };
            assert!(path.bend_segment(1, 0.4, p(37.0, -80.0)));
            let curve = Curve::from_segment(Point::default(), &path.segments[1]).unwrap();
            assert_eq!(curve.degree, 3);
            assert_close(curve.evaluate(0.4), p(37.0, -80.0));
            assert_eq!(path.segments[2], PathSegment::MoveTo { point: p(400.0, 300.0) });
        }
    }

    #[test]
    fn bend_propagates_persistent_tangents_and_rejects_invalid_edits_atomically() {
        let mut path = open_curve();
        assert!(path.convert_anchor(1, AnchorMode::Symmetric));
        let start = path.point_position(PathPointId::anchor(1)).unwrap();
        let curve = Curve::from_segment(start, &path.segments[2]).unwrap();
        let target = curve.evaluate(0.35) + p(60.0, 30.0);
        assert!(path.bend_segment(2, 0.35, target));
        assert!(path.validate_anchor_modes());
        let curve = Curve::from_segment(start, &path.segments[2]).unwrap();
        assert_close(curve.evaluate(0.35), target);
        let before = path.clone();
        assert!(!path.bend_segment(2, 0.0, target));
        assert!(!path.bend_segment(2, 1.0, target));
        assert!(!path.bend_segment(2, f64::NAN, target));
        assert!(!path.bend_segment(0, 0.5, target));
        assert!(!path.bend_segment(2, 0.5, p(f64::INFINITY, 0.0)));
        assert_eq!(path, before);
    }

    #[test]
    fn bend_declines_linked_single_anchor_loop_when_midpoint_cannot_move() {
        let mut path = VectorPath {
            segments: vec![
                PathSegment::MoveTo { point: Point::default() },
                PathSegment::CubicTo {
                    control1: p(40.0, 80.0),
                    control2: p(-40.0, -80.0),
                    point: Point::default(),
                },
                PathSegment::Close,
            ],
            ..VectorPath::default()
        };
        path.anchor_modes.insert(0, AnchorMode::Symmetric);
        assert!(path.validate_anchor_modes());
        let before = path.clone();
        assert!(!path.bend_segment(1, 0.5, p(20.0, 60.0)));
        assert_eq!(path, before);
    }

    #[test]
    fn bending_implicit_close_keeps_the_edge_id_and_tail_metadata_stable() {
        let mut path = VectorPath::polygon(&[p(0.0, 0.0), p(100.0, 0.0), p(100.0, 100.0)], true);
        path.segments.extend([
            PathSegment::MoveTo { point: p(500.0, 0.0) },
            PathSegment::LineTo { point: p(600.0, 0.0) },
        ]);
        assert!(path.convert_anchor(5, AnchorMode::Smooth));
        assert!(path.bend_segment(3, 0.5, p(0.0, 100.0)));
        assert_eq!(path.segments[4], PathSegment::Close);
        assert_eq!(path.anchor_mode(6), Some(AnchorMode::Smooth));
        let count = path.segments.len();
        assert!(path.bend_segment(3, 0.5, p(-20.0, 110.0)));
        assert_eq!(path.segments.len(), count);
        assert_close(Curve::from_segment(p(100.0, 100.0), &path.segments[3]).unwrap().evaluate(0.5), p(-20.0, 110.0));
        assert!(path.validate_anchor_modes());
    }

    #[test]
    fn erasing_open_edge_splits_contour_without_reconnecting_or_changing_retained_curves() {
        let mut path = open_curve();
        assert!(path.convert_anchor(1, AnchorMode::Smooth));
        let before = path.clone();
        assert!(path.erase_segment(1));
        assert_eq!(path.segments[0], before.segments[0]);
        assert_eq!(path.segments[1], PathSegment::MoveTo { point: p(100.0, 100.0) });
        assert_eq!(path.segments[2], before.segments[2]);
        assert_eq!(path.anchor_mode(1), Some(AnchorMode::Smooth));
        assert!(path.validate_anchor_modes());
        assert_eq!(Topology::new(&path).unwrap().edges.len(), 1);
        assert!(!path.erase_segment(1));
    }

    #[test]
    fn erasing_closed_edge_rotates_retained_anchors_and_modes_without_curve_changes() {
        let mut path = open_curve();
        path.segments.push(PathSegment::Close);
        assert!(path.convert_anchor(1, AnchorMode::Smooth));
        let before = path.clone();
        assert!(path.erase_segment(1));
        assert_eq!(path.segments[0], PathSegment::MoveTo { point: p(100.0, 100.0) });
        assert_eq!(path.segments[1], before.segments[2]);
        assert_eq!(path.segments[2], PathSegment::LineTo { point: Point::default() });
        assert_eq!(path.anchor_mode(0), Some(AnchorMode::Smooth));
        assert!(!path.segments.iter().any(|edge| matches!(edge, PathSegment::Close)));
        assert!(path.validate_anchor_modes());
    }

    #[test]
    fn bend_and_erase_use_normalized_world_coordinates_and_transaction_history() {
        let mut document = crate::Document::new("Vector tools");
        let page = document.pages[0].id;
        let mut node = Node::new(NodeKind::Vector, "Curve", 100.0, 120.0, 200.0, 150.0);
        node.path = open_curve();
        node.rotation = 27.0;
        let id = document.insert_node(page, node).unwrap();
        let mut editor = Editor::new(document);
        let original = editor.document.to_json().unwrap();
        let world = editor.document.world_transform(id).unwrap();
        let start = world.apply(Curve::from_segment(Point::default(), &editor.document.nodes[&id].path.segments[1]).unwrap().evaluate(0.5));
        editor.begin_transaction("Bend curve");
        for step in 1..=12 {
            assert!(editor.bend_vector_segment(id, 1, 0.5, start + p(-5.0 * step as f64, 4.0 * step as f64)));
        }
        assert!(editor.commit_transaction());
        assert_eq!(editor.history_len(), 1);
        let bent = editor.document.to_json().unwrap();
        assert!(editor.erase_vector_segment(id, 1));
        assert_eq!(editor.history_len(), 2);
        assert!(editor.undo());
        assert_eq!(editor.document.to_json().unwrap(), bent);
        assert!(editor.undo());
        assert_eq!(editor.document.to_json().unwrap(), original);
    }
}
