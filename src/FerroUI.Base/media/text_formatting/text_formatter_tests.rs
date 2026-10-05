//! Port of upstream's formatter tests onto the test harness (fixed advance
//! fonts, one glyph per codepoint). Where upstream's expectation depends on
//! the metrics of a real font, the value is recomputed from the fixed advance
//! and the comment says so.

use std::rc::Rc;

use crate::media::text_formatting::testing::{
    advance, format_line, format_lines, line_text, paragraph_properties, paragraph_properties_with,
    run_properties, utf16, CustomDrawableRun, EndOfLineTextSource, InvisibleRun, ListTextSource,
    MultiBufferTextSource, SingleBufferTextSource, TextTestScope, CJK_FAMILY, DEFAULT_FAMILY, EMOJI_FAMILY,
};
use crate::media::text_formatting::unicode::LineBreakEnumerator;
use crate::media::text_formatting::wrapping_text_line_break::WrappingTextLineBreak;
use crate::media::text_formatting::{
    FormattedTextSource, GenericTextParagraphProperties, GenericTextRunProperties, ITextSource, ShapedTextRun,
    TextCharacters, TextCollapsingProperties, TextEndOfLine, TextEndOfParagraph, TextFormatter, TextLine,
    TextLineBreak, TextParagraphProperties, TextRun, TextRunProperties, TextTrailingWordEllipsis,
    DEFAULT_TEXT_SOURCE_LENGTH,
};
use crate::media::{
    BaselineAlignment, Brushes, FlowDirection, IBrush, TextAlignment, TextCollapsingCreateInfo, TextTrimming,
    TextWrapping, Typeface,
};
use crate::utilities::ValueSpan;
use crate::Size;

/// The default em size of generic run properties.
const EM: f64 = 12.0;

fn default_properties() -> Rc<dyn TextRunProperties> {
    run_properties(EM)
}

fn properties_with_foreground(font_rendering_em_size: f64, foreground: Rc<dyn IBrush>) -> Rc<dyn TextRunProperties> {
    Rc::new(GenericTextRunProperties::with_all(
        Typeface::default_typeface(),
        font_rendering_em_size,
        None,
        Some(foreground),
        None,
        BaselineAlignment::Baseline,
        None,
        None,
    ))
}

fn no_wrap(properties: &Rc<dyn TextRunProperties>) -> Rc<dyn TextParagraphProperties> {
    paragraph_properties(properties, TextWrapping::NoWrap)
}

/// Upstream's `SimpleTextSource`: the rest of the text, then an end of paragraph.
fn simple_source(text: &str, properties: &Rc<dyn TextRunProperties>) -> SingleBufferTextSource {
    SingleBufferTextSource::with_end_of_paragraph(text, properties.clone(), true)
}

fn shaped(run: &Rc<dyn TextRun>) -> &ShapedTextRun {
    run.downcast_ref::<ShapedTextRun>().expect("a shaped run")
}

fn utf16_len(text: &str) -> i32 {
    text.encode_utf16().count() as i32
}

#[test]
fn should_format_text_runs_with_default_style() {
    let _scope = TextTestScope::new();

    let text = "0123456789";

    let foreground: Rc<dyn IBrush> = Brushes::black();
    let default_properties = properties_with_foreground(EM, foreground.clone());

    let text_source = SingleBufferTextSource::new(text, default_properties.clone());

    let text_line = format_line(&text_source, 0, f64::INFINITY, &no_wrap(&default_properties), None).unwrap();

    let runs = text_line.text_runs();

    assert_eq!(runs.len(), 1);

    let run_properties = runs[0].properties().unwrap();

    assert_eq!(run_properties.typeface(), default_properties.typeface());
    assert!(Rc::ptr_eq(run_properties.foreground_brush().unwrap(), &foreground));
    assert_eq!(runs[0].length(), text.len() as i32);
}

#[test]
fn should_format_text_runs_with_multiple_buffers() {
    let _scope = TextTestScope::new();

    let default_properties = default_properties();

    let text_source = MultiBufferTextSource::new(default_properties.clone());

    let text_line = format_line(&text_source, 0, f64::INFINITY, &no_wrap(&default_properties), None).unwrap();

    assert_eq!(text_line.text_runs().len(), 5);
    assert_eq!(text_line.length(), 50);
}

/// "Hello", two runs without text, " World", then end of paragraph.
struct TextSourceWithDummyRuns {
    runs: Vec<(i32, i32, Rc<dyn TextRun>)>,
}

impl TextSourceWithDummyRuns {
    fn new(properties: &Rc<dyn TextRunProperties>) -> Self {
        Self {
            runs: vec![
                (0, 5, Rc::new(TextCharacters::from_str("Hello", properties.clone()))),
                (5, 1, Rc::new(InvisibleRun::new(DEFAULT_TEXT_SOURCE_LENGTH))),
                (6, 1, Rc::new(InvisibleRun::new(DEFAULT_TEXT_SOURCE_LENGTH))),
                (7, 6, Rc::new(TextCharacters::from_str(" World", properties.clone()))),
            ],
        }
    }
}

impl ITextSource for TextSourceWithDummyRuns {
    fn get_text_run(&self, text_source_index: i32) -> Option<Rc<dyn TextRun>> {
        for (start, length, run) in &self.runs {
            if text_source_index < start + length {
                return Some(run.clone());
            }
        }

        Some(Rc::new(TextEndOfParagraph::new()))
    }
}

#[test]
fn should_format_text_line_with_non_text_text_runs() {
    let _scope = TextTestScope::new();

    let default_properties = default_properties();
    let text_source = TextSourceWithDummyRuns::new(&default_properties);

    let text_line = format_line(&text_source, 0, f64::INFINITY, &no_wrap(&default_properties), None).unwrap();

    assert_eq!(text_line.text_runs().len(), 5);
    assert_eq!(text_line.length(), 14);
}

