//! Smoothed geometry must agree across retained artwork, child clips and undo.
use super::*;
use ferrodesign_core::Fill;
use ferroui_base::media::imaging::RenderTargetBitmap;
use ferroui_base::{FerroLocator, PixelRect, PixelSize};

fn render(painter: &mut ScenePainter, editor: &Editor) -> (Vec<u8>, RenderStats) {
    let target = RenderTargetBitmap::new(PixelSize::new(128, 128));
    let stats = {
        let mut context = target.create_drawing_context();
        painter.render_editor(&mut context, editor, &editor.document.pages[0].children,
            Rect::new(0.0, 0.0, 128.0, 128.0))
    };
    let mut pixels = vec![0; 128 * 128 * 4];
    target.copy_pixels(PixelRect::new(0, 0, 128, 128), &mut pixels, 128 * 4);
    assert_eq!(stats.invalid_nodes, 0);
    assert_eq!(stats.unsupported_effects, 0);
    (pixels, stats)
}

fn fixture(kind: NodeKind) -> (Editor, NodeId) {
    let mut doc = Document::new("Shared corner geometry");
    let mut node = Node::new(kind, "Shape", 12.0, 12.0, 104.0, 104.0);
    node.corner_radius = 24.0;
    node.corner_radii = Some([24.0, 32.0, 18.0, 12.0]);
    node.corner_smoothing = 0.6;
    node.fills = vec![Fill::solid(Color::rgb(25, 90, 210))];
    let id = doc.insert_node(doc.pages[0].id, node).unwrap();
    (Editor::new(doc), id)
}

#[test]
fn native_smoothed_shape_equals_its_public_cubic_outline() {
    let scope = FerroLocator::enter_scope();
    ferroui_vello::VelloPlatform::initialize();
    let (editor, id) = fixture(NodeKind::Rectangle);
    let pixels = render(&mut ScenePainter::default(), &editor).0;
    let mut doc = editor.document.clone();
    let node = doc.nodes.get_mut(&id).unwrap();
    node.path = ferrodesign_core::corner_path(node);
    node.kind = NodeKind::Vector;
    node.corner_smoothing = 0.0;
    node.corner_radius = 0.0;
    node.corner_radii = None;
    let vector = render(&mut ScenePainter::default(), &Editor::new(doc)).0;
    assert_eq!(pixels, vector);
    assert!(pixels.chunks_exact(4).any(|p| p[3] == 255));
    scope.dispose();
}

#[test]
fn smoothed_child_clip_uses_the_same_outline_as_the_parent_fill() {
    let scope = FerroLocator::enter_scope();
    ferroui_vello::VelloPlatform::initialize();
    let (editor, id) = fixture(NodeKind::Frame);
    let expected = render(&mut ScenePainter::default(), &editor).0;
    let mut doc = editor.document.clone();
    doc.nodes.get_mut(&id).unwrap().fills.clear();
    let child = Node::new(NodeKind::Rectangle, "Oversized child", -20.0, -20.0, 144.0, 144.0)
        .with_fill(Color::rgb(25, 90, 210));
    doc.insert_node(id, child).unwrap();
    let clipped = render(&mut ScenePainter::default(), &Editor::new(doc)).0;
    assert!(expected.iter().zip(&clipped).all(|(a, b)| a.abs_diff(*b) <= 2),
        "A child clip must follow the actual smooth frame outline");
    scope.dispose();
}

#[test]
fn smoothing_invalidates_one_retained_visual_and_history_restores_pixels() {
    let scope = FerroLocator::enter_scope();
    ferroui_vello::VelloPlatform::initialize();
    let (mut editor, id) = fixture(NodeKind::Rectangle);
    let mut painter = ScenePainter::default();
    let before = render(&mut painter, &editor).0;
    let repeated = render(&mut painter, &editor);
    assert_eq!(before, repeated.0);
    assert_eq!(repeated.1.rebuilt_visuals, 0);
    assert_eq!(editor.set_corner_smoothing(&[id], 0.0).unwrap(), 1);
    let after = render(&mut painter, &editor);
    assert_eq!(after.1.rebuilt_visuals, 1);
    assert_ne!(before, after.0);
    assert!(editor.undo());
    assert_eq!(render(&mut painter, &editor).0, before);
    assert!(editor.redo());
    assert_eq!(render(&mut painter, &editor).0, after.0);
    scope.dispose();
}

