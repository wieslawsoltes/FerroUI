use crate::column_ruler_renderer::{pixel_align, snap_to_device_pixels};
use crate::tests::{laid_out_editor, render_editor, test_scope};
use crate::{
    BackgroundRendererRef, ColumnRulerRenderer, CurrentLineHighlightRenderer,
    FoldingElementGenerator, IBackgroundRenderer, KnownLayer, SnippetBackgroundRenderer,
    SnippetController, TextArea, TextDocument, TextEditor,
};
use ferroui_base::input::IScrollable;
use ferroui_base::media::immutable::ImmutableSolidColorBrush;
use ferroui_base::media::{Color, DrawingGroup, GeometryDrawing};
use ferroui_base::{Matrix, Point, Rect, Ref, Size, Vector};
use ferroui_edit_core::folding::FoldingManager;
use ferroui_edit_core::highlighting::HighlightingDefinition;
use ferroui_edit_core::snippets::{
    DeactivateReason, ISnippetElement, Snippet, SnippetElement, SnippetElementRef, SnippetError,
    SnippetInsertionContext, SnippetSession,
};
use std::cell::RefCell;
use std::rc::Rc;

fn drawings(renderer: &dyn IBackgroundRenderer, view: &TextArea) -> Vec<Ref<GeometryDrawing>> {
    let group = DrawingGroup::new();
    let mut context = group.open();
    renderer.draw(view, &mut context).unwrap();
    context.dispose();
    group
        .children()
        .iter()
        .map(|child| child.cast::<GeometryDrawing>().unwrap())
        .collect()
}

fn near(left: f64, right: f64) {
    assert!((left - right).abs() < 0.00001, "{left} != {right}");
}

fn bounds_equal(actual: Rect, expected: Rect) {
    near(actual.x, expected.x);
    near(actual.y, expected.y);
    near(actual.width, expected.width);
    near(actual.height, expected.height);
}

#[test]
fn column_rulers_use_native_space_width_gutter_scroll_and_ordered_columns() {
    let _scope = test_scope();
    let editor = laid_out_editor(&"x".repeat(200));
    let view = editor.text_area();
    render_editor(&editor);
    view.set_offset(Vector::new(19.25, 0.0));
    let renderer = Rc::new(ColumnRulerRenderer::new());
    assert_eq!(renderer.layer(), KnownLayer::Background);
    renderer.set_columns([0, 4, 4, -2]);
    let identity = BackgroundRendererRef::from(renderer.clone());
    view.background_renderers().add(identity.clone());
    let recorded = drawings(&*renderer, &view);
    assert_eq!(recorded.len(), 4);
    for (drawing, column) in recorded.iter().zip(renderer.columns()) {
        let x = view.content_viewport().x
            + pixel_align(view.wide_space_width() * f64::from(column), 1.0)
            - view.offset().x;
        bounds_equal(
            drawing.geometry().unwrap().bounds(),
            Rect::new(
                x,
                0.0,
                0.0,
                view.extent().height.max(view.content_viewport().height),
            ),
        );
        let pen = drawing.pen().unwrap();
        assert_eq!(pen.thickness(), 1.0);
        assert_eq!(
            pen.brush().unwrap().as_solid_color_brush().unwrap().color(),
            ColumnRulerRenderer::DEFAULT_FOREGROUND
        );
    }
    assert_eq!(
        render_editor(&editor)
            .iter()
            .filter(|line| line.starts_with("DrawLine LightGray@1"))
            .count(),
        4
    );
    let cached = view.cached_visual_line_count();
    renderer.set_ruler([80], None);
    assert_eq!(view.cached_visual_line_count(), cached);
    assert!(drawings(&*renderer, &view).is_empty());
    assert!(view.background_renderers().remove(&identity));
}

#[test]
fn ruler_device_snapping_matches_negative_midpoints_and_transformed_coordinates() {
    for (value, expected) in [
        (0.0, 0.5),
        (0.1, 0.5),
        (0.9, 0.5),
        (1.0, 1.5),
        (-0.2, -0.5),
        (-1.0, -1.5),
    ] {
        near(pixel_align(value, 1.0), expected);
    }
    near(pixel_align(2.3, 0.5), 2.25);
    let transform = Matrix::create_translation(0.25, 1.75) * Matrix::create_scale(2.0, 2.0);
    let point = Point::new(10.1, 3.2);
    let device = point.transform(transform);
    let snapped = snap_to_device_pixels(point, Some(transform)).transform(transform);
    near(snapped.x, device.x.trunc() + 0.5);
    near(snapped.y, device.y.trunc() + 0.5);
    assert_eq!(snap_to_device_pixels(point, None), point);
    assert_eq!(
        snap_to_device_pixels(point, Some(Matrix::create_scale(0.0, 1.0))),
        point
    );
    assert_eq!(
        snap_to_device_pixels(Point::new(-0.8, -1.7), Some(Matrix::IDENTITY)),
        Point::new(0.5, -0.5)
    );
}

