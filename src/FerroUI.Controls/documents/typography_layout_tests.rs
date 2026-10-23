use crate::documents::{Paragraph, RichTextDocument, Run, Typography};
use crate::presenters::TextPresenter;
use crate::test_support::test_scope;
use crate::{Control, RichTextBlock, RichTextBlockOverflow, SelectableTextBlock, TextBlock};
use ferroui_base::media::text_formatting::testing::{paragraph_properties, TestTextShaperImpl};
use ferroui_base::media::text_formatting::{
    FormattedTextSource, ShapedBuffer, TextFormatter, TextLayout, TextRunProperties, TextShaper,
    TextShaperOptions,
};
use ferroui_base::media::{
    Brushes, FontFeature, FontFeatureCollection, GlyphTypeface, ITextShaperTypeface, TextWrapping,
};
use ferroui_base::platform::ITextShaperImpl;
use ferroui_base::utilities::{ReadOnlyMemory, ValueSpan};
use ferroui_base::{FerroLocator, Rect, Size};
use std::cell::RefCell;
use std::rc::Rc;

fn layout(control: &Control) {
    control.measure(Size::new(400.0, 300.0));
    control.arrange(Rect::new(0.0, 0.0, 400.0, 300.0));
    assert!(control.is_measure_valid());
}

fn assert_feature(properties: &dyn TextRunProperties, tag: &str, value: i32) {
    let features = properties
        .font_features()
        .expect("the formatter received typography");
    assert_eq!(
        features
            .list()
            .snapshot()
            .iter()
            .rev()
            .find(|feature| feature.tag == tag)
            .map(|feature| feature.value),
        Some(value)
    );
}

fn assert_layout_feature(layout: &TextLayout, tag: &str, value: i32) {
    let mut seen = 0;
    for line in layout.text_lines() {
        for run in line.text_runs().iter() {
            if let Some(properties) = run.properties() {
                assert_feature(&**properties, tag, value);
                seen += 1;
            }
        }
    }
    assert!(seen > 0);
}

fn layout_text(layout: &TextLayout) -> String {
    let mut text = String::new();
    for line in layout.text_lines() {
        for run in line.text_runs().iter() {
            text.push_str(&run.text().to_string_lossy());
        }
    }
    text
}

#[derive(Default)]
struct FeatureProbe {
    calls: RefCell<Vec<(String, Vec<FontFeature>)>>,
}

struct ObservedShaper {
    inner: TestTextShaperImpl,
    probe: Rc<FeatureProbe>,
}

impl ITextShaperImpl for ObservedShaper {
    fn shape_text(
        &self,
        text: &ReadOnlyMemory<u16>,
        options: &TextShaperOptions,
    ) -> Rc<ShapedBuffer> {
        self.probe.calls.borrow_mut().push((
            String::from_utf16_lossy(text.span()),
            options
                .font_features()
                .map(|features| features.as_ref().clone())
                .unwrap_or_default(),
        ));
        self.inner.shape_text(text, options)
    }
    fn create_typeface(&self, glyph_typeface: &GlyphTypeface) -> Rc<dyn ITextShaperTypeface> {
        self.inner.create_typeface(glyph_typeface)
    }
}

fn observe_shaping() -> Rc<FeatureProbe> {
    let probe = Rc::new(FeatureProbe::default());
    let shaper = Rc::new(TextShaper::new(Rc::new(ObservedShaper {
        inner: TestTextShaperImpl::new(),
        probe: probe.clone(),
    })));
    FerroLocator::current_mutable()
        .bind::<TextShaper>()
        .to_constant(shaper);
    probe
}

fn kern(value: i32) -> FontFeature {
    FontFeature {
        tag: "kern".to_owned(),
        value,
        start: 0,
        end: -1,
    }
}

#[test]
fn typography_formatter_respects_feature_boundaries_and_coalesces_equivalent_values() {
    let _scope = test_scope();
    for equivalent in [false, true] {
        let probe = observe_shaping();
        let first = Run::new();
        let second = Run::new();
        Typography::set_kerning(&first, false);
        Typography::set_kerning(&second, !equivalent);
        let first_properties = first.create_text_run_properties();
        let second_properties = second.create_text_run_properties();
        let source = FormattedTextSource::new(
            ReadOnlyMemory::from_str("AV"),
            first_properties.clone(),
            Some(Rc::from([ValueSpan::new(1, 1, second_properties)])),
        );
        let line = <dyn TextFormatter>::current()
            .format_line(
                &source,
                0,
                200.0,
                &paragraph_properties(&first_properties, TextWrapping::NoWrap),
                None,
            )
            .unwrap();
        let calls = probe.calls.borrow();
        let text = calls.iter().map(|call| call.0.as_str()).collect::<Vec<_>>();
        if equivalent {
            assert_eq!(text, ["AV"]);
        } else {
            assert_eq!(text, ["A", "V"]);
            assert!(calls[0].1.contains(&kern(0)));
            assert!(!calls[1].1.contains(&kern(0)));
        }
        line.dispose();
    }
}

