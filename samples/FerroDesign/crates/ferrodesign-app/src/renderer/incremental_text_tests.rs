//! Inline text is painted by its native editor while ScenePainter suppresses
//! that exact leaf. These tests compare the remaining scene with a fresh full
//! preparation, including preview/history, ancestors and conservative exits.
use super::tests::{equal_pixels, equals_fresh, rectangles, render, render_roots};
use super::*;
use ferrodesign_core::Fill;
use ferroui_base::FerroLocator;
use ferroui_base::media::FontManager;
use ferroui_base::platform::{IAssetLoader, StandardAssetLoader};

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

fn text() -> Node {
    let mut node = Node::new(NodeKind::Text, "Inline text", 8.0, 8.0, 100.0, 64.0).with_text(
        "A",
        20.0,
        Color::BLACK,
    );
    node.font_family = "fonts:Inter#Inter".into();
    node
}

fn equals_suppressed(
    painter: &mut ScenePainter,
    editor: &Editor,
    edited: NodeId,
) -> (Vec<u8>, RenderStats) {
    let result = render(painter, editor, 0.0, true);
    let mut reference = ScenePainter::default();
    reference.set_editing_text(Some(edited));
    equal_pixels(&result.0, &render(&mut reference, editor, 0.0, false).0);
    assert_eq!(
        painter.rendered_editor_bounds(editor, &editor.document.pages[0].children),
        reference.rendered_bounds(&editor.document, &editor.document.pages[0].children)
    );
    result
}

fn no_visual_preparation(stats: RenderStats) {
    assert_eq!(
        (
            stats.full_prepares,
            stats.incremental_prepares,
            stats.prepared_nodes,
            stats.rebuilt_visuals,
        ),
        (0, 1, 0, 0)
    );
}

#[test]
fn suppressed_inline_text_previews_in_ten_thousand_nodes_keep_scene_and_history() {
    let scope = backend();
    let (rectangles, edited) = rectangles(10_000);
    let mut document = rectangles.document;
    let node = document.nodes.get_mut(&edited).unwrap();
    let id = node.id;
    let parent = node.parent;
    *node = text();
    node.id = id;
    node.parent = parent;
    let mut editor = Editor::new(document);
    let mut painter = ScenePainter::default();
    let (_, initial) = render(&mut painter, &editor, 0.0, true);
    assert_eq!((initial.full_prepares, initial.prepared_nodes), (1, 10_000));

    painter.set_editing_text(Some(edited));
    let (suppressed, started) = render(&mut painter, &editor, 0.0, true);
    assert_eq!((started.full_prepares, started.prepared_nodes), (1, 9_999));
    assert!(!painter.prepared_indices.contains_key(&edited));
    let original_text = editor.document.nodes[&edited].text.clone();
    let history = editor.history_len();
    editor.begin_node_transaction(&[edited], "Inline text");
    for value in ["AB", "AB\nC", "AB\nCDEF"] {
        editor.document.nodes.get_mut(&edited).unwrap().text = value.into();
        editor.refresh_preview_nodes(&[edited]);
        let (pixels, stats) = render(&mut painter, &editor, 0.0, true);
        equal_pixels(&pixels, &suppressed);
        no_visual_preparation(stats);
        assert!(stats.root_query_reused);
        assert_eq!(stats.root_bounds_tests, 0);
        assert_eq!(stats.total_full_prepares, 2);
        assert_eq!(stats.total_prepared_nodes, 19_999);
        assert_eq!(stats.total_rebuilt_visuals, initial.total_rebuilt_visuals);
        assert_eq!(editor.history_len(), history);
    }
    equal_pixels(
        &equals_suppressed(&mut painter, &editor, edited).0,
        &suppressed,
    );
    assert!(editor.commit_transaction());
    assert_eq!(editor.history_len(), history + 1);
    let committed = editor.document.nodes[&edited].text.clone();
    assert_eq!(render(&mut painter, &editor, 0.0, true).1.full_prepares, 0);

    assert!(editor.undo());
    assert_eq!(editor.document.nodes[&edited].text, original_text);
    no_visual_preparation(render(&mut painter, &editor, 0.0, true).1);
    assert!(editor.redo());
    assert_eq!(editor.document.nodes[&edited].text, committed);
    no_visual_preparation(render(&mut painter, &editor, 0.0, true).1);

    editor.begin_node_transaction(&[edited], "Cancelled inline text");
    editor.document.nodes.get_mut(&edited).unwrap().text = "Cancelled".into();
    editor.refresh_preview_nodes(&[edited]);
    no_visual_preparation(render(&mut painter, &editor, 0.0, true).1);
    editor.cancel_transaction();
    assert_eq!(editor.document.nodes[&edited].text, committed);
    assert_eq!(editor.history_len(), history + 1);
    no_visual_preparation(equals_suppressed(&mut painter, &editor, edited).1);

    // Leaving editing deliberately rebuilds traversal once and displays the
    // final text through the normal public FerroUI shaping/rendering path.
    painter.set_editing_text(None);
    let (finished, stats) = equals_fresh(&mut painter, &editor);
    assert_eq!((stats.full_prepares, stats.prepared_nodes), (1, 10_000));
    assert_eq!(stats.rebuilt_visuals, 1);
    assert_ne!(finished, suppressed);
    scope.dispose();
}

