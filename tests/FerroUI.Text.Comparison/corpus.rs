//! The strings every library lays out, generated in code: deterministic, the same on every run and
//! for every library, and no text of anyone's. The generator is the 32-bit xorshift of the
//! gauntlet benchmark (`samples/GauntletBenchmark/perf_data.rs`), seeded per string as the
//! gauntlet seeds a post, so that the strings of a quick run are the first strings of a full one.
//!
//! - `label`: 5,000 short labels of 3 to 20 characters: cell coordinates (`C12.L3.4`), prices
//!   (`$1,234.56`), signed percentages (`+2.31%`) and one or two capitalized pseudo-words.
//! - `card`: 500 card bodies of 120 to 260 characters of pseudo-words, the length of the post
//!   bodies of the gauntlet: words of 2 to 11 letters, separated by spaces, with commas and
//!   periods, each body ending with a period.
//! - `article`: 20 paragraphs of 2,000 to 4,000 characters of the same prose.
//!
//! The words are at most 11 letters and the labels at most 20 characters: at 14 px no word of
//! the corpus is wider than the narrowest width of the scenarios (180 px), so no library has to
//! break inside a word, and the wrapping modes of the libraries (break inside a word only when it
//! does not fit, or overflow) lay out the same lines.

use crate::Scale;

/// The width of the cold and same-width scenarios, in pixels.
pub const LAYOUT_WIDTH: f32 = 240.0;

/// The widths of the rewrap sweep: 180 to 320 pixels in steps of 7, cycled.
pub const SWEEP_FIRST_WIDTH: f32 = 180.0;
pub const SWEEP_STEP: f32 = 7.0;
pub const SWEEP_STEPS: usize = 21;

/// The width of step `step` of the sweep (180, 187, ..., 320, 180, ...).
pub fn sweep_width(step: usize) -> f32 {
    SWEEP_FIRST_WIDTH + SWEEP_STEP * (step % SWEEP_STEPS) as f32
}

/// A corpus of strings.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Corpus {
    Label,
    Card,
    Article,
}

impl Corpus {
    pub const ALL: [Corpus; 3] = [Corpus::Label, Corpus::Card, Corpus::Article];

    /// The name of the corpus in the parameters of a benchmark.
    pub fn name(self) -> &'static str {
        match self {
            Corpus::Label => "label",
            Corpus::Card => "card",
            Corpus::Article => "article",
        }
    }

    /// The number of strings of the corpus: the full counts, or a few for the quick tests.
    pub fn count(self, scale: Scale) -> usize {
        match (self, scale) {
            (Corpus::Label, Scale::Full) => 5_000,
            (Corpus::Card, Scale::Full) => 500,
            (Corpus::Article, Scale::Full) => 20,
            (Corpus::Label, Scale::Quick) => 40,
            (Corpus::Card, Scale::Quick) => 8,
            (Corpus::Article, Scale::Quick) => 2,
        }
    }

    /// The strings of the corpus.
    pub fn texts(self, scale: Scale) -> Vec<String> {
        (0..self.count(scale) as i32)
            .map(|index| match self {
                Corpus::Label => label(index),
                Corpus::Card => card(index),
                Corpus::Article => article(index),
            })
            .collect()
    }
}

/// The seeds of the strings of each corpus start apart, so that no two corpora share a stream.
const LABEL_SEED: i32 = 1_000_000;
const CARD_SEED: i32 = 2_000_000;
const ARTICLE_SEED: i32 = 3_000_000;

/// 32-bit xorshift, as in `samples/GauntletBenchmark/perf_data.rs`.
pub struct Rng {
    state: u32,
}

impl Rng {
    pub fn new(seed: i32) -> Self {
        let mut state = (seed as u32).wrapping_mul(0x9E37_79B9) ^ 0xA5A5_A5A5;
        if state == 0 {
            state = 1;
        }
        Self { state }
    }

    pub fn next(&mut self) -> u32 {
        let mut x = self.state;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.state = x;
        x
    }

    /// A value in `0..bound`.
    pub fn below(&mut self, bound: i32) -> i32 {
        (self.next() % bound as u32) as i32
    }
}

const CONSONANTS: &[u8] = b"bcdfghjklmnprstvwz";
const VOWELS: &[u8] = b"aeiou";

/// A pronounceable lower-case pseudo-word of 2 to 11 letters: consonants and vowels alternating.
fn word(rng: &mut Rng) -> String {
    let length = 2 + rng.below(10) as usize;
    let mut vowel = rng.below(3) == 0;
    let mut text = String::with_capacity(length);
    for _ in 0..length {
        let letters = if vowel { VOWELS } else { CONSONANTS };
        text.push(char::from(letters[rng.below(letters.len() as i32) as usize]));
        vowel = !vowel;
    }
    text
}

fn capitalize(text: &mut str) {
    // The words are lower-case ASCII.
    if let Some(first) = text.get_mut(0..1) {
        first.make_ascii_uppercase();
    }
}

