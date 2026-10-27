use std::cell::{Cell, RefCell};
use std::rc::Rc;

use crate::test_support::{engine, measure_width, FONT, PRE_WRAP};
use crate::*;

type ShapeCallback = dyn Fn(&str, &str, PretextShapeOptions) -> PretextShapedRun;

#[derive(Clone)]
struct Factory {
    shape: Rc<ShapeCallback>,
    supported: bool,
    priority: i32,
}

impl Factory {
    fn new(shape: impl Fn(&str, &str, PretextShapeOptions) -> PretextShapedRun + 'static) -> Self {
        Self { shape: Rc::new(shape), supported: true, priority: 0 }
    }
}

impl IPretextTextMeasurerFactory for Factory {
    fn name(&self) -> &str { "CombinedTest" }
    fn is_supported(&self) -> bool { self.supported }
    fn priority(&self) -> i32 { self.priority }
    fn create(&self, font: &str) -> Result<Rc<dyn IPretextTextMeasurer>, PretextError> { DelegateTextMeasurerFactory::new(measure_width).create(font) }
    fn shaper_factory(&self) -> Option<Rc<dyn IPretextTextShaperFactory>> { Some(Rc::new(self.clone())) }
}

impl IPretextTextShaperFactory for Factory {
    fn name(&self) -> &str { "CombinedTest" }
    fn is_supported(&self) -> bool { self.supported }
    fn create_shaper(&self, font: &str) -> Result<Rc<dyn IPretextTextShaper>, PretextError> { Ok(Rc::new(Shaper { font: font.into(), shape: self.shape.clone() })) }
}

struct Shaper { font: String, shape: Rc<ShapeCallback> }
impl IPretextTextShaper for Shaper {
    fn shape_text(&self, text: &str, options: PretextShapeOptions) -> Result<PretextShapedRun, PretextError> { Ok((self.shape)(text, &self.font, options)) }
}

fn mapped(text: &str, font: &str, _: PretextShapeOptions) -> PretextShapedRun {
    let mut utf16_index = 0;
    let glyphs: Vec<_> = text.chars().enumerate().map(|(index, ch)| {
        let glyph = PretextShapedGlyph { glyph_id: ch as u32, cluster: utf16_index, x: index as f64, y: 0.0, x_advance: 1.0, y_advance: 0.0, x_offset: 0.0, y_offset: 0.0, font_run_index: 0 };
        utf16_index += ch.len_utf16();
        glyph
    }).collect();
    let count = glyphs.len();
    PretextShapedRun::new(PretextGlyphRunKind::Mapped, glyphs, vec![PretextShapedFontRun { index: 0, font: font.into(), first_glyph_index: 0, glyph_count: count }], count as f64, 0.0)
}

fn counting_engine() -> (PretextLayout, Rc<Cell<usize>>) {
    let calls = Rc::new(Cell::new(0));
    let counter = calls.clone();
    (PretextLayout::new(Rc::new(Factory::new(move |text, font, options| {
        counter.set(counter.get() + 1); mapped(text, font, options)
    }))), calls)
}

fn prepared(engine: &PretextLayout, text: &str) -> Rc<PreparedTextWithSegments> { engine.prepare_with_segments(text, FONT, PrepareOptions::default()).unwrap() }
fn range(start: LayoutCursor, end: LayoutCursor) -> LayoutLineRange { LayoutLineRange { start, end, width: 0.0 } }

#[test]
fn shape_text_uses_configured_measurer_factory_when_it_can_shape() {
    let engine = PretextLayout::new(Rc::new(Factory::new(mapped)));
    let run = engine.shape_text("abc", FONT, PretextShapeOptions::default()).unwrap();
    assert_eq!(run.kind(), PretextGlyphRunKind::Mapped);
    assert_eq!(run.glyphs().len(), 3);
    assert_eq!(run.font_runs()[0].font, FONT);
    assert_eq!(run.font_runs()[0].glyph_count, 3);
    assert_eq!(run.advance_x(), 3.0);
    assert!(!run.has_missing_glyphs());
}

