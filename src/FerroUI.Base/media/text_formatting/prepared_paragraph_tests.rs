//! Prepared paragraphs and the ink of a line measured when it is asked for (steps 3 and 6 of
//! docs/porting/text-layout-performance.md).

use std::rc::Rc;
use std::sync::Arc;

use crate::media::text_formatting::prepared_paragraph::{SHARED_CAPACITY, SHARED_MAX_TEXT_LENGTH};
use crate::media::text_formatting::testing::{
    paragraph_properties, paragraph_properties_with, run_properties, SingleBufferTextSource, TextTestScope,
};
use crate::media::text_formatting::{
    ITextSource, PreparedLine, PreparedParagraph, ShapedTextRun, TextLayout, TextLine, TextParagraphProperties,
    TextRunCache, TextShaperOptions,
};
use crate::media::{FlowDirection, FontManager, TextAlignment, TextTrimming, TextWrapping};
use crate::platform::IGlyphRunImpl;

const TEXT: &str = "The quick brown fox jumps over the lazy dog.\nA verylongwordthatdoesnotfitinanarrowcolumn and a tail \
                    of words, with\ta tab and trailing spaces   \n\nLast paragraph.";

fn fresh_layout(text: &str, properties: &Rc<dyn TextParagraphProperties>, width: f64) -> TextLayout {
    let source: Rc<dyn ITextSource> =
        Rc::new(SingleBufferTextSource::new(text, properties.default_text_run_properties().clone()));
    TextLayout::from_text_source(source, properties.clone(), None, width, f64::INFINITY, 0, None)
}

fn lines_of(layout: &TextLayout) -> Vec<PreparedLine> {
    layout
        .text_lines()
        .iter()
        .map(|line| PreparedLine {
            start: line.first_text_source_index(),
            length: line.length(),
            width: line.width(),
            width_including_trailing_whitespace: line.width_including_trailing_whitespace(),
            height: line.height(),
            baseline: line.baseline(),
        })
        .collect()
}

fn platform_runs(line: &Rc<dyn TextLine>) -> Vec<Option<Arc<dyn IGlyphRunImpl>>> {
    line.text_runs()
        .iter()
        .filter_map(|run| run.downcast_ref::<ShapedTextRun>())
        .map(|run| run.glyph_run().created_platform_impl())
        .collect()
}

#[test]
fn a_prepared_paragraph_has_the_lines_of_a_new_layout_at_every_width() {
    let _scope = TextTestScope::new();

    for wrapping in [TextWrapping::Wrap, TextWrapping::WrapWithOverflow, TextWrapping::NoWrap] {
        let properties = paragraph_properties(&run_properties(12.0), wrapping);
        let prepared = PreparedParagraph::new(TEXT, properties.clone());

        // Down and up again: a width is laid out after a wider and after a narrower one.
        let widths = (4..240).chain((4..240).rev()).map(|step| f64::from(step) * 2.25);
        for width in widths {
            let expected = fresh_layout(TEXT, &properties, width);
            assert_eq!(prepared.lines(width), lines_of(&expected), "{wrapping:?} at {width}");
            assert_eq!(prepared.line_count(width), expected.text_lines().len());
            assert_eq!(prepared.height(width), expected.height());
            expected.dispose();
        }
    }
}

#[test]
fn a_width_inside_the_hold_keeps_the_layout() {
    let _scope = TextTestScope::new();
    let properties = paragraph_properties(&run_properties(12.0), TextWrapping::Wrap);
    let prepared = PreparedParagraph::new("one two three four", properties);

    let first = prepared.layout(100.0);
    let (low, high) = first.width_hold().expect("plain text has a hold");
    let inside = (low + high) / 2.0;
    let second = prepared.layout(inside);
    assert!(Rc::ptr_eq(&first, &second));
    assert!(!Rc::ptr_eq(&first, &prepared.layout(high + 1.0)));
}

