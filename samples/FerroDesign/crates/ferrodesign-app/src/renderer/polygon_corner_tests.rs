//! Native pixels, picking and retained preparation must share the fillet path.
use super::*;
use ferrodesign_core::{Fill, MaskType, StrokeWidthProfile};
use ferroui_base::media::imaging::RenderTargetBitmap;
use ferroui_base::platform::PixelFormat;
use ferroui_base::{FerroLocator, PixelRect, PixelSize};

const SIZE: i32 = 220;
fn render(painter: &mut ScenePainter, editor: &Editor) -> (Vec<u8>, RenderStats) {
    let bitmap = RenderTargetBitmap::new(PixelSize::new(SIZE, SIZE));
    let stats = {
        let mut context = bitmap.create_drawing_context();
        painter.render_editor(
            &mut context,
            editor,
            &editor.document.pages[0].children,
            Rect::new(0.0, 0.0, SIZE as f64, SIZE as f64),
        )
    };
    let mut pixels = vec![0; (SIZE * SIZE * 4) as usize];
    bitmap.copy_pixels(PixelRect::new(0, 0, SIZE, SIZE), &mut pixels, SIZE * 4);
    if bitmap.format() == Some(PixelFormat::BGRA8888) {
        for pixel in pixels.chunks_exact_mut(4) {
            pixel.swap(0, 2);
        }
    }
    assert_eq!(stats.invalid_nodes, 0);
    assert_eq!(stats.unsupported_effects, 0);
    (pixels, stats)
}

fn fixture(kind: NodeKind, radius: f64) -> (Editor, NodeId) {
    let mut document = Document::new("Native polygon corners");
    let mut node = Node::new(kind, "Rounded shape", 20.0, 20.0, 160.0, 160.0);
    node.polygon_sides = if kind == NodeKind::Polygon { 3 } else { 5 };
    node.star_inner_radius = 0.45;
    node.corner_radius = radius;
    node.fills = vec![Fill::solid(Color::rgb(20, 90, 210))];
    node.strokes.clear();
    let id = document.insert_node(document.pages[0].id, node).unwrap();
    (Editor::new(document), id)
}

fn as_vector(editor: &Editor, id: NodeId) -> Editor {
    let mut document = editor.document.clone();
    let node = document.nodes.get_mut(&id).unwrap();
    node.path = ferrodesign_core::polygon_corner_path(node);
    node.kind = NodeKind::Vector;
    node.corner_radius = 0.0;
    Editor::new(document)
}

fn pixel(pixels: &[u8], x: usize, y: usize) -> &[u8] {
    let start = (y * SIZE as usize + x) * 4;
    &pixels[start..start + 4]
}

#[test]
fn native_polygon_and_star_pixels_equal_the_public_cubic_path_under_affine_transforms() {
    let _dispatcher = ferroui_base::threading::Dispatcher::unit_test_scope();
    let scope = FerroLocator::enter_scope();
    ferroui_vello::VelloPlatform::initialize();
    for kind in [NodeKind::Polygon, NodeKind::Star] {
        for radius in [0.0, 16.5, 1e6] {
            for affine in [false, true] {
                let (mut editor, id) = fixture(kind, radius);
                if affine {
                    editor.update_node(id, "Affine", |node| {
                        node.set_local_transform(Transform {
                            a: -0.95,
                            b: 0.2,
                            c: 0.25,
                            d: 0.85,
                            e: 170.0,
                            f: 10.0,
                        })
                    });
                }
                let actual = render(&mut ScenePainter::default(), &editor).0;
                let expected = render(&mut ScenePainter::default(), &as_vector(&editor, id)).0;
                assert_eq!(
                    actual, expected,
                    "{kind:?}, radius {radius}, affine {affine}"
                );
                assert!(actual.chunks_exact(4).any(|pixel| pixel[3] == 255));
            }
        }
    }
    scope.dispose();
}

#[test]
fn native_convex_corner_holes_and_concave_star_notches_match_picking() {
    let _dispatcher = ferroui_base::threading::Dispatcher::unit_test_scope();
    let scope = FerroLocator::enter_scope();
    ferroui_vello::VelloPlatform::initialize();
    for kind in [NodeKind::Polygon, NodeKind::Star] {
        let (mut editor, id) = fixture(kind, 0.0);
        let sharp = render(&mut ScenePainter::default(), &editor).0;
        assert_eq!(pixel(&sharp, 100, 25)[3], 255);
        editor.set_corner_radius(&[id], 20.0).unwrap();
        let rounded = render(&mut ScenePainter::default(), &editor).0;
        assert_eq!(
            pixel(&rounded, 100, 25)[3],
            0,
            "The former sharp tip is transparent"
        );
        assert_eq!(pixel(&rounded, 100, 100), &[20, 90, 210, 255]);
        let node = &editor.document.nodes[&id];
        assert!(!node.hit_test_local(Point::new(80.5, 5.5), 0.0));
        assert!(node.hit_test_local(Point::new(80.5, 80.5), 0.0));
        if kind == NodeKind::Star {
            let notch = node.polygon_points()[1] + Point::new(20.0, 20.0);
            let mut filled_notch = false;
            for y in notch.y as usize - 5..=notch.y as usize + 5 {
                for x in notch.x as usize - 5..=notch.x as usize + 5 {
                    if pixel(&sharp, x, y)[3] <= 8 && pixel(&rounded, x, y)[3] >= 240 {
                        assert!(node.hit_test_local(
                            Point::new(x as f64 + 0.5 - 20.0, y as f64 + 0.5 - 20.0),
                            0.0
                        ));
                        filled_notch = true;
                    }
                }
            }
            assert!(
                filled_notch,
                "A concave circular fillet fills the old sharp notch"
            );
        }
    }
    scope.dispose();
}

