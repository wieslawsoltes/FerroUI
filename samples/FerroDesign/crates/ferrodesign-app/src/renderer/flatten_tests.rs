use super::*;
use ferrodesign_core::{Fill, GradientStop};
use ferroui_base::media::imaging::RenderTargetBitmap;
use ferroui_base::media::FontManager;
use ferroui_base::platform::{IAssetLoader, PixelFormat, StandardAssetLoader};
use ferroui_base::{FerroLocator, PixelRect, PixelSize};

fn backend() -> Rc<dyn ferroui_base::reactive::IDisposable> {
    let scope = FerroLocator::enter_scope();
    FerroLocator::current_mutable()
        .bind::<dyn IAssetLoader>()
        .to_constant(Rc::new(StandardAssetLoader::new(None)));
    ferroui_vello::VelloPlatform::initialize();
    ferroui_harfbuzz::HarfBuzzPlatform::initialize();
    FontManager::current()
        .add_font_collection(Rc::new(ferroui_fonts_inter::InterFontCollection::new()));
    scope
}

fn pixels(editor: &Editor) -> Vec<u8> {
    pixels_with(&mut ScenePainter::default(), editor).0
}

fn pixels_with(painter: &mut ScenePainter, editor: &Editor) -> (Vec<u8>, RenderStats) {
    pixels_with_view(painter, editor, Transform::IDENTITY)
}

fn pixels_with_view(
    painter: &mut ScenePainter,
    editor: &Editor,
    view: Transform,
) -> (Vec<u8>, RenderStats) {
    let bitmap = RenderTargetBitmap::new(PixelSize::new(180, 140));
    let stats;
    {
        let mut context = bitmap.create_drawing_context();
        let transform = context.push_transform(ui_matrix(view));
        stats = painter.render_editor(
            &mut context,
            editor,
            &editor.document.pages[0].children,
            Rect::new(0.0, 0.0, 180.0, 140.0),
        );
        assert_eq!(stats.invalid_nodes, 0);
        assert_eq!(stats.unsupported_effects, 0);
        context.pop(transform);
    }
    let mut pixels = vec![0; 180 * 140 * 4];
    bitmap.copy_pixels(PixelRect::new(0, 0, 180, 140), &mut pixels, 180 * 4);
    if bitmap.format() == Some(PixelFormat::BGRA8888) {
        for pixel in pixels.chunks_exact_mut(4) {
            pixel.swap(0, 2);
        }
    }
    (pixels, stats)
}

fn equivalent(before: &[u8], after: &[u8]) {
    let bad = before
        .chunks_exact(4)
        .zip(after.chunks_exact(4))
        .filter(|(a, b)| a.iter().zip(*b).any(|(a, b)| a.abs_diff(*b) > 4))
        .count();
    if bad >= 180 * 140 / 100 {
        if let Some(directory) = std::env::var_os("FERRODESIGN_PIXEL_DIAGNOSTICS") {
            let directory = std::path::PathBuf::from(directory);
            std::fs::create_dir_all(&directory).unwrap();
            let name = std::thread::current().name().unwrap_or("flatten").replace("::", "_");
            std::fs::write(directory.join(format!("{name}-before.rgba")), before).unwrap();
            std::fs::write(directory.join(format!("{name}-after.rgba")), after).unwrap();
        }
    }
    assert!(
        bad < 180 * 140 / 100,
        "{bad} pixels differ by more than 4/255"
    );
}