// Evaluate Bernstein polynomials at fixed parameter intervals. This reference
// deliberately uses neither VectorPath::flattened nor the renderer's Kurbo
// flattening, so a shared flattening error cannot make the coverage test pass.
fn dense_corner_outline(path: &ferrodesign_core::VectorPath, steps: usize) -> Vec<Point> {
    let mut points = Vec::new();
    let mut current = Point::new(0.0, 0.0);
    for segment in &path.segments {
        match *segment {
            PathSegment::MoveTo { point } | PathSegment::LineTo { point } => {
                points.push(point);
                current = point;
            }
            PathSegment::CubicTo { control1, control2, point } => {
                for step in 1..=steps {
                    let t = step as f64 / steps as f64;
                    let u = 1.0 - t;
                    points.push(current * (u * u * u)
                        + control1 * (3.0 * u * u * t)
                        + control2 * (3.0 * u * t * t)
                        + point * (t * t * t));
                }
                current = point;
            }
            PathSegment::Close => {}
            _ => panic!("The smooth-corner fixture contains only lines and cubics"),
        }
    }
    points
}

// Sutherland-Hodgman clipping against one pixel edge. Integrating the resulting
// polygon's area gives coverage, independently of either rasterizer's AA rule.
fn clip_coverage_polygon(points: &[Point], axis: usize, edge: f64, greater: bool) -> Vec<Point> {
    let coordinate = |point: Point| if axis == 0 { point.x } else { point.y };
    let inside = |point: Point| if greater { coordinate(point) >= edge } else { coordinate(point) <= edge };
    let Some(&last) = points.last() else { return Vec::new(); };
    let mut result = Vec::new();
    let mut previous = last;
    for &point in points {
        if inside(previous) != inside(point) {
            let t = (edge - coordinate(previous)) / (coordinate(point) - coordinate(previous));
            result.push(previous + (point - previous) * t);
        }
        if inside(point) { result.push(point); }
        previous = point;
    }
    result
}

fn dense_corner_coverage(path: &ferrodesign_core::VectorPath, steps: usize) -> Vec<f64> {
    let polygon = dense_corner_outline(path, steps);
    let mut coverage = Vec::with_capacity(200 * 140);
    for y in 0..140 {
        let row = clip_coverage_polygon(&polygon, 1, y as f64, true);
        let row = clip_coverage_polygon(&row, 1, y as f64 + 1.0, false);
        for x in 0..200 {
            let cell = clip_coverage_polygon(&row, 0, x as f64, true);
            let cell = clip_coverage_polygon(&cell, 0, x as f64 + 1.0, false);
            let origin = Point::new(x as f64, y as f64);
            let twice_area: f64 = cell.iter().zip(cell.iter().cycle().skip(1))
                .map(|(a, b)| {
                    let a = *a - origin;
                    let b = *b - origin;
                    a.x * b.y - a.y * b.x
                }).sum();
            coverage.push((twice_area.abs() * 0.5).clamp(0.0, 1.0) * 255.0);
        }
    }
    coverage
}

