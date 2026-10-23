//! Exercise the editor's public FerroUI brush path, pixels and retained caches.
use super::*;
use ferrodesign_core::{Fill, GradientStop};
use ferroui_base::media::imaging::RenderTargetBitmap;
use ferroui_base::platform::PixelFormat;
use ferroui_base::{FerroLocator, PixelRect, PixelSize};

fn fixture(angle: f64) -> (Editor, NodeId) {
    let mut document = Document::new("Angular pixels");
    let mut node = Node::new(NodeKind::Rectangle, "Sweep", 20.0, 10.0, 120.0, 100.0);
    node.fills = vec![Fill {
        paint: Paint::AngularGradient {
            center: Point::new(0.5, 0.5), angle,
            stops: vec![
                GradientStop { offset: 0.0, color: Color::rgb(255, 0, 0) },
                GradientStop { offset: 1.0, color: Color::rgb(0, 0, 255) },
            ],
        },
        ..Fill::solid(Color::BLACK)
    }];
    let id = document.insert_node(document.pages[0].id, node).unwrap();
    (Editor::new(document), id)
}

fn render(painter: &mut ScenePainter, editor: &Editor) -> (Vec<u8>, RenderStats) {
    let bitmap = RenderTargetBitmap::new(PixelSize::new(160, 120));
    let stats;
    {
        let mut context = bitmap.create_drawing_context();
        stats = painter.render_editor(&mut context, editor, &editor.document.pages[0].children,
            Rect::new(0.0, 0.0, 160.0, 120.0));
    }
    assert_eq!(stats.invalid_nodes, 0);
    assert_eq!(stats.unsupported_effects, 0);
    let mut pixels = vec![0; 160 * 120 * 4];
    bitmap.copy_pixels(PixelRect::new(0, 0, 160, 120), &mut pixels, 160 * 4);
    if bitmap.format() == Some(PixelFormat::BGRA8888) {
        for pixel in pixels.chunks_exact_mut(4) { pixel.swap(0, 2); }
    }
    (pixels, stats)
}

fn pixel(pixels: &[u8], x: usize, y: usize) -> &[u8] {
    let offset = (y * 160 + x) * 4;
    &pixels[offset..offset + 4]
}

#[test]
fn angular_gradient_pixels_start_above_center_and_sweep_clockwise() {
    let scope = FerroLocator::enter_scope();
    ferroui_vello::VelloPlatform::initialize();
    let (editor, _) = fixture(0.0);
    let pixels = render(&mut ScenePainter::default(), &editor).0;
    let start = pixel(&pixels, 83, 20);
    let end = pixel(&pixels, 76, 20);
    let half = pixel(&pixels, 80, 100);
    assert!(start[0] > 235 && start[2] < 20, "clockwise start: {start:?}");
    assert!(end[2] > 235 && end[0] < 20, "end before seam: {end:?}");
    assert!(half[0].abs_diff(half[2]) < 12 && half[3] == 255, "half sweep: {half:?}");
    scope.dispose();
}

#[test]
fn angular_gradient_incremental_angle_center_and_stop_edits_match_fresh_pixels() {
    let scope = FerroLocator::enter_scope();
    ferroui_vello::VelloPlatform::initialize();
    let (mut editor, id) = fixture(0.0);
    let mut painter = ScenePainter::default();
    let before = render(&mut painter, &editor).0;
    for property in 0..3 {
        editor.update_node(id, "Angular property", |node| {
            let Paint::AngularGradient { center, angle, stops } = &mut node.fills[0].paint else { unreachable!() };
            match property {
                0 => *angle = 90.0,
                1 => center.x = 0.35,
                _ => stops[0].color = Color::rgb(0, 255, 0),
            }
        });
        let (actual, stats) = render(&mut painter, &editor);
        assert_eq!(stats.prepared_nodes, 1);
        assert_eq!(stats.rebuilt_visuals, 1);
        assert_ne!(actual, before);
        assert_eq!(actual, render(&mut ScenePainter::default(), &editor).0);
    }
    let steady = render(&mut painter, &editor).1;
    assert_eq!(steady.prepared_nodes, 0, "unchanged angular brushes remain retained");
    scope.dispose();
}

