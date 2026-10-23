//! Region replacement, public FerroUI pixels, masks and scoped invalidation.
use super::*;
use ferrodesign_core::{Fill, GradientStop, VectorNetwork, VectorPath};
use ferroui_base::media::imaging::RenderTargetBitmap;
use ferroui_base::platform::PixelFormat;
use ferroui_base::{FerroLocator, PixelRect, PixelSize};

const SIZE: i32 = 128;

fn fixture(rule: FillRule) -> (Editor, NodeId) {
    let path = VectorPath {
        fill_rule: rule,
        ..VectorPath::polygon(&[
            Point::new(0.0, 0.0), Point::new(100.0, 0.0),
            Point::new(100.0, 100.0), Point::new(0.0, 100.0),
        ], true)
    };
    let mut network = VectorNetwork::from_path(&path).unwrap();
    let vertex = |p| *network.vertices.iter().find(|(_, v)| v.position == p).unwrap().0;
    let a = vertex(Point::new(0.0, 0.0));
    let b = vertex(Point::new(100.0, 100.0));
    network.connect(a, b).unwrap();
    let mut node = Node::new(NodeKind::Vector, "Region paints", 12.0, 12.0, 100.0, 100.0);
    node.fills = vec![Fill::solid(Color::rgba(0, 0, 255, 128))];
    node.vector_network = Some(network);
    let mut doc = Document::new("Regions");
    let id = doc.insert_node(doc.pages[0].id, node).unwrap();
    (Editor::new(doc), id)
}

fn render(painter: &mut ScenePainter, editor: &Editor) -> (Vec<u8>, RenderStats) {
    let bitmap = RenderTargetBitmap::new(PixelSize::new(SIZE, SIZE));
    let stats;
    {
        let mut context = bitmap.create_drawing_context();
        stats = painter.render_editor(&mut context, editor, &editor.document.pages[0].children,
            Rect::new(0.0, 0.0, SIZE as f64, SIZE as f64));
    }
    assert_eq!(stats.invalid_nodes, 0);
    assert_eq!(stats.unsupported_effects, 0);
    let mut pixels = vec![0; (SIZE * SIZE * 4) as usize];
    bitmap.copy_pixels(PixelRect::new(0, 0, SIZE, SIZE), &mut pixels, SIZE * 4);
    if bitmap.format() == Some(PixelFormat::BGRA8888) {
        for pixel in pixels.chunks_exact_mut(4) { pixel.swap(0, 2); }
    }
    (pixels, stats)
}

fn pixel(pixels: &[u8], x: usize, y: usize) -> [u8; 4] {
    let offset = (y * SIZE as usize + x) * 4;
    pixels[offset..offset + 4].try_into().unwrap()
}

fn near(pixels: &[u8], x: usize, y: usize, expected: [u8; 4]) {
    let actual = pixel(pixels, x, y);
    assert!(actual.iter().zip(expected).all(|(a,b)| a.abs_diff(b) <= 2),
        "pixel({x},{y}) = {actual:?}, expected {expected:?}");
}

#[test]
fn region_paints_replace_inherited_alpha_and_keep_other_faces_for_both_rules() {
    let scope = FerroLocator::enter_scope();
    ferroui_vello::VelloPlatform::initialize();
    for rule in [FillRule::NonZero, FillRule::EvenOdd] {
        let (mut editor, id) = fixture(rule);
        assert!(editor.paint_network_face(id, Point::new(75.0, 25.0),
            Some(vec![Fill::solid(Color::rgba(255, 0, 0, 128))])).unwrap());
        let pixels = render(&mut ScenePainter::default(), &editor).0;
        near(&pixels, 87, 37, [128, 0, 0, 128]);
        near(&pixels, 37, 87, [0, 0, 128, 128]);
        // The inherited fill must not remain under the red region (alpha192).
        assert_eq!(pixel(&pixels, 87, 37)[3], 128);
    }
    scope.dispose();
}

#[test]
fn region_paints_empty_override_is_a_hole_and_reset_restores_exact_pixels() {
    let scope = FerroLocator::enter_scope();
    ferroui_vello::VelloPlatform::initialize();
    for rule in [FillRule::NonZero, FillRule::EvenOdd] {
        let (mut editor, id) = fixture(rule);
        let mut painter = ScenePainter::default();
        let original = render(&mut painter, &editor).0;
        editor.paint_network_face(id, Point::new(75.0, 25.0), Some(Vec::new())).unwrap();
        let region = editor.document.nodes[&id].vector_network.as_ref().unwrap()
            .explicit_region_at(Point::new(75.0, 25.0)).unwrap();
        let (pixels, stats) = render(&mut painter, &editor);
        near(&pixels, 87, 37, [0, 0, 0, 0]);
        near(&pixels, 37, 87, [0, 0, 128, 128]);
        assert_eq!(stats.prepared_nodes, 1);
        assert_eq!(stats.rebuilt_visuals, 1);
        assert!(!editor.document.nodes[&id].hit_test_local(Point::new(75.0,25.0), 0.0));
        assert!(editor.set_network_region_fills(id, region, None).unwrap());
        assert_eq!(render(&mut painter, &editor).0, original);
    }
    scope.dispose();
}

