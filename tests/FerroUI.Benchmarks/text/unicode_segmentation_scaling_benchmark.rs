//! Word and sentence segmentation over text shapes whose cost for a
//! character differs: ordinary prose, a script without spaces, emoji
//! sequences carrying joiners, and the long runs of spaces or closing
//! punctuation that the sentence rules have to look through. Each shape is
//! measured at two lengths, so the cost for a character is comparable across
//! them: an enumerator that stays linear holds that figure between the two
//! lengths, one that rescans does not.

use crate::harness::Registry;
use ferroui_base::media::text_formatting::unicode::{SentenceBreakEnumerator, WordBreakEnumerator};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TextShape {
    /// English prose with abbreviations, quotes and decimals.
    Prose,

    /// Han text with ideographic full stops and no spaces.
    Cjk,

    /// Emoji sequences joined by ZWJ, interleaved with ASCII.
    Emoji,

    /// Nothing but spaces.
    SpaceRun,

    /// A letter followed by closing punctuation.
    CloseRun,

    /// A sentence terminator followed by spaces, holding the terminator context open.
    TerminatorRun,

    /// Flags, so every codepoint is a regional indicator.
    FlagRun,
}

const PROSE_SAMPLE: &str =
    "Dr. Smith went home. He slept until 6.30 a.m. and read \"The U.S.A. Today\" for an hour! Was it worth it? Probably not. ";

const CJK_SAMPLE: &str = "这是一个测试。我们需要更多的文本来测量分段。";

// Woman technologist, then a four-person family, both built from ZWJ (Format) sequences.
const EMOJI_SAMPLE: &str =
    "\u{1F469}\u{200D}\u{1F4BB} \u{1F468}\u{200D}\u{1F469}\u{200D}\u{1F467}\u{200D}\u{1F466} \u{1F389} Ship it. ";

const FLAG_SAMPLE: &str = "\u{1F1E9}\u{1F1EA}\u{1F1EB}\u{1F1F7}\u{1F1EE}\u{1F1F9}\u{1F1EA}\u{1F1F8}";

pub struct UnicodeSegmentationScalingBenchmark {
    /// The text in UTF-16 code units, as the string of upstream is; the
    /// lengths are counts of code units.
    text: Vec<u16>,
}

impl UnicodeSegmentationScalingBenchmark {
    pub fn new(shape: TextShape, length: usize) -> Self {
        let text = match shape {
            TextShape::Prose => Self::tile(PROSE_SAMPLE, length),
            TextShape::Cjk => Self::tile(CJK_SAMPLE, length),
            TextShape::Emoji => Self::tile(EMOJI_SAMPLE, length),
            TextShape::SpaceRun => vec![u16::from(b' '); length],
            TextShape::CloseRun => {
                let mut text = vec![u16::from(b'A')];
                text.resize(length, u16::from(b')'));
                text
            }
            TextShape::TerminatorRun => {
                let mut text = vec![u16::from(b'A'), u16::from(b'.')];
                text.resize(length, u16::from(b' '));
                text
            }
            TextShape::FlagRun => Self::tile(FLAG_SAMPLE, length),
        };

        Self { text }
    }

    pub fn word_break(&self) -> i32 {
        let mut enumerator = WordBreakEnumerator::new(self.text.as_slice());
        let mut count = 0;

        while enumerator.move_next().is_some() {
            count += 1;
        }

        count
    }

    pub fn sentence_break(&self) -> i32 {
        let mut enumerator = SentenceBreakEnumerator::new(self.text.as_slice());
        let mut count = 0;

        while enumerator.move_next().is_some() {
            count += 1;
        }

        count
    }

    // Repeats the sample to the requested code-unit length, cutting back to a codepoint
    // boundary so a supplementary-plane scalar is never split into a lone surrogate.
    fn tile(sample: &str, length: usize) -> Vec<u16> {
        let sample: Vec<u16> = sample.encode_utf16().collect();
        let mut builder: Vec<u16> = Vec::with_capacity(length + sample.len());

        while builder.len() < length {
            builder.extend_from_slice(&sample);
        }

        let mut end = length;

        // A high surrogate.
        if (0xD800..=0xDBFF).contains(&builder[end - 1]) {
            end -= 1;
        }

        builder.truncate(end);
        builder
    }
}

pub fn register(registry: &mut Registry) {
    let mut class = registry.class("text", "UnicodeSegmentationScalingBenchmark").min_iteration_time(150);
    for shape in [
        TextShape::Prose,
        TextShape::Cjk,
        TextShape::Emoji,
        TextShape::SpaceRun,
        TextShape::CloseRun,
        TextShape::TerminatorRun,
        TextShape::FlagRun,
    ] {
        for length in [1024_usize, 16384] {
            let parameters = format!("Shape={shape:?}, Length={length}");
            class.benchmark(
                "word_break",
                parameters.clone(),
                move || UnicodeSegmentationScalingBenchmark::new(shape, length),
                |b| b.word_break(),
            );
            class.benchmark(
                "sentence_break",
                parameters,
                move || UnicodeSegmentationScalingBenchmark::new(shape, length),
                |b| b.sentence_break(),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn unicode_segmentation_scaling_benchmark() {
        crate::harness::smoke_class(super::register, "UnicodeSegmentationScalingBenchmark");
    }
}
