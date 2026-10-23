//! Exported XAML must load through FerroUI and reproduce the native artwork.
use super::*;
use crate::renderer::ScenePainter;
use ferrodesign_core::{Document, FillRule, NetworkEdgeRef, Rect, VectorNetwork, VectorRegionId};
use ferroui_base::media::imaging::RenderTargetBitmap;
use ferroui_base::platform::PixelFormat;
use ferroui_base::{FerroLocator, PixelRect, PixelSize, Size};
use ferroui_controls::Canvas;
use ferroui_markup_xaml_loader::FerroRuntimeXamlLoader;

const WIDTH: i32 = 180;
const HEIGHT: i32 = 140;

fn square(network: &mut VectorNetwork, x: f64, y: f64, width: f64, height: f64) -> VectorRegionId {
    let vertices = [Point::new(x, y), Point::new(x + width, y),
        Point::new(x + width, y + height), Point::new(x, y + height)]
        .map(|point| network.add_vertex(point).unwrap());
    let edges = (0..4).map(|index| NetworkEdgeRef::forward(
        network.connect(vertices[index], vertices[(index + 1) % 4]).unwrap())).collect();
    network.add_region(vec![edges]).unwrap()
}

fn fixture(rule: FillRule) -> (Document, NodeId, VectorRegionId) {
    let mut network = VectorNetwork { fill_rule: rule, ..Default::default() };
    square(&mut network, 10.0, 10.0, 100.0, 80.0);
    let red = square(&mut network, 30.0, 20.0, 100.0, 50.0);
    let clear = square(&mut network, 60.0, 30.0, 30.0, 30.0);
    network.regions.get_mut(&red).unwrap().fills = Some(vec![
        Fill::solid(Color::rgba(255, 0, 0, 128)), Fill::solid(Color::rgb(0, 220, 0)),
    ]);
    network.regions.get_mut(&clear).unwrap().fills = Some(Vec::new());
    let mut node = Node::new(NodeKind::Vector, "Region artwork", 0.0, 0.0, 160.0, 110.0);
    node.vector_network = Some(network);
    node.fills = vec![Fill::solid(Color::rgba(20, 70, 210, 128))];
    let mut document = Document::new("Region XAML pixels");
    let id = document.insert_node(1, node).unwrap();
    (document, id, red)
}

fn bytes(bitmap: &RenderTargetBitmap) -> Vec<u8> {
    let mut bytes = vec![0; (WIDTH * HEIGHT * 4) as usize];
    bitmap.copy_pixels(PixelRect::new(0, 0, WIDTH, HEIGHT), &mut bytes, WIDTH * 4);
    if bitmap.format() == Some(PixelFormat::BGRA8888) {
        for pixel in bytes.chunks_exact_mut(4) { pixel.swap(0, 2); }
    }
    bytes
}

fn render(document: &Document, id: NodeId) -> (Vec<u8>, Vec<u8>) {
    let native = RenderTargetBitmap::new(PixelSize::new(WIDTH, HEIGHT));
    {
        let mut context = native.create_drawing_context();
        let stats = ScenePainter::default().render(&mut context, document, &[id],
            Rect::new(0.0, 0.0, WIDTH as f64, HEIGHT as f64));
        assert_eq!(stats.invalid_nodes, 0);
        assert_eq!(stats.unsupported_effects, 0);
    }
    ferroui_markup_xaml::register_types();
    let xaml = export_xaml(document, &[id]).unwrap();
    let loaded = FerroRuntimeXamlLoader::load(&xaml, None, None, None, false)
        .unwrap_or_else(|error| panic!("Exported native XAML must load: {error}\n{xaml}"));
    let canvas = from_markup_value::<Ref<Canvas>>(&Some(loaded)).expect("root Canvas");
    canvas.measure(Size::new(WIDTH as f64, HEIGHT as f64));
    canvas.arrange(ferroui_base::Rect::new(0.0, 0.0, WIDTH as f64, HEIGHT as f64));
    let exported = RenderTargetBitmap::new(PixelSize::new(WIDTH, HEIGHT));
    exported.render(&canvas.upcast());
    (bytes(&native), bytes(&exported))
}

fn equivalent(native: &[u8], exported: &[u8]) {
    let bad = native.chunks_exact(4).zip(exported.chunks_exact(4))
        .filter(|(a, b)| a.iter().zip(*b).any(|(a, b)| a.abs_diff(*b) > 4)).count();
    assert!(bad < (WIDTH * HEIGHT / 100) as usize,
        "{bad} pixels differ by more than 4/255 across actual FerroUI XAML export/load");
}