#[test]
fn flatten_native_rounded_shape_preserves_gradient_stroke_and_affine_pixels() {
    let scope = backend();
    let mut document = Document::new("Shape outline");
    let mut node =
        Node::new(NodeKind::Rectangle, "Rounded", 30.0, 30.0, 90.0, 60.0).with_radius(15.0);
    node.rotation = 16.0;
    node.affine_transform = Some(Transform {
        a: 1.0,
        b: 0.0,
        c: 0.2,
        d: 1.0,
        e: 0.0,
        f: 0.0,
    });
    node.fills = vec![Fill {
        paint: Paint::LinearGradient {
            start: Point::new(0.0, 0.0),
            end: Point::new(1.0, 1.0),
            stops: vec![
                GradientStop {
                    offset: 0.0,
                    color: Color::rgb(220, 30, 70),
                },
                GradientStop {
                    offset: 1.0,
                    color: Color::rgb(20, 160, 230),
                },
            ],
        },
        opacity: 1.0,
        visible: true,
    }];
    node.strokes = vec![Stroke::new(Color::BLACK, 3.0)];
    let id = document.insert_node(1, node).unwrap();
    let mut editor = Editor::new(document);
    editor.selection = vec![id];
    let before = pixels(&editor);
    let original = editor.document.to_json().unwrap();
    flatten_selection(&mut editor).unwrap();
    assert_eq!(editor.document.nodes[&id].kind, NodeKind::Vector);
    equivalent(&before, &pixels(&editor));
    assert!(editor.undo());
    assert_eq!(editor.document.to_json().unwrap(), original);
    scope.dispose();
}

#[test]
fn flatten_native_leaf_conversion_refreshes_one_visual_and_matches_fresh_pixels() {
    let scope = backend();
    let mut document = Document::new("Scoped conversion");
    for index in 0..64 {
        document
            .insert_node(
                1,
                Node::new(
                    NodeKind::Rectangle,
                    "Shape",
                    (index % 8) as f64 * 20.0,
                    (index / 8) as f64 * 16.0,
                    16.0,
                    12.0,
                )
                .with_radius(4.0)
                .with_fill(Color::rgb(210, 50, 90)),
            )
            .unwrap();
    }
    let mut editor = Editor::new(document);
    editor.selection = vec![editor.document.pages[0].children[3]];
    let mut painter = ScenePainter::default();
    let (before, initial) = pixels_with(&mut painter, &editor);
    assert_eq!(initial.prepared_nodes, 64);
    flatten_selection(&mut editor).unwrap();
    let (after, stats) = pixels_with(&mut painter, &editor);
    assert_eq!(
        (
            stats.full_prepares,
            stats.incremental_prepares,
            stats.prepared_nodes,
            stats.rebuilt_visuals
        ),
        (0, 1, 1, 1)
    );
    assert_eq!(
        after,
        pixels(&editor),
        "cached conversion equals complete preparation"
    );
    assert!(editor.undo());
    let (undone, stats) = pixels_with(&mut painter, &editor);
    assert_eq!(stats.prepared_nodes, 1);
    assert_eq!(undone, before);
    assert!(editor.redo());
    let (redone, stats) = pixels_with(&mut painter, &editor);
    assert_eq!(stats.prepared_nodes, 1);
    assert_eq!(redone, after);
    scope.dispose();
}

#[test]
fn flatten_native_boolean_retains_hole_inherited_fill_hierarchy_and_pixels() {
    let scope = backend();
    let mut document = Document::new("Boolean outline");
    let mut boolean = Node::new(NodeKind::Boolean, "Difference", 30.0, 20.0, 100.0, 90.0);
    boolean.boolean_operation = BooleanOperation::Subtract;
    boolean.rotation = 12.0;
    let id = document.insert_node(1, boolean).unwrap();
    document
        .insert_node(
            id,
            Node::new(NodeKind::Rectangle, "Outer", 0.0, 0.0, 100.0, 90.0)
                .with_fill(Color::rgb(200, 70, 40)),
        )
        .unwrap();
    document
        .insert_node(
            id,
            Node::new(NodeKind::Ellipse, "Hole", 25.0, 20.0, 50.0, 50.0),
        )
        .unwrap();
    let mut editor = Editor::new(document);
    editor.selection = vec![id];
    let before = pixels(&editor);
    flatten_selection(&mut editor).unwrap();
    assert_eq!(editor.document.nodes.len(), 1);
    let node = &editor.document.nodes[&id];
    assert_eq!(node.fills[0].paint, Paint::Solid(Color::rgb(200, 70, 40)));
    assert!(!node.path.contains(Point::new(50.0, 45.0)));
    equivalent(&before, &pixels(&editor));
    scope.dispose();
}