#[test]
fn should_format_text_line_with_non_text_text_runs_right_to_left() {
    let _scope = TextTestScope::new();

    let default_properties = default_properties();
    let text_source = TextSourceWithDummyRuns::new(&default_properties);

    let paragraph_properties = paragraph_properties_with(
        &default_properties,
        TextWrapping::NoWrap,
        TextAlignment::Left,
        FlowDirection::RightToLeft,
    );

    let text_line = format_line(&text_source, 0, f64::INFINITY, &paragraph_properties, None).unwrap();

    let runs = text_line.text_runs();

    assert_eq!(runs.len(), 5);
    assert_eq!(text_line.length(), 14);

    assert_eq!(shaped(&runs[0]).text().to_string_lossy(), "Hello");
    assert!(runs[4].is::<TextEndOfParagraph>());
}

#[test]
fn should_reset_bidi_levels_of_trailing_whitespaces_after_text_wrapping() {
    let _scope = TextTestScope::new();

    let default_properties = default_properties();
    let paragraph_properties = paragraph_properties_with(
        &default_properties,
        TextWrapping::Wrap,
        TextAlignment::Right,
        FlowDirection::RightToLeft,
    );

    let text_source = simple_source("aaa bbb", &default_properties);

    // Upstream wraps at 50 with its test font; with the fixed advance of 6 the
    // whole text is 42 wide, so 30 (five glyphs) is used to wrap after "aaa ".
    let first_line = format_line(&text_source, 0, 30.0, &paragraph_properties, None).unwrap();

    let runs = first_line.text_runs();

    assert_eq!(runs.len(), 2);

    let first = shaped(&runs[0]);
    let second = shaped(&runs[1]);

    assert_eq!(first.text().to_string_lossy(), " ");
    assert_eq!(second.text().to_string_lossy(), "aaa");

    assert_eq!(first.bidi_level(), 1);
    assert_eq!(second.bidi_level(), 2);
}

#[test]
fn should_reset_bidi_levels_of_trailing_whitespaces_after_text_wrapping_2() {
    let _scope = TextTestScope::new();

    let default_properties = default_properties();
    let paragraph_properties = paragraph_properties(&default_properties, TextWrapping::Wrap);

    let text_source = simple_source("\u{05D0}\u{05D0}\u{05D0} \u{05D1}\u{05D1}\u{05D1}", &default_properties);

    // Seven glyphs are 42 wide: 40 wraps after the space, as upstream.
    let first_line = format_line(&text_source, 0, 40.0, &paragraph_properties, None).unwrap();

    let runs = first_line.text_runs();

    assert_eq!(runs.len(), 2);

    let first = shaped(&runs[0]);
    let second = shaped(&runs[1]);

    assert_eq!(first.text().to_string_lossy(), "\u{05D0}\u{05D0}\u{05D0}");
    assert_eq!(second.text().to_string_lossy(), " ");

    assert_eq!(first.bidi_level(), 1);
    assert_eq!(second.bidi_level(), 0);
}

#[test]
fn should_format_text_runs_with_text_run_styles() {
    let _scope = TextTestScope::new();

    let text = "0123456789";

    let default_properties = default_properties();

    let spans: Vec<ValueSpan<Rc<dyn TextRunProperties>>> = vec![
        ValueSpan::new(0, 3, default_properties.clone()),
        ValueSpan::new(3, 3, run_properties(13.0)),
        ValueSpan::new(6, 3, run_properties(14.0)),
        ValueSpan::new(9, 1, default_properties.clone()),
    ];

    let text_source = FormattedTextSource::new(utf16(text), default_properties.clone(), Some(Rc::from(spans.clone())));

    let text_line = format_line(&text_source, 0, f64::INFINITY, &no_wrap(&default_properties), None).unwrap();

    assert_eq!(text_line.length(), text.len() as i32);

    let runs = text_line.text_runs();

    for (i, span) in spans.iter().enumerate() {
        assert_eq!(runs[i].length(), span.length());
    }

    // Addition: the width follows from the three em sizes (3 + 1 glyphs at 12, 3 at 13, 3 at 14).
    assert_eq!(
        text_line.width_including_trailing_whitespace(),
        4.0 * advance(12.0) + 3.0 * advance(13.0) + 3.0 * advance(14.0)
    );
}

#[test]
fn should_produce_unique_runs() {
    let _scope = TextTestScope::new();

    // The emoji and the CJK character come from the fallback fonts of the harness.
    for (text, number_of_runs) in [
        ("0123", 1),
        ("\r\n", 1),
        ("\u{1F44D}b", 2),
        ("a\u{1F44D}b", 3),
        ("a\u{1F44D}\u{5B50}b", 4),
    ] {
        let default_properties = default_properties();
        let text_source = SingleBufferTextSource::new(text, default_properties.clone());

        let text_line = format_line(&text_source, 0, f64::INFINITY, &no_wrap(&default_properties), None).unwrap();

        assert_eq!(text_line.text_runs().len(), number_of_runs, "{text:?}");
    }
}

