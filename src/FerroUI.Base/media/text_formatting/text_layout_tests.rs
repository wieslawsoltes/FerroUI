//! Port of upstream's text layout tests onto the test harness (fixed advance
//! fonts: 6 per glyph at the default em size of 12, 12 per emoji; a line
//! height of 13.2; one glyph per codepoint). Expectations that upstream takes
//! from the metrics of a real font are recomputed from the fixed advance; the
//! comments say where. Also holds the layout based tests of upstream's run
//! cache tests and empty shaped buffer tests.

use std::rc::Rc;

use crate::media::text_formatting::testing::{
    advance, line_height, run_properties, SingleBufferTextSource, TestFont, TextTestScope, CJK_FAMILY,
    DEFAULT_FAMILY, EMOJI_FAMILY, GLYPH_ADVANCE,
};
use crate::media::text_formatting::unicode::{Codepoint, GraphemeEnumerator};
use crate::media::text_formatting::{
    DrawableTextRun, GenericTextParagraphProperties, GenericTextRunProperties, ITextSource, InterWordJustification, ShapedTextRun,
    TextCharacters, TextFormatter, TextFormatterImpl, TextLayout, TextLayoutOptions, TextLine,
    TextParagraphProperties, TextRun, TextRunCache, TextRunProperties,
};
use crate::media::{
    BaselineAlignment, Brushes, CharacterHit, FlowDirection, IBrush, TextAlignment, TextTrimming, TextWrapping,
    Typeface,
};
use crate::utilities::{ReadOnlyMemory, ValueSpan};
use crate::Point;

const SINGLE_LINE_TEXT: &str = "0123456789";
const MULTI_LINE_TEXT: &str = "01 23 45 678\r\rabc def gh ij";
const RIGHT_TO_LEFT_TEXT: &str = "זה כיף סתם לשמוע איך תנצח קרפד עץ טוב בגן";

const EM: f64 = 12.0;

fn black() -> Option<Rc<dyn IBrush>> {
    let brush: Rc<dyn IBrush> = Brushes::black();

    Some(brush)
}

fn red() -> Rc<dyn IBrush> {
    Brushes::red()
}

/// The options of `new TextLayout(text, Typeface.Default, 12, Brushes.Black)`.
fn options() -> TextLayoutOptions {
    TextLayoutOptions { font_size: EM, foreground: black(), ..Default::default() }
}

fn layout(text: &str, options: TextLayoutOptions) -> TextLayout {
    TextLayout::new(text, Typeface::default_typeface(), options)
}

/// `new GenericTextRunProperties(Typeface.Default, 12, foregroundBrush: foreground)`.
fn styled(foreground: &Rc<dyn IBrush>) -> Rc<dyn TextRunProperties> {
    Rc::new(GenericTextRunProperties::with_all(
        Typeface::default_typeface(),
        EM,
        None,
        Some(foreground.clone()),
        None,
        BaselineAlignment::Baseline,
        None,
        None,
    ))
}

fn overrides(start: i32, length: i32, foreground: &Rc<dyn IBrush>) -> Option<Rc<[ValueSpan<Rc<dyn TextRunProperties>>]>> {
    Some(Rc::from(vec![ValueSpan::new(start, length, styled(foreground))]))
}

fn shaped(run: &Rc<dyn TextRun>) -> &ShapedTextRun {
    run.downcast_ref::<ShapedTextRun>().expect("a shaped run")
}

fn utf16_len(text: &str) -> i32 {
    text.encode_utf16().count() as i32
}

/// `text.Substring(start, length)` in UTF-16 code units.
fn substring(text: &str, start: i32, length: i32) -> String {
    let units: Vec<u16> = text.encode_utf16().collect();

    String::from_utf16_lossy(&units[start as usize..(start + length) as usize])
}

fn has_foreground(run: &Rc<dyn TextRun>, foreground: &Rc<dyn IBrush>) -> bool {
    run.properties().and_then(|properties| properties.foreground_brush()).is_some_and(|brush| Rc::ptr_eq(brush, foreground))
}

/// The position of the run's text in its buffer (upstream's `GetStartCharIndex`).
fn start_char_index(run: &ShapedTextRun) -> i32 {
    run.text().offset_in_owner() as i32
}

/// The text source positions of the glyphs of a run. A run's clusters are
/// relative to the text it was shaped from; the smallest cluster is the run's
/// first character, so rebasing on it maps them onto text source indices.
fn text_positions(run: &ShapedTextRun) -> Vec<i32> {
    let clusters: Vec<i32> = run.shaped_buffer().glyph_infos().iter().map(|glyph| glyph.glyph_cluster).collect();

    let base_cluster = clusters.iter().copied().min().unwrap_or(0);

    clusters.iter().map(|cluster| cluster - base_cluster + start_char_index(run)).collect()
}

fn glyph_advances(line: &dyn TextLine) -> Vec<f64> {
    let mut advances = Vec::new();

    for run in line.text_runs().iter() {
        if let Some(shaped) = run.downcast_ref::<ShapedTextRun>() {
            for glyph in shaped.glyph_run().glyph_infos().borrow().iter() {
                advances.push(glyph.glyph_advance);
            }
        }
    }

    advances
}

fn last_glyph_advance(line: &dyn TextLine) -> f64 {
    let runs = line.text_runs();

    let last = runs.iter().filter_map(|run| run.downcast_ref::<ShapedTextRun>()).last().unwrap();

    let glyphs = last.glyph_run().glyph_infos();

    let glyphs = glyphs.borrow();

    glyphs[glyphs.len() - 1].glyph_advance
}

fn shaped_runs(line: &dyn TextLine) -> Vec<Rc<ShapedTextRun>> {
    line.text_runs().iter().filter_map(|run| run.clone().downcast_rc::<ShapedTextRun>()).collect()
}

#[track_caller]
fn assert_close(expected: f64, actual: f64, precision: i32) {
    let tolerance = 0.5 * 10f64.powi(-precision);

    assert!((expected - actual).abs() <= tolerance, "expected {expected}, actual {actual}");
}

#[track_caller]
fn assert_greater_than(x: f64, y: f64, message: &str) {
    assert!(x > y, "{message}. {x} is not > {y}");
}

/// `new GenericTextParagraphProperties(defaultProperties)`.
fn default_paragraph(default_properties: &Rc<dyn TextRunProperties>) -> Rc<dyn TextParagraphProperties> {
    Rc::new(GenericTextParagraphProperties::new(default_properties.clone()))
}

fn format_single_line(text_source: &dyn ITextSource, default_properties: &Rc<dyn TextRunProperties>) -> Rc<dyn TextLine> {
    TextFormatterImpl::new()
        .format_line(text_source, 0, f64::INFINITY, &default_paragraph(default_properties), None)
        .expect("a line")
}

/// A text source of one text whose first `split_at` characters have other
/// properties than the rest.
struct SplitStyleTextSource {
    text: ReadOnlyMemory<u16>,
    split_at: i32,
    first: Rc<dyn TextRunProperties>,
    second: Rc<dyn TextRunProperties>,
}

impl SplitStyleTextSource {
    fn new(text: &str, split_at: i32, first: Rc<dyn TextRunProperties>, second: Rc<dyn TextRunProperties>) -> Self {
        Self { text: ReadOnlyMemory::<u16>::from_str(text), split_at, first, second }
    }
}

impl ITextSource for SplitStyleTextSource {
    fn get_text_run(&self, text_source_index: i32) -> Option<Rc<dyn TextRun>> {
        if text_source_index >= self.text.len() as i32 {
            return None;
        }

        if text_source_index < self.split_at {
            return Some(Rc::new(TextCharacters::new(
                self.text.slice(text_source_index as usize, (self.split_at - text_source_index) as usize),
                self.first.clone(),
            )));
        }

        Some(Rc::new(TextCharacters::new(self.text.slice_from(text_source_index as usize), self.second.clone())))
    }
}

#[test]
fn should_break_lines() {
    for (text, number_of_lines) in [("01234\r01234\r", 3), ("01234\r01234", 2)] {
        let _scope = TextTestScope::new();

        let layout = layout(text, options());

        assert_eq!(layout.text_lines().len(), number_of_lines);
    }
}

#[test]
fn should_apply_text_style_span_to_text_in_between() {
    let _scope = TextTestScope::new();

    let foreground = red();

    let layout = layout(
        MULTI_LINE_TEXT,
        TextLayoutOptions { text_style_overrides: overrides(1, 2, &foreground), ..options() },
    );

    let text_line = &layout.text_lines()[0];

    let text_runs = text_line.text_runs();

    assert_eq!(text_runs.len(), 3);

    let text_run = &text_runs[1];

    assert_eq!(text_run.length(), 2);

    let actual = text_run.text().to_string_lossy();

    assert_eq!(actual, "1 ");

    assert!(text_run.properties().is_some());
    assert!(has_foreground(text_run, &foreground));
}

