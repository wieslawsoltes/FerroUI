use super::codepoint::Codepoint;
use super::grapheme_break_class::GraphemeBreakClass;
use super::word_break_class::WordBreakClass;
use super::word_segment::WordSegment;

/// Enumerates Unicode word-boundary segments.
#[derive(Clone, Debug)]
pub struct WordBreakEnumerator<'a> {
    text: &'a [u16],
    offset: usize,
    codepoint_offset: usize,

    // Whether an odd number of regional indicators precedes the position the walk has
    // reached. WB15 and WB16 need only that parity, so it is carried along with the walk
    // instead of being recounted from the start of the run at every indicator.
    odd_regional_indicator_run: bool,
}

#[derive(Clone, Copy)]
struct WordBreakUnit {
    codepoint: Codepoint,
    word_break_class: WordBreakClass,
    start: usize,
    end: usize,
}

impl WordBreakUnit {
    #[inline]
    fn new(codepoint: Codepoint, start: usize, end: usize) -> Self {
        Self { codepoint, word_break_class: codepoint.word_break_class(), start, end }
    }
}

impl<'a> WordBreakEnumerator<'a> {
    #[inline]
    pub const fn new(text: &'a [u16]) -> Self {
        Self { text, offset: 0, codepoint_offset: 0, odd_regional_indicator_run: false }
    }

    /// Moves to the next [`WordSegment`].
    ///
    /// Returns the current word-boundary segment, or `None` at the end of the text.
    pub fn move_next(&mut self) -> Option<WordSegment<'a>> {
        if self.offset >= self.text.len() {
            return None;
        }

        let segment_start = self.offset;
        let mut current = self.read_forward(self.offset);
        self.consume(current.word_break_class);
        let mut current_end = current.end;

        // Each WordBreakUnit covers exactly one code point, so counting accepted
        // units yields the segment's code-point length for the WordSegment readouts.
        let mut codepoint_length = 1;

        while current_end < self.text.len() {
            let next = self.read_forward(current_end);

            if self.is_boundary(&current, &next) {
                break;
            }

            current = next;
            self.consume(current.word_break_class);
            current_end = current.end;
            codepoint_length += 1;
        }

        let segment = WordSegment::with_text(
            segment_start,
            &self.text[segment_start..current_end],
            self.codepoint_offset,
            codepoint_length,
        );
        self.offset = current_end;
        self.codepoint_offset += codepoint_length;
        Some(segment)
    }

    fn is_boundary(&self, current: &WordBreakUnit, next: &WordBreakUnit) -> bool {
        // WB3, WB3a, WB3b, and WB3c are evaluated before WB4, so these
        // rules use the adjacent code points exactly as they appear in text.
        if current.word_break_class == WordBreakClass::CarriageReturn && next.word_break_class == WordBreakClass::LineFeed {
            return false;
        }

        if is_newline(current) || is_newline(next) {
            return true;
        }

        if current.word_break_class == WordBreakClass::ZWJ
            && next.codepoint.grapheme_break_class() == GraphemeBreakClass::ExtendedPictographic
        {
            return false;
        }

        if current.word_break_class == WordBreakClass::WSegSpace && next.word_break_class == WordBreakClass::WSegSpace {
            return false;
        }

        if is_ignored(next.word_break_class) {
            return false;
        }

        let left = self.get_effective_previous(current);
        let right = next.word_break_class;

        if is_ah_letter(left.word_break_class) && is_ah_letter(right) {
            return false;
        }

        if is_ah_letter(left.word_break_class)
            && is_mid_letter_mid_num_let_q(right)
            && self.try_get_next_significant(next.end).is_some_and(|after| is_ah_letter(after.word_break_class))
        {
            return false;
        }

        if is_ah_letter(right)
            && is_mid_letter_mid_num_let_q(left.word_break_class)
            && self.try_get_previous_significant(left.start).is_some_and(|before| is_ah_letter(before.word_break_class))
        {
            return false;
        }

        if left.word_break_class == WordBreakClass::HebrewLetter && right == WordBreakClass::SingleQuote {
            return false;
        }

        if left.word_break_class == WordBreakClass::HebrewLetter
            && right == WordBreakClass::DoubleQuote
            && self
                .try_get_next_significant(next.end)
                .is_some_and(|after| after.word_break_class == WordBreakClass::HebrewLetter)
        {
            return false;
        }

        if right == WordBreakClass::HebrewLetter
            && left.word_break_class == WordBreakClass::DoubleQuote
            && self
                .try_get_previous_significant(left.start)
                .is_some_and(|before| before.word_break_class == WordBreakClass::HebrewLetter)
        {
            return false;
        }

        if left.word_break_class == WordBreakClass::Numeric && right == WordBreakClass::Numeric {
            return false;
        }

        if is_ah_letter(left.word_break_class) && right == WordBreakClass::Numeric {
            return false;
        }

        if left.word_break_class == WordBreakClass::Numeric && is_ah_letter(right) {
            return false;
        }

        if right == WordBreakClass::Numeric
            && is_mid_num_mid_num_let_q(left.word_break_class)
            && self
                .try_get_previous_significant(left.start)
                .is_some_and(|before| before.word_break_class == WordBreakClass::Numeric)
        {
            return false;
        }

        if left.word_break_class == WordBreakClass::Numeric
            && is_mid_num_mid_num_let_q(right)
            && self
                .try_get_next_significant(next.end)
                .is_some_and(|after| after.word_break_class == WordBreakClass::Numeric)
        {
            return false;
        }

        if left.word_break_class == WordBreakClass::Katakana && right == WordBreakClass::Katakana {
            return false;
        }

        if is_ah_letter_numeric_katakana_extend_num_let(left.word_break_class) && right == WordBreakClass::ExtendNumLet {
            return false;
        }

        if left.word_break_class == WordBreakClass::ExtendNumLet && is_ah_letter_numeric_katakana(right) {
            return false;
        }

        if left.word_break_class == WordBreakClass::RegionalIndicator
            && right == WordBreakClass::RegionalIndicator
            && self.odd_regional_indicator_run
        {
            return false;
        }

        true
    }

    // Extends the left-hand context by one code point. WB4 treats Extend, Format and ZWJ as
    // transparent, so they leave the regional indicator run they sit inside intact.
    fn consume(&mut self, word_break_class: WordBreakClass) {
        if is_ignored(word_break_class) {
            return;
        }

        self.odd_regional_indicator_run =
            word_break_class == WordBreakClass::RegionalIndicator && !self.odd_regional_indicator_run;
    }

    fn get_effective_previous(&self, current: &WordBreakUnit) -> WordBreakUnit {
        if !is_ignored(current.word_break_class) {
            return *current;
        }

        let mut scan_end = current.start;

        while let Some(previous) = self.try_read_backward(scan_end) {
            if !is_ignored(previous.word_break_class) {
                // WB4 does not ignore format or extend code points across
                // start-of-text or hard line breaks.
                return if is_newline(&previous) { *current } else { previous };
            }

            scan_end = previous.start;
        }

        *current
    }

    fn try_get_previous_significant(&self, end: usize) -> Option<WordBreakUnit> {
        let mut scan_end = end;

        while let Some(codepoint) = self.try_read_backward(scan_end) {
            if !is_ignored(codepoint.word_break_class) {
                return Some(codepoint);
            }

            scan_end = codepoint.start;
        }

        None
    }

    fn try_get_next_significant(&self, start: usize) -> Option<WordBreakUnit> {
        let mut scan_start = start;

        while let Some(codepoint) = self.try_read_forward(scan_start) {
            if !is_ignored(codepoint.word_break_class) {
                return Some(codepoint);
            }

            scan_start = codepoint.end;
        }

        None
    }

    #[inline]
    fn read_forward(&self, start: usize) -> WordBreakUnit {
        let (codepoint, count) = Codepoint::read_at(self.text, start);
        WordBreakUnit::new(codepoint, start, start + count)
    }

    fn try_read_forward(&self, start: usize) -> Option<WordBreakUnit> {
        if start >= self.text.len() {
            return None;
        }

        Some(self.read_forward(start))
    }

    fn try_read_backward(&self, end: usize) -> Option<WordBreakUnit> {
        if end == 0 {
            return None;
        }

        let mut start = end - 1;

        if start > 0 && is_low_surrogate(self.text[start]) && is_high_surrogate(self.text[start - 1]) {
            start -= 1;
        }

        Some(self.read_forward(start))
    }
}

