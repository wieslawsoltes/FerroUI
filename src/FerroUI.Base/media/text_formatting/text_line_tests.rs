//! Port of upstream's text line tests onto the test harness (fixed advance
//! fonts: 6 per glyph at the default em size of 12; one glyph per codepoint).
//! Expectations that upstream takes from the metrics of a real font are
//! recomputed from the fixed advance; the comments say where.

use std::rc::Rc;

use crate::media::text_formatting::testing::{
    advance, format_line, line_height, line_text, paragraph_properties, paragraph_properties_with, run_properties,
    CustomDrawableRun, InvisibleRun, ListTextSource, RecordingDrawingSink, DrawCall, SingleBufferTextSource,
    TextTestScope, ASCENDER, DESCENDER, DESIGN_EM_HEIGHT, LINE_GAP,
};
use crate::media::text_formatting::{
    GenericTextParagraphProperties, ITextSource, InterWordJustification, ShapedTextRun, TextBounds, TextCharacters,
    TextEndOfParagraph, TextLine, TextLineImpl, TextMetrics, TextParagraphProperties, TextRun, TextRunProperties,
};
use crate::media::{
    CharacterHit, FlowDirection, TextAlignment, TextCollapsingCreateInfo, TextPathSegmentTrimming, TextTrimming,
    TextWrapping, Typeface,
};
use crate::{Point, Rect};

const MULTI_LINE_TEXT: &str = "012345678\r\r0123456789";

const EM: f64 = 12.0;

fn default_properties() -> Rc<dyn TextRunProperties> {
    run_properties(EM)
}

fn no_wrap(properties: &Rc<dyn TextRunProperties>) -> Rc<dyn TextParagraphProperties> {
    paragraph_properties(properties, TextWrapping::NoWrap)
}

fn format_text(text: &str) -> Rc<dyn TextLine> {
    let default_properties = default_properties();
    let text_source = SingleBufferTextSource::new(text, default_properties.clone());

    format_line(&text_source, 0, f64::INFINITY, &no_wrap(&default_properties), None).unwrap()
}

fn shaped(run: &Rc<dyn TextRun>) -> &ShapedTextRun {
    run.downcast_ref::<ShapedTextRun>().expect("a shaped run")
}

fn utf16_len(text: &str) -> i32 {
    text.encode_utf16().count() as i32
}

/// The position of the run's text in its buffer (upstream's `GetStartCharIndex`).
fn start_char_index(run: &ShapedTextRun) -> i32 {
    run.text().offset_in_owner() as i32
}

/// What has to be added to a glyph cluster of the run to get a text source
/// position. Upstream's helpers add the start of the run's text; runs that
/// were shaped together with their predecessors (same font, contiguous text —
/// more common with the single font of the harness) keep clusters relative to
/// the shaped text, so the first cluster is taken off.
fn cluster_offset(run: &ShapedTextRun) -> i32 {
    start_char_index(run) - run.glyph_run().metrics().first_cluster
}

fn is_right_to_left(text_line: &dyn TextLine) -> bool {
    text_line.text_runs().iter().any(|run| !shaped(run).shaped_buffer().is_left_to_right())
}

fn build_glyph_clusters(text_line: &dyn TextLine) -> Vec<i32> {
    let mut glyph_clusters = Vec::new();
    let mut last_cluster = -1;

    for run in text_line.text_runs().iter() {
        let text_run = shaped(run);
        let run_offset = cluster_offset(text_run);

        for glyph in text_run.shaped_buffer().glyph_infos().iter() {
            let current_cluster = glyph.glyph_cluster + run_offset;

            if last_cluster == current_cluster {
                continue;
            }

            glyph_clusters.push(current_cluster);
            last_cluster = current_cluster;
        }
    }

    glyph_clusters
}

fn build_rects(text_line: &dyn TextLine) -> Vec<Rect> {
    let mut rects: Vec<Rect> = Vec::new();
    let height = text_line.height();
    let mut current_x = 0.0;
    let mut last_cluster = -1;

    for run in text_line.text_runs().iter() {
        let text_run = shaped(run);
        let run_offset = cluster_offset(text_run);

        for glyph in text_run.shaped_buffer().glyph_infos().iter() {
            let current_cluster = glyph.glyph_cluster + run_offset;

            if last_cluster != current_cluster {
                rects.push(Rect::new(current_x, 0.0, glyph.glyph_advance, height));
            } else {
                // Another glyph of the cluster that produced the last rect: widen it.
                let rect = rects.last_mut().unwrap();
                *rect = rect.with_width(rect.width + glyph.glyph_advance);
            }

            current_x += glyph.glyph_advance;
            last_cluster = current_cluster;
        }
    }

    rects
}

/// The clusters of a line in logical order (runs ordered by their text position).
fn logical_clusters(text_line: &dyn TextLine) -> Vec<i32> {
    let runs = text_line.text_runs();

    let mut ordered: Vec<&ShapedTextRun> = runs.iter().map(shaped).collect();

    ordered.sort_by_key(|run| start_char_index(run));

    let mut clusters = Vec::new();

    for run in ordered {
        let run_offset = cluster_offset(run);

        let mut run_clusters: Vec<i32> =
            run.shaped_buffer().glyph_infos().iter().map(|glyph| glyph.glyph_cluster + run_offset).collect();

        if !run.shaped_buffer().is_left_to_right() {
            run_clusters.reverse();
        }

        clusters.extend(run_clusters);
    }

    clusters
}

#[test]
fn should_get_first_character_hit() {
    let _scope = TextTestScope::new();

    let default_properties = default_properties();
    let text_source = SingleBufferTextSource::new(MULTI_LINE_TEXT, default_properties.clone());

    let mut current_index = 0;

    while current_index < MULTI_LINE_TEXT.len() as i32 {
        let text_line =
            format_line(&text_source, current_index, f64::INFINITY, &no_wrap(&default_properties), None).unwrap();

        let first_character_hit = text_line.get_previous_caret_character_hit(CharacterHit::new(i32::MIN));

        assert_eq!(first_character_hit.first_character_index(), text_line.first_text_source_index());

        current_index += text_line.length();
    }
}

