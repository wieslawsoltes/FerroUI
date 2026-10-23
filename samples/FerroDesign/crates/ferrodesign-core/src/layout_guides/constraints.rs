//! Parent-local constraints relative to responsive layout-guide bands.
//!
//! Figma documents stretch guides as constraint references and fixed guides as
//! frame-relative references. Stack order chooses the first usable stretch
//! guide of each orientation. Hidden guides remain layout references.
use crate::{Constraint, Constraints, LayoutGrid, LayoutGridAlignment, LayoutGridBands,
    LayoutGridPattern, Node, Point, Rect, Transform, GEOMETRY_EPSILON};

fn nearest_band(bands: LayoutGridBands, coordinate: f64) -> u32 {
    // The nearest interval is the interval with the nearest center because the
    // bands have equal width. Ties consistently choose the later interval.
    ((coordinate - bands.start - bands.size / 2.0) / bands.step)
        .round().clamp(0.0, (bands.count - 1) as f64) as u32
}

fn interval(bands: LayoutGridBands, first: u32, last: u32) -> (f64, f64) {
    let first = first.min(bands.count - 1);
    let last = last.max(first).min(bands.count - 1);
    let start = bands.start + first as f64 * bands.step;
    let end = bands.start + last as f64 * bands.step + bands.size;
    (start, end)
}

fn frame_axis(position: f64, size: f64, old: f64, new: f64, constraint: Constraint) -> (f64, f64) {
    let delta = new - old;
    match constraint {
        Constraint::Start => (position, size),
        Constraint::End => (position + delta, size),
        Constraint::Center => (position + delta / 2.0, size),
        Constraint::Stretch => (position, (size + delta).max(0.0)),
        Constraint::Scale if old.abs() > GEOMETRY_EPSILON => (position * new / old, (size * new / old).max(0.0)),
        Constraint::Scale => (position, size),
    }
}

fn guide_axis(position: f64, size: f64, old: LayoutGridBands, new: LayoutGridBands, constraint: Constraint) -> (f64, f64) {
    let first = nearest_band(old, position);
    let last = nearest_band(old, position + size).max(first);
    let (old_start, old_end) = interval(old, first, last);
    let (new_start, new_end) = interval(new, first, last);
    match constraint {
        Constraint::Start => (position + new_start - old_start, size),
        Constraint::End => (position + new_end - old_end, size),
        Constraint::Center => {
            let center = nearest_band(old, position + size / 2.0);
            let (a, b) = interval(old, center, center);
            let (c, d) = interval(new, center, center);
            (position + (c + d - a - b) / 2.0, size)
        }
        Constraint::Stretch => (position + new_start - old_start,
            (size + (new_end - new_start) - (old_end - old_start)).max(0.0)),
        Constraint::Scale => {
            let scale = (new_end - new_start) / (old_end - old_start);
            (new_start + (position - old_start) * scale, (size * scale).max(0.0))
        }
    }
}

/// Resolve a child's parent-local visual bounds during a frame resize.
///
/// Returns `None` when no usable stretch guide exists, retaining the ordinary
/// frame-constraint path. Work is bounded by the authored stack (at most 64
/// guides); a million sections require only arithmetic, never an expanded list.
/// Fixed/auto counts keep the old section indices, clamped to the new count.
pub fn layout_guide_constrained_bounds(bounds: Rect, constraints: Constraints, guides: &[LayoutGrid], old: Point, new: Point) -> Option<Rect> {
    if !bounds.is_finite() || ![old.x, old.y, new.x, new.y].iter().all(|v| v.is_finite() && *v >= 0.0) {
        return None;
    }
    let mut horizontal = None;
    let mut vertical = None;
    for guide in guides.iter().take(super::MAX_LAYOUT_GUIDES) {
        if guide.pattern.axis().is_none_or(|axis| axis.alignment != LayoutGridAlignment::Stretch) { continue; }
        let (slot, before, after) = match guide.pattern {
            LayoutGridPattern::Columns { .. } => (&mut horizontal, old.x, new.x),
            LayoutGridPattern::Rows { .. } => (&mut vertical, old.y, new.y),
            LayoutGridPattern::Square => continue,
        };
        if slot.is_none() {
            if let (Some(a), Some(b)) = (guide.bands(before), guide.bands(after)) { *slot = Some((a, b)); }
        }
        if horizontal.is_some() && vertical.is_some() { break; }
    }
    if horizontal.is_none() && vertical.is_none() { return None; }
    let (x, width) = horizontal.map_or_else(
        || frame_axis(bounds.x, bounds.width, old.x, new.x, constraints.horizontal),
        |(a, b)| guide_axis(bounds.x, bounds.width, a, b, constraints.horizontal));
    let (y, height) = vertical.map_or_else(
        || frame_axis(bounds.y, bounds.height, old.y, new.y, constraints.vertical),
        |(a, b)| guide_axis(bounds.y, bounds.height, a, b, constraints.vertical));
    let result = Rect::new(x, y, width, height);
    result.is_finite().then_some(result)
}