/// Addition: the fallback runs carry the typeface of the font that covers them.
#[test]
fn fallback_runs_use_the_matched_typeface() {
    let _scope = TextTestScope::new();

    let default_properties = default_properties();
    let text_source = SingleBufferTextSource::new("a\u{1F44D}\u{5B50}b", default_properties.clone());

    let text_line = format_line(&text_source, 0, f64::INFINITY, &no_wrap(&default_properties), None).unwrap();

    let families: Vec<String> = text_line
        .text_runs()
        .iter()
        .map(|run| run.properties().unwrap().cached_glyph_typeface().family_name().to_owned())
        .collect();

    assert_eq!(families, [DEFAULT_FAMILY, EMOJI_FAMILY, CJK_FAMILY, DEFAULT_FAMILY]);

    // The emoji font of the harness is twice as wide.
    assert_eq!(text_line.width(), 5.0 * advance(EM));
}

#[test]
fn should_not_absorb_whitespace_into_a_fallback_run() {
    let _scope = TextTestScope::new();

    let default_properties = default_properties();

    let text_source =
        SingleBufferTextSource::new("\u{1F44D} \u{1F44D} \u{1F44D} \u{1F44D}", default_properties.clone());

    let text_line = format_line(&text_source, 0, f64::INFINITY, &no_wrap(&default_properties), None).unwrap();

    // Four emoji in a fallback font, separated by three spaces the primary font covers:
    // the spaces keep the primary's metrics instead of the emoji font's, so they form
    // runs of their own.
    let runs = text_line.text_runs();

    assert_eq!(runs.len(), 7);

    for (i, run) in runs.iter().enumerate() {
        let is_space = i % 2 == 1;

        assert_eq!(run.length(), if is_space { 1 } else { 2 });
        assert_eq!(is_space, default_properties.typeface() == run.properties().unwrap().typeface());
    }
}

#[test]
fn should_split_run_on_script() {
    let _scope = TextTestScope::new();

    let default_properties = default_properties();

    let text_source = SingleBufferTextSource::new(
        "ABCD\u{0627}\u{0644}\u{062F}\u{0648}\u{0644}\u{064A}",
        default_properties.clone(),
    );

    let text_line = format_line(&text_source, 0, f64::INFINITY, &no_wrap(&default_properties), None).unwrap();

    assert_eq!(text_line.text_runs()[0].length(), 4);
}

#[test]
fn should_wrap_with_overflow() {
    let _scope = TextTestScope::new();

    // U+10437 is covered by no font of the harness: it shapes to the missing glyph.
    for (text, expected_characters_per_line, expected_number_of_lines) in
        [("\u{10437}\u{10437}\u{10437}\u{10437}\u{10437}", 10, 1), ("01234 56789 01234 56789", 6, 4)]
    {
        let default_properties = default_properties();
        let paragraph_properties = paragraph_properties(&default_properties, TextWrapping::WrapWithOverflow);

        let text_source =
            SingleBufferTextSource::with_end_of_paragraph("ABCDEFHFFHFJHKHFK", default_properties.clone(), true);

        let line = format_line(&text_source, 0, 33.0, &paragraph_properties, None).unwrap();

        // The remaining runs of the wrapped line are never formatted: release them.
        release_remaining_runs(&*line);

        let text_source = SingleBufferTextSource::new(text, default_properties.clone());

        let text_length = utf16_len(text);

        let mut number_of_lines = 0;
        let mut current_position = 0;

        while current_position < text_length {
            let text_line = format_line(&text_source, current_position, 1.0, &paragraph_properties, None).unwrap();

            if text_length - current_position > expected_characters_per_line {
                assert_eq!(text_line.length(), expected_characters_per_line);
            }

            current_position += text_line.length();
            number_of_lines += 1;

            release_remaining_runs(&*text_line);
        }

        assert_eq!(number_of_lines, expected_number_of_lines);
    }
}

/// Takes the runs a wrapped line left for the next line (nothing else does
/// when a test formats every line from the text source again).
fn release_remaining_runs(line: &dyn TextLine) {
    if let Some(line_break) = line.text_line_break() {
        let _ = WrappingTextLineBreak::acquire_remaining_runs(&line_break);
    }
}

#[test]
fn text_trimming_should_trim_correctly() {
    const WIDTH: f64 = 160.0;
    const EM_SIZE: f64 = 20.0;

    let text = "one \u{05E9}\u{05EA}\u{05D9}\u{05D9}\u{05DD} three \u{05D0}\u{05E8}\u{05D1}\u{05E2}";

    // The expectations concatenate the run texts in visual run order. They are
    // recomputed for the fixed advance (10 at em size 20, so sixteen glyphs
    // fit): the default font of the harness covers Hebrew, so the runs are cut
    // at the bidi level changes only.
    //
    // Left to right: "one " (40), Hebrew (50), " three " (70), Hebrew (40). 150 are
    // available next to the ellipsis: " three" fits (character) and no word of " three " does.
    //
    // Right to left: "one" (30), " Hebrew " (70), "three" (50) take the 150 exactly; the
    // collapsed runs are reordered with the ellipsis (level 1) leftmost.
    for (trimmed, direction, word_ellipsis) in [
        ("one \u{05E9}\u{05EA}\u{05D9}\u{05D9}\u{05DD} three\u{2026}", FlowDirection::LeftToRight, false),
        ("\u{2026}three \u{05E9}\u{05EA}\u{05D9}\u{05D9}\u{05DD} one", FlowDirection::RightToLeft, false),
        ("one \u{05E9}\u{05EA}\u{05D9}\u{05D9}\u{05DD}\u{2026}", FlowDirection::LeftToRight, true),
        ("\u{2026}three \u{05E9}\u{05EA}\u{05D9}\u{05D9}\u{05DD} one", FlowDirection::RightToLeft, true),
    ] {
        let _scope = TextTestScope::new();

        let default_properties = run_properties(EM_SIZE);
        let paragraph_properties =
            paragraph_properties_with(&default_properties, TextWrapping::NoWrap, TextAlignment::Start, direction);

        let text_source = simple_source(text, &default_properties);

        let text_line = format_line(&text_source, 0, f64::INFINITY, &paragraph_properties, None).unwrap();

        let text_trimming =
            if word_ellipsis { <dyn TextTrimming>::word_ellipsis() } else { <dyn TextTrimming>::character_ellipsis() };

        let collapsing_properties = text_trimming.create_collapsing_properties(&TextCollapsingCreateInfo::new(
            WIDTH,
            default_properties.clone(),
            direction,
        ));

        let collapsed_line = text_line.clone().collapse(&[Some(collapsing_properties)]);

        assert!(collapsed_line.has_collapsed());
        assert_eq!(line_text(&*collapsed_line), trimmed, "{direction:?}, word: {word_ellipsis}");
        assert!(collapsed_line.width_including_trailing_whitespace() <= WIDTH);
    }
}