#[test]
fn should_get_last_character_hit() {
    let _scope = TextTestScope::new();

    let default_properties = default_properties();
    let text_source = SingleBufferTextSource::new(MULTI_LINE_TEXT, default_properties.clone());

    let mut current_index = 0;
    let mut line_count = 0;

    while current_index < MULTI_LINE_TEXT.len() as i32 {
        let text_line =
            format_line(&text_source, current_index, f64::INFINITY, &no_wrap(&default_properties), None).unwrap();

        let last_character_hit = text_line.get_next_caret_character_hit(CharacterHit::new(i32::MAX));

        assert_eq!(
            last_character_hit.first_character_index() + last_character_hit.trailing_length(),
            text_line.first_text_source_index() + text_line.length()
        );

        current_index += text_line.length();
        line_count += 1;
    }

    // Addition: the text has two hard breaks.
    assert_eq!(line_count, 3);
}

#[test]
fn should_get_next_caret_character_hit_bidi() {
    let _scope = TextTestScope::new();

    let text_line = format_text("\u{05D0}\u{05D1}\u{05D2} 1 ABC");

    let clusters = logical_clusters(&*text_line);

    assert_eq!(clusters, (0..9).collect::<Vec<_>>());

    let mut next_character_hit = CharacterHit::with_trailing_length(0, clusters[1] - clusters[0]);

    for cluster in &clusters {
        assert_eq!(next_character_hit.first_character_index(), *cluster);

        next_character_hit = text_line.get_next_caret_character_hit(next_character_hit);
    }

    let last_character_hit = next_character_hit;

    next_character_hit = text_line.get_next_caret_character_hit(last_character_hit);

    assert_eq!(next_character_hit, last_character_hit);
}

#[test]
fn should_get_previous_caret_character_hit_bidi() {
    let _scope = TextTestScope::new();

    let text = "\u{05D0}\u{05D1}\u{05D2} 1 ABC";

    let text_line = format_text(text);

    let mut clusters = logical_clusters(&*text_line);

    clusters.reverse();

    let mut next_character_hit = CharacterHit::new(utf16_len(text) - 1);

    for cluster in &clusters {
        let current_caret_index = next_character_hit.first_character_index() + next_character_hit.trailing_length();

        assert_eq!(current_caret_index, *cluster);

        next_character_hit = text_line.get_previous_caret_character_hit(next_character_hit);
    }

    let last_character_hit = next_character_hit;

    next_character_hit = text_line.get_previous_caret_character_hit(last_character_hit);

    assert_eq!(next_character_hit, last_character_hit);
}

/// U+10437 has no font in the harness (missing glyph), the party popper comes from the emoji fallback.
const CARET_TEXTS: [&str; 3] =
    ["\u{10437}\u{10437}\u{10437}\u{10437}\u{10437}", "01234567\u{1F389}\n", "\u{10437}1234"];

#[test]
fn should_get_next_caret_character_hit() {
    let _scope = TextTestScope::new();

    for text in CARET_TEXTS {
        let text_line = format_text(text);

        let clusters = build_glyph_clusters(&*text_line);

        let mut next_character_hit = CharacterHit::new(0);

        for expected_cluster in &clusters {
            let actual_cluster = next_character_hit.first_character_index() + next_character_hit.trailing_length();

            assert_eq!(actual_cluster, *expected_cluster, "{text:?}");

            next_character_hit = text_line.get_next_caret_character_hit(next_character_hit);
        }

        let mut last_character_hit = next_character_hit;

        next_character_hit = text_line.get_next_caret_character_hit(last_character_hit);

        assert_eq!(next_character_hit, last_character_hit);

        next_character_hit = CharacterHit::with_trailing_length(0, clusters[1] - clusters[0]);

        for cluster in &clusters {
            assert_eq!(next_character_hit.first_character_index(), *cluster, "{text:?}");

            next_character_hit = text_line.get_next_caret_character_hit(next_character_hit);
        }

        last_character_hit = next_character_hit;

        next_character_hit = text_line.get_next_caret_character_hit(last_character_hit);

        assert_eq!(next_character_hit, last_character_hit);
    }
}

#[test]
fn should_get_previous_caret_character_hit() {
    let _scope = TextTestScope::new();

    for text in CARET_TEXTS {
        let text_line = format_text(text);

        let clusters = build_glyph_clusters(&*text_line);

        let text_length = utf16_len(text);

        let mut previous_character_hit = CharacterHit::new(text_length);

        for cluster in clusters.iter().rev() {
            previous_character_hit = text_line.get_previous_caret_character_hit(previous_character_hit);

            assert_eq!(
                previous_character_hit.first_character_index() + previous_character_hit.trailing_length(),
                *cluster,
                "{text:?}"
            );
        }

        let first_character_hit = previous_character_hit;

        previous_character_hit = text_line.get_previous_caret_character_hit(first_character_hit);

        assert_eq!(previous_character_hit.first_character_index(), first_character_hit.first_character_index());
        assert_eq!(previous_character_hit.trailing_length(), 0);

        let last_cluster = clusters[clusters.len() - 1];

        previous_character_hit = CharacterHit::with_trailing_length(last_cluster, text_length - last_cluster);

        for cluster in clusters.iter().skip(1).rev() {
            previous_character_hit = text_line.get_previous_caret_character_hit(previous_character_hit);

            assert_eq!(
                previous_character_hit.first_character_index() + previous_character_hit.trailing_length(),
                *cluster,
                "{text:?}"
            );
        }
    }
}

#[test]
fn should_get_distance_from_character_hit() {
    let _scope = TextTestScope::new();

    let text_line = format_text(MULTI_LINE_TEXT);

    let mut current_distance = 0.0;

    for run in text_line.text_runs().iter() {
        let glyph_run = shaped(run).glyph_run();
        let list = glyph_run.glyph_infos();

        for glyph in list.borrow().iter() {
            let distance = text_line.get_distance_from_character_hit(CharacterHit::new(glyph.glyph_cluster));

            assert_eq!(distance, current_distance);

            current_distance += glyph.glyph_advance;
        }
    }

    let actual_distance =
        text_line.get_distance_from_character_hit(CharacterHit::new(MULTI_LINE_TEXT.len() as i32));

    assert_eq!(actual_distance, current_distance);

    // Addition: the first line is nine digits and a line break without advance.
    assert_eq!(current_distance, 9.0 * advance(EM));
    assert_eq!(text_line.length(), 10);
    assert_eq!(text_line.new_line_length(), 1);
}