#[test]
fn shape_text_reuses_cached_output_until_cache_clear() {
    let (engine, calls) = counting_engine();
    let first = engine.shape_text("abc", FONT, PretextShapeOptions::default()).unwrap();
    let second = engine.shape_text("abc", FONT, PretextShapeOptions::default()).unwrap();
    let rtl = engine.shape_text("abc", FONT, PretextShapeOptions { direction: PretextTextDirection::RightToLeft }).unwrap();
    assert!(Rc::ptr_eq(&first, &second));
    assert!(!Rc::ptr_eq(&first, &rtl));
    assert_eq!(calls.get(), 2);
    engine.clear_cache();
    assert!(!Rc::ptr_eq(&first, &engine.shape_text("abc", FONT, PretextShapeOptions::default()).unwrap()));
    assert_eq!(calls.get(), 3);
}

#[test]
fn shape_text_forwards_direction_options_to_configured_shaper() {
    let observed = Rc::new(Cell::new(PretextTextDirection::Auto));
    let capture = observed.clone();
    let engine = PretextLayout::new(Rc::new(Factory::new(move |text, font, options| { capture.set(options.direction); mapped(text, font, options) })));
    engine.shape_text("123", FONT, PretextShapeOptions { direction: PretextTextDirection::RightToLeft }).unwrap();
    assert_eq!(observed.get(), PretextTextDirection::RightToLeft);
}

#[test]
fn shaping_contract_reports_clusters_as_utf16_text_indices() {
    let engine = PretextLayout::new(Rc::new(Factory::new(mapped)));
    let shaped = engine.shape_text("a😀b", FONT, PretextShapeOptions::default()).unwrap();
    assert_eq!(shaped.glyphs().iter().map(|glyph| glyph.cluster).collect::<Vec<_>>(), [0, 1, 3]);
}

#[test]
fn shape_prepared_text_reuses_full_prepared_shaping_for_safe_line_ranges() {
    let (engine, calls) = counting_engine();
    let prepared = prepared(&engine, "hello world");
    let shaped = engine.shape_prepared_text(prepared.clone(), PretextShapeOptions::default()).unwrap();
    let cached = engine.shape_prepared_text(prepared.clone(), PretextShapeOptions::default()).unwrap();
    assert!(Rc::ptr_eq(&shaped, &cached));
    let line = PretextLayout::layout_next_line_range(&prepared, LayoutCursor::default(), 1000.0).unwrap();
    let first = PretextLayout::materialize_shaped_line_range(&shaped, &line).unwrap();
    let second = PretextLayout::materialize_shaped_line_range(&cached, &line).unwrap();
    assert!(Rc::ptr_eq(&first.shaped_run, &second.shaped_run));
    assert_eq!(calls.get(), 1);
    assert_eq!(first.shaped_run.glyphs().len(), 11);
}

#[test]
fn shape_prepared_text_slices_full_first_segment_ranges() {
    let (engine, calls) = counting_engine();
    let shaped = engine.shape_prepared_text(prepared(&engine, "abcdef"), PretextShapeOptions::default()).unwrap();
    let line = PretextLayout::materialize_shaped_line_range(&shaped, &range(LayoutCursor::default(), LayoutCursor::new(0, 6))).unwrap();
    assert_eq!(calls.get(), 1);
    assert_eq!(line.shaped_run.glyphs().len(), 6);
    assert_eq!(line.shaped_run.advance_x(), 6.0);
}

#[test]
fn shape_prepared_text_preserves_negative_advances_when_slicing() {
    let engine = PretextLayout::new(Rc::new(Factory::new(|text, font, options| {
        let run = mapped(text, font, options);
        let glyphs = run.glyphs().iter().enumerate().map(|(i, glyph)| PretextShapedGlyph { x: -(i as f64), y: -(i as f64) * 2.0, x_advance: -1.0, y_advance: -2.0, ..*glyph }).collect();
        PretextShapedRun::new(PretextGlyphRunKind::Shaped, glyphs, run.font_runs().to_vec(), -run.advance_x(), -run.advance_x() * 2.0)
    })));
    let shaped = engine.shape_prepared_text(prepared(&engine, "abc"), PretextShapeOptions { direction: PretextTextDirection::RightToLeft }).unwrap();
    let line = PretextLayout::materialize_shaped_line_range(&shaped, &range(LayoutCursor::default(), LayoutCursor::new(0, 3))).unwrap();
    assert_eq!(line.shaped_run.advance_x(), -3.0);
    assert_eq!(line.shaped_run.advance_y(), -6.0);
}

