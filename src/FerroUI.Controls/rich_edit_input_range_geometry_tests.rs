//! Real managed-client queries over visible composition and shaped layout.
use super::*;
use crate::documents::{InlineUIContainer, LineBreak, Paragraph, Run};
use crate::RichTextBlockOverflow;
use ferroui_base::input::text_input::{TextInputMethodRangeGeometry, TextSelection};
use ferroui_base::input::IScrollable;
use ferroui_base::layout::ILayoutManager;
use ferroui_base::media::text_formatting::{
    ITextSource, TextFormatter, TextFormatterImpl, TextLine, TextLineBreak, TextParagraphProperties,
};
use ferroui_base::media::{MatrixTransform, TextLineBounds, TextTrimming, TextWrapping};
use ferroui_base::{FerroLocator, Matrix, Vector};

fn layout(f: &Fixture) {
    f.window.layout_manager().execute_layout_pass();
}

fn query(f: &Fixture, start: i32, end: i32) -> TextInputMethodRangeGeometry {
    f.owner
        .text_input_method_client()
        .text_range_geometry(TextSelection::new(start, end))
        .unwrap()
}

fn rectangles(geometry: &TextInputMethodRangeGeometry) -> Vec<Rect> {
    geometry
        .fragments()
        .iter()
        .map(|fragment| fragment.rectangle.transform_to_aabb(fragment.transform))
        .collect()
}

#[test]
fn rich_input_range_geometry_uses_requested_range_without_moving_the_selection() {
    let f = Fixture::new(true);
    f.owner.set_text("first line\nsecond line").unwrap();
    f.owner.select(0, 0);
    f.owner.editor().clear_history();
    layout(&f);
    let before = f.owner.document().snapshot();
    let selection = f.owner.editor().selection();
    let requested = query(&f, 11, 17);
    assert_eq!(requested.range(), TextSelection::new(11, 17));
    assert_eq!(rectangles(&requested), f.owner.get_rects_for_range(11..17));
    assert!(rectangles(&requested)[0].y > f.owner.cursor_rectangle().y);
    let caret = query(&f, 14, 14);
    assert_eq!(
        rectangles(&caret),
        [f.owner.get_rect_from_character_index(14)]
    );
    assert!(requested.is_current());
    assert_eq!(f.owner.document().snapshot(), before);
    assert_eq!(f.owner.editor().selection(), selection);
    assert!(!f.owner.can_undo());
}

#[test]
fn rich_input_range_geometry_preserves_surrogate_and_font_cluster_requests() {
    let f = Fixture::new(true);
    f._text.shaper().add_ligature("ffi");
    f.owner.set_text("😀 ffi a\u{301} אבג").unwrap();
    f.owner.select(0, 0);
    layout(&f);
    let emoji = rectangles(&query(&f, 0, 2));
    assert!(!emoji.is_empty());
    for (start, end) in [(0, 1), (1, 2)] {
        let half = query(&f, start, end);
        assert_eq!(half.range(), TextSelection::new(start, end));
        assert_eq!(rectangles(&half), emoji);
    }
    let ligature = rectangles(&query(&f, 3, 6));
    assert_eq!(rectangles(&query(&f, 4, 5)), ligature);
    assert_eq!(rectangles(&query(&f, 8, 9)), rectangles(&query(&f, 7, 9)));
    // A read-only insertion query uses the formatter's edge for the requested
    // code unit. It does not create an invalid editor selection at that offset.
    assert_eq!(
        rectangles(&query(&f, 1, 1)),
        [f.owner.get_rect_from_character_index(1)]
    );
    assert_eq!(
        f.owner.editor().selection(),
        crate::rich_text::EditorSelection::new(0, 0)
    );
    assert_eq!(
        rectangles(&query(&f, 10, 13)),
        f.owner.get_rects_for_range(10..13)
    );
}