#[test]
fn should_wrap() {
    let _scope = TextTestScope::new();

    // Upstream measures the advance of 'a' in two real fonts; here every glyph has the fixed advance.
    for (text, number_of_characters_per_line) in [
        (
            "Whether to turn off HTTPS. This option only applies if Individual, \
             IndividualB2C, SingleOrg, or MultiOrg aren't used for &#8209;&#8209;auth.",
            40,
        ),
        ("01234 56789 01234 56789", 7),
    ] {
        let units: Vec<u16> = text.encode_utf16().collect();

        let mut expected: Vec<i32> =
            LineBreakEnumerator::new(&units).map(|line_break| line_break.position_wrap() as i32 - 1).collect();

        let default_properties = default_properties();
        let text_source = SingleBufferTextSource::new(text, default_properties.clone());
        let paragraph_properties = paragraph_properties(&default_properties, TextWrapping::Wrap);

        let paragraph_width = advance(EM) * number_of_characters_per_line as f64;

        let mut current_position = 0;

        while current_position < units.len() as i32 {
            let text_line =
                format_line(&text_source, current_position, paragraph_width, &paragraph_properties, None).unwrap();

            let end = text_line.first_text_source_index() + text_line.length() - 1;

            let index = expected.iter().position(|&position| position == end).expect("a line ends at a line break");

            expected.drain(..=index);

            current_position += text_line.length();

            release_remaining_runs(&*text_line);
        }
    }
}

#[test]
fn should_produce_fixed_height_lines() {
    let _scope = TextTestScope::new();

    let default_properties = default_properties();
    let text_source = SingleBufferTextSource::new("012345", default_properties.clone());

    let paragraph_properties: Rc<dyn TextParagraphProperties> = Rc::new(GenericTextParagraphProperties::with_options(
        default_properties.clone(),
        TextAlignment::Left,
        TextWrapping::NoWrap,
        50.0,
        0.0,
    ));

    let text_line = format_line(&text_source, 0, f64::INFINITY, &paragraph_properties, None).unwrap();

    assert_eq!(text_line.height(), 50.0);
}

#[test]
fn should_not_produce_text_line_wider_than_paragraph_width() {
    let _scope = TextTestScope::new();

    let text = "Multiline TextBlock with TextWrapping.\r\rLorem ipsum dolor sit amet, consectetur adipiscing elit. \
                Vivamus magna. Cras in mi at felis aliquet congue. Ut a est eget ligula molestie gravida. Curabitur massa. \
                Donec eleifend, libero at sagittis mollis, tellus est malesuada tellus, at luctus turpis elit sit amet quam. \
                Vivamus pretium ornare est.";

    let default_properties = default_properties();
    let paragraph_properties = paragraph_properties(&default_properties, TextWrapping::Wrap);
    let text_source = SingleBufferTextSource::new(text, default_properties.clone());

    let mut text_source_index = 0;

    while text_source_index < text.len() as i32 {
        let text_line = format_line(&text_source, text_source_index, 200.0, &paragraph_properties, None).unwrap();

        assert!(text_line.width() <= 200.0);

        text_source_index += text_line.length();

        release_remaining_runs(&*text_line);
    }

    assert_eq!(text_source_index, text.len() as i32);
}

#[test]
fn wrap_should_not_produce_empty_lines() {
    let _scope = TextTestScope::new();

    let text = "012345";

    let default_properties = default_properties();
    let paragraph_properties = paragraph_properties(&default_properties, TextWrapping::Wrap);
    let text_source = SingleBufferTextSource::new(text, default_properties.clone());

    let mut text_source_index = 0;

    while text_source_index < text.len() as i32 {
        let text_line = format_line(&text_source, text_source_index, 3.0, &paragraph_properties, None).unwrap();

        assert_ne!(text_line.length(), 0);

        text_source_index += text_line.length();

        release_remaining_runs(&*text_line);
    }

    assert_eq!(text_source_index, text.len() as i32);
}