#[test]
fn should_get_character_hit_from_distance() {
    let _scope = TextTestScope::new();

    for text in [
        "ABC012345", // LeftToRight
        "\u{05D6}\u{05D4} \u{05DB}\u{05D9}\u{05E3} \u{05E1}\u{05EA}\u{05DD} \u{05DC}\u{05E9}\u{05DE}\u{05D5}\u{05E2}", // RightToLeft
    ] {
        let text_line = format_text(text);

        let is_right_to_left = is_right_to_left(&*text_line);
        let rects = build_rects(&*text_line);
        let glyph_clusters = build_glyph_clusters(&*text_line);

        for (cluster, rect) in glyph_clusters.iter().zip(&rects) {
            let character_hit = text_line.get_character_hit_from_distance(rect.left());

            assert_eq!(
                character_hit.first_character_index() + character_hit.trailing_length(),
                if is_right_to_left { cluster + 1 } else { *cluster },
                "{text:?}"
            );
        }
    }
}

#[test]
fn should_collapse_line() {
    let _scope = TextTestScope::new();

    // Recomputed for the fixed advance (6 per glyph, the ellipsis included):
    // * prefix: upstream collapses a wider line at 120; here the line is 102 wide, so 96 (sixteen
    //   glyphs) is used, which keeps the prefix of eight, the ellipsis and seven trailing characters;
    // * 58 leaves 52 next to the ellipsis: eight characters, or the first word.
    for (text, width, trimming, expected) in [
        ("01234 01234 01234", 96.0, <dyn TextTrimming>::prefix_character_ellipsis(), "01234 01\u{2026}4 01234"),
        ("01234 01234", 58.0, <dyn TextTrimming>::character_ellipsis(), "01234 01\u{2026}"),
        ("01234 01234", 58.0, <dyn TextTrimming>::word_ellipsis(), "01234\u{2026}"),
        ("01234", 9.0, <dyn TextTrimming>::character_ellipsis(), "\u{2026}"),
        ("01234", 2.0, <dyn TextTrimming>::character_ellipsis(), ""),
    ] {
        let default_properties = default_properties();

        let text_line = format_text(text);

        assert!(!text_line.has_collapsed());

        let collapsing_properties = trimming.create_collapsing_properties(&TextCollapsingCreateInfo::new(
            width,
            default_properties.clone(),
            FlowDirection::LeftToRight,
        ));

        let collapsed_line = text_line.clone().collapse(&[Some(collapsing_properties)]);

        assert!(collapsed_line.has_collapsed());
        assert_eq!(line_text(&*collapsed_line), expected, "{text:?} at {width} with {trimming}");
    }
}

/// Addition: a line that fits is not collapsed and is returned as it is; so
/// is a line without collapsing properties.
#[test]
fn collapse_returns_the_line_itself_when_nothing_changes() {
    let _scope = TextTestScope::new();

    let default_properties = default_properties();

    let text_line = format_text("01234");

    for trimming in [
        <dyn TextTrimming>::character_ellipsis(),
        <dyn TextTrimming>::word_ellipsis(),
        <dyn TextTrimming>::prefix_character_ellipsis(),
        <dyn TextTrimming>::leading_character_ellipsis(),
        <dyn TextTrimming>::path_segment_ellipsis(),
    ] {
        let collapsing_properties = trimming.create_collapsing_properties(&TextCollapsingCreateInfo::new(
            100.0,
            default_properties.clone(),
            FlowDirection::LeftToRight,
        ));

        let collapsed_line = text_line.clone().collapse(&[Some(collapsing_properties)]);

        assert!(Rc::ptr_eq(&collapsed_line, &text_line), "{trimming}");
    }

    assert!(Rc::ptr_eq(&text_line.clone().collapse(&[]), &text_line));
    assert!(Rc::ptr_eq(&text_line.clone().collapse(&[None]), &text_line));
}

/// A drawable run, "_A_A", a drawable run, "_A_A".
struct DrawableRunTextSource {
    properties: Rc<dyn TextRunProperties>,
}

impl ITextSource for DrawableRunTextSource {
    fn get_text_run(&self, text_source_index: i32) -> Option<Rc<dyn TextRun>> {
        match text_source_index {
            0 | 5 => Some(Rc::new(CustomDrawableRun::new(self.properties.clone()))),
            1 | 6 => Some(Rc::new(TextCharacters::from_str("_A_A", self.properties.clone()))),
            _ => None,
        }
    }
}

fn format_drawable_runs() -> Rc<dyn TextLine> {
    let default_properties = default_properties();
    let text_source = DrawableRunTextSource { properties: default_properties.clone() };

    format_line(&text_source, 0, f64::INFINITY, &no_wrap(&default_properties), None).unwrap()
}

#[test]
fn should_get_next_character_hit_for_drawable_runs() {
    let _scope = TextTestScope::new();

    let text_line = format_drawable_runs();

    assert_eq!(text_line.text_runs().len(), 4);

    let mut current_hit = CharacterHit::new(0);

    for expected in 1..=4 {
        current_hit = text_line.get_next_caret_character_hit(current_hit);

        assert_eq!(current_hit.first_character_index(), expected);
        assert_eq!(current_hit.trailing_length(), 0);
    }
}

#[test]
fn should_get_previous_character_hit_for_drawable_runs() {
    let _scope = TextTestScope::new();

    let text_line = format_drawable_runs();

    assert_eq!(text_line.text_runs().len(), 4);

    let mut current_hit = CharacterHit::with_trailing_length(3, 1);

    for expected in (0..=3).rev() {
        current_hit = text_line.get_previous_caret_character_hit(current_hit);

        assert_eq!(current_hit.first_character_index(), expected);
        assert_eq!(current_hit.trailing_length(), 0);
    }
}

