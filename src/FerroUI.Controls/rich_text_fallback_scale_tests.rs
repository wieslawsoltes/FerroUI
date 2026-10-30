//! Provider scale transport through real rich layout and its cache ownership
//! fences. The controlled mapper supplies known scales; no platform scale
//! availability is inferred from these tests.

use super::RichTextLayout;
use crate::documents::{Paragraph, Run};
use crate::{RichTextBlock, TextBlock};
use ferroui_base::media::text_formatting::testing::{
    line_text, TestFont, TestPlatformTypeface, TextTestScope,
};
use ferroui_base::media::text_formatting::ShapedTextRun;
use ferroui_base::media::{
    FontFallbackMapping, FontFamily, FontSimulations, GlyphTypeface, TextLineBounds, TextWrapping,
    Typeface,
};
use ferroui_base::platform::IFontFallbackMapper;
use ferroui_base::threading::Dispatcher;
use ferroui_base::utilities::CultureInfo;
use ferroui_base::{Rect, Ref, Size};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

struct Mapper {
    face: Rc<GlyphTypeface>,
    scale: Cell<f64>,
    calls: Cell<usize>,
    next: RefCell<Option<Box<dyn Fn()>>>,
}

impl Mapper {
    fn new() -> Rc<Self> {
        let font = TestFont::new("Provider Face")
            .with_ranges(&[(0x20, 0x7e), (0x3000, 0x30ff)])
            .with_advance(750)
            .with_vertical_metrics(1000, -500)
            .with_line_gap(700);
        Rc::new(Self {
            face: GlyphTypeface::new(
                Rc::new(TestPlatformTypeface::new(font)),
                FontSimulations::None,
            )
            .unwrap(),
            scale: Cell::new(2.0),
            calls: Cell::new(0),
            next: RefCell::new(None),
        })
    }
}

impl IFontFallbackMapper for Mapper {
    fn try_map_characters(
        &self,
        text: &[u16],
        requested: &Typeface,
        _: Option<&CultureInfo>,
    ) -> Option<FontFallbackMapping> {
        assert_eq!(requested.font_family().name(), "Requested");
        self.calls.set(self.calls.get() + 1);
        let scale = self.scale.get();
        let next = self.next.borrow_mut().take();
        if let Some(next) = next {
            next();
        }
        FontFallbackMapping::new(self.face.clone(), text.len(), scale)
    }
}

fn scope(mapper: Rc<Mapper>) -> TextTestScope {
    TextTestScope::with_fonts_and_fallback_mapper(
        vec![
            TestFont::new("Requested").with_ranges(&[(0x20, 0x7e)]),
            TestFont::new("Legacy Fallback").with_ranges(&[(0x3000, 0x30ff)]),
        ],
        mapper,
    )
}

fn view(text: &str) -> (Ref<RichTextBlock>, Ref<Run>) {
    let view = RichTextBlock::new();
    view.set_font_family(FontFamily::new("Requested"));
    view.set_font_size(20.0);
    view.set_text_wrapping(TextWrapping::NoWrap);
    let paragraph = Paragraph::new();
    let run = Run::with_text(Some(text));
    paragraph.add_child(run.clone());
    view.blocks().add(paragraph);
    (view, run)
}

fn measure(view: &RichTextBlock) {
    view.measure(Size::new(300.0, 200.0));
    view.arrange(Rect::new(0.0, 0.0, 300.0, 200.0));
}

fn text(layout: &RichTextLayout) -> String {
    layout
        .lines
        .iter()
        .map(|entry| line_text(&*entry.line))
        .collect()
}

fn close(actual: f64, expected: f64) {
    assert!(
        (actual - expected).abs() < 1e-8,
        "expected {expected}, got {actual}"
    );
}

