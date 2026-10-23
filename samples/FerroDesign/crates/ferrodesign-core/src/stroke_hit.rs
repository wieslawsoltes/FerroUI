//! Prepared local outlines reused by the scene index during pointer picking.
use crate::{distance_to_segment, outline_path, Node, Point, Rect, StrokeAlign, VectorPath};

#[derive(Clone, Debug)]
pub(crate) struct VariableStrokeHits {
    source: Node,
    fill: PreparedPath,
    strokes: Vec<(StrokeAlign, PreparedPath)>,
    bounds: Rect,
}

impl VariableStrokeHits {
    pub(crate) fn prepare(node: &Node) -> Option<Self> {
        let strokes: Vec<_> = node.strokes.iter()
            .filter(|stroke| stroke.visible && stroke.opacity > 0.0 && stroke.width > 0.0 && !stroke.width_profile.is_uniform())
            .filter_map(|stroke| {
                crate::variable_width_outline(node, stroke).ok().map(|outline| {
                    let align = if node.has_closed_stroke_contours() { stroke.align } else { StrokeAlign::Center };
                    (align, outline)
                })
            }).collect();
        if strokes.is_empty() { return None; }
        let bounds = strokes.iter().filter_map(|(_, path)| path.tight_bounds())
            .fold(node.local_bounds(), Rect::union);
        // Retain geometry inputs only. A translated frame with many children
        // must neither copy its subtree nor tessellate its border again.
        let mut source = Node::new(node.kind, "", 0.0, 0.0, node.width, node.height);
        source.corner_radius = node.corner_radius;
        source.corner_radii = node.corner_radii;
        source.corner_smoothing = node.corner_smoothing;
        source.polygon_sides = node.polygon_sides;
        source.star_inner_radius = node.star_inner_radius;
        source.path = node.path.clone();
        source.vector_network = node.vector_network.clone();
        source.strokes = node.strokes.iter().map(|stroke| crate::Stroke {
            paint: crate::Paint::Solid(crate::Color::BLACK), width: stroke.width,
            opacity: stroke.opacity, visible: stroke.visible, align: stroke.align,
            cap: stroke.cap, join: stroke.join, dash: stroke.dash.clone(),
            width_profile: stroke.width_profile.clone(),
        }).collect();
        Some(Self { source, fill: PreparedPath::new(&outline_path(node)),
            strokes: strokes.into_iter().map(|(align, path)| (align, PreparedPath::new(&path))).collect(), bounds })
    }

    pub(crate) fn matches(&self, node: &Node) -> bool {
        let old = &self.source;
        old.kind == node.kind && old.width == node.width && old.height == node.height
            && old.corner_radius == node.corner_radius && old.corner_radii == node.corner_radii
            && old.corner_smoothing == node.corner_smoothing && old.polygon_sides == node.polygon_sides
            && old.star_inner_radius == node.star_inner_radius && old.path == node.path
            && old.vector_network == node.vector_network && old.strokes.len() == node.strokes.len()
            && old.strokes.iter().zip(&node.strokes).all(|(a, b)| a.width == b.width && a.opacity == b.opacity
                && a.visible == b.visible && a.align == b.align && a.cap == b.cap && a.join == b.join
                && a.dash == b.dash && a.width_profile == b.width_profile)
    }

    pub(crate) fn bounds(&self) -> Rect { self.bounds }

    pub(crate) fn hit(&self, point: Point, tolerance: f64) -> bool {
        if !self.bounds.inflate(tolerance).contains(point) { return false; }
        let in_fill = self.strokes.iter().any(|(align, _)| *align != StrokeAlign::Center) && self.fill.contains(point);
        self.strokes.iter().any(|(align, path)| {
            if (*align == StrokeAlign::Inside && !in_fill) || (*align == StrokeAlign::Outside && in_fill) { return false; }
            path.contains(point) || (tolerance > 0.0 && path.contours.iter().any(|contour|
                contour.windows(2).any(|edge| distance_to_segment(point, edge[0], edge[1]) <= tolerance)))
        })
    }
}

/// Keep flattened ink in the retained index. Pointer motion must never allocate
/// or repeat path flattening; placement-only updates retain these allocations.
#[derive(Clone, Debug)]
struct PreparedPath {
    rule: crate::FillRule,
    contours: Vec<Vec<Point>>,
}
impl PreparedPath {
    fn new(path: &VectorPath) -> Self { Self { rule: path.fill_rule, contours: path.flattened(0.1) } }
    fn contains(&self, point: Point) -> bool { crate::geometry::path_contours_contain(self.rule, &self.contours, point) }
}

#[cfg(test)]
#[path = "stroke_hit_tests.rs"]
mod tests;
