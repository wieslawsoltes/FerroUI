//! Pixel and cache behavior through the public FerroUI drawing/geometry APIs.
use super::*;
use ferrodesign_core::{Fill, MaskType, StrokeWidthPoint, StrokeWidthProfile};
use ferroui_base::media::imaging::RenderTargetBitmap;
use ferroui_base::platform::PixelFormat;
use ferroui_base::{FerroLocator, PixelRect, PixelSize};

const W: i32 = 180;
const H: i32 = 150;
fn fixture() -> (Editor, NodeId) {
    let mut document = Document::new("Variable ink pixels");
    let mut line = Node::new(NodeKind::Line, "Wedge", 25.0, 75.0, 110.0, 0.0);
    line.fills.clear();
    line.strokes = vec![Stroke { width_profile: StrokeWidthProfile::Wedge,
        ..Stroke::new(Color::rgb(240, 20, 40), 40.0) }];
    let id = document.insert_node(1, line).unwrap();
    (Editor::new(document), id)
}
fn render(painter: &mut ScenePainter, editor: &Editor) -> (Vec<u8>, RenderStats) {
    let bitmap = RenderTargetBitmap::new(PixelSize::new(W, H));
    let stats = {
        let mut context = bitmap.create_drawing_context();
        painter.render_editor(&mut context, editor, &editor.document.pages[0].children,
            Rect::new(0.0, 0.0, W as f64, H as f64))
    };
    assert_eq!(stats.invalid_nodes, 0);
    assert_eq!(stats.unsupported_effects, 0);
    let mut pixels = vec![0; (W * H * 4) as usize];
    bitmap.copy_pixels(PixelRect::new(0, 0, W, H), &mut pixels, W * 4);
    if bitmap.format() == Some(PixelFormat::BGRA8888) {
        for pixel in pixels.chunks_exact_mut(4) { pixel.swap(0, 2); }
    }
    (pixels, stats)
}
fn pixel(pixels: &[u8], x: usize, y: usize) -> &[u8] {
    let at = (y * W as usize + x) * 4; &pixels[at..at + 4]
}

#[test]
fn variable_stroke_native_pixels_follow_profile_and_retain_geometry_on_placement() {
    let _dispatcher = ferroui_base::threading::Dispatcher::unit_test_scope();
    let scope = FerroLocator::enter_scope(); ferroui_vello::VelloPlatform::initialize();
    let (mut editor, id) = fixture();
    let mut painter = ScenePainter::default();
    let before = render(&mut painter, &editor).0;
    assert_eq!(pixel(&before, 35, 90)[3], 255, "wide beginning must be painted");
    assert_eq!(pixel(&before, 125, 90)[3], 0, "tapered end must stay empty");
    let retained = render(&mut painter, &editor).1;
    assert_eq!(retained.prepared_nodes, 0);
    assert_eq!(retained.rebuilt_visuals, 0);
    editor.update_node(id, "Move", |node| node.x += 5.0);
    let (moved, stats) = render(&mut painter, &editor);
    assert_eq!(stats.rebuilt_visuals, 0, "placement reuses its prepared public geometry");
    assert_eq!(moved, render(&mut ScenePainter::default(), &editor).0);
    editor.set_stroke_width_profile(&[id], 0, StrokeWidthProfile::Eye).unwrap();
    let (eye, stats) = render(&mut painter, &editor);
    assert_eq!(stats.rebuilt_visuals, 1);
    assert_ne!(eye, moved);
    assert_eq!(eye, render(&mut ScenePainter::default(), &editor).0);
    assert!(editor.undo());
    assert_eq!(render(&mut painter, &editor).0, moved);
    scope.dispose();
}

#[test]
fn variable_stroke_aligned_band_clips_once_and_composites_alpha_once() {
    let _dispatcher = ferroui_base::threading::Dispatcher::unit_test_scope();
    let scope = FerroLocator::enter_scope(); ferroui_vello::VelloPlatform::initialize();
    for align in [StrokeAlign::Inside, StrokeAlign::Outside] {
        let mut document = Document::new("Aligned profile");
        let mut node = Node::new(NodeKind::Rectangle, "Wide rectangle", 40.0, 40.0, 100.0, 70.0);
        node.fills.clear(); node.opacity = 0.5;
        node.strokes = vec![Stroke { align, opacity: 0.5,
            width_profile: StrokeWidthProfile::Custom { points: vec![StrokeWidthPoint::new(0.5, 2.0)] },
            ..Stroke::new(Color::rgba(220, 30, 60, 128), 10.0) }];
        document.insert_node(1, node).unwrap();
        let editor = Editor::new(document); let pixels = render(&mut ScenePainter::default(), &editor).0;
        let (inside, outside) = (pixel(&pixels, 90, 57), pixel(&pixels, 90, 23));
        let (painted, clear) = if align == StrokeAlign::Inside { (inside, outside) } else { (outside, inside) };
        assert!((i32::from(painted[3]) - 32).abs() <= 1, "128 * half stroke * half layer: {painted:?}");
        assert_eq!(clear[3], 0, "alignment clips the opposite side");
        assert_eq!(pixel(&pixels, 90, 75)[3], 0, "double width is not applied twice");
    }
    scope.dispose();
}

#[test]
fn variable_stroke_vector_mask_uses_profiled_ink_and_refreshes_on_width_change() {
    let _dispatcher = ferroui_base::threading::Dispatcher::unit_test_scope();
    let scope = FerroLocator::enter_scope(); ferroui_vello::VelloPlatform::initialize();
    let (mut editor, id) = fixture();
    let node = editor.document.nodes.get_mut(&id).unwrap(); node.is_mask = true; node.mask_type = MaskType::Vector;
    let mut fill = Node::new(NodeKind::Rectangle, "Masked content", 0.0, 0.0, W as f64, H as f64);
    fill.fills = vec![Fill::solid(Color::rgb(30, 180, 80))]; fill.strokes.clear();
    editor.document.insert_node(1, fill).unwrap();
    let mut painter = ScenePainter::default(); let pixels = render(&mut painter, &editor).0;
    assert_eq!(pixel(&pixels, 35, 90), &[30, 180, 80, 255]);
    assert_eq!(pixel(&pixels, 125, 90)[3], 0);
    editor.set_stroke_width_profile(&[id], 0, StrokeWidthProfile::Eye).unwrap();
    let next = render(&mut painter, &editor).0;
    assert_ne!(next, pixels);
    assert_eq!(next, render(&mut ScenePainter::default(), &editor).0);
    scope.dispose();
}