#[test]
fn should_get_character_hit_from_distance_for_drawable_runs() {
    let _scope = TextTestScope::new();

    let text_line = format_drawable_runs();

    // The second drawable run spans 38..52 (14 + four glyphs of 6): 50 is in its trailing half.
    let character_hit = text_line.get_character_hit_from_distance(50.0);

    assert_eq!(character_hit.first_character_index(), 5);
    assert_eq!(character_hit.trailing_length(), 1);

    // Upstream probes 32 with its font; here the third character of the text (position 3)
    // spans 26..32, so 27 is in its leading half.
    let character_hit = text_line.get_character_hit_from_distance(27.0);

    assert_eq!(character_hit.first_character_index(), 3);
    assert_eq!(character_hit.trailing_length(), 0);
}

#[test]
fn should_get_distance_from_character_hit_drawable_runs() {
    let _scope = TextTestScope::new();

    let text_line = format_drawable_runs();

    assert_eq!(text_line.get_distance_from_character_hit(CharacterHit::new(1)), 14.0);

    // Upstream: greater than 14. With the fixed advance: one glyph further.
    assert_eq!(text_line.get_distance_from_character_hit(CharacterHit::new(2)), 14.0 + advance(EM));
}

/// Four runs of ten characters, each in a buffer and with properties of its own.
struct MixedTextBufferTextSource;

impl ITextSource for MixedTextBufferTextSource {
    fn get_text_run(&self, text_source_index: i32) -> Option<Rc<dyn TextRun>> {
        let text = match text_source_index {
            0 => "aaaaaaaaaa",
            10 => "bbbbbbbbbb",
            20 => "cccccccccc",
            30 => "dddddddddd",
            _ => return None,
        };

        Some(Rc::new(TextCharacters::from_str(text, default_properties())))
    }
}

fn format_mixed(first_text_source_index: i32) -> Rc<dyn TextLine> {
    let default_properties = default_properties();

    format_line(&MixedTextBufferTextSource, first_text_source_index, f64::INFINITY, &no_wrap(&default_properties), None)
        .unwrap()
}

#[test]
fn should_get_distance_from_character_hit_mixed_text_buffer() {
    let _scope = TextTestScope::new();

    let text_line = format_mixed(0);

    assert_eq!(text_line.text_runs().len(), 4);

    // Upstream: multiples of the width of ten glyphs of its font. Here ten glyphs are 60 wide.
    assert_eq!(text_line.get_distance_from_character_hit(CharacterHit::new(10)), 60.0);
    assert_eq!(text_line.get_distance_from_character_hit(CharacterHit::new(20)), 120.0);
    assert_eq!(text_line.get_distance_from_character_hit(CharacterHit::new(30)), 180.0);
    assert_eq!(
        text_line.get_distance_from_character_hit(CharacterHit::new(40)),
        text_line.width_including_trailing_whitespace()
    );
}

fn sum_of_widths(text_bounds: &[TextBounds]) -> f64 {
    text_bounds.iter().map(|bounds| bounds.rectangle().width).sum()
}

#[test]
fn should_get_text_bounds_from_mixed_text_buffer() {
    let _scope = TextTestScope::new();

    let text_line = format_mixed(0);

    // Upstream: multiples of the width of ten glyphs of its font. Here ten glyphs are 60 wide.
    for (length, width) in [(10, 60.0), (20, 120.0), (30, 180.0), (40, text_line.width_including_trailing_whitespace())] {
        let text_bounds = text_line.get_text_bounds(0, length);

        assert_eq!(text_bounds.len(), 1);
        assert_eq!(sum_of_widths(&text_bounds), width);
    }
}

#[test]
fn should_get_text_bounds_for_line_break() {
    let _scope = TextTestScope::new();

    for new_line in ["\r\n", "\n"] {
        let text_line = format_text(new_line);

        let text_bounds = text_line.get_text_bounds(0, new_line.len() as i32);

        assert_eq!(text_bounds.len(), 1);
        assert_eq!(text_bounds[0].text_run_bounds().len(), 1);
        assert_eq!(text_bounds[0].text_run_bounds()[0].length(), new_line.len() as i32);
    }
}

#[test]
fn should_get_text_range() {
    let _scope = TextTestScope::new();

    let text = "\u{05E9}\u{05D3}\u{05D2}\u{05DB}\u{05DB}\u{05E2}A\u{05D9}\u{05E9}\u{05D3}\u{05D2}YDASYW\u{05D7}\u{05D9}\u{05D7}SA\u{05D8}\u{05D5}HUH\u{05D0}\u{05D0}'\u{05E7}'/\u{05E7}";

    let text_line = format_text(text);

    let line_width = text_line.width_including_trailing_whitespace();

    let text_bounds = text_line.get_text_bounds(0, utf16_len(text));

    let runs = text_line.text_runs();

    let run_bounds: Vec<_> = text_bounds.iter().flat_map(|bounds| bounds.text_run_bounds().iter()).collect();

    assert_eq!(run_bounds.len(), runs.len());

    for (run, bounds) in runs.iter().zip(&run_bounds) {
        let shaped_run = shaped(run);

        assert_eq!(bounds.text_source_character_index(), start_char_index(shaped_run));
        assert!(std::ptr::addr_eq(Rc::as_ptr(run), Rc::as_ptr(bounds.text_run())));
        assert_eq!(bounds.rectangle().width, shaped_run.glyph_run().bounds().width);
    }

    let mut last_right: Option<f64> = None;

    for current_bounds in &text_bounds {
        if let Some(last_right) = last_right {
            assert_eq!(current_bounds.rectangle().left(), last_right);
        }

        let sum_of_run_width: f64 =
            current_bounds.text_run_bounds().iter().map(|run_bounds| run_bounds.rectangle().width).sum();

        assert_eq!(current_bounds.rectangle().width, sum_of_run_width);

        last_right = Some(current_bounds.rectangle().right());
    }

    assert_eq!(sum_of_widths(&text_bounds), line_width);
}

