//! The break tables of shaped buffers (desktop-performance.md section 9.16): a layout that reads
//! its break opportunities from the table of the text its runs were split from makes the lines of
//! one that finds them again from the start of every line, as upstream does.

use crate::media::text_formatting::shaped_buffer::BREAK_TABLES;
use crate::media::text_formatting::testing::TextTestScope;
use crate::media::text_formatting::{TextLayout, TextLayoutOptions};
use crate::media::{Brushes, TextTrimming, TextWrapping, Typeface};

const TEXTS: [&str; 12] = [
    "Lorem ipsum dolor sit amet, consectetur adipiscing elit; sed do eiusmod tempor incididunt.",
    "\"Quoted words\" and (parenthesized words) and [brackets] stay with their neighbours: « guillemets ».",
    "Prices $1,234.56 and -2.31% and 1/2 and 10:30 and 3.14159 and +44 (0) 20 7946 0958 hold together.",
    "Hyphen-ated words, en–dash and em—dash, slashes/like/this, and https://example.com/a/long/path?q=1.",
    "中文文本没有空格但是可以在字符之间换行。日本語のテキストも同じです。한국어 문장은 띄어쓰기를 합니다.",
    "Emoji 👩‍👩‍👧‍👦 families and flags 🇵🇱🇺🇦🇯🇵 and skin tones 👍🏽 next to words 👍 here.",
    "No\u{00A0}break\u{00A0}spaces and narrow\u{202F}no-break and zero\u{200B}width spaces and soft\u{00AD}hyphens.",
    "Combining marks: e\u{0301}te\u{0301} cafe\u{0301} na\u{0308}ive and Hangul jamo \u{1100}\u{1161}\u{11A8} together.",
    "Two  spaces   and    four     spaces between words and a trailing space ",
    "Lines\nwith required\r\nbreaks and a\u{2028}line separator and a\u{2029}paragraph separator.",
    "Supercalifragilisticexpialidocious antidisestablishmentarianism pneumonoultramicroscopic words.",
    "Mixed עברית text and العربية text with English words between them, numbers 123 and 456.",
];

fn layout(text: &str, width: f64, wrapping: TextWrapping) -> Vec<(i32, i32)> {
    let layout = TextLayout::new(
        text,
        Typeface::default_typeface(),
        TextLayoutOptions {
            font_size: 12.0,
            foreground: Some(Brushes::black()),
            max_width: width,
            text_trimming: Some(<dyn TextTrimming>::none()),
            text_wrapping: wrapping,
            ..TextLayoutOptions::default()
        },
    );
    let lines = layout.text_lines().iter().map(|line| (line.first_text_source_index(), line.length())).collect();
    layout.dispose();
    lines
}

#[test]
fn layouts_with_break_tables_make_the_lines_of_layouts_without() {
    let _scope = TextTestScope::new();
    let mut compared = 0;
    for text in TEXTS {
        for wrapping in [TextWrapping::Wrap, TextWrapping::WrapWithOverflow] {
            let mut width = 6.0;
            while width < 700.0 {
                BREAK_TABLES.with(|on| on.set(false));
                let expected = layout(text, width, wrapping);
                BREAK_TABLES.with(|on| on.set(true));
                let actual = layout(text, width, wrapping);
                assert_eq!(expected, actual, "{text:?} at {width}, {wrapping:?}");
                compared += 1;
                width += 1.75;
            }
        }
    }
    assert!(compared > 4_000);
}