#[test]
fn shape_prepared_text_reshapes_unsafe_partial_segment_ranges() {
    let (engine, calls) = counting_engine();
    let shaped = engine.shape_prepared_text(prepared(&engine, "abcdef"), PretextShapeOptions::default()).unwrap();
    let range = range(LayoutCursor::default(), LayoutCursor::new(0, 2));
    let first = PretextLayout::materialize_shaped_line_range(&shaped, &range).unwrap();
    let second = PretextLayout::materialize_shaped_line_range(&shaped, &range).unwrap();
    assert!(Rc::ptr_eq(&first.shaped_run, &second.shaped_run));
    assert_eq!(calls.get(), 2);
    assert_eq!(first.shaped_run.advance_x(), 2.0);
}

#[test]
fn shape_prepared_text_reshapes_invisible_break_boundary_ranges() {
    let calls = Rc::new(RefCell::new(Vec::new()));
    let capture = calls.clone();
    let engine = PretextLayout::new(Rc::new(Factory::new(move |text, font, options| {
        capture.borrow_mut().push(text.to_owned());
        if text == "fi" {
            let mut run = mapped("ﬁ", font, options);
            let mut glyph = run.glyphs()[0]; glyph.x_advance = 2.0;
            run = PretextShapedRun::new(PretextGlyphRunKind::Shaped, vec![glyph], run.font_runs().to_vec(), 2.0, 0.0); run
        } else { mapped(text, font, options) }
    })));
    let prepared = engine.prepare_with_segments("f\ni", FONT, PRE_WRAP).unwrap();
    let shaped = engine.shape_prepared_text(prepared.clone(), PretextShapeOptions::default()).unwrap();
    let first = PretextLayout::layout_next_line_range(&prepared, LayoutCursor::default(), 1000.0).unwrap();
    let second = PretextLayout::layout_next_line_range(&prepared, first.end, 1000.0).unwrap();
    assert_eq!(PretextLayout::materialize_shaped_line_range(&shaped, &first).unwrap().shaped_run.advance_x(), 1.0);
    assert_eq!(PretextLayout::materialize_shaped_line_range(&shaped, &second).unwrap().shaped_run.advance_x(), 1.0);
    assert_eq!(*calls.borrow(), ["fi", "f", "i"]);
}

#[test]
fn try_shape_text_returns_none_when_no_shaper_is_available() {
    let engine = engine();
    assert!(engine.try_shape_text("abc", FONT, PretextShapeOptions::default()).is_none());
    let mut factory = Factory::new(mapped); factory.supported = false;
    engine.set_text_shaper_factory(Some(Rc::new(factory)));
    assert!(engine.try_shape_text("abc", FONT, PretextShapeOptions::default()).is_none());
}

#[test]
fn shape_text_explicit_shaper_survives_measurer_configuration() {
    let engine = engine();
    engine.set_text_shaper_factory(Some(Rc::new(Factory::new(|text, _, options| mapped(text, "ExplicitShaper", options)))));
    engine.set_text_measurer_factory(Rc::new(DelegateTextMeasurerFactory::new(measure_width)));
    assert_eq!(engine.shape_text("abc", FONT, PretextShapeOptions::default()).unwrap().font_runs()[0].font, "ExplicitShaper");
}

#[test]
fn shape_options_default_returns_fresh_value() {
    let mut first = PretextShapeOptions::default(); first.direction = PretextTextDirection::RightToLeft;
    assert_eq!(PretextShapeOptions::default().direction, PretextTextDirection::Auto);
    assert_eq!(first.direction, PretextTextDirection::RightToLeft);
}