#[test]
fn flatten_native_text_uses_actual_shaping_rich_fonts_and_decorations() {
    let scope = backend();
    let mut document = Document::new("Text outline");
    let mut node = Node::new(NodeKind::Text, "Outlined text", 15.0, 20.0, 150.0, 100.0).with_text(
        "office\nFerroUI",
        26.0,
        Color::rgb(230, 40, 90),
    );
    node.font_family = "fonts:Inter#Inter".into();
    node.letter_spacing = 1.5;
    node.text_align = TextAlign::Center;
    node.text_decoration = TextDecoration::Underline;
    let id = document.insert_node(1, node).unwrap();
    let mut editor = Editor::new(document);
    editor.selection = vec![id];
    let before = pixels(&editor);
    assert!(before.chunks_exact(4).filter(|pixel| pixel[3] > 0).count() > 200);
    flatten_selection(&mut editor).unwrap();
    assert!(!editor.document.nodes[&id].path.segments.is_empty());
    assert!(editor.document.nodes[&id]
        .path
        .segments
        .iter()
        .any(|segment| matches!(
            segment,
            PathSegment::QuadraticTo { .. } | PathSegment::CubicTo { .. }
        )));
    equivalent(&before, &pixels(&editor));
    scope.dispose();
}

#[test]
fn flatten_native_text_preserves_fractional_views_and_rich_run_positions() {
    let scope = backend();
    let mut document = Document::new("Fractional text outline");
    let mut node = Node::new(NodeKind::Text, "Scalable text", 12.25, 15.5, 145.0, 100.0)
        .with_text("office\nAV ffi", 22.5, Color::rgb(230, 40, 90));
    node.font_family = "fonts:Inter#Inter".into();
    node.letter_spacing = 0.375;
    node.line_height = 1.6;
    node.text_align = TextAlign::Center;
    node.text_vertical_align = TextVerticalAlign::Center;
    node.text_decoration = TextDecoration::Underline;
    node.text_runs.push(ferrodesign_core::TextRun {
        start: 0,
        end: 6,
        font_family: None,
        font_size: Some(27.25),
        font_weight: Some(600),
        color: None,
        italic: None,
        decoration: Some(TextDecoration::Strikethrough),
    });
    let id = document.insert_node(1, node).unwrap();
    let mut editor = Editor::new(document);
    editor.selection = vec![id];
    // The host supplies the camera transform. Upright fractional scales expose
    // device baseline snapping; an affine view also retains shaped run offsets.
    let views = [
        Transform::IDENTITY,
        Transform { a: 0.8, b: 0.0, c: 0.0, d: 1.1, e: 10.375, f: 7.625 },
        Transform { a: 0.95, b: 0.08, c: 0.12, d: 0.9, e: 2.25, f: 1.75 },
    ];
    let mut painter = ScenePainter::default();
    let before: Vec<_> = views
        .iter()
        .map(|view| pixels_with_view(&mut painter, &editor, *view).0)
        .collect();
    for image in &before {
        assert!(image.chunks_exact(4).filter(|pixel| pixel[3] > 0).count() > 200);
    }
    flatten_selection(&mut editor).unwrap();
    for (view, before) in views.iter().zip(&before) {
        let (after, _) = pixels_with_view(&mut painter, &editor, *view);
        equivalent(before, &after);
    }
    scope.dispose();
}

