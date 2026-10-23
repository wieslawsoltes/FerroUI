//! Load the exported paths through the public XAML loader and compare actual
//! Vello pixels with the design renderer, rather than comparing XML strings.
use super::*;
use crate::renderer::ScenePainter;
use ferrodesign_core::{Document, ParagraphStylePatch, TextDirection, TextRun, TextStyleChange};
use ferroui_base::media::imaging::RenderTargetBitmap;
use ferroui_base::media::FontManager;
use ferroui_base::platform::{IAssetLoader, PixelFormat, StandardAssetLoader};
use ferroui_base::{FerroLocator, PixelRect, PixelSize, Size};
use ferroui_controls::Canvas;
use ferroui_markup_xaml_loader::FerroRuntimeXamlLoader;

const WIDTH: i32 = 420;
const HEIGHT: i32 = 320;

fn backend() -> Rc<dyn ferroui_base::reactive::IDisposable> {
    let scope = FerroLocator::enter_scope();
    FerroLocator::current_mutable().bind::<dyn IAssetLoader>()
        .to_constant(Rc::new(StandardAssetLoader::new(None)));
    ferroui_vello::VelloPlatform::initialize();
    ferroui_harfbuzz::HarfBuzzPlatform::initialize();
    FontManager::current().add_font_collection(Rc::new(ferroui_fonts_inter::InterFontCollection::new()));
    scope
}

fn text(value: &str) -> Node {
    let mut node = Node::new(NodeKind::Text, "Native outlined paragraphs", 0.0, 0.0, 210.0, 260.0)
        .with_text(value, 20.0, Color::rgba(35, 80, 210, 180));
    node.font_family = "fonts:Inter#Inter".into();
    node.line_height = 1.35;
    node.text_direction = Some(TextDirection::Auto);
    node.text_decoration = core::TextDecoration::Underline;
    node
}

fn bytes(bitmap: &RenderTargetBitmap) -> Vec<u8> {
    let mut result = vec![0; (WIDTH * HEIGHT * 4) as usize];
    bitmap.copy_pixels(PixelRect::new(0, 0, WIDTH, HEIGHT), &mut result, WIDTH * 4);
    if bitmap.format() == Some(PixelFormat::BGRA8888) {
        for pixel in result.chunks_exact_mut(4) { pixel.swap(0, 2); }
    }
    result
}

fn assert_pixels(node: Node) -> String {
    let mut document = Document::new("Text XAML pixel parity");
    let id = document.insert_node(1, node).unwrap();
    assert_document_pixels(document, &[id])
}

fn assert_document_pixels(document: Document, roots: &[NodeId]) -> String {
    let original = document.to_json().unwrap();
    let xaml = export_xaml(&document, roots).unwrap();
    assert!(!xaml.contains("<TextBlock"), "Independent paragraphs must use the actual native artwork");
    assert!(xaml.contains("<Path "));
    ferroui_markup_xaml::register_types();
    let loaded = FerroRuntimeXamlLoader::load(&xaml, None, None, None, false)
        .unwrap_or_else(|error| panic!("Native text XAML must load: {error}\n{xaml}"));
    let canvas = from_markup_value::<Ref<Canvas>>(&Some(loaded)).expect("Canvas root");
    canvas.set_use_layout_rounding(false);
    canvas.measure(Size::new(WIDTH as f64, HEIGHT as f64));
    canvas.arrange(ferroui_base::Rect::new(0.0, 0.0, WIDTH as f64, HEIGHT as f64));
    let bitmap = RenderTargetBitmap::new(PixelSize::new(WIDTH, HEIGHT));
    bitmap.render(&canvas.upcast());
    let exported = bytes(&bitmap);
    let bitmap = RenderTargetBitmap::new(PixelSize::new(WIDTH, HEIGHT));
    {
        let stats = ScenePainter::default().render(&mut bitmap.create_drawing_context(), &document, roots,
            core::Rect::new(0.0, 0.0, WIDTH as f64, HEIGHT as f64));
        assert_eq!((stats.invalid_nodes, stats.unsupported_effects, stats.missing_images), (0, 0, 0));
    }
    let native = bytes(&bitmap);
    assert!(native.chunks_exact(4).filter(|pixel| pixel[3] > 0).count() > 100);
    let max_error = native.iter().zip(&exported).map(|(a, b)| a.abs_diff(*b)).max().unwrap();
    if max_error > 1 {
        if let Some(directory) = std::env::var_os("FERRODESIGN_PIXEL_DIAGNOSTICS") {
            let directory = std::path::PathBuf::from(directory);
            std::fs::create_dir_all(&directory).unwrap();
            let name = std::thread::current().name().unwrap_or("text_export").replace("::", "_");
            std::fs::write(directory.join(format!("{name}-native.rgba")), &native).unwrap();
            std::fs::write(directory.join(format!("{name}-xaml.rgba")), &exported).unwrap();
            std::fs::write(directory.join(format!("{name}.xaml")), &xaml).unwrap();
        }
    }
    assert!(max_error <= 1, "Loaded XAML differs from native text by {max_error}/255 in at least one channel");
    assert_eq!(document.to_json().unwrap(), original, "Export must not mutate the editable text or ranges");
    xaml
}