#[test]
fn shape_prepared_text_options_are_returned_as_snapshots() {
    let (engine, _) = counting_engine();
    let shaped = engine.shape_prepared_text(prepared(&engine, "abc"), PretextShapeOptions { direction: PretextTextDirection::LeftToRight }).unwrap();
    let mut snapshot = shaped.options(); snapshot.direction = PretextTextDirection::RightToLeft;
    assert_eq!(shaped.direction(), PretextTextDirection::LeftToRight);
    assert_eq!(shaped.options().direction, PretextTextDirection::LeftToRight);
    assert_eq!(snapshot.direction, PretextTextDirection::RightToLeft);
}

#[test]
fn registered_backend_selection_uses_supported_priority_and_combined_shaping() {
    let low = Rc::new(Factory::new(mapped));
    let mut high = Factory::new(|text, _, options| mapped(text, "High", options)); high.priority = 10;
    let mut unsupported = high.clone(); unsupported.priority = 100; unsupported.supported = false;
    let engine = PretextLayout::from_factories([low as Rc<dyn IPretextTextMeasurerFactory>, Rc::new(unsupported), Rc::new(high)]).unwrap();
    assert_eq!(engine.shape_text("hello", FONT, PretextShapeOptions::default()).unwrap().font_runs()[0].font, "High");
    assert!(PretextLayout::from_factories(Vec::new()).is_err());
}

#[test]
fn retained_prepared_shaping_keeps_its_backend_after_reconfiguration() {
    let (engine, calls) = counting_engine();
    let prepared = prepared(&engine, "abcdef");
    let original = engine.shape_prepared_text(prepared.clone(), PretextShapeOptions::default()).unwrap();
    engine.set_text_shaper_factory(Some(Rc::new(Factory::new(|text, _, options| mapped(text, "Replacement", options)))));
    let replacement = engine.shape_prepared_text(prepared, PretextShapeOptions::default()).unwrap();
    let partial = range(LayoutCursor::default(), LayoutCursor::new(0, 2));
    assert_eq!(PretextLayout::materialize_shaped_line_range(&original, &partial).unwrap().shaped_run.font_runs()[0].font, FONT);
    assert_eq!(PretextLayout::materialize_shaped_line_range(&replacement, &partial).unwrap().shaped_run.font_runs()[0].font, "Replacement");
    assert_eq!(calls.get(), 2);
}

#[test]
fn shaped_preparation_cache_does_not_keep_documents_alive() {
    let (engine, _) = counting_engine();
    let prepared = prepared(&engine, "temporary document");
    let weak = Rc::downgrade(&prepared);
    let shaped = engine.shape_prepared_text(prepared.clone(), PretextShapeOptions::default()).unwrap();
    drop(prepared); drop(shaped);
    assert!(weak.upgrade().is_none());
}

#[test]
fn shaped_utf16_cursor_ranges_rebase_astral_clusters() {
    let (engine, _) = counting_engine();
    let text = prepared(&engine, "a😀b");
    let shaped = engine.shape_prepared_text(text.clone(), PretextShapeOptions::default()).unwrap();
    let all = range(LayoutCursor::default(), LayoutCursor::new(text.segments().len(), 0));
    let line = PretextLayout::materialize_shaped_line_range(&shaped, &all).unwrap();
    assert_eq!(line.shaped_run.glyphs().iter().map(|glyph| glyph.cluster).collect::<Vec<_>>(), [0, 1, 3]);
}