#[test]
fn should_wrap_and_apply_style() {
    for length in [27, 22] {
        let _scope = TextTestScope::new();

        let text = "Multiline TextBox with TextWrapping.";

        let foreground = red();

        let expected = layout(
            text,
            TextLayoutOptions { text_wrapping: TextWrapping::Wrap, max_width: 200.0, ..options() },
        );

        let expected_lines: Vec<String> = expected
            .text_lines()
            .iter()
            .map(|line| substring(text, line.first_text_source_index(), line.length()))
            .collect();

        let actual = layout(
            text,
            TextLayoutOptions {
                text_wrapping: TextWrapping::Wrap,
                max_width: 200.0,
                text_style_overrides: overrides(0, length, &foreground),
                ..options()
            },
        );

        let actual_lines: Vec<String> = actual
            .text_lines()
            .iter()
            .map(|line| substring(text, line.first_text_source_index(), line.length()))
            .collect();

        assert_eq!(actual_lines.len(), expected_lines.len());

        for j in 0..actual.text_lines().len() {
            assert_eq!(actual_lines[j], expected_lines[j]);
        }
    }
}

#[test]
fn should_not_alter_lines_after_text_style_span_was_applied() {
    let _scope = TextTestScope::new();

    const TEXT: &str = "אחד !\ntwo !\nשְׁלוֹשָׁה !";

    fn get_glyphs(text_layout: &TextLayout) -> Vec<String> {
        text_layout
            .text_lines()
            .iter()
            .map(|line| {
                line.text_runs()
                    .iter()
                    .flat_map(|run| {
                        shaped(run)
                            .shaped_buffer()
                            .glyph_infos()
                            .iter()
                            .map(|glyph| glyph.glyph_index.to_string())
                            .collect::<Vec<_>>()
                    })
                    .collect::<Vec<_>>()
                    .join("|")
            })
            .collect()
    }

    let red = red();

    let expected = layout(TEXT, TextLayoutOptions { text_wrapping: TextWrapping::Wrap, ..options() });

    let expected_glyphs = get_glyphs(&expected);

    let text: Vec<u16> = TEXT.encode_utf16().collect();

    let mut outer = GraphemeEnumerator::new(&text);
    let mut inner = GraphemeEnumerator::new(&text);
    let mut i = 0;
    let mut j = 0;

    loop {
        while let Some(grapheme) = inner.move_next() {
            j += grapheme.length() as i32;

            if j + i > text.len() as i32 {
                break;
            }

            let actual = layout(
                TEXT,
                TextLayoutOptions {
                    text_wrapping: TextWrapping::Wrap,
                    text_style_overrides: overrides(i, j, &red),
                    ..options()
                },
            );

            let actual_glyphs = get_glyphs(&actual);

            assert_eq!(actual_glyphs.len(), expected_glyphs.len());

            for k in 0..expected_glyphs.len() {
                assert_eq!(actual_glyphs[k], expected_glyphs[k], "span ({i}, {j}), line {k}");
            }
        }

        let Some(grapheme) = outer.move_next() else {
            break;
        };

        inner = GraphemeEnumerator::new(&text);

        i += grapheme.length() as i32;
    }
}

#[test]
fn should_apply_text_style_span_to_text_at_start() {
    let _scope = TextTestScope::new();

    let foreground = red();

    let layout = layout(
        SINGLE_LINE_TEXT,
        TextLayoutOptions { text_style_overrides: overrides(0, 2, &foreground), ..options() },
    );

    let text_line = &layout.text_lines()[0];

    let text_runs = text_line.text_runs();

    assert_eq!(text_runs.len(), 2);

    let text_run = &text_runs[0];

    assert_eq!(text_run.length(), 2);

    let actual = &SINGLE_LINE_TEXT[..text_run.length() as usize];

    assert_eq!(actual, "01");

    assert!(text_run.properties().is_some());
    assert!(has_foreground(text_run, &foreground));
}

#[test]
fn should_apply_text_style_span_to_text_at_end() {
    let _scope = TextTestScope::new();

    let foreground = red();

    let layout = layout(
        SINGLE_LINE_TEXT,
        TextLayoutOptions { text_style_overrides: overrides(8, 2, &foreground), ..options() },
    );

    let text_line = &layout.text_lines()[0];

    let text_runs = text_line.text_runs();

    assert_eq!(text_runs.len(), 2);

    let text_run = &text_runs[1];

    assert_eq!(text_run.length(), 2);

    let actual = text_run.text().to_string_lossy();

    assert_eq!(actual, "89");

    assert!(text_run.properties().is_some());
    assert!(has_foreground(text_run, &foreground));
}

#[test]
fn should_apply_text_style_span_to_single_character() {
    let _scope = TextTestScope::new();

    let foreground = red();

    let layout = layout("0", TextLayoutOptions { text_style_overrides: overrides(0, 1, &foreground), ..options() });

    let text_line = &layout.text_lines()[0];

    let text_runs = text_line.text_runs();

    assert_eq!(text_runs.len(), 1);

    let text_run = &text_runs[0];

    assert_eq!(text_run.length(), 1);

    assert!(text_run.properties().is_some());
    assert!(has_foreground(text_run, &foreground));
}

#[test]
fn should_apply_text_span_to_unicode_string_in_between() {
    let _scope = TextTestScope::new();

    const TEXT: &str = "😄😄😄😄";

    let foreground = red();

    let layout = layout(TEXT, TextLayoutOptions { text_style_overrides: overrides(2, 2, &foreground), ..options() });

    let text_line = &layout.text_lines()[0];

    let text_runs = text_line.text_runs();

    assert_eq!(text_runs.len(), 3);

    let text_run = &text_runs[1];

    assert_eq!(text_run.length(), 2);

    let actual = text_run.text().to_string_lossy();

    assert_eq!(actual, "😄");

    assert!(text_run.properties().is_some());
    assert!(has_foreground(text_run, &foreground));
}

#[test]
fn text_length_should_be_equal_to_text_line_length_sum() {
    let _scope = TextTestScope::new();

    let layout = layout(MULTI_LINE_TEXT, options());

    assert_eq!(layout.text_lines().iter().map(|line| line.length()).sum::<i32>(), utf16_len(MULTI_LINE_TEXT));
}

#[test]
fn text_length_should_be_equal_to_text_run_text_length_sum() {
    let _scope = TextTestScope::new();

    let layout = layout(MULTI_LINE_TEXT, options());

    assert_eq!(
        layout
            .text_lines()
            .iter()
            .map(|text_line| text_line.text_runs().iter().map(|text_run| text_run.length()).sum::<i32>())
            .sum::<i32>(),
        utf16_len(MULTI_LINE_TEXT)
    );
}

#[test]
fn text_length_should_be_equal_to_text_run_text_length_sum_after_wrap_with_style_applied() {
    let _scope = TextTestScope::new();

    const TEXT: &str = "Multiline TextBox with TextWrapping.\r\rLorem ipsum dolor sit amet";

    let foreground = red();

    let layout = layout(
        TEXT,
        TextLayoutOptions {
            text_wrapping: TextWrapping::Wrap,
            max_width: 180.0,
            text_style_overrides: overrides(0, 24, &foreground),
            ..options()
        },
    );

    assert_eq!(
        layout
            .text_lines()
            .iter()
            .map(|text_line| text_line.text_runs().iter().map(|text_run| text_run.length()).sum::<i32>())
            .sum::<i32>(),
        utf16_len(TEXT)
    );
}

#[test]
fn should_apply_text_style_span_to_multi_line() {
    let _scope = TextTestScope::new();

    let foreground = red();

    let layout = layout(
        MULTI_LINE_TEXT,
        TextLayoutOptions {
            max_width: 200.0,
            max_height: 125.0,
            text_style_overrides: overrides(5, 20, &foreground),
            ..options()
        },
    );

    assert!(has_foreground(&layout.text_lines()[0].text_runs()[1], &foreground));
    assert!(has_foreground(&layout.text_lines()[1].text_runs()[0], &foreground));
    assert!(has_foreground(&layout.text_lines()[2].text_runs()[0], &foreground));
}

#[test]
fn should_hit_test_surrogate_pair() {
    let _scope = TextTestScope::new();

    const TEXT: &str = "😄😄";

    let layout = layout(TEXT, options());

    let text_runs = layout.text_lines()[0].text_runs();

    let shaped_run = shaped(&text_runs[0]);

    let glyph_run = shaped_run.glyph_run();

    let width = glyph_run.bounds().width;

    let (character_hit, _) = glyph_run.get_character_hit_from_distance(width);

    assert_eq!(character_hit.first_character_index(), 2);

    assert_eq!(character_hit.trailing_length(), 2);
}

/// Upstream's emoji font shapes the emoji modifier sequence to one glyph; the
/// harness gets the same from a font that covers U+261D and a ligature.
#[test]
fn should_create_valid_clusters_for_text() {
    let cases: [(&str, &[i32]); 3] = [("☝🏿", &[0]), ("☝🏿 ab", &[0, 0, 1, 2]), ("ab ☝🏿", &[0, 1, 2, 0])];

    for (text, clusters) in cases {
        let scope = TextTestScope::with_fonts(vec![
            TestFont::new(DEFAULT_FAMILY).with_ranges(&[(0x20, 0x7E)]),
            TestFont::new(EMOJI_FAMILY)
                .with_ranges(&[(0x2600, 0x27BF), (0x1F300, 0x1FAFF)])
                .with_advance(GLYPH_ADVANCE * 2),
        ]);

        scope.shaper().add_ligature("☝🏿");

        let layout = layout(text, options());

        let text_line = &layout.text_lines()[0];

        let mut index = 0;

        for text_run in text_line.text_runs().iter() {
            let shaped_run = shaped(text_run);

            let glyph_clusters: Vec<i32> =
                shaped_run.shaped_buffer().glyph_infos().iter().map(|glyph| glyph.glyph_cluster).collect();

            let expected: Vec<i32> = clusters.iter().copied().skip(index).take(glyph_clusters.len()).collect();

            assert_eq!(glyph_clusters, expected, "{text:?}");

            index += glyph_clusters.len();
        }

        assert_eq!(index, clusters.len(), "{text:?}");
    }
}