#[test]
fn polygon_radius_rebuilds_one_retained_geometry_and_undo_restores_exact_pixels() {
    let _dispatcher = ferroui_base::threading::Dispatcher::unit_test_scope();
    let scope = FerroLocator::enter_scope();
    ferroui_vello::VelloPlatform::initialize();
    let (mut editor, id) = fixture(NodeKind::Star, 0.0);
    let mut painter = ScenePainter::default();
    let before = render(&mut painter, &editor).0;
    assert_eq!(render(&mut painter, &editor).1.rebuilt_visuals, 0);
    let document_before = editor.document.to_json().unwrap();
    assert_eq!(editor.set_corner_radius(&[id], 16.5).unwrap(), 1);
    let (after, stats) = render(&mut painter, &editor);
    assert_eq!(stats.rebuilt_visuals, 1);
    assert_ne!(after, before);
    assert_eq!(editor.history_len(), 1);
    assert_eq!(after, render(&mut ScenePainter::default(), &editor).0);
    assert!(editor.undo());
    assert_eq!(editor.document.to_json().unwrap(), document_before);
    assert_eq!(render(&mut painter, &editor).0, before);
    assert!(editor.redo());
    assert_eq!(render(&mut painter, &editor).0, after);
    editor.update_node(id, "Placement", |node| node.x += 8.0);
    let (moved, stats) = render(&mut painter, &editor);
    assert_eq!(stats.rebuilt_visuals, 0);
    assert_eq!(moved, render(&mut ScenePainter::default(), &editor).0);
    scope.dispose();
}

#[test]
fn uniform_and_variable_stroke_stacks_use_the_rounded_polygon_centerline() {
    let _dispatcher = ferroui_base::threading::Dispatcher::unit_test_scope();
    let scope = FerroLocator::enter_scope();
    ferroui_vello::VelloPlatform::initialize();
    for kind in [NodeKind::Polygon, NodeKind::Star] {
        for align in [
            StrokeAlign::Center,
            StrokeAlign::Inside,
            StrokeAlign::Outside,
        ] {
            let (mut editor, id) = fixture(kind, 16.5);
            editor.update_node(id, "Strokes", |node| {
                node.opacity = 0.6;
                node.strokes = vec![
                    Stroke {
                        align,
                        width_profile: StrokeWidthProfile::Eye,
                        opacity: 0.7,
                        ..Stroke::new(Color::rgb(235, 30, 55), 12.0)
                    },
                    Stroke::new(Color::rgb(30, 210, 80), 3.0),
                ];
            });
            let actual = render(&mut ScenePainter::default(), &editor).0;
            let expected = render(&mut ScenePainter::default(), &as_vector(&editor, id)).0;
            assert_eq!(actual, expected, "{kind:?}, {align:?}");
        }
    }
    scope.dispose();
}

#[test]
fn rounded_star_vector_mask_uses_actual_corner_geometry_and_retains_updates() {
    let _dispatcher = ferroui_base::threading::Dispatcher::unit_test_scope();
    let scope = FerroLocator::enter_scope();
    ferroui_vello::VelloPlatform::initialize();
    let (mut editor, id) = fixture(NodeKind::Star, 20.0);
    editor.update_node(id, "Mask", |node| {
        node.is_mask = true;
        node.mask_type = MaskType::Vector;
    });
    let mut child = Node::new(
        NodeKind::Rectangle,
        "Masked paint",
        0.0,
        0.0,
        SIZE as f64,
        SIZE as f64,
    );
    child.fills = vec![Fill::solid(Color::rgb(25, 180, 70))];
    child.strokes.clear();
    editor.insert_node(editor.active_page, child).unwrap();
    let mut painter = ScenePainter::default();
    let before = render(&mut painter, &editor).0;
    assert_eq!(pixel(&before, 100, 25)[3], 0);
    assert_eq!(pixel(&before, 100, 100), &[25, 180, 70, 255]);
    assert_eq!(
        before,
        render(&mut ScenePainter::default(), &as_vector(&editor, id)).0
    );
    editor.set_corner_radius(&[id], 0.0).unwrap();
    let after = render(&mut painter, &editor).0;
    assert_eq!(pixel(&after, 100, 25), &[25, 180, 70, 255]);
    assert_eq!(after, render(&mut ScenePainter::default(), &editor).0);
    assert!(editor.undo());
    assert_eq!(render(&mut painter, &editor).0, before);
    scope.dispose();
}