#[test]
fn rich_inline_shaping_preserves_source_indices_fonts_and_atomic_width() {
    let (engine, _) = counting_engine();
    let rich = engine.prepare_rich_inline(&[RichInlineItem::new("Hello ", FONT), RichInlineItem::new("@maya", "700 12px Test Sans").with_break_mode(RichInlineBreakMode::Never).with_extra_width(18.0)]).unwrap();
    let range = PretextLayout::layout_next_rich_inline_line_range(&rich, 1000.0, None).unwrap();
    let shaped = engine.materialize_shaped_rich_inline_line_range(&rich, &range, PretextShapeOptions::default()).unwrap();
    assert_eq!(shaped.fragments.len(), 2);
    assert_eq!(shaped.fragments[1].item_index, 1);
    assert_eq!(shaped.fragments[1].shaped_run.font_runs()[0].font, "700 12px Test Sans");
    assert_eq!(shaped.fragments[1].occupied_width, range.fragments[1].occupied_width);
}

#[test]
fn invalid_shaped_cursor_returns_error_without_panicking() {
    let (engine, _) = counting_engine();
    let shaped = engine.shape_prepared_text(prepared(&engine, "abc"), PretextShapeOptions::default()).unwrap();
    assert!(PretextLayout::materialize_shaped_line_range(&shaped, &range(LayoutCursor::default(), LayoutCursor::new(9, 0))).is_err());
    assert!(PretextLayout::materialize_shaped_line_range(&shaped, &range(LayoutCursor::new(0, 4), LayoutCursor::new(1, 0))).is_err());
}

#[test]
fn convenience_and_rich_calls_reuse_line_caches_without_retaining_wrappers() {
    let (engine, calls) = counting_engine();
    let prepared = prepared(&engine, "abcdef");
    let partial = range(LayoutCursor::default(), LayoutCursor::new(0, 2));
    let first = engine.materialize_shaped_line_range_with_options(prepared.clone(), &partial, PretextShapeOptions::default()).unwrap();
    let second = engine.materialize_shaped_line_range_with_options(prepared, &partial, PretextShapeOptions::default()).unwrap();
    assert!(Rc::ptr_eq(&first.shaped_run, &second.shaped_run));
    assert_eq!(calls.get(), 2);
    let rich = engine.prepare_rich_inline(&[RichInlineItem::new("word", FONT)]).unwrap();
    let line = PretextLayout::layout_next_rich_inline_line_range(&rich, 1000.0, None).unwrap();
    let first = engine.materialize_shaped_rich_inline_line_range(&rich, &line, PretextShapeOptions::default()).unwrap();
    let second = engine.materialize_shaped_rich_inline_line_range(&rich, &line, PretextShapeOptions::default()).unwrap();
    assert!(Rc::ptr_eq(&first.fragments[0].shaped_run, &second.fragments[0].shaped_run));
}

#[test]
fn builder_reconfiguration_invalidates_previously_used_shaper_state() {
    let (engine, _) = counting_engine();
    engine.shape_text("abc", FONT, PretextShapeOptions::default()).unwrap();
    let engine = engine.with_shaper_factory(Rc::new(Factory::new(|text, _, options| mapped(text, "New", options))));
    assert_eq!(engine.shape_text("abc", FONT, PretextShapeOptions::default()).unwrap().font_runs()[0].font, "New");
}

#[test]
fn replacing_a_factory_drops_it_outside_registry_borrows() {
    struct ReentrantDrop { owner: std::rc::Weak<PretextLayout>, dropped: Rc<Cell<bool>> }
    impl IPretextTextMeasurerFactory for ReentrantDrop {
        fn name(&self) -> &str { "reentrant" }
        fn create(&self, font: &str) -> Result<Rc<dyn IPretextTextMeasurer>, PretextError> { DelegateTextMeasurerFactory::new(measure_width).create(font) }
    }
    impl Drop for ReentrantDrop {
        fn drop(&mut self) {
            if let Some(owner) = self.owner.upgrade() { let _current = owner.text_measurer_factory(); owner.clear_cache(); }
            self.dropped.set(true);
        }
    }
    let engine = Rc::new(engine());
    let dropped = Rc::new(Cell::new(false));
    engine.set_text_measurer_factory(Rc::new(ReentrantDrop { owner: Rc::downgrade(&engine), dropped: dropped.clone() }));
    engine.set_text_measurer_factory(Rc::new(DelegateTextMeasurerFactory::new(measure_width)));
    assert!(dropped.get());
}