#[test]
fn should_break_with_break_char() {
    let cases = [
        ("abcde\r\n", 7),     // Carriage Return + Line Feed
        ("abcde\u{000A}", 6), // Line Feed
        ("abcde\u{000B}", 6), // Vertical Tab
        ("abcde\u{000C}", 6), // Form Feed
        ("abcde\u{000D}", 6), // Carriage Return
    ];

    for (text, expected_length) in cases {
        let _scope = TextTestScope::new();

        let layout = layout(text, options());

        assert_eq!(layout.text_lines().len(), 2);

        let text_runs = layout.text_lines()[0].text_runs();

        assert_eq!(text_runs.len(), 1);

        let run = shaped(&text_runs[0]);

        assert_eq!(run.glyph_run().glyph_infos().count(), expected_length);

        assert_eq!(run.shaped_buffer().get(5).glyph_cluster, 5);

        if expected_length == 7 {
            assert_eq!(run.shaped_buffer().get(6).glyph_cluster, 5);
        }
    }
}

#[test]
fn should_have_one_run_with_common_script() {
    let _scope = TextTestScope::new();

    let layout = layout("abcde\r\n", options());

    assert_eq!(layout.text_lines()[0].text_runs().len(), 1);
}

#[test]
fn should_layout_corrupted_text() {
    let _scope = TextTestScope::new();

    // Seven lone high surrogates (not expressible as a `&str`).
    let text = ReadOnlyMemory::from_vec(vec![0xD802u16; 7]);

    let layout = TextLayout::from_utf16(text, Typeface::default_typeface(), options());

    let text_line = &layout.text_lines()[0];

    let text_runs = text_line.text_runs();

    let text_run = shaped(&text_runs[0]);

    assert_eq!(text_run.length(), 7);

    let replacement_glyph = Typeface::default_typeface()
        .glyph_typeface()
        .character_to_glyph_map()
        .get_glyph(i32::from(Codepoint::REPLACEMENT_CODEPOINT));

    for glyph_info in text_run.glyph_run().glyph_infos().borrow().iter() {
        assert_eq!(glyph_info.glyph_index, replacement_glyph);
    }
}

#[test]
fn should_include_first_line_when_constraint_is_surpassed() {
    for text in ["0123456789\r0123456789", "0123456789"] {
        let _scope = TextTestScope::new();

        // Upstream computes `LineSpacing * (12.0 / DesignEmHeight)` from the metrics of its
        // font; with the harness metrics that product is one ulp off the height the line
        // computes (`LineSpacing * 12.0 / DesignEmHeight`), so the harness value is used.
        let line_height = line_height(EM);

        let layout = layout(text, TextLayoutOptions { max_height: line_height - line_height * 0.5, ..options() });

        assert_eq!(layout.text_lines().len(), 1);

        assert_eq!(layout.height(), line_height);
    }
}

#[test]
fn should_not_exceed_max_lines() {
    let cases = [
        ("0123456789\r\n0123456789\r\n0123456789", 0, 3),
        ("0123456789\r\n0123456789\r\n0123456789", 1, 1),
        ("0123456789\r\n0123456789\r\n0123456789", 4, 3),
    ];

    for (text, max_lines, expected_lines) in cases {
        let _scope = TextTestScope::new();

        let layout = layout(text, TextLayoutOptions { max_width: 50.0, max_lines, ..options() });

        assert_eq!(layout.text_lines().len(), expected_lines);
    }
}

#[test]
fn should_add_ellipsis_when_max_lines_cuts_short_wrapped_line() {
    let _scope = TextTestScope::new();

    for trimming in [<dyn TextTrimming>::character_ellipsis(), <dyn TextTrimming>::word_ellipsis()] {
        const TEXT: &str = "Lorem ipsum dolor sit amet, consectetur adipiscing elit, sed do eiusmod tempor incididunt ut labore et dolore magna aliqua. Ut enim ad minim veniam, quis nostrud exercitation ullamco laboris nisi ut aliquip ex ea commodo consequat. Duis aute irure dolor in reprehenderit in voluptate velit esse cillum dolore eu fugiat nulla pariatur. Excepteur sint occaecat cupidatat non proident, sunt in culpa qui officia deserunt mollit anim id est laborum.";
        const MAX_LINES: i32 = 2;

        let layout = layout(
            TEXT,
            TextLayoutOptions {
                text_wrapping: TextWrapping::Wrap,
                text_trimming: Some(trimming),
                max_width: 180.0,
                max_lines: MAX_LINES,
                ..options()
            },
        );

        assert_eq!(layout.text_lines().len() as i32, MAX_LINES);

        let last_line = &layout.text_lines()[layout.text_lines().len() - 1];
        let formatted_line_text: String =
            last_line.text_runs().iter().map(|run| run.text().to_string_lossy()).collect();

        assert!(last_line.has_collapsed(), "The wrapped line cut off by MaxLines must be collapsed.");
        assert_eq!(formatted_line_text.chars().last(), Some('\u{2026}'));

        layout.dispose();
    }
}

#[test]
fn should_produce_fixed_height_lines() {
    let _scope = TextTestScope::new();

    let layout = layout(MULTI_LINE_TEXT, TextLayoutOptions { line_height: 50.0, ..options() });

    for line in layout.text_lines() {
        assert_eq!(line.height(), 50.0);
    }
}

#[test]
fn should_process_multiple_new_lines_properly() {
    let _scope = TextTestScope::new();

    let text = "123\r\n\r\n456\r\n\r\n";

    let layout = layout(text, options());

    assert_eq!(layout.text_lines().len(), 5);

    let first_run_text = |line: usize| layout.text_lines()[line].text_runs()[0].text().to_string_lossy();

    assert_eq!(first_run_text(0), "123\r\n");
    assert_eq!(first_run_text(1), "\r\n");
    assert_eq!(first_run_text(2), "456\r\n");
    assert_eq!(first_run_text(3), "\r\n");
}

#[test]
fn should_wrap_min_one_character_every_line() {
    let _scope = TextTestScope::new();

    let layout = layout(
        SINGLE_LINE_TEXT,
        TextLayoutOptions { text_wrapping: TextWrapping::Wrap, max_width: 3.0, ..options() },
    );

    // every character should be new line as there not enough space for even one character
    assert_eq!(layout.text_lines().len(), SINGLE_LINE_TEXT.len());
}

#[test]
fn should_hit_test_text_range_right_to_left() {
    let _scope = TextTestScope::new();

    const START: i32 = 0;
    const LENGTH: i32 = 10;

    let text_layout = layout(RIGHT_TO_LEFT_TEXT, options());

    let selected_text = layout(&substring(RIGHT_TO_LEFT_TEXT, START, LENGTH), options());

    let rects = text_layout.hit_test_text_range(START, LENGTH);

    assert_eq!(rects.len(), 1);

    let selected_rect = rects[0];

    assert_close(selected_text.width_including_trailing_whitespace(), selected_rect.width, 2);
}

#[test]
fn should_hit_test_text_range_bidi() {
    const TEXT: &str = "זה כיףabcDEFזה כיף";

    let _scope = TextTestScope::new();

    let layout = layout(TEXT, options());

    let text_line = &layout.text_lines()[0];

    let start = text_line.get_distance_from_character_hit(CharacterHit::with_trailing_length(5, 1));

    let end = text_line.get_distance_from_character_hit(CharacterHit::with_trailing_length(6, 1));

    let rects = layout.hit_test_text_range(0, 7);

    assert_eq!(rects.len(), 1);

    let expected = rects[0];

    assert_eq!(expected.left(), start);
    assert_eq!(expected.right(), end);
}

#[test]
fn should_hit_test_text_range() {
    let _scope = TextTestScope::new();

    let layout = layout(SINGLE_LINE_TEXT, options());

    let line_rects = layout.hit_test_text_range(0, SINGLE_LINE_TEXT.len() as i32);

    assert_eq!(line_rects.len(), layout.text_lines().len());

    for (i, text_line) in layout.text_lines().iter().enumerate() {
        let rect = line_rects[i];

        assert_eq!(rect.width, text_line.width_including_trailing_whitespace());
    }

    let rects: Vec<f64> = layout.text_lines().iter().flat_map(|line| glyph_advances(&**line)).collect();

    for i in 0..SINGLE_LINE_TEXT.len() {
        let mut j = 1;

        while i + j < SINGLE_LINE_TEXT.len() {
            let expected: f64 = rects[i..i + j].iter().sum();
            let actual: f64 = layout.hit_test_text_range(i as i32, j as i32).iter().map(|rect| rect.width).sum();

            assert_eq!(actual, expected);

            j += 1;
        }
    }
}