#[test]
fn region_paints_incremental_stack_edits_history_and_camera_reuse_match_fresh_pixels() {
    let scope = FerroLocator::enter_scope();
    ferroui_vello::VelloPlatform::initialize();
    let (mut editor, id) = fixture(FillRule::NonZero);
    editor.paint_network_face(id, Point::new(75.0,25.0), Some(vec![Fill::solid(Color::rgb(240,40,60))])).unwrap();
    let region = editor.document.nodes[&id].vector_network.as_ref().unwrap()
        .explicit_region_at(Point::new(75.0,25.0)).unwrap();
    let mut painter = ScenePainter::default();
    let original = render(&mut painter, &editor).0;
    for step in 0..3 {
        let mut fills = vec![Fill::solid(Color::rgb(30,240,60))];
        match step {
            0 => fills[0].opacity = 0.6,
            1 => fills[0].paint = Paint::AngularGradient {
                center: Point::new(0.5,0.5), angle: 45.0,
                stops: vec![GradientStop { offset: 0.0, color: Color::BLACK },
                    GradientStop { offset: 1.0, color: Color::WHITE }],
            },
            _ => { fills.push(Fill::solid(Color::rgb(0,0,255))); fills[0].opacity = 0.5; }
        }
        editor.set_network_region_fills(id, region, Some(fills)).unwrap();
        let (actual, stats) = render(&mut painter, &editor);
        assert_eq!(stats.prepared_nodes,1);
        assert_eq!(stats.rebuilt_visuals,1);
        assert_ne!(actual,original);
        assert_eq!(actual,render(&mut ScenePainter::default(),&editor).0);
    }
    for _ in 0..3 { editor.undo(); }
    assert_eq!(render(&mut painter,&editor).0,original);
    for _ in 0..3 { editor.redo(); }
    assert_eq!(render(&mut painter,&editor).0,render(&mut ScenePainter::default(),&editor).0);
    assert_eq!(render(&mut painter,&editor).1.prepared_nodes,0);
    scope.dispose();
}

#[test]
fn region_paints_layer_opacity_applies_once_and_first_stack_row_is_topmost() {
    let scope = FerroLocator::enter_scope();
    ferroui_vello::VelloPlatform::initialize();
    let (mut editor,id) = fixture(FillRule::NonZero);
    editor.paint_network_face(id,Point::new(75.0,25.0),Some(vec![
        Fill::solid(Color::rgba(255,0,0,128)), Fill::solid(Color::rgb(0,255,0)),
    ])).unwrap();
    editor.update_node(id,"Layer opacity",|node| node.opacity=0.5);
    let pixels=render(&mut ScenePainter::default(),&editor).0;
    near(&pixels,87,37,[64,64,0,128]);
    near(&pixels,37,87,[0,0,64,64]);
    scope.dispose();
}

#[test]
fn region_paints_vector_and_alpha_masks_respect_transparent_replacements() {
    let scope = FerroLocator::enter_scope();
    ferroui_vello::VelloPlatform::initialize();
    for kind in [MaskType::Vector,MaskType::Alpha] {
        let (mut editor,id)=fixture(FillRule::NonZero);
        editor.paint_network_face(id,Point::new(75.0,25.0),Some(Vec::new())).unwrap();
        editor.update_node(id,"Mask",|node| {node.is_mask=true;node.mask_type=kind;});
        let content=editor.document.insert_node(editor.active_page,
            Node::new(NodeKind::Rectangle,"Masked",12.0,12.0,100.0,100.0).with_fill(Color::rgb(255,0,0))).unwrap();
        let pixels=render(&mut ScenePainter::default(),&editor).0;
        near(&pixels,87,37,[0,0,0,0]);
        let alpha=if kind==MaskType::Vector {255}else{128};
        near(&pixels,37,87,[alpha,0,0,alpha]);
        let roots=editor.document.pages[0].children.clone();
        let mut painter=ScenePainter::default();
        assert!(!painter.mask_contains(&editor,&roots,content,Point::new(87.0,37.0)));
        assert!(painter.mask_contains(&editor,&roots,content,Point::new(37.0,87.0)));
    }
    scope.dispose();
}