#[test]
fn should_get_character_hit_for_distance_with_text_end_of_line() {
    let _scope = TextTestScope::new();

    let default_properties = default_properties();
    let text_source = SingleBufferTextSource::with_end_of_paragraph("Hello World", default_properties.clone(), true);

    let text_line = format_line(&text_source, 0, 1000.0, &no_wrap(&default_properties), None).unwrap();

    let character_hit = text_line.get_character_hit_from_distance(1000.0);

    assert_eq!(character_hit.first_character_index(), 10);
    assert_eq!(character_hit.trailing_length(), 1);
}

#[test]
fn should_get_next_caret_character_hit_from_mixed_text_buffer() {
    let _scope = TextTestScope::new();

    let text_line = format_mixed(0);

    let mut character_hit = text_line.get_next_caret_character_hit(CharacterHit::with_trailing_length(9, 1));

    assert_eq!(character_hit, CharacterHit::with_trailing_length(10, 1));

    character_hit = text_line.get_next_caret_character_hit(character_hit);

    assert_eq!(character_hit, CharacterHit::with_trailing_length(11, 1));

    character_hit = text_line.get_next_caret_character_hit(CharacterHit::with_trailing_length(19, 1));

    assert_eq!(character_hit, CharacterHit::with_trailing_length(20, 1));

    character_hit = text_line.get_next_caret_character_hit(CharacterHit::new(10));

    assert_eq!(character_hit, CharacterHit::new(11));

    character_hit = text_line.get_next_caret_character_hit(character_hit);

    assert_eq!(character_hit, CharacterHit::new(12));

    character_hit = text_line.get_next_caret_character_hit(CharacterHit::new(20));

    assert_eq!(character_hit, CharacterHit::new(21));
}

#[test]
fn should_get_previous_caret_character_hit_from_mixed_text_buffer() {
    let _scope = TextTestScope::new();

    let text_line = format_mixed(0);

    let mut character_hit = text_line.get_previous_caret_character_hit(CharacterHit::with_trailing_length(20, 1));

    assert_eq!(character_hit, CharacterHit::new(20));

    character_hit = text_line.get_previous_caret_character_hit(CharacterHit::with_trailing_length(10, 1));

    assert_eq!(character_hit, CharacterHit::new(10));

    character_hit = text_line.get_previous_caret_character_hit(character_hit);

    assert_eq!(character_hit, CharacterHit::new(9));

    character_hit = text_line.get_previous_caret_character_hit(CharacterHit::new(21));

    assert_eq!(character_hit, CharacterHit::new(20));

    character_hit = text_line.get_previous_caret_character_hit(CharacterHit::new(11));

    assert_eq!(character_hit, CharacterHit::new(10));

    character_hit = text_line.get_previous_caret_character_hit(character_hit);

    assert_eq!(character_hit, CharacterHit::new(9));
}

#[test]
fn should_get_character_hit_from_distance_from_mixed_text_buffer() {
    let _scope = TextTestScope::new();

    let text_line = format_mixed(20);

    let character_hit = text_line.get_character_hit_from_distance(f64::INFINITY);

    assert_eq!(character_hit.first_character_index() + character_hit.trailing_length(), 40);
}

fn format_runs(runs: Vec<Rc<dyn TextRun>>) -> Rc<dyn TextLine> {
    let default_properties = default_properties();

    format_line(&ListTextSource::new(runs), 0, f64::INFINITY, &no_wrap(&default_properties), None).unwrap()
}

fn characters(text: &str) -> Rc<dyn TextRun> {
    Rc::new(TextCharacters::from_str(text, default_properties()))
}

fn hidden(length: i32) -> Rc<dyn TextRun> {
    Rc::new(InvisibleRun::new(length))
}

#[test]
#[should_panic(expected = "textLength ('0') must be a non-zero value.")]
fn should_throw_argument_out_of_range_exception_for_zero_text_length() {
    let _scope = TextTestScope::new();

    let text_line = format_runs(vec![characters("1234")]);

    text_line.get_text_bounds(0, 0);
}

#[test]
fn should_get_text_bounds_for_negative_text_length() {
    let _scope = TextTestScope::new();

    let text_line = format_runs(vec![characters("1234")]);

    let text_bounds = text_line.get_text_bounds(0, -1);

    assert!(!text_bounds.is_empty());

    let first_bounds = &text_bounds[0];

    assert!(first_bounds.text_run_bounds().is_empty());
    assert_eq!(first_bounds.rectangle().width, 0.0);
    assert_eq!(first_bounds.rectangle().left(), 0.0);
}

#[test]
fn should_get_text_bounds_for_exceeding_text_length() {
    let _scope = TextTestScope::new();

    let text_line = format_runs(vec![characters("1234")]);

    let text_bounds = text_line.get_text_bounds(10, 1);

    assert!(!text_bounds.is_empty());

    let first_bounds = &text_bounds[0];

    assert!(first_bounds.text_run_bounds().is_empty());
    assert_eq!(first_bounds.rectangle().width, 0.0);
    assert_eq!(first_bounds.rectangle().right(), text_line.width_including_trailing_whitespace());
}

#[test]
fn should_get_text_bounds_for_mixed_hidden_runs_with_ligature() {
    let scope = TextTestScope::new();

    // Upstream's font has an "ff" ligature; the shaper of the harness is told to form one.
    scope.shaper().add_ligature("ff");

    let text_line = format_runs(vec![
        hidden(1),
        characters("Authenti"),
        hidden(1),
        hidden(1),
        characters("ff"),
        hidden(1),
        hidden(1),
    ]);

    let text_bounds = text_line.get_text_bounds(12, 1);

    assert!(!text_bounds.is_empty());

    let first_bounds = &text_bounds[0];

    assert!(!first_bounds.text_run_bounds().is_empty());
    assert_eq!(first_bounds.text_run_bounds()[0].text_source_character_index(), 12);
}

#[test]
fn should_get_text_bounds_for_mixed_hidden_runs() {
    let _scope = TextTestScope::new();

    let text_line = format_runs(vec![
        hidden(1),
        characters("Authenti"),
        hidden(1),
        hidden(1),
        Rc::new(TextEndOfParagraph::with_length(1)),
    ]);

    let text_bounds = text_line.get_text_bounds(8, 1);

    assert!(!text_bounds.is_empty());
    assert!(!text_bounds[0].text_run_bounds().is_empty());
}