#[test]
fn text_xaml_preserves_actual_paragraph_wrapping_rtl_rich_paints_decorations_and_affine_layer_alpha() {
    let _dispatcher = ferroui_base::threading::Dispatcher::unit_test_scope();
    let scope = backend();
    let mut node = text("AV ffi shaped text wraps across several lines\r\nשלום second\nLast");
    let ranges = core::text_paragraph_ranges(&node.text);
    node.apply_paragraph_style(ranges[0].clone(), &ParagraphStylePatch {
        alignment: TextStyleChange::Set(TextAlign::Center), indent: TextStyleChange::Set(11.25),
        line_height: TextStyleChange::Set(1.5), space_before: TextStyleChange::Set(3.5),
        space_after: TextStyleChange::Set(9.25), letter_spacing: TextStyleChange::Set(0.125),
        ..Default::default()
    }).unwrap();
    node.apply_paragraph_style(ranges[1].clone(), &ParagraphStylePatch {
        direction: TextStyleChange::Set(TextDirection::RightToLeft), alignment: TextStyleChange::Set(TextAlign::Right),
        indent: TextStyleChange::Set(7.5), space_before: TextStyleChange::Set(5.25),
        space_after: TextStyleChange::Set(8.0), line_height: TextStyleChange::Set(1.75), ..Default::default()
    }).unwrap();
    node.text_runs.push(TextRun { start: 0, end: 2, font_family: None, font_size: Some(28.5), font_weight: Some(700),
        color: Some(Color::rgba(220, 35, 80, 160)), italic: Some(true), decoration: Some(core::TextDecoration::Strikethrough) });
    node.fills[0].opacity = 0.63;
    node.fills.push(Fill { paint: Paint::Solid(Color::rgba(240, 140, 20, 115)), opacity: 0.37, visible: true });
    node.opacity = 0.61;
    node.set_local_transform(core::Transform { a: 1.05, b: 0.08, c: 0.12, d: 0.95, e: 31.125, f: 13.375 });
    let xaml = assert_pixels(node);
    assert!(xaml.contains("<Canvas.OpacityMask>"));
    assert!(xaml.contains("Color=\"#A0DC2350\" Opacity=\"0.63\""));
    assert!(xaml.contains("<Path.Stroke>"));
    assert!(xaml.matches("<Path ").count() > 40, "Each native glyph and decoration retains its own draw");
    scope.dispose();
}

#[test]
fn text_xaml_keeps_translucent_overlapping_glyphs_and_zero_width_auto_size() {
    let _dispatcher = ferroui_base::threading::Dispatcher::unit_test_scope();
    let scope = backend();
    for auto_size in [false, true] {
        let mut node = text("oooo\nAV");
        node.font_size = 36.25;
        node.letter_spacing = -13.0;
        node.text_decoration = core::TextDecoration::None;
        node.opacity = 0.57;
        node.fills[0].opacity = 0.63;
        if auto_size {
            node.text_auto_resize = core::TextAutoResize::WidthAndHeight;
            node.width = 0.0;
            node.height = 0.0;
        }
        node.set_local_transform(core::Transform::translation(29.125, 21.375));
        let xaml = assert_pixels(node);
        assert_eq!(xaml.matches("<Path ").count(), 6, "Six glyphs must not become two combined paragraph fills");
    }
    scope.dispose();
}