#[test]
fn inherited_only_network_xaml_keeps_translucent_curved_edge_coverage() {
    let _dispatcher = ferroui_base::threading::Dispatcher::unit_test_scope();
    let scope = FerroLocator::enter_scope();
    ferroui_vello::VelloPlatform::initialize();
    for rule in [FillRule::NonZero, FillRule::EvenOdd] {
        let mut network = VectorNetwork { fill_rule: rule, ..Default::default() };
        let region = square(&mut network, 18.25, 24.75, 126.5, 83.5);
        let edge = network.regions[&region].contours[0][0].edge;
        network.edges.get_mut(&edge).unwrap().curve = core::NetworkCurve::Cubic {
            control1: Point::new(39.5, 1.25), control2: Point::new(122.75, 6.5),
        };
        square(&mut network, 52.25, 46.75, 43.5, 30.5);
        let mut node = Node::new(NodeKind::Vector, "Inherited curved network", 0.0, 0.0, 170.0, 130.0);
        node.vector_network = Some(network);
        node.fills = vec![Fill::solid(Color::rgba(40, 90, 220, 128))];
        let mut document = Document::new("Legacy inherited XAML pixels");
        let id = document.insert_node(1, node).unwrap();
        let xaml = export_xaml(&document, &[id]).unwrap();
        assert!(!xaml.contains(" Clip="), "an inherited-only fill needs no self clip");
        let (native, exported) = render(&document, id);
        equivalent(&native, &exported);
        let alpha_difference = native.chunks_exact(4).zip(exported.chunks_exact(4))
            .map(|(a, b)| a[3].abs_diff(b[3])).max().unwrap();
        assert!(alpha_difference <= 1, "curved edge alpha changed by {alpha_difference}/255");
    }
    scope.dispose();
}

#[test]
fn network_xaml_loads_and_retains_replacement_cutouts_paint_order_and_layer_alpha() {
    let _dispatcher = ferroui_base::threading::Dispatcher::unit_test_scope();
    let scope = FerroLocator::enter_scope();
    ferroui_vello::VelloPlatform::initialize();
    for rule in [FillRule::NonZero, FillRule::EvenOdd] {
        let (mut document, id, _) = fixture(rule);
        document.nodes.get_mut(&id).unwrap().opacity = 0.75;
        let (native, exported) = render(&document, id);
        equivalent(&native, &exported);
        let alpha = |x: usize, y: usize| exported[(y * WIDTH as usize + x) * 4 + 3];
        assert_eq!(alpha(75, 45), 0, "an empty explicit stack must cut every earlier fill");
        assert!((i32::from(alpha(20, 40)) - 96).abs() <= 1);
        assert!((i32::from(alpha(40, 40)) - 191).abs() <= 1);
    }
    scope.dispose();
}

#[test]
fn network_xaml_all_native_gradients_keep_their_original_bounds_and_affine_transform() {
    let _dispatcher = ferroui_base::threading::Dispatcher::unit_test_scope();
    let scope = FerroLocator::enter_scope();
    ferroui_vello::VelloPlatform::initialize();
    let stops = vec![GradientStop { offset: 0.0, color: Color::rgba(255, 0, 0, 160) },
        GradientStop { offset: 0.5, color: Color::rgb(0, 255, 0) },
        GradientStop { offset: 0.5, color: Color::rgb(0, 0, 255) },
        GradientStop { offset: 1.0, color: Color::BLACK }];
    for paint in [
        Paint::LinearGradient { start: Point::new(0.0, 0.0), end: Point::new(1.0, 0.0), stops: stops.clone() },
        Paint::RadialGradient { center: Point::new(0.3, 0.6), radius: 0.7, stops: stops.clone() },
        Paint::AngularGradient { center: Point::new(0.25, 0.75), angle: 35.0, stops },
    ] {
        let (mut document, id, region) = fixture(FillRule::NonZero);
        let node = document.nodes.get_mut(&id).unwrap();
        let fill = Fill { paint, opacity: 0.8, visible: true };
        node.fills = vec![fill.clone()];
        node.vector_network.as_mut().unwrap().regions.get_mut(&region).unwrap().fills = Some(vec![fill]);
        node.set_local_transform(core::Transform { a: 0.9, b: 0.12, c: -0.08, d: 1.05, e: 14.0, f: 9.0 });
        let (native, exported) = render(&document, id);
        equivalent(&native, &exported);
    }
    scope.dispose();
}

fn width_profile() -> core::StrokeWidthProfile {
    core::StrokeWidthProfile::Custom { points: vec![
        core::StrokeWidthPoint::new(0.0, 1.0),
        core::StrokeWidthPoint::new(0.31, 1.7),
        core::StrokeWidthPoint::new(0.72, 0.55),
        core::StrokeWidthPoint::new(1.0, 1.0),
    ] }
}