#[test]
fn rich_text_fallback_scale_reaches_real_bounds_caret_and_ranges_without_changing_source_font_size()
{
    let _dispatcher = Dispatcher::unit_test_scope();
    let mapper = Mapper::new();
    let _text = scope(mapper.clone());
    let (view, run) = view("あい");
    let original = view.document().snapshot();
    let revision = view.document().revision();
    for (mode, baseline, height, legacy_baseline, legacy_height) in [
        (TextLineBounds::Full, 32.0, 44.0, 17.0, 22.0),
        (TextLineBounds::TrimToCapHeight, 22.4, 34.4, 11.2, 16.2),
        (TextLineBounds::TrimToBaseline, 32.0, 32.0, 17.0, 17.0),
        (TextLineBounds::Tight, 22.4, 22.4, 11.2, 11.2),
    ] {
        view.set_text_line_bounds(mode);
        measure(&view);
        let layout = view.page_layout();
        close(layout.lines[0].line.baseline(), baseline);
        close(layout.lines[0].line.height(), height);
        close(layout.lines[0].line.width(), 60.0);
        let caret = view.get_rect_from_character_index(1);
        close(caret.height, height);
        close(caret.x, 30.0);
        let ranges = view.get_rects_for_range(0..1);
        assert_eq!(ranges.len(), 1);
        close(ranges[0].width, 30.0);
        close(ranges[0].height, height);
        let runs = layout.lines[0].line.text_runs();
        let shaped = runs[0].downcast_ref::<ShapedTextRun>().unwrap();
        assert!(Rc::ptr_eq(
            &shaped.run_properties().cached_glyph_typeface(),
            &mapper.face
        ));
        close(shaped.run_properties().font_rendering_em_size(), 20.0);
        close(shaped.shaped_buffer().font_rendering_em_size(), 40.0);
        close(view.font_size(), 20.0);
        close(run.font_size(), 20.0);

        let before_plain = mapper.calls.get();
        let plain = TextBlock::new();
        plain.set_font_family(FontFamily::new("Requested"));
        plain.set_font_size(20.0);
        plain.set_text(Some("あい"));
        plain.set_text_line_bounds(mode);
        plain.measure(Size::new(300.0, 200.0));
        plain.arrange(Rect::new(0.0, 0.0, 300.0, 200.0));
        let plain_layout = plain.text_layout();
        let lines = plain_layout.text_lines();
        close(lines[0].baseline(), legacy_baseline);
        close(lines[0].height(), legacy_height);
        close(lines[0].width(), 20.0);
        assert_eq!(mapper.calls.get(), before_plain);
    }
    assert_eq!(view.document().snapshot(), original);
    assert_eq!(view.document().revision(), revision);
}

#[test]
fn rich_text_fallback_scale_source_mutation_during_mapping_retires_the_outer_layout() {
    let _dispatcher = Dispatcher::unit_test_scope();
    let mapper = Mapper::new();
    let _text = scope(mapper.clone());
    let (view, run) = view("old");
    measure(&view);
    view.invalidate_document_layout();
    let before = view.page_layout_generation();
    let revision = view.document().revision();
    let changed_run = run.clone();
    *mapper.next.borrow_mut() = Some(Box::new(move || changed_run.set_text(Some("new text"))));
    let stale = view.page_layout();
    assert_eq!(text(&stale), "old");
    assert_eq!(view.document().text(), "new text");
    assert!(view.document().revision() > revision);
    assert_ne!(view.page_layout_generation(), before);
    assert!(view.cached_page_layout().is_none());
    let current = view.page_layout();
    assert_eq!(text(&current), "new text");
    assert!(Rc::ptr_eq(&current, &view.cached_page_layout().unwrap()));
    assert!(!Rc::ptr_eq(&stale.source, &current.source));
    close(current.lines[0].line.width(), 240.0);
    close(run.font_size(), 20.0);
    let accepted_revision = view.document().revision();
    assert!(Rc::ptr_eq(&current, &view.page_layout()));
    assert_eq!(view.document().revision(), accepted_revision);
}

#[test]
fn rich_text_fallback_scale_reentry_preserves_newer_cache_and_does_not_mutate_document() {
    let _dispatcher = Dispatcher::unit_test_scope();
    let mapper = Mapper::new();
    let _text = scope(mapper.clone());
    let (view, _) = view("ab");
    measure(&view);
    view.invalidate_document_layout();
    let snapshot = view.document().snapshot();
    let revision = view.document().revision();
    let current = Rc::new(RefCell::new(None));
    let captured = current.clone();
    let owner = view.clone();
    let provider = Rc::downgrade(&mapper);
    *mapper.next.borrow_mut() = Some(Box::new(move || {
        provider.upgrade().unwrap().scale.set(0.5);
        owner.invalidate_document_layout();
        captured.replace(Some(owner.page_layout()));
    }));
    let stale = view.page_layout();
    close(stale.lines[0].line.height(), 44.0);
    let accepted = current.borrow().clone().unwrap();
    close(accepted.lines[0].line.height(), 11.0);
    close(accepted.lines[0].line.width(), 15.0);
    assert!(Rc::ptr_eq(&accepted, &view.cached_page_layout().unwrap()));
    assert!(!Rc::ptr_eq(&stale, &accepted));
    assert!(!Rc::ptr_eq(&stale.source, &accepted.source));
    let calls = mapper.calls.get();
    assert!(Rc::ptr_eq(&accepted, &view.page_layout()));
    assert_eq!(mapper.calls.get(), calls);
    assert_eq!(view.document().snapshot(), snapshot);
    assert_eq!(view.document().revision(), revision);
}