#[test]
fn should_produce_wrapped_and_trimmed_lines() {
    let _scope = TextTestScope::new();

    let text = "Lorem ipsum dolor sit amet, consectetur adipisicing elit, sed do eiusmod tempor";

    // Recomputed for the fixed advance: "Lorem" is 24 wide per glyph (em 48), everything else
    // 16 (em 32; the bold and italic typefaces of upstream resolve to the same test font).
    // 300 fits "Lorem ipsum " (120 + 7 * 16 = 232) but not "dolor"; then eighteen glyphs per line.
    let expected_lines = ["Lorem ipsum ", "dolor sit amet, ", "consectetur ", "adipisicing elit, ", "sed do eiusmod ", "tempor"];

    let default_properties = properties_with_foreground(32.0, Brushes::black());

    let style_spans: Vec<ValueSpan<Rc<dyn TextRunProperties>>> = vec![
        ValueSpan::new(0, 5, run_properties(48.0)),
        ValueSpan::new(6, 11, run_properties(32.0)),
        ValueSpan::new(28, 28, run_properties(32.0)),
    ];

    let text_source = FormattedTextSource::new(utf16(text), default_properties.clone(), Some(Rc::from(style_spans)));

    let paragraph_properties = paragraph_properties(&default_properties, TextWrapping::WrapWithOverflow);

    let mut current_position = 0;
    let mut current_height = 0.0;
    let mut current_line_index = 0;

    while current_position < text.len() as i32 && current_line_index < expected_lines.len() {
        let mut text_line = format_line(&text_source, current_position, 300.0, &paragraph_properties, None).unwrap();

        release_remaining_runs(&*text_line);

        current_position += text_line.length();

        if text_line.width() > 300.0 || current_height + text_line.height() > 240.0 {
            let collapsing_properties: Rc<dyn TextCollapsingProperties> = Rc::new(TextTrailingWordEllipsis::new(
                crate::media::text_trimming::DEFAULT_ELLIPSIS_CHAR,
                300.0,
                default_properties.clone(),
                FlowDirection::LeftToRight,
            ));

            text_line = text_line.collapse(&[Some(collapsing_properties)]);
        }

        current_height += text_line.height();

        let start = text_line.first_text_source_index() as usize;
        let current_text = &text[start..start + text_line.length() as usize];

        assert_eq!(current_text, expected_lines[current_line_index]);

        current_line_index += 1;
    }

    assert_eq!(current_line_index, expected_lines.len());
}

#[test]
fn should_align_text_line() {
    let _scope = TextTestScope::new();

    let hebrew = "\u{05E9}\u{05E0}\u{05D1}\u{05D2}\u{05E7}";

    for (text, text_alignment, flow_direction) in [
        ("0123456789", TextAlignment::Left, FlowDirection::LeftToRight),
        ("0123456789", TextAlignment::Center, FlowDirection::LeftToRight),
        ("0123456789", TextAlignment::Right, FlowDirection::LeftToRight),
        ("0123456789", TextAlignment::Left, FlowDirection::RightToLeft),
        ("0123456789", TextAlignment::Center, FlowDirection::RightToLeft),
        ("0123456789", TextAlignment::Right, FlowDirection::RightToLeft),
        (hebrew, TextAlignment::Left, FlowDirection::RightToLeft),
        (hebrew, TextAlignment::Center, FlowDirection::RightToLeft),
        (hebrew, TextAlignment::Right, FlowDirection::RightToLeft),
    ] {
        let default_properties = default_properties();
        let paragraph_properties =
            paragraph_properties_with(&default_properties, TextWrapping::NoWrap, text_alignment, flow_direction);

        let text_source = SingleBufferTextSource::new(text, default_properties.clone());

        let text_line = format_line(&text_source, 0, 100.0, &paragraph_properties, None).unwrap();

        let expected_offset = match text_alignment {
            TextAlignment::Center => 50.0 - text_line.width() / 2.0,
            TextAlignment::Right => 100.0 - text_line.width_including_trailing_whitespace(),
            _ => 0.0,
        };

        assert_eq!(text_line.start(), expected_offset);

        // Addition: the concrete values for the fixed advance (ten glyphs are 60, five are 30 wide).
        let width = utf16_len(text) as f64 * advance(EM);

        assert_eq!(text_line.width(), width);
    }
}

/// Addition: start, end, justify and content detection resolve against the
/// paragraph's (or the content's) direction.
#[test]
fn should_align_text_line_logically() {
    let _scope = TextTestScope::new();

    let hebrew = "\u{05E9}\u{05E0}\u{05D1}\u{05D2}\u{05E7}";

    for (text, text_alignment, flow_direction, expected_start) in [
        ("01234", TextAlignment::Start, FlowDirection::LeftToRight, 0.0),
        ("01234", TextAlignment::Start, FlowDirection::RightToLeft, 70.0),
        ("01234", TextAlignment::End, FlowDirection::LeftToRight, 70.0),
        ("01234", TextAlignment::End, FlowDirection::RightToLeft, 0.0),
        ("01234", TextAlignment::Justify, FlowDirection::RightToLeft, 70.0),
        ("abcde", TextAlignment::DetectFromContent, FlowDirection::LeftToRight, 0.0),
        (hebrew, TextAlignment::DetectFromContent, FlowDirection::LeftToRight, 70.0),
    ] {
        let default_properties = default_properties();
        let paragraph_properties =
            paragraph_properties_with(&default_properties, TextWrapping::NoWrap, text_alignment, flow_direction);

        let text_source = SingleBufferTextSource::new(text, default_properties.clone());

        let text_line = format_line(&text_source, 0, 100.0, &paragraph_properties, None).unwrap();

        assert_eq!(text_line.start(), expected_start, "{text_alignment:?} {flow_direction:?}");
    }
}

#[test]
fn should_format_line_with_emergency_breaks() {
    let _scope = TextTestScope::new();

    let default_properties = default_properties();
    let paragraph_properties = paragraph_properties(&default_properties, TextWrapping::Wrap);
    let text_source =
        SingleBufferTextSource::new("0123456789_0123456789_0123456789_0123456789", default_properties.clone());

    let text_line = format_line(&text_source, 0, 33.0, &paragraph_properties, None).unwrap();

    // 33 fits five glyphs of 6.
    assert_eq!(text_line.length(), 5);

    let line_break = text_line.text_line_break().unwrap();

    assert!(WrappingTextLineBreak::is_wrapping(&line_break));
    assert!(line_break.is_split());

    let remaining_runs = WrappingTextLineBreak::acquire_remaining_runs(&line_break).unwrap();

    assert!(!remaining_runs.is_empty());

    // A second call gives nothing.
    assert!(WrappingTextLineBreak::acquire_remaining_runs(&line_break).is_none());
}