fn exported_corner_alpha(painter: &mut ScenePainter, editor: &Editor, id: NodeId) -> Vec<u8> {
    use ferroui_base::media::imaging::{BitmapEncoderOptions, PngBitmapEncoderOptions};
    let bounds = painter.rendered_editor_bounds(editor, &[id]).unwrap();
    assert_eq!(bounds, Rect::new(40.0, 40.0, 200.0, 140.0));
    let target = RenderTargetBitmap::new(PixelSize::new(200, 140));
    {
        let mut context = target.create_drawing_context();
        let transform = context.push_transform(Matrix::create_translation(-bounds.x, -bounds.y));
        painter.set_raster_scale(1.0);
        let stats = painter.render_editor(&mut context, editor, &[id], bounds);
        assert_eq!(stats.invalid_nodes, 0);
        assert_eq!(stats.unsupported_effects, 0);
        context.pop(transform);
        context.dispose();
    }
    let mut pixels = vec![0; 200 * 140 * 4];
    target.copy_pixels(PixelRect::new(0, 0, 200, 140), &mut pixels, 200 * 4);
    let mut encoded = Vec::new();
    target.save(&mut encoded, &BitmapEncoderOptions::Png(PngBitmapEncoderOptions::default())).unwrap();
    let decoded = Bitmap::from_stream(&mut Cursor::new(encoded)).unwrap();
    let mut roundtrip = vec![0; pixels.len()];
    decoded.copy_pixels(PixelRect::new(0, 0, 200, 140), &mut roundtrip, 200 * 4);
    let alpha: Vec<_> = pixels.chunks_exact(4).map(|pixel| pixel[3]).collect();
    assert_eq!(alpha, roundtrip.chunks_exact(4).map(|pixel| pixel[3]).collect::<Vec<_>>(),
        "Public PNG encoding/decoding must preserve rendered alpha coverage");
    target.dispose();
    alpha
}

#[test]
fn native_smoothed_export_matches_independent_dense_curve_coverage_after_history() {
    let _dispatcher = ferroui_base::threading::Dispatcher::unit_test_scope();
    let scope = FerroLocator::enter_scope();
    ferroui_vello::VelloPlatform::initialize();
    // Exact browser export fixture: nonzero world placement, cropped to its
    // 200x140 output, with one opaque fill and no stroke, clip or effect.
    let mut document = Document::new("Independent smooth-corner coverage");
    let mut node = Node::new(NodeKind::Rectangle, "Smooth", 40.0, 40.0, 200.0, 140.0);
    node.corner_radius = 40.0;
    node.fills = vec![Fill::solid(Color::rgb(77, 63, 230))];
    let id = document.insert_node(document.pages[0].id, node).unwrap();
    let mut editor = Editor::new(document);
    let mut painter = ScenePainter::default();
    let before = exported_corner_alpha(&mut painter, &editor, id);
    assert_eq!(editor.set_corner_smoothing(&[id], 0.6).unwrap(), 1);
    let after = exported_corner_alpha(&mut painter, &editor, id);
    assert_ne!(before, after);
    assert!(editor.undo());
    assert_eq!(exported_corner_alpha(&mut painter, &editor, id), before);
    assert!(editor.redo());
    assert_eq!(exported_corner_alpha(&mut painter, &editor, id), after);
    assert_eq!(exported_corner_alpha(&mut ScenePainter::default(), &editor, id), after,
        "A fresh export and the retained post-redo shape must have identical coverage");

    let path = ferrodesign_core::corner_path(&editor.document.nodes[&id]);
    let coarse = dense_corner_coverage(&path, 256);
    let reference = dense_corner_coverage(&path, 512);
    let convergence = coarse.iter().zip(&reference)
        .map(|(a, b)| (a - b).abs()).fold(0.0_f64, f64::max);
    assert!(convergence < 0.1, "Dense reference must converge below 0.1 alpha unit: {convergence}");
    let mut differing = 0;
    let mut max_delta = 0.0_f64;
    let mut absolute_area = 0.0;
    let mut samples = Vec::new();
    for (index, (&alpha, &expected)) in after.iter().zip(&reference).enumerate() {
        let delta = (f64::from(alpha) - expected.round()).abs();
        max_delta = max_delta.max(delta);
        absolute_area += delta / 255.0;
        if delta > 12.0 {
            differing += 1;
            if samples.len() < 12 { samples.push((index % 200, index / 200, alpha, expected)); }
        }
    }
    let native_area = after.iter().map(|&value| f64::from(value) / 255.0).sum::<f64>();
    let reference_area = reference.iter().map(|value| value / 255.0).sum::<f64>();
    // Keep the existing browser criterion, now against mathematical coverage
    // rather than another decoder. A failure carries bounded scalar evidence
    // to distinguish CPU rasterization from SVG decoding or retained geometry.
    assert!(differing <= 48,
        "Dense smooth-corner coverage: differing_over_12={differing}, max_delta={max_delta}, native_area={native_area}, reference_area={reference_area}, absolute_area={absolute_area}, convergence={convergence}, samples={samples:?}");
    scope.dispose();
}