#[test]
fn should_wrap_right_to_left() {
    const TEXT: &str =
        "يَجِبُ عَلَى الإنْسَانِ أن يَكُونَ أمِيْنَاً وَصَادِقَاً مَعَ نَفْسِهِ وَمَعَ أَهْلِهِ وَجِيْرَانِهِ وَأَنْ يَبْذُلَ كُلَّ جُهْدٍ فِي إِعْلاءِ شَأْنِ الوَطَنِ وَأَنْ يَعْمَلَ عَلَى مَا يَجْلِبُ السَّعَادَةَ لِلنَّاسِ . ولَن يَتِمَّ لَهُ ذلِك إِلا بِأَنْ يُقَدِّمَ المَنْفَعَةَ العَامَّةَ عَلَى المَنْفَعَةِ الخَاصَّةِ وَهذَا مِثَالٌ لِلتَّضْحِيَةِ .";

    let _scope = TextTestScope::new();

    let mut max_width = 366;

    while max_width < 900 {
        let layout = layout(
            TEXT,
            TextLayoutOptions {
                text_wrapping: TextWrapping::Wrap,
                flow_direction: FlowDirection::RightToLeft,
                max_width: max_width as f64,
                ..options()
            },
        );

        for text_line in layout.text_lines() {
            assert!(text_line.width() <= max_width as f64);

            let mut runs = shaped_runs(&**text_line);

            runs.sort_by_key(|run| start_char_index(run));

            let actual: String = runs.iter().map(|run| run.text().to_string_lossy()).collect();

            let expected = substring(TEXT, text_line.first_text_source_index(), text_line.length());

            assert_eq!(actual, expected);
        }

        max_width += 33;
    }
}

#[test]
fn should_layout_empty_string() {
    let _scope = TextTestScope::new();

    let layout = layout("", options());

    assert!(layout.height() > 0.0);
}

#[test]
fn should_hit_test_point_right_to_left() {
    let _scope = TextTestScope::new();

    let text = "אאא AAA";

    let layout = layout(text, TextLayoutOptions { flow_direction: FlowDirection::RightToLeft, ..options() });

    let text_runs = layout.text_lines()[0].text_runs();

    let first_run = shaped(&text_runs[0]);

    let mut hit = layout.hit_test_point(Point::default());

    assert_eq!(hit.text_position(), 4);

    let first_run_positions = text_positions(first_run);
    let first_run_advances: Vec<f64> =
        first_run.glyph_run().glyph_infos().borrow().iter().map(|glyph| glyph.glyph_advance).collect();
    let mut current_x = 0.0;

    for i in 0..first_run_positions.len() {
        let cluster = first_run_positions[i];
        let advance = first_run_advances[i];

        hit = layout.hit_test_point(Point::new(current_x, 0.0));

        assert_eq!(hit.text_position(), cluster);

        let hit_range = layout.hit_test_text_range(hit.text_position(), 1);

        let distance = hit_range[0].left();

        assert_close(current_x, distance, 2);

        current_x += advance;
    }

    let second_run = shaped(&text_runs[1]);

    hit = layout.hit_test_point(Point::new(first_run.size().width, 0.0));

    assert_eq!(hit.text_position(), 7);

    hit = layout.hit_test_point(Point::new(layout.text_lines()[0].width_including_trailing_whitespace(), 0.0));

    assert_eq!(hit.text_position(), 0);

    let second_run_positions = text_positions(second_run);
    let second_run_advances: Vec<f64> =
        second_run.glyph_run().glyph_infos().borrow().iter().map(|glyph| glyph.glyph_advance).collect();
    current_x = first_run.size().width + 0.5;

    for i in 0..second_run_positions.len() {
        let cluster = second_run_positions[i];
        let advance = second_run_advances[i];

        hit = layout.hit_test_point(Point::new(current_x, 0.0));

        assert_eq!(hit.character_hit().first_character_index(), cluster);

        let hit_range = layout
            .hit_test_text_range(hit.character_hit().first_character_index(), hit.character_hit().trailing_length());

        let distance = hit_range[0].left() + 0.5;

        assert_close(current_x, distance, 2);

        current_x += advance;
    }
}

#[test]
fn should_get_character_hit_from_distance_rtl() {
    let _scope = TextTestScope::new();

    let text = "أَبْجَدِيَّة عَرَبِيَّة";

    let layout = layout(text, options());

    let text_line = &layout.text_lines()[0];

    // Runs come in visual order, so the first run is the leftmost one. Its glyph clusters
    // are relative to the text it was shaped from, so they are rebased onto text source
    // indices.
    let text_runs = text_line.text_runs();

    let first_run = shaped(&text_runs[0]);

    let first_cluster = text_positions(first_run)[0];

    let character_hit = text_line.get_character_hit_from_distance(0.0);

    assert_eq!(character_hit.first_character_index(), first_cluster);

    assert_eq!(character_hit.first_character_index() + character_hit.trailing_length(), utf16_len(text));

    let distance = text_line.get_distance_from_character_hit(character_hit);

    assert_eq!(distance, 0.0);

    let distance =
        text_line.get_distance_from_character_hit(CharacterHit::new(character_hit.first_character_index()));

    // The first glyph with an advance (the harness shapes the marks of the
    // last cluster to glyphs of their own, without advance, in front of it).
    let first_advance = first_run
        .shaped_buffer()
        .glyph_infos()
        .iter()
        .map(|glyph| glyph.glyph_advance)
        .find(|advance| *advance > 0.0)
        .unwrap();

    assert_close(first_advance, distance, 5);

    let rect = layout.hit_test_text_position(22);

    assert_close(first_advance, rect.left(), 5);

    let rect = layout.hit_test_text_position(23);

    assert_close(0.0, rect.left(), 5);
}

#[test]
fn should_get_character_hit_from_distance_rtl_with_text_styles() {
    let _scope = TextTestScope::new();

    let text = "أَبْجَدِيَّة عَرَبِيَّة";

    let units: Vec<u16> = text.encode_utf16().collect();

    let mut i = 0;

    let mut grapheme_enumerator = GraphemeEnumerator::new(&units);

    let red = red();

    while let Some(grapheme) = grapheme_enumerator.move_next() {
        let text_style_overrides = overrides(i, grapheme.length() as i32, &red);

        i += grapheme.length() as i32;

        let layout = layout(text, TextLayoutOptions { text_style_overrides, ..options() });

        let text_line = &layout.text_lines()[0];

        let shaped_runs = shaped_runs(&**text_line);

        let run_starts: Vec<(Rc<dyn TextRun>, i32)> = text_line
            .get_text_bounds(text_line.first_text_source_index(), text_line.length())
            .iter()
            .flat_map(|bounds| bounds.text_run_bounds().to_vec())
            .filter(|bounds| bounds.text_run().is::<ShapedTextRun>())
            .map(|bounds| (bounds.text_run().clone(), bounds.text_source_character_index()))
            .collect();

        let mut clusters: Vec<i32> = Vec::new();

        for run in &shaped_runs {
            let raw_clusters: Vec<i32> =
                run.shaped_buffer().glyph_infos().iter().map(|glyph| glyph.glyph_cluster).collect();

            let run_start = run_starts
                .iter()
                .find(|(text_run, _)| {
                    std::ptr::addr_eq(Rc::as_ptr(text_run), Rc::as_ptr(run))
                })
                .map(|(_, start)| *start);

            let Some(run_start) = run_start.filter(|_| !raw_clusters.is_empty()) else {
                clusters.extend(raw_clusters);
                continue;
            };

            // A run's clusters are relative to the text it was shaped from, which is its
            // own text for a freshly shaped run but the parent's for a split child. The
            // smallest cluster is the run's first character either way, so rebasing on it
            // maps both onto text source indices.
            let base_cluster = raw_clusters.iter().copied().min().unwrap();

            clusters.extend(raw_clusters.iter().map(|cluster| cluster - base_cluster + run_start));
        }

        let glyph_advances = glyph_advances(&**text_line);

        let mut current_x = 0.0;

        let mut cluster = utf16_len(text);

        for j in 0..clusters.len() - 1 {
            let glyph_advance = glyph_advances[j];

            let character_hit = text_line.get_character_hit_from_distance(current_x);

            assert!(
                cluster == character_hit.first_character_index() + character_hit.trailing_length(),
                "grapheme={}, j={j}, cluster={cluster}, hit={}+{}, currentX={current_x}, runs={}, clusters={clusters:?}",
                i - grapheme.length() as i32,
                character_hit.first_character_index(),
                character_hit.trailing_length(),
                shaped_runs.len()
            );

            let distance = text_line.get_distance_from_character_hit(CharacterHit::new(cluster));

            assert_close(current_x, distance, 5);

            current_x += glyph_advance;

            if glyph_advance > 0.0 {
                cluster = clusters[j];
            }
        }
    }
}

