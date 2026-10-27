//! The deterministic source test measurer. It intentionally has contextual
//! decimal advances, so tests cannot accidentally assume additive characters.

use std::rc::Rc;

use crate::pretext_layout_measurement::text_elements;
use unicode_general_category::{get_general_category, GeneralCategory};

use crate::*;

pub const FONT: &str = "16px Test Sans";
pub const LINE_HEIGHT: f64 = 19.0;
pub const PRE_WRAP: PrepareOptions = PrepareOptions {
    white_space: WhiteSpaceMode::PreWrap,
    word_break: WordBreakMode::Normal,
};

pub fn engine() -> PretextLayout {
    PretextLayout::new(Rc::new(DelegateTextMeasurerFactory::new(measure_width)))
}

pub fn measure_width(text: &str, font: &str) -> f64 {
    let size = PretextFontParser::parse(font).size;
    let mut previous_was_decimal_digit = false;
    text.chars().map(|ch| {
        let decimal = get_general_category(ch) == GeneralCategory::DecimalNumber;
        let advance = if ch == ' ' { size * 0.33 }
        else if ch == '\t' { size * 1.32 }
        else if matches!(ch as u32, 0x1f300..=0x1faff | 0x2600..=0x26ff | 0x2700..=0x27bf | 0xfe0f) { size }
        else if decimal { size * if previous_was_decimal_digit { 0.48 } else { 0.52 } }
        else if is_wide_character(ch) { size }
        else if ".,!?;:%)]}'\"”’»›…—-".contains(ch) { size * 0.4 }
        else { size * 0.6 };
        previous_was_decimal_digit = decimal;
        advance
    }).sum()
}

pub fn is_wide_character(ch: char) -> bool {
    matches!(ch as u32,
        0x4e00..=0x9fff | 0x3400..=0x4dbf | 0xf900..=0xfaff | 0x2f800..=0x2fa1f |
        0x20000..=0x2a6df | 0x2a700..=0x2b73f | 0x2b740..=0x2b81f | 0x2b820..=0x2ceaf |
        0x2ceb0..=0x2ebef | 0x2ebf0..=0x2ee5d | 0x30000..=0x3134f | 0x31350..=0x323af |
        0x323b0..=0x33479 | 0x3000..=0x303f | 0x3040..=0x309f | 0x30a0..=0x30ff |
        0xac00..=0xd7af | 0xff00..=0xffef)
}

pub fn slice_prepared_text(
    prepared: &PreparedTextWithSegments,
    start: LayoutCursor,
    end: LayoutCursor,
) -> String {
    let mut result = String::new();
    for index in start.segment_index..=end.segment_index.min(prepared.segments().len()) {
        let Some(segment) = prepared.segments().get(index) else {
            break;
        };
        let from = if index == start.segment_index {
            start.grapheme_index
        } else {
            0
        };
        let to = if index == end.segment_index {
            end.grapheme_index
        } else {
            usize::MAX
        };
        for grapheme in text_elements(segment)
            .skip(from)
            .take(to.saturating_sub(from))
        {
            result.push_str(grapheme);
        }
    }
    result
}
