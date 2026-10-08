//! Port of upstream's `Media/TextFormatting/TextLayoutTests.cs` of the Skia
//! unit tests: text layout on the real test fonts (the embedded assets as the
//! system fonts, Noto Mono the default family), shaped with HarfBuzz.
//!
//! xUnit's `Assert.Equal(double, double, precision)` rounds both values to
//! `precision` decimals (ties to even) and compares them; `assert_equal_precision`
//! does the same. Upstream's `TextTestHelper.GetStartCharIndex` (the position
//! of a run's text in its string) is the offset of the text in its owner.

use super::SingleBufferTextSource;
use crate::unit_tests::media::CustomFontManagerImpl;
use crate::unit_tests::mock_platform_render_interface;
use crate::PlatformRenderInterface;
use ferroui_base::media::text_formatting::unicode::{Codepoint, GraphemeEnumerator};
use ferroui_base::media::text_formatting::{
    DrawableTextRun, GenericTextParagraphProperties, GenericTextRunProperties, ITextSource, InterWordJustification, ShapedTextRun,
    TextCharacters, TextFormatter, TextFormatterImpl, TextLayout, TextLayoutOptions, TextLine,
    TextParagraphProperties, TextRun, TextRunProperties,
};
use ferroui_base::media::{
    BaselineAlignment, Brushes, CharacterHit, Colors, FlowDirection, IBrush, SolidColorBrush, TextAlignment,
    TextTrimming, TextWrapping, Typeface,
};
use ferroui_base::utilities::{ReadOnlyMemory, ValueSpan};
use ferroui_base::{Point, Rect, Size};
use ferroui_controls::testing::{UnitTestApplication, UnitTestApplicationScope};
use std::rc::Rc;

const SINGLE_LINE_TEXT: &str = "0123456789";
const MULTI_LINE_TEXT: &str = "01 23 45 678\r\rabc def gh ij";
const RIGHT_TO_LEFT_TEXT: &str = "זה כיף סתם לשמוע איך תנצח קרפד עץ טוב בגן";

// --- helpers ---

/// `Brushes.Black` (immutable already, so also `Brushes.Black.ToImmutable()`).
fn black() -> Rc<dyn IBrush> {
    Brushes::black()
}

/// `new SolidColorBrush(Colors.Red).ToImmutable()`.
fn red_immutable() -> Rc<dyn IBrush> {
    SolidColorBrush::with_color(Colors::RED).to_immutable()
}

/// `new TextLayout(text, Typeface.Default, 12, foreground)` with the other
/// arguments at their defaults; callers override fields.
fn options(foreground: Rc<dyn IBrush>) -> TextLayoutOptions {
    TextLayoutOptions { font_size: 12.0, foreground: Some(foreground), ..Default::default() }
}

fn layout(text: &str, options: TextLayoutOptions) -> TextLayout {
    TextLayout::new(text, Typeface::default_typeface(), options)
}

/// `new GenericTextRunProperties(typeface, fontRenderingEmSize, foregroundBrush: foreground)`.
fn run_properties(typeface: Typeface, font_rendering_em_size: f64, foreground: Option<Rc<dyn IBrush>>) -> Rc<dyn TextRunProperties> {
    Rc::new(GenericTextRunProperties::with_all(
        typeface,
        font_rendering_em_size,
        None,
        foreground,
        None,
        BaselineAlignment::Baseline,
        None,
        None,
    ))
}

/// `new[] { new ValueSpan<TextRunProperties>(start, length, new GenericTextRunProperties(Typeface.Default, 12, foregroundBrush: foreground)) }`.
fn spans(start: i32, length: i32, foreground: &Rc<dyn IBrush>) -> Option<Rc<[ValueSpan<Rc<dyn TextRunProperties>>]>> {
    Some(Rc::from(vec![ValueSpan::new(start, length, run_properties(Typeface::default_typeface(), 12.0, Some(foreground.clone())))]))
}

/// `(ShapedTextRun)run`.
fn shaped(run: &Rc<dyn TextRun>) -> &ShapedTextRun {
    run.downcast_ref::<ShapedTextRun>().expect("a ShapedTextRun")
}

/// `line.TextRuns.OfType<ShapedTextRun>()`.
fn shaped_runs(line: &dyn TextLine) -> Vec<Rc<ShapedTextRun>> {
    line.text_runs().iter().filter_map(|run| run.clone().downcast_rc::<ShapedTextRun>()).collect()
}

/// `Assert.Equal(foreground, run.Properties?.ForegroundBrush)`.
#[track_caller]
fn assert_foreground(foreground: &Rc<dyn IBrush>, run: &Rc<dyn TextRun>) {
    let actual = run.properties().and_then(|properties| properties.foreground_brush().cloned());

    assert!(actual.is_some_and(|actual| foreground.equals(&*actual)), "the run's foreground is not the expected brush");
}

/// `text.Length` (UTF-16 code units).
fn utf16_len(text: &str) -> i32 {
    text.encode_utf16().count() as i32
}

/// `text.Substring(start, length)` in UTF-16 code units.
fn substring(text: &str, start: i32, length: i32) -> String {
    let units: Vec<u16> = text.encode_utf16().collect();

    String::from_utf16_lossy(&units[start as usize..(start + length) as usize])
}

/// `text.IndexOf(value)` in UTF-16 code units.
fn index_of(text: &str, value: &str) -> i32 {
    utf16_len(&text[..text.find(value).expect("the value is in the text")])
}

/// Upstream's `TextTestHelper.GetStartCharIndex`.
fn get_start_char_index(text: ReadOnlyMemory<u16>) -> i32 {
    text.offset_in_owner() as i32
}

/// xUnit's `Assert.Equal(expected, actual, precision)`.
#[track_caller]
fn assert_equal_precision(expected: f64, actual: f64, precision: i32) {
    let factor = 10f64.powi(precision);
    let round = |value: f64| (value * factor).round_ties_even() / factor;

    assert!(round(expected) == round(actual), "expected {expected}, actual {actual} (precision {precision})");
}

/// xUnit's `Assert.InRange(actual, low, high)`.
#[track_caller]
fn assert_in_range(actual: f64, low: f64, high: f64) {
    assert!(low <= actual && actual <= high, "{actual} is not in the range [{low}, {high}]");
}

#[track_caller]
fn assert_greater_than(x: f64, y: f64, message: &str) {
    assert!(x > y, "{message}. {x} is not > {y}");
}

fn start() -> UnitTestApplicationScope {
    UnitTestApplication::start(
        mock_platform_render_interface()
            .with_render_interface(Rc::new(PlatformRenderInterface::new(None, None)))
            .with_font_manager_impl(Rc::new(CustomFontManagerImpl::new())),
    )
}

// --- tests ---

#[test]
fn should_break_lines() {
    for (text, number_of_lines) in [("01234\r01234\r", 3), ("01234\r01234", 2)] {
        let _app = start();

        let layout = layout(text, options(black()));

        assert_eq!(layout.text_lines().len(), number_of_lines, "{text:?}");
    }
}

