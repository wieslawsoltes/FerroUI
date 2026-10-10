//! The hot path of a codepoint from end to end: decoding surrogates with
//! `Codepoint::read_at`, the properties that look Unicode data up in the
//! generated tries, and a pass over every property, which is about what a
//! traversal of the layout pipeline pays for a codepoint.

use super::random::Random;
use crate::harness::Registry;
use ferroui_base::media::text_formatting::unicode::{Codepoint, CodepointEnumerator};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TextDistribution {
    /// Pure ASCII (no surrogate decoding cost).
    Ascii,

    /// BMP mix of Latin, Cyrillic, Greek, CJK (one UTF-16 unit per scalar).
    Bmp,

    /// Mix of supplementary plane scalars (math, emoji, CJK Ext B): every
    /// other scalar requires surrogate-pair decoding.
    Supplementary,
}

const SCALAR_COUNT: usize = 1024;

pub struct CodepointBenchmark {
    /// The text in UTF-16 code units, as the string of upstream is.
    text: Vec<u16>,
}

impl CodepointBenchmark {
    pub fn new(distribution: TextDistribution) -> Self {
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
        // Pick from a spread of common BMP ranges; skip the surrogate region.
        let bucket = rng.next_max(4);
        match bucket {
            0 => rng.next_range(0x0020, 0x007F) as u32, // Latin
            1 => rng.next_range(0x0400, 0x0500) as u32, // Cyrillic
            2 => rng.next_range(0x0370, 0x0400) as u32, // Greek
            _ => rng.next_range(0x4E00, 0x9FFF) as u32, // CJK Unified Ideographs
        }
    }

    fn sample_supplementary_scalar(rng: &mut Random) -> u32 {
        // Alternate BMP and supplementary so the benchmark exercises both
        // branches of `Codepoint::read_at`: pure-supplementary text isn't
        // representative of any real layout workload.
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

    pub fn read_at_sequence(&self) -> u32 {
        let span = self.text.as_slice();
        let mut sum = 0_u32;

        let mut i = 0;
        while i < span.len() {
            let (codepoint, count) = Codepoint::read_at(span, i);
            sum = sum.wrapping_add(codepoint.value());
            i += count;
        }

        sum
    }

    pub fn codepoint_enumerator_sequence(&self) -> u32 {
        let mut enumerator = CodepointEnumerator::new(self.text.as_slice());
        let mut sum = 0_u32;

        while let Some(cp) = enumerator.move_next() {
            sum = sum.wrapping_add(cp.value());
        }

        sum
    }

    pub fn sequence_general_category(&self) -> i32 {
        let span = self.text.as_slice();
        let mut sum = 0_i32;

        let mut i = 0;
        while i < span.len() {
            let (cp, count) = Codepoint::read_at(span, i);
            sum = sum.wrapping_add(cp.general_category() as i32);
            i += count;
        }

        sum
    }

    pub fn sequence_script(&self) -> i32 {
        let span = self.text.as_slice();
        let mut sum = 0_i32;

        let mut i = 0;
        while i < span.len() {
            let (cp, count) = Codepoint::read_at(span, i);
            sum = sum.wrapping_add(cp.script() as i32);
            i += count;
        }

        sum
    }

    pub fn sequence_bi_di_class(&self) -> i32 {
        let span = self.text.as_slice();
        let mut sum = 0_i32;

        let mut i = 0;
        while i < span.len() {
            let (cp, count) = Codepoint::read_at(span, i);
            sum = sum.wrapping_add(cp.bi_di_class() as i32);
            i += count;
        }

        sum
    }

    /// Worst-case representative of a layout pipeline pass that needs every
    /// property per codepoint. Seven trie lookups per scalar (Category +
    /// Script + BiDi + LineBreak + WordBreak + GraphemeBreak + EastAsianWidth).
    /// This is the headline number for the "should we merge tries?" question.
    pub fn sequence_all_properties(&self) -> i32 {
        let span = self.text.as_slice();
        let mut sum = 0_i32;

        let mut i = 0;
        while i < span.len() {
            let (cp, count) = Codepoint::read_at(span, i);
            sum = sum.wrapping_add(cp.general_category() as i32);
            sum = sum.wrapping_add(cp.script() as i32);
            sum = sum.wrapping_add(cp.bi_di_class() as i32);
            sum = sum.wrapping_add(cp.line_break_class() as i32);
            sum = sum.wrapping_add(cp.word_break_class() as i32);
            sum = sum.wrapping_add(cp.grapheme_break_class() as i32);
            sum = sum.wrapping_add(cp.east_asian_width_class() as i32);
            i += count;
        }

        sum
    }

    pub fn try_get_paired_bracket_sequence(&self) -> i32 {
        let span = self.text.as_slice();
        let mut paired = 0_i32;

        let mut i = 0;
        while i < span.len() {
            let (cp, count) = Codepoint::read_at(span, i);
            if cp.try_get_paired_bracket().is_some() {
                paired += 1;
            }
            i += count;
        }

        paired
    }
}

pub fn register(registry: &mut Registry) {
    let mut class = registry.class("text", "CodepointBenchmark").min_iteration_time(150);
    for distribution in [TextDistribution::Ascii, TextDistribution::Bmp, TextDistribution::Supplementary] {
        let parameters = format!("Distribution={distribution:?}");
        class.benchmark(
            "read_at_sequence",
            parameters.clone(),
            move || CodepointBenchmark::new(distribution),
            |b| b.read_at_sequence(),
        );
        class.benchmark(
            "codepoint_enumerator_sequence",
            parameters.clone(),
            move || CodepointBenchmark::new(distribution),
            |b| b.codepoint_enumerator_sequence(),
        );
        class.benchmark(
            "sequence_general_category",
            parameters.clone(),
            move || CodepointBenchmark::new(distribution),
            |b| b.sequence_general_category(),
        );
        class.benchmark(
            "sequence_script",
            parameters.clone(),
            move || CodepointBenchmark::new(distribution),
            |b| b.sequence_script(),
        );
        class.benchmark(
            "sequence_bi_di_class",
            parameters.clone(),
            move || CodepointBenchmark::new(distribution),
            |b| b.sequence_bi_di_class(),
        );
        class.benchmark(
            "sequence_all_properties",
            parameters.clone(),
            move || CodepointBenchmark::new(distribution),
            |b| b.sequence_all_properties(),
        );
        class.benchmark(
            "try_get_paired_bracket_sequence",
            parameters,
            move || CodepointBenchmark::new(distribution),
            |b| b.try_get_paired_bracket_sequence(),
        );
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn codepoint_benchmark() {
        crate::harness::smoke_class(super::register, "CodepointBenchmark");
    }
}