#[test]
fn flatten_native_translucent_text_overlap_requires_independent_paint_regions() {
    let scope = backend();
    for (text, spacing, decoration, color_alpha) in [
        ("HH", -30.0, TextDecoration::None, false),
        ("H", 0.0, TextDecoration::Strikethrough, true),
    ] {
        let mut document = Document::new("Text overlap alpha");
        let mut node = Node::new(
            NodeKind::Text,
            "Separate paint passes",
            30.0,
            20.0,
            140.0,
            100.0,
        )
        .with_text(text, 56.0, Color::BLACK);
        node.font_family = "fonts:Inter#Inter".into();
        node.letter_spacing = spacing;
        node.text_decoration = decoration;
        if color_alpha {
            let mut color = Color::BLACK;
            color.a = 128;
            node.fills[0].paint = Paint::Solid(color);
        } else {
            node.fills[0].opacity = 0.5;
        }
        let id = document.insert_node(1, node).unwrap();
        let mut editor = Editor::new(document);
        editor.selection = vec![id];
        let before = pixels(&editor);
        // Deliberately render the naive outline to prove the representational
        // gap independently of the public command's conservative preflight.
        let path = outline(
            &editor.document,
            &FlattenRequest {
                roots: vec![id],
                destination_world: editor.document.world_transform(id).unwrap(),
            },
        )
        .unwrap();
        let mut naive = editor.document.clone();
        let node = naive.nodes.get_mut(&id).unwrap();
        node.kind = NodeKind::Vector;
        node.path = path;
        let after = pixels(&Editor::new(naive));
        let accumulated = before
            .chunks_exact(4)
            .zip(after.chunks_exact(4))
            .filter(|(source, outline)| source[3] >= 180 && (110..=140).contains(&outline[3]))
            .count();
        assert!(accumulated > 10, "{text:?}/{decoration:?}: {accumulated} pixels demonstrate independently composited alpha");
        let snapshot = editor.document.to_json().unwrap();
        let revision = editor.document.revision;
        let history = editor.history_len();
        assert!(flatten_selection(&mut editor)
            .unwrap_err()
            .to_string()
            .contains("Translucent text"));
        assert_eq!(editor.document.to_json().unwrap(), snapshot);
        assert_eq!(editor.document.revision, revision);
        assert_eq!(editor.history_len(), history);
        assert_eq!(editor.selection, vec![id]);
    }
    scope.dispose();
}

#[test]
fn flatten_native_opaque_overlapping_text_retains_isolated_layer_opacity() {
    let scope = backend();
    let mut document = Document::new("Layer opacity");
    let mut node = Node::new(NodeKind::Text, "Isolated text", 30.0, 20.0, 140.0, 100.0).with_text(
        "HH",
        56.0,
        Color::BLACK,
    );
    node.font_family = "fonts:Inter#Inter".into();
    node.letter_spacing = -30.0;
    node.text_decoration = TextDecoration::Strikethrough;
    node.opacity = 0.5;
    let id = document.insert_node(1, node).unwrap();
    let mut editor = Editor::new(document);
    editor.selection = vec![id];
    let before = pixels(&editor);
    flatten_selection(&mut editor).unwrap();
    let after = pixels(&editor);
    assert_eq!(editor.document.nodes[&id].opacity, 0.5);
    assert!(
        before
            .chunks_exact(4)
            .filter(|pixel| pixel[3] > 100)
            .count()
            > 200
    );
    assert!(before
        .chunks_exact(4)
        .chain(after.chunks_exact(4))
        .all(|pixel| pixel[3] <= 130));
    equivalent(&before, &after);
    scope.dispose();
}

#[test]
fn flatten_native_opaque_union_preserves_overlapping_sibling_pixels() {
    let scope = backend();
    let mut document = Document::new("Union outline");
    let a = document
        .insert_node(
            1,
            Node::new(NodeKind::Rectangle, "A", 20.0, 30.0, 80.0, 60.0)
                .with_fill(Color::rgb(20, 160, 80)),
        )
        .unwrap();
    let b = document
        .insert_node(
            1,
            Node::new(NodeKind::Ellipse, "B", 70.0, 45.0, 70.0, 60.0)
                .with_fill(Color::rgb(20, 160, 80)),
        )
        .unwrap();
    let mut editor = Editor::new(document);
    editor.selection = vec![b, a];
    let before = pixels(&editor);
    assert_eq!(flatten_selection(&mut editor).unwrap(), a);
    assert_eq!(editor.document.pages[0].children, vec![a]);
    equivalent(&before, &pixels(&editor));
    scope.dispose();
}

