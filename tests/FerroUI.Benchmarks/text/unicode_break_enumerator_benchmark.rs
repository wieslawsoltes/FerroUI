//! The cost of iterating with the four Unicode break enumerators from end to
//! end. Their inner loops call the property getters the tries are behind, so
//! the benchmark has both the cost of the trie lookups and the overhead of
//! the algorithm for a segment, in the shape text layout pays for a string.

use super::random::Random;
use crate::harness::Registry;
use ferroui_base::media::text_formatting::unicode::{
    GraphemeEnumerator, LineBreakEnumerator, SentenceBreakEnumerator, WordBreakEnumerator,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TextDistribution {
    /// Pure ASCII: no surrogate decoding cost.
    Ascii,

    /// BMP mix of Latin, Cyrillic, Greek, CJK.
    Bmp,

    /// Mix of BMP and supplementary plane scalars.
    Supplementary,
}

const SCALAR_COUNT: usize = 1024;

pub struct UnicodeBreakEnumeratorBenchmark {
    /// The text in UTF-16 code units, as the string of upstream is.
    text: Vec<u16>,
}

impl UnicodeBreakEnumeratorBenchmark {
    pub fn new(distribution: TextDistribution) -> Self {
        // The fixture intentionally mirrors the distributions of the
        // codepoint benchmark so the two benchmark suites are comparable: any
        // added cost above the raw codepoint sequence is the break-algorithm
        // overhead.
        let mut rng = Random::new(42);
        let mut text: Vec<u16> = Vec::with_capacity(SCALAR_COUNT * 2);

        for _ in 0..SCALAR_COUNT {
            let scalar = match distribution {
                TextDistribution::Ascii => rng.next_range(0x20, 0x7F) as u32,
                TextDistribution::Bmp => Self::sample_bmp_scalar(&mut rng),
                TextDistribution::Supplementary => Self::sample_supplementary_scalar(&mut rng),
            };

            let scalar = char::from_u32(scalar).expect("the sampled ranges hold scalar values");
            text.extend_from_slice(scalar.encode_utf16(&mut [0; 2]));
        }

        Self { text }
    }

    fn sample_bmp_scalar(rng: &mut Random) -> u32 {
        let bucket = rng.next_max(4);
        match bucket {
            0 => rng.next_range(0x0020, 0x007F) as u32, // Latin
            1 => rng.next_range(0x0400, 0x0500) as u32, // Cyrillic
            2 => rng.next_range(0x0370, 0x0400) as u32, // Greek
            _ => rng.next_range(0x4E00, 0x9FFF) as u32, // CJK Unified Ideographs
        }
    }

    fn sample_supplementary_scalar(rng: &mut Random) -> u32 {
        if (rng.next() & 1) == 0 {
            return Self::sample_bmp_scalar(rng);
        }

        let bucket = rng.next_max(3);
        match bucket {
            0 => rng.next_range(0x1F300, 0x1F600) as u32, // emoji
            1 => rng.next_range(0x20000, 0x2A6DF) as u32, // CJK Ext B
            _ => rng.next_range(0x1D400, 0x1D800) as u32, // math alphanumerics
        }
    }

    pub fn line_break_enumerator_sequence(&self) -> i32 {
        let mut enumerator = LineBreakEnumerator::new(self.text.as_slice());
        let mut count = 0;

        while enumerator.move_next().is_some() {
            count += 1;
        }

        count
    }

    pub fn word_break_enumerator_sequence(&self) -> i32 {
        let mut enumerator = WordBreakEnumerator::new(self.text.as_slice());
        let mut count = 0;

        while enumerator.move_next().is_some() {
            count += 1;
        }

        count
    }

    pub fn sentence_break_enumerator_sequence(&self) -> i32 {
        let mut enumerator = SentenceBreakEnumerator::new(self.text.as_slice());
        let mut count = 0;

        while enumerator.move_next().is_some() {
            count += 1;
        }

        count
    }

    pub fn grapheme_enumerator_sequence(&self) -> i32 {
        let mut enumerator = GraphemeEnumerator::new(self.text.as_slice());
        let mut count = 0;

        while enumerator.move_next().is_some() {
            count += 1;
        }

        count
    }
}

pub fn register(registry: &mut Registry) {
    let mut class = registry.class("text", "UnicodeBreakEnumeratorBenchmark").min_iteration_time(150);
    for distribution in [TextDistribution::Ascii, TextDistribution::Bmp, TextDistribution::Supplementary] {
        let parameters = format!("Distribution={distribution:?}");
        class.benchmark(
            "line_break_enumerator_sequence",
            parameters.clone(),
            move || UnicodeBreakEnumeratorBenchmark::new(distribution),
            |b| b.line_break_enumerator_sequence(),
        );
        class.benchmark(
            "word_break_enumerator_sequence",
            parameters.clone(),
            move || UnicodeBreakEnumeratorBenchmark::new(distribution),
            |b| b.word_break_enumerator_sequence(),
        );
        class.benchmark(
            "sentence_break_enumerator_sequence",
            parameters.clone(),
            move || UnicodeBreakEnumeratorBenchmark::new(distribution),
            |b| b.sentence_break_enumerator_sequence(),
        );
        class.benchmark(
            "grapheme_enumerator_sequence",
            parameters,
            move || UnicodeBreakEnumeratorBenchmark::new(distribution),
            |b| b.grapheme_enumerator_sequence(),
        );
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn unicode_break_enumerator_benchmark() {
        crate::harness::smoke_class(super::register, "UnicodeBreakEnumeratorBenchmark");
    }
}