#[test]
fn text_xaml_preserves_nested_ancestor_shear_reflection_and_isolated_overlap_alpha() {
    let _dispatcher = ferroui_base::threading::Dispatcher::unit_test_scope();
    let scope = backend();
    for background in [
        Paint::Solid(Color::rgba(235, 210, 170, 220)),
        Paint::LinearGradient { start: Point::new(0.0, 0.0), end: Point::new(1.0, 1.0), stops: vec![
            GradientStop { offset: 0.0, color: Color::rgba(235, 210, 170, 220) },
            GradientStop { offset: 1.0, color: Color::rgba(130, 180, 225, 190) },
        ] },
    ] {
        let mut document = Document::new("Nested exact text artwork");
        let mut outer = Node::new(NodeKind::Frame, "Sheared translucent parent", 0.0, 0.0, 245.5, 190.25);
        outer.clip_content = false;
        outer.blend_mode = core::BlendMode::Normal;
        outer.opacity = 0.68;
        outer.fills = vec![Fill { paint: background, opacity: 0.83, visible: true }];
        outer.set_local_transform(core::Transform { a: 1.0, b: 0.12, c: 0.14, d: 0.9, e: 32.125, f: 21.375 });
        let outer_id = document.insert_node(1, outer).unwrap();
        let mut inner = Node::new(NodeKind::Group, "Rotated reflected parent", 18.375, 14.625, 210.0, 140.0);
        inner.rotation = -11.0;
        inner.flip_x = true;
        inner.blend_mode = core::BlendMode::Normal;
        inner.opacity = 0.57;
        let inner_id = document.insert_node(outer_id, inner).unwrap();
        let mut first = text("oo AV\nffi");
        first.width = 170.0;
        first.height = 100.0;
        first.font_size = 28.0;
        first.letter_spacing = -7.0;
        first.opacity = 0.73;
        first.x = 20.25;
        first.y = 13.125;
        first.apply_paragraph_style(0..0, &ParagraphStylePatch {
            indent: TextStyleChange::Set(5.25), space_after: TextStyleChange::Set(3.5),
            ..Default::default()
        }).unwrap();
        document.insert_node(inner_id, first.clone()).unwrap();
        let mut second = first;
        second.name = "Overlapping independent text".into();
        second.x += 4.125;
        second.y += 3.375;
        second.fills = vec![Fill::solid(Color::rgba(215, 30, 80, 170))];
        document.insert_node(inner_id, second).unwrap();
        // The same generic transform path also owns ordinary non-text nodes.
        let mut shape = Node::new(NodeKind::Rectangle, "Affine sibling", 0.0, 0.0, 35.25, 24.5)
            .with_fill(Color::rgba(30, 155, 75, 200));
        shape.set_local_transform(core::Transform { a: -0.9, b: 0.12, c: 0.18, d: 1.1, e: 213.375, f: 128.125 });
        shape.opacity = 0.62;
        document.insert_node(outer_id, shape).unwrap();
        let xaml = assert_document_pixels(document, &[outer_id]);
        assert!(!xaml.contains("Canvas.Left=") && !xaml.contains("RenderTransform=\"rotate("));
        assert!(xaml.matches("<Canvas.OpacityMask>").count() >= 4,
            "Both native ancestors and the overlapping text layers retain separate compositing scopes");
        assert!(xaml.contains("<Rectangle.RenderTransform><MatrixTransform"));
        assert!(xaml.contains("<Rectangle.OpacityMask>"));
    }
    scope.dispose();
}