#[test]
fn should_not_alter_text_runs_after_text_styles_were_applied() {
    let _scope = TextTestScope::new();

    for text in [
        "\u{05E4}\u{05E2}\u{05D9}\u{05DC}\u{05D5}\u{05EA} \u{05D4}\u{05D1}\u{05D9}\u{05E0}\u{05D0}\u{05D5}\u{05DD}, W3C!",
        "abcABC",
        "\u{05D8}\u{05D8}\u{05D8}\u{05D8} abcDEF \u{05D8}\u{05D8}\u{05D8}\u{05D8}",
    ] {
        let default_properties = default_properties();
        let paragraph_properties = no_wrap(&default_properties);

        let foreground: Rc<dyn IBrush> = Brushes::red();

        let glyphs_of = |line: &dyn TextLine| -> Vec<u16> {
            line.text_runs()
                .iter()
                .flat_map(|run| shaped(run).glyph_run().glyph_infos().borrow().iter().map(|glyph| glyph.glyph_index).collect::<Vec<_>>())
                .collect()
        };

        let expected_text_line = format_line(
            &SingleBufferTextSource::new(text, default_properties.clone()),
            0,
            f64::INFINITY,
            &paragraph_properties,
            None,
        )
        .unwrap();

        let expected_glyphs = glyphs_of(&*expected_text_line);

        let text_length = utf16_len(text);

        for i in 0..text_length {
            let mut j = 1;

            while i + j < text_length {
                let spans: Vec<ValueSpan<Rc<dyn TextRunProperties>>> =
                    vec![ValueSpan::new(i, j, properties_with_foreground(EM, foreground.clone()))];

                let text_source =
                    FormattedTextSource::new(utf16(text), default_properties.clone(), Some(Rc::from(spans)));

                let text_line = format_line(&text_source, 0, f64::INFINITY, &paragraph_properties, None).unwrap();

                assert_eq!(glyphs_of(&*text_line), expected_glyphs, "span {i}+{j} of {text:?}");

                j += 1;
            }
        }
    }
}

#[test]
fn should_format_line_with_drawable_runs() {
    let _scope = TextTestScope::new();

    let default_run_properties = default_properties();
    let paragraph_properties = no_wrap(&default_run_properties);

    let text = "Hello World ->";

    let text_source = ListTextSource::new(vec![
        Rc::new(TextCharacters::from_str(text, default_run_properties.clone())),
        Rc::new(CustomDrawableRun::without_properties(Size::new(50.0, 50.0), 0.0)),
        Rc::new(TextCharacters::from_str(text, default_run_properties.clone())),
    ]);

    let text_line =
        <dyn TextFormatter>::current().format_line(&text_source, 0, f64::INFINITY, &paragraph_properties, None).unwrap();

    let runs = text_line.text_runs();

    assert_eq!(runs.len(), 3);
    assert!(runs[1].is::<CustomDrawableRun>());

    // Addition: the drawable run takes part in the metrics of the line.
    assert_eq!(text_line.width(), 28.0 * advance(EM) + 50.0);
    assert_eq!(text_line.length(), 29);
}

#[test]
fn should_format_with_end_of_line_run() {
    let _scope = TextTestScope::new();

    let default_run_properties = default_properties();
    let paragraph_properties = no_wrap(&default_run_properties);

    let text_line = format_line(&EndOfLineTextSource, 0, f64::INFINITY, &paragraph_properties, None).unwrap();

    assert!(text_line.text_line_break().is_some());
    assert_eq!(text_line.length(), DEFAULT_TEXT_SOURCE_LENGTH);
}

struct EmptyTextSource;

impl ITextSource for EmptyTextSource {
    fn get_text_run(&self, _text_source_index: i32) -> Option<Rc<dyn TextRun>> {
        None
    }
}

#[test]
fn should_return_null_for_empty_text_source() {
    let _scope = TextTestScope::new();

    let default_run_properties = default_properties();
    let paragraph_properties = no_wrap(&default_run_properties);

    assert!(format_line(&EmptyTextSource, 0, f64::INFINITY, &paragraph_properties, None).is_none());
}

#[test]
fn should_retain_text_end_of_paragraph_with_text_wrapping() {
    let _scope = TextTestScope::new();

    let default_run_properties = default_properties();
    let paragraph_properties = paragraph_properties(&default_run_properties, TextWrapping::Wrap);

    let text = "Hello World";
    let text_source = simple_source(text, &default_run_properties);

    let mut pos = 0;
    let mut previous_line_break: Option<Rc<TextLineBreak>> = None;
    let mut text_line: Option<Rc<dyn TextLine>> = None;

    while pos < text.len() as i32 {
        let line = format_line(&text_source, pos, 30.0, &paragraph_properties, previous_line_break.as_ref()).unwrap();

        pos += line.length();
        previous_line_break = line.text_line_break();
        text_line = Some(line);
    }

    let line_break = text_line.unwrap().text_line_break().unwrap();

    assert!(line_break.text_end_of_line().is_some());
}