#[test]
fn rich_input_range_geometry_queries_visible_preedit_and_retires_after_cancel() {
    let f = Fixture::new(true);
    let before = f.owner.document().snapshot();
    let session = f.connection().begin_composition().unwrap();
    session.update("😀preview", Some(2)).unwrap();
    layout(&f);
    let client = f.owner.text_input_method_client();
    assert_eq!(client.surrounding_text(), "abcd");
    assert_eq!(f.owner.text(), "a😀previewd");
    let geometry = query(&f, 3, 10);
    assert_eq!(rectangles(&geometry), f.owner.get_rects_for_range(3..10));
    assert!(geometry.is_current());
    assert!(session.is_current());
    assert!(!f.owner.can_undo());
    assert!(session.cancel().unwrap());
    assert!(!geometry.is_current());
    assert_eq!(f.owner.document().snapshot(), before);
    assert!(!f.owner.can_undo());
}

#[test]
fn rich_input_range_geometry_keeps_scrolled_off_viewport_text_in_owner_coordinates() {
    let f = Fixture::new(true);
    f.owner.set_text(&vec!["abcdefgh"; 30].join("\n")).unwrap();
    f.owner.select(0, 0);
    layout(&f);
    let first = query(&f, 1, 4);
    let presenter = f.owner.text_presenter();
    presenter.set_offset(Vector::new(0.0, 120.0));
    let offset = presenter.offset();
    assert!(offset.y > 0.0);
    assert!(!first.is_current());
    let scrolled = query(&f, 1, 4);
    assert_eq!(rectangles(&scrolled), f.owner.get_rects_for_range(1..4));
    assert!(rectangles(&scrolled)[0].y < 0.0);
    assert_eq!(presenter.offset(), offset);
    assert_eq!(f.owner.caret_index(), 0);
}

#[test]
fn rich_input_range_geometry_uses_break_allocations_without_a_selection_marker() {
    let f = Fixture::new(true);
    f.owner.set_text("a\nb").unwrap();
    f.owner.select(0, 0);
    layout(&f);
    let separator = rectangles(&query(&f, 1, 2));
    let caret = f.owner.get_rect_from_character_index(1);
    assert_eq!(separator, [Rect::new(caret.x, caret.y, 0.0, caret.height)]);
    assert_eq!(f.owner.get_rects_for_range(1..2)[0].width, 4.0);
    assert!(query(&f, 0, 3).fragments().len() >= 3);
    f.owner.set_text("").unwrap();
    layout(&f);
    let empty = query(&f, 0, 0);
    assert_eq!(
        rectangles(&empty),
        [f.owner.get_rect_from_character_index(0)]
    );
    assert!(rectangles(&empty)[0].height > 0.0);
}

#[test]
fn rich_input_range_geometry_covers_embedded_controls_soft_breaks_and_wrapped_bidi() {
    let f = Fixture::new(true);
    let paragraph = Paragraph::with_text("a");
    let child = Border::new();
    child.set_width(24.0);
    child.set_height(18.0);
    paragraph.add_child(InlineUIContainer::with_child(child));
    paragraph.add_child(LineBreak::new());
    paragraph.add_child(Run::with_text(Some("אבג abc אבג abc אבג abc")));
    let document = crate::documents::RichTextDocument::new();
    document.blocks().add(paragraph);
    document.blocks().add(Paragraph::with_text("last"));
    f.owner.set_document(document);
    f.owner.set_text_wrapping(TextWrapping::Wrap);
    f.owner.set_width(80.0);
    layout(&f);
    let length = f.owner.document().text_length() as i32;
    let geometry = query(&f, 0, length);
    assert!(geometry.fragments().len() >= 4);
    assert_eq!(
        rectangles(&query(&f, 1, 2)),
        f.owner.get_rects_for_range(1..2)
    );
    assert!(geometry.is_current());
}