#[test]
fn should_handle_new_line_in_rtl_text() {
    let _scope = TextTestScope::new();

    let default_properties = default_properties();
    let paragraph_properties = paragraph_properties_with(
        &default_properties,
        TextWrapping::Wrap,
        TextAlignment::Right,
        FlowDirection::RightToLeft,
    );

    for text in [
        "test\r\n",
        "hello\r\nworld",
        "\u{0645}\u{0631}\u{062D}\u{0628}\u{0627}\r\n\u{0628}\u{0627}\u{0644}",
        "hello \u{0645}\u{0631}\u{062D}\u{0628}\u{0627}\r\nworld",
        "\u{0645}\u{0631}\u{062D}\u{0628}\u{0627} hello\r\nworld",
    ] {
        let text_source = SingleBufferTextSource::new(text, default_properties.clone());

        let text_line = format_line(&text_source, 0, f64::INFINITY, &paragraph_properties, None).unwrap();

        assert_eq!(text_line.new_line_length(), 2, "{text:?}");
    }
}

#[test]
fn should_get_in_cluster_backspace_hit() {
    let scope = TextTestScope::new();

    scope.shaper().add_ligature("ff");

    let text_line = format_text("ff");

    let backspace_hit = text_line.get_backspace_caret_character_hit(CharacterHit::with_trailing_length(1, 1));

    assert_eq!(backspace_hit.first_character_index(), 1);
}

#[test]
fn should_get_text_bounds_bidi_left_to_right() {
    let _scope = TextTestScope::new();

    let default_properties = default_properties();

    let text = "\u{05D0}\u{05D0}\u{05D0} AAA";

    let text_source = SingleBufferTextSource::new(text, default_properties.clone());

    let text_line = format_line(&text_source, 0, 200.0, &no_wrap(&default_properties), None).unwrap();

    let runs = text_line.text_runs();

    let first_run_width = shaped(&runs[0]).glyph_run().bounds().width;
    let second_run_width = shaped(&runs[1]).glyph_run().bounds().width;

    let text_bounds = text_line.get_text_bounds(0, 3);

    assert_eq!(text_bounds.len(), 1);
    assert_eq!(sum_of_widths(&text_bounds), first_run_width);

    let text_bounds = text_line.get_text_bounds(3, 4);

    assert_eq!(text_bounds.len(), 1);
    assert_eq!(sum_of_widths(&text_bounds), second_run_width);

    let text_bounds = text_line.get_text_bounds(0, 4);

    assert_eq!(text_bounds.len(), 2);
    assert_eq!(text_bounds[0].rectangle().width, first_run_width);
    // Upstream: the width of a space of its font.
    assert_eq!(text_bounds[1].rectangle().width, advance(EM));
    assert_eq!(text_bounds[1].rectangle().left(), first_run_width);

    let text_bounds = text_line.get_text_bounds(0, utf16_len(text));

    assert_eq!(text_bounds.len(), 2);
    assert_eq!(sum_of_widths(&text_bounds), text_line.width_including_trailing_whitespace());
}

#[test]
fn should_get_text_bounds_bidi_right_to_left() {
    let _scope = TextTestScope::new();

    let default_properties = default_properties();

    let text = "\u{05D0}\u{05D0}\u{05D0} AAA";

    let text_source = SingleBufferTextSource::new(text, default_properties.clone());

    let paragraph_properties = paragraph_properties_with(
        &default_properties,
        TextWrapping::NoWrap,
        TextAlignment::Left,
        FlowDirection::RightToLeft,
    );

    let text_line = format_line(&text_source, 0, 200.0, &paragraph_properties, None).unwrap();

    // Runs come in visual order: the Latin word sits leftmost, then the Hebrew word with its
    // space. (Upstream's space is a run of its own because its Hebrew comes from a fallback
    // font; the default font of the harness covers Hebrew.)
    let runs = text_line.text_runs();

    assert_eq!(runs.len(), 2);

    let latin_width = shaped(&runs[0]).glyph_run().bounds().width;
    let hebrew_and_space_width = shaped(&runs[1]).glyph_run().bounds().width;

    assert_eq!(shaped(&runs[0]).text().to_string_lossy(), "AAA");

    let run_lengths = |text_bounds: &[TextBounds]| -> i32 {
        text_bounds.iter().flat_map(|bounds| bounds.text_run_bounds().iter()).map(|bounds| bounds.length()).sum()
    };

    let text_bounds = text_line.get_text_bounds(0, 4);

    assert_eq!(text_bounds.len(), 1);
    assert_eq!(sum_of_widths(&text_bounds), hebrew_and_space_width);

    let text_bounds = text_line.get_text_bounds(4, 3);

    assert_eq!(text_bounds.len(), 1);
    assert_eq!(run_lengths(&text_bounds), 3);
    assert_eq!(sum_of_widths(&text_bounds), latin_width);

    let text_bounds = text_line.get_text_bounds(0, 5);

    assert_eq!(text_bounds.len(), 2);
    assert_eq!(run_lengths(&text_bounds), 5);

    assert_eq!(text_bounds[1].rectangle().width, hebrew_and_space_width);
    // Upstream: the width of one glyph of its font.
    assert_eq!(text_bounds[0].rectangle().width, advance(EM));
    assert_eq!(text_bounds[0].rectangle().right(), text_line.start() + advance(EM));
    assert_eq!(text_bounds[1].rectangle().left(), text_line.start() + latin_width);

    let text_bounds = text_line.get_text_bounds(0, utf16_len(text));

    assert_eq!(text_bounds.len(), 2);
    assert_eq!(run_lengths(&text_bounds), 7);
    assert_eq!(sum_of_widths(&text_bounds), text_line.width_including_trailing_whitespace());
}