/// Upstream asserts bands for the widths of real fonts; the widths here are
/// recomputed from the fixed advances (6 per character, 12 per emoji), which
/// happen to lie in upstream's bands too.
#[test]
fn hit_test_text_range_range_valid_length() {
    let cases = [
        ("mgfg🧐df f sdf", "g🧐d", 2.0 * advance(EM) + 2.0 * advance(EM)),
        ("وه. وقد تعرض لانتقادات", "دات", 3.0 * advance(EM)),
        ("وه. وقد تعرض لانتقادات", "تعرض", 4.0 * advance(EM)),
        (" علمية 😱ومضللة ،", " علمية 😱ومضللة ،", 15.0 * advance(EM) + 2.0 * advance(EM)),
        ("في عام 2018 ، رفعت ل", "في عام 2018 ، رفعت ل", 20.0 * advance(EM)),
    ];

    for (text, text_to_select, expected_width) in cases {
        let _scope = TextTestScope::new();

        let layout = layout(text, options());
        let start = utf16_len(&text[..text.find(text_to_select).unwrap()]);
        let selection_rectangles = layout.hit_test_text_range(start, utf16_len(text_to_select));

        assert_eq!(selection_rectangles.len(), 1, "{text:?}");

        let rect = selection_rectangles[0];

        assert_close(expected_width, rect.width, 5);
    }
}

/// Upstream's expected rectangles are pixel values of real fonts; they are
/// recomputed here from the fixed advances (6 per character, 12 per emoji).
#[test]
fn should_hit_test_text_range_between_runs() {
    let cases: [(&str, i32, i32, FlowDirection, &[(f64, f64)]); 4] = [
        ("012🧐210", 2, 4, FlowDirection::LeftToRight, &[(12.0, 36.0)]),
        ("210🧐012", 2, 4, FlowDirection::RightToLeft, &[(0.0, 6.0), (18.0, 30.0), (42.0, 48.0)]),
        ("שנב🧐שנב", 2, 4, FlowDirection::LeftToRight, &[(12.0, 36.0)]),
        ("שנב🧐שנב", 2, 4, FlowDirection::RightToLeft, &[(12.0, 36.0)]),
    ];

    for (text, start, length, flow_direction, expected_rects) in cases {
        let _scope = TextTestScope::new();

        let text_layout = layout(text, TextLayoutOptions { flow_direction, ..options() });

        let rects = text_layout.hit_test_text_range(start, length);

        assert_eq!(rects.len(), expected_rects.len(), "{text:?} {flow_direction:?}: {rects:?}");

        for (i, (expected_left, expected_right)) in expected_rects.iter().enumerate() {
            assert_close(*expected_left, rects[i].left(), 2);

            assert_close(*expected_right, rects[i].right(), 2);
        }
    }
}

#[test]
fn should_hit_test_text_range_with_line_breaks() {
    let _scope = TextTestScope::new();

    // Upstream uses the new line of the environment.
    const NEW_LINE: &str = "\n";

    let before_linebreak = "Line before linebreak";
    let after_linebreak = "Line after linebreak";
    let text = format!("{before_linebreak}{NEW_LINE}{NEW_LINE}{after_linebreak}");

    let text_layout = layout(&text, options());

    let end = (text.len() - after_linebreak.len() + 1) as i32;

    let rects = text_layout.hit_test_text_range(0, end);

    assert_eq!(rects.len(), 3);

    let end_x = text_layout.text_lines()[2].get_distance_from_character_hit(CharacterHit::new(end));

    // First character should be covered (upstream: 7.201171875, the advance of its font).
    assert_close(advance(EM), end_x, 2);
}

#[test]
fn should_hit_test_text_position_end_of_line_rtl() {
    let text = "גש\r\n";

    let _scope = TextTestScope::new();

    let text_layout = layout(text, TextLayoutOptions { flow_direction: FlowDirection::RightToLeft, ..options() });

    let rect = text_layout.hit_test_text_position(utf16_len(text));

    // Upstream: 16.32, the line height of its font.
    assert_eq!(rect.top(), line_height(EM));
}

/// Upstream runs this against a font with an "fi" ligature that only exists on
/// one platform; the harness shaper is given the ligature.
#[test]
fn should_handle_text_style_with_ligature() {
    let scope = TextTestScope::new();

    scope.shaper().add_ligature("fi");

    let text = "fi";

    let typeface = Typeface::default_typeface();

    let white: Rc<dyn IBrush> = Brushes::white();

    let style: Rc<dyn TextRunProperties> = Rc::new(GenericTextRunProperties::with_all(
        typeface.clone(),
        GenericTextRunProperties::DEFAULT_FONT_RENDERING_EM_SIZE,
        None,
        Some(white),
        None,
        BaselineAlignment::Baseline,
        None,
        None,
    ));

    let text_layout = TextLayout::new(
        text,
        typeface,
        TextLayoutOptions { text_style_overrides: Some(Rc::from(vec![ValueSpan::new(1, 1, style)])), ..options() },
    );

    assert_eq!(text_layout.text_lines().iter().map(|line| line.length()).sum::<i32>(), 2);
}

/// Upstream measures a glyph of its symbols font, whose advance and line
/// height are one em; the harness builds a font of that shape.
#[test]
fn should_measure_text_layout_symbol_with_and_width_including_trailing_whitespace() {
    const SYMBOLS_FAMILY: &str = "Test Symbols";

    let _scope = TextTestScope::with_fonts(vec![
        TestFont::new(DEFAULT_FAMILY).with_ranges(&[(0x20, 0x7E)]),
        TestFont::new(SYMBOLS_FAMILY).with_ranges(&[(0xE900, 0xE9FF)]).with_advance(1000).with_line_gap(0),
    ]);

    let white: Rc<dyn IBrush> = Brushes::white();

    let text_layout = TextLayout::new(
        "\u{e971}",
        Typeface::from_name(SYMBOLS_FAMILY),
        TextLayoutOptions { font_size: 12.0, foreground: Some(white), ..Default::default() },
    );

    assert_eq!((text_layout.width(), text_layout.height()), (12.0, 12.0));
    assert_eq!(text_layout.width_including_trailing_whitespace(), 12.0);
}

#[test]
fn should_wrap_with_line_end() {
    let _scope = TextTestScope::new();

    let foreground = black();

    let default_properties: Rc<dyn TextRunProperties> = Rc::new(GenericTextRunProperties::with_all(
        Typeface::default_typeface(),
        72.0,
        None,
        foreground,
        None,
        BaselineAlignment::Baseline,
        None,
        None,
    ));

    let paragraph_properties: Rc<dyn TextParagraphProperties> = Rc::new(GenericTextParagraphProperties::with_options(
        default_properties.clone(),
        TextAlignment::Left,
        TextWrapping::Wrap,
        0.0,
        0.0,
    ));

    let text_layout = TextLayout::from_text_source(
        Rc::new(SingleBufferTextSource::with_end_of_paragraph("01", default_properties, true)),
        paragraph_properties,
        None,
        36.0,
        f64::INFINITY,
        0,
        None,
    );

    assert_eq!(text_layout.text_lines().len(), 2);

    let last_line = text_layout.text_lines().last().unwrap();

    let text_runs = last_line.text_runs();

    assert_eq!(text_runs.len(), 2);

    let last_run = text_runs.last().unwrap();

    assert!(last_run.as_text_end_of_line().is_some());
}

/// Upstream uses a monospaced font; every harness font is one.
#[test]
fn should_measure_text_layout_symbol_with_and_width_including_trailing_whitespace_and_min_text_width() {
    let _scope = TextTestScope::new();

    let white = || -> TextLayoutOptions {
        let white: Rc<dyn IBrush> = Brushes::white();

        TextLayoutOptions { font_size: 12.0, foreground: Some(white), ..Default::default() }
    };

    let text_layout0 = layout("aaaa", white());
    assert_eq!(text_layout0.width(), text_layout0.width_including_trailing_whitespace());

    let text_layout01 = layout("a a", white());
    let text_layout1 = layout("a a ", white());
    assert_eq!(
        (text_layout1.width_including_trailing_whitespace(), text_layout1.height()),
        (text_layout0.width(), text_layout0.height())
    );
    assert_eq!(text_layout1.width_including_trailing_whitespace(), text_layout0.width_including_trailing_whitespace());

    let text_layout2 = layout(" aa ", white());
    assert_eq!((text_layout2.width(), text_layout2.height()), (text_layout1.width(), text_layout1.height()));
    assert_eq!(text_layout2.width_including_trailing_whitespace(), text_layout0.width_including_trailing_whitespace());
    assert_eq!(text_layout2.width(), text_layout01.width());

    let text_layout3 = layout("    ", white());
    assert_eq!((text_layout3.width(), text_layout3.height()), (0.0, text_layout0.height()));
    assert_eq!(text_layout3.width_including_trailing_whitespace(), text_layout0.width_including_trailing_whitespace());
    assert_eq!(text_layout3.width(), 0.0);
}

#[test]
fn inter_word_justification_does_not_stretch_last_cjk_glyph() {
    let _scope = TextTestScope::new();

    // Pure CJK (Han) has no inter-word spaces; UAX#14 LB31 yields a break opportunity
    // between essentially every ideograph, so justification distributes space
    // inter-character. Justifying to a width wider than the shaped line must widen
    // interior glyphs (including the first) but leave the final visible glyph's advance
    // untouched - otherwise the last ideograph is not flush to the line edge. Drives
    // InterWordJustification directly with an explicit target width to avoid the
    // widest-line / last-line behaviour of the full TextLayout pipeline.
    const TEXT: &str = "一二三四五";

    let default_properties = run_properties(EM);
    let text_source = SingleBufferTextSource::new(TEXT, default_properties.clone());

    let text_line = format_single_line(&text_source, &default_properties);

    let natural_width = text_line.width_including_trailing_whitespace();
    let before = glyph_advances(&*text_line);

    assert!(before.len() >= 2);

    let target_width = natural_width + 40.0;

    text_line.justify(&InterWordJustification::new(target_width));

    let after = glyph_advances(&*text_line);

    assert_eq!(after.len(), before.len());

    // The line is stretched to the requested width.
    assert_close(target_width, text_line.width_including_trailing_whitespace(), 3);

    // The last visible glyph keeps its original advance (no trailing overshoot).
    assert_close(before[before.len() - 1], after[after.len() - 1], 3);

    // The first glyph participates in justification (the leading gap is widened).
    assert_greater_than(after[0], before[0], "The first glyph should be widened");
}