#[test]
fn rich_input_range_geometry_reports_invalid_hidden_unmeasured_and_trimmed_text() {
    let f = Fixture::new(true);
    layout(&f);
    let client = f.owner.text_input_method_client();
    for range in [
        TextSelection::new(-1, 0),
        TextSelection::new(3, 2),
        TextSelection::new(0, 5),
    ] {
        assert_eq!(
            client.text_range_geometry(range).unwrap_err(),
            TextInputMethodError::InvalidRange
        );
    }
    let fresh = RichEditBox::new();
    assert_eq!(
        fresh
            .text_input_method_client()
            .text_range_geometry(TextSelection::new(0, 0))
            .unwrap_err(),
        TextInputMethodError::Unavailable
    );
    f.owner.set_is_visible(false);
    assert_eq!(
        client
            .text_range_geometry(TextSelection::new(0, 1))
            .unwrap_err(),
        TextInputMethodError::Unavailable
    );
    f.owner.set_is_visible(true);
    f.owner
        .set_text("a long line whose omitted suffix has no visible geometry")
        .unwrap();
    f.owner.set_width(65.0);
    f.owner.set_text_wrapping(TextWrapping::Wrap);
    f.owner.select(0, 0);
    f.owner.text_presenter().set_max_lines(1);
    f.owner
        .text_presenter()
        .set_text_trimming(<dyn TextTrimming>::character_ellipsis());
    layout(&f);
    assert!(f.owner.text_presenter().is_text_trimmed());
    let length = f.owner.document().text_length() as i32;
    assert!(query(&f, 0, 1).is_current());
    for range in [
        TextSelection::new(0, length),
        TextSelection::new(length - 1, length),
        TextSelection::new(length, length),
    ] {
        assert_eq!(
            client.text_range_geometry(range).unwrap_err(),
            TextInputMethodError::Unavailable
        );
    }
}

#[test]
fn rich_input_range_geometry_tracks_inherited_format_transforms_and_layout_identity() {
    let f = Fixture::new(true);
    layout(&f);
    let mutation = f.owner.document().mutation_version();
    let old = query(&f, 0, 1);
    f.owner.set_font_size(f.owner.font_size() + 8.0);
    assert_eq!(f.owner.document().mutation_version(), mutation);
    assert!(!old.is_current());
    layout(&f);
    let new = query(&f, 0, 1);
    let presenter = f.owner.text_presenter();
    presenter.set_render_transform(Some(
        MatrixTransform::with_matrix(Matrix::new(0.0, 1.0, -1.0, 0.0, 8.0, 12.0)).into(),
    ));
    assert!(!new.is_current());
    let transformed = query(&f, 0, 1);
    assert_eq!(rectangles(&transformed), f.owner.get_rects_for_range(0..1));
    assert_ne!(transformed.fragments()[0].transform, Matrix::IDENTITY);
    assert!(transformed.is_current());
}

#[test]
fn rich_input_range_geometry_follows_inherited_line_bounds_and_retires_old_layouts() {
    let f = Fixture::new(true);
    f.owner.set_text("Ag\nAg").unwrap();
    f.owner.set_font_size(20.0);
    f.owner.select(0, 0);
    layout(&f);
    let mutation = f.owner.document().mutation_version();
    let mut previous: Option<TextInputMethodRangeGeometry> = None;
    let mut heights = Vec::new();
    for mode in [
        TextLineBounds::Full,
        TextLineBounds::TrimToCapHeight,
        TextLineBounds::TrimToBaseline,
        TextLineBounds::Tight,
    ] {
        // The semantic owner supplies the inherited property; the rich view
        // uses the same property identity as other text controls.
        f.owner
            .set_value(crate::TextBlock::text_line_bounds_property(), mode);
        assert_eq!(f.owner.text_presenter().text_line_bounds(), mode);
        assert_eq!(f.owner.document().mutation_version(), mutation);
        if let Some(old) = previous.take() {
            assert!(!old.is_current(), "bounds change must retire the old page");
        }
        layout(&f);
        let range = query(&f, 1, 2);
        let bounds = rectangles(&range);
        assert_eq!(bounds, f.owner.get_rects_for_range(1..2));
        let line = &f.owner.line_metrics()[0];
        assert_eq!(bounds[0].y, line.bounds.y);
        assert_eq!(bounds[0].height, line.bounds.height);
        let caret = rectangles(&query(&f, 2, 2))[0];
        let separator = rectangles(&query(&f, 2, 3))[0];
        assert_eq!(caret.y, line.bounds.y);
        assert_eq!(caret.height, line.bounds.height);
        assert_eq!(separator, Rect::new(caret.x, caret.y, 0.0, caret.height));
        assert!(range.is_current());
        heights.push(line.bounds.height);
        previous = Some(range);
    }
    // Top and bottom trimming act independently. The formatter owns these
    // metrics; the platform query must not restore a Full-height caret minimum.
    let [full, top, bottom, tight]: [f64; 4] = heights.try_into().unwrap();
    assert!(full > top && full > bottom && top > tight && bottom > tight);
    assert!(((full - top) - (bottom - tight)).abs() < 1e-8);
    assert!(((full - bottom) - (top - tight)).abs() < 1e-8);
    f.owner
        .clear_value(crate::TextBlock::text_line_bounds_property());
    assert!(!previous.unwrap().is_current());
    layout(&f);
    assert_eq!(rectangles(&query(&f, 1, 2))[0].height, full);
    assert_eq!(f.owner.document().mutation_version(), mutation);
}