#[test]
fn should_hit_test_string_with_invisible_runs() {
    let _scope = TextTestScope::new();

    let default_run_properties = default_properties();
    let paragraph_properties = no_wrap(&default_run_properties);

    let source = ListTextSource::new(vec![
        Rc::new(InvisibleRun::new(1)),
        Rc::new(TextCharacters::from_str("Hello", properties_with_foreground(EM, Brushes::black()))),
        Rc::new(InvisibleRun::new(1)),
        Rc::new(TextCharacters::from_str("world", properties_with_foreground(EM, Brushes::red()))),
    ]);

    let text_line = format_line(&source, 0, f64::INFINITY, &paragraph_properties, None).unwrap();

    for offset in [3, 8] {
        let glyph_center = text_line.get_text_bounds(offset, 1)[0].rectangle().center();

        let hit = text_line.get_character_hit_from_distance(glyph_center.x);

        assert_eq!(hit.first_character_index(), offset);
    }
}

#[test]
fn get_text_bounds_for_text_line_with_zero_width_spaces_does_not_freeze() {
    let _scope = TextTestScope::new();

    let default_run_properties = default_properties();
    let paragraph_properties = no_wrap(&default_run_properties);

    let source = ListTextSource::new(vec![
        Rc::new(TextCharacters::from_str("\u{200B}\u{200B}", default_run_properties.clone())),
        Rc::new(InvisibleRun::new(1)),
        Rc::new(TextEndOfParagraph::new()),
    ]);

    let text_line = format_line(&source, 0, f64::INFINITY, &paragraph_properties, None).unwrap();

    let bounds = text_line.get_text_bounds(0, 3);

    assert_eq!(bounds.len(), 1);
    assert_eq!(bounds[0].text_run_bounds().len(), 2);
}

#[test]
fn line_formatting_for_oversized_embedded_runs_does_not_produce_empty_lines() {
    let _scope = TextTestScope::new();

    for wrapping in [TextWrapping::NoWrap, TextWrapping::Wrap, TextWrapping::WrapWithOverflow] {
        let default_run_properties = default_properties();
        let paragraph_properties = paragraph_properties(&default_run_properties, wrapping);

        let source =
            ListTextSource::new(vec![Rc::new(CustomDrawableRun::without_properties(Size::new(200.0, 10.0), 0.0))]);

        let text_line = format_line(&source, 0, 100.0, &paragraph_properties, None).unwrap();

        assert_eq!(text_line.width_including_trailing_whitespace(), 200.0);
    }
}

#[test]
fn line_formatting_for_oversized_embedded_runs_inside_normal_text_does_not_produce_empty_lines() {
    let _scope = TextTestScope::new();

    for wrapping in [TextWrapping::NoWrap, TextWrapping::Wrap, TextWrapping::WrapWithOverflow] {
        let default_run_properties = default_properties();
        let paragraph_properties = paragraph_properties(&default_run_properties, wrapping);

        let source = ListTextSource::new(vec![
            Rc::new(TextCharacters::from_str("Hello", default_run_properties.clone())),
            Rc::new(CustomDrawableRun::without_properties(Size::new(200.0, 10.0), 0.0)),
            Rc::new(InvisibleRun::new(1)),
            Rc::new(TextEndOfLine::new()),
            Rc::new(TextCharacters::from_str("world", default_run_properties.clone())),
            Rc::new(TextEndOfParagraph::with_length(1)),
        ]);

        let mut lines: Vec<Rc<dyn TextLine>> = Vec::new();
        let mut dcp = 0;

        for c in 0.. {
            assert!(c < 1000, "Infinite loop");

            let text_line = format_line(&source, dcp, 30.0, &paragraph_properties, None).unwrap();

            dcp += text_line.length();

            let line_break = text_line.text_line_break();

            release_remaining_runs(&*text_line);

            lines.push(text_line);

            let ends_paragraph = line_break
                .as_ref()
                .and_then(|line_break| line_break.text_end_of_line())
                .is_some_and(|end_of_line| end_of_line.is::<TextEndOfParagraph>());

            if ends_paragraph {
                break;
            }
        }

        assert!(!lines.is_empty());
        assert!(lines.iter().all(|line| line.length() > 0));
    }
}

struct IncrementalTabProperties {
    default_text_run_properties: Rc<dyn TextRunProperties>,
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
    let _scope = TextTestScope::new();

    let default_run_properties = default_properties();
    let paragraph_properties: Rc<dyn TextParagraphProperties> =
        Rc::new(IncrementalTabProperties { default_text_run_properties: default_run_properties.clone() });

    let source = ListTextSource::new(vec![Rc::new(TextCharacters::from_str("ff", default_run_properties.clone()))]);

    let text_line = format_line(&source, 0, f64::INFINITY, &paragraph_properties, None).unwrap();

    let backspace_hit = text_line.get_backspace_caret_character_hit(crate::media::CharacterHit::new(2));

    assert_eq!(backspace_hit.first_character_index(), 1);
    assert_eq!(backspace_hit.trailing_length(), 0);
}

/// Addition: a tab takes the incremental tab width of the paragraph, or four spaces.
#[test]
fn tab_uses_the_incremental_tab_width() {
    let _scope = TextTestScope::new();

    let default_run_properties = default_properties();

    let source = SingleBufferTextSource::new("a\tb", default_run_properties.clone());

    let paragraph_properties: Rc<dyn TextParagraphProperties> =
        Rc::new(IncrementalTabProperties { default_text_run_properties: default_run_properties.clone() });

    let text_line = format_line(&source, 0, f64::INFINITY, &paragraph_properties, None).unwrap();

    assert_eq!(text_line.width(), 2.0 * advance(EM) + 64.0);

    let text_line = format_line(&source, 0, f64::INFINITY, &no_wrap(&default_run_properties), None).unwrap();

    assert_eq!(text_line.width(), 6.0 * advance(EM));
}

