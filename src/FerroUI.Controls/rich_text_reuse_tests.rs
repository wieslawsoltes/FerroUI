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

#[test]
fn a_selection_change_keeps_the_paragraphs_it_does_not_touch() {
    let _scope = crate::test_support::test_scope();
    let view = view();
    let before = caches(&view);
    let before_lines = lines(&view);
    view.select(0, 3);
    view.measure(Size::new(300.0, f64::INFINITY));
    let after = caches(&view);
    let after_lines = lines(&view);
    assert!(!Rc::ptr_eq(&before[0], &after[0]));
    assert!(Rc::ptr_eq(&before[1], &after[1]));
    assert!(Rc::ptr_eq(&before[2], &after[2]));
    assert!(!Rc::ptr_eq(&before_lines[0], &after_lines[0]));
    assert!(Rc::ptr_eq(&before_lines[2], &after_lines[2]));
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