#[test]
fn an_alignment_that_moves_with_the_width_gets_a_layout_of_that_width() {
    let _scope = TextTestScope::new();
    let run = run_properties(12.0);
    let properties =
        paragraph_properties_with(&run, TextWrapping::Wrap, TextAlignment::Center, FlowDirection::LeftToRight);
    let prepared = PreparedParagraph::new("one two three four", properties);

    let first = prepared.layout(100.0);
    let (low, high) = first.width_hold().expect("plain text has a hold");
    let second = prepared.layout((low + high) / 2.0);
    assert!(!Rc::ptr_eq(&first, &second));
}

#[test]
fn a_layout_a_caller_holds_stays_valid_at_another_width() {
    let _scope = TextTestScope::new();
    let properties = paragraph_properties(&run_properties(12.0), TextWrapping::Wrap);
    let prepared = PreparedParagraph::new(TEXT, properties.clone());

    let held = prepared.layout(120.0);
    let expected = lines_of(&held);
    let extent = held.extent();
    let _other = prepared.layout(300.0);
    assert_eq!(lines_of(&held), expected);
    assert_eq!(held.extent(), extent);
}

#[test]
fn the_natural_width_is_the_width_without_a_constraint() {
    let _scope = TextTestScope::new();
    let properties = paragraph_properties(&run_properties(12.0), TextWrapping::Wrap);
    let prepared = PreparedParagraph::new(TEXT, properties.clone());
    let expected = fresh_layout(TEXT, &properties, f64::INFINITY);

    assert_eq!(prepared.natural_width(), expected.width());
    expected.dispose();
}

#[test]
fn a_shared_paragraph_is_kept_from_the_second_use() {
    let _scope = TextTestScope::new();
    let properties = paragraph_properties(&run_properties(12.0), TextWrapping::Wrap);
    let kept = || FontManager::current().prepared_paragraphs().len();

    let first = PreparedParagraph::shared("once and again", &properties);
    assert_eq!(kept(), 0);
    let second = PreparedParagraph::shared("once and again", &properties);
    assert_eq!(kept(), 1);
    assert!(!Rc::ptr_eq(&first, &second));
    assert!(Rc::ptr_eq(&second, &PreparedParagraph::shared("once and again", &properties)));

    let other_size = paragraph_properties(&run_properties(13.0), TextWrapping::Wrap);
    assert!(!Rc::ptr_eq(&second, &PreparedParagraph::shared("once and again", &other_size)));
}

#[test]
fn shared_paragraphs_nothing_holds_are_let_go_past_the_capacity() {
    let _scope = TextTestScope::new();
    let properties = paragraph_properties(&run_properties(12.0), TextWrapping::Wrap);
    let held = {
        PreparedParagraph::shared("held", &properties);
        PreparedParagraph::shared("held", &properties)
    };

    for index in 0..SHARED_CAPACITY + 64 {
        let text = format!("text {index}");
        PreparedParagraph::shared(&text, &properties);
        PreparedParagraph::shared(&text, &properties);
    }

    assert!(FontManager::current().prepared_paragraphs().len() <= SHARED_CAPACITY);
    assert!(Rc::ptr_eq(&held, &PreparedParagraph::shared("held", &properties)));
}

#[test]
fn a_very_long_text_is_not_shared() {
    let _scope = TextTestScope::new();
    let properties = paragraph_properties(&run_properties(12.0), TextWrapping::Wrap);
    let text = "a".repeat(SHARED_MAX_TEXT_LENGTH + 1);

    PreparedParagraph::shared(&text, &properties);
    PreparedParagraph::shared(&text, &properties);
    assert_eq!(FontManager::current().prepared_paragraphs().len(), 0);
}

#[test]
fn a_layout_makes_no_platform_glyph_run_until_its_ink_is_asked_for() {
    let _scope = TextTestScope::new();
    let properties = paragraph_properties(&run_properties(12.0), TextWrapping::Wrap);
    let layout = fresh_layout("one two three four five six", &properties, 40.0);

    assert!(layout.width() > 0.0 && layout.height() > 0.0);
    for line in layout.text_lines() {
        assert!(platform_runs(line).iter().all(Option::is_none));
    }

    assert!(layout.extent() > 0.0);
    for line in layout.text_lines() {
        assert!(platform_runs(line).iter().all(Option::is_some));
    }
}

