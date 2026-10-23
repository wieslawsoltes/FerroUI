//! Actual Vello pixels enforce effect-stack order and the layer/group paint
//! distinction documented by Figma, rather than testing prepared vectors only.
//! https://help.figma.com/hc/en-us/articles/360041488473-Apply-effects-to-layers
use super::*;
use ferrodesign_core::{EffectPropertyEdit, EffectStackEdit, VectorPath};
use ferroui_base::media::imaging::RenderTargetBitmap;
use ferroui_base::platform::{IPlatformRenderInterface, PixelFormat};
use ferroui_base::{FerroLocator, PixelRect, PixelSize};

const SIZE: i32 = 128;
const RED: Color = Color::rgb(255, 0, 0);
const BLUE: Color = Color::rgb(0, 0, 255);
const GREEN: Color = Color::rgb(0, 255, 0);
const YELLOW: Color = Color::rgb(255, 255, 0);

fn backend() -> Rc<dyn ferroui_base::reactive::IDisposable> {
    let scope = FerroLocator::enter_scope();
    FerroLocator::current_mutable()
        .bind::<dyn IPlatformRenderInterface>()
        .to_constant(Rc::new(ferroui_vello::PlatformRenderInterface::default()));
    scope
}

fn render(painter: &mut ScenePainter, editor: &Editor) -> (Vec<u8>, RenderStats) {
    let bitmap = RenderTargetBitmap::new(PixelSize::new(SIZE, SIZE));
    let stats;
    {
        let mut context = bitmap.create_drawing_context();
        painter.set_raster_scale(1.0);
        stats = painter.render_editor(
            &mut context,
            editor,
            &editor.document.pages[0].children,
            Rect::new(0.0, 0.0, SIZE as f64, SIZE as f64),
        );
    }
    let mut pixels = vec![0; (SIZE * SIZE * 4) as usize];
    bitmap.copy_pixels(PixelRect::new(0, 0, SIZE, SIZE), &mut pixels, SIZE * 4);
    if bitmap.format() == Some(PixelFormat::BGRA8888) {
        for pixel in pixels.chunks_exact_mut(4) {
            pixel.swap(0, 2);
        }
    }
    assert_eq!(stats.unsupported_effects, 0);
    assert_eq!(stats.invalid_nodes, 0);
    (pixels, stats)
}

fn pixel(pixels: &[u8], x: usize, y: usize) -> [u8; 4] {
    let start = (y * SIZE as usize + x) * 4;
    pixels[start..start + 4].try_into().unwrap()
}

fn assert_pixel(pixels: &[u8], x: usize, y: usize, color: Color) {
    assert_eq!(
        pixel(pixels, x, y),
        [color.r, color.g, color.b, color.a],
        "pixel ({x}, {y})"
    );
}

fn shape(kind: NodeKind) -> Node {
    let mut node = Node::new(kind, "Shadow layer", 32.0, 32.0, 64.0, 64.0).with_fill(Color::WHITE);
    if kind == NodeKind::Vector {
        // Identical silhouette, but the renderer uses real path geometry and
        // path-shadow composition instead of its BoxShadows optimization.
        node.path = VectorPath::polygon(
            &[
                Point::new(0.0, 0.0),
                Point::new(64.0, 0.0),
                Point::new(64.0, 64.0),
                Point::new(0.0, 64.0),
            ],
            true,
        );
    }
    node
}

fn shadow(kind: EffectKind, color: Color) -> Effect {
    Effect {
        kind,
        color,
        offset: Point::new(16.0, 0.0),
        radius: 0.0,
        spread: 0.0,
        visible: true,
    }
}

fn editor_with(node: Node) -> (Editor, NodeId) {
    let mut document = Document::new("Shadow ordering pixels");
    let layer = document.insert_node(document.pages[0].id, node).unwrap();
    (Editor::new(document), layer)
}

fn assert_shadow_order(kind: NodeKind, effect_kind: EffectKind) {
    let scope = backend();
    let mut node = shape(kind);
    node.effects = vec![shadow(effect_kind, RED), shadow(effect_kind, BLUE)];
    let (mut editor, layer) = editor_with(node);
    editor.selection = vec![layer];
    let mut painter = ScenePainter::default();
    let (before, initial) = render(&mut painter, &editor);
    assert_eq!(initial.full_prepares, 1);
    let x = if effect_kind == EffectKind::DropShadow {
        104
    } else {
        40
    };
    assert_pixel(&before, x, 64, RED);
    assert_pixel(&before, 64, 64, Color::WHITE);

    // Exercise the authoring command and its incremental rendering path. This
    // verifies the user-visible reorder, not merely static construction order.
    assert_eq!(
        editor
            .edit_effects(&[layer], EffectStackEdit::Move { index: 0, to: 1 })
            .unwrap(),
        1
    );
    let (reordered, stats) = render(&mut painter, &editor);
    assert_eq!(
        (
            stats.full_prepares,
            stats.incremental_prepares,
            stats.prepared_nodes
        ),
        (0, 1, 1)
    );
    assert_pixel(&reordered, x, 64, BLUE);
    assert_pixel(&reordered, 64, 64, Color::WHITE);
    assert_ne!(before, reordered);
    assert_eq!(reordered, render(&mut ScenePainter::default(), &editor).0);

    assert!(editor.undo());
    assert_eq!(render(&mut painter, &editor).0, before);
    assert!(editor.redo());
    assert_eq!(render(&mut painter, &editor).0, reordered);
    scope.dispose();
}

