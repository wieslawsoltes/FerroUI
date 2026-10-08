//! Port of upstream's `Media/TextFormatting/TextFormatterTests.cs` of the
//! Skia unit tests.
//!
//! `start()` (upstream's `public static Start()`) and `ListTextSource` are
//! used by the other suites of the folder, as upstream.

use crate::unit_tests::media::text_formatting::{MultiBufferTextSource, SingleBufferTextSource};
use crate::unit_tests::media::CustomFontManagerImpl;
use crate::unit_tests::mock_platform_render_interface;
use crate::PlatformRenderInterface;
use ferroui_base::media::text_formatting::unicode::LineBreakEnumerator;
use ferroui_base::media::text_formatting::{
    DrawableTextRun, FormattedTextSource, GenericTextParagraphProperties, GenericTextRunProperties, ITextDrawingSink,
    ITextSource, ShapedTextRun, TextCharacters, TextEndOfLine, TextEndOfParagraph, TextFormatter, TextFormatterImpl,
    TextLine, TextLineBreak, TextParagraphProperties, TextRun, TextRunProperties, TextTrailingWordEllipsis,
    WrappingTextLineBreak, DEFAULT_TEXT_SOURCE_LENGTH,
};
use ferroui_base::media::{
    BaselineAlignment, Brushes, CharacterHit, Colors, FlowDirection, FontFamily, FontManager, FontStretch, FontStyle,
    FontWeight, IBrush, SolidColorBrush, TextAlignment, TextCollapsingCreateInfo, TextTrimming, TextWrapping,
    Typeface, DEFAULT_ELLIPSIS_CHAR,
};
use ferroui_base::platform::IFontManagerImpl;
use ferroui_base::utilities::{ReadOnlyMemory, Uri, UriKind, ValueSpan};
use ferroui_base::{FerroLocator, Matrix, Point, Rect, Size};
use ferroui_controls::testing::{UnitTestApplication, UnitTestApplicationScope};
use std::any::Any;
use std::rc::Rc;

/// Upstream's `new GenericTextRunProperties(typeface, fontRenderingEmSize, foregroundBrush: brush)`.
fn run_properties_with_foreground(
    typeface: Typeface,
    font_rendering_em_size: f64,
    foreground_brush: Rc<dyn IBrush>,
) -> Rc<GenericTextRunProperties> {
    Rc::new(GenericTextRunProperties::with_all(
        typeface,
        font_rendering_em_size,
        None,
        Some(foreground_brush),
        None,
        BaselineAlignment::Baseline,
        None,
        None,
    ))
}

/// Upstream's `new GenericTextParagraphProperties(defaultProperties, textWrapping: wrapping)`.
fn paragraph_properties(
    default_properties: Rc<dyn TextRunProperties>,
    text_wrapping: TextWrapping,
) -> Rc<dyn TextParagraphProperties> {
    Rc::new(GenericTextParagraphProperties::with_options(default_properties, TextAlignment::Left, text_wrapping, 0.0, 0.0))
}

fn utf16_len(text: &str) -> i32 {
    text.encode_utf16().count() as i32
}

#[test]
fn should_format_text_runs_with_default_style() {
    let _scope = start();

    let text = "0123456789";

    let default_properties = run_properties_with_foreground(Typeface::default_typeface(), 12.0, Brushes::black());

    let text_source = SingleBufferTextSource::new(text, default_properties.clone(), false);

    let formatter = TextFormatterImpl::new();

    let text_line = formatter.format_line(
        &text_source,
        0,
        f64::INFINITY,
        &(Rc::new(GenericTextParagraphProperties::new(default_properties.clone())) as Rc<dyn TextParagraphProperties>),
        None,
    );

    let text_line = text_line.expect("a text line");

    let text_runs = text_line.text_runs();

    assert_eq!(text_runs.len(), 1);

    let text_run = &text_runs[0];

    let properties = text_run.properties().expect("properties");

    assert_eq!(default_properties.typeface(), properties.typeface());

    assert_eq!(default_properties.foreground_brush(), properties.foreground_brush());

    assert_eq!(utf16_len(text), text_run.length());
}

#[test]
fn should_format_text_runs_with_multiple_buffers() {
    let _scope = start();

    let default_properties = run_properties_with_foreground(Typeface::default_typeface(), 12.0, Brushes::black());

    let text_source = MultiBufferTextSource::new(default_properties.clone());

    let formatter = TextFormatterImpl::new();

    let text_line = formatter
        .format_line(
            &text_source,
            0,
            f64::INFINITY,
            &(Rc::new(GenericTextParagraphProperties::new(default_properties)) as Rc<dyn TextParagraphProperties>),
            None,
        )
        .expect("a text line");

    assert_eq!(5, text_line.text_runs().len());

    assert_eq!(50, text_line.length());
}

struct TextSourceWithDummyRuns {
    text_runs: Vec<ValueSpan<Rc<dyn TextRun>>>,
}

impl TextSourceWithDummyRuns {
    fn new(properties: Rc<dyn TextRunProperties>) -> Self {
        Self {
            text_runs: vec![
                ValueSpan::new(0, 5, Rc::new(TextCharacters::from_str("Hello", properties.clone())) as Rc<dyn TextRun>),
                ValueSpan::new(5, 1, Rc::new(DummyRun::new()) as Rc<dyn TextRun>),
                ValueSpan::new(6, 1, Rc::new(DummyRun::new()) as Rc<dyn TextRun>),
                ValueSpan::new(7, 6, Rc::new(TextCharacters::from_str(" World", properties)) as Rc<dyn TextRun>),
            ],
        }
    }
}

impl ITextSource for TextSourceWithDummyRuns {
    fn get_text_run(&self, text_source_index: i32) -> Option<Rc<dyn TextRun>> {
        for run in &self.text_runs {
            if text_source_index < run.start() + run.length() {
                return Some(run.value().clone());
            }
        }

        Some(Rc::new(TextEndOfParagraph::new()))
    }
}

struct DummyRun {
    length: i32,
}

impl DummyRun {
    fn new() -> Self {
        Self { length: DEFAULT_TEXT_SOURCE_LENGTH }
    }
}