#[test]
fn flatten_native_multi_union_preserves_evenodd_holes() {
    let scope = backend();
    let mut document = Document::new("Even-odd union");
    let mut ring = Node::new(NodeKind::Vector, "Ring", 15.0, 20.0, 100.0, 90.0)
        .with_fill(Color::rgb(30, 170, 90));
    let rectangle = |x, y, w, h| {
        VectorPath::polygon(
            &[
                Point::new(x, y),
                Point::new(x + w, y),
                Point::new(x + w, y + h),
                Point::new(x, y + h),
            ],
            true,
        )
    };
    ring.path = rectangle(0.0, 0.0, 100.0, 90.0);
    ring.path
        .segments
        .extend(rectangle(25.0, 25.0, 50.0, 40.0).segments);
    ring.path.fill_rule = FillRule::EvenOdd;
    let a = document.insert_node(1, ring).unwrap();
    let b = document
        .insert_node(
            1,
            Node::new(NodeKind::Ellipse, "Outside", 100.0, 60.0, 50.0, 50.0)
                .with_fill(Color::rgb(30, 170, 90)),
        )
        .unwrap();
    let mut editor = Editor::new(document);
    editor.selection = vec![a, b];
    let before = pixels(&editor);
    flatten_selection(&mut editor).unwrap();
    let node = &editor.document.nodes[&a];
    let hole = editor
        .document
        .world_transform(a)
        .unwrap()
        .inverse()
        .unwrap()
        .apply(Point::new(65.0, 65.0));
    assert!(
        !node.path.contains(hole),
        "the even-odd inner contour remains a hole"
    );
    equivalent(&before, &pixels(&editor));
    scope.dispose();
}

#[test]
fn flatten_native_round_shape_effects_keep_rendered_appearance() {
    let scope = backend();
    let mut document = Document::new("Effect outline");
    let mut node = Node::new(NodeKind::Rectangle, "Shadowed", 45.0, 35.0, 70.0, 55.0)
        .with_radius(12.0)
        .with_fill(Color::rgb(230, 150, 40));
    let mut shadow = Effect::for_kind(EffectKind::DropShadow);
    shadow.radius = 4.0;
    shadow.spread = 1.0;
    shadow.offset = Point::new(4.0, 5.0);
    node.effects = vec![shadow];
    let id = document.insert_node(1, node).unwrap();
    let mut editor = Editor::new(document);
    editor.selection = vec![id];
    let before = pixels(&editor);
    flatten_selection(&mut editor).unwrap();
    equivalent(&before, &pixels(&editor));
    scope.dispose();
}

fn naive_vector_pixels(editor: &Editor, id: NodeId) -> Vec<u8> {
    let path = outline(
        &editor.document,
        &FlattenRequest {
            roots: vec![id],
            destination_world: editor.document.world_transform(id).unwrap(),
        },
    )
    .unwrap();
    let mut naive = editor.document.clone();
    let node = naive.nodes.get_mut(&id).unwrap();
    node.kind = NodeKind::Vector;
    node.path = path;
    pixels(&Editor::new(naive))
}

fn assert_atomic_rejection(editor: &mut Editor, reason: &str) {
    let before = editor.document.to_json().unwrap();
    let selection = editor.selection.clone();
    let revision = editor.document.revision;
    let history = editor.history_len();
    let redo = editor.can_redo();
    let error = flatten_selection(editor).unwrap_err().to_string();
    assert!(error.contains(reason), "{error}");
    assert_eq!(editor.document.to_json().unwrap(), before);
    assert_eq!(editor.document.revision, revision);
    assert_eq!(editor.history_len(), history);
    assert_eq!(editor.selection, selection);
    assert_eq!(editor.can_redo(), redo);
}