#[test]
fn rich_input_range_geometry_requires_complete_attached_overflow_coverage() {
    let f = Fixture::new(true);
    f.owner.set_text("first\nsecond\nthird").unwrap();
    let presenter = f.owner.text_presenter();
    presenter.set_max_lines(1);
    let overflow = RichTextBlockOverflow::new();
    overflow.set_height(100.0);
    presenter.set_overflow_content_target(Some(overflow.clone()));
    layout(&f);
    assert!(presenter.has_overflow_content());
    assert!(query(&f, 0, 2).is_current());
    let length = f.owner.document().text_length() as i32;
    assert_eq!(
        f.owner
            .text_input_method_client()
            .text_range_geometry(TextSelection::new(0, length))
            .unwrap_err(),
        TextInputMethodError::Unavailable
    );

    // The fixture already owns the editor through a StackPanel. Add the
    // overflow to that same attached parent; clearing Window.Content does not
    // remove the editor from the old panel's child collection.
    let panel = f
        .owner
        .visual_parent()
        .unwrap()
        .cast::<StackPanel>()
        .unwrap();
    panel.children().add(overflow.clone());
    f.window.set_height(240.0);
    layout(&f);
    let all = query(&f, 0, length);
    assert!(all.fragments().len() >= 5);
    assert!(all.is_current());
    overflow.set_is_visible(false);
    assert!(!all.is_current());
    assert_eq!(
        f.owner
            .text_input_method_client()
            .text_range_geometry(TextSelection::new(0, length))
            .unwrap_err(),
        TextInputMethodError::Unavailable
    );
}

struct CallbackFormatter {
    inner: TextFormatterImpl,
    callback: RefCell<Option<Box<dyn FnOnce()>>>,
}

impl TextFormatter for CallbackFormatter {
    fn format_line(
        &self,
        source: &dyn ITextSource,
        index: i32,
        width: f64,
        properties: &Rc<dyn TextParagraphProperties>,
        previous: Option<&Rc<TextLineBreak>>,
    ) -> Option<Rc<dyn TextLine>> {
        let callback = self.callback.borrow_mut().take();
        if let Some(callback) = callback {
            callback();
        }
        self.inner
            .format_line(source, index, width, properties, previous)
    }
}

