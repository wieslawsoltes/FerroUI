//! The break tables of shaped buffers (desktop-performance.md section 9.16): a layout that reads
//! its break opportunities from the table of the text its runs were split from makes the lines of
//! one that finds them again from the start of every line, as upstream does.

use crate::media::text_formatting::shaped_buffer::{BreakTable, BREAK_TABLES};
use crate::media::text_formatting::testing::TextTestScope;
use crate::media::text_formatting::unicode::LineBreak;
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

/// Pieces an edit puts into a text: words, spaces, punctuation the rules look around, digits,
/// marks that lean on the unit before, emoji sequences, ideographs, right-to-left letters and
/// required breaks.
const PIECES: [&str; 40] = [
    "word", "a", "of the", " ", "  ", "42", "1,234.56", "$", "%", "-", "\u{2014}", "/", ".", ", ", "; ", "(", ")", "\"",
    "'", "\u{00AB} ", " \u{00BB}", "\u{0301}", "\u{200D}", "\u{200B}", "\u{00A0}", "\u{00AD}", "\u{2060}", "\n",
    "\r\n", "\u{2028}", "\u{4E2D}\u{6587}", "\u{3002}", "\u{1F1F5}\u{1F1F1}", "\u{1F1FA}",
    "\u{1F469}\u{200D}\u{1F467}", "\u{1F44D}\u{1F3FD}", "\u{05E2}\u{05D1}", "\u{1100}\u{1161}", "x-y", "e.g. ",
];

/// A small deterministic generator (the test needs no randomness from the system).
struct Xorshift(u64);

impl Xorshift {
    fn next(&mut self, below: usize) -> usize {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        (self.0 % below.max(1) as u64) as usize
    }
}

#[test]
fn the_break_table_of_an_edited_text_is_that_of_the_whole_text() {
    // The table of a text after an edit, made from the table of the text before (step 7 of
    // text-layout-performance.md), against an enumeration of the new text from its start:
    // random pieces inserted, removed and replaced at random code units (also inside a surrogate
    // pair or a sequence), each edit made on the text of the edit before.
    let mut random = Xorshift(0x2545_F491_4F6C_DD1D);
    let (mut edits, mut shorter) = (0, 0);
    for case in 0..TEXTS.len() * 10 {
        let mut text: Vec<u16> = TEXTS[case % TEXTS.len()].repeat(1 + case % 4).encode_utf16().collect();
        for step in 0..60 {
            let old = text.clone();
            let at = random.next(text.len() + 1);
            let removed = match random.next(4) {
                0 => 0,
                1 => 1,
                _ => random.next(12),
            }
            .min(text.len() - at);
            let mut inserted = Vec::new();
            for _ in 0..random.next(4) {
                inserted.extend(PIECES[random.next(PIECES.len())].encode_utf16());
            }
            text.splice(at..at + removed, inserted);
            if text.is_empty() {
                text.extend("word ".encode_utf16());
            }
            let (whole, edited) = BreakTable::of_edit(&old, &text);
            if whole != edited {
                // The first break that differs, with the breaks and the code units around it.
                let common = whole.len().min(edited.len());
                let first = whole.iter().zip(&edited).position(|(x, y)| x != y).unwrap_or(common);
                let around = |breaks: &[LineBreak]| {
                    breaks[first.saturating_sub(2)..(first + 3).min(breaks.len())].to_vec()
                };
                let at = whole[first.saturating_sub(1)].position_wrap();
                panic!(
                    "case {case}, step {step}: break {first} of {} -> {} code units: whole {:?}, edited {:?}, \
                     text {:04X?}",
                    old.len(),
                    text.len(),
                    around(&whole),
                    around(&edited),
                    &text[at.saturating_sub(8)..(at + 12).min(text.len())]
                );
            }
            edits += 1;
            shorter += usize::from(BreakTable::edit_enumerates(&old, &text) * 2 < text.len());
        }
    }
    // Most edits of these texts enumerate less than half of the text again.
    assert!(shorter * 2 >= edits, "{shorter} of {edits} edits enumerated less than half of the text");
}

#[test]
fn a_word_replaced_in_a_long_paragraph_enumerates_its_surroundings() {
    let words = |changed: usize, value: usize| -> Vec<u16> {
        let word = |word: usize| format!("{:05}", if word == changed { value } else { word });
        (0..2_000).map(word).collect::<Vec<_>>().join(" ").encode_utf16().collect()
    };
    let mut old = words(5, 5);
    for frame in 1..40 {
        let text = words((frame * 37) % 2_000, frame);
        let (whole, edited) = BreakTable::of_edit(&old, &text);
        assert_eq!(whole, edited, "frame {frame}");
        assert!(BreakTable::edit_enumerates(&old, &text) <= 320, "frame {frame}");
        old = text;
    }
}