#[test]
fn the_ink_of_a_justified_layout_is_the_ink_of_its_lines_as_laid_out() {
    let _scope = TextTestScope::new();
    let run = run_properties(12.0);
    let text = "one two three four five six seven eight nine ten";
    let with = |alignment| paragraph_properties_with(&run, TextWrapping::Wrap, alignment, FlowDirection::LeftToRight);
    let left = fresh_layout(text, &with(TextAlignment::Left), 80.0);
    let justified = fresh_layout(text, &with(TextAlignment::Justify), 80.0);

    assert_eq!(justified.extent(), left.extent());
    assert_eq!(justified.overhang_after(), left.overhang_after());
    assert_eq!(justified.overhang_leading(), left.overhang_leading());
    assert_eq!(justified.overhang_trailing(), left.overhang_trailing());
}

#[test]
fn the_ink_of_a_trimmed_layout_is_the_ink_of_its_lines_as_laid_out() {
    let _scope = TextTestScope::new();
    let properties = paragraph_properties(&run_properties(12.0), TextWrapping::Wrap);
    let text = "one two three four five six seven eight nine ten";
    let make = |trimming: Rc<dyn TextTrimming>| {
        let source: Rc<dyn ITextSource> =
            Rc::new(SingleBufferTextSource::new(text, properties.default_text_run_properties().clone()));
        TextLayout::from_text_source(source, properties.clone(), Some(trimming), 80.0, f64::INFINITY, 2, None)
    };
    let plain = make(<dyn TextTrimming>::none());
    let trimmed = make(<dyn TextTrimming>::character_ellipsis());

    assert!(trimmed.has_collapsed_lines());
    assert_eq!(trimmed.extent(), plain.extent());
    assert_eq!(trimmed.overhang_after(), plain.overhang_after());
    assert_eq!(trimmed.overhang_leading(), plain.overhang_leading());
    assert_eq!(trimmed.overhang_trailing(), plain.overhang_trailing());
}

#[test]
fn a_line_that_was_drawn_gives_its_glyph_run_to_the_next_layout() {
    let _scope = TextTestScope::new();
    let properties = paragraph_properties(&run_properties(12.0), TextWrapping::Wrap);
    let text = "word ".repeat(400);
    let source: Rc<dyn ITextSource> =
        Rc::new(SingleBufferTextSource::new(&text, properties.default_text_run_properties().clone()));
    let cache = Rc::new(TextRunCache::new());
    let make = || {
        TextLayout::from_text_source(
            source.clone(),
            properties.clone(),
            None,
            200.0,
            f64::INFINITY,
            0,
            Some(cache.clone()),
        )
    };

    // The test shaper does not record what an edit needs, so the shaping is kept by hand, as a
    // shaper that does has it kept when the text is shaped.
    let shaped = make();
    let buffer = shaped.text_lines()[0]
        .text_runs()
        .iter()
        .find_map(|run| run.downcast_ref::<ShapedTextRun>().map(|run| run.shaped_buffer().clone()))
        .expect("the line has a shaped run");
    let glyph_typeface = properties.default_text_run_properties().cached_glyph_typeface();
    cache.keep_shaping(&TextShaperOptions::with_all(glyph_typeface, 12.0, 0, None, 0.0, 0.0, None), &buffer);
    shaped.dispose();

    let first = make();
    // What drawing asks of a line.
    let drawn = first.text_lines()[0]
        .text_runs()
        .iter()
        .find_map(|run| run.downcast_ref::<ShapedTextRun>().map(|run| run.glyph_run().platform_impl()))
        .expect("the line has a shaped run");
    first.dispose();

    let second = make();
    let kept = platform_runs(&second.text_lines()[0]);
    assert!(kept.iter().flatten().any(|platform_impl| Arc::ptr_eq(platform_impl, &drawn)));
    second.dispose();
}

#[test]
fn a_disposed_line_does_not_measure_ink() {
    let _scope = TextTestScope::new();
    let properties = paragraph_properties(&run_properties(12.0), TextWrapping::Wrap);
    let layout = fresh_layout("one two three", &properties, 40.0);
    let line = layout.text_lines()[0].clone();

    layout.dispose();
    assert_eq!(line.extent(), 0.0);
    assert!(platform_runs(&line).iter().all(Option::is_none));
}
