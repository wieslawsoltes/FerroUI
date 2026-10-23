//! Keep authoritative vector geometry aligned with layout-assigned dimensions.

use super::*;

impl Node {
    /// Scale vector coordinates before a caller assigns new width/height.
    /// Position and dimensions remain the caller's responsibility: explicit
    /// resize preserves its transformed origin, while layout assigns position.
    /// Unchanged sizes and non-vector nodes take the constant-time path.
    pub(crate) fn resize_vector_geometry(&mut self, width: f64, height: f64) -> bool {
        if self.kind != NodeKind::Vector || (self.width == width && self.height == height) {
            return true;
        }
        if !width.is_finite() || !height.is_finite() || width < 0.0 || height < 0.0 {
            return false;
        }
        let sx = if self.width > GEOMETRY_EPSILON { width / self.width } else { 1.0 };
        let sy = if self.height > GEOMETRY_EPSILON { height / self.height } else { 1.0 };
        if !sx.is_finite() || !sy.is_finite() {
            return false;
        }
        if sx == 1.0 && sy == 1.0 {
            return true;
        }
        let scale = Transform::scale(sx, sy);
        if let Some(network) = &self.vector_network {
            let network = network.transformed(scale);
            if network.validate().is_err() {
                return false;
            }
            self.vector_network = Some(network);
        } else {
            let path = self.path.transformed(scale);
            if Topology::new(&path).is_none() || !path.validate_anchor_modes() {
                return false;
            }
            self.path = path;
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{AutoLayout, Constraint, Document, Editor, LayoutAxis, Padding, SizingMode, VectorNetwork};

    fn vector(network: bool) -> Node {
        let mut node = Node::new(NodeKind::Vector, "Layout curve", 10.0, 20.0, 100.0, 50.0);
        node.path = VectorPath {
            segments: vec![
                PathSegment::MoveTo { point: Point::new(0.0, 0.0) },
                PathSegment::CubicTo {
                    control1: Point::new(25.0, 50.0),
                    control2: Point::new(75.0, 50.0),
                    point: Point::new(100.0, 0.0),
                },
            ],
            ..VectorPath::default()
        };
        if network {
            node.vector_network = VectorNetwork::from_path(&node.path);
            node.path = VectorPath::default();
        }
        node
    }

    fn assert_scaled(actual: &Node, before: &Node, sx: f64, sy: f64) {
        let scale = Transform::scale(sx, sy);
        if let Some(network) = &before.vector_network {
            assert_eq!(actual.vector_network.as_ref().unwrap(), &network.transformed(scale));
            assert!(actual.path.segments.is_empty());
        } else {
            assert_eq!(actual.path, before.path.transformed(scale));
            assert!(actual.vector_network.is_none());
        }
    }

    #[test]
    fn constraints_scale_vector_geometry_with_assigned_bounds_for_paths_and_networks() {
        for network in [false, true] {
            let mut document = Document::new("Constraint vectors");
            let page = document.pages[0].id;
            let frame = document.insert_node(page, Node::new(NodeKind::Frame, "Frame", 0.0, 0.0, 200.0, 100.0)).unwrap();
            let mut node = vector(network);
            node.constraints.horizontal = Constraint::Scale;
            node.constraints.vertical = Constraint::Stretch;
            let id = document.insert_node(frame, node).unwrap();
            let before = document.nodes[&id].clone();
            document.apply_constraints(frame, 200.0, 100.0, 400.0, 150.0);
            let actual = &document.nodes[&id];
            assert_eq!((actual.x, actual.y, actual.width, actual.height), (20.0, 20.0, 200.0, 100.0));
            assert_scaled(actual, &before, 2.0, 2.0);
            document.validate().unwrap();
        }
    }

    #[test]
    fn auto_layout_fill_resizes_vector_geometry_once_and_converges() {
        for network in [false, true] {
            let mut document = Document::new("Auto layout vectors");
            let page = document.pages[0].id;
            let mut frame = Node::new(NodeKind::Frame, "Frame", 0.0, 0.0, 400.0, 200.0);
            frame.layout = Some(AutoLayout {
                axis: LayoutAxis::Row,
                padding: Padding::all(0.0),
                gap: 0.0,
                width: SizingMode::Fixed,
                height: SizingMode::Fixed,
                ..AutoLayout::default()
            });
            let frame = document.insert_node(page, frame).unwrap();
            let mut node = vector(network);
            node.layout_child.width = SizingMode::Fill;
            node.layout_child.height = SizingMode::Fill;
            let id = document.insert_node(frame, node).unwrap();
            let before = document.nodes[&id].clone();
            let result = document.layout_node(frame);
            assert!(result.changed.contains(&id));
            let actual = &document.nodes[&id];
            assert_eq!((actual.x, actual.y, actual.width, actual.height), (0.0, 0.0, 400.0, 200.0));
            assert_scaled(actual, &before, 4.0, 4.0);
            let arranged = actual.clone();
            assert!(document.layout_all().changed.is_empty());
            assert_eq!(document.nodes[&id], arranged);
            document.validate().unwrap();
        }
    }

    #[test]
    fn parent_resize_undo_restores_constrained_network_geometry_and_stable_ids() {
        let mut document = Document::new("Resize network frame");
        let page = document.pages[0].id;
        let frame = document.insert_node(page, Node::new(NodeKind::Frame, "Frame", 0.0, 0.0, 200.0, 100.0)).unwrap();
        let mut node = vector(true);
        node.constraints.horizontal = Constraint::Scale;
        node.constraints.vertical = Constraint::Scale;
        let id = document.insert_node(frame, node).unwrap();
        let mut editor = Editor::new(document);
        let before = editor.document.to_json().unwrap();
        let original = editor.document.nodes[&id].clone();
        editor.resize_node(frame, 600.0, 200.0);
        assert_eq!(editor.history_len(), 1);
        assert_scaled(&editor.document.nodes[&id], &original, 3.0, 2.0);
        let resized = editor.document.to_json().unwrap();
        assert!(editor.undo());
        assert_eq!(editor.document.to_json().unwrap(), before);
        assert!(editor.redo());
        assert_eq!(editor.document.to_json().unwrap(), resized);
    }

    #[test]
    fn vector_geometry_resize_declines_overflow_without_partial_changes() {
        for network in [false, true] {
            let mut node = vector(network);
            node.width = 1e-7;
            let before = node.clone();
            assert!(!node.resize_vector_geometry(1e308, 100.0));
            assert_eq!(node, before);
        }
    }
}