#[test]
fn should_apply_text_style_span_to_text_in_between() {
    let _app = start();

    let foreground = red_immutable();

    let layout = layout(MULTI_LINE_TEXT, TextLayoutOptions { text_style_overrides: spans(1, 2, &foreground), ..options(black()) });

    let text_line = &layout.text_lines()[0];

    let text_runs = text_line.text_runs();

    assert_eq!(text_runs.len(), 3);

    let text_run = &text_runs[1];

    assert_eq!(text_run.length(), 2);

    let actual = text_run.text().to_string_lossy();

    assert_eq!(actual, "1 ");

    assert!(text_run.properties().is_some());
    assert_foreground(&foreground, text_run);
}

#[test]
fn should_wrap_and_apply_style() {
    for length in [27, 22] {
        let _app = start();

        let text = "Multiline TextBox with TextWrapping.";

        let foreground = red_immutable();

        let expected = layout(text, TextLayoutOptions { text_wrapping: TextWrapping::Wrap, max_width: 200.0, ..options(black()) });

        let expected_lines: Vec<String> = expected
            .text_lines()
            .iter()
            .map(|x| substring(text, x.first_text_source_index(), x.length()))
            .collect();

        let actual = layout(
            text,
            TextLayoutOptions {
                text_wrapping: TextWrapping::Wrap,
                max_width: 200.0,
                text_style_overrides: spans(0, length, &foreground),
                ..options(black())
            },
        );

        let actual_lines: Vec<String> =
            actual.text_lines().iter().map(|x| substring(text, x.first_text_source_index(), x.length())).collect();

        assert_eq!(actual_lines.len(), expected_lines.len(), "length {length}");

        for j in 0..actual.text_lines().len() {
            let expected_text = &expected_lines[j];

            let actual_text = &actual_lines[j];

            assert_eq!(actual_text, expected_text, "length {length}");
        }
    }
}

#[test]
fn should_not_alter_lines_after_text_style_span_was_applied() {
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

    let _app = start();

    const TEXT: &str = "אחד !\ntwo !\nשְׁלוֹשָׁה !";

    let red = red_immutable();
    let black = black();

    let expected = layout(TEXT, TextLayoutOptions { text_wrapping: TextWrapping::Wrap, ..options(black.clone()) });

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
                    text_style_overrides: spans(i, j, &red),
                    ..options(black.clone())
                },
            );

            let actual_glyphs = get_glyphs(&actual);

            assert_eq!(actual_glyphs.len(), expected_glyphs.len(), "span ({i}, {j})");

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
    let _app = start();

    let foreground = red_immutable();

    let layout = layout(SINGLE_LINE_TEXT, TextLayoutOptions { text_style_overrides: spans(0, 2, &foreground), ..options(black()) });

    let text_line = &layout.text_lines()[0];

    let text_runs = text_line.text_runs();

    assert_eq!(text_runs.len(), 2);

    let text_run = &text_runs[0];

    assert_eq!(text_run.length(), 2);

    let actual = &SINGLE_LINE_TEXT[..text_run.length() as usize];

    assert_eq!(actual, "01");

    assert!(text_run.properties().is_some());
    assert_foreground(&foreground, text_run);
}

#[test]
fn should_apply_text_style_span_to_text_at_end() {
    let _app = start();

    let foreground = red_immutable();

    let layout = layout(SINGLE_LINE_TEXT, TextLayoutOptions { text_style_overrides: spans(8, 2, &foreground), ..options(black()) });

    let text_line = &layout.text_lines()[0];

    let text_runs = text_line.text_runs();

    assert_eq!(text_runs.len(), 2);

    let text_run = &text_runs[1];

    assert_eq!(text_run.length(), 2);

    let actual = text_run.text().to_string_lossy();

    assert_eq!(actual, "89");

    assert!(text_run.properties().is_some());
    assert_foreground(&foreground, text_run);
}

#[test]
fn should_apply_text_style_span_to_single_character() {
    let _app = start();

    let foreground = red_immutable();

    let layout = layout("0", TextLayoutOptions { text_style_overrides: spans(0, 1, &foreground), ..options(black()) });

    let text_line = &layout.text_lines()[0];

    let text_runs = text_line.text_runs();

    assert_eq!(text_runs.len(), 1);

    let text_run = &text_runs[0];

    assert_eq!(text_run.length(), 1);

    assert!(text_run.properties().is_some());
    assert_foreground(&foreground, text_run);
}

#[test]
fn should_apply_text_span_to_unicode_string_in_between() {
    let _app = start();

    const TEXT: &str = "😄😄😄😄";

    let foreground = red_immutable();

    let layout = layout(TEXT, TextLayoutOptions { text_style_overrides: spans(2, 2, &foreground), ..options(black()) });

    let text_line = &layout.text_lines()[0];

    let text_runs = text_line.text_runs();

    assert_eq!(text_runs.len(), 3);

    let text_run = &text_runs[1];

    assert_eq!(text_run.length(), 2);

    let actual = text_run.text().to_string_lossy();

    assert_eq!(actual, "😄");

    assert!(text_run.properties().is_some());
    assert_foreground(&foreground, text_run);
}

#[test]
fn text_length_should_be_equal_to_text_line_length_sum() {
    let _app = start();

    let layout = layout(MULTI_LINE_TEXT, options(black()));

    assert_eq!(layout.text_lines().iter().map(|x| x.length()).sum::<i32>(), utf16_len(MULTI_LINE_TEXT));
}