fn left_and_justified(text: &str, max_width: f64) -> (TextLayout, TextLayout) {
    let left = layout(
        text,
        TextLayoutOptions {
            text_alignment: TextAlignment::Left,
            text_wrapping: TextWrapping::Wrap,
            max_width,
            ..options()
        },
    );

    let justified = layout(
        text,
        TextLayoutOptions {
            text_alignment: TextAlignment::Justify,
            text_wrapping: TextWrapping::Wrap,
            max_width,
            ..options()
        },
    );

    (left, justified)
}

#[test]
fn should_justify_wrapped_cjk_line_to_max_width() {
    let _scope = TextTestScope::new();

    // With the MaxWidth justification target (not the widest produced line), a wrapped
    // CJK paragraph fills each justified line to the paragraph margin, without stretching
    // the last visible glyph of the line. Compared against a Left layout of the same
    // text/width so the assertions do not depend on the fallback font's metrics.
    const TEXT: &str = "一二三四五六七八九十一二三四五六七八九十";
    const MAX_WIDTH: f64 = 100.0;

    let (left, justified) = left_and_justified(TEXT, MAX_WIDTH);

    // A wrapped, non-last line (the last line's treatment is a separate concern).
    assert!(justified.text_lines().len() >= 2);

    let left_line = &left.text_lines()[0];
    let justified_line = &justified.text_lines()[0];

    // The line must have slack to distribute, otherwise the test proves nothing.
    assert_greater_than(
        MAX_WIDTH,
        left_line.width_including_trailing_whitespace(),
        "The unjustified line must be narrower than MaxWidth",
    );

    let before = glyph_advances(&**left_line);
    let after = glyph_advances(&**justified_line);

    assert_eq!(after.len(), before.len());

    // The wrapped non-last line fills to MaxWidth (not the widest produced line).
    assert_close(MAX_WIDTH, justified_line.width_including_trailing_whitespace(), 3);

    // The last visible glyph is unchanged; the first glyph is widened.
    assert_close(before[before.len() - 1], after[after.len() - 1], 3);
    assert_greater_than(after[0], before[0], "The first glyph should be widened");
}

#[test]
fn does_not_justify_last_line_of_wrapped_paragraph() {
    let _scope = TextTestScope::new();

    // The last line of a justified paragraph stays start-aligned (its natural width),
    // while the preceding wrapped lines fill to the margin.
    let text = "一".repeat(40);
    const MAX_WIDTH: f64 = 80.0;

    let (left, justified) = left_and_justified(&text, MAX_WIDTH);

    let line_count = justified.text_lines().len();

    assert!(line_count >= 2);

    let last_left = &left.text_lines()[line_count - 1];
    let last_justified = &justified.text_lines()[line_count - 1];

    // Precondition: the last line is shorter than the margin, so stretching would show.
    assert_greater_than(
        MAX_WIDTH,
        last_left.width_including_trailing_whitespace(),
        "The last line must be shorter than MaxWidth for the test to be meaningful",
    );

    // The last line is not stretched.
    assert_close(
        last_left.width_including_trailing_whitespace(),
        last_justified.width_including_trailing_whitespace(),
        3,
    );

    // A preceding wrapped line is still justified to the margin.
    assert_close(MAX_WIDTH, justified.text_lines()[0].width_including_trailing_whitespace(), 3);
}

#[test]
fn does_not_justify_line_ending_in_hard_break() {
    let _scope = TextTestScope::new();

    // "一二三" ends in an explicit newline (a hard break); it stays start-aligned while
    // the following width-wrapped, non-last line fills to the margin.
    let text = format!("一二三\n{}", "一".repeat(40));
    const MAX_WIDTH: f64 = 80.0;

    let (left, justified) = left_and_justified(&text, MAX_WIDTH);

    // Line 0 ends in '\n', line 1 is a wrapped non-last line, line 2 is the last line.
    assert!(justified.text_lines().len() >= 3);

    let hard_break_left = &left.text_lines()[0];
    let hard_break_justified = &justified.text_lines()[0];

    assert_greater_than(
        MAX_WIDTH,
        hard_break_left.width_including_trailing_whitespace(),
        "The hard-break line must be shorter than MaxWidth for the test to be meaningful",
    );

    // The hard-break line is not stretched.
    assert_close(
        hard_break_left.width_including_trailing_whitespace(),
        hard_break_justified.width_including_trailing_whitespace(),
        3,
    );

    // The following width-wrapped, non-last line is justified to the margin.
    assert_close(MAX_WIDTH, justified.text_lines()[1].width_including_trailing_whitespace(), 3);
}

#[test]
fn justify_does_not_mutate_shared_shaped_buffer() {
    let _scope = TextTestScope::new();

    // A TextRunCache keeps the same ShapedTextRun (and its pooled glyph storage) alive
    // across layouts. Justification must copy-on-write rather than mutate that shared
    // buffer in place. Simulate the cache's reference with an added reference and assert the
    // original buffer is untouched while the line's run is replaced with a widened copy.
    const TEXT: &str = "一二三四五";

    let default_properties = run_properties(EM);
    let text_source = SingleBufferTextSource::new(TEXT, default_properties.clone());

    let text_line = format_single_line(&text_source, &default_properties);

    let original_run = shaped_runs(&*text_line).remove(0);
    let original_buffer = original_run.shaped_buffer().clone();
    let original_first_advance = original_buffer.get(0).glyph_advance;

    // Second owner (stands in for the TextRunCache) so the buffer survives the run's
    // disposal during justification.
    original_run.add_reference();

    text_line.justify(&InterWordJustification::new(original_run.size().width + 40.0));

    // The shared buffer is not mutated...
    assert_close(original_first_advance, original_buffer.get(0).glyph_advance, 5);

    // ...and the line now holds a different run whose first glyph was widened.
    let justified_run = shaped_runs(&*text_line).remove(0);

    assert!(!Rc::ptr_eq(&original_run, &justified_run));
    assert_greater_than(
        justified_run.shaped_buffer().get(0).glyph_advance,
        original_first_advance,
        "The justified copy's first glyph should be widened",
    );

    original_run.dispose();
}

#[test]
fn justify_repoints_indexed_runs_at_replacement() {
    let _scope = TextTestScope::new();

    // Draw and hit-test resolve runs through the bidi-reordered IndexedTextRun list, so
    // after justification replaces a run its IndexedTextRun must point at the
    // replacement. GetTextBounds walks the indexed runs; its total must match the
    // justified line width, not the pre-justification width.
    let text = "一".repeat(40);
    const MAX_WIDTH: f64 = 80.0;

    let (_, justified) = left_and_justified(&text, MAX_WIDTH);

    assert!(justified.text_lines().len() >= 2);

    let line = &justified.text_lines()[0];

    let bounds = line.get_text_bounds(line.first_text_source_index(), line.length());

    let bounds_width: f64 = bounds.iter().map(|bounds| bounds.rectangle().width).sum();

    assert_close(line.width_including_trailing_whitespace(), bounds_width, 2);
    assert_close(MAX_WIDTH, bounds_width, 2);
}

#[test]
fn justify_distributes_across_multiple_runs() {
    let _scope = TextTestScope::new();

    // Two shaped runs on one line (split by a font-size change). Each must receive its
    // own break opportunities: the pre-fix apply loop drained the whole queue against
    // the first run, leaving later runs unjustified.
    const TEXT: &str = "一二三四五六";

    let first = run_properties(20.0);
    let second = run_properties(12.0);
    let text_source = SplitStyleTextSource::new(TEXT, 3, first.clone(), second);

    let text_line = format_single_line(&text_source, &first);

    let runs = shaped_runs(&*text_line);

    // Confirm the line really is multi-run, otherwise the test proves nothing.
    assert!(runs.len() >= 2);

    let before_widths: Vec<f64> = runs.iter().map(|run| run.size().width).collect();

    text_line.justify(&InterWordJustification::new(text_line.width_including_trailing_whitespace() + 60.0));

    let after_runs = shaped_runs(&*text_line);

    assert_eq!(after_runs.len(), runs.len());

    // Every shaped run participated in justification, not just the first.
    for (i, after_run) in after_runs.iter().enumerate() {
        assert_greater_than(after_run.size().width, before_widths[i], &format!("Run {i} should be widened"));
    }
}