#[test]
fn current_line_renderer_uses_pinned_colors_and_suppresses_disabled_or_unseen_lines() {
    let _scope = test_scope();
    let editor = laid_out_editor("first\nsecond\nlast");
    let view = editor.text_area();
    let renderer = Rc::new(CurrentLineHighlightRenderer::new());
    view.background_renderers().add(renderer.clone().into());
    renderer.set_line(2);
    assert!(drawings(&*renderer, &view).is_empty());
    renderer.set_enabled(true);
    let recorded = drawings(&*renderer, &view);
    assert_eq!(recorded.len(), 1);
    bounds_equal(
        recorded[0].geometry().unwrap().bounds(),
        view.visual_line_bounds(2).unwrap(),
    );
    assert_eq!(
        recorded[0]
            .brush()
            .unwrap()
            .as_solid_color_brush()
            .unwrap()
            .color(),
        CurrentLineHighlightRenderer::DEFAULT_BACKGROUND
    );
    assert_eq!(
        recorded[0]
            .pen()
            .unwrap()
            .brush()
            .unwrap()
            .as_solid_color_brush()
            .unwrap()
            .color(),
        CurrentLineHighlightRenderer::DEFAULT_BORDER
    );
    let cached = view.cached_visual_line_count();
    renderer.set_background_brush(Some(Rc::new(ImmutableSolidColorBrush::from_uint32(
        0xff010203,
    ))));
    renderer.set_border_pen(None);
    assert_eq!(view.cached_visual_line_count(), cached);
    assert!(drawings(&*renderer, &view)[0].pen().is_none());
    renderer.set_default_colors();
    assert!(drawings(&*renderer, &view)[0].pen().is_some());
    renderer.set_line(0);
    assert!(drawings(&*renderer, &view).is_empty());
    renderer.set_line(200);
    assert!(drawings(&*renderer, &view).is_empty());
}

#[test]
fn current_line_geometry_covers_all_wrapped_rows_and_native_font_heights() {
    let _scope = test_scope();
    let editor = laid_out_editor("small\nTALL words words words words\nlast");
    editor.set_word_wrap(true);
    editor.set_syntax_highlighting(Some(Rc::new(HighlightingDefinition::load(
        r#"<SyntaxDefinition><RuleSet><Rule fontSize="40">TALL</Rule></RuleSet></SyntaxDefinition>"#,
    ).unwrap())));
    let view = editor.text_area();
    view.measure(Size::new(120.0, 300.0));
    view.arrange(Rect::new(0.0, 0.0, 120.0, 300.0));
    let renderer = CurrentLineHighlightRenderer::new();
    renderer.set_enabled(true);
    renderer.set_line(2);
    let recorded = drawings(&renderer, &view);
    let bounds = recorded[0].geometry().unwrap().bounds();
    bounds_equal(bounds, view.visual_line_bounds(2).unwrap());
    assert!(bounds.height > view.line_height() * 2.0);
    assert_eq!(bounds.width, view.content_viewport().width);
}

#[test]
fn current_line_geometry_uses_the_visible_owner_of_folded_lines() {
    let _scope = test_scope();
    let editor = laid_out_editor("a{\n hidden\n}z\nlast");
    let manager = editor.install_folding();
    manager.create_folding(1, 12).unwrap().set_is_folded(true);
    let view = editor.text_area();
    let renderer = CurrentLineHighlightRenderer::new();
    renderer.set_enabled(true);
    renderer.set_line(2);
    let recorded = drawings(&renderer, &view);
    bounds_equal(
        recorded[0].geometry().unwrap().bounds(),
        view.visual_line_bounds(1).unwrap(),
    );
    assert_eq!(view.visual_line_bounds(1), view.visual_line_bounds(3));
}