impl TextRun for DummyRun {
    fn length(&self) -> i32 {
        self.length
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn into_any_rc(self: Rc<Self>) -> Rc<dyn Any> {
        self
    }
}

#[test]
fn should_format_text_line_with_non_text_text_runs() {
    let _scope = start();

    let default_properties = run_properties_with_foreground(Typeface::default_typeface(), 12.0, Brushes::black());

    let text_source = TextSourceWithDummyRuns::new(default_properties.clone());

    let formatter = TextFormatterImpl::new();

    let text_line = formatter
        .format_line(
            &text_source,
            0,
            f64::INFINITY,
            &(Rc::new(GenericTextParagraphProperties::new(default_properties)) as Rc<dyn TextParagraphProperties>),
            None,
        )
        .expect("a text line");

    assert_eq!(5, text_line.text_runs().len());

    assert_eq!(14, text_line.length());
}

#[test]
fn should_format_text_line_with_non_text_text_runs_right_to_left() {
    let _scope = start();

    let default_properties = run_properties_with_foreground(Typeface::default_typeface(), 12.0, Brushes::black());

    let text_source = TextSourceWithDummyRuns::new(default_properties.clone());

    let formatter = TextFormatterImpl::new();

    let paragraph_properties: Rc<dyn TextParagraphProperties> = Rc::new(GenericTextParagraphProperties::with_all(
        FlowDirection::RightToLeft,
        TextAlignment::Left,
        true,
        true,
        default_properties,
        TextWrapping::NoWrap,
        0.0,
        0.0,
        0.0,
    ));

    let text_line =
        formatter.format_line(&text_source, 0, f64::INFINITY, &paragraph_properties, None).expect("a text line");

    let text_runs = text_line.text_runs();

    assert_eq!(5, text_runs.len());

    assert_eq!(14, text_line.length());

    let first = text_runs[0].downcast_ref::<ShapedTextRun>();

    let last = text_runs[4].downcast_ref::<TextEndOfParagraph>();

    assert!(first.is_some());

    assert!(last.is_some());

    assert_eq!(ReadOnlyMemory::from_str("Hello"), first.unwrap().text());
}

#[test]
fn should_reset_bidi_levels_of_trailing_whitespaces_after_text_wrapping() {
    let _scope = start();

    let text = "aaa bbb";

    let default_properties: Rc<dyn TextRunProperties> =
        Rc::new(GenericTextRunProperties::new(Typeface::default_typeface()));

    let paragraph_properties: Rc<dyn TextParagraphProperties> = Rc::new(GenericTextParagraphProperties::with_all(
        FlowDirection::RightToLeft,
        TextAlignment::Right,
        true,
        true,
        default_properties.clone(),
        TextWrapping::Wrap,
        0.0,
        0.0,
        0.0,
    ));

    let text_source = SimpleTextSource::new(text, default_properties);

    let formatter = TextFormatterImpl::new();

    let first_line = formatter.format_line(&text_source, 0, 50.0, &paragraph_properties, None).expect("a text line");

    let text_runs = first_line.text_runs();

    assert_eq!(2, text_runs.len());

    let first = text_runs[0].downcast_ref::<ShapedTextRun>().expect("a shaped run");

    let second = text_runs[1].downcast_ref::<ShapedTextRun>().expect("a shaped run");

    assert_eq!(" ", first.text().to_string_lossy());

    assert_eq!("aaa", second.text().to_string_lossy());

    assert_eq!(1, first.bidi_level());

    assert_eq!(2, second.bidi_level());
}

#[test]
fn should_reset_bidi_levels_of_trailing_whitespaces_after_text_wrapping_2() {
    let _scope = start();

    let text = "אאא בבב";

    let default_properties: Rc<dyn TextRunProperties> =
        Rc::new(GenericTextRunProperties::new(Typeface::default_typeface()));

    let paragraph_properties: Rc<dyn TextParagraphProperties> = Rc::new(GenericTextParagraphProperties::with_all(
        FlowDirection::LeftToRight,
        TextAlignment::Left,
        true,
        true,
        default_properties.clone(),
        TextWrapping::Wrap,
        0.0,
        0.0,
        0.0,
    ));

    let text_source = SimpleTextSource::new(text, default_properties);

    let formatter = TextFormatterImpl::new();

    let first_line = formatter.format_line(&text_source, 0, 40.0, &paragraph_properties, None).expect("a text line");

    let text_runs = first_line.text_runs();

    assert_eq!(2, text_runs.len());

    let first = text_runs[0].downcast_ref::<ShapedTextRun>().expect("a shaped run");

    let second = text_runs[1].downcast_ref::<ShapedTextRun>().expect("a shaped run");

    assert_eq!("אאא", first.text().to_string_lossy());

    assert_eq!(" ", second.text().to_string_lossy());

    assert_eq!(1, first.bidi_level());

    assert_eq!(0, second.bidi_level());
}

#[test]
fn should_format_text_runs_with_text_run_styles() {
    let _scope = start();

    let text = "0123456789";

    let default_properties: Rc<dyn TextRunProperties> =
        run_properties_with_foreground(Typeface::default_typeface(), 12.0, Brushes::black());

    let generic_text_run_properties_runs: Vec<ValueSpan<Rc<dyn TextRunProperties>>> = vec![
        ValueSpan::new(0, 3, default_properties.clone()),
        ValueSpan::new(3, 3, run_properties_with_foreground(Typeface::default_typeface(), 13.0, Brushes::black())),
        ValueSpan::new(6, 3, run_properties_with_foreground(Typeface::default_typeface(), 14.0, Brushes::black())),
        ValueSpan::new(9, 1, default_properties.clone()),
    ];

    let text_source = FormattedTextSource::new(
        ReadOnlyMemory::from_str(text),
        default_properties.clone(),
        Some(Rc::from(generic_text_run_properties_runs.clone())),
    );

    let formatter = TextFormatterImpl::new();

    let text_line = formatter
        .format_line(
            &text_source,
            0,
            f64::INFINITY,
            &(Rc::new(GenericTextParagraphProperties::new(default_properties)) as Rc<dyn TextParagraphProperties>),
            None,
        )
        .expect("a text line");

    assert_eq!(utf16_len(text), text_line.length());

    let text_runs = text_line.text_runs();

    for (i, generic_text_run_properties_run) in generic_text_run_properties_runs.iter().enumerate() {
        let text_run = &text_runs[i];

        assert_eq!(generic_text_run_properties_run.length(), text_run.length());
    }
}

#[test]
fn should_produce_unique_runs() {
    for (text, number_of_runs) in
        [("0123", 1), ("\r\n", 1), ("👍b", 2), ("a👍b", 3), ("a👍子b", 4)]
    {
        let _scope = start();

        let default_properties = Rc::new(GenericTextRunProperties::new(Typeface::default_typeface()));

        let text_source = SingleBufferTextSource::new(text, default_properties.clone(), false);

        let formatter = TextFormatterImpl::new();

        let text_line = formatter
            .format_line(
                &text_source,
                0,
                f64::INFINITY,
                &(Rc::new(GenericTextParagraphProperties::new(default_properties)) as Rc<dyn TextParagraphProperties>),
                None,
            )
            .expect("a text line");

        assert_eq!(number_of_runs, text_line.text_runs().len(), "{text:?}");
    }
}

#[test]
fn should_not_absorb_whitespace_into_a_fallback_run() {
    let _scope = start();

    let default_properties = Rc::new(GenericTextRunProperties::new(Typeface::default_typeface()));

    let text = "👍 👍 👍 👍";

    let text_source = SingleBufferTextSource::new(text, default_properties.clone(), false);

    let formatter = TextFormatterImpl::new();

    let text_line = formatter
        .format_line(
            &text_source,
            0,
            f64::INFINITY,
            &(Rc::new(GenericTextParagraphProperties::new(default_properties.clone()))
                as Rc<dyn TextParagraphProperties>),
            None,
        )
        .expect("a text line");

    // Four emoji in a fallback font, separated by three spaces the primary font covers:
    // the spaces keep the primary's metrics instead of the emoji font's, so they form
    // runs of their own.
    let text_runs = text_line.text_runs();

    assert_eq!(7, text_runs.len());

    for (i, text_run) in text_runs.iter().enumerate() {
        let is_space = i % 2 == 1;

        assert_eq!(if is_space { 1 } else { 2 }, text_run.length());
        assert_eq!(is_space, default_properties.typeface() == text_run.properties().unwrap().typeface());
    }
}

#[test]
fn should_split_run_on_script() {
    let _scope = start();

    let text = "ABCDالدولي";

    let default_properties = Rc::new(GenericTextRunProperties::new(Typeface::default_typeface()));

    let text_source = SingleBufferTextSource::new(text, default_properties.clone(), false);

    let formatter = TextFormatterImpl::new();

    let text_line = formatter
        .format_line(
            &text_source,
            0,
            f64::INFINITY,
            &(Rc::new(GenericTextParagraphProperties::new(default_properties)) as Rc<dyn TextParagraphProperties>),
            None,
        )
        .expect("a text line");

    let first_run = text_line.text_runs()[0].clone();

    assert_eq!(4, first_run.length());
}

#[test]
fn should_wrap_with_overflow() {
    for (text, expected_characters_per_line, expected_number_of_lines) in
        [("𐐷𐐷𐐷𐐷𐐷", 10, 1), ("01234 56789 01234 56789", 6, 4)]
    {
        let _scope = start();

        let default_properties = Rc::new(GenericTextRunProperties::new(Typeface::default_typeface()));

        let paragraph_properties = paragraph_properties(default_properties.clone(), TextWrapping::WrapWithOverflow);

        let text_source = SingleBufferTextSource::new("ABCDEFHFFHFJHKHFK", default_properties.clone(), true);

        let formatter = TextFormatterImpl::new();

        formatter.format_line(&text_source, 0, 33.0, &paragraph_properties, None);

        let text_source = SingleBufferTextSource::new(text, default_properties, false);

        let mut number_of_lines = 0;

        let mut current_position = 0;

        let text_length = utf16_len(text);

        while current_position < text_length {
            let text_line = formatter
                .format_line(&text_source, current_position, 1.0, &paragraph_properties, None)
                .expect("a text line");

            if text_length - current_position > expected_characters_per_line {
                assert_eq!(expected_characters_per_line, text_line.length(), "{text:?}");
            }

            current_position += text_line.length();

            number_of_lines += 1;
        }

        assert_eq!(expected_number_of_lines, number_of_lines, "{text:?}");
    }
}

// The expectations concatenate the run texts in visual run order, so where the spaces sit
// in the string depends on how the line is cut into runs. Each space is a run of its own
// now (the primary font owns it, not the Hebrew fallback), which regroups the same
// characters - the right-to-left rows below show the same content, differently split.
#[test]
fn text_trimming_should_trim_correctly() {
    const WIDTH: f64 = 160.0;
    const EM_SIZE: f64 = 20.0;

    for (text, trimmed, direction, word_ellipsis) in [
        ("one שתיים three ארבע", "one שתיים thr…", FlowDirection::LeftToRight, false),
        ("one שתיים three ארבע", "…thr שתיים one", FlowDirection::RightToLeft, false),
        ("one שתיים three ארבע", "one שתיים…", FlowDirection::LeftToRight, true),
        ("one שתיים three ארבע", "… שתיים one", FlowDirection::RightToLeft, true),
    ] {
        let _scope = start();

        let default_properties: Rc<dyn TextRunProperties> =
            Rc::new(GenericTextRunProperties::with_font_size(Typeface::default_typeface(), EM_SIZE));

        let paragraph_properties: Rc<dyn TextParagraphProperties> = Rc::new(GenericTextParagraphProperties::with_all(
            direction,
            TextAlignment::Start,
            true,
            true,
            default_properties.clone(),
            TextWrapping::NoWrap,
            0.0,
            0.0,
            0.0,
        ));

        let text_source = SimpleTextSource::new(text, default_properties.clone());

        let formatter = TextFormatterImpl::new();

        let text_line =
            formatter.format_line(&text_source, 0, f64::INFINITY, &paragraph_properties, None).expect("a text line");

        let text_trimming =
            if word_ellipsis { <dyn TextTrimming>::word_ellipsis() } else { <dyn TextTrimming>::character_ellipsis() };

        let collapsing_properties = text_trimming.create_collapsing_properties(&TextCollapsingCreateInfo::new(
            WIDTH,
            default_properties,
            direction,
        ));

        let collapsed_line = text_line.collapse(&[Some(collapsing_properties)]);

        let trimmed_result: String =
            collapsed_line.text_runs().iter().map(|x| x.text().to_string_lossy()).collect();

        assert_eq!(trimmed, trimmed_result, "{direction:?}, word ellipsis: {word_ellipsis}");
    }
}

#[test]
fn should_wrap() {
    for (text, family_name, number_of_characters_per_line) in [
        (
            "Whether to turn off HTTPS. This option only applies if Individual, \
             IndividualB2C, SingleOrg, or MultiOrg aren't used for &#8209;&#8209;auth.",
            "Noto Sans",
            40,
        ),
        ("01234 56789 01234 56789", "Noto Mono", 7),
    ] {
        let _scope = start();

        let units: Vec<u16> = text.encode_utf16().collect();

        let mut line_breaker = LineBreakEnumerator::new(&units);

        let mut expected: Vec<i32> = Vec::new();

        while let Some(line_break) = line_breaker.move_next() {
            expected.push(line_break.position_wrap() as i32 - 1);
        }

        let typeface = Typeface::from_name(&format!(
            "resm:FerroUI.Skia.UnitTests.Assets?assembly=ferroui-skia#{family_name}"
        ));

        let default_properties = Rc::new(GenericTextRunProperties::new(Typeface::default_typeface()));

        let text_source = SingleBufferTextSource::new(text, default_properties.clone(), false);

        let formatter = TextFormatterImpl::new();

        let glyph_typeface = typeface.glyph_typeface();

        let glyph = glyph_typeface.character_to_glyph_map().get_glyph('a' as i32);

        let advance = glyph_typeface.try_get_horizontal_glyph_advance(glyph).unwrap_or_default();

        let scale = 12.0 / glyph_typeface.metrics().design_em_height as f64;

        let paragraph_width = advance as f64 * scale * number_of_characters_per_line as f64;

        let mut current_position = 0;

        while current_position < units.len() as i32 {
            let text_line = formatter
                .format_line(
                    &text_source,
                    current_position,
                    paragraph_width,
                    &paragraph_properties(default_properties.clone(), TextWrapping::Wrap),
                    None,
                )
                .expect("a text line");

            let end = text_line.first_text_source_index() + text_line.length() - 1;

            assert!(expected.contains(&end), "{family_name}: {end} is not a line break of {expected:?}");

            let index = expected.iter().position(|&position| position == end).unwrap();

            for _ in 0..=index {
                expected.remove(0);
            }

            current_position += text_line.length();
        }
    }
}

#[test]
fn should_produce_fixed_height_lines() {
    let _scope = start();

    let text = "012345";

    let default_properties = Rc::new(GenericTextRunProperties::new(Typeface::default_typeface()));

    let text_source = SingleBufferTextSource::new(text, default_properties.clone(), false);

    let formatter = TextFormatterImpl::new();

    let text_line = formatter
        .format_line(
            &text_source,
            0,
            f64::INFINITY,
            &(Rc::new(GenericTextParagraphProperties::with_options(
                default_properties,
                TextAlignment::Left,
                TextWrapping::NoWrap,
                50.0,
                0.0,
            )) as Rc<dyn TextParagraphProperties>),
            None,
        )
        .expect("a text line");

    assert_eq!(50.0, text_line.height());
}

#[test]
fn should_not_produce_text_line_wider_than_paragraph_width() {
    let _scope = start();

    let text = "Multiline TextBlock with TextWrapping.\r\rLorem ipsum dolor sit amet, consectetur adipiscing elit. \
                Vivamus magna. Cras in mi at felis aliquet congue. Ut a est eget ligula molestie gravida. Curabitur massa. \
                Donec eleifend, libero at sagittis mollis, tellus est malesuada tellus, at luctus turpis elit sit amet quam. \
                Vivamus pretium ornare est.";

    let default_properties = Rc::new(GenericTextRunProperties::new(Typeface::default_typeface()));

    let paragraph_properties = paragraph_properties(default_properties.clone(), TextWrapping::Wrap);

    let text_source = SingleBufferTextSource::new(text, default_properties, false);

    let formatter = TextFormatterImpl::new();

    let mut text_source_index = 0;

    while text_source_index < utf16_len(text) {
        let text_line = formatter
            .format_line(&text_source, text_source_index, 200.0, &paragraph_properties, None)
            .expect("a text line");

        assert!(text_line.width() <= 200.0, "{}", text_line.width());

        text_source_index += text_line.length();
    }
}

#[test]
fn wrap_should_not_produce_empty_lines() {
    let _scope = start();

    let text = "012345";

    let default_properties = Rc::new(GenericTextRunProperties::new(Typeface::default_typeface()));
    let paragraph_properties = paragraph_properties(default_properties.clone(), TextWrapping::Wrap);
    let text_source = SingleBufferTextSource::new(text, default_properties, false);
    let formatter = TextFormatterImpl::new();

    let mut text_source_index = 0;

    while text_source_index < utf16_len(text) {
        let text_line = formatter
            .format_line(&text_source, text_source_index, 3.0, &paragraph_properties, None)
            .expect("a text line");

        assert_ne!(0, text_line.length());

        text_source_index += text_line.length();
    }

    assert_eq!(utf16_len(text), text_source_index);
}

#[test]
fn should_produce_wrapped_and_trimmed_lines() {
    for (text, expected_lines) in [(
        "Lorem ipsum dolor sit amet, consectetur adipisicing elit, sed do eiusmod tempor",
        ["Lorem ipsum ", "dolor sit amet, ", "consectetur ", "adipisicing ", "elit, sed do ", "eiusmod tempor"],
    )] {
        let _scope = start();

        let typeface = Typeface::default_typeface();

        let default_properties: Rc<dyn TextRunProperties> =
            run_properties_with_foreground(typeface.clone(), 32.0, Brushes::black());

        let style_spans: Vec<ValueSpan<Rc<dyn TextRunProperties>>> = vec![
            ValueSpan::new(0, 5, Rc::new(GenericTextRunProperties::with_font_size(typeface.clone(), 48.0))),
            ValueSpan::new(
                6,
                11,
                Rc::new(GenericTextRunProperties::with_font_size(
                    Typeface::with_style(
                        FontFamily::default_family(),
                        FontStyle::Normal,
                        FontWeight::Bold,
                        FontStretch::Normal,
                    ),
                    32.0,
                )),
            ),
            ValueSpan::new(
                28,
                28,
                Rc::new(GenericTextRunProperties::with_font_size(
                    Typeface::with_style(
                        FontFamily::default_family(),
                        FontStyle::Italic,
                        FontWeight::Normal,
                        FontStretch::Normal,
                    ),
                    32.0,
                )),
            ),
        ];

        let text_source =
            FormattedTextSource::new(ReadOnlyMemory::from_str(text), default_properties.clone(), Some(Rc::from(style_spans)));

        let formatter = TextFormatterImpl::new();

        let mut current_position = 0;

        let mut current_height = 0.0;

        let mut current_line_index = 0;

        while current_position < utf16_len(text) && current_line_index < expected_lines.len() {
            let mut text_line = formatter
                .format_line(
                    &text_source,
                    current_position,
                    300.0,
                    &paragraph_properties(default_properties.clone(), TextWrapping::WrapWithOverflow),
                    None,
                )
                .expect("a text line");

            current_position += text_line.length();

            if text_line.width() > 300.0 || current_height + text_line.height() > 240.0 {
                text_line = text_line.collapse(&[Some(Rc::new(TextTrailingWordEllipsis::new(
                    DEFAULT_ELLIPSIS_CHAR,
                    300.0,
                    default_properties.clone(),
                    FlowDirection::LeftToRight,
                )))]);
            }

            current_height += text_line.height();

            let start = text_line.first_text_source_index() as usize;

            let current_text = &text[start..start + text_line.length() as usize];

            assert_eq!(expected_lines[current_line_index], current_text);

            current_line_index += 1;
        }

        assert_eq!(expected_lines.len(), current_line_index);
    }
}

#[test]
fn should_align_text_line() {
    for (text, text_alignment, flow_direction) in [
        ("0123456789", TextAlignment::Left, FlowDirection::LeftToRight),
        ("0123456789", TextAlignment::Center, FlowDirection::LeftToRight),
        ("0123456789", TextAlignment::Right, FlowDirection::LeftToRight),
        ("0123456789", TextAlignment::Left, FlowDirection::RightToLeft),
        ("0123456789", TextAlignment::Center, FlowDirection::RightToLeft),
        ("0123456789", TextAlignment::Right, FlowDirection::RightToLeft),
        ("שנבגק", TextAlignment::Left, FlowDirection::RightToLeft),
        ("שנבגק", TextAlignment::Center, FlowDirection::RightToLeft),
        ("שנבגק", TextAlignment::Right, FlowDirection::RightToLeft),
    ] {
        let _scope = start();

        let default_properties = Rc::new(GenericTextRunProperties::new(Typeface::default_typeface()));

        let paragraph_properties: Rc<dyn TextParagraphProperties> = Rc::new(GenericTextParagraphProperties::with_all(
            flow_direction,
            text_alignment,
            true,
            true,
            default_properties.clone(),
            TextWrapping::NoWrap,
            0.0,
            0.0,
            0.0,
        ));

        let text_source = SingleBufferTextSource::new(text, default_properties, false);
        let formatter = TextFormatterImpl::new();

        let text_line = formatter.format_line(&text_source, 0, 100.0, &paragraph_properties, None).expect("a text line");

        let expected_offset = match text_alignment {
            TextAlignment::Center => 50.0 - text_line.width() / 2.0,
            TextAlignment::Right => 100.0 - text_line.width_including_trailing_whitespace(),
            _ => 0.0,
        };

        assert_eq!(expected_offset, text_line.start(), "{text:?}, {text_alignment:?}, {flow_direction:?}");
    }
}

#[test]
fn should_wrap_syriac() {
    let _scope = start();

    let text = "܀ ܁ ܂ ܃ ܄ ܅ ܆ ܇ ܈ ܉ ܊ ܋ ܌ ܍ ܏ ܐ ܑ ܒ ܓ ܔ ܕ ܖ ܗ ܘ ܙ ܚ ܛ ܜ ܝ ܞ ܟ ܠ ܡ ܢ ܣ ܤ ܥ ܦ ܧ ܨ ܩ ܪ ܫ ܬ ܰ ܱ ܲ ܳ ܴ ܵ ܶ ܷ ܸ ܹ ܺ ܻ ܼ ܽ ܾ ܿ ݀ ݁ ݂ ݃ ݄ ݅ ݆ ݇ ݈ ݉ ݊";
    let default_properties = Rc::new(GenericTextRunProperties::new(Typeface::default_typeface()));

    let paragraph_properties = paragraph_properties(default_properties.clone(), TextWrapping::Wrap);

    let text_source = SingleBufferTextSource::new(text, default_properties, false);
    let formatter = TextFormatterImpl::new();

    let mut text_position = 87;
    let mut last_break: Option<Rc<TextLineBreak>> = None;

    while text_position < utf16_len(text) {
        let text_line = formatter
            .format_line(&text_source, text_position, 50.0, &paragraph_properties, last_break.as_ref())
            .expect("a text line");

        assert_eq!(text_line.length(), text_line.text_runs().iter().map(|x| x.length()).sum::<i32>());

        text_position += text_line.length();

        last_break = text_line.text_line_break();
    }
}

#[test]
fn should_format_line_with_emergency_breaks() {
    let _scope = start();

    let default_properties = Rc::new(GenericTextRunProperties::new(Typeface::default_typeface()));
    let paragraph_properties = paragraph_properties(default_properties.clone(), TextWrapping::Wrap);

    let text_source =
        SingleBufferTextSource::new("0123456789_0123456789_0123456789_0123456789", default_properties, false);
    let formatter = TextFormatterImpl::new();

    let text_line = formatter.format_line(&text_source, 0, 33.0, &paragraph_properties, None).expect("a text line");

    let remaining_runs_line_break = text_line.text_line_break().expect("a line break");
    assert!(WrappingTextLineBreak::is_wrapping(&remaining_runs_line_break));
    let remaining_runs = WrappingTextLineBreak::acquire_remaining_runs(&remaining_runs_line_break);
    assert!(remaining_runs.is_some());
    assert!(!remaining_runs.unwrap().is_empty());
}

#[test]
fn should_not_alter_text_runs_after_text_styles_were_applied() {
    for text in ["פעילות הבינאום, W3C!", "abcABC", "זה כיף סתם לשמוע איך תנצח קרפד עץ טוב בגן", "טטטט abcDEF טטטט"] {
        let _scope = start();

        let formatter = TextFormatterImpl::new();

        let default_properties = Rc::new(GenericTextRunProperties::new(Typeface::default_typeface()));

        let paragraph_properties = paragraph_properties(default_properties.clone(), TextWrapping::NoWrap);

        let foreground: Rc<dyn IBrush> = SolidColorBrush::with_color(Colors::RED).to_immutable();

        let expected_text_line = formatter
            .format_line(
                &SingleBufferTextSource::new(text, default_properties.clone(), false),
                0,
                f64::INFINITY,
                &paragraph_properties,
                None,
            )
            .expect("a text line");

        let glyph_indices = |text_line: &dyn TextLine| -> Vec<u16> {
            text_line
                .text_runs()
                .iter()
                .map(|run| run.downcast_ref::<ShapedTextRun>().expect("a shaped run"))
                .flat_map(|run| {
                    run.glyph_run().glyph_infos().borrow().iter().map(|glyph| glyph.glyph_index).collect::<Vec<_>>()
                })
                .collect()
        };

        let expected_glyphs = glyph_indices(&*expected_text_line);

        let text_length = utf16_len(text);

        for i in 0..text_length {
            let mut j = 1;

            while i + j < text_length {
                let spans: Vec<ValueSpan<Rc<dyn TextRunProperties>>> = vec![ValueSpan::new(
                    i,
                    j,
                    run_properties_with_foreground(Typeface::default_typeface(), 12.0, foreground.clone()),
                )];

                let text_source = FormattedTextSource::new(
                    ReadOnlyMemory::from_str(text),
                    default_properties.clone(),
                    Some(Rc::from(spans)),
                );

                let text_line = formatter
                    .format_line(&text_source, 0, f64::INFINITY, &paragraph_properties, None)
                    .expect("a text line");

                let actual_glyphs = glyph_indices(&*text_line);

                assert_eq!(expected_glyphs, actual_glyphs, "{text:?}: span {i}, {j}");

                j += 1;
            }
        }
    }
}

#[test]
fn should_format_line_with_drawable_runs() {
    let default_run_properties: Rc<dyn TextRunProperties> =
        run_properties_with_foreground(Typeface::default_typeface(), 12.0, Brushes::black());
    let paragraph_properties: Rc<dyn TextParagraphProperties> =
        Rc::new(GenericTextParagraphProperties::new(default_run_properties));
    let text_source = CustomTextSource::new("Hello World ->");

    let _scope = start();

    let text_line = <dyn TextFormatter>::current()
        .format_line(&text_source, 0, f64::INFINITY, &paragraph_properties, None)
        .expect("a text line");

    let text_runs = text_line.text_runs();

    assert_eq!(3, text_runs.len());

    assert!(text_runs[1].is::<RectangleRun>());
}

#[test]
fn should_format_with_end_of_line_run() {
    let _scope = start();

    let default_run_properties: Rc<dyn TextRunProperties> =
        Rc::new(GenericTextRunProperties::new(Typeface::default_typeface()));
    let paragraph_properties: Rc<dyn TextParagraphProperties> =
        Rc::new(GenericTextParagraphProperties::new(default_run_properties));
    let text_source = EndOfLineTextSource;

    let text_line = <dyn TextFormatter>::current()
        .format_line(&text_source, 0, f64::INFINITY, &paragraph_properties, None)
        .expect("a text line");

    assert!(text_line.text_line_break().is_some());

    assert_eq!(DEFAULT_TEXT_SOURCE_LENGTH, text_line.length());
}

#[test]
fn should_return_null_for_empty_text_source() {
    let _scope = start();

    let default_run_properties: Rc<dyn TextRunProperties> =
        Rc::new(GenericTextRunProperties::new(Typeface::default_typeface()));
    let paragraph_properties: Rc<dyn TextParagraphProperties> =
        Rc::new(GenericTextParagraphProperties::new(default_run_properties));
    let text_source = EmptyTextSource;

    let text_line =
        <dyn TextFormatter>::current().format_line(&text_source, 0, f64::INFINITY, &paragraph_properties, None);

    assert!(text_line.is_none());
}

#[test]
fn should_retain_text_end_of_paragraph_with_text_wrapping() {
    let _scope = start();

    let default_run_properties: Rc<dyn TextRunProperties> =
        Rc::new(GenericTextRunProperties::new(Typeface::default_typeface()));
    let paragraph_properties = paragraph_properties(default_run_properties.clone(), TextWrapping::Wrap);

    let text = "Hello World";

    let text_source = SimpleTextSource::new(text, default_run_properties);

    let mut pos = 0;

    let mut previous_line_break: Option<Rc<TextLineBreak>> = None;
    let mut text_line: Option<Rc<dyn TextLine>> = None;

    while pos < utf16_len(text) {
        let line = <dyn TextFormatter>::current()
            .format_line(&text_source, pos, 30.0, &paragraph_properties, previous_line_break.as_ref())
            .expect("a text line");

        pos += line.length();

        previous_line_break = line.text_line_break();

        text_line = Some(line);
    }

    let text_line = text_line.expect("a text line");
    let text_line_break = text_line.text_line_break().expect("a line break");
    assert!(text_line_break.text_end_of_line().is_some());
}

#[test]
fn should_hit_test_string_with_invisible_runs() {
    let default_run_properties: Rc<dyn TextRunProperties> =
        run_properties_with_foreground(Typeface::default_typeface(), 12.0, Brushes::black());
    let paragraph_properties: Rc<dyn TextParagraphProperties> =
        Rc::new(GenericTextParagraphProperties::new(default_run_properties));

    let _scope = start();

    let hello = TextCharacters::from_str(
        "Hello",
        run_properties_with_foreground(Typeface::default_typeface(), 12.0, Brushes::black()),
    );
    let world = TextCharacters::from_str(
        "world",
        run_properties_with_foreground(Typeface::default_typeface(), 12.0, Brushes::red()),
    );

    let source = ListTextSource::new(vec![
        Rc::new(InvisibleRun::new(1)),
        Rc::new(hello),
        Rc::new(InvisibleRun::new(1)),
        Rc::new(world),
    ]);

    let text_line = <dyn TextFormatter>::current()
        .format_line(&source, 0, f64::INFINITY, &paragraph_properties, None)
        .expect("a text line");

    let verify_hit = |offset: i32| {
        let glyph_center = text_line.get_text_bounds(offset, 1)[0].rectangle().center();
        let hit = text_line.get_character_hit_from_distance(glyph_center.x);
        assert_eq!(offset, hit.first_character_index());
    };
    verify_hit(3);
    verify_hit(8);
}

#[test]
fn get_text_bounds_for_text_line_with_zero_width_spaces_does_not_freeze() {
    let default_run_properties: Rc<dyn TextRunProperties> =
        run_properties_with_foreground(Typeface::default_typeface(), 12.0, Brushes::black());
    let paragraph_properties: Rc<dyn TextParagraphProperties> =
        Rc::new(GenericTextParagraphProperties::new(default_run_properties));

    let _scope = start();

    let text = TextCharacters::from_str(
        "\u{200B}\u{200B}",
        run_properties_with_foreground(Typeface::default_typeface(), 12.0, Brushes::black()),
    );

    let source =
        ListTextSource::new(vec![Rc::new(text), Rc::new(InvisibleRun::new(1)), Rc::new(TextEndOfParagraph::new())]);

    let text_line = <dyn TextFormatter>::current()
        .format_line(&source, 0, f64::INFINITY, &paragraph_properties, None)
        .expect("a text line");

    let bounds = text_line.get_text_bounds(0, 3);

    assert_eq!(1, bounds.len());

    let run_bounds = bounds[0].text_run_bounds();

    assert_eq!(2, run_bounds.len());
}

#[test]
fn line_formatting_for_oversized_embedded_runs_does_not_produce_empty_lines() {
    for wrapping in [TextWrapping::NoWrap, TextWrapping::Wrap, TextWrapping::WrapWithOverflow] {
        let default_run_properties: Rc<dyn TextRunProperties> =
            run_properties_with_foreground(Typeface::default_typeface(), 12.0, Brushes::black());
        let paragraph_properties = paragraph_properties(default_run_properties, wrapping);

        let _scope = start();

        let source =
            ListTextSource::new(vec![Rc::new(RectangleRun::new(Rect::new(0.0, 0.0, 200.0, 10.0), Brushes::aqua()))]);
        let text_line = <dyn TextFormatter>::current()
            .format_line(&source, 0, 100.0, &paragraph_properties, None)
            .expect("a text line");
        assert_eq!(200.0, text_line.width_including_trailing_whitespace(), "{wrapping:?}");
    }
}

#[test]
fn line_formatting_for_oversized_embedded_runs_inside_normal_text_does_not_produce_empty_lines() {
    for wrapping in [TextWrapping::NoWrap, TextWrapping::Wrap, TextWrapping::WrapWithOverflow] {
        let default_run_properties: Rc<dyn TextRunProperties> =
            run_properties_with_foreground(Typeface::default_typeface(), 12.0, Brushes::black());
        let paragraph_properties = paragraph_properties(default_run_properties, wrapping);

        let _scope = start();

        let typeface = Typeface::new(
            FontFamily::parse("resm:FerroUI.Skia.UnitTests.Fonts?assembly=ferroui-skia#DejaVu Sans").unwrap(),
        );

        let text1 = TextCharacters::from_str(
            "Hello",
            run_properties_with_foreground(typeface.clone(), 12.0, Brushes::black()),
        );
        let text2 = TextCharacters::from_str("world", run_properties_with_foreground(typeface, 12.0, Brushes::black()));

        let source = ListTextSource::new(vec![
            Rc::new(text1),
            Rc::new(RectangleRun::new(Rect::new(0.0, 0.0, 200.0, 10.0), Brushes::aqua())),
            Rc::new(InvisibleRun::new(1)),
            Rc::new(TextEndOfLine::new()),
            Rc::new(text2),
            Rc::new(TextEndOfParagraph::with_length(1)),
        ]);

        let mut lines: Vec<Rc<dyn TextLine>> = Vec::new();
        let mut dcp = 0;
        let mut c = 0;
        loop {
            assert!(c < 1000, "Infinite loop");
            let text_line = <dyn TextFormatter>::current()
                .format_line(&source, dcp, 30.0, &paragraph_properties, None)
                .expect("a text line");
            dcp += text_line.length();

            let eol = text_line.text_line_break();

            lines.push(text_line);

            if eol
                .as_ref()
                .and_then(|eol| eol.text_end_of_line())
                .is_some_and(|text_end_of_line| text_end_of_line.is::<TextEndOfParagraph>())
            {
                break;
            }

            c += 1;
        }

        assert!(!lines.is_empty(), "{wrapping:?}");
    }
}

struct IncrementalTabProperties {
    default_text_run_properties: Rc<dyn TextRunProperties>,
}

impl IncrementalTabProperties {
    fn new(default_text_run_properties: Rc<dyn TextRunProperties>) -> Self {
        Self { default_text_run_properties }
    }
}

impl TextParagraphProperties for IncrementalTabProperties {
    fn flow_direction(&self) -> FlowDirection {
        FlowDirection::default()
    }