#[test]
fn typography_formatter_keeps_ordered_raw_overrides_as_distinct_shaping_inputs() {
    let _scope = test_scope();
    let probe = observe_shaping();
    let first = Run::new();
    let second = Run::new();
    Typography::set_slashed_zero(&first, true);
    Typography::set_slashed_zero(&second, true);
    first.set_font_features(Some(FontFeatureCollection::from_items([kern(0), kern(1)])));
    second.set_font_features(Some(FontFeatureCollection::from_items([kern(1), kern(0)])));
    let first_properties = first.create_text_run_properties();
    let source = FormattedTextSource::new(
        ReadOnlyMemory::from_str("AV"),
        first_properties.clone(),
        Some(Rc::from([ValueSpan::new(
            1,
            1,
            second.create_text_run_properties(),
        )])),
    );
    let line = <dyn TextFormatter>::current()
        .format_line(
            &source,
            0,
            200.0,
            &paragraph_properties(&first_properties, TextWrapping::NoWrap),
            None,
        )
        .unwrap();
    let calls = probe.calls.borrow();
    assert_eq!(calls.len(), 2);
    assert_eq!(calls[0].1.last(), Some(&kern(1)));
    assert_eq!(calls[1].1.last(), Some(&kern(0)));
    line.dispose();
}

#[test]
fn typography_reaches_plain_selectable_and_presenter_selection_and_preedit_runs() {
    let _scope = test_scope();
    let plain = TextBlock::new();
    plain.set_text(Some("AV"));
    Typography::set_kerning(&plain, false);
    layout(&plain);
    assert_eq!(layout_text(&plain.text_layout()), "AV");
    assert_layout_feature(&plain.text_layout(), "kern", 0);
    let selectable = SelectableTextBlock::new();
    selectable.set_text(Some("AV"));
    selectable.set_selection_foreground_brush(Some(Brushes::blue()));
    selectable.set_selection_start(0);
    selectable.set_selection_end(1);
    Typography::set_kerning(&selectable, false);
    layout(&selectable);
    assert_eq!(layout_text(&selectable.text_layout()), "AV");
    assert_layout_feature(&selectable.text_layout(), "kern", 0);
    let presenter = TextPresenter::new();
    presenter.set_text(Some("AV"));
    presenter.set_selection_foreground_brush(Some(Brushes::blue()));
    presenter.set_show_selection_highlight(true);
    presenter.set_selection_start(0);
    presenter.set_selection_end(1);
    Typography::set_kerning(&presenter, false);
    layout(&presenter);
    assert_eq!(layout_text(&presenter.text_layout()), "AV");
    assert_layout_feature(&presenter.text_layout(), "kern", 0);
    presenter.set_caret_index(1);
    presenter.set_preedit_text(Some("X"));
    assert_eq!(layout_text(&presenter.text_layout()), "AXV");
    assert_layout_feature(&presenter.text_layout(), "kern", 0);
    assert_eq!(presenter.preedit_text().as_deref(), Some("X"));
}

#[test]
fn typography_selected_inline_keeps_its_explicit_reset_to_the_normal_font_features() {
    let _scope = test_scope();
    let selectable = SelectableTextBlock::new();
    Typography::set_kerning(&selectable, false);
    let run = Run::with_text(Some("AV"));
    Typography::set_kerning(&run, true);
    selectable.inlines().unwrap().add(run.clone());
    // Establish a finite constraint, then invalidate the inline-run cache by
    // selecting. The first requested selected layout must use the inline reset.
    layout(&selectable);
    selectable.set_selection_foreground_brush(Some(Brushes::blue()));
    selectable.set_selection_start(0);
    selectable.set_selection_end(1);
    for enabled in [true, false, true] {
        Typography::set_kerning(&run, enabled);
        let layout = selectable.text_layout();
        assert_eq!(layout_text(&layout), "AV");
        let mut seen = 0;
        for line in layout.text_lines() {
            for run in line.text_runs().iter() {
                if let Some(properties) = run.properties() {
                    if enabled {
                        assert!(properties.font_features().is_none());
                    } else {
                        assert_feature(&**properties, "kern", 0);
                    }
                    seen += 1;
                }
            }
        }
        assert!(seen > 0);
    }
}