fn gradient_paints() -> Vec<Paint> {
    let stops = vec![
        GradientStop { offset: 0.0, color: Color::rgba(240, 40, 20, 180) },
        GradientStop { offset: 0.45, color: Color::rgb(20, 180, 80) },
        GradientStop { offset: 0.45, color: Color::rgb(30, 70, 230) },
        GradientStop { offset: 1.0, color: Color::BLACK },
    ];
    vec![
        Paint::LinearGradient { start: Point::new(0.1, 0.0), end: Point::new(0.8, 1.0), stops: stops.clone() },
        Paint::RadialGradient { center: Point::new(0.25, 0.65), radius: 0.8, stops: stops.clone() },
        Paint::AngularGradient { center: Point::new(0.65, 0.25), angle: 37.0, stops },
    ]
}

#[test]
fn variable_width_xaml_presets_custom_points_caps_and_mixed_stack_preserve_native_pixels() {
    let _dispatcher = ferroui_base::threading::Dispatcher::unit_test_scope();
    let scope = FerroLocator::enter_scope();
    ferroui_vello::VelloPlatform::initialize();
    for (index, profile) in [core::StrokeWidthProfile::Wedge, core::StrokeWidthProfile::Taper,
        core::StrokeWidthProfile::QuarterTaper, core::StrokeWidthProfile::Eye,
        core::StrokeWidthProfile::MirroredTaper, width_profile()].into_iter().enumerate() {
        let mut node = Node::new(NodeKind::Line, "Profiled line", 0.0, 0.0, 120.0,
            if index % 2 == 0 { 0.0 } else { 30.0 });
        node.fills.clear();
        node.opacity = 0.7;
        node.set_local_transform(core::Transform { a: -0.9, b: 0.15, c: 0.1, d: 1.0, e: 150.0, f: 35.0 });
        node.strokes = vec![
            Stroke { width_profile: profile, opacity: 0.8,
                cap: [core::StrokeCap::Butt, core::StrokeCap::Round, core::StrokeCap::Square][index % 3],
                ..Stroke::new(Color::rgba(230, 30, 60, 190), 18.0) },
            Stroke { cap: core::StrokeCap::Round, ..Stroke::new(Color::rgb(20, 80, 220), 3.0) },
        ];
        let mut document = Document::new("Profiled XAML line");
        let page = document.pages[0].id;
        let id = document.insert_node(page, node).unwrap();
        let xaml = export_xaml(&document, &[id]).unwrap();
        assert!(xaml.contains("<Canvas.OpacityMask>"));
        assert!(xaml.contains(&format!("x:Key=\"Profile{id}Width0\"")));
        assert!(xaml.contains("StrokeThickness=\"3\""), "uniform stack entry stays native");
        let (native, exported) = render(&document, id);
        assert!(native.chunks_exact(4).filter(|pixel| pixel[3] != 0).count() > 300);
        equivalent(&native, &exported);
    }
    scope.dispose();
}

#[test]
fn aligned_variable_width_xaml_keeps_raw_band_gradient_bounds_and_clips_only_once() {
    let _dispatcher = ferroui_base::threading::Dispatcher::unit_test_scope();
    let scope = FerroLocator::enter_scope();
    ferroui_vello::VelloPlatform::initialize();
    for align in [core::StrokeAlign::Center, core::StrokeAlign::Inside, core::StrokeAlign::Outside] {
        for paint in gradient_paints() {
            let mut node = Node::new(NodeKind::Rectangle, "Gradient border", 35.0, 35.0, 110.0, 70.0);
            node.fills.clear();
            node.corner_radii = Some([22.0, 12.0, 18.0, 9.0]);
            node.corner_smoothing = 0.7;
            node.opacity = 0.65;
            node.strokes = vec![Stroke { paint, align, opacity: 0.8, width_profile: width_profile(),
                ..Stroke::new(Color::BLACK, 9.0) }];
            let mut document = Document::new("Aligned XAML gradient");
            let page = document.pages[0].id;
            let id = document.insert_node(page, node).unwrap();
            let xaml = export_xaml(&document, &[id]).unwrap();
            let raw_path = format!("Data=\"{{StaticResource Profile{id}Width0}}\"");
            assert!(xaml.contains(&raw_path), "alignment must not replace the raw band used by relative brushes");
            if align == core::StrokeAlign::Center {
                assert!(!xaml.contains(" Clip="));
            } else {
                assert!(xaml.contains(" Clip="));
                if align == core::StrokeAlign::Outside { assert!(xaml.contains("GeometryCombineMode=\"Exclude\"")); }
            }
            let (native, exported) = render(&document, id);
            assert!(native.chunks_exact(4).filter(|pixel| pixel[3] != 0).count() > 350);
            equivalent(&native, &exported);
        }
    }
    scope.dispose();
}