#[test]
fn snippet_native_fill_and_linked_outlines_follow_caret_and_anchor_changes() {
    let _scope = test_scope();
    let editor = laid_out_editor("");
    let view = editor.text_area();
    let baseline = view.background_renderers().count();
    let controller = SnippetController::insert(
        &editor,
        &Snippet::parse("${2:bb} / ${1:aa} / $1 / $2$0").unwrap(),
    )
    .unwrap();
    assert_eq!(view.background_renderers().count(), baseline + 2);
    let background = controller.field_background_renderer().unwrap();
    let foreground = controller.active_field_renderer().unwrap();
    assert_eq!(background.layer(), KnownLayer::Background);
    assert_eq!(foreground.layer(), KnownLayer::Text);
    let fills = drawings(&*background, &view);
    assert_eq!(fills.len(), 2);
    for fill in &fills {
        assert!(fill.pen().is_none());
        let brush = fill.brush().unwrap();
        assert_eq!(
            brush.as_solid_color_brush().unwrap().color(),
            SnippetBackgroundRenderer::DEFAULT_BACKGROUND
        );
        assert_eq!(brush.opacity(), 0.4);
    }
    let outlines = drawings(&*foreground, &view);
    assert_eq!(outlines.len(), 1);
    assert!(outlines[0].brush().is_none());
    let pen = outlines[0].pen().unwrap();
    assert_eq!(pen.dash_style().unwrap().dashes(), Some(vec![0.0, 2.0]));
    let old_bounds = outlines[0].geometry().unwrap().bounds();
    controller.session().replace_active("long😀").unwrap();
    editor
        .set_caret_offset(controller.session().active_segment().unwrap().offset)
        .unwrap();
    let new_bounds = drawings(&*foreground, &view)[0]
        .geometry()
        .unwrap()
        .bounds();
    assert!(new_bounds.width > old_bounds.width);
    let fields = controller.session().fields();
    let first = fields.iter().find(|field| field.id == 1).unwrap();
    editor
        .set_caret_offset(first.bound_segments[0].offset + 1)
        .unwrap();
    assert!(
        drawings(&*foreground, &view).is_empty(),
        "bound occurrences alone do not activate a field outline"
    );
    editor.set_caret_offset(first.segment.end_offset()).unwrap();
    assert_eq!(
        drawings(&*foreground, &view).len(),
        1,
        "field endpoints include the caret"
    );
    let second = fields.iter().find(|field| field.id == 2).unwrap();
    editor.set_caret_offset(second.segment.offset + 1).unwrap();
    assert!(
        drawings(&*foreground, &view)[0]
            .geometry()
            .unwrap()
            .bounds()
            .x
            < new_bounds.x
    );
    let log = render_editor(&editor);
    let fill_index = log
        .iter()
        .position(|line| line.starts_with("DrawGeometry LimeGreen none"))
        .unwrap();
    let text_index = log
        .iter()
        .position(|line| line.starts_with("DrawGlyphRun"))
        .unwrap();
    assert!(fill_index < text_index);
    controller.finish();
    assert_eq!(view.background_renderers().count(), baseline);
    assert!(controller.field_background_renderer().is_none());
    assert!(drawings(&*background, &view).is_empty());
    assert!(drawings(&*foreground, &view).is_empty());
}

#[test]
fn snippet_highlights_cover_wrapped_bidi_fields_and_empty_placeholder_markers() {
    let _scope = test_scope();
    let editor = laid_out_editor("");
    editor.set_word_wrap(true);
    let view = editor.text_area();
    view.measure(Size::new(105.0, 300.0));
    view.arrange(Rect::new(0.0, 0.0, 105.0, 300.0));
    let controller = SnippetController::insert(
        &editor,
        &Snippet::parse("${1:aאב words words words}/$1$0").unwrap(),
    )
    .unwrap();
    let range = controller.session().active_segment().unwrap();
    assert!(view.segment_rectangles(&range, false).unwrap().len() > 1);
    let fill = drawings(&*controller.field_background_renderer().unwrap(), &view);
    assert_eq!(fill.len(), 1);
    assert!(fill[0].geometry().unwrap().bounds().height > view.line_height());
    controller.finish();
    editor.set_text("").unwrap();
    let empty = SnippetController::insert(&editor, &Snippet::parse("${1:}$0").unwrap()).unwrap();
    let fill = drawings(&*empty.field_background_renderer().unwrap(), &view);
    assert_eq!(fill.len(), 1);
    assert!(fill[0].geometry().unwrap().bounds().width > 0.0);
    assert_eq!(
        drawings(&*empty.active_field_renderer().unwrap(), &view).len(),
        1
    );
}

#[test]
fn snippet_renderer_registration_is_removed_for_deactivation_drop_and_document_replacement() {
    let _scope = test_scope();
    let editor = laid_out_editor("");
    let view = editor.text_area();
    let baseline = view.background_renderers().count();
    for reason in [
        DeactivateReason::EscapePressed,
        DeactivateReason::ReturnPressed,
        DeactivateReason::Deleted,
        DeactivateReason::Error,
    ] {
        editor.set_text("").unwrap();
        let controller =
            SnippetController::insert(&editor, &Snippet::parse("${1:a}/$1$0").unwrap()).unwrap();
        controller.session().deactivate(reason);
        assert_eq!(view.background_renderers().count(), baseline);
        assert!(controller.active_field_renderer().is_none());
    }
    editor.set_text("").unwrap();
    let controller =
        SnippetController::insert(&editor, &Snippet::parse("${1:a}$0").unwrap()).unwrap();
    let weak = Rc::downgrade(&controller);
    let session = controller.session();
    let renderer = controller.field_background_renderer().unwrap();
    drop(controller);
    assert!(weak.upgrade().is_none());
    assert!(session.is_closed());
    assert_eq!(view.background_renderers().count(), baseline);
    assert!(drawings(&*renderer, &view).is_empty());
    editor.set_text("").unwrap();
    let controller =
        SnippetController::insert(&editor, &Snippet::parse("${1:a}$0").unwrap()).unwrap();
    editor.set_document(TextDocument::from_text("replacement"));
    assert!(!controller.is_active());
    assert_eq!(view.background_renderers().count(), baseline);
}