#[test]
fn rectangle_drop_shadows_put_first_effect_on_top() {
    assert_shadow_order(NodeKind::Rectangle, EffectKind::DropShadow);
}

#[test]
fn rectangle_inner_shadows_put_first_effect_on_top() {
    assert_shadow_order(NodeKind::Rectangle, EffectKind::InnerShadow);
}

#[test]
fn vector_drop_shadows_put_first_effect_on_top() {
    assert_shadow_order(NodeKind::Vector, EffectKind::DropShadow);
}

#[test]
fn vector_inner_shadows_put_first_effect_on_top() {
    assert_shadow_order(NodeKind::Vector, EffectKind::InnerShadow);
}

#[test]
fn curved_path_shadows_retain_order_through_clipping() {
    assert_shadow_order(NodeKind::Ellipse, EffectKind::DropShadow);
    assert_shadow_order(NodeKind::Ellipse, EffectKind::InnerShadow);
}

fn inside_stroke() -> Stroke {
    let mut stroke = Stroke::new(GREEN, 8.0);
    stroke.align = StrokeAlign::Inside;
    stroke
}

#[test]
fn leaf_inner_shadow_changes_fill_pixels_without_covering_opaque_strokes() {
    let scope = backend();
    for kind in [NodeKind::Rectangle, NodeKind::Vector, NodeKind::Ellipse] {
        let mut node = shape(kind);
        node.strokes = vec![inside_stroke()];
        node.effects = vec![shadow(EffectKind::InnerShadow, RED)];
        node.effects[0].visible = false;
        let (mut editor, layer) = editor_with(node);
        let mut painter = ScenePainter::default();
        let (before, _) = render(&mut painter, &editor);
        assert_pixel(&before, 36, 64, GREEN);
        assert_pixel(&before, 44, 64, Color::WHITE);

        editor
            .edit_effects(
                &[layer],
                EffectStackEdit::Property {
                    index: 0,
                    edit: EffectPropertyEdit::Visible(true),
                },
            )
            .unwrap();
        let (after, stats) = render(&mut painter, &editor);
        assert_eq!((stats.full_prepares, stats.incremental_prepares), (0, 1));
        assert_pixel(&after, 36, 64, GREEN);
        assert_pixel(&after, 44, 64, RED);
        assert_ne!(before, after, "{kind:?}");
        let mut opaque_stroke_pixels = 0;
        for (before, after) in before.chunks_exact(4).zip(after.chunks_exact(4)) {
            if before == [0, 255, 0, 255] {
                assert_eq!(
                    before, after,
                    "opaque {kind:?} stroke was covered by an inner shadow"
                );
                opaque_stroke_pixels += 1;
            }
        }
        assert!(opaque_stroke_pixels > 400);
        assert_eq!(after, render(&mut ScenePainter::default(), &editor).0);
    }
    scope.dispose();
}

#[test]
fn boolean_result_uses_leaf_shadow_order_and_keeps_operands_suppressed() {
    let scope = backend();
    let mut node = shape(NodeKind::Boolean);
    node.strokes = vec![inside_stroke()];
    node.effects = vec![shadow(EffectKind::InnerShadow, RED)];
    let mut document = Document::new("Boolean shadow pixels");
    let layer = document.insert_node(document.pages[0].id, node).unwrap();
    document
        .insert_node(
            layer,
            Node::new(NodeKind::Rectangle, "Operand", 0.0, 0.0, 64.0, 64.0).with_fill(YELLOW),
        )
        .unwrap();
    document
        .insert_node(
            layer,
            Node::new(
                NodeKind::Ellipse,
                "Contained operand",
                16.0,
                16.0,
                24.0,
                24.0,
            )
            .with_fill(BLUE),
        )
        .unwrap();
    let editor = Editor::new(document);
    let (pixels, _) = render(&mut ScenePainter::default(), &editor);
    assert_pixel(&pixels, 36, 64, GREEN);
    assert_pixel(&pixels, 44, 64, RED);
    assert_pixel(&pixels, 64, 64, Color::WHITE);
    scope.dispose();
}

#[test]
fn containers_keep_inner_shadow_above_their_compound_paints() {
    let scope = backend();
    for kind in [NodeKind::Group, NodeKind::Frame] {
        let mut node = shape(kind);
        node.strokes = vec![inside_stroke()];
        node.effects = vec![shadow(EffectKind::InnerShadow, RED)];
        let mut document = Document::new("Container shadow pixels");
        let layer = document.insert_node(document.pages[0].id, node).unwrap();
        let (position, size) = if kind == NodeKind::Group {
            (0.0, 64.0)
        } else {
            (12.0, 40.0)
        };
        document
            .insert_node(
                layer,
                Node::new(NodeKind::Rectangle, "Child", position, position, size, size)
                    .with_fill(YELLOW),
            )
            .unwrap();
        let editor = Editor::new(document);
        let (pixels, _) = render(&mut ScenePainter::default(), &editor);
        assert_pixel(&pixels, 36, 64, RED);
        assert_pixel(&pixels, 64, 64, YELLOW);
    }
    // Empty frames keep their container semantics too; classification must
    // not depend on whether a child happened to be visible in this frame.
    let mut empty = shape(NodeKind::Frame);
    empty.strokes = vec![inside_stroke()];
    empty.effects = vec![shadow(EffectKind::InnerShadow, RED)];
    let (editor, _) = editor_with(empty);
    assert_pixel(
        &render(&mut ScenePainter::default(), &editor).0,
        36,
        64,
        RED,
    );
    scope.dispose();
}