#[test]
fn justify_distributes_across_a_word_gap_at_a_run_boundary() {
    let _scope = TextTestScope::new();

    // The emoji needs a fallback font, so this line's word gaps sit next to a run
    // boundary. Break opportunities are collected run by run, and the break the
    // enumerator always reports at the end of the text it is given is discarded as an
    // artifact - so a real word gap that coincides with a run boundary yields no
    // opportunity at all and never widens.
    const TEXT: &str = "abc \u{1F600} def";

    let default_properties = run_properties(EM);
    let text_source = SingleBufferTextSource::new(TEXT, default_properties.clone());

    let format = || format_single_line(&text_source, &default_properties);

    fn gap(line: &dyn TextLine, space_index: i32) -> f64 {
        line.get_distance_from_character_hit(CharacterHit::new(space_index + 1))
            - line.get_distance_from_character_hit(CharacterHit::new(space_index))
    }

    let reference = format();

    let first_gap_before = gap(&*reference, 3);
    let second_gap_before = gap(&*reference, 6);

    let text_line = format();

    // Confirm the line really is multi-run, otherwise the test proves nothing.
    assert!(text_line.text_runs().len() > 1, "The emoji should force a fallback run");

    text_line.justify(&InterWordJustification::new(text_line.width_including_trailing_whitespace() + 40.0));

    let first_gap = gap(&*text_line, 3);
    let second_gap = gap(&*text_line, 6);

    assert_greater_than(first_gap, first_gap_before, "The first word gap should be widened");
    assert_greater_than(second_gap, second_gap_before, "The second word gap should be widened");

    // Both gaps are break opportunities, so they take an equal share of the added width.
    assert_close(first_gap - first_gap_before, second_gap - second_gap_before, 3);
}

#[test]
fn justify_does_not_space_trailing_whitespace() {
    let cases = [
        "aa bb   ",                           // trailing ASCII spaces
        "一二\u{3000}\u{3000}\u{3000}", // trailing ideographic (U+3000) spaces
    ];

    for text in cases {
        let _scope = TextTestScope::new();

        // Trailing whitespace (which sits before a wrap point or hard break) must never
        // receive justification space; the full distributed amount lands in the visible
        // region instead. This is guaranteed by the LineBreakEnumerator (it emits no
        // non-required break inside trailing whitespace), so no explicit guard is needed in
        // InterWordJustification - this test locks that invariant. Width excludes trailing
        // whitespace, so it must grow by the entire distributed space; if trailing
        // whitespace absorbed part of it, Width would grow less.
        const EXTRA: f64 = 40.0;

        let default_properties = run_properties(EM);
        let text_source = SingleBufferTextSource::new(text, default_properties.clone());

        let text_line = format_single_line(&text_source, &default_properties);

        // Confirm the line actually carries trailing whitespace, otherwise the test proves nothing.
        assert!(text_line.trailing_whitespace_length() >= 2);
        assert_greater_than(
            text_line.width_including_trailing_whitespace(),
            text_line.width(),
            "The line must have measurable trailing whitespace",
        );

        let width_before = text_line.width();

        text_line.justify(&InterWordJustification::new(text_line.width() + EXTRA));

        assert_close(width_before + EXTRA, text_line.width(), 2);
    }
}

fn assert_wrapped_non_last_line_fills_to_max_width(text: &str, max_width: f64) {
    let (left, justified) = left_and_justified(text, max_width);

    assert!(justified.text_lines().len() >= 2);

    let left_line = &left.text_lines()[0];
    let justified_line = &justified.text_lines()[0];

    assert_greater_than(
        max_width,
        left_line.width(),
        "The unjustified line's visible content must be narrower than MaxWidth",
    );

    // The non-last wrapped line's visible content fills to the margin. (Width excludes any
    // trailing whitespace, which hangs past the margin.)
    assert_close(max_width, justified_line.width(), 2);

    // The last line stays start-aligned.
    let last_left = &left.text_lines()[left.text_lines().len() - 1];
    let last_justified = &justified.text_lines()[justified.text_lines().len() - 1];

    assert_close(
        last_left.width_including_trailing_whitespace(),
        last_justified.width_including_trailing_whitespace(),
        2,
    );
}

#[test]
fn should_justify_wrapped_latin_line() {
    // The classic inter-word case. A wrapped non-last Latin line fills to the margin; the
    // last line stays start-aligned.
    let _scope = TextTestScope::new();

    assert_wrapped_non_last_line_fills_to_max_width(
        "the quick brown fox jumps over the lazy dog and then runs away quite quickly today",
        140.0,
    );
}

#[test]
fn does_not_justify_latin_line_with_trailing_spaces_before_hard_break() {
    let _scope = TextTestScope::new();

    // A line ending in trailing spaces + a hard break stays start-aligned (its trailing
    // whitespace is not stretched), while the following wrapped lines still fill to the
    // margin.
    let text = format!("aa bb   \n{}", vec!["cc"; 40].join(" "));
    const MAX_WIDTH: f64 = 120.0;

    let (left, justified) = left_and_justified(&text, MAX_WIDTH);

    assert!(justified.text_lines().len() >= 3);

    let line0_left = &left.text_lines()[0];
    let line0_justified = &justified.text_lines()[0];

    assert_greater_than(
        MAX_WIDTH,
        line0_left.width_including_trailing_whitespace(),
        "The hard-break line must be shorter than MaxWidth for the test to be meaningful",
    );

    // The trailing-spaces + '\n' line is not stretched.
    assert_close(
        line0_left.width_including_trailing_whitespace(),
        line0_justified.width_including_trailing_whitespace(),
        2,
    );

    // A following width-wrapped, non-last line fills its visible content to the margin.
    assert_close(MAX_WIDTH, justified.text_lines()[1].width(), 2);
}

#[test]
fn justify_distributes_across_latin_and_cjk() {
    let _scope = TextTestScope::new();

    // A mixed Latin+CJK line distributes space across both the inter-word gap and the
    // inter-ideograph gaps (the line reaches the target width) without stretching the
    // last visible glyph.
    const TEXT: &str = "ab 日本語";

    let default_properties = run_properties(EM);
    let text_source = SingleBufferTextSource::new(TEXT, default_properties.clone());

    let text_line = format_single_line(&text_source, &default_properties);

    let last_glyph_before = last_glyph_advance(&*text_line);

    let target = text_line.width_including_trailing_whitespace() + 40.0;

    text_line.justify(&InterWordJustification::new(target));

    // Space was distributed across the mixed content (the line reaches the target)...
    assert_close(target, text_line.width_including_trailing_whitespace(), 2);

    // ...but the last visible glyph is not stretched.
    assert_close(last_glyph_before, last_glyph_advance(&*text_line), 3);
}

#[test]
fn justify_arabic_does_not_corrupt_shared_buffer() {
    let _scope = TextTestScope::new();

    // Arabic is right-to-left, so justification exercises the copy-on-write path on a
    // run in visual order. It must run without corrupting a cache-shared buffer.
    const TEXT: &str = "مرحبا بالعالم";

    let default_properties = run_properties(EM);
    let text_source = SingleBufferTextSource::new(TEXT, default_properties.clone());

    let text_line = format_single_line(&text_source, &default_properties);

    let original_run = shaped_runs(&*text_line).remove(0);
    let original_buffer = original_run.shaped_buffer().clone();
    let original_advances: Vec<f64> =
        (0..original_buffer.length()).map(|i| original_buffer.get(i).glyph_advance).collect();

    // Second owner (stands in for a TextRunCache) so the buffer survives run disposal.
    original_run.add_reference();

    let width_before = text_line.width_including_trailing_whitespace();

    text_line.justify(&InterWordJustification::new(width_before + 40.0));

    // Justification happened (the inter-word gap was widened)...
    assert_greater_than(
        text_line.width_including_trailing_whitespace(),
        width_before,
        "Justifying an Arabic line should widen it",
    );

    // ...and the shared buffer was not mutated in place.
    for (i, original_advance) in original_advances.iter().enumerate() {
        assert_close(*original_advance, original_buffer.get(i).glyph_advance, 5);
    }

    original_run.dispose();
}

#[test]
fn width_excludes_only_the_lines_true_trailing_whitespace() {
    let _scope = TextTestScope::new();

    // Two runs split by a font-size change. The FIRST (interior) run also ends in a
    // space of its own ("foo "), but that space is followed by more visible content
    // ("bar") in the next run, so it is NOT trailing whitespace for the line as a
    // whole - only the space at the true end of the line is. A reference line without
    // any trailing space isolates exactly how much Width should differ.
    let first = run_properties(20.0);
    let second = run_properties(14.0);

    let reference =
        format_single_line(&SplitStyleTextSource::new("foo bar", 4, first.clone(), second.clone()), &first);

    let with_trailing_space =
        format_single_line(&SplitStyleTextSource::new("foo bar ", 4, first.clone(), second), &first);

    // Confirm both lines are genuinely multi-run, and the reference truly has no
    // trailing whitespace, otherwise the comparison proves nothing.
    assert!(shaped_runs(&*reference).len() >= 2);
    assert_eq!(reference.trailing_whitespace_length(), 0);

    // Only the one added trailing space is excluded - not that space AND "foo "'s own
    // interior trailing space too.
    assert_eq!(with_trailing_space.trailing_whitespace_length(), 1);
    assert_close(reference.width(), with_trailing_space.width(), 3);
}

// --- the layout based tests of upstream's run cache tests ---