struct CaptureInsertion(Rc<RefCell<Option<SnippetSession>>>);
impl ISnippetElement for CaptureInsertion {
    fn insert(&self, context: &mut SnippetInsertionContext) -> Result<(), SnippetError> {
        context.insert_replaceable(1, "name")?;
        let capture = self.0.clone();
        context.on_insertion_completed(move |session| {
            *capture.borrow_mut() = Some(session.clone());
        });
        Ok(())
    }
}

#[test]
fn snippet_deactivation_from_renderer_collection_callback_leaves_no_partial_registration() {
    let _scope = test_scope();
    let editor = laid_out_editor("");
    let renderers = editor.text_area().background_renderers();
    let baseline = renderers.count();
    let capture = Rc::new(RefCell::new(None::<SnippetSession>));
    let saved = capture.clone();
    let token = renderers.add_collection_changed(Rc::new(move |event| {
        if !event.new_items.is_empty() {
            if let Some(session) = saved.borrow().as_ref() {
                session.deactivate(DeactivateReason::Unknown);
            }
        }
    }));
    let snippet = Snippet {
        elements: vec![SnippetElement::Custom(SnippetElementRef::new(
            CaptureInsertion(capture),
        ))],
    };
    let controller = SnippetController::insert(&editor, &snippet).unwrap();
    renderers.remove_collection_changed(token);
    assert!(!controller.is_active());
    assert!(controller.field_background_renderer().is_none());
    assert_eq!(renderers.count(), baseline);
}

#[test]
fn snippet_deactivation_during_native_line_construction_does_not_paint_retained_fields() {
    let _scope = test_scope();
    let editor = laid_out_editor("");
    let controller =
        SnippetController::insert(&editor, &Snippet::parse("${1:name}$0").unwrap()).unwrap();
    let view = editor.text_area();
    let renderer = controller.field_background_renderer().unwrap();
    let session = controller.session();
    let subscription = view.visual_line_construction_starting(move |_| {
        session.deactivate(DeactivateReason::Unknown);
    });
    view.invalidate_layout_cache();
    assert!(drawings(&*renderer, &view).is_empty());
    assert!(!controller.is_active());
    subscription.dispose();
}

#[test]
fn retained_renderers_do_not_keep_sessions_or_editor_views_alive() {
    let _scope = test_scope();
    let editor = laid_out_editor("");
    let view = editor.text_area();
    let session = Snippet::parse("${1:a}")
        .unwrap()
        .insert(editor.document(), 0, "")
        .unwrap();
    let weak_session = session.downgrade();
    let renderer = Rc::new(SnippetBackgroundRenderer::background(&session));
    view.background_renderers().add(renderer.clone().into());
    drop(session);
    assert!(weak_session.upgrade().is_none());
    assert!(drawings(&*renderer, &view).is_empty());
    let ruler = Rc::new(ColumnRulerRenderer::new());
    view.background_renderers().add(ruler.clone().into());
    let manager = Rc::new(FoldingManager::new(editor.document()));
    let folding = Rc::new(FoldingElementGenerator::new(manager.clone()));
    view.element_generators().add(folding.clone().into());
    let weak_view = view.downgrade();
    drop(view);
    drop(editor);
    assert!(weak_view.upgrade().is_none());
    manager.create_folding(0, 1).unwrap().set_is_folded(true);
    ruler.set_columns([80]);
    renderer.set_background_brush(Some(Rc::new(ImmutableSolidColorBrush::new(
        Color::from_rgb(1, 2, 3),
    ))));
}

#[test]
fn snippets_with_named_anchors_only_do_not_install_builtin_field_decorations() {
    let _scope = test_scope();
    let editor = TextEditor::new();
    let baseline = editor.text_area().background_renderers().count();
    let snippet = Snippet {
        elements: vec![
            SnippetElement::Text("text".into()),
            SnippetElement::Anchor {
                name: "bookmark".into(),
            },
        ],
    };
    let controller = SnippetController::insert(&editor, &snippet).unwrap();
    assert!(controller.is_active());
    assert!(controller.field_background_renderer().is_none());
    assert_eq!(editor.text_area().background_renderers().count(), baseline);
}