#[test]
fn should_get_text_bounds_with_end_of_paragraph() {
    let _scope = TextTestScope::new();

    let default_properties = default_properties();

    for (text, first_index) in [("abc", 3), ("\u{0644}\u{0648}\u{062D}\u{0629} \u{0627}\u{0644}", 0)] {
        let text_source = SingleBufferTextSource::with_end_of_paragraph(text, default_properties.clone(), true);

        let text_line = format_line(&text_source, 0, f64::INFINITY, &no_wrap(&default_properties), None).unwrap();

        let text_bounds = text_line.get_text_bounds(first_index, 1);

        assert_eq!(text_bounds.len(), 1);
        assert!(!text_bounds[0].text_run_bounds().is_empty());
    }
}

#[test]
fn should_ignore_null_terminator() {
    let _scope = TextTestScope::new();

    // NUL characters keep their position but have no width (they shape as word joiners).
    for (text, glyphs) in [("\0", 0.0), ("a\0b", 2.0), ("\0\0ab", 2.0)] {
        let text_line = format_text(text);

        assert_eq!(text_line.length(), utf16_len(text));
        assert_eq!(text_line.width_including_trailing_whitespace(), glyphs * advance(EM), "{text:?}");
    }
}

#[test]
fn should_add_half_line_gap_to_baseline() {
    let _scope = TextTestScope::new();

    let text_line = format_text("F");

    let text_metrics = TextMetrics::new(&Typeface::default_typeface().glyph_typeface(), EM);

    let expected_baseline = -text_metrics.ascent + text_metrics.line_gap / 2.0;

    assert_eq!(text_line.baseline(), expected_baseline);

    // Addition: the values for the metrics of the harness font.
    let scale = EM / DESIGN_EM_HEIGHT as f64;

    assert_eq!(text_line.baseline(), ASCENDER as f64 * scale + LINE_GAP as f64 * scale / 2.0);
    assert_eq!(text_line.height(), line_height(EM));
    assert_eq!(text_line.extent(), (ASCENDER - DESCENDER) as f64 * scale);
}

fn format_with_line_height(text: &str, line_height: f64) -> Rc<dyn TextLine> {
    let default_properties = default_properties();
    let text_source = SingleBufferTextSource::new(text, default_properties.clone());

    let paragraph_properties: Rc<dyn TextParagraphProperties> = Rc::new(GenericTextParagraphProperties::with_options(
        default_properties,
        TextAlignment::Left,
        TextWrapping::NoWrap,
        line_height,
        0.0,
    ));

    format_line(&text_source, 0, f64::INFINITY, &paragraph_properties, None).unwrap()
}

#[test]
fn should_clamp_baseline_when_line_height_is_smaller_than_natural() {
    let _scope = TextTestScope::new();

    let text_metrics = TextMetrics::new(&Typeface::default_typeface().glyph_typeface(), EM);

    let natural = -text_metrics.ascent + text_metrics.descent + text_metrics.line_gap;
    let smaller_line_height = natural - 2.0;

    // Force a smaller line height than ascent+descent+lineGap
    let text_line = format_with_line_height("F", smaller_line_height);

    // In this case, baseline should equal -Ascent (lineGap ignored)
    assert_eq!(text_line.baseline(), -text_metrics.ascent);
    assert_eq!(text_line.height(), smaller_line_height);
}

#[test]
fn should_distribute_extra_space_when_line_height_is_larger_than_natural() {
    let _scope = TextTestScope::new();

    let text_metrics = TextMetrics::new(&Typeface::default_typeface().glyph_typeface(), EM);

    let natural = -text_metrics.ascent + text_metrics.descent + text_metrics.line_gap;
    let larger_line_height = natural + 50.0;

    let text_line = format_with_line_height("F", larger_line_height);

    // Extra space is distributed evenly above and below
    let extra = larger_line_height - (text_metrics.descent - text_metrics.ascent);
    let expected_baseline = -text_metrics.ascent + extra / 2.0;

    assert!((text_line.baseline() - expected_baseline).abs() < 0.00001);
    assert!((text_line.height() - larger_line_height).abs() < 0.00001);
}

#[test]
fn backspace_should_treat_crlf_as_a_unit() {
    let _scope = TextTestScope::new();

    let text_line = format_text("one\r\n");

    let backspace_hit = text_line.get_backspace_caret_character_hit(CharacterHit::new(5));

    assert_eq!(backspace_hit.first_character_index(), 3);
}

/// Only the text of shaped runs.
fn extract_text_from_runs(text_line: &dyn TextLine) -> String {
    text_line
        .text_runs()
        .iter()
        .filter_map(|run| run.downcast_ref::<ShapedTextRun>())
        .map(|run| run.text().to_string_lossy())
        .collect()
}

fn collapse_path(text: &str, width: f64) -> String {
    let default_properties = default_properties();

    let text_line = format_text(text);

    let trimming = TextPathSegmentTrimming::new("*");

    let collapsing_properties = trimming.create_collapsing_properties(&TextCollapsingCreateInfo::new(
        width,
        default_properties,
        FlowDirection::LeftToRight,
    ));

    let collapsed_line = text_line.collapse(&[Some(collapsing_properties)]);

    extract_text_from_runs(&*collapsed_line)
}

#[test]
fn should_collapse_with_text_path_segment_trimming_without_path_segment() {
    let _scope = TextTestScope::new();

    // 15 leaves 9 next to the symbol: one glyph.
    assert_eq!(collapse_path("foo", 15.0), "*o");
}

#[test]
fn should_collapse_with_text_path_segment_trimming_no_space() {
    let _scope = TextTestScope::new();

    assert_eq!(collapse_path("foo", 8.0), "*");
}

#[test]
fn truncate_path_path_ending_with_slash_returns_non_empty() {
    let _scope = TextTestScope::new();

    for path in ["somedirectory\\", "somedirectory/"] {
        let result = collapse_path(path, 50.0);

        assert!(result.contains("ory"), "{result:?}");

        // Addition: 50 leaves 44 next to the symbol: seven glyphs.
        assert_eq!(result, format!("*{}", &path[path.len() - 7..]));
    }
}

#[test]
fn should_collapse_with_ellipsis() {
    let _scope = TextTestScope::new();

    for path in ["directory\\file.txt", "directory/file.txt"] {
        assert_eq!(collapse_path(path, 8.0), "*");
    }
}