#[test]
fn suppressed_text_retains_ancestor_transforms_derived_geometry_and_sibling_updates() {
    let scope = backend();
    for kind in [NodeKind::Group, NodeKind::Boolean] {
        let mut document = Document::new("Suppressed nested text");
        let mut frame = Node::new(NodeKind::Frame, "Clipped frame", 8.0, 8.0, 94.0, 92.0);
        frame.clip_content = true;
        frame.corner_radius = 14.0;
        frame.opacity = 0.8;
        let frame = document.insert_node(1, frame).unwrap();
        let mut derived = Node::new(kind, "Derived outline", 8.0, 6.0, 80.0, 80.0)
            .with_fill(Color::rgb(40, 160, 230));
        derived.strokes.push(Stroke::new(Color::BLACK, 2.0));
        let derived = document.insert_node(frame, derived).unwrap();
        let edited = document.insert_node(derived, text()).unwrap();
        let visible = document
            .insert_node(
                derived,
                Node::new(
                    NodeKind::Rectangle,
                    "Visible operand",
                    12.0,
                    12.0,
                    45.0,
                    45.0,
                )
                .with_fill(Color::rgb(220, 60, 40)),
            )
            .unwrap();
        let mut editor = Editor::new(document);
        let mut painter = ScenePainter::default();
        painter.set_editing_text(Some(edited));
        let original = render(&mut painter, &editor, 0.0, true).0;

        editor.update_node(edited, "Text and its authored bounds", |node| {
            node.text = "Larger text".into();
            node.width = 900.0;
            node.height = 700.0;
        });
        let (pixels, stats) = equals_suppressed(&mut painter, &editor, edited);
        equal_pixels(&pixels, &original);
        assert_eq!(
            (
                stats.full_prepares,
                stats.incremental_prepares,
                stats.prepared_nodes
            ),
            (0, 1, 2)
        );
        assert_eq!(stats.rebuilt_visuals, 0);

        editor.update_node(frame, "Transform and clip parent", |node| {
            node.set_local_transform(
                Transform::translation(19.0, 9.0)
                    * Transform::rotation(11.0)
                    * Transform::scale(0.9, 0.8),
            );
            node.corner_radius = 25.0;
            node.opacity = 0.65;
        });
        let (moved, stats) = equals_suppressed(&mut painter, &editor, edited);
        assert_ne!(moved, original);
        assert_eq!(
            (
                stats.full_prepares,
                stats.incremental_prepares,
                stats.prepared_nodes
            ),
            (0, 1, 3)
        );

        editor.update_nodes(&[edited, visible], "Text with visible sibling", |node| {
            if node.id == edited {
                node.text.push('!');
            } else {
                node.x += 18.0;
                node.fills = vec![Fill::solid(Color::rgb(20, 210, 80))];
            }
        });
        let (changed, stats) = equals_suppressed(&mut painter, &editor, edited);
        assert_ne!(changed, moved);
        assert_eq!(
            (
                stats.full_prepares,
                stats.incremental_prepares,
                stats.prepared_nodes
            ),
            (0, 1, 3)
        );
    }
    scope.dispose();
}

