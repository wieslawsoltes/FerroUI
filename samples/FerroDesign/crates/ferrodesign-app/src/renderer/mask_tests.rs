//! Actual active-backend pixels for sibling masks and pass-through groups.
use super::*;
use ferrodesign_core::{CompositingEdit, Fill, GradientStop};
use ferroui_base::media::imaging::RenderTargetBitmap;
use ferroui_base::platform::{IPlatformRenderInterface, PixelFormat};
use ferroui_base::{FerroLocator, PixelRect, PixelSize};

const SIZE: i32 = 128;

fn backend() -> Rc<dyn ferroui_base::reactive::IDisposable> {
    let scope = FerroLocator::enter_scope();
    FerroLocator::current_mutable()
        .bind::<dyn IPlatformRenderInterface>()
        .to_constant(Rc::new(ferroui_vello::PlatformRenderInterface::default()));
    scope
}

fn render(painter: &mut ScenePainter, editor: &Editor) -> (Vec<u8>, RenderStats) {
    render_on_host(painter, editor, None)
}

fn render_on_host(
    painter: &mut ScenePainter,
    editor: &Editor,
    host: Option<Color>,
) -> (Vec<u8>, RenderStats) {
    let bitmap = RenderTargetBitmap::new(PixelSize::new(SIZE, SIZE));
    let stats;
    {
        let mut context = bitmap.create_drawing_context();
        if let Some(color) = host {
            context.fill_rectangle(
                &solid_brush(color, 1.0),
                UiRect::new(0.0, 0.0, SIZE as f64, SIZE as f64),
                0.0,
            );
        }
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
    assert_eq!(stats.unsupported_effects, 0, "{stats:?}");
    assert_eq!(stats.invalid_nodes, 0);
    (pixels, stats)
}

fn pixel(data: &[u8], x: usize, y: usize) -> [u8; 4] {
    let offset = (y * SIZE as usize + x) * 4;
    data[offset..offset + 4].try_into().unwrap()
}

fn near(data: &[u8], x: usize, y: usize, expected: [u8; 4]) {
    let actual = pixel(data, x, y);
    assert!(
        actual.iter().zip(expected).all(|(a, b)| a.abs_diff(b) <= 2),
        "at({x},{y}) {actual:?} != {expected:?}"
    );
}

fn rectangle(color: Color) -> Node {
    Node::new(NodeKind::Rectangle, "Layer", 0.0, 0.0, 128.0, 128.0).with_fill(color)
}

fn mask(color: Color, kind: MaskType) -> Node {
    let mut node = rectangle(color);
    node.is_mask = true;
    node.mask_type = kind;
    node
}

#[test]
fn masks_imported_svg_preserves_alpha_luminance_and_clip_geometry_pixels() {
    let scope = backend();
    let source = r##"<svg width="128" height="128">
        <defs>
            <mask id="a" mask-type="alpha" maskUnits="userSpaceOnUse"
                  x="0" y="0" width="128" height="128">
                <rect width="32" height="128" fill="lime" opacity=".5"/>
            </mask>
            <mask id="l" maskUnits="userSpaceOnUse"
                  x="0" y="0" width="128" height="128">
                <rect x="32" width="32" height="128" fill="lime" opacity=".5"/>
            </mask>
            <clipPath id="v">
                <path d="M64 0H96V128H64Z" fill="none"
                      stroke="black" stroke-width="40" opacity="0"/>
            </clipPath>
        </defs>
        <rect width="128" height="128" fill="red" mask="url(#a)"/>
        <rect width="128" height="128" fill="red" mask="url(#l)"/>
        <rect width="128" height="128" fill="red" clip-path="url(#v)"/>
    </svg>"##;
    let document = Document::from_svg(source, "SVG mask pixels").unwrap();
    let (pixels, _) = render(&mut ScenePainter::default(), &Editor::new(document));
    near(&pixels, 16, 64, [128, 0, 0, 128]);
    near(&pixels, 48, 64, [92, 0, 0, 92]);
    // SVG clipPath ignores paint, opacity and strokes, unlike an authored
    // vector mask: the referenced path contributes its filled interior only.
    near(&pixels, 80, 64, [255, 0, 0, 255]);
    near(&pixels, 112, 64, [0, 0, 0, 0]);
    scope.dispose();
}

#[test]
fn masks_and_unmasked_blends_use_document_backdrops_consistently_over_host_decoration() {
    let scope = backend();
    for authored_backdrop in [false, true] {
        let mut document = Document::new("Document blend boundary");
        if authored_backdrop {
            document
                .insert_node(1, rectangle(Color::rgb(128, 128, 255)))
                .unwrap();
        }
        let mut source = mask(Color::WHITE, MaskType::Vector);
        source.width = 64.0;
        source.visible = false;
        let source = document.insert_node(1, source).unwrap();
        let mut target = rectangle(Color::rgba(255, 0, 0, 128));
        target.blend_mode = BlendMode::Multiply;
        let target = document.insert_node(1, target).unwrap();
        let mut editor = Editor::new(document);
        let mut painter = ScenePainter::default();
        for kind in [
            None,
            Some(MaskType::Vector),
            Some(MaskType::Alpha),
            Some(MaskType::Luminance),
        ] {
            editor.update_node(source, "Change equivalent mask", |node| {
                node.visible = kind.is_some();
                node.mask_type = kind.unwrap_or(MaskType::Alpha);
            });
            let expected = if authored_backdrop {
                [128, 64, 127, 255]
            } else {
                [128, 0, 0, 128]
            };
            near(&render(&mut painter, &editor).0, 20, 40, expected);
            let host = render_on_host(&mut painter, &editor, Some(Color::rgb(64, 96, 128))).0;
            near(
                &host,
                20,
                40,
                if authored_backdrop {
                    [128, 64, 127, 255]
                } else {
                    [160, 48, 64, 255]
                },
            );
            if kind.is_some() {
                near(
                    &host,
                    100,
                    40,
                    if authored_backdrop {
                        [128, 128, 255, 255]
                    } else {
                        [64, 96, 128, 255]
                    },
                );
            }
        }
        editor.update_node(source, "Remove visible mask", |node| node.visible = false);
        editor
            .edit_compositing(&[target], CompositingEdit::BlendMode(BlendMode::Normal))
            .unwrap();
        render(&mut painter, &editor);
        assert!(
            !painter.isolates_document,
            "Normal-only scenes keep their direct rendering path after a scoped blend edit"
        );
        editor.undo();
        render(&mut painter, &editor);
        assert!(
            painter.isolates_document,
            "Undo restores the cached document-isolation requirement"
        );
    }
    scope.dispose();
}

#[test]
fn masks_picking_reuses_rendered_coverage_and_refreshes_before_repaint_or_undo() {
    let scope = backend();
    let mut document = Document::new("Mask picking");
    let mut source = mask(Color::rgba(255, 255, 255, 128), MaskType::Alpha);
    source.width = 64.0;
    let source = document.insert_node(1, source).unwrap();
    let target = document
        .insert_node(1, rectangle(Color::rgb(255, 0, 0)))
        .unwrap();
    let mut editor = Editor::new(document);
    let mut painter = ScenePainter::default();
    render(&mut painter, &editor);
    let roots = editor.document.pages[0].children.clone();
    let rasterizations = painter.backdrops.rasterizations();
    for _ in 0..10 {
        assert!(painter.mask_contains(&editor, &roots, target, Point::new(20.0, 20.0)));
        assert!(!painter.mask_contains(&editor, &roots, target, Point::new(100.0, 20.0)));
    }
    assert_eq!(
        painter.backdrops.rasterizations(),
        rasterizations,
        "Repeated pointer queries reuse the rendered mask surface and alpha"
    );
    assert!(
        painter.mask_contains(&editor, &roots, source, Point::new(20.0, 20.0)),
        "A mask source remains independently selectable"
    );
    editor.update_node(source, "Move mask before repaint", |node| node.x = 64.0);
    assert!(!painter.mask_contains(&editor, &roots, target, Point::new(20.0, 20.0)));
    assert!(painter.mask_contains(&editor, &roots, target, Point::new(100.0, 20.0)));
    editor.undo();
    assert!(painter.mask_contains(&editor, &roots, target, Point::new(20.0, 20.0)));
    assert!(!painter.mask_contains(&editor, &roots, target, Point::new(100.0, 20.0)));
    editor
        .edit_compositing(&[source], CompositingEdit::MaskEnabled(false))
        .unwrap();
    assert!(painter.mask_contains(&editor, &roots, target, Point::new(100.0, 20.0)));
    editor.undo();
    assert!(!painter.mask_contains(&editor, &roots, target, Point::new(100.0, 20.0)));
    scope.dispose();
}

#[test]
fn masks_picking_respects_gradient_luminance_and_cumulative_transparent_pixels() {
    let scope = backend();
    let mut document = Document::new("Pixel coverage picking");
    let mut source = mask(Color::WHITE, MaskType::Alpha);
    source.fills[0].paint = Paint::LinearGradient {
        start: Point::new(0.0, 0.0),
        end: Point::new(1.0, 0.0),
        stops: vec![
            GradientStop {
                offset: 0.0,
                color: Color::TRANSPARENT,
            },
            GradientStop {
                offset: 0.5,
                color: Color::TRANSPARENT,
            },
            GradientStop {
                offset: 1.0,
                color: Color::WHITE,
            },
        ],
    };
    let source = document.insert_node(1, source).unwrap();
    let second = document
        .insert_node(1, mask(Color::WHITE, MaskType::Alpha))
        .unwrap();
    let target = document
        .insert_node(1, rectangle(Color::rgb(255, 0, 0)))
        .unwrap();
    let mut editor = Editor::new(document);
    let roots = editor.document.pages[0].children.clone();
    let mut painter = ScenePainter::default();
    assert!(!painter.mask_contains(&editor, &roots, target, Point::new(20.0, 20.0)));
    assert!(painter.mask_contains(&editor, &roots, target, Point::new(110.0, 20.0)));
    editor.update_node(source, "Black luminance mask", |node| {
        node.mask_type = MaskType::Luminance;
        node.fills = vec![Fill::solid(Color::BLACK)];
    });
    assert!(!painter.mask_contains(&editor, &roots, target, Point::new(110.0, 20.0)));
    editor.update_nodes(&[source, second], "Very low alpha chain", |node| {
        node.mask_type = MaskType::Alpha;
        node.fills = vec![Fill::solid(Color::rgba(255, 255, 255, 1))];
    });
    near(&render(&mut painter, &editor).0, 40, 40, [0, 0, 0, 0]);
    assert!(
        !painter.mask_contains(&editor, &roots, target, Point::new(40.0, 40.0)),
        "Two individually nonzero masks can produce a zero-coverage output pixel"
    );
    scope.dispose();
}

#[test]
fn masks_picking_vector_holes_and_isolated_roots_follow_rendered_scopes() {
    let scope = backend();
    let mut document = Document::new("Vector scope picking");
    let mut group = Node::new(NodeKind::Group, "Mask group", 0.0, 0.0, 128.0, 128.0);
    group.fills.clear();
    let group = document.insert_node(1, group).unwrap();
    let mut source = Node::new(NodeKind::Ellipse, "Ring", 32.0, 32.0, 32.0, 32.0);
    source.fills.clear();
    source.strokes = vec![Stroke::new(Color::rgba(0, 0, 0, 1), 10.0)];
    source.strokes[0].align = StrokeAlign::Outside;
    source.is_mask = true;
    source.mask_type = MaskType::Vector;
    document.insert_node(group, source).unwrap();
    let target = document
        .insert_node(group, rectangle(Color::rgb(255, 0, 0)))
        .unwrap();
    let unrelated = document
        .insert_node(1, rectangle(Color::rgb(0, 0, 255)))
        .unwrap();
    let editor = Editor::new(document);
    let roots = &editor.document.pages[0].children;
    let mut painter = ScenePainter::default();
    assert!(painter.mask_contains(&editor, roots, target, Point::new(27.0, 48.0)));
    assert!(!painter.mask_contains(&editor, roots, target, Point::new(48.0, 48.0)));
    assert!(
        painter.mask_contains(&editor, roots, unrelated, Point::new(48.0, 48.0)),
        "The mask scope ends at its parent boundary"
    );
    assert!(
        painter.mask_contains(&editor, &[target], target, Point::new(48.0, 48.0)),
        "An isolated presentation root excludes external sibling mask scopes"
    );
    scope.dispose();
}

#[test]
fn masks_alpha_vector_luminance_use_distinct_coverage_and_preserve_history() {
    let scope = backend();
    let mut document = Document::new("Mask modes");
    let mut source = mask(Color::rgba(0, 255, 0, 128), MaskType::Alpha);
    source.width = 64.0;
    let source = document.insert_node(1, source).unwrap();
    document
        .insert_node(1, rectangle(Color::rgb(255, 0, 0)))
        .unwrap();
    let mut editor = Editor::new(document);
    let mut painter = ScenePainter::default();
    let (alpha, _) = render(&mut painter, &editor);
    near(&alpha, 20, 20, [128, 0, 0, 128]);
    near(&alpha, 100, 20, [0, 0, 0, 0]);
    editor
        .edit_compositing(&[source], CompositingEdit::MaskType(MaskType::Vector))
        .unwrap();
    let (vector, stats) = render(&mut painter, &editor);
    near(&vector, 20, 20, [255, 0, 0, 255]);
    assert_eq!(stats.incremental_prepares, 1);
    editor
        .edit_compositing(&[source], CompositingEdit::MaskType(MaskType::Luminance))
        .unwrap();
    let (luma, _) = render(&mut painter, &editor);
    near(&luma, 20, 20, [92, 0, 0, 92]);
    assert_eq!(luma, render(&mut ScenePainter::default(), &editor).0);
    editor.undo();
    assert_eq!(vector, render(&mut painter, &editor).0);
    editor.redo();
    assert_eq!(luma, render(&mut painter, &editor).0);
    editor.update_node(source, "Zero mask opacity", |node| node.opacity = 0.0);
    near(&render(&mut painter, &editor).0, 20, 20, [0, 0, 0, 0]);
    editor.update_node(source, "Empty mask coverage", |node| {
        node.opacity = 1.0;
        node.width = 0.0;
    });
    near(&render(&mut painter, &editor).0, 20, 20, [0, 0, 0, 0]);
    editor.update_node(source, "Hide mask", |node| node.visible = false);
    near(&render(&mut painter, &editor).0, 20, 20, [255, 0, 0, 255]);
    scope.dispose();
}

#[test]
fn masks_vector_includes_stroke_regions_without_filling_their_holes() {
    let scope = backend();
    let mut document = Document::new("Stroke mask");
    let mut source = Node::new(NodeKind::Ellipse, "Ring", 32.0, 32.0, 32.0, 32.0);
    source.fills.clear();
    source.strokes = vec![Stroke::new(Color::rgba(0, 0, 0, 1), 10.0)];
    source.strokes[0].align = StrokeAlign::Outside;
    source.opacity = 0.2;
    source.is_mask = true;
    source.mask_type = MaskType::Vector;
    document.insert_node(1, source).unwrap();
    document
        .insert_node(1, rectangle(Color::rgb(255, 0, 0)))
        .unwrap();
    let (pixels, _) = render(&mut ScenePainter::default(), &Editor::new(document));
    near(&pixels, 27, 48, [255, 0, 0, 255]);
    near(&pixels, 48, 48, [0, 0, 0, 0]);
    near(&pixels, 10, 48, [0, 0, 0, 0]);
    scope.dispose();
}

#[test]
fn masks_alpha_gradient_and_transparent_luminance_are_not_binary_clips() {
    let scope = backend();
    let mut document = Document::new("Gradient mask");
    let mut source = mask(Color::WHITE, MaskType::Alpha);
    source.fills[0].paint = Paint::LinearGradient {
        start: Point::new(0.0, 0.0),
        end: Point::new(1.0, 0.0),
        stops: vec![
            GradientStop {
                offset: 0.0,
                color: Color::TRANSPARENT,
            },
            GradientStop {
                offset: 1.0,
                color: Color::WHITE,
            },
        ],
    };
    let source = document.insert_node(1, source).unwrap();
    document
        .insert_node(1, rectangle(Color::rgb(255, 0, 0)))
        .unwrap();
    let mut editor = Editor::new(document);
    let (pixels, _) = render(&mut ScenePainter::default(), &editor);
    assert!(pixel(&pixels, 24, 40)[3] < pixel(&pixels, 80, 40)[3]);
    assert!(pixel(&pixels, 24, 40)[3] > 0 && pixel(&pixels, 80, 40)[3] < 255);
    editor.update_node(source, "Black luminance", |node| {
        node.mask_type = MaskType::Luminance;
        node.fills = vec![Fill::solid(Color::BLACK)];
    });
    near(
        &render(&mut ScenePainter::default(), &editor).0,
        40,
        40,
        [0, 0, 0, 0],
    );
    scope.dispose();
}

#[test]
fn masks_cumulative_sibling_scopes_apply_each_alpha_once_and_reuse_cache() {
    let scope = backend();
    let mut document = Document::new("Chained masks");
    document
        .insert_node(1, mask(Color::rgba(255, 255, 255, 128), MaskType::Alpha))
        .unwrap();
    let mut earlier = rectangle(Color::rgb(0, 255, 0));
    earlier.width = 50.0;
    document.insert_node(1, earlier).unwrap();
    document
        .insert_node(1, mask(Color::rgba(255, 255, 255, 128), MaskType::Alpha))
        .unwrap();
    let mut later = rectangle(Color::rgb(255, 0, 0));
    later.x = 70.0;
    later.width = 50.0;
    document.insert_node(1, later).unwrap();
    let editor = Editor::new(document);
    let mut painter = ScenePainter::default();
    let (pixels, first) = render(&mut painter, &editor);
    near(&pixels, 20, 30, [0, 128, 0, 128]);
    near(&pixels, 90, 30, [64, 0, 0, 64]);
    assert_eq!(first.backdrop_rasterizations, 2);
    let (cached, second) = render(&mut painter, &editor);
    assert_eq!(pixels, cached);
    assert_eq!(second.backdrop_rasterizations, 0);
    assert!(second.backdrop_cache_hits > 0);
    scope.dispose();
}

#[test]
fn masks_preserve_child_blending_with_external_backdrop() {
    let scope = backend();
    let mut document = Document::new("Masked blend");
    document
        .insert_node(1, rectangle(Color::rgb(128, 128, 255)))
        .unwrap();
    document
        .insert_node(1, mask(Color::rgba(255, 255, 255, 128), MaskType::Alpha))
        .unwrap();
    let mut child = rectangle(Color::rgb(255, 0, 0));
    child.blend_mode = BlendMode::Multiply;
    document.insert_node(1, child).unwrap();
    near(
        &render(&mut ScenePainter::default(), &Editor::new(document)).0,
        40,
        40,
        [128, 64, 127, 255],
    );
    scope.dispose();
}

#[test]
fn masks_group_boundary_transform_and_incremental_source_changes_match_fresh_pixels() {
    let scope = backend();
    let mut document = Document::new("Nested transformed mask");
    let mut group = Node::new(NodeKind::Group, "Group", 10.0, 10.0, 90.0, 90.0);
    group.fills.clear();
    group.blend_mode = BlendMode::PassThrough;
    group.opacity = 0.7;
    group.clip_content = true;
    group.corner_radius = 10.0;
    let group = document.insert_node(1, group).unwrap();
    let mut source = mask(Color::rgba(255, 255, 255, 160), MaskType::Alpha);
    source.width = 45.0;
    let source = document.insert_node(group, source).unwrap();
    document
        .insert_node(group, rectangle(Color::rgb(255, 0, 0)))
        .unwrap();
    let mut outside = rectangle(Color::rgb(0, 0, 255));
    outside.x = 105.0;
    outside.width = 20.0;
    document.insert_node(1, outside).unwrap();
    let mut editor = Editor::new(document);
    let mut painter = ScenePainter::default();
    for edit in 0..5 {
        if edit > 0 {
            editor.update_node(source, "Move mask", |node| {
                node.x += 6.0;
                node.rotation += 7.0;
            });
        }
        let (pixels, _) = render(&mut painter, &editor);
        assert_eq!(pixels, render(&mut ScenePainter::default(), &editor).0);
        near(&pixels, 115, 30, [0, 0, 255, 255]);
        // This lower-right point remains outside the rotating source in every
        // iteration; its upper-right counterpart enters the source at 28°.
        near(&pixels, 95, 95, [0, 0, 0, 0]);
    }
    scope.dispose();
}

#[test]
fn pass_through_fractional_group_opacity_interpolates_the_completed_blended_result_once() {
    let scope = backend();
    let mut document = Document::new("Pass through");
    document
        .insert_node(1, rectangle(Color::rgb(128, 128, 255)))
        .unwrap();
    let mut group = rectangle(Color::TRANSPARENT);
    group.kind = NodeKind::Group;
    group.fills.clear();
    group.blend_mode = BlendMode::PassThrough;
    group.opacity = 0.5;
    let group = document.insert_node(1, group).unwrap();
    for (color, x) in [(Color::rgb(255, 0, 0), 0.0), (Color::rgb(0, 0, 255), 40.0)] {
        let mut child = rectangle(color);
        child.x = x;
        child.width = 70.0;
        child.blend_mode = BlendMode::Multiply;
        document.insert_node(group, child).unwrap();
    }
    let mut editor = Editor::new(document);
    let mut painter = ScenePainter::default();
    let (pixels, _) = render(&mut painter, &editor);
    near(&pixels, 20, 40, [128, 64, 128, 255]);
    near(&pixels, 50, 40, [64, 64, 128, 255]);
    near(&pixels, 90, 40, [64, 64, 255, 255]);
    editor
        .edit_compositing(&[group], CompositingEdit::BlendMode(BlendMode::Normal))
        .unwrap();
    let (isolated, _) = render(&mut painter, &editor);
    near(&isolated, 20, 40, [192, 64, 128, 255]);
    editor.undo();
    assert_eq!(pixels, render(&mut painter, &editor).0);
    scope.dispose();
}

#[test]
fn pass_through_translucent_backdrop_alpha_is_not_added_twice() {
    let scope = backend();
    let mut document = Document::new("Premultiplied backdrop");
    document
        .insert_node(1, rectangle(Color::rgba(0, 0, 255, 128)))
        .unwrap();
    let mut group = rectangle(Color::TRANSPARENT);
    group.kind = NodeKind::Group;
    group.fills.clear();
    group.blend_mode = BlendMode::PassThrough;
    group.opacity = 0.5;
    let group = document.insert_node(1, group).unwrap();
    let mut child = rectangle(Color::rgb(255, 255, 0));
    child.blend_mode = BlendMode::Multiply;
    document.insert_node(group, child).unwrap();
    near(
        &render(&mut ScenePainter::default(), &Editor::new(document)).0,
        40,
        40,
        [64, 64, 64, 192],
    );
    scope.dispose();
}