    fn text_alignment(&self) -> TextAlignment {
        TextAlignment::default()
    }

    fn line_height(&self) -> f64 {
        0.0
    }

    fn first_line_in_paragraph(&self) -> bool {
        false
    }

    fn default_text_run_properties(&self) -> &Rc<dyn TextRunProperties> {
        &self.default_text_run_properties
    }

    fn text_wrapping(&self) -> TextWrapping {
        TextWrapping::default()
    }

    fn indent(&self) -> f64 {
        0.0
    }

    fn default_incremental_tab(&self) -> f64 {
        64.0
    }
}

#[test]
fn line_with_incremental_tab_should_return_correct_backspace_position() {
    let _scope = start();

    let typeface =
        Typeface::new(FontFamily::parse("resm:FerroUI.Skia.UnitTests.Fonts?assembly=ferroui-skia#DejaVu Sans").unwrap());

    let default_run_properties: Rc<dyn TextRunProperties> =
        run_properties_with_foreground(typeface.clone(), 12.0, Brushes::black());
    let paragraph_properties: Rc<dyn TextParagraphProperties> =
        Rc::new(IncrementalTabProperties::new(default_run_properties));

    let text = TextCharacters::from_str("ff", run_properties_with_foreground(typeface, 12.0, Brushes::black()));

    let source = ListTextSource::new(vec![Rc::new(text)]);

    let text_line = <dyn TextFormatter>::current()
        .format_line(&source, 0, f64::INFINITY, &paragraph_properties, None)
        .expect("a text line");

    let backspace_hit = text_line.get_backspace_caret_character_hit(CharacterHit::new(2));
    assert_eq!(1, backspace_hit.first_character_index());
    assert_eq!(0, backspace_hit.trailing_length());
}

#[test]
fn should_wrap_chinese() {
    let _scope = start();

    let default_run_properties: Rc<dyn TextRunProperties> =
        Rc::new(GenericTextRunProperties::new(Typeface::default_typeface()));
    let paragraph_properties = paragraph_properties(default_run_properties.clone(), TextWrapping::Wrap);

    let text = "一二三四 TEXT 一二三四五六七八九十零";

    let text_line = <dyn TextFormatter>::current()
        .format_line(&SimpleTextSource::new(text, default_run_properties), 0, 120.0, &paragraph_properties, None)
        .expect("a text line");

    assert_eq!(3, text_line.text_runs().len());
}

/// Upstream's `new FontFamily(new Uri("resm:...Assets?assembly=..."), "Noto Mono")`.
fn noto_mono() -> Typeface {
    let base_uri = Uri::try_create("resm:FerroUI.Skia.UnitTests.Assets?assembly=ferroui-skia", UriKind::Absolute)
        .expect("a valid uri");

    Typeface::new(FontFamily::with_base_uri(Some(&base_uri), "Noto Mono"))
}

#[test]
fn should_match_character_for_spacing_combining_mark() {
    let _scope = start();

    let text = "𖾇";

    let typeface = noto_mono();
    let default_run_properties: Rc<dyn TextRunProperties> = Rc::new(GenericTextRunProperties::new(typeface));
    let paragraph_properties = paragraph_properties(default_run_properties.clone(), TextWrapping::Wrap);
    let text_line = <dyn TextFormatter>::current()
        .format_line(&SimpleTextSource::new(text, default_run_properties), 0, 120.0, &paragraph_properties, None)
        .expect("a text line");

    let text_runs = text_line.text_runs();

    assert!(!text_runs.is_empty());

    let first_run = &text_runs[0];

    let properties = first_run.properties().expect("properties");

    assert_eq!("Noto Sans Miao", properties.typeface().glyph_typeface().family_name());
}

#[test]
fn drawable_run_with_same_baseline_and_size_should_not_alter_line_height() {
    let _scope = start();

    let text = "ABC";

    let typeface = noto_mono();
    let default_run_properties: Rc<dyn TextRunProperties> = Rc::new(GenericTextRunProperties::new(typeface));
    let paragraph_properties = paragraph_properties(default_run_properties.clone(), TextWrapping::Wrap);

    let embedded_text_line = <dyn TextFormatter>::current()
        .format_line(
            &SimpleTextSource::new(text, default_run_properties.clone()),
            0,
            120.0,
            &paragraph_properties,
            None,
        )
        .expect("a text line");

    let expected_height = embedded_text_line.height();
    let expected_baseline = embedded_text_line.baseline();

    let text_source = ListTextSource::new(vec![
        Rc::new(TextCharacters::from_str("ABC", default_run_properties)),
        Rc::new(EmbeddedTextLineRun::new(embedded_text_line)),
    ]);

    let text_line = <dyn TextFormatter>::current()
        .format_line(&text_source, 0, f64::INFINITY, &paragraph_properties, None)
        .expect("a text line");

    assert_eq!(expected_height, text_line.height());

    assert_eq!(expected_baseline, text_line.baseline());
}

#[test]
fn drawable_run_with_same_baseline_and_bigger_height_should_not_alter_baseline() {
    let _scope = start();

    let text = "ABC";

    let typeface = noto_mono();
    let default_run_properties: Rc<dyn TextRunProperties> = Rc::new(GenericTextRunProperties::new(typeface));
    let paragraph_properties = paragraph_properties(default_run_properties.clone(), TextWrapping::Wrap);

    let embedded_text_line = <dyn TextFormatter>::current()
        .format_line(
            &SimpleTextSource::new(text, default_run_properties.clone()),
            0,
            120.0,
            &paragraph_properties,
            None,
        )
        .expect("a text line");

    let expected_height = embedded_text_line.height() + 10.0;

    let embedded_size = Size::new(embedded_text_line.width(), expected_height);

    let expected_baseline = embedded_text_line.baseline();

    let text_source = ListTextSource::new(vec![
        Rc::new(TextCharacters::from_str("ABC", default_run_properties)),
        Rc::new(CustomDrawableRun::new(embedded_size, expected_baseline)),
    ]);

    let text_line = <dyn TextFormatter>::current()
        .format_line(&text_source, 0, f64::INFINITY, &paragraph_properties, None)
        .expect("a text line");

    assert_eq!(expected_height, text_line.height());

    assert_eq!(expected_baseline, text_line.baseline());
}

struct CustomDrawableRun {
    size: Size,
    baseline: f64,
}

impl CustomDrawableRun {
    fn new(size: Size, base_line: f64) -> Self {
        Self { size, baseline: base_line }
    }
}

impl TextRun for CustomDrawableRun {
    fn as_drawable(&self) -> Option<&dyn DrawableTextRun> {
        Some(self)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn into_any_rc(self: Rc<Self>) -> Rc<dyn Any> {
        self
    }
}

impl DrawableTextRun for CustomDrawableRun {
    fn size(&self) -> Size {
        self.size
    }