#[test]
fn text_xaml_ancestor_background_ignores_visible_zero_opacity_fills() {
    let _dispatcher = ferroui_base::threading::Dispatcher::unit_test_scope();
    let scope = backend();
    let mut document = Document::new("Inactive ancestor paint");
    let mut parent = Node::new(NodeKind::Frame, "One active background", 24.0, 20.0, 240.0, 150.0);
    parent.clip_content = false;
    parent.blend_mode = core::BlendMode::Normal;
    parent.fills = vec![
        Fill { paint: Paint::Solid(Color::rgba(255, 0, 0, 255)), opacity: 0.0, visible: true },
        Fill::solid(Color::rgba(37, 99, 235, 255)),
    ];
    let parent = document.insert_node(1, parent).unwrap();
    let mut child = text("Native text\nafter an inactive fill");
    child.x = 12.0;
    child.y = 10.0;
    child.width = 200.0;
    child.height = 110.0;
    child.fills = vec![Fill::solid(Color::BLACK)];
    document.insert_node(parent, child).unwrap();

    // The native renderer and the supported-stack guard both ignore the first
    // paint. Looking up merely the first visible fill would lose the blue
    // background, even though exactly one active background is supported.
    let xaml = assert_document_pixels(document, &[parent]);
    assert!(xaml.contains("Background=\"#FF2563EB\""));
    assert!(!xaml.contains("#FFFF0000"));
    scope.dispose();
}

#[test]
fn text_xaml_rejects_unrepresented_ancestor_features_before_any_result_is_returned() {
    let plain = Node::new(NodeKind::Frame, "Unsupported text ancestor", 0.0, 0.0, 200.0, 150.0);
    for parent in [
        { let mut node = plain.clone(); node.effects.push(Effect::shadow(Color::BLACK, Point::new(2.0, 3.0), 4.0)); node },
        { let mut node = plain.clone(); node.is_mask = true; node },
        { let mut node = plain.clone(); node.blend_mode = core::BlendMode::Multiply; node },
        { let mut node = plain.clone(); node.strokes.push(Stroke::new(Color::BLACK, 2.0)); node },
        { let mut node = plain.clone(); node.fills.push(Fill::solid(Color::BLACK)); node },
        { let mut node = plain.clone(); node.corner_radius = 14.0; node },
        { let mut node = plain.clone(); node.fills[0].paint = Paint::Image { href: "missing.png".into(), scale: core::ImageScale::Fill, crop: None }; node },
    ] {
        let mut document = Document::new("Atomic unsupported ancestor");
        let earlier = document.insert_node(1, Node::new(NodeKind::Rectangle, "Earlier valid output", 0.0, 0.0, 20.0, 20.0)).unwrap();
        let parent = document.insert_node(1, parent).unwrap();
        document.insert_node(parent, text("Native paragraphs")).unwrap();
        let before = document.to_json().unwrap();
        // Each ancestor is rejected before reaching text shaping, so no font
        // services are needed and a partly serialized selection never escapes.
        let error = export_xaml(&document, &[earlier, parent]).unwrap_err();
        assert!(error.contains("Unsupported text ancestor") && error.contains("native PNG"));
        assert_eq!(document.to_json().unwrap(), before);
    }
}

#[test]
fn text_xaml_rejects_unsupported_foregrounds_and_layer_features_before_output() {
    let mut unsupported = text("Unsupported text");
    unsupported.fills[0].paint = Paint::LinearGradient { start: Point::new(0.0, 0.0), end: Point::new(1.0, 0.0),
        stops: vec![GradientStop { offset: 0.0, color: Color::BLACK }, GradientStop { offset: 1.0, color: Color::WHITE }] };
    // Prevalidation happens before shaping, so this test needs no font service.
    for node in [
        unsupported.clone(),
        { let mut node = text("Stroke"); node.strokes.push(Stroke::new(Color::BLACK, 2.0)); node },
        { let mut node = text("Effect"); node.effects.push(Effect::shadow(Color::BLACK, Point::new(1.0, 2.0), 3.0)); node },
        { let mut node = text("Blend"); node.blend_mode = core::BlendMode::Multiply; node },
        { let mut node = text("Mask"); node.is_mask = true; node },
    ] {
        let mut output = "previous output".to_string();
        let error = write_node(&node, 1, &mut output).unwrap_err();
        assert!(error.contains("Exact text XAML export") && error.contains("native PNG"));
        assert_eq!(output, "previous output");
    }
    let mut document = Document::new("Atomic mixed selection");
    let shape = document.insert_node(1, Node::new(NodeKind::Rectangle, "Earlier valid layer", 0.0, 0.0, 20.0, 20.0)).unwrap();
    let bad = document.insert_node(1, unsupported).unwrap();
    let before = document.to_json().unwrap();
    assert!(export_xaml(&document, &[shape, bad]).unwrap_err().contains("solid foregrounds"));
    assert_eq!(document.to_json().unwrap(), before);
}