#[test]
fn flatten_native_box_shadow_exposes_interior_through_translucent_fills_and_rejects() {
    let scope = backend();
    for fill_mode in 0..3 {
        let mut document = Document::new("Box-shadow interior");
        let mut node = Node::new(
            NodeKind::Rectangle,
            "Clipped shadow",
            40.0,
            30.0,
            80.0,
            70.0,
        )
        .with_radius(12.0)
        .with_fill(Color::rgb(0, 0, 255));
        match fill_mode {
            0 => node.fills.clear(),
            1 => node.fills[0].opacity = 0.5,
            _ => {
                node.fills[0].paint = Paint::Solid(Color {
                    a: 128,
                    ..Color::rgb(0, 0, 255)
                });
            }
        }
        node.effects = vec![Effect::shadow(
            Color::rgb(255, 0, 0),
            Point::new(4.0, 0.0),
            0.0,
        )];
        let id = document.insert_node(1, node).unwrap();
        let mut editor = Editor::new(document);
        editor.add_node(NodeKind::Ellipse, 0.0, 0.0, 10.0, 10.0);
        assert!(editor.undo());
        editor.selection = vec![id];
        let before = pixels(&editor);
        // Render a deliberately naive conversion independently of preflight.
        // A path shadow fills the silhouette behind the paint; BoxShadows
        // excludes that interior even when the paint is absent or translucent.
        let after = naive_vector_pixels(&editor, id);
        let exposed = before
            .chunks_exact(4)
            .zip(after.chunks_exact(4))
            .filter(|(source, vector)| source[3] <= 128 && vector[3] >= 250)
            .count();
        assert!(
            exposed > 200,
            "fill mode {fill_mode}: {exposed} interior shadow pixels"
        );
        assert_atomic_rejection(&mut editor, "Box-shaped drop shadows");
        assert!(editor.can_redo());
    }
    scope.dispose();
}

#[test]
fn flatten_native_empty_container_inner_shadow_stroke_order_changes_and_rejects() {
    let scope = backend();
    for kind in [NodeKind::Frame, NodeKind::Section] {
        let mut document = Document::new("Container inner shadow");
        let mut node =
            Node::new(kind, "Container ordering", 40.0, 30.0, 80.0, 70.0).with_fill(Color::WHITE);
        let mut stroke = Stroke::new(Color::rgb(0, 255, 0), 8.0);
        stroke.align = StrokeAlign::Inside;
        node.strokes = vec![stroke];
        node.effects = vec![Effect {
            kind: EffectKind::InnerShadow,
            color: Color::rgb(255, 0, 0),
            offset: Point::new(16.0, 0.0),
            radius: 0.0,
            spread: 0.0,
            visible: true,
        }];
        let id = document.insert_node(1, node).unwrap();
        let mut editor = Editor::new(document);
        editor.add_node(NodeKind::Ellipse, 0.0, 0.0, 10.0, 10.0);
        assert!(editor.undo());
        editor.selection = vec![id];
        let before = pixels(&editor);
        let after = naive_vector_pixels(&editor, id);
        let sample = (60 * 180 + 44) * 4;
        assert_eq!(
            &before[sample..sample + 4],
            &[255, 0, 0, 255],
            "{kind:?} shadows cover strokes"
        );
        assert_eq!(
            &after[sample..sample + 4],
            &[0, 255, 0, 255],
            "vector strokes cover inner shadows"
        );
        assert_atomic_rejection(&mut editor, "inner shadows cover its strokes");
        assert!(editor.can_redo());
    }
    scope.dispose();
}

