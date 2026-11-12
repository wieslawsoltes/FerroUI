//! An edited text shaped from the shaping of the text before (`TextShaper::shape_edited_text`,
//! step 7 of docs/porting/text-layout-performance.md) has the glyphs a shaping of the whole text
//! gives: random edits of random paragraphs, in a static and a variable font, each compared with
//! a shaping from nothing, and each shaped from the edit before so that the flags an edit records
//! are checked by the next one.

use crate::HarfBuzzTextShaper;
use ferroui_base::media::fonts::testing::TestPlatformTypeface;
use ferroui_base::media::text_formatting::{GlyphInfo, ShapedBuffer, TextShaper, TextShaperOptions};
use ferroui_base::media::{FontSimulations, GlyphTypeface};
use ferroui_base::utilities::ReadOnlyMemory;
use ferroui_controls::testing::{TestServices, UnitTestApplication, UnitTestApplicationScope};
use std::rc::Rc;

const INTER: &[u8] = include_bytes!("../../FerroUI.Base/test_assets/fonts/Inter-Regular.ttf");
const INTER_VARIABLE: &[u8] = include_bytes!("../../FerroUI.Base/test_assets/fonts/InterVariable.ttf");
/// A font whose lookups leave the glyphs of a Latin text clear of the flags, so that edits keep
/// glyphs; Inter's contextual alternates look at the text around nearly every glyph, so HarfBuzz
/// marks nearly every glyph of it and its edits are shaped whole.
const SOURCE_SANS: &[u8] = include_bytes!("../../../samples/ControlCatalog/Assets/Fonts/SourceSansPro-Regular.ttf");

/// Words with kerning pairs, contextual forms, digits, punctuation and a combining mark.
const WORDS: &[&str] = &[
    "office", "efficient", "AVATAR", "Today", "Wave", "flow", "first", "1234", "00042", "->", "<=", "e\u{301}t\u{e9}",
    "Ta", "Yo", "LT", "fjord", "(quoted)", "x", "tt", "\u{2014}", "1/2", "A.V.", "WAVE", "Kerning", "office->",
];

fn start() -> UnitTestApplicationScope {
    let shaper = Rc::new(HarfBuzzTextShaper::new());
    UnitTestApplication::start(TestServices::mock_threading_interface().with_text_shaper_impl(shaper))
}

fn options(font: &[u8], size: f64) -> TextShaperOptions {
    let platform_typeface =
        TestPlatformTypeface::from_bytes(font.to_vec(), FontSimulations::None).expect("the font loads");
    let typeface = GlyphTypeface::new(platform_typeface, FontSimulations::None).expect("the font has its tables");
    TextShaperOptions::with_all(typeface, size, 0, None, 0.0, 0.0, None)
}

fn utf16(text: &str) -> ReadOnlyMemory<u16> {
    ReadOnlyMemory::from_vec(text.encode_utf16().collect())
}

/// A small deterministic generator (the tests need no randomness from the system).
struct Random(u64);

impl Random {
    fn next(&mut self, below: usize) -> usize {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        (self.0 % below.max(1) as u64) as usize
    }
}

fn paragraph(random: &mut Random, words: usize) -> Vec<String> {
    (0..words).map(|_| WORDS[random.next(WORDS.len())].to_string()).collect()
}

/// One random edit of the words: a word replaced, inserted or removed, a word changed inside, or
/// a word added at the end (a message that grows).
fn edit(random: &mut Random, words: &mut Vec<String>) {
    let at = random.next(words.len());
    match random.next(5) {
        0 => words[at] = WORDS[random.next(WORDS.len())].to_string(),
        1 => words.insert(at, WORDS[random.next(WORDS.len())].to_string()),
        2 if words.len() > 60 => {
            words.remove(at);
        }
        3 => {
            let word = &mut words[at];
            let cut = word.char_indices().map(|(index, _)| index).nth(random.next(word.chars().count())).unwrap_or(0);
            word.insert_str(cut, WORDS[random.next(WORDS.len())]);
        }
        _ => words.push(WORDS[random.next(WORDS.len())].to_string()),
    }
}

fn glyphs(buffer: &ShapedBuffer) -> Vec<GlyphInfo> {
    buffer.glyph_infos().to_vec()
}

#[test]
fn an_edited_text_is_shaped_as_the_whole_text_is() {
    let _app = start();
    let shaper = TextShaper::current();
    let mut random = Random(0x9E37_79B9_7F4A_7C15);
    let (mut kept, mut edits) = (0, 0);
    for (font, size) in [(INTER, 16.0), (INTER_VARIABLE, 13.5), (SOURCE_SANS, 15.0)] {
        let options = options(font, size);
        let counts = font.as_ptr() == SOURCE_SANS.as_ptr();
        for case in 0..40 {
            let count = 60 + random.next(80);
            let mut words = paragraph(&mut random, count);
            let mut previous = shaper.shape_text(&utf16(&words.join(" ")), &options.clone().with_concat_flags(true));
            for step in 0..8 {
                edit(&mut random, &mut words);
                let text = utf16(&words.join(" "));
                let whole = shaper.shape_text(&text, &options);
                edits += usize::from(counts);
                previous = match shaper.shape_edited_text(&previous, &text, &options) {
                    Some(edited) => {
                        kept += usize::from(counts);
                        assert_eq!(glyphs(&edited), glyphs(&whole), "case {case}, step {step}: {}", words.join(" "));
                        edited
                    }
                    None => shaper.shape_text(&text, &options.clone().with_concat_flags(true)),
                };
            }
        }
    }
    // In a font that leaves the glyphs clear most edits keep glyphs: an edit at the very start of
    // a text is shaped whole.
    assert!(kept * 10 >= edits * 8, "{kept} of {edits} edits kept glyphs");
}

#[test]
fn a_word_replaced_in_a_long_paragraph_keeps_the_rest() {
    // The paragraph patch of the benchmark's patchwork: five-digit words, one replaced.
    let _app = start();
    let shaper = TextShaper::current();
    let options = options(SOURCE_SANS, 10.0);
    let words = |changed: usize, value: usize| -> String {
        let word = |word: usize| format!("{:05}", if word == changed { value } else { word });
        (0..2_000).map(word).collect::<Vec<_>>().join(" ")
    };
    let mut previous = shaper.shape_text(&utf16(&words(5, 5)), &options.clone().with_concat_flags(true));
    for frame in 1..40 {
        let text = utf16(&words((frame * 37) % 2_000, frame));
        let edited = shaper.shape_edited_text(&previous, &text, &options).expect("the edit keeps glyphs");
        assert_eq!(glyphs(&edited), glyphs(&shaper.shape_text(&text, &options)), "frame {frame}");
        previous = edited;
    }
}

#[test]
fn nothing_is_kept_for_an_edit_at_the_start_or_without_flags() {
    let _app = start();
    let shaper = TextShaper::current();
    let options = options(INTER, 16.0);
    let old = "Today the office was efficient. ".repeat(12);
    let without_flags = shaper.shape_text(&utf16(&old), &options);
    assert!(shaper.shape_edited_text(&without_flags, &utf16(&format!("{old}!")), &options).is_none());
    let with_flags = shaper.shape_text(&utf16(&old), &options.clone().with_concat_flags(true));
    assert!(shaper.shape_edited_text(&with_flags, &utf16(&format!("A{old}")), &options).is_none());
}