#[test]
fn should_wrap_chinese() {
    let _scope = TextTestScope::new();

    let default_run_properties = default_properties();
    let paragraph_properties = paragraph_properties(&default_run_properties, TextWrapping::Wrap);

    let text = "\u{4E00}\u{4E8C}\u{4E09}\u{56DB} TEXT \u{4E00}\u{4E8C}\u{4E09}\u{56DB}\u{4E94}\u{516D}\u{4E03}\u{516B}\u{4E5D}\u{5341}\u{96F6}";

    let text_line =
        format_line(&simple_source(text, &default_run_properties), 0, 120.0, &paragraph_properties, None).unwrap();

    // 120 fits twenty glyphs: the four ideographs, " TEXT " and ten more ideographs
    // (an ideograph can break after itself), in three runs.
    assert_eq!(text_line.text_runs().len(), 3);
    assert_eq!(text_line.length(), 20);
}

#[test]
fn drawable_run_with_same_baseline_and_size_should_not_alter_line_height() {
    // The font has no line gap (as upstream's): the baseline of a line is then its ascent.
    let _scope = TextTestScope::without_line_gap();

    let default_run_properties = default_properties();
    let paragraph_properties = paragraph_properties(&default_run_properties, TextWrapping::Wrap);

    let embedded_text_line =
        format_line(&simple_source("ABC", &default_run_properties), 0, 120.0, &paragraph_properties, None).unwrap();

    let expected_height = embedded_text_line.height();
    let expected_baseline = embedded_text_line.baseline();

    assert_eq!(expected_height, EM);

    let text_source = ListTextSource::new(vec![
        Rc::new(TextCharacters::from_str("ABC", default_run_properties.clone())),
        Rc::new(CustomDrawableRun::without_properties(
            Size::new(embedded_text_line.width(), embedded_text_line.height()),
            embedded_text_line.baseline(),
        )),
    ]);

    let text_line = format_line(&text_source, 0, f64::INFINITY, &paragraph_properties, None).unwrap();

    assert_eq!(text_line.height(), expected_height);
    assert_eq!(text_line.baseline(), expected_baseline);
}

#[test]
fn drawable_run_with_same_baseline_and_bigger_height_should_not_alter_baseline() {
    // The font has no line gap (as upstream's): the baseline of a line is then its ascent.
    let _scope = TextTestScope::without_line_gap();

    let default_run_properties = default_properties();
    let paragraph_properties = paragraph_properties(&default_run_properties, TextWrapping::Wrap);

    let embedded_text_line =
        format_line(&simple_source("ABC", &default_run_properties), 0, 120.0, &paragraph_properties, None).unwrap();

    let expected_height = embedded_text_line.height() + 10.0;
    let embedded_size = Size::new(embedded_text_line.width(), expected_height);
    let expected_baseline = embedded_text_line.baseline();

    let text_source = ListTextSource::new(vec![
        Rc::new(TextCharacters::from_str("ABC", default_run_properties.clone())),
        Rc::new(CustomDrawableRun::without_properties(embedded_size, expected_baseline)),
    ]);

    let text_line = format_line(&text_source, 0, f64::INFINITY, &paragraph_properties, None).unwrap();

    assert_eq!(text_line.height(), expected_height);
    assert_eq!(text_line.baseline(), expected_baseline);
}

/// Addition: every line of a wrapped paragraph, formatted through the line breaks.
#[test]
fn wrapped_lines_continue_through_the_line_breaks() {
    let _scope = TextTestScope::new();

    let default_run_properties = default_properties();
    let paragraph_properties = paragraph_properties(&default_run_properties, TextWrapping::Wrap);

    let text = "The quick brown fox jumps";
    let text_source = simple_source(text, &default_run_properties);

    // 60 fits ten glyphs.
    let lines = format_lines(&text_source, text.len() as i32, 60.0, &paragraph_properties);

    assert_eq!(
        lines.iter().map(|line| line_text(&**line)).collect::<Vec<_>>(),
        ["The quick ", "brown fox ", "jumps"]
    );

    assert_eq!(lines.iter().map(|line| line.first_text_source_index()).collect::<Vec<_>>(), [0, 10, 20]);
}

// ── a run that shapes to no glyphs (upstream's EmptyShapedBufferTests) ──

#[test]
fn line_break_that_shapes_to_no_glyphs_keeps_its_characters() {
    let scope = TextTestScope::new();

    // Upstream uses a font without glyphs for the line break; the shaper of the harness is told to drop it.
    scope.shaper().set_drop_line_breaks(true);

    let default_properties = default_properties();

    let text_line = format_line(
        &SingleBufferTextSource::new("\r\nfoo", default_properties.clone()),
        0,
        100.0,
        &paragraph_properties(&default_properties, TextWrapping::Wrap),
        None,
    )
    .unwrap();

    let runs = text_line.text_runs();

    assert_eq!(runs.len(), 1);

    let run = shaped(&runs[0]);

    assert_eq!(run.length(), 2);
    assert_eq!(text_line.length(), 2);

    // The premise of the test: shaping really did produce nothing for these characters.
    assert_eq!(run.shaped_buffer().length(), 0);
}

/// Upstream formats this through `TextLayout` (that form is in `text_layout_tests.rs`); the lines are
/// formatted directly here.
#[test]
fn should_wrap_text_that_starts_with_a_line_break() {
    let scope = TextTestScope::new();

    scope.shaper().set_drop_line_breaks(true);

    let default_properties = default_properties();

    let text = "\r\nPassword update failed";

    let lines = format_lines(
        &SingleBufferTextSource::new(text, default_properties.clone()),
        text.len() as i32,
        290.0,
        &paragraph_properties(&default_properties, TextWrapping::Wrap),
    );

    assert_eq!(lines.len(), 2);
    assert_eq!(lines[0].length(), 2);
    assert_eq!(lines[1].length(), 22);
}