#[test]
fn network_xaml_variable_width_and_uniform_stacks_keep_region_replacement_and_transform() {
    let _dispatcher = ferroui_base::threading::Dispatcher::unit_test_scope();
    let scope = FerroLocator::enter_scope();
    ferroui_vello::VelloPlatform::initialize();
    for rule in [FillRule::NonZero, FillRule::EvenOdd] {
        let (mut document, id, _) = fixture(rule);
        let node = document.nodes.get_mut(&id).unwrap();
        node.opacity = 0.7;
        node.set_local_transform(core::Transform { a: 0.9, b: 0.08, c: -0.05, d: 0.85, e: 20.0, f: 10.0 });
        node.strokes = vec![
            Stroke { paint: gradient_paints().remove(2), align: core::StrokeAlign::Outside,
                opacity: 0.8, width_profile: width_profile(), ..Stroke::new(Color::BLACK, 5.0) },
            Stroke::new(Color::rgba(20, 20, 20, 160), 1.5),
            Stroke { width_profile: core::StrokeWidthProfile::Eye, opacity: 0.6,
                ..Stroke::new(Color::rgb(20, 210, 90), 9.0) },
        ];
        let xaml = export_xaml(&document, &[id]).unwrap();
        assert!(xaml.contains(&format!("x:Key=\"Network{id}Width0\"")));
        assert!(xaml.contains(&format!("x:Key=\"Network{id}Width2\"")));
        assert!(!xaml.contains(&format!("x:Key=\"Network{id}Width1\"")));
        let (native, exported) = render(&document, id);
        assert!(native.chunks_exact(4).filter(|pixel| pixel[3] != 0).count() > 2_000);
        equivalent(&native, &exported);
    }
    scope.dispose();
}

#[test]
fn profiled_frame_xaml_clips_children_without_clipping_its_outside_border() {
    let _dispatcher = ferroui_base::threading::Dispatcher::unit_test_scope();
    let scope = FerroLocator::enter_scope();
    ferroui_vello::VelloPlatform::initialize();
    let mut document = Document::new("Profiled frame child clipping");
    let page = document.pages[0].id;
    let mut frame = Node::new(NodeKind::Frame, "Smooth frame", 30.0, 30.0, 110.0, 80.0);
    frame.corner_radii = Some([25.0, 12.0, 20.0, 8.0]);
    frame.corner_smoothing = 0.7;
    frame.opacity = 0.6;
    frame.fills = vec![Fill::solid(Color::rgb(20, 80, 210))];
    frame.strokes = vec![Stroke { align: core::StrokeAlign::Outside,
        width_profile: core::StrokeWidthProfile::Custom { points: vec![core::StrokeWidthPoint::new(0.0, 1.0)] },
        ..Stroke::new(Color::BLACK, 7.0) }];
    let id = document.insert_node(page, frame).unwrap();
    let child = Node::new(NodeKind::Rectangle, "Clipped red content", -20.0, -20.0, 150.0, 120.0)
        .with_fill(Color::rgb(230, 30, 20));
    document.insert_node(id, child).unwrap();
    let xaml = export_xaml(&document, &[id]).unwrap();
    assert!(xaml.contains(&format!("<Canvas Width=\"110\" Height=\"80\" Clip=\"{{StaticResource Profile{id}Fill}}\">")));
    let (native, exported) = render(&document, id);
    equivalent(&native, &exported);
    let pixel = |bytes: &[u8], x: usize, y: usize| <[u8; 4]>::try_from(&bytes[(y * WIDTH as usize + x) * 4..][..4]).unwrap();
    for bytes in [&native, &exported] {
        let outside = pixel(bytes, 80, 25);
        assert_eq!(&outside[..3], &[0, 0, 0], "the exterior border keeps its own paint");
        assert!((i32::from(outside[3]) - 153).abs() <= 1, "layer opacity covers the exterior border");
        let content = pixel(bytes, 80, 40);
        assert!(content[0] > content[2], "child content paints above the frame fill");
        assert!((i32::from(content[3]) - 153).abs() <= 1, "layer opacity composites parent and child once");
        assert_eq!(pixel(bytes, 30, 30)[3], 0, "smoothed child clip excludes the corner");
    }
    scope.dispose();
}

#[test]
fn network_xaml_strokes_the_edge_graph_once_and_css_identifies_regional_artwork() {
    let _dispatcher = ferroui_base::threading::Dispatcher::unit_test_scope();
    let scope = FerroLocator::enter_scope();
    ferroui_vello::VelloPlatform::initialize();
    let (mut document, id, _) = fixture(FillRule::NonZero);
    let node = document.nodes.get_mut(&id).unwrap();
    node.strokes = vec![core::Stroke { opacity: 0.5, cap: core::StrokeCap::Round,
        join: core::StrokeJoin::Round, ..core::Stroke::new(Color::BLACK, 3.0) }];
    assert!(css(node).contains("Export SVG"));
    assert!(!css(node).contains("background:"));
    let (native, exported) = render(&document, id);
    equivalent(&native, &exported);
    scope.dispose();
}