#[test]
fn text_layout_recreated_from_cache_with_bidi_has_same_glyph_order() {
    let _scope = TextTestScope::new();

    // Mixed LTR/RTL text used for two successive TextLayout instances that share a cache.
    // Before the fix, the second instance would double-reverse RTL glyph buffers.
    let text = "Hello \u{0645}\u{0631}\u{062D}\u{0628}\u{0627} World";

    let cache = Rc::new(TextRunCache::new());

    let with_cache =
        || TextLayoutOptions { font_size: 12.0, text_run_cache: Some(cache.clone()), ..Default::default() };

    let layout1 = layout(text, with_cache());

    // Second layout: triggers cache-hit path - previously double-reordered RTL runs.
    let layout2 = layout(text, with_cache());

    assert_eq!(layout2.text_lines().len(), layout1.text_lines().len());

    for line_idx in 0..layout1.text_lines().len() {
        let line1 = &layout1.text_lines()[line_idx];
        let line2 = &layout2.text_lines()[line_idx];

        assert_eq!(line2.length(), line1.length());

        let runs1 = line1.text_runs();
        let runs2 = line2.text_runs();

        assert_eq!(runs2.len(), runs1.len());

        for i in 0..runs1.len() {
            let (Some(run1), Some(run2)) =
                (runs1[i].downcast_ref::<ShapedTextRun>(), runs2[i].downcast_ref::<ShapedTextRun>())
            else {
                continue;
            };

            assert_eq!(run2.bidi_level(), run1.bidi_level());
            assert_eq!(run2.shaped_buffer().is_left_to_right(), run1.shaped_buffer().is_left_to_right());
            assert_eq!(run2.shaped_buffer().length(), run1.shaped_buffer().length());

            for j in 0..run1.shaped_buffer().length() {
                assert_eq!(run2.shaped_buffer().get(j).glyph_cluster, run1.shaped_buffer().get(j).glyph_cluster);
            }
        }
    }

    layout2.dispose();
    layout1.dispose();
    cache.dispose();
}

#[test]
fn text_layout_with_cache_matches_without() {
    let _scope = TextTestScope::new();

    let text = "The quick brown fox jumps over the lazy dog";

    // Layout without cache.
    let layout1 = layout(
        text,
        TextLayoutOptions { font_size: 12.0, text_wrapping: TextWrapping::Wrap, max_width: 100.0, ..Default::default() },
    );

    // Layout with cache.
    let cache = Rc::new(TextRunCache::new());

    let layout2 = layout(
        text,
        TextLayoutOptions {
            font_size: 12.0,
            text_wrapping: TextWrapping::Wrap,
            max_width: 100.0,
            text_run_cache: Some(cache.clone()),
            ..Default::default()
        },
    );

    assert_eq!(layout2.text_lines().len(), layout1.text_lines().len());
    assert_eq!(layout2.height(), layout1.height());
    assert_eq!(layout2.width_including_trailing_whitespace(), layout1.width_including_trailing_whitespace());

    // Second layout from cache with different width.
    let layout3 = layout(
        text,
        TextLayoutOptions {
            font_size: 12.0,
            text_wrapping: TextWrapping::Wrap,
            max_width: 80.0,
            text_run_cache: Some(cache.clone()),
            ..Default::default()
        },
    );

    // Should still be valid (more lines due to narrower width).
    assert!(layout3.text_lines().len() >= layout2.text_lines().len());
    assert!(layout3.height() > 0.0);

    layout1.dispose();
    layout2.dispose();
    layout3.dispose();

    cache.dispose();
}

// --- the layout based test of upstream's empty shaped buffer tests ---

#[test]
fn should_wrap_text_that_starts_with_a_line_break() {
    let scope = TextTestScope::new();

    // The premise of upstream's test: the shaper produces no glyph for the line break.
    scope.shaper().set_drop_line_breaks(true);

    // MaxLines bounds the layout loop: a line that covers no text never advances the text
    // source, so without it a regression here hangs the test run instead of failing it.
    let layout = layout(
        "\r\nPassword update failed",
        TextLayoutOptions { text_wrapping: TextWrapping::Wrap, max_width: 290.0, max_lines: 5, ..options() },
    );

    assert_eq!(layout.text_lines().len(), 2);

    assert_eq!(layout.text_lines()[0].length(), 2);
    assert_eq!(layout.text_lines()[1].length(), 22);
}

// --- additions (not in upstream's test file) ---

/// Addition: the members that have no upstream test of their own.
#[test]
fn addition_layout_metrics_drawing_and_line_lookup() {
    use crate::media::text_formatting::testing::{DrawCall, RecordingDrawingSink};

    let _scope = TextTestScope::new();

    let text_layout = layout("ab cd\nef", TextLayoutOptions { letter_spacing: 0.0, max_lines: 3, ..options() });

    assert_eq!(text_layout.text_lines().len(), 2);
    assert_eq!(text_layout.width(), 5.0 * advance(EM));
    assert_eq!(text_layout.width_including_trailing_whitespace(), 5.0 * advance(EM));
    assert_eq!(text_layout.height(), 2.0 * line_height(EM));
    assert_eq!(text_layout.baseline(), text_layout.text_lines()[0].baseline());
    assert_eq!(text_layout.extent(), text_layout.text_lines()[0].extent());
    assert_eq!(text_layout.overhang_after(), text_layout.text_lines()[1].overhang_after());
    assert_eq!(text_layout.overhang_leading(), text_layout.text_lines()[0].overhang_leading());
    assert_eq!(text_layout.overhang_trailing(), text_layout.text_lines()[0].overhang_trailing());
    assert!(text_layout.line_height().is_nan());
    assert_eq!(text_layout.letter_spacing(), 0.0);
    assert_eq!(text_layout.max_width(), f64::INFINITY);
    assert_eq!(text_layout.max_height(), f64::INFINITY);
    assert_eq!(text_layout.max_lines(), 3);

    assert_eq!(text_layout.get_line_index_from_character_index(-1, false), 0);
    assert_eq!(text_layout.get_line_index_from_character_index(0, false), 0);
    assert_eq!(text_layout.get_line_index_from_character_index(5, false), 0);
    assert_eq!(text_layout.get_line_index_from_character_index(6, false), 1);
    assert_eq!(text_layout.get_line_index_from_character_index(6, true), 0);
    assert_eq!(text_layout.get_line_index_from_character_index(8, true), 1);
    assert_eq!(text_layout.get_line_index_from_character_index(100, false), 1);

    // A caret rectangle on the second line.
    let rect = text_layout.hit_test_text_position(7);

    assert_eq!((rect.x, rect.y, rect.width, rect.height), (advance(EM), line_height(EM), advance(EM), line_height(EM)));

    // Hit testing below the last line is a trailing hit on the last line.
    let hit = text_layout.hit_test_point(Point::new(1000.0, 1000.0));

    assert_eq!(hit.text_position(), 8);
    assert!(hit.is_trailing());
    assert!(!hit.is_inside());

    let hit = text_layout.hit_test_point(Point::new(advance(EM) * 1.25, 1.0));

    assert_eq!(hit.text_position(), 1);
    assert!(hit.is_inside());
    assert!(!hit.is_trailing());

    assert!(text_layout.hit_test_text_range(-5, 2).is_empty());

    let mut sink = RecordingDrawingSink::new();

    text_layout.draw(&mut sink, Point::new(10.0, 20.0));

    let glyph_runs: Vec<(String, Point)> = sink
        .calls()
        .into_iter()
        .filter_map(|call| match call {
            DrawCall::GlyphRun { text, origin } => Some((text, origin)),
            _ => None,
        })
        .collect();

    assert_eq!(glyph_runs.len(), 2);
    assert_eq!(glyph_runs[0].0, "ab cd\n");
    assert_eq!(glyph_runs[0].1, Point::new(10.0, 20.0));
    assert_eq!(glyph_runs[1].0, "ef");
    assert_eq!(glyph_runs[1].1, Point::new(10.0, 20.0 + line_height(EM)));
    assert_eq!(sink.transform_depth(), 0);

    text_layout.dispose();
}

/// Addition: a zero constraint gives one empty line; the max height trims the
/// last line that fits.
#[test]
fn addition_zero_constraint_and_max_height_trimming() {
    let _scope = TextTestScope::new();

    let empty = layout("abc", TextLayoutOptions { max_width: 0.0, ..options() });

    assert_eq!(empty.text_lines().len(), 1);
    assert_eq!(empty.text_lines()[0].length(), 0);
    assert_eq!(empty.height(), line_height(EM));
    assert_eq!(empty.width(), 0.0);

    let trimmed = layout(
        "aaa bbb ccc ddd eee fff",
        TextLayoutOptions {
            text_wrapping: TextWrapping::Wrap,
            text_trimming: Some(<dyn TextTrimming>::character_ellipsis()),
            max_width: 8.0 * advance(EM),
            max_height: 2.0 * line_height(EM),
            ..options()
        },
    );

    assert_eq!(trimmed.text_lines().len(), 2);
    assert!(trimmed.text_lines()[1].has_collapsed());
    assert!(!trimmed.text_lines()[0].has_collapsed());
    assert_eq!(trimmed.height(), 2.0 * line_height(EM));

    // The CJK family is only reached through the fallback.
    let cjk = layout("一", options());

    assert_eq!(
        shaped(&cjk.text_lines()[0].text_runs()[0]).glyph_run().glyph_typeface().family_name(),
        CJK_FAMILY
    );

    let formatter = <dyn TextFormatter>::current();

    assert!(Rc::ptr_eq(&formatter, &<dyn TextFormatter>::current()));
}