#[test]
fn typography_rich_empty_paragraph_and_overflow_continuations_share_the_effective_input() {
    let _scope = test_scope();
    let view = RichTextBlock::new();
    Typography::set_kerning(&view, false);
    assert_feature(
        &*view.page_layout().source.paragraphs[0].properties,
        "kern",
        0,
    );
    view.blocks().add(Paragraph::new());
    assert_feature(
        &*view.page_layout().source.paragraphs[0].properties,
        "kern",
        0,
    );
    view.set_document(RichTextDocument::from_text("AV\nAV\nAV\nAV"));
    let overflow = RichTextBlockOverflow::new();
    view.set_overflow_content_target(Some(overflow.clone()));
    view.measure(Size::new(100.0, 15.0));
    view.arrange(Rect::new(0.0, 0.0, 100.0, 15.0));
    layout(&overflow);
    let initial = view.page_layout();
    let continued = overflow.page_layout();
    assert!(Rc::ptr_eq(&initial.source, &continued.source));
    assert!(!continued.lines.is_empty());
    for line in initial.lines.iter().chain(continued.lines.iter()) {
        for run in line.line.text_runs().iter() {
            if let Some(properties) = run.properties() {
                assert_feature(&**properties, "kern", 0);
            }
        }
    }
    Typography::set_kerning(&view, true);
    assert!(!view.is_measure_valid());
    assert!(!overflow.is_measure_valid());
    let next = overflow.page_layout();
    assert!(!Rc::ptr_eq(&next.source, &continued.source));
    assert!(next.source.paragraphs[0]
        .properties
        .font_features()
        .is_none());
}

#[test]
fn typography_text_block_observes_raw_mutation_and_releases_replaced_collection_and_owner() {
    let _scope = test_scope();
    let old = FontFeatureCollection::from_items([kern(1)]);
    let current = FontFeatureCollection::new();
    let block = TextBlock::new();
    block.set_text(Some("AV"));
    Typography::set_kerning(&block, false);
    block.set_font_features(Some(old.clone()));
    layout(&block);
    assert!(old.list().has_collection_changed_subscribers());
    old.add(kern(0));
    assert!(!block.is_measure_valid());
    layout(&block);
    assert_layout_feature(&block.text_layout(), "kern", 0);
    block.set_font_features(Some(current.clone()));
    layout(&block);
    assert!(!old.list().has_collection_changed_subscribers());
    old.add(kern(1));
    assert!(block.is_measure_valid());
    let weak = block.downgrade();
    drop(block);
    assert!(weak.upgrade().is_none());
    assert!(!current.list().has_collection_changed_subscribers());
}

#[test]
fn typography_presenter_observes_raw_mutation_without_cancelling_preedit_and_detaches() {
    let _scope = test_scope();
    let features = FontFeatureCollection::new();
    let presenter = TextPresenter::new();
    presenter.set_text(Some("AV"));
    presenter.set_caret_index(1);
    presenter.set_preedit_text(Some("X"));
    Typography::set_kerning(&presenter, false);
    presenter.set_font_features(Some(features.clone()));
    layout(&presenter);
    features.add(kern(1));
    assert!(!presenter.is_measure_valid());
    assert_eq!(presenter.preedit_text().as_deref(), Some("X"));
    assert_layout_feature(&presenter.text_layout(), "kern", 1);
    presenter.set_font_features(None);
    assert!(!features.list().has_collection_changed_subscribers());
    layout(&presenter);
    features.add(kern(0));
    assert!(presenter.is_measure_valid());
    presenter.set_font_features(Some(features.clone()));
    let weak = presenter.downgrade();
    drop(presenter);
    assert!(weak.upgrade().is_none());
    assert!(!features.list().has_collection_changed_subscribers());
}

#[test]
fn typography_empty_rich_view_observes_raw_features_without_changing_document_history() {
    let _scope = test_scope();
    let features = FontFeatureCollection::new();
    let view = RichTextBlock::new();
    Typography::set_kerning(&view, false);
    view.set_font_features(Some(features.clone()));
    layout(&view);
    let document = view.document();
    let snapshot = document.snapshot();
    let revision = document.revision();
    let mutation = document.mutation_version();
    features.add(kern(1));
    assert!(!view.is_measure_valid());
    assert_feature(
        &*view.page_layout().source.paragraphs[0].properties,
        "kern",
        1,
    );
    assert_eq!(document.snapshot(), snapshot);
    assert_eq!(document.revision(), revision);
    assert_eq!(document.mutation_version(), mutation);
    let weak = view.downgrade();
    drop(view);
    assert!(weak.upgrade().is_none());
    assert!(!features.list().has_collection_changed_subscribers());
}