/// Prose of at least `target` characters and at most `target + 12`: words separated by spaces,
/// a comma after about one word in fourteen and a period after about one in sixteen (the next
/// word capitalized), ending with a period.
fn prose(rng: &mut Rng, target: usize) -> String {
    let mut text = String::with_capacity(target + 16);
    let mut capitalize_next = true;
    while text.len() < target {
        if !text.is_empty() {
            text.push(' ');
        }
        let mut next = word(rng);
        if capitalize_next {
            capitalize(&mut next);
            capitalize_next = false;
        }
        text.push_str(&next);
        let roll = rng.below(100);
        if roll < 7 {
            text.push(',');
        } else if roll < 13 {
            text.push('.');
            capitalize_next = true;
        }
    }
    if text.ends_with(',') {
        text.pop();
    }
    if !text.ends_with('.') {
        text.push('.');
    }
    text
}

/// `1234567` as `1,234,567`.
fn grouped(value: i32) -> String {
    let digits = value.to_string();
    let mut text = String::with_capacity(digits.len() + digits.len() / 3);
    for (index, digit) in digits.chars().enumerate() {
        if index > 0 && (digits.len() - index) % 3 == 0 {
            text.push(',');
        }
        text.push(digit);
    }
    text
}

/// Label `index`: 3 to 20 characters.
pub fn label(index: i32) -> String {
    let mut rng = Rng::new(LABEL_SEED + index);
    match index % 4 {
        // A cell coordinate: C12.L3.4.
        0 => format!("C{}.L{}.{}", rng.below(100), rng.below(10), rng.below(10)),
        // A price: $1,234.56.
        1 => {
            let dollars = rng.below(1_000_000);
            let cents = rng.below(100);
            format!("${}.{cents:02}", grouped(dollars))
        }
        // A signed percentage: +2.31%.
        2 => {
            let sign = if rng.below(2) == 0 { '+' } else { '-' };
            let hundredths = rng.below(10_000);
            format!("{sign}{}.{:02}%", hundredths / 100, hundredths % 100)
        }
        // One or two capitalized pseudo-words.
        _ => {
            let mut text = word(&mut rng);
            capitalize(&mut text);
            if rng.below(2) == 0 {
                let mut second = word(&mut rng);
                capitalize(&mut second);
                if text.len() + 1 + second.len() <= 20 {
                    text.push(' ');
                    text.push_str(&second);
                }
            }
            if text.len() < 3 {
                text.push(char::from(b'0' + rng.below(10) as u8));
            }
            text
        }
    }
}

/// Card body `index`: 120 to 260 characters.
pub fn card(index: i32) -> String {
    let mut rng = Rng::new(CARD_SEED + index);
    let target = 120 + rng.below(129) as usize;
    prose(&mut rng, target)
}

/// Article paragraph `index`: 2,000 to 4,000 characters.
pub fn article(index: i32) -> String {
    let mut rng = Rng::new(ARTICLE_SEED + index);
    let target = 2_000 + rng.below(1_989) as usize;
    prose(&mut rng, target)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_corpora_have_the_stated_counts_and_lengths() {
        let labels = Corpus::Label.texts(Scale::Full);
        assert_eq!(labels.len(), 5_000);
        assert!(labels.iter().all(|label| (3..=20).contains(&label.len())), "a label is 3 to 20 characters");

        let cards = Corpus::Card.texts(Scale::Full);
        assert_eq!(cards.len(), 500);
        assert!(cards.iter().all(|card| (120..=260).contains(&card.len())), "a card is 120 to 260 characters");

        let articles = Corpus::Article.texts(Scale::Full);
        assert_eq!(articles.len(), 20);
        assert!(
            articles.iter().all(|article| (2_000..=4_000).contains(&article.len())),
            "a paragraph is 2,000 to 4,000 characters"
        );
    }

    #[test]
    fn the_words_are_two_to_eleven_letters() {
        for card in Corpus::Card.texts(Scale::Full) {
            for word in card.split(' ') {
                let letters = word.trim_end_matches([',', '.']).len();
                assert!((2..=11).contains(&letters), "{word:?} in {card:?}");
            }
        }
    }

    #[test]
    fn the_corpora_are_deterministic_and_quick_runs_take_their_first_strings() {
        assert_eq!(Corpus::Card.texts(Scale::Full), Corpus::Card.texts(Scale::Full));
        let full = Corpus::Label.texts(Scale::Full);
        let quick = Corpus::Label.texts(Scale::Quick);
        assert_eq!(&full[..quick.len()], &quick[..]);
        assert_eq!(label(1).chars().next(), Some('$'));
        assert!(label(2).ends_with('%'));
        assert!(label(0).starts_with('C'));
    }

    #[test]
    fn prices_group_their_thousands() {
        assert_eq!(grouped(0), "0");
        assert_eq!(grouped(999), "999");
        assert_eq!(grouped(1_234), "1,234");
        assert_eq!(grouped(999_999), "999,999");
    }

    #[test]
    fn the_sweep_runs_from_180_to_320_in_steps_of_7() {
        assert_eq!(sweep_width(0), 180.0);
        assert_eq!(sweep_width(SWEEP_STEPS - 1), 320.0);
        assert_eq!(sweep_width(SWEEP_STEPS), 180.0);
    }
}
