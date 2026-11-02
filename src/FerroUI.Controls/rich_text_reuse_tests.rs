//! F1 (#243 review S7): what keeps and what rebuilds the per-paragraph runs, shaping caches and
//! lines of a `RichTextBlock`.

use crate::documents::Paragraph;
use crate::RichTextBlock;
use ferroui_base::media::FontFamily;
use ferroui_base::{Ref, Size};
use std::rc::Rc;

fn view() -> Ref<RichTextBlock> {
    let view = RichTextBlock::new();
    for text in ["first paragraph", "second paragraph", "third paragraph"] {
        view.blocks().add(Paragraph::with_text(text));
    }
    view.measure(Size::new(300.0, f64::INFINITY));
    view
}

fn caches(view: &RichTextBlock) -> Vec<Rc<ferroui_base::media::text_formatting::TextRunCache>> {
    view.page_layout().source.paragraphs.iter().map(|paragraph| paragraph.cache.clone()).collect()
}

fn lines(view: &RichTextBlock) -> Vec<Rc<dyn ferroui_base::media::text_formatting::TextLine>> {
    view.page_layout().lines.iter().map(|line| line.line.clone()).collect()
}

fn text_sources(view: &RichTextBlock) -> Vec<Rc<Vec<Rc<dyn ferroui_base::media::text_formatting::TextRun>>>> {
    view.page_layout().source.paragraphs.iter().map(|paragraph| paragraph.source.text_runs().clone()).collect()
}

#[test]
fn a_selection_change_keeps_every_paragraph() {
    // #439: the selection is drawn over the lines, not as a formatting override of the runs. A
    // change of the selection, or of how it is drawn, keeps the text source, the shaping cache
    // and the lines of every paragraph, and the page layout itself; it only draws the view again.
    use ferroui_base::media::Brushes;
    let _scope = crate::test_support::test_scope();
    let view = view();
    view.set_selection_highlight_color(Some(Brushes::red()));
    view.set_selection_foreground_brush(Some(Brushes::white()));
    view.measure(Size::new(300.0, f64::INFINITY));
    let layout = view.page_layout();
    let before = caches(&view);
    let before_lines = lines(&view);
    let before_sources = text_sources(&view);
    let changes: [fn(&RichTextBlock); 9] = [
        |view: &RichTextBlock| view.select(0, 3),
        |view: &RichTextBlock| view.select(5, 40),
        |view: &RichTextBlock| view.select(16, 17),
        |view: &RichTextBlock| view.select_all(),
        |view: &RichTextBlock| view.select(20, 20),
        |view: &RichTextBlock| {
            view.select(2, 30);
            view.set_selection_foreground_brush(None);
        },
        |view: &RichTextBlock| view.set_selection_highlight_color(Some(Brushes::blue())),
        |view: &RichTextBlock| view.set_is_text_selection_enabled(false),
        |view: &RichTextBlock| view.clear_selection(),
    ];
    for change in changes {
        change(&view);
        assert!(view.is_measure_valid());
        assert!(Rc::ptr_eq(&layout, &view.cached_page_layout().unwrap()));
        view.measure(Size::new(300.0, f64::INFINITY));
        let after = caches(&view);
        let after_lines = lines(&view);
        let after_sources = text_sources(&view);
        assert_eq!(before.len(), after.len());
        assert_eq!(before_lines.len(), after_lines.len());
        for (before, after) in before.iter().zip(&after) {
            assert!(Rc::ptr_eq(before, after));
        }
        for (before, after) in before_lines.iter().zip(&after_lines) {
            assert!(Rc::ptr_eq(before, after));
        }
        for (before, after) in before_sources.iter().zip(&after_sources) {
            assert!(Rc::ptr_eq(before, after));
        }
    }
}

#[test]
fn a_font_change_of_the_view_rebuilds_every_paragraph() {
    let _scope = crate::test_support::test_scope();
    for change in [
        (|view: &RichTextBlock| view.set_font_size(24.0)) as fn(&RichTextBlock),
        |view: &RichTextBlock| view.set_font_family(FontFamily::parse("Courier New").unwrap()),
    ] {
        let view = view();
        let before = caches(&view);
        let before_lines = lines(&view);
        change(&view);
        view.measure(Size::new(300.0, f64::INFINITY));
        let after = caches(&view);
        let after_lines = lines(&view);
        assert_eq!(before.len(), after.len());
        for (before, after) in before.iter().zip(&after) {
            assert!(!Rc::ptr_eq(before, after));
        }
        for (before, after) in before_lines.iter().zip(&after_lines) {
            assert!(!Rc::ptr_eq(before, after));
        }
    }
}

#[test]
fn a_paragraph_break_and_its_undo_keep_the_paragraphs_they_do_not_touch() {
    // F1c (#439): a structural edit, and the undo that restores the snapshot from before it,
    // rebuild the paragraphs they change; the others keep their elements and so their runs,
    // shaping caches and lines.
    use crate::rich_text::{EditKind, EditorSelection, RichTextEditor};
    let _scope = crate::test_support::test_scope();
    let view = view();
    let editor = RichTextEditor::new(view.rich_document());
    let before = caches(&view);
    let before_lines = lines(&view);
    // "first paragraph\nsecond| paragraph\nthird paragraph"
    editor.set_selection(EditorSelection::caret(22)).unwrap();
    editor.insert_text("\n", EditKind::Typing).unwrap();
    view.measure(Size::new(300.0, f64::INFINITY));
    let split = caches(&view);
    let split_lines = lines(&view);
    assert_eq!(split.len(), 4);
    assert!(Rc::ptr_eq(&before[0], &split[0]));
    assert!(!Rc::ptr_eq(&before[1], &split[1]));
    assert!(Rc::ptr_eq(&before[2], &split[3]));
    assert!(Rc::ptr_eq(&before_lines[0], &split_lines[0]));
    assert!(Rc::ptr_eq(&before_lines[2], &split_lines[3]));
    editor.undo().unwrap();
    view.measure(Size::new(300.0, f64::INFINITY));
    let undone = caches(&view);
    let undone_lines = lines(&view);
    assert_eq!(undone.len(), 3);
    assert!(Rc::ptr_eq(&before[0], &undone[0]));
    assert!(Rc::ptr_eq(&before[2], &undone[2]));
    assert!(Rc::ptr_eq(&before_lines[0], &undone_lines[0]));
    assert!(Rc::ptr_eq(&before_lines[2], &undone_lines[2]));
}