#[test]
fn flatten_native_box_shadow_outward_spread_rounds_square_corners_and_rejects() {
    let scope = backend();
    for kind in [EffectKind::DropShadow, EffectKind::InnerShadow] {
        let mut document = Document::new("Square shadow corners");
        let mut node = Node::new(NodeKind::Rectangle, "Square box", 45.0, 35.0, 70.0, 55.0)
            .with_fill(Color::WHITE);
        let inset = kind == EffectKind::InnerShadow;
        node.effects = vec![Effect {
            kind,
            color: Color::rgb(255, 0, 0),
            offset: if inset {
                Point::new(20.0, 20.0)
            } else {
                Point::default()
            },
            radius: 0.0,
            spread: if inset { -12.0 } else { 12.0 },
            visible: true,
        }];
        let id = document.insert_node(1, node).unwrap();
        let mut editor = Editor::new(document);
        editor.add_node(NodeKind::Ellipse, 0.0, 0.0, 10.0, 10.0);
        assert!(editor.undo());
        editor.selection = vec![id];
        let before = pixels(&editor);
        let after = naive_vector_pixels(&editor, id);
        // Outsetting a box retains its sharp outer (or hole) corner. A
        // round-join vector stroke rounds that very corner. Sample well away
        // from antialiased boundaries so the distinction is unambiguous.
        let (x, y, box_pixel, vector_pixel) = if inset {
            (54, 44, [255, 255, 255, 255], [255, 0, 0, 255])
        } else {
            (34, 24, [255, 0, 0, 255], [0, 0, 0, 0])
        };
        let sample = (y * 180 + x) * 4;
        assert_eq!(
            &before[sample..sample + 4],
            &box_pixel,
            "{kind:?} box corner"
        );
        assert_eq!(
            &after[sample..sample + 4],
            &vector_pixel,
            "{kind:?} vector corner"
        );
        assert_atomic_rejection(&mut editor, "Outward box-shadow spread");
        assert!(editor.can_redo());
    }
    scope.dispose();
}

#[test]
fn flatten_native_mixed_colour_glyph_text_rejects_partial_geometry_atomically() {
    use ferroui_base::media::fonts::{EmbeddedFontCollection, IFontCollection};
    use ferroui_base::utilities::Uri;
    let scope = backend();
    // Load a real colour font through the public font collection API; the
    // ordinary first glyph makes a silently partial outline look nonempty.
    let font_file = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../../../src/Skia/FerroUI.Skia/test_assets/assets/TwitterColorEmoji-SVGinOT.ttf")
        .canonicalize()
        .unwrap();
    let collection = Rc::new(EmbeddedFontCollection::new(
        Uri::absolute("fonts:FlattenColour").unwrap(),
        Uri::absolute(&format!("file://{}", font_file.display())).unwrap(),
    ));
    assert_eq!(collection.count(), 1);
    let family = format!("fonts:FlattenColour#{}", collection.get(0).name());
    FontManager::current().add_font_collection(collection);
    let mut document = Document::new("Complete text outline");
    let mut node = Node::new(NodeKind::Text, "Mixed glyphs", 10.0, 10.0, 150.0, 80.0).with_text(
        "a😀",
        30.0,
        Color::BLACK,
    );
    node.font_family = "fonts:Inter#Inter".into();
    node.text_runs.push(ferrodesign_core::TextRun {
        start: 1,
        end: "a😀".len(),
        font_family: Some(family),
        font_size: None,
        font_weight: None,
        color: None,
        italic: None,
        decoration: None,
    });
    let text = make_text_layout(&node, solid_brush(Color::BLACK, 1.0));
    assert!(text.layout.build_geometry(text.origin).bounds().width > 0.0);
    assert!(text.layout.try_build_geometry(text.origin).is_none());
    let id = document.insert_node(1, node).unwrap();
    let mut editor = Editor::new(document);
    editor.add_node(NodeKind::Rectangle, 0.0, 0.0, 10.0, 10.0);
    assert!(editor.undo());
    editor.selection = vec![id];
    let before = editor.document.to_json().unwrap();
    let revision = editor.document.revision;
    let history = editor.history_len();
    let error = flatten_selection(&mut editor).unwrap_err().to_string();
    assert!(error.contains("glyph outlines"), "{error}");
    assert_eq!(editor.document.to_json().unwrap(), before);
    assert_eq!(editor.document.revision, revision);
    assert_eq!(editor.selection, vec![id]);
    assert_eq!(editor.history_len(), history);
    assert!(editor.can_redo());
    scope.dispose();
}