impl<'a> Iterator for WordBreakEnumerator<'a> {
    type Item = WordSegment<'a>;

    #[inline]
    fn next(&mut self) -> Option<WordSegment<'a>> {
        self.move_next()
    }
}

#[inline]
fn is_low_surrogate(unit: u16) -> bool {
    (0xDC00..=0xDFFF).contains(&unit)
}

#[inline]
fn is_high_surrogate(unit: u16) -> bool {
    (0xD800..=0xDBFF).contains(&unit)
}

#[inline]
fn is_ah_letter(word_break_class: WordBreakClass) -> bool {
    matches!(word_break_class, WordBreakClass::ALetter | WordBreakClass::HebrewLetter)
}

#[inline]
fn is_ah_letter_numeric_katakana(word_break_class: WordBreakClass) -> bool {
    is_ah_letter(word_break_class) || matches!(word_break_class, WordBreakClass::Numeric | WordBreakClass::Katakana)
}

#[inline]
fn is_ah_letter_numeric_katakana_extend_num_let(word_break_class: WordBreakClass) -> bool {
    is_ah_letter_numeric_katakana(word_break_class) || word_break_class == WordBreakClass::ExtendNumLet
}

#[inline]
fn is_ignored(word_break_class: WordBreakClass) -> bool {
    matches!(word_break_class, WordBreakClass::Extend | WordBreakClass::Format | WordBreakClass::ZWJ)
}

#[inline]
fn is_mid_letter_mid_num_let_q(word_break_class: WordBreakClass) -> bool {
    matches!(
        word_break_class,
        WordBreakClass::MidLetter | WordBreakClass::MidNumLet | WordBreakClass::SingleQuote
    )
}

#[inline]
fn is_mid_num_mid_num_let_q(word_break_class: WordBreakClass) -> bool {
    matches!(word_break_class, WordBreakClass::MidNum | WordBreakClass::MidNumLet | WordBreakClass::SingleQuote)
}

#[inline]
fn is_newline(codepoint: &WordBreakUnit) -> bool {
    matches!(
        codepoint.word_break_class,
        WordBreakClass::CarriageReturn | WordBreakClass::LineFeed | WordBreakClass::Newline
    )
}