    fn baseline(&self) -> f64 {
        self.baseline
    }

    fn draw(&self, _drawing_context: &mut dyn ITextDrawingSink, _origin: Point) {
        // no op
    }
}

struct EmbeddedTextLineRun {
    text_line: Rc<dyn TextLine>,
}

impl EmbeddedTextLineRun {
    fn new(text_line: Rc<dyn TextLine>) -> Self {
        Self { text_line }
    }
}

impl TextRun for EmbeddedTextLineRun {
    fn as_drawable(&self) -> Option<&dyn DrawableTextRun> {
        Some(self)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn into_any_rc(self: Rc<Self>) -> Rc<dyn Any> {
        self
    }
}

impl DrawableTextRun for EmbeddedTextLineRun {
    fn size(&self) -> Size {
        Size::new(self.text_line.width(), self.text_line.height())
    }

    fn baseline(&self) -> f64 {
        self.text_line.baseline()
    }

    fn draw(&self, drawing_context: &mut dyn ITextDrawingSink, origin: Point) {
        self.text_line.draw(drawing_context, origin);
    }
}

/// Upstream's `protected readonly record struct SimpleTextSource`.
pub(crate) struct SimpleTextSource {
    text: ReadOnlyMemory<u16>,
    default_properties: Rc<dyn TextRunProperties>,
}

impl SimpleTextSource {
    pub(crate) fn new(text: &str, default_properties: Rc<dyn TextRunProperties>) -> Self {
        Self { text: ReadOnlyMemory::from_str(text), default_properties }
    }
}

impl ITextSource for SimpleTextSource {
    fn get_text_run(&self, text_source_index: i32) -> Option<Rc<dyn TextRun>> {
        if text_source_index as usize > self.text.len() {
            return Some(Rc::new(TextEndOfParagraph::new()));
        }

        let run_text = self.text.slice_from(text_source_index as usize);

        if run_text.is_empty() {
            return Some(Rc::new(TextEndOfParagraph::new()));
        }

        Some(Rc::new(TextCharacters::new(run_text, self.default_properties.clone())))
    }
}

struct EmptyTextSource;

impl ITextSource for EmptyTextSource {
    fn get_text_run(&self, _text_source_index: i32) -> Option<Rc<dyn TextRun>> {
        None
    }
}

struct EndOfLineTextSource;

impl ITextSource for EndOfLineTextSource {
    fn get_text_run(&self, _text_source_index: i32) -> Option<Rc<dyn TextRun>> {
        Some(Rc::new(TextEndOfLine::new()))
    }
}

struct CustomTextSource {
    text: String,
}

impl CustomTextSource {
    fn new(text: &str) -> Self {
        Self { text: text.to_owned() }
    }
}

impl ITextSource for CustomTextSource {
    fn get_text_run(&self, text_source_index: i32) -> Option<Rc<dyn TextRun>> {
        let text_length = utf16_len(&self.text);

        if text_source_index >= text_length + DEFAULT_TEXT_SOURCE_LENGTH + text_length {
            return None;
        }

        if text_source_index == text_length {
            return Some(Rc::new(RectangleRun::new(Rect::new(0.0, 0.0, 50.0, 50.0), Brushes::green())));
        }

        Some(Rc::new(TextCharacters::from_str(
            &self.text,
            run_properties_with_foreground(Typeface::default_typeface(), 12.0, Brushes::black()),
        )))
    }
}

/// Upstream's `internal class ListTextSource`.
pub(crate) struct ListTextSource {
    runs: Vec<Rc<dyn TextRun>>,
}

impl ListTextSource {
    pub(crate) fn new(runs: Vec<Rc<dyn TextRun>>) -> Self {
        Self { runs }
    }
}

impl ITextSource for ListTextSource {
    fn get_text_run(&self, text_source_index: i32) -> Option<Rc<dyn TextRun>> {
        let mut off = 0;
        for run in &self.runs {
            if text_source_index >= off && text_source_index - off < run.length() {
                if run.length() == 1 {
                    return Some(run.clone());
                }
                let chars = run.downcast_ref::<TextCharacters>().expect("a run longer than one is text characters");
                return Some(Rc::new(TextCharacters::new(
                    chars.text().slice_from((text_source_index - off) as usize),
                    chars.run_properties().clone(),
                )));
            }

            off += run.length();
        }

        None
    }
}

struct RectangleRun {
    rect: Rect,
    fill: Rc<dyn IBrush>,
}

impl RectangleRun {
    fn new(rect: Rect, fill: Rc<dyn IBrush>) -> Self {
        Self { rect, fill }
    }
}

impl TextRun for RectangleRun {
    fn as_drawable(&self) -> Option<&dyn DrawableTextRun> {
        Some(self)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn into_any_rc(self: Rc<Self>) -> Rc<dyn Any> {
        self
    }
}

impl DrawableTextRun for RectangleRun {
    fn size(&self) -> Size {
        self.rect.size()
    }