/// Apply guide constraints without flattening rotated, flipped or sheared
/// geometry. A transformed child keeps its source geometry and receives the
/// parent-axis affine adjustment; ordinary children retain editable dimensions.
pub(crate) fn apply_layout_guide_constraints(child: &mut Node, guides: &[LayoutGrid], old: Point, new: Point) -> bool {
    if guides.is_empty() { return false; }
    let transform = child.local_transform();
    let bounds = transform.transform_rect(child.local_bounds());
    let Some(next) = layout_guide_constrained_bounds(bounds, child.constraints, guides, old, new) else { return false; };
    if bounds == next { return true; }
    if child.affine_transform.is_none() && child.rotation.rem_euclid(360.0).abs() < GEOMETRY_EPSILON {
        if child.resize_vector_geometry(next.width, next.height) {
            child.x = next.x; child.y = next.y; child.width = next.width; child.height = next.height;
        }
    } else {
        let sx = if bounds.width > GEOMETRY_EPSILON { next.width / bounds.width } else { 1.0 };
        let sy = if bounds.height > GEOMETRY_EPSILON { next.height / bounds.height } else { 1.0 };
        let adjustment = Transform::translation(next.x, next.y) * Transform::scale(sx, sy)
            * Transform::translation(-bounds.x, -bounds.y);
        child.set_local_transform(adjustment * transform);
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Document, Editor, LayoutGridAxis, NodeKind};

    fn columns() -> LayoutGrid {
        LayoutGrid { pattern: LayoutGridPattern::Columns { axis: LayoutGridAxis {
            alignment: LayoutGridAlignment::Stretch, count: Some(3), gutter: 10.0, offset: 10.0,
        } }, ..LayoutGrid::default() }
    }

    #[test]
    fn five_constraints_follow_nearest_stretch_columns_with_frame_fallback_on_other_axis() {
        // 300 wide -> three 86 2/3 wide columns, 600 -> three 186 2/3.
        let guide = columns();
        let bands = guide.bands(300.0).unwrap();
        let x = bands.start + bands.step + 5.0;
        for (constraint, expected_x, expected_width) in [
            (Constraint::Start, x + 100.0, 20.0),
            (Constraint::End, x + 200.0, 20.0),
            (Constraint::Center, x + 150.0, 20.0),
            (Constraint::Stretch, x + 100.0, 120.0),
            (Constraint::Scale, bands.start + (bands.step + 100.0) + 5.0 * (bands.size + 100.0) / bands.size,
                20.0 * (bands.size + 100.0) / bands.size),
        ] {
            let result = layout_guide_constrained_bounds(Rect::new(x, 20.0, 20.0, 30.0),
                Constraints { horizontal: constraint, vertical: Constraint::End }, &[guide],
                Point::new(300.0, 100.0), Point::new(600.0, 160.0)).unwrap();
            assert!((result.x - expected_x).abs() < 1e-8, "{constraint:?}: {result:?}");
            assert!((result.width - expected_width).abs() < 1e-8, "{constraint:?}: {result:?}");
            assert_eq!((result.y, result.height), (80.0, 30.0));
        }
    }

    #[test]
    fn spanning_hidden_guides_keep_edge_offsets_and_first_compatible_stack_order() {
        let mut first = columns(); first.visible = false;
        let mut second = columns(); second.pattern = second.pattern.with_axis(LayoutGridAxis { count: Some(6), ..second.pattern.axis().unwrap() });
        let old = first.bands(300.0).unwrap();
        let bounds = Rect::new(15.0, 5.0, old.step + old.size - 10.0, 10.0);
        let next = layout_guide_constrained_bounds(bounds, Constraints { horizontal: Constraint::Stretch, ..Constraints::default() },
            &[first, second], Point::new(300.0, 100.0), Point::new(600.0, 100.0)).unwrap();
        assert_eq!(next.x, 15.0);
        assert!((next.width - bounds.width - 200.0).abs() < 1e-8);
        let fixed = LayoutGrid { pattern: first.pattern.with_axis(LayoutGridAxis { alignment: LayoutGridAlignment::Center, ..first.pattern.axis().unwrap() }), ..first };
        assert!(layout_guide_constrained_bounds(bounds, Constraints::default(), &[fixed, LayoutGrid::default()],
            Point::new(300.0, 100.0), Point::new(600.0, 100.0)).is_none());
    }

    #[test]
    fn transformed_child_preserves_source_geometry_and_exact_resize_history() {
        let mut document = Document::new("Guide constraints");
        let mut parent = Node::new(NodeKind::Frame, "Responsive", 25.0, 30.0, 300.0, 200.0);
        parent.rotation = 23.0; parent.set_layout_guides(&[columns()]);
        let parent = document.insert_node(1, parent).unwrap();
        let mut child = Node::new(NodeKind::Rectangle, "Sheared", 105.0, 20.0, 35.0, 25.0);
        child.rotation = 17.0;
        child.affine_transform = Some(Transform { c: 0.2, ..Transform::IDENTITY });
        child.constraints.horizontal = Constraint::Stretch;
        let id = document.insert_node(parent, child).unwrap();
        let mut editor = Editor::new(document);
        let original = editor.document.nodes[&id].clone();
        let old_bounds = original.local_transform().transform_rect(original.local_bounds());
        let expected = layout_guide_constrained_bounds(old_bounds, original.constraints, &[columns()], Point::new(300.0, 200.0), Point::new(600.0, 200.0)).unwrap();
        let before = editor.to_json().unwrap(); let history = editor.history_len();
        editor.resize_node(parent, 600.0, 200.0);
        let updated = &editor.document.nodes[&id];
        let actual = updated.local_transform().transform_rect(updated.local_bounds());
        for (a, b) in [(actual.x, expected.x), (actual.y, expected.y), (actual.width, expected.width), (actual.height, expected.height)] { assert!((a - b).abs() < 1e-7); }
        assert_eq!((updated.width, updated.height), (original.width, original.height));
        assert_eq!(editor.history_len(), history + 1);
        let after = editor.to_json().unwrap();
        assert!(editor.undo()); assert_eq!(editor.to_json().unwrap(), before);
        assert!(editor.redo()); assert_eq!(editor.to_json().unwrap(), after);
        editor.document.validate().unwrap();
    }

    #[test]
    fn guide_constraints_skip_auto_layout_flow_but_resize_absolute_and_locked_children() {
        let mut document = Document::new("Guide constraints");
        let mut parent = Node::new(NodeKind::Frame, "Responsive", 0.0, 0.0, 300.0, 200.0);
        parent.layout = Some(crate::AutoLayout::default()); parent.set_layout_guides(&[columns()]);
        let parent = document.insert_node(1, parent).unwrap();
        let mut child = Node::new(NodeKind::Rectangle, "Child", 120.0, 20.0, 20.0, 20.0);
        child.constraints.horizontal = Constraint::End;
        let flow = document.insert_node(parent, child.clone()).unwrap();
        child.layout_child.absolute = true; child.locked = true;
        let absolute = document.insert_node(parent, child).unwrap();
        document.apply_constraints(parent, 300.0, 200.0, 600.0, 200.0);
        assert_eq!(document.nodes[&flow].x, 120.0);
        assert!((document.nodes[&absolute].x - 320.0).abs() < 1e-8);
    }

    #[test]
    fn collapsed_and_million_section_guides_do_not_expand_or_create_nonfinite_geometry() {
        let mut guide = columns();
        guide.pattern = guide.pattern.with_axis(LayoutGridAxis { count: Some(1_000_000), gutter: 0.0, offset: 0.0, ..guide.pattern.axis().unwrap() });
        let result = layout_guide_constrained_bounds(Rect::new(0.5, 1.0, 0.25, 1.0), Constraints { horizontal: Constraint::Stretch, ..Constraints::default() },
            &[guide], Point::new(1.0, 10.0), Point::new(2.0, 10.0)).unwrap();
        assert!(result.is_finite());
        assert!(layout_guide_constrained_bounds(Rect::new(1.0, 1.0, 1.0, 1.0), Constraints::default(), &[columns()],
            Point::new(300.0, 100.0), Point::new(5.0, 100.0)).is_none());
    }
}
