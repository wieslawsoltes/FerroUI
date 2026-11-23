//! The bidi algorithm over text shapes whose cost for a character differs:
//! ordinary prose, a paragraph of numbers and spaces (no strong character for
//! the weak rules to stop at), Arabic and Hebrew text with numbers, brackets,
//! and the isolates and embeddings that split a paragraph into level runs.
//! Each shape is measured at two lengths, so the cost for a character is
//! comparable across them: an algorithm that stays linear holds that figure
//! between the two lengths, one that searches backward from each character
//! does not.

use crate::harness::Registry;
use ferroui_base::media::text_formatting::unicode::{BidiAlgorithm, BidiData};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TextShape {
    /// English prose.
    Prose,

    /// Five-digit numbers separated by spaces: European numbers and
    /// whitespace only.
    Numbers,

    /// Arabic words with European digits, which rule W2 turns into Arabic numbers.
    ArabicNumbers,

    /// Hebrew and Latin words with numbers.
    HebrewMixed,

    /// Bracket pairs around numbers and words.
    Brackets,

    /// Words in first strong isolates.
    Isolates,

    /// Words in right to left embeddings.
    Embeddings,
}

const PROSE_SAMPLE: &str = "The quick brown fox jumps over the lazy dog, twice a day. ";

const NUMBERS_SAMPLE: &str = "12345 67890 ";

const ARABIC_NUMBERS_SAMPLE: &str = "\u{0627}\u{0644}\u{0639}\u{062F}\u{062F} 12345 \u{0648} 67,890.5 ";

const HEBREW_MIXED_SAMPLE: &str = "\u{05E9}\u{05DC}\u{05D5}\u{05DD} hello 123 \u{05E2}\u{05D5}\u{05DC}\u{05DD} world 45% ";

const BRACKETS_SAMPLE: &str = "(12) [ab] {\u{05D0}\u{05D1}} (c [3]) ";

const ISOLATES_SAMPLE: &str = "\u{2068}abc\u{2069} 12 \u{2068}\u{05D0}\u{05D1}\u{2069} ";

const EMBEDDINGS_SAMPLE: &str = "\u{202B}abc 12\u{202C} de ";

pub struct BidiScalingBenchmark {
    /// The text in UTF-16 code units; the lengths are counts of code units.
    text: Vec<u16>,
    data: BidiData,
    algorithm: BidiAlgorithm,
}

impl BidiScalingBenchmark {
    pub fn new(shape: TextShape, length: usize) -> Self {
        let sample = match shape {
            TextShape::Prose => PROSE_SAMPLE,
            TextShape::Numbers => NUMBERS_SAMPLE,
            TextShape::ArabicNumbers => ARABIC_NUMBERS_SAMPLE,
            TextShape::HebrewMixed => HEBREW_MIXED_SAMPLE,
            TextShape::Brackets => BRACKETS_SAMPLE,
            TextShape::Isolates => ISOLATES_SAMPLE,
            TextShape::Embeddings => EMBEDDINGS_SAMPLE,
        };

        Self { text: Self::tile(sample, length), data: BidiData::new(), algorithm: BidiAlgorithm::new() }
    }

    /// What the text formatter does for a paragraph: the classes of the text,
    /// then its levels.
    pub fn resolve(&mut self) -> usize {
        self.data.reset();
        self.data.set_paragraph_embedding_level(0);
        self.data.append(&self.text);
        self.algorithm.process(&mut self.data);
        self.algorithm.resolved_levels().len()
    }

    // Repeats the sample, whole, up to the requested code-unit length: every sample is in the
    // basic plane and ends where its isolates and embeddings are closed.
    fn tile(sample: &str, length: usize) -> Vec<u16> {
        let sample: Vec<u16> = sample.encode_utf16().collect();
        let mut text: Vec<u16> = Vec::with_capacity(length);

        while text.len() + sample.len() <= length {
            text.extend_from_slice(&sample);
        }

        text
    }
}

pub fn register(registry: &mut Registry) {
    let mut class = registry.class("text", "BidiScalingBenchmark").min_iteration_time(150);
    for shape in [
        TextShape::Prose,
        TextShape::Numbers,
        TextShape::ArabicNumbers,
        TextShape::HebrewMixed,
        TextShape::Brackets,
        TextShape::Isolates,
        TextShape::Embeddings,
    ] {
        for length in [1024_usize, 16384] {
            class.benchmark(
                "resolve",
                format!("Shape={shape:?}, Length={length}"),
                move || BidiScalingBenchmark::new(shape, length),
                |b| b.resolve(),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn bidi_scaling_benchmark() {
        crate::harness::smoke_class(super::register, "BidiScalingBenchmark");
    }
}