    fn baseline(&self) -> f64 {
        0.0
    }

    fn draw(&self, drawing_context: &mut dyn ITextDrawingSink, origin: Point) {
        drawing_context.push_transform(Matrix::create_translation(origin.x, 0.0));
        drawing_context.draw_rectangle(Some(&self.fill), None, self.rect);
        drawing_context.pop_transform();
    }
}

struct InvisibleRun {
    length: i32,
}

impl InvisibleRun {
    fn new(length: i32) -> Self {
        Self { length }
    }
}

impl TextRun for InvisibleRun {
    fn length(&self) -> i32 {
        self.length
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn into_any_rc(self: Rc<Self>) -> Rc<dyn Any> {
        self
    }
}

/// Upstream's `public static IDisposable Start()`: the mock platform with the
/// Skia render interface, and the custom font manager (the embedded test
/// assets as system fonts) as the font backend.
pub(crate) fn start() -> UnitTestApplicationScope {
    let disposable = UnitTestApplication::start(
        mock_platform_render_interface().with_render_interface(Rc::new(PlatformRenderInterface::default())),
    );

    let custom_font_manager_impl = Rc::new(CustomFontManagerImpl::new());

    FerroLocator::current_mutable()
        .bind::<dyn IFontManagerImpl>()
        .to_constant(custom_font_manager_impl.clone() as Rc<dyn IFontManagerImpl>);

    let font_manager = FontManager::new(custom_font_manager_impl.clone());

    FerroLocator::current_mutable().bind::<FontManager>().to_constant(font_manager.clone());

    font_manager.add_font_collection(custom_font_manager_impl.system_fonts());

    disposable
}