#[test]
fn rich_input_range_geometry_rejects_reentrant_document_selection_and_view_changes() {
    for change in 0..4 {
        let f = Fixture::new(true);
        layout(&f);
        let owner = f.owner.clone();
        let invoked = Rc::new(Cell::new(false));
        let observed = invoked.clone();
        let nested = Rc::new(RefCell::new(None));
        let nested_callback = nested.clone();
        let batch = (change == 0).then(|| f.owner.document().begin_edit());
        let formatter: Rc<dyn TextFormatter> = Rc::new(CallbackFormatter {
            inner: TextFormatterImpl::new(),
            callback: RefCell::new(Some(Box::new(move || {
                observed.set(true);
                match change {
                    0 => owner.document().replace_text(0..1, "z").unwrap(),
                    1 => owner.select(0, 0),
                    2 => owner.text_presenter().set_font_size(30.0),
                    _ => {
                        owner.text_presenter().set_font_size(31.0);
                        nested_callback.replace(Some(owner.text_presenter().page_layout()));
                    }
                }
            }))),
        });
        FerroLocator::current_mutable()
            .bind::<dyn TextFormatter>()
            .to_constant(formatter);
        // Clear the cached page while retaining its measured constraint. The
        // query formats it and encounters the real service callback above.
        f.owner.text_presenter().invalidate_document_layout();
        assert_eq!(
            f.owner
                .text_input_method_client()
                .text_range_geometry(TextSelection::new(1, 2))
                .unwrap_err(),
            TextInputMethodError::StateChanged,
            "change {change}"
        );
        assert!(invoked.get());
        if let Some(expected) = nested.borrow().as_ref() {
            assert!(
                Rc::ptr_eq(
                    expected,
                    &f.owner.text_presenter().cached_page_layout().unwrap()
                ),
                "the older formatter must not replace the nested accepted layout"
            );
        }
        drop(batch);
        let recovered = rectangles(&query(&f, 1, 2));
        f.owner.text_presenter().invalidate_document_layout();
        assert_eq!(
            recovered,
            rectangles(&query(&f, 1, 2)),
            "a rejected outer formatter must not leave a stale cached page, change {change}"
        );
    }
}

#[test]
fn rich_input_range_geometry_does_not_retain_removed_controls_or_input_sessions() {
    let (weak, geometry) = {
        let f = Fixture::new(true);
        layout(&f);
        (f.owner.downgrade(), query(&f, 0, 1))
    };
    assert!(weak.upgrade().is_none());
    assert!(!geometry.is_current());
}

#[repr(C)]
struct SourceCallbackRun {
    base: Run,
    callback: RefCell<Option<Box<dyn FnOnce()>>>,
}

ferroui_base::ferro_class!(SourceCallbackRun: Run);
ferroui_base::ferro_impl_classes!(SourceCallbackRun:
    ferroui_base::FerroObjectImpl,
    ferroui_base::StyledElementImpl,
    crate::documents::TextElementImpl
);

impl crate::documents::InlineImpl for SourceCallbackRun {
    fn build_text_run(
        this: &Self,
        runs: &mut Vec<Rc<dyn ferroui_base::media::text_formatting::TextRun>>,
    ) {
        use crate::documents::InlineImplExt;
        // Capture actual runs before the callback changes their inherited font.
        Self::parent_build_text_run(this, runs);
        let callback = this.callback.borrow_mut().take();
        if let Some(callback) = callback {
            callback();
        }
    }
}

#[test]
fn rich_input_range_geometry_source_callbacks_preserve_newer_source_and_layout_caches() {
    for nested in [false, true] {
        let f = Fixture::new(true);
        let run: Ref<SourceCallbackRun> = ferroui_base::instantiate(SourceCallbackRun {
            base: Run::construct(),
            callback: RefCell::new(None),
        });
        run.set_text(Some("source callback"));
        let paragraph = Paragraph::new();
        paragraph.add_child(run.clone());
        let document = crate::documents::RichTextDocument::new();
        document.blocks().add(paragraph);
        f.owner.set_document(document);
        layout(&f);
        let owner = f.owner.clone();
        let retained = Rc::new(RefCell::new(None));
        let captured = retained.clone();
        let invoked = Rc::new(Cell::new(false));
        let observed = invoked.clone();
        run.callback.replace(Some(Box::new(move || {
            observed.set(true);
            owner.text_presenter().set_font_size(32.0);
            if nested {
                captured.replace(Some(owner.text_presenter().page_layout()));
            }
        })));
        f.owner.text_presenter().invalidate_document_layout();
        assert_eq!(
            f.owner
                .text_input_method_client()
                .text_range_geometry(TextSelection::new(0, 1))
                .unwrap_err(),
            TextInputMethodError::StateChanged
        );
        assert!(invoked.get());
        if let Some(expected) = retained.borrow().as_ref() {
            assert!(Rc::ptr_eq(
                expected,
                &f.owner.text_presenter().cached_page_layout().unwrap()
            ));
        }
        let recovered = rectangles(&query(&f, 0, 1));
        f.owner.text_presenter().invalidate_document_layout();
        assert_eq!(recovered, rectangles(&query(&f, 0, 1)), "nested {nested}");
    }
}