#[test]
fn text_length_should_be_equal_to_text_run_text_length_sum() {
    let _app = start();

    let layout = layout(MULTI_LINE_TEXT, options(black()));

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
    let _app = start();

    const TEXT: &str = "Multiline TextBox with TextWrapping.\r\rLorem ipsum dolor sit amet";

    let foreground = red_immutable();

    let layout = layout(
        TEXT,
        TextLayoutOptions {
            text_wrapping: TextWrapping::Wrap,
            max_width: 180.0,
            text_style_overrides: spans(0, 24, &foreground),
            ..options(black())
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
    let _app = start();

    let foreground = red_immutable();

    let layout = layout(
        MULTI_LINE_TEXT,
        TextLayoutOptions {
            max_width: 200.0,
            max_height: 125.0,
            text_style_overrides: spans(5, 20, &foreground),
            ..options(black())
        },
    );

    assert_foreground(&foreground, &layout.text_lines()[0].text_runs()[1]);
    assert_foreground(&foreground, &layout.text_lines()[1].text_runs()[0]);
    assert_foreground(&foreground, &layout.text_lines()[2].text_runs()[0]);
}

#[test]
fn should_hit_test_surrogate_pair() {
    let _app = start();

    const TEXT: &str = "😄😄";

    let layout = layout(TEXT, options(black()));

    let text_runs = layout.text_lines()[0].text_runs();

    let shaped_run = shaped(&text_runs[0]);

    let glyph_run = shaped_run.glyph_run();

    let width = glyph_run.bounds().width;

    let (character_hit, _) = glyph_run.get_character_hit_from_distance(width);

    assert_eq!(character_hit.first_character_index(), 2);

    assert_eq!(character_hit.trailing_length(), 2);
}

#[test]
fn should_create_valid_clusters_for_text() {
    let cases: [(&str, &[i32]); 3] = [("☝🏿", &[0]), ("☝🏿 ab", &[0, 0, 1, 2]), ("ab ☝🏿", &[0, 1, 2, 0])];

    for (text, clusters) in cases {
        let _app = start();

        let layout = layout(text, options(black()));

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
        let _app = start();

        let layout = layout(text, options(black()));

        assert_eq!(layout.text_lines().len(), 2, "{text:?}");

        let text_runs = layout.text_lines()[0].text_runs();

        assert_eq!(text_runs.len(), 1, "{text:?}");

        assert_eq!(shaped(&text_runs[0]).glyph_run().glyph_infos().count(), expected_length, "{text:?}");

        assert_eq!(shaped(&text_runs[0]).shaped_buffer().get(5).glyph_cluster, 5, "{text:?}");

        if expected_length == 7 {
            assert_eq!(shaped(&text_runs[0]).shaped_buffer().get(6).glyph_cluster, 5, "{text:?}");
        }
    }
}

#[test]
fn should_have_one_run_with_common_script() {
    let _app = start();

    let layout = layout("abcde\r\n", options(black()));

    assert_eq!(layout.text_lines()[0].text_runs().len(), 1);
}

#[test]
fn should_layout_corrupted_text() {
    let _app = start();

    // `new string(new[] { '\uD802', ... })`: seven lone high surrogates, which
    // a `&str` cannot hold.
    let text = ReadOnlyMemory::from_vec(vec![0xD802u16; 7]);

    let layout = TextLayout::from_utf16(text, Typeface::default_typeface(), options(black()));

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
        let _app = start();

        let glyph_typeface = Typeface::default_typeface().glyph_typeface();

        let em_height = glyph_typeface.metrics().design_em_height as f64;

        let line_height = glyph_typeface.metrics().line_spacing() as f64 * (12.0 / em_height);

        let layout = layout(text, TextLayoutOptions { max_height: line_height - line_height * 0.5, ..options(black()) });

        assert_eq!(layout.text_lines().len(), 1, "{text:?}");

        assert_eq!(layout.height(), line_height, "{text:?}");
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
        let _app = start();

        let layout = layout(text, TextLayoutOptions { max_width: 50.0, max_lines, ..options(black()) });

        assert_eq!(layout.text_lines().len(), expected_lines, "{text:?}, {max_lines}");
    }
}

/// Upstream's `MaxLinesEllipsisData`.
fn max_lines_ellipsis_data() -> [Rc<dyn TextTrimming>; 2] {
    [<dyn TextTrimming>::character_ellipsis(), <dyn TextTrimming>::word_ellipsis()]
}

#[test]
fn should_add_ellipsis_when_max_lines_cuts_short_wrapped_line() {
    for (row, trimming) in max_lines_ellipsis_data().into_iter().enumerate() {
        let _app = start();

        const TEXT: &str = "Lorem ipsum dolor sit amet, consectetur adipiscing elit, sed do eiusmod tempor incididunt ut labore et dolore magna aliqua. Ut enim ad minim veniam, quis nostrud exercitation ullamco laboris nisi ut aliquip ex ea commodo consequat. Duis aute irure dolor in reprehenderit in voluptate velit esse cillum dolore eu fugiat nulla pariatur. Excepteur sint occaecat cupidatat non proident, sunt in culpa qui officia deserunt mollit anim id est laborum.";
        const MAX_LINES: i32 = 2;

        let layout = layout(
            TEXT,
            TextLayoutOptions {
                text_wrapping: TextWrapping::Wrap,
                text_trimming: Some(trimming),
                max_width: 180.0,
                max_lines: MAX_LINES,
                ..options(black())
            },
        );

        assert_eq!(layout.text_lines().len() as i32, MAX_LINES, "row {row}");

        let last_line = &layout.text_lines()[layout.text_lines().len() - 1];
        let formatted_line_text: String = last_line.text_runs().iter().map(|r| r.text().to_string_lossy()).collect();

        assert!(last_line.has_collapsed(), "The wrapped line cut off by MaxLines must be collapsed.");
        assert_eq!(formatted_line_text.chars().last(), Some('\u{2026}'), "row {row}");

        layout.dispose();
    }
}

#[test]
fn should_produce_fixed_height_lines() {
    let _app = start();

    let layout = layout(MULTI_LINE_TEXT, TextLayoutOptions { line_height: 50.0, ..options(black()) });

    for line in layout.text_lines() {
        assert_eq!(line.height(), 50.0);
    }
}

const TEXT: &str = "日本でTest一番読まれている英字新聞・ジャパンタイムズが発信する国内外ニュースと、様々なジャンルの特集記事。";

#[test]
#[ignore = "Only used for profiling."]
fn should_wrap() {
    let _app = start();

    for _ in 0..2000 {
        let _layout =
            layout(TEXT, TextLayoutOptions { text_wrapping: TextWrapping::Wrap, max_width: 50.0, ..options(black()) });
    }
}

#[test]
fn should_process_multiple_new_lines_properly() {
    let _app = start();

    let text = "123\r\n\r\n456\r\n\r\n";
    let layout = layout(text, options(black()));

    assert_eq!(layout.text_lines().len(), 5);

    assert_eq!(layout.text_lines()[0].text_runs()[0].text().to_string_lossy(), "123\r\n");
    assert_eq!(layout.text_lines()[1].text_runs()[0].text().to_string_lossy(), "\r\n");
    assert_eq!(layout.text_lines()[2].text_runs()[0].text().to_string_lossy(), "456\r\n");
    assert_eq!(layout.text_lines()[3].text_runs()[0].text().to_string_lossy(), "\r\n");
}

#[test]
fn should_wrap_min_one_character_every_line() {
    let _app = start();

    let layout =
        layout(SINGLE_LINE_TEXT, TextLayoutOptions { text_wrapping: TextWrapping::Wrap, max_width: 3.0, ..options(black()) });

    //every character should be new line as there not enough space for even one character
    assert_eq!(layout.text_lines().len(), SINGLE_LINE_TEXT.len());
}

#[test]
fn should_hit_test_text_range_right_to_left() {
    let _app = start();

    const START: i32 = 0;
    const LENGTH: i32 = 10;

    let layout_ = layout(RIGHT_TO_LEFT_TEXT, options(black()));

    let selected_text = layout(&substring(RIGHT_TO_LEFT_TEXT, START, LENGTH), options(black()));

    let rects = layout_.hit_test_text_range(START, LENGTH);

    assert_eq!(rects.len(), 1);

    let selected_rect = rects[0];

    assert_equal_precision(selected_text.width_including_trailing_whitespace(), selected_rect.width, 2);
}

#[test]
fn should_hit_test_text_range_bi_di() {
    const TEXT: &str = "זה כיףabcDEFזה כיף";

    let _app = start();

    let layout = layout(TEXT, options(black()));

    let text_line = &layout.text_lines()[0];

    let start = text_line.get_distance_from_character_hit(CharacterHit::with_trailing_length(5, 1));

    let end = text_line.get_distance_from_character_hit(CharacterHit::with_trailing_length(6, 1));

    let rects = layout.hit_test_text_range(0, 7);

    assert_eq!(rects.len(), 1);

    let expected = rects[0];

    assert_eq!(start, expected.left());
    assert_eq!(end, expected.right());
}

#[test]
fn should_hit_test_text_range() {
    let _app = start();

    let layout = layout(SINGLE_LINE_TEXT, options(black()));

    let line_rects = layout.hit_test_text_range(0, SINGLE_LINE_TEXT.len() as i32);

    assert_eq!(line_rects.len(), layout.text_lines().len());

    for i in 0..layout.text_lines().len() {
        let text_line = &layout.text_lines()[i];
        let rect = line_rects[i];

        assert_eq!(rect.width, text_line.width_including_trailing_whitespace());
    }

    let rects: Vec<f64> = layout
        .text_lines()
        .iter()
        .flat_map(|x| shaped_runs(&**x))
        .flat_map(|x| x.shaped_buffer().glyph_infos().iter().map(|glyph| glyph.glyph_advance).collect::<Vec<_>>())
        .collect();

    for i in 0..SINGLE_LINE_TEXT.len() {
        let mut j = 1;

        while i + j < SINGLE_LINE_TEXT.len() {
            let expected: f64 = rects[i..i + j].iter().sum();
            let actual: f64 = layout.hit_test_text_range(i as i32, j as i32).iter().map(|x| x.width).sum();

            assert_eq!(actual, expected, "range ({i}, {j})");

            j += 1;
        }
    }
}

#[test]
fn should_wrap_right_to_left() {
    const TEXT: &str =
        "يَجِبُ عَلَى الإنْسَانِ أن يَكُونَ أمِيْنَاً وَصَادِقَاً مَعَ نَفْسِهِ وَمَعَ أَهْلِهِ وَجِيْرَانِهِ وَأَنْ يَبْذُلَ كُلَّ جُهْدٍ فِي إِعْلاءِ شَأْنِ الوَطَنِ وَأَنْ يَعْمَلَ عَلَى مَا يَجْلِبُ السَّعَادَةَ لِلنَّاسِ . ولَن يَتِمَّ لَهُ ذلِك إِلا بِأَنْ يُقَدِّمَ المَنْفَعَةَ العَامَّةَ عَلَى المَنْفَعَةِ الخَاصَّةِ وَهذَا مِثَالٌ لِلتَّضْحِيَةِ .";

    let _app = start();

    let mut max_width = 366;

    while max_width < 900 {
        let layout = layout(
            TEXT,
            TextLayoutOptions {
                text_wrapping: TextWrapping::Wrap,
                flow_direction: FlowDirection::RightToLeft,
                max_width: max_width as f64,
                ..options(black())
            },
        );

        for text_line in layout.text_lines() {
            assert!(text_line.width() <= max_width as f64, "max width {max_width}");

            let mut runs = shaped_runs(&**text_line);

            runs.sort_by_key(|x| get_start_char_index(x.text()));

            let actual: String = runs.iter().map(|x| x.text().to_string_lossy()).collect();

            let expected = substring(TEXT, text_line.first_text_source_index(), text_line.length());

            assert_eq!(actual, expected, "max width {max_width}");
        }

        max_width += 33;
    }
}

#[test]
fn should_layout_empty_string() {
    let _app = start();

    let layout = layout("", options(black()));

    assert!(layout.height() > 0.0);
}

#[test]
fn should_hit_test_point_right_to_left() {
    let _app = start();

    let text = "אאא AAA";

    let layout = layout(text, TextLayoutOptions { flow_direction: FlowDirection::RightToLeft, ..options(black()) });

    let text_runs = layout.text_lines()[0].text_runs();

    let first_run = shaped(&text_runs[0]);

    let mut hit = layout.hit_test_point(Point::default());

    assert_eq!(hit.text_position(), 4);

    let first_run_offset = get_start_char_index(first_run.text());
    let mut current_x = 0.0;

    let first_run_glyphs = first_run.glyph_run().glyph_infos();

    for i in 0..first_run_glyphs.count() {
        let cluster = first_run_glyphs.borrow()[i].glyph_cluster + first_run_offset;
        let advance = first_run_glyphs.borrow()[i].glyph_advance;

        hit = layout.hit_test_point(Point::new(current_x, 0.0));

        assert_eq!(hit.text_position(), cluster, "first run, glyph {i}");

        let hit_range = layout.hit_test_text_range(hit.text_position(), 1);

        let distance = hit_range[0].left();

        assert_equal_precision(current_x, distance, 2);

        current_x += advance;
    }

    let second_run = shaped(&text_runs[1]);

    hit = layout.hit_test_point(Point::new(first_run.size().width, 0.0));

    assert_eq!(hit.text_position(), 7);

    hit = layout.hit_test_point(Point::new(layout.text_lines()[0].width_including_trailing_whitespace(), 0.0));

    assert_eq!(hit.text_position(), 0);

    let second_run_offset = get_start_char_index(second_run.text());
    current_x = first_run.size().width + 0.5;

    let second_run_glyphs = second_run.glyph_run().glyph_infos();

    for i in 0..second_run_glyphs.count() {
        let cluster = second_run_glyphs.borrow()[i].glyph_cluster + second_run_offset;
        let advance = second_run_glyphs.borrow()[i].glyph_advance;

        hit = layout.hit_test_point(Point::new(current_x, 0.0));

        assert_eq!(hit.character_hit().first_character_index(), cluster, "second run, glyph {i}");

        let hit_range =
            layout.hit_test_text_range(hit.character_hit().first_character_index(), hit.character_hit().trailing_length());

        let distance = hit_range[0].left() + 0.5;

        assert_equal_precision(current_x, distance, 2);

        current_x += advance;
    }
}

#[test]
fn should_get_character_hit_from_distance_rtl() {
    let _app = start();

    let text = "أَبْجَدِيَّة عَرَبِيَّة";

    let layout = layout(text, options(black()));

    let text_line = &layout.text_lines()[0];

    // Runs come in visual order, so TextRuns[0] is the leftmost one - the last word of
    // this right-to-left line. Its glyph clusters are relative to its own text, so the
    // run's start has to be added to compare them with a text source index.
    let text_runs = text_line.text_runs();

    let first_run = shaped(&text_runs[0]);

    let first_cluster = get_start_char_index(first_run.text()) + first_run.shaped_buffer().get(0).glyph_cluster;

    let character_hit = text_line.get_character_hit_from_distance(0.0);

    assert_eq!(character_hit.first_character_index(), first_cluster);

    assert_eq!(character_hit.first_character_index() + character_hit.trailing_length(), utf16_len(text));

    let distance = text_line.get_distance_from_character_hit(character_hit);

    assert_eq!(distance, 0.0);

    let distance = text_line.get_distance_from_character_hit(CharacterHit::new(character_hit.first_character_index()));

    let first_advance = first_run.shaped_buffer().get(0).glyph_advance;

    assert_equal_precision(first_advance, distance, 5);

    let rect = layout.hit_test_text_position(22);

    assert_equal_precision(first_advance, rect.left(), 5);

    let rect = layout.hit_test_text_position(23);

    assert_equal_precision(0.0, rect.left(), 5);
}

#[test]
fn should_get_character_hit_from_distance_rtl_with_text_styles() {
    let _app = start();

    let text = "أَبْجَدِيَّة عَرَبِيَّة";

    let units: Vec<u16> = text.encode_utf16().collect();

    let mut i = 0;

    let mut grapheme_enumerator = GraphemeEnumerator::new(&units);

    while let Some(grapheme) = grapheme_enumerator.move_next() {
        let text_style_overrides: Option<Rc<[ValueSpan<Rc<dyn TextRunProperties>>]>> = Some(Rc::from(vec![
            ValueSpan::new(i, grapheme.length() as i32, run_properties(Typeface::default_typeface(), 12.0, Some(Brushes::red()))),
        ]));

        i += grapheme.length() as i32;

        let layout = layout(text, TextLayoutOptions { text_style_overrides, ..options(black()) });

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
            let raw_clusters: Vec<i32> = run.shaped_buffer().glyph_infos().iter().map(|glyph| glyph.glyph_cluster).collect();

            let run_start = run_starts
                .iter()
                .find(|(text_run, _)| std::ptr::addr_eq(Rc::as_ptr(text_run), Rc::as_ptr(run)))
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

        let glyph_advances: Vec<f64> = shaped_runs
            .iter()
            .flat_map(|x| x.shaped_buffer().glyph_infos().iter().map(|glyph| glyph.glyph_advance).collect::<Vec<_>>())
            .collect();

        let mut current_x = 0.0;

        let mut cluster = utf16_len(text);

        for j in 0..clusters.len() - 1 {
            let glyph_advance = glyph_advances[j];

            let character_hit = text_line.get_character_hit_from_distance(current_x);

            assert!(
                cluster == character_hit.first_character_index() + character_hit.trailing_length(),
                "grapheme={}, j={j}, cluster={cluster}, hit={}+{}, currentX={current_x}, textLen={}, runs={}, clusters=[{}]",
                i - grapheme.length() as i32,
                character_hit.first_character_index(),
                character_hit.trailing_length(),
                utf16_len(text),
                shaped_runs.len(),
                clusters.iter().map(|c| c.to_string()).collect::<Vec<_>>().join(",")
            );

            let distance = text_line.get_distance_from_character_hit(CharacterHit::new(cluster));

            assert_equal_precision(current_x, distance, 5);

            current_x += glyph_advance;

            if glyph_advance > 0.0 {
                cluster = clusters[j];
            }
        }
    }
}

#[test]
fn hit_test_text_range_range_valid_length() {
    let cases = [
        ("mgfg🧐df f sdf", "g🧐d", 20.0, 40.0),
        ("وه. وقد تعرض لانتقادات", "دات", 5.0, 30.0),
        ("وه. وقد تعرض لانتقادات", "تعرض", 20.0, 50.0),
        // The spaces of an Arabic line are drawn with the primary font rather than the Arabic
        // fallback, which is wider at this size - hence the bands sit above where they used to.
        (" علمية 😱ومضللة ،", " علمية 😱ومضللة ،", 80.0, 120.0),
        ("في عام 2018 ، رفعت ل", "في عام 2018 ، رفعت ل", 120.0, 150.0),
    ];

    for (text, text_to_select, min_width, max_width) in cases {
        let _app = start();

        let layout = layout(text, options(black()));
        let start = index_of(text, text_to_select);
        let selection_rectangles = layout.hit_test_text_range(start, utf16_len(text_to_select));
        assert_eq!(selection_rectangles.len(), 1, "{text:?}");
        let rect = selection_rectangles[0];
        assert_in_range(rect.width, min_width, max_width);
    }
}

#[test]
fn should_hit_test_text_range_between_runs() {
    let cases = [
        ("012🧐210", 2, 4, FlowDirection::LeftToRight, "14.40234375,40.8046875"),
        ("210🧐012", 2, 4, FlowDirection::RightToLeft, "0,7.201171875;21.603515625,33.603515625;48.005859375,55.20703125"),
        ("שנב🧐שנב", 2, 4, FlowDirection::LeftToRight, "11.268,38.208"),
        ("שנב🧐שנב", 2, 4, FlowDirection::RightToLeft, "11.268,38.208"),
    ];

    for (text, start, length, flow_direction, expected) in cases {
        let _app = self::start();

        let expected_rects: Vec<Rect> = expected
            .split(';')
            .map(|x| {
                let start_end: Vec<&str> = x.split(',').collect();

                let start: f64 = start_end[0].parse().unwrap();

                let end: f64 = start_end[1].parse().unwrap();

                Rect::new(start, 0.0, end - start, 0.0)
            })
            .collect();

        let text_layout = layout(text, TextLayoutOptions { flow_direction, ..options(black()) });

        let rects = text_layout.hit_test_text_range(start, length);

        assert_eq!(rects.len(), expected_rects.len(), "{text:?} {flow_direction:?}: {rects:?}");

        let _end_x = text_layout.text_lines()[0].get_distance_from_character_hit(CharacterHit::new(2));
        let _start_x = text_layout.text_lines()[0].get_distance_from_character_hit(CharacterHit::with_trailing_length(5, 1));

        for i in 0..expected_rects.len() {
            let expected_rect = expected_rects[i];

            assert_equal_precision(expected_rect.left(), rects[i].left(), 2);

            assert_equal_precision(expected_rect.right(), rects[i].right(), 2);
        }
    }
}

#[test]
fn should_hit_test_text_range_with_line_breaks() {
    let _app = start();

    // `Environment.NewLine`.
    let new_line = if cfg!(windows) { "\r\n" } else { "\n" };

    let before_linebreak = "Line before linebreak";
    let after_linebreak = "Line after linebreak";
    let text = format!("{before_linebreak}{new_line}{new_line}{after_linebreak}");

    let text_layout = layout(&text, options(black()));

    let end = utf16_len(&text) - utf16_len(after_linebreak) + 1;

    let rects = text_layout.hit_test_text_range(0, end);

    assert_eq!(rects.len(), 3);

    let end_x = text_layout.text_lines()[2].get_distance_from_character_hit(CharacterHit::new(end));

    //First character should be covered
    assert_equal_precision(7.201171875, end_x, 2);
}

#[test]
fn should_hit_test_text_position_end_of_line_rtl() {
    let text = "גש\r\n";

    let _app = start();

    let text_layout = layout(text, TextLayoutOptions { flow_direction: FlowDirection::RightToLeft, ..options(black()) });

    let rect = text_layout.hit_test_text_position(utf16_len(text));

    assert_eq!(rect.top(), 16.32);
}

#[test]
#[cfg_attr(not(windows), ignore = "Font only available on Windows")]
fn should_handle_text_style_with_ligature() {
    let _app = start();

    let text = "fi";

    let typeface = Typeface::from_name("Calibri");

    let text_layout = TextLayout::new(
        text,
        typeface.clone(),
        TextLayoutOptions {
            text_style_overrides: Some(Rc::from(vec![ValueSpan::new(
                1,
                1,
                run_properties(typeface, GenericTextRunProperties::DEFAULT_FONT_RENDERING_EM_SIZE, Some(Brushes::white())),
            )])),
            ..options(black())
        },
    );

    // `Assert.NotNull(textLayout)`: a Rust value cannot be null; the test is that the
    // layout is created.
    drop(text_layout);
}

#[test]
fn should_measure_text_layout_symbol_with_and_width_including_trailing_whitespace() {
    const SYMBOLS_FONT: &str = "resm:FerroUI.Skia.UnitTests.Fonts?assembly=ferroui-skia#Symbols";

    let _app = start();

    let text_layout = TextLayout::new("\u{e971}", Typeface::from_name(SYMBOLS_FONT), options(Brushes::white()));

    assert_eq!(Size::new(text_layout.width(), text_layout.height()), Size::new(12.0, 12.0));
    assert_eq!(text_layout.width_including_trailing_whitespace(), 12.0);
}

#[test]
fn should_wrap_with_line_end() {
    let _app = start();

    let default_properties = Rc::new(GenericTextRunProperties::with_all(
        Typeface::default_typeface(),
        72.0,
        None,
        Some(black()),
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
        Rc::new(SingleBufferTextSource::new("01", default_properties, true)),
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

#[test]
fn should_measure_text_layout_symbol_with_and_width_including_trailing_whitespace_and_min_text_width() {
    let _app = start();

    const MONOSPACE_FONT: &str = "resm:FerroUI.Skia.UnitTests.Assets?assembly=ferroui-skia#Noto Mono";

    let type_face = Typeface::from_name(MONOSPACE_FONT);

    let _glyph_typeface = type_face.glyph_typeface();

    let white = || options(Brushes::white());

    let text_layout0 = TextLayout::new("aaaa", type_face.clone(), white());
    assert_eq!(text_layout0.width(), text_layout0.width_including_trailing_whitespace());

    let text_layout01 = TextLayout::new("a a", type_face.clone(), white());
    let text_layout1 = TextLayout::new("a a ", type_face.clone(), white());
    assert_eq!(
        Size::new(text_layout1.width_including_trailing_whitespace(), text_layout1.height()),
        Size::new(text_layout0.width(), text_layout0.height())
    );
    assert_eq!(text_layout1.width_including_trailing_whitespace(), text_layout0.width_including_trailing_whitespace());

    let text_layout2 = TextLayout::new(" aa ", type_face.clone(), white());
    assert_eq!(Size::new(text_layout2.width(), text_layout2.height()), Size::new(text_layout1.width(), text_layout1.height()));
    assert_eq!(text_layout2.width_including_trailing_whitespace(), text_layout0.width_including_trailing_whitespace());
    assert_eq!(text_layout2.width(), text_layout01.width());

    let text_layout3 = TextLayout::new("    ", type_face, white());
    assert_eq!(Size::new(text_layout3.width(), text_layout3.height()), Size::new(0.0, text_layout0.height()));
    assert_eq!(text_layout3.width_including_trailing_whitespace(), text_layout0.width_including_trailing_whitespace());
    assert_eq!(text_layout3.width(), 0.0);
}

/// `new GenericTextRunProperties(Typeface.Default)`.
fn default_run_properties() -> Rc<GenericTextRunProperties> {
    Rc::new(GenericTextRunProperties::new(Typeface::default_typeface()))
}

/// `formatter.FormatLine(textSource, 0, double.PositiveInfinity, new GenericTextParagraphProperties(defaultProperties))`.
fn format_line(
    formatter: &TextFormatterImpl,
    text_source: &dyn ITextSource,
    default_properties: Rc<dyn TextRunProperties>,
) -> Option<Rc<dyn TextLine>> {
    let paragraph_properties: Rc<dyn TextParagraphProperties> = Rc::new(GenericTextParagraphProperties::new(default_properties));

    formatter.format_line(text_source, 0, f64::INFINITY, &paragraph_properties, None)
}

/// The left and justified layouts of the justification tests.
fn left_and_justified(text: &str, max_width: f64) -> (TextLayout, TextLayout) {
    let foreground = black();

    let left = layout(
        text,
        TextLayoutOptions {
            text_alignment: TextAlignment::Left,
            text_wrapping: TextWrapping::Wrap,
            max_width,
            ..options(foreground.clone())
        },
    );

    let justified = layout(
        text,
        TextLayoutOptions {
            text_alignment: TextAlignment::Justify,
            text_wrapping: TextWrapping::Wrap,
            max_width,
            ..options(foreground)
        },
    );

    (left, justified)
}

#[test]
fn inter_word_justification_does_not_stretch_last_cjk_glyph() {
    let _app = start();

    // Pure CJK (Han) has no inter-word spaces; UAX#14 LB31 yields a break opportunity
    // between essentially every ideograph, so justification distributes space
    // inter-character. Justifying to a width wider than the shaped line must widen
    // interior glyphs (including the first) but leave the final visible glyph's advance
    // untouched - otherwise the last ideograph is not flush to the line edge. Drives
    // InterWordJustification directly with an explicit target width to avoid the
    // widest-line / last-line behaviour of the full TextLayout pipeline.
    const TEXT: &str = "一二三四五";

    let default_properties = default_run_properties();
    let text_source = SingleBufferTextSource::new(TEXT, default_properties.clone(), false);
    let formatter = TextFormatterImpl::new();

    let text_line = format_line(&formatter, &text_source, default_properties);

    assert!(text_line.is_some());

    let text_line = text_line.unwrap();

    let natural_width = text_line.width_including_trailing_whitespace();
    let before = glyph_advances(&*text_line);

    assert!(before.len() >= 2);

    let target_width = natural_width + 40.0;

    text_line.justify(&InterWordJustification::new(target_width));

    let after = glyph_advances(&*text_line);

    assert_eq!(after.len(), before.len());

    // The line is stretched to the requested width.
    assert_equal_precision(target_width, text_line.width_including_trailing_whitespace(), 3);

    // The last visible glyph keeps its original advance (no trailing overshoot).
    assert_equal_precision(before[before.len() - 1], after[after.len() - 1], 3);

    // The first glyph participates in justification (the leading gap is widened).
    assert_greater_than(after[0], before[0], "The first glyph should be widened");
}

#[test]
fn should_justify_wrapped_cjk_line_to_max_width() {
    let _app = start();

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
    assert_equal_precision(MAX_WIDTH, justified_line.width_including_trailing_whitespace(), 3);

    // The last visible glyph is unchanged; the first glyph is widened.
    assert_equal_precision(before[before.len() - 1], after[after.len() - 1], 3);
    assert_greater_than(after[0], before[0], "The first glyph should be widened");
}

#[test]
fn does_not_justify_last_line_of_wrapped_paragraph() {
    let _app = start();

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
    assert_equal_precision(
        last_left.width_including_trailing_whitespace(),
        last_justified.width_including_trailing_whitespace(),
        3,
    );

    // A preceding wrapped line is still justified to the margin.
    assert_equal_precision(MAX_WIDTH, justified.text_lines()[0].width_including_trailing_whitespace(), 3);
}

#[test]
fn does_not_justify_line_ending_in_hard_break() {
    let _app = start();

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
    assert_equal_precision(
        hard_break_left.width_including_trailing_whitespace(),
        hard_break_justified.width_including_trailing_whitespace(),
        3,
    );

    // The following width-wrapped, non-last line is justified to the margin.
    assert_equal_precision(MAX_WIDTH, justified.text_lines()[1].width_including_trailing_whitespace(), 3);
}

#[test]
fn justify_does_not_mutate_shared_shaped_buffer() {
    let _app = start();

    // A TextRunCache keeps the same ShapedTextRun (and its pooled glyph storage) alive
    // across layouts. Justification must copy-on-write rather than mutate that shared
    // buffer in place. Simulate the cache's reference with AddRef and assert the
    // original buffer is untouched while the line's run is replaced with a widened copy.
    const TEXT: &str = "一二三四五";

    let default_properties = default_run_properties();
    let text_source = SingleBufferTextSource::new(TEXT, default_properties.clone(), false);
    let formatter = TextFormatterImpl::new();

    let text_line = format_line(&formatter, &text_source, default_properties);

    assert!(text_line.is_some());

    let text_line = text_line.unwrap();

    let original_run = shaped_runs(&*text_line).remove(0);
    let original_buffer = original_run.shaped_buffer().clone();
    let original_first_advance = original_buffer.get(0).glyph_advance;

    // Second owner (stands in for the TextRunCache) so the buffer survives the run's
    // disposal during justification.
    original_run.add_ref();

    text_line.justify(&InterWordJustification::new(original_run.size().width + 40.0));

    // The shared buffer is not mutated...
    assert_equal_precision(original_first_advance, original_buffer.get(0).glyph_advance, 5);

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
    let _app = start();

    // Draw and hit-test resolve runs through the bidi-reordered IndexedTextRun list, so
    // after justification replaces a run its IndexedTextRun must point at the
    // replacement. GetTextBounds walks the indexed runs; its total must match the
    // justified line width, not the pre-justification width.
    let text = "一".repeat(40);
    const MAX_WIDTH: f64 = 80.0;

    let justified = layout(
        &text,
        TextLayoutOptions {
            text_alignment: TextAlignment::Justify,
            text_wrapping: TextWrapping::Wrap,
            max_width: MAX_WIDTH,
            ..options(black())
        },
    );

    assert!(justified.text_lines().len() >= 2);

    let line = &justified.text_lines()[0];

    let bounds = line.get_text_bounds(line.first_text_source_index(), line.length());

    assert_equal_precision(line.width_including_trailing_whitespace(), bounds.iter().map(|b| b.rectangle().width).sum(), 2);
    assert_equal_precision(MAX_WIDTH, bounds.iter().map(|b| b.rectangle().width).sum(), 2);
}

#[test]
fn justify_distributes_across_multiple_runs() {
    let _app = start();

    // Two shaped runs on one line (split by a font-size change). Each must receive its
    // own break opportunities: the pre-fix apply loop drained the whole queue against
    // the first run, leaving later runs unjustified.
    const TEXT: &str = "一二三四五六";

    let first: Rc<dyn TextRunProperties> = Rc::new(GenericTextRunProperties::with_font_size(Typeface::default_typeface(), 20.0));
    let second: Rc<dyn TextRunProperties> = Rc::new(GenericTextRunProperties::with_font_size(Typeface::default_typeface(), 12.0));
    let text_source = SplitStyleTextSource::new(TEXT, 3, first.clone(), second);
    let formatter = TextFormatterImpl::new();

    let text_line = format_line(&formatter, &text_source, first);

    assert!(text_line.is_some());

    let text_line = text_line.unwrap();

    let runs = shaped_runs(&*text_line);

    // Confirm the line really is multi-run, otherwise the test proves nothing.
    assert!(runs.len() >= 2);

    let before_widths: Vec<f64> = runs.iter().map(|r| r.size().width).collect();

    text_line.justify(&InterWordJustification::new(text_line.width_including_trailing_whitespace() + 60.0));

    let after_runs = shaped_runs(&*text_line);

    assert_eq!(after_runs.len(), runs.len());

    // Every shaped run participated in justification, not just the first.
    for i in 0..after_runs.len() {
        assert_greater_than(after_runs[i].size().width, before_widths[i], &format!("Run {i} should be widened"));
    }
}

#[test]
fn justify_distributes_across_a_word_gap_at_a_run_boundary() {
    let _app = start();

    // The emoji needs a fallback font, so this line's word gaps sit next to a run
    // boundary. Break opportunities are collected run by run, and the break the
    // enumerator always reports at the end of the text it is given is discarded as an
    // artifact - so a real word gap that coincides with a run boundary yields no
    // opportunity at all and never widens.
    const TEXT: &str = "abc \u{1F600} def";

    let default_properties = default_run_properties();
    let text_source = SingleBufferTextSource::new(TEXT, default_properties.clone(), false);
    let formatter = TextFormatterImpl::new();

    let format = || format_line(&formatter, &text_source, default_properties.clone()).unwrap();

    fn gap(line: &dyn TextLine, space_index: i32) -> f64 {
        line.get_distance_from_character_hit(CharacterHit::new(space_index + 1))
            - line.get_distance_from_character_hit(CharacterHit::new(space_index))
    }

    let reference = format();

    let first_gap_before = gap(&*reference, 3);
    let second_gap_before = gap(&*reference, 6);

    let text_line = format();

    // Confirm the line really is multi-run, otherwise the test proves nothing.
    assert_greater_than(text_line.text_runs().len() as f64, 1.0, "The emoji should force a fallback run");

    text_line.justify(&InterWordJustification::new(text_line.width_including_trailing_whitespace() + 40.0));

    let first_gap = gap(&*text_line, 3);
    let second_gap = gap(&*text_line, 6);

    assert_greater_than(first_gap, first_gap_before, "The first word gap should be widened");
    assert_greater_than(second_gap, second_gap_before, "The second word gap should be widened");

    // Both gaps are break opportunities, so they take an equal share of the added width.
    assert_equal_precision(first_gap - first_gap_before, second_gap - second_gap_before, 3);
}

#[test]
fn justify_does_not_space_trailing_whitespace() {
    let cases = [
        "aa bb   ",                      // trailing ASCII spaces
        "一二\u{3000}\u{3000}\u{3000}", // trailing ideographic (U+3000) spaces
    ];

    for text in cases {
        let _app = start();

        // Trailing whitespace (which sits before a wrap point or hard break) must never
        // receive justification space; the full distributed amount lands in the visible
        // region instead. This is guaranteed by the LineBreakEnumerator (it emits no
        // non-required break inside trailing whitespace), so no explicit guard is needed in
        // InterWordJustification - this test locks that invariant. Width excludes trailing
        // whitespace, so it must grow by the entire distributed space; if trailing
        // whitespace absorbed part of it, Width would grow less.
        const EXTRA: f64 = 40.0;

        let default_properties = default_run_properties();
        let text_source = SingleBufferTextSource::new(text, default_properties.clone(), false);
        let formatter = TextFormatterImpl::new();

        let text_line = format_line(&formatter, &text_source, default_properties);

        assert!(text_line.is_some(), "{text:?}");

        let text_line = text_line.unwrap();

        // Confirm the line actually carries trailing whitespace, otherwise the test proves nothing.
        assert!(text_line.trailing_whitespace_length() >= 2, "{text:?}");
        assert_greater_than(
            text_line.width_including_trailing_whitespace(),
            text_line.width(),
            "The line must have measurable trailing whitespace",
        );

        let width_before = text_line.width();

        text_line.justify(&InterWordJustification::new(text_line.width() + EXTRA));

        assert_equal_precision(width_before + EXTRA, text_line.width(), 2);
    }
}

#[test]
fn should_justify_wrapped_latin_line() {
    // The classic inter-word case. A wrapped non-last Latin line fills to the margin; the
    // last line stays start-aligned.
    let _app = start();

    assert_wrapped_non_last_line_fills_to_max_width(
        "the quick brown fox jumps over the lazy dog and then runs away quite quickly today",
        140.0,
    );
}

// Korean Hangul justifies inter-syllable via the same code path as CJK (LB26/27 bind within
// a syllable, LB31 breaks between syllable blocks). A live Korean advance test is not
// possible here because the test harness's fallback fonts render Hangul with zero advance
// (no Korean font installed), so the CJK tests stand in for it.

#[test]
fn does_not_justify_latin_line_with_trailing_spaces_before_hard_break() {
    let _app = start();

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
    assert_equal_precision(
        line0_left.width_including_trailing_whitespace(),
        line0_justified.width_including_trailing_whitespace(),
        2,
    );

    // A following width-wrapped, non-last line fills its visible content to the margin.
    assert_equal_precision(MAX_WIDTH, justified.text_lines()[1].width(), 2);
}

#[test]
fn justify_distributes_across_latin_and_cjk() {
    let _app = start();

    // A mixed Latin+CJK line distributes space across both the inter-word gap and the
    // inter-ideograph gaps (the line reaches the target width) without stretching the
    // last visible glyph.
    const TEXT: &str = "ab 日本語";

    let default_properties = default_run_properties();
    let text_source = SingleBufferTextSource::new(TEXT, default_properties.clone(), false);
    let formatter = TextFormatterImpl::new();

    let text_line = format_line(&formatter, &text_source, default_properties);

    assert!(text_line.is_some());

    let text_line = text_line.unwrap();

    let last_glyph_before = last_glyph_advance(&*text_line);

    let target = text_line.width_including_trailing_whitespace() + 40.0;

    text_line.justify(&InterWordJustification::new(target));

    // Space was distributed across the mixed content (the line reaches the target)...
    assert_equal_precision(target, text_line.width_including_trailing_whitespace(), 2);

    // ...but the last visible glyph is not stretched.
    assert_equal_precision(last_glyph_before, last_glyph_advance(&*text_line), 3);
}

#[test]
fn justify_arabic_does_not_corrupt_shared_buffer() {
    let _app = start();

    // Arabic is cursive and right-to-left (ligated, not one-char-per-cluster), so
    // justification exercises the copy-on-write path on a non-trivial run. It must run
    // without corrupting a cache-shared buffer.
    const TEXT: &str = "مرحبا بالعالم";

    let default_properties = default_run_properties();
    let text_source = SingleBufferTextSource::new(TEXT, default_properties.clone(), false);
    let formatter = TextFormatterImpl::new();

    let text_line = format_line(&formatter, &text_source, default_properties);

    assert!(text_line.is_some());

    let text_line = text_line.unwrap();

    let original_run = shaped_runs(&*text_line).remove(0);
    let original_buffer = original_run.shaped_buffer().clone();
    let original_advances: Vec<f64> = (0..original_buffer.length()).map(|i| original_buffer.get(i).glyph_advance).collect();

    // Second owner (stands in for a TextRunCache) so the buffer survives run disposal.
    original_run.add_ref();

    let width_before = text_line.width_including_trailing_whitespace();

    text_line.justify(&InterWordJustification::new(width_before + 40.0));

    // Justification happened (the inter-word gap was widened)...
    assert_greater_than(
        text_line.width_including_trailing_whitespace(),
        width_before,
        "Justifying an Arabic line should widen it",
    );

    // ...and the shared buffer was not mutated in place.
    for i in 0..original_buffer.length() {
        assert_equal_precision(original_advances[i], original_buffer.get(i).glyph_advance, 5);
    }

    original_run.dispose();
}

fn assert_wrapped_non_last_line_fills_to_max_width(text: &str, max_width: f64) {
    let (left, justified) = left_and_justified(text, max_width);

    assert!(justified.text_lines().len() >= 2);

    let left_line = &left.text_lines()[0];
    let justified_line = &justified.text_lines()[0];

    assert_greater_than(max_width, left_line.width(), "The unjustified line's visible content must be narrower than MaxWidth");

    // The non-last wrapped line's visible content fills to the margin. (Width excludes any
    // trailing whitespace, which hangs past the margin.)
    assert_equal_precision(max_width, justified_line.width(), 2);

    // The last line stays start-aligned.
    let last_left = &left.text_lines()[left.text_lines().len() - 1];
    let last_justified = &justified.text_lines()[justified.text_lines().len() - 1];

    assert_equal_precision(last_left.width_including_trailing_whitespace(), last_justified.width_including_trailing_whitespace(), 2);
}

fn last_glyph_advance(line: &dyn TextLine) -> f64 {
    let runs = shaped_runs(line);

    let glyphs = runs.last().unwrap().glyph_run().glyph_infos();

    let glyphs = glyphs.borrow();

    glyphs[glyphs.len() - 1].glyph_advance
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
        Self { text: ReadOnlyMemory::from_str(text), split_at, first, second }
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

#[test]
fn width_excludes_only_the_lines_true_trailing_whitespace() {
    let _app = start();

    // Two runs split by a font-size change. The FIRST (interior) run also ends in a
    // space of its own ("foo "), but that space is followed by more visible content
    // ("bar") in the next run, so it is NOT trailing whitespace for the line as a
    // whole - only the space at the true end of the line is. A reference line without
    // any trailing space isolates exactly how much Width should differ.
    let first: Rc<dyn TextRunProperties> = Rc::new(GenericTextRunProperties::with_font_size(Typeface::default_typeface(), 20.0));
    let second: Rc<dyn TextRunProperties> = Rc::new(GenericTextRunProperties::with_font_size(Typeface::default_typeface(), 14.0));
    let formatter = TextFormatterImpl::new();

    let reference = format_line(&formatter, &SplitStyleTextSource::new("foo bar", 4, first.clone(), second.clone()), first.clone());

    let with_trailing_space = format_line(&formatter, &SplitStyleTextSource::new("foo bar ", 4, first.clone(), second), first);

    assert!(reference.is_some());
    assert!(with_trailing_space.is_some());

    let reference = reference.unwrap();
    let with_trailing_space = with_trailing_space.unwrap();

    // Confirm both lines are genuinely multi-run, and the reference truly has no
    // trailing whitespace, otherwise the comparison proves nothing.
    assert!(shaped_runs(&*reference).len() >= 2);
    assert_eq!(reference.trailing_whitespace_length(), 0);

    // Only the one added trailing space is excluded - not that space AND "foo "'s own
    // interior trailing space too.
    assert_eq!(with_trailing_space.trailing_whitespace_length(), 1);
    assert_equal_precision(reference.width(), with_trailing_space.width(), 3);
}
