use super::*;
use ferrodesign_core::{Fill, GradientStop, NetworkEdgeRef, VectorNetwork, VectorRegionId};
use ferroui_base::media::imaging::RenderTargetBitmap;
use ferroui_base::platform::{IPlatformRenderInterface, PixelFormat};
use ferroui_base::{FerroLocator, PixelRect, PixelSize};

fn backend() -> Rc<dyn ferroui_base::reactive::IDisposable> {
    let scope = FerroLocator::enter_scope();
    FerroLocator::current_mutable().bind::<dyn IPlatformRenderInterface>()
        .to_constant(Rc::new(ferroui_vello::PlatformRenderInterface::default()));
    scope
}

fn square(network: &mut VectorNetwork, x: f64, y: f64, width: f64, height: f64) -> VectorRegionId {
    let vertices = [Point::new(x, y), Point::new(x + width, y), Point::new(x + width, y + height), Point::new(x, y + height)]
        .map(|point| network.add_vertex(point).unwrap());
    let edges = (0..4).map(|index| NetworkEdgeRef::forward(network.connect(vertices[index], vertices[(index + 1) % 4]).unwrap())).collect();
    network.add_region(vec![edges]).unwrap()
}

fn fixture() -> (Document, NodeId, VectorRegionId) {
    let mut network = VectorNetwork::default();
    square(&mut network, 10.0, 10.0, 100.0, 80.0);
    let red = square(&mut network, 30.0, 20.0, 100.0, 50.0);
    let clear = square(&mut network, 60.0, 30.0, 30.0, 30.0);
    network.regions.get_mut(&red).unwrap().fills = Some(vec![
        Fill::solid(Color::rgba(255, 0, 0, 128)), Fill::solid(Color::rgb(0, 220, 0)),
    ]);
    network.regions.get_mut(&clear).unwrap().fills = Some(Vec::new());
    let mut node = Node::new(NodeKind::Vector, "Regions", 0.0, 0.0, 180.0, 100.0);
    node.vector_network = Some(network);
    node.fills = vec![Fill::solid(Color::rgba(20, 70, 210, 128))];
    let mut document = Document::new("Region SVG pixels");
    let id = document.insert_node(1, node).unwrap();
    (document, id, red)
}

fn pixels(document: &Document) -> Vec<u8> {
    let bitmap = RenderTargetBitmap::new(PixelSize::new(180, 140));
    {
        let mut context = bitmap.create_drawing_context();
        let stats = ScenePainter::default().render(&mut context, document, &document.pages[0].children, Rect::new(0.0, 0.0, 180.0, 140.0));
        assert_eq!(stats.invalid_nodes, 0);
        assert_eq!(stats.unsupported_effects, 0);
    }
    let mut bytes = vec![0; 180 * 140 * 4];
    bitmap.copy_pixels(PixelRect::new(0, 0, 180, 140), &mut bytes, 180 * 4);
    if bitmap.format() == Some(PixelFormat::BGRA8888) {
        for pixel in bytes.chunks_exact_mut(4) { pixel.swap(0, 2); }
    }
    bytes
}

fn equivalent(before: &[u8], after: &[u8]) {
    let bad = before.chunks_exact(4).zip(after.chunks_exact(4))
        .filter(|(a, b)| a.iter().zip(*b).any(|(a, b)| a.abs_diff(*b) > 4)).count();
    assert!(bad < 180 * 140 / 100, "{bad} pixels differ by more than 4/255 across native SVG reimport");
}

#[test]
fn region_svg_native_roundtrip_retains_translucent_replacement_empty_faces_and_paint_order() {
    let scope = backend();
    let (document, _, _) = fixture();
    let imported = Document::from_svg(&document.export_svg(1, &[]), "Imported").unwrap();
    let original = pixels(&document);
    let restored = pixels(&imported);
    equivalent(&original, &restored);
    let sample = |x: usize, y: usize| &restored[(y * 180 + x) * 4..(y * 180 + x) * 4 + 4];
    assert_eq!(sample(75, 45)[3], 0, "explicit empty region must cut every underlying fill");
    assert!(sample(40, 40)[0] > 100 && sample(40, 40)[1] > 80);
    assert!((i32::from(sample(20, 40)[3]) - 128).abs() <= 1);
    scope.dispose();
}

#[test]
fn region_svg_native_roundtrip_keeps_original_gradient_bounds_after_replacement() {
    let scope = backend();
    let (mut document, id, region) = fixture();
    let gradient = Fill { paint: Paint::LinearGradient { start: Point::new(0.0, 0.0), end: Point::new(1.0, 0.0),
        stops: vec![GradientStop { offset: 0.0, color: Color::rgb(255, 0, 0) }, GradientStop { offset: 1.0, color: Color::rgb(0, 0, 255) }] }, visible: true, opacity: 0.8 };
    let node = document.nodes.get_mut(&id).unwrap();
    node.fills = vec![gradient.clone()];
    node.vector_network.as_mut().unwrap().regions.get_mut(&region).unwrap().fills = Some(vec![gradient]);
    let imported = Document::from_svg(&document.export_svg(1, &[]), "Gradients").unwrap();
    equivalent(&pixels(&document), &pixels(&imported));
    scope.dispose();
}

#[test]
fn region_svg_native_roundtrip_keeps_vector_mask_coverage_and_ignores_transparent_paints() {
    let scope = backend();
    let (mut document, id, _) = fixture();
    let node = document.nodes.get_mut(&id).unwrap();
    node.is_mask = true;
    node.mask_type = MaskType::Vector;
    node.fills = vec![Fill::solid(Color::TRANSPARENT)];
    document.insert_node(1, Node::new(NodeKind::Rectangle, "Masked content", 0.0, 0.0, 180.0, 100.0).with_fill(Color::rgb(20, 60, 220))).unwrap();
    let source = document.export_svg(1, &[]);
    let imported = Document::from_svg(&source, "Vector mask").unwrap();
    equivalent(&pixels(&document), &pixels(&imported));
    scope.dispose();
}

#[test]
fn svg_native_roundtrip_preserves_standard_fill_and_stroke_stack_order() {
    let scope = backend();
    let mut document = Document::new("Stack order");
    let mut node = Node::new(NodeKind::Rectangle, "Stacked paints", 0.0, 0.0, 180.0, 100.0);
    node.fills = vec![Fill::solid(Color::rgba(255, 0, 0, 128)), Fill::solid(Color::rgb(0, 220, 0))];
    let mut top_stroke = Stroke::new(Color::rgba(0, 0, 255, 128), 8.0);
    top_stroke.align = StrokeAlign::Inside;
    let mut lower_stroke = Stroke::new(Color::rgb(255, 255, 0), 8.0);
    lower_stroke.align = StrokeAlign::Inside;
    node.strokes = vec![top_stroke, lower_stroke];
    document.insert_node(1, node).unwrap();
    let imported = Document::from_svg(&document.export_svg(1, &[]), "Stack order import").unwrap();
    let original = pixels(&document);
    equivalent(&original, &pixels(&imported));
    let sample = |x: usize, y: usize| &original[(y * 180 + x) * 4..(y * 180 + x) * 4 + 4];
    assert!(sample(40, 40)[0] > 100 && sample(40, 40)[1] > 80);
    assert!(sample(3, 40).iter().take(3).all(|channel| *channel > 100));
    scope.dispose();
}