#[test]
fn angular_gradient_fill_and_layer_opacity_accumulate_once() {
    let scope = FerroLocator::enter_scope();
    ferroui_vello::VelloPlatform::initialize();
    let (mut editor, id) = fixture(45.0);
    editor.update_node(id, "Translucent sweep", |node| {
        node.opacity = 0.5;
        node.fills[0].opacity = 0.5;
        let Paint::AngularGradient { stops, .. } = &mut node.fills[0].paint else { unreachable!() };
        for stop in stops { stop.color = Color::rgba(240, 60, 120, 128); }
    });
    let pixels = render(&mut ScenePainter::default(), &editor).0;
    let sample = pixel(&pixels, 100, 80);
    assert!((sample[3] as i32 - 32).abs() <= 1, "128 × 0.5 fill × 0.5 layer: {sample:?}");
    scope.dispose();
}

#[test]
fn angular_gradient_stroke_uses_native_sweep_and_retains_empty_interior() {
    let scope = FerroLocator::enter_scope();
    ferroui_vello::VelloPlatform::initialize();
    let (mut editor, id) = fixture(0.0);
    editor.update_node(id, "Angular stroke", |node| {
        node.strokes = vec![Stroke { paint: node.fills[0].paint.clone(), ..Stroke::new(Color::BLACK, 8.0) }];
        node.fills.clear();
    });
    let pixels = render(&mut ScenePainter::default(), &editor).0;
    assert_eq!(pixel(&pixels, 80, 60)[3], 0);
    let top_right = pixel(&pixels, 90, 10);
    let top_left = pixel(&pixels, 70, 10);
    assert!(top_right[0] > 220 && top_right[2] < 35, "stroke starts red: {top_right:?}");
    assert!(top_left[2] > 220 && top_left[0] < 35, "stroke finishes blue: {top_left:?}");
    scope.dispose();
}

#[test]
fn angular_gradient_center_and_outside_strokes_cover_pixels_beyond_geometry() {
    let scope = FerroLocator::enter_scope();
    ferroui_vello::VelloPlatform::initialize();
    for alignment in [StrokeAlign::Center, StrokeAlign::Outside] {
        let (mut editor, id) = fixture(0.0);
        editor.update_node(id, "Stroke alignment", |node| {
            node.y = 30.0;
            node.height = 60.0;
            node.strokes = vec![Stroke { paint: node.fills[0].paint.clone(),
                align: alignment, ..Stroke::new(Color::BLACK, 12.0) }];
            node.fills.clear();
        });
        let pixels = render(&mut ScenePainter::default(), &editor).0;
        let red = pixel(&pixels, 84, 26);
        let blue = pixel(&pixels, 75, 26);
        assert!(red[0] > 235 && red[2] < 20 && red[3] == 255, "{alignment:?} exterior red: {red:?}");
        assert!(blue[2] > 235 && blue[0] < 20 && blue[3] == 255, "{alignment:?} exterior blue: {blue:?}");
    }
    scope.dispose();
}

#[test]
fn angular_gradient_horizontal_stroke_uses_nonzero_pen_bounds() {
    let scope = FerroLocator::enter_scope();
    ferroui_vello::VelloPlatform::initialize();
    let (mut editor, id) = fixture(0.0);
    editor.update_node(id, "Horizontal angular line", |node| {
        node.kind = NodeKind::Line;
        node.y = 60.0;
        node.width = 100.0;
        node.height = 0.0;
        let mut paint = node.fills[0].paint.clone();
        if let Paint::AngularGradient { center, .. } = &mut paint { *center = Point::new(0.25, 0.75); }
        node.strokes = vec![Stroke { paint, ..Stroke::new(Color::BLACK, 20.0) }];
        node.fills.clear();
    });
    let pixels = render(&mut ScenePainter::default(), &editor).0;
    let red = pixel(&pixels, 45, 53);
    let blue = pixel(&pixels, 35, 53);
    assert!(red[0] > 230 && red[2] < 25 && red[3] == 255, "line red: {red:?}");
    assert!(blue[2] > 230 && blue[0] < 25 && blue[3] == 255, "line blue: {blue:?}");
    scope.dispose();
}