#[test]
fn suppressed_text_keeps_fallback_for_other_hidden_nodes_and_external_ancestors() {
    let scope = backend();
    let mut document = Document::new("Conservative missing indices");
    let edited = document.insert_node(1, text()).unwrap();
    let mut hidden = Node::new(NodeKind::Rectangle, "Hidden", 10.0, 10.0, 40.0, 40.0);
    hidden.visible = false;
    let hidden = document.insert_node(1, hidden).unwrap();
    let parent = document
        .insert_node(
            1,
            Node::new(NodeKind::Frame, "Export ancestor", 12.0, 12.0, 90.0, 90.0),
        )
        .unwrap();
    let exported = document.insert_node(parent, text()).unwrap();
    let mut editor = Editor::new(document);
    let mut painter = ScenePainter::default();
    painter.set_editing_text(Some(edited));
    render(&mut painter, &editor, 0.0, true);
    editor.update_nodes(&[edited, hidden], "Text and hidden sibling", |node| {
        node.x += 1.0
    });
    assert_eq!(
        equals_suppressed(&mut painter, &editor, edited)
            .1
            .full_prepares,
        1
    );
    assert!(!painter.prepare_incremental(&editor.document, &[u64::MAX], &[]));

    // Isolated export roots do not prepare their document ancestors. Editing
    // such a suppressed root must not conceal that unrelated missing entry.
    painter.set_editing_text(Some(exported));
    render_roots(&mut painter, &editor, &[exported], 0.0, true);
    editor.update_node(exported, "Exported text", |node| node.text.push('!'));
    let (pixels, stats) = render_roots(&mut painter, &editor, &[exported], 0.0, true);
    let mut reference = ScenePainter::default();
    reference.set_editing_text(Some(exported));
    equal_pixels(
        &pixels,
        &render_roots(&mut reference, &editor, &[exported], 0.0, false).0,
    );
    assert_eq!(stats.full_prepares, 1);
    scope.dispose();
}

#[test]
fn suppressed_text_exception_rejects_unsupported_roles_hidden_state_and_invalid_bounds() {
    let scope = backend();
    let changes: [fn(&mut Node); 7] = [
        |node| node.kind = NodeKind::Rectangle,
        |node| node.children.push(u64::MAX),
        |node| node.is_mask = true,
        |node| node.visible = false,
        |node| node.opacity = 0.0,
        |node| node.width = f64::NAN,
        |node| node.opacity = f64::INFINITY,
    ];
    for change in changes {
        let mut document = Document::new("Suppression guard");
        let edited = document.insert_node(1, text()).unwrap();
        let mut editor = Editor::new(document);
        let mut painter = ScenePainter::default();
        painter.set_editing_text(Some(edited));
        render(&mut painter, &editor, 0.0, true);
        assert!(painter.prepare_incremental(&editor.document, &[edited], &[]));
        assert!(!painter.prepare_incremental(&editor.document, &[], &[edited]));
        change(editor.document.nodes.get_mut(&edited).unwrap());
        // Exercise the renderer guard itself; structural editor edits also
        // publish a full change record before reaching this method.
        assert!(!painter.prepare_incremental(&editor.document, &[edited], &[]));
    }
    scope.dispose();
}

#[test]
fn suppressed_root_text_preserves_backdrop_prefix_and_continuity_fallbacks() {
    let scope = backend();
    let mut document = Document::new("Suppressed text and backdrop");
    document
        .insert_node(
            1,
            Node::new(NodeKind::Rectangle, "White", 0.0, 0.0, 64.0, 128.0).with_fill(Color::WHITE),
        )
        .unwrap();
    document
        .insert_node(
            1,
            Node::new(NodeKind::Rectangle, "Black", 64.0, 0.0, 64.0, 128.0).with_fill(Color::BLACK),
        )
        .unwrap();
    let edited = document.insert_node(1, text()).unwrap();
    let mut glass = Node::new(NodeKind::Rectangle, "Glass", 32.0, 32.0, 64.0, 64.0);
    glass.fills.clear();
    glass.effects.push(Effect {
        kind: EffectKind::BackgroundBlur,
        color: Color::TRANSPARENT,
        radius: 6.0,
        offset: Point::default(),
        spread: 0.0,
        visible: true,
    });
    document.insert_node(1, glass).unwrap();
    let mut editor = Editor::new(document);
    let mut painter = ScenePainter::default();
    painter.set_editing_text(Some(edited));
    assert_eq!(
        render(&mut painter, &editor, 0.0, true)
            .1
            .backdrop_rasterizations,
        1
    );
    editor.update_node(edited, "Type", |node| node.text.push('B'));
    let (_, stats) = equals_suppressed(&mut painter, &editor, edited);
    no_visual_preparation(stats);
    assert_eq!(
        (stats.backdrop_rasterizations, stats.backdrop_cache_hits),
        (0, 1)
    );

    // An explicit refresh, a replaced editor, and an exhausted change ledger
    // still require complete preparation even when only text is suppressed.
    editor.refresh();
    assert_eq!(
        equals_suppressed(&mut painter, &editor, edited)
            .1
            .full_prepares,
        1
    );
    let mut replacement = Editor::new(editor.document.clone());
    assert_eq!(
        equals_suppressed(&mut painter, &replacement, edited)
            .1
            .full_prepares,
        1
    );
    for _ in 0..130 {
        replacement.update_node(edited, "Unobserved type", |node| node.text.push('C'));
    }
    assert_eq!(
        equals_suppressed(&mut painter, &replacement, edited)
            .1
            .full_prepares,
        1
    );
    scope.dispose();
}