#[test]
fn should_trim_path_at_the_end() {
    let _scope = TextTestScope::new();

    // Upstream: "*.txt" with its font. 40 leaves 34 next to the symbol: five glyphs of 6.
    assert_eq!(collapse_path("verylongdirectory\\file.txt", 40.0), "*e.txt");
}

/// Addition: a middle segment of a path is replaced when that makes the line fit.
#[test]
fn path_segment_trimming_collapses_the_middle_segment() {
    let _scope = TextTestScope::new();

    // 22 glyphs (132). Without "bbbbbbbb" and with the symbol: 15 glyphs (90).
    assert_eq!(collapse_path("aaa/bbbbbbbb/cccc.txt", 96.0), "aaa/*/cccc.txt");
}

/// Addition: drawing a line draws every run at its position on the baseline.
#[test]
fn draw_positions_runs_along_the_line() {
    let _scope = TextTestScope::new();

    let default_properties: Rc<dyn TextRunProperties> = Rc::new(
        crate::media::text_formatting::GenericTextRunProperties::with_all(
            Typeface::default_typeface(),
            EM,
            None,
            Some(crate::media::Brushes::black()),
            None,
            crate::media::BaselineAlignment::Baseline,
            None,
            None,
        ),
    );

    let text_source = SingleBufferTextSource::new("ab \u{05D0}\u{05D1}", default_properties.clone());

    let paragraph_properties = paragraph_properties_with(
        &default_properties,
        TextWrapping::NoWrap,
        TextAlignment::Right,
        FlowDirection::LeftToRight,
    );

    let text_line = format_line(&text_source, 0, 100.0, &paragraph_properties, None).unwrap();

    assert_eq!(text_line.start(), 70.0);

    let mut sink = RecordingDrawingSink::new();

    text_line.draw(&mut sink, Point::new(10.0, 20.0));

    // The baseline offset of a run on its own baseline is zero here: the line's baseline
    // and the run's baseline are the same distance (ascent plus half the line gap).
    assert_eq!(
        sink.calls(),
        [
            DrawCall::GlyphRun { text: "ab ".to_owned(), origin: Point::new(80.0, 20.0) },
            DrawCall::GlyphRun { text: "\u{05D0}\u{05D1}".to_owned(), origin: Point::new(98.0, 20.0) },
        ]
    );
    assert_eq!(sink.transform_depth(), 0);
}

/// Addition: the bounds of a line and its ink bounds follow the metrics.
#[test]
fn bounds_and_ink_bounds_follow_the_metrics() {
    let _scope = TextTestScope::new();

    let default_properties = default_properties();
    let text_source = SingleBufferTextSource::new("abc ", default_properties.clone());

    let paragraph_properties = paragraph_properties_with(
        &default_properties,
        TextWrapping::NoWrap,
        TextAlignment::Center,
        FlowDirection::LeftToRight,
    );

    let text_line = format_line(&text_source, 0, 100.0, &paragraph_properties, None).unwrap();

    assert_eq!(text_line.width(), 18.0);
    assert_eq!(text_line.width_including_trailing_whitespace(), 24.0);
    assert_eq!(text_line.trailing_whitespace_length(), 1);
    assert_eq!(text_line.start(), 41.0);
    assert!(!text_line.has_overflowed());

    let line_impl = text_line.as_any().downcast_ref::<TextLineImpl>().unwrap();

    assert_eq!(line_impl.bounds(), Rect::new(41.0, 0.0, 24.0, line_height(EM)));

    let scale = EM / DESIGN_EM_HEIGHT as f64;

    // The ink of the test glyph runs spans their advances from the ascent to the descent;
    // the line aligns it at the common baseline, which puts its top at the top of the line.
    let ink_bounds = line_impl.ink_bounds();

    assert_eq!(ink_bounds.left(), 41.0);
    assert_eq!(ink_bounds.width, 24.0);
    assert!(ink_bounds.top().abs() < 1e-9);
    assert!((ink_bounds.height - (ASCENDER - DESCENDER) as f64 * scale).abs() < 1e-9);

    assert_eq!(text_line.overhang_leading(), 0.0);
    assert_eq!(text_line.overhang_trailing(), 0.0);
}

/// Addition: justification widens the gaps at the break opportunities until
/// the visible content fills the paragraph width.
#[test]
fn inter_word_justification_fills_the_paragraph_width() {
    let _scope = TextTestScope::new();

    let default_properties = default_properties();
    let text_source = SingleBufferTextSource::new("aa bb cc dd", default_properties.clone());

    let paragraph_properties = paragraph_properties_with(
        &default_properties,
        TextWrapping::Wrap,
        TextAlignment::Justify,
        FlowDirection::LeftToRight,
    );

    // 60 fits ten glyphs: "aa bb cc " (trailing space included), 48 without the trailing space.
    let text_line = format_line(&text_source, 0, 60.0, &paragraph_properties, None).unwrap();

    assert_eq!(line_text(&*text_line), "aa bb cc ");
    assert_eq!(text_line.width(), 48.0);

    let shaped_before = text_line.text_runs()[0].clone();

    text_line.justify(&InterWordJustification::new(60.0));

    // Two break opportunities inside the visible content share the remaining 12.
    assert!((text_line.width() - 60.0).abs() < 1e-9);
    assert!((text_line.width_including_trailing_whitespace() - 66.0).abs() < 1e-9);

    let runs = text_line.text_runs();

    assert_eq!(runs.len(), 1);

    let advances: Vec<f64> =
        shaped(&runs[0]).shaped_buffer().glyph_infos().iter().map(|glyph| glyph.glyph_advance).collect();

    assert_eq!(advances, [6.0, 6.0, 12.0, 6.0, 6.0, 12.0, 6.0, 6.0, 6.0]);

    // The run was replaced by a private copy; the original was released.
    assert!(!std::ptr::addr_eq(Rc::as_ptr(&runs[0]), Rc::as_ptr(&shaped_before)));
    assert_eq!(shaped(&shaped_before).shaped_buffer().length(), 0);

    // Hit testing follows the justified advances.
    assert_eq!(text_line.get_distance_from_character_hit(CharacterHit::new(3)), 24.0);
}
