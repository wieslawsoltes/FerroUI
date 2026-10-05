use super::codepoint::Codepoint;
use super::sentence_break_class::SentenceBreakClass;
use super::sentence_segment::SentenceSegment;

/// Enumerates Unicode sentence-boundary segments per UAX #29 rules SB1–SB11, SB998.
///
/// The enumerator borrows the text, so every operation is allocation-free. It reads
/// codepoints with [`Codepoint::read_at`] and classifies them via the [`SentenceBreakClass`]
/// property backed by the segmentation trie. Each codepoint is visited a constant
/// number of times: the left-hand context every rule needs is folded into a few fields as the
/// walk advances, and the one rule that looks ahead reuses the scan it already made.
#[derive(Clone, Debug)]
pub struct SentenceBreakEnumerator<'a> {
    text: &'a [u16],
    offset: usize,

    // The left-hand context, updated one codepoint at a time as the walk advances.
    // significant and prior_significant hold the last two classes that are not
    // Extend/Format, ignorable the class of the last codepoint when that one is.
    significant: SentenceBreakClass,
    prior_significant: SentenceBreakClass,
    ignorable: SentenceBreakClass,
    has_significant: bool,
    has_prior_significant: bool,
    last_is_ignorable: bool,

    // How far the text to the left matches the (STerm | ATerm) Close* Sp* (Sep | CR | LF)?
    // grammar shared by SB8 through SB11, and which terminator opened it.
    terminator: SentenceBreakClass,
    terminator_stage: TerminatorStage,

    // The SB8 lookahead, memoized. A scan that ran from the start and stopped at
    // the end of the range gives the same answer for every start position in between, so the
    // Close* Sp* run following an ATerm is scanned once rather than once per position.
    // `None` is the "no scan yet" state (-1/-1 upstream).
    lookahead: Option<(usize, usize)>,
    lookahead_result: bool,
}

// Stages of the (STerm | ATerm) Close* Sp* (Sep | CR | LF)? grammar, in the order
// they may appear. A match only ever moves forward through them.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u8)]
enum TerminatorStage {
    None,
    Terminator,
    Close,
    Space,
    Separator,
}

#[derive(Clone, Copy)]
struct SentenceBreakUnit {
    sentence_break_class: SentenceBreakClass,
    start: usize,
    end: usize,
}

impl SentenceBreakUnit {
    #[inline]
    fn new(codepoint: Codepoint, start: usize, end: usize) -> Self {
        Self { sentence_break_class: codepoint.sentence_break_class(), start, end }
    }
}

impl<'a> SentenceBreakEnumerator<'a> {
    /// Initializes a new instance of the [`SentenceBreakEnumerator`] struct.
    ///
    /// * `text` - The text to enumerate sentence segments over.
    #[inline]
    pub const fn new(text: &'a [u16]) -> Self {
        Self {
            text,
            offset: 0,
            significant: SentenceBreakClass::Other,
            prior_significant: SentenceBreakClass::Other,
            ignorable: SentenceBreakClass::Other,
            has_significant: false,
            has_prior_significant: false,
            last_is_ignorable: false,
            terminator: SentenceBreakClass::Other,
            terminator_stage: TerminatorStage::None,
            lookahead: None,
            lookahead_result: false,
        }
    }

    /// Moves to the next [`SentenceSegment`].
    ///
    /// Returns the current sentence-boundary segment, or `None` at the end of the text.
    pub fn move_next(&mut self) -> Option<SentenceSegment<'a>> {
        if self.offset >= self.text.len() {
            return None;
        }

        let segment_start = self.offset;
        let mut current = self.read_forward(self.offset);
        self.consume(current.sentence_break_class);
        let mut current_end = current.end;

        while current_end < self.text.len() {
            let next = self.read_forward(current_end);

            if self.is_boundary(&current, &next) {
                break;
            }

            current = next;
            self.consume(current.sentence_break_class);
            current_end = current.end;
        }

        let segment = SentenceSegment::new(segment_start, &self.text[segment_start..current_end]);
        self.offset = current_end;
        Some(segment)
    }

    // UAX-29 sentence boundary rules SB1–SB11 / SB998.
    // Rules are tested in order; the first matching rule wins.
    // SB1 (sot ÷) and SB2 (÷ eot) are implicit: the loop above starts at the
    // start of text and stops at the text length.
    fn is_boundary(&mut self, current: &SentenceBreakUnit, next: &SentenceBreakUnit) -> bool {
        // SB3: CR × LF — no break between CR and LF.
        if current.sentence_break_class == SentenceBreakClass::CarriageReturn
            && next.sentence_break_class == SentenceBreakClass::LineFeed
        {
            return false;
        }

        // SB4: Break after paragraph separators (Sep, CR, LF).
        if is_sep(current.sentence_break_class) {
            return true;
        }

        // SB5: X (Extend | Format) × X — Extend/Format do not break from the preceding char.
        // The right-side check keeps the walk from advancing past them; the left side is
        // resolved by left(), which reports the base an ignorable attaches to.
        if is_ignorable(next.sentence_break_class) {
            return false;
        }

        let left = self.left();
        let right = next.sentence_break_class;

        // SB6: ATerm × Numeric
        if left == SentenceBreakClass::ATerm && right == SentenceBreakClass::Numeric {
            return false;
        }

        // SB7: (Upper | Lower) ATerm × Upper
        if left == SentenceBreakClass::ATerm
            && right == SentenceBreakClass::Upper
            && self.has_prior_significant
            && is_upper_or_lower(self.prior_significant)
        {
            return false;
        }

        // SB8: ATerm Close* Sp* × (¬{OLetter|Upper|Sep|CR|LF|STerm|ATerm})* Lower
        // If the left context is ATerm Close* Sp*, and scanning right we find a Lower
        // (without hitting a blocker first), do not break.
        if self.terminator == SentenceBreakClass::ATerm
            && self.matches_terminator_context(TerminatorStage::Space)
            && self.has_lower_ahead(next.start)
        {
            return false;
        }

        // SB8a: (STerm | ATerm) Close* Sp* × (SContinue | STerm | ATerm)
        if self.matches_terminator_context(TerminatorStage::Space)
            && (right == SentenceBreakClass::SContinue
                || right == SentenceBreakClass::STerm
                || right == SentenceBreakClass::ATerm)
        {
            return false;
        }

        // SB9: (STerm | ATerm) Close* × (Close | Sp | Sep | CR | LF)
        if self.matches_terminator_context(TerminatorStage::Close)
            && (right == SentenceBreakClass::Close || right == SentenceBreakClass::Sp || is_sep(right))
        {
            return false;
        }

        // SB10: (STerm | ATerm) Close* Sp* × (Sp | Sep | CR | LF)
        if self.matches_terminator_context(TerminatorStage::Space) && (right == SentenceBreakClass::Sp || is_sep(right)) {
            return false;
        }

        // SB11: (STerm | ATerm) Close* Sp* (Sep | CR | LF)? ÷
        if self.matches_terminator_context(TerminatorStage::Separator) {
            return true;
        }

        // SB998: Otherwise, no break.
        false
    }

    // Folds one codepoint into the left-hand context.
    fn consume(&mut self, cls: SentenceBreakClass) {
        if is_ignorable(cls) {
            self.ignorable = cls;
            self.last_is_ignorable = true;

            // SB5 attaches an ignorable to the preceding base, leaving the context as it
            // was. After a paragraph separator there is no base to attach to, so the
            // ignorable stands on its own and matches no terminator grammar.
            if !self.has_significant || is_sep(self.significant) {
                self.terminator = SentenceBreakClass::Other;
                self.terminator_stage = TerminatorStage::None;
            }

            return;
        }

        self.prior_significant = self.significant;
        self.has_prior_significant = self.has_significant;
        self.significant = cls;
        self.has_significant = true;
        self.last_is_ignorable = false;

        // Walk the (STerm | ATerm) Close* Sp* (Sep | CR | LF)? grammar. Its stages are
        // ordered, so a class that would move back through them ends the match instead.
        if matches!(cls, SentenceBreakClass::ATerm | SentenceBreakClass::STerm) {
            self.terminator = cls;
            self.terminator_stage = TerminatorStage::Terminator;
        } else if self.terminator_stage == TerminatorStage::None {
            // No terminator to the left, so there is nothing to advance.
        } else if cls == SentenceBreakClass::Close && self.terminator_stage <= TerminatorStage::Close {
            self.terminator_stage = TerminatorStage::Close;
        } else if cls == SentenceBreakClass::Sp && self.terminator_stage <= TerminatorStage::Space {
            self.terminator_stage = TerminatorStage::Space;
        } else if is_sep(cls) && self.terminator_stage <= TerminatorStage::Space {
            self.terminator_stage = TerminatorStage::Separator;
        } else {
            self.terminator = SentenceBreakClass::Other;
            self.terminator_stage = TerminatorStage::None;
        }
    }

    // The effective class to the left of the next codepoint, per SB5: the base an
    // Extend/Format attaches to, or the ignorable itself when it has no base.
    #[inline]
    fn left(&self) -> SentenceBreakClass {
        if self.last_is_ignorable && (!self.has_significant || is_sep(self.significant)) {
            self.ignorable
        } else {
            self.significant
        }
    }

    // True when the text to the left matches (STerm | ATerm) Close* Sp* (Sep | CR | LF)?
    // truncated at the given stage: Close for SB9, Space for SB8/SB8a/SB10, Separator for SB11.
    #[inline]
    fn matches_terminator_context(&self, up_to: TerminatorStage) -> bool {
        self.terminator_stage != TerminatorStage::None && self.terminator_stage <= up_to
    }

    // SB8 lookahead: true iff a Lower is reachable from start without passing a blocker
    // (OLetter | Upper | Sep | CR | LF | STerm | ATerm). Extend/Format are transparent per SB5.
    fn has_lower_ahead(&mut self, start: usize) -> bool {
        if let Some((lookahead_start, lookahead_end)) = self.lookahead {
            if start >= lookahead_start && start <= lookahead_end {
                return self.lookahead_result;
            }
        }

        let mut position = start;
        let mut result = false;

        while position < self.text.len() {
            let (codepoint, count) = Codepoint::read_at(self.text, position);
            let cls = codepoint.sentence_break_class();

            if cls == SentenceBreakClass::Lower {
                result = true;
                break;
            }

            if is_sb8_blocker(cls) {
                break;
            }

            position += count;
        }

        self.lookahead = Some((start, position));
        self.lookahead_result = result;

        result
    }

    #[inline]
    fn read_forward(&self, start: usize) -> SentenceBreakUnit {
        let (codepoint, count) = Codepoint::read_at(self.text, start);
        SentenceBreakUnit::new(codepoint, start, start + count)
    }
}

impl<'a> Iterator for SentenceBreakEnumerator<'a> {
    type Item = SentenceSegment<'a>;

    #[inline]
    fn next(&mut self) -> Option<SentenceSegment<'a>> {
        self.move_next()
    }
}

// SB4: Sep | CR | LF are paragraph separators.
#[inline(always)]
fn is_sep(cls: SentenceBreakClass) -> bool {
    const MASK: u64 = (1u64 << SentenceBreakClass::Sep as i32)
        | (1u64 << SentenceBreakClass::CarriageReturn as i32)
        | (1u64 << SentenceBreakClass::LineFeed as i32);
    ((1u64 << cls as i32) & MASK) != 0
}

// SB5: Extend and Format are transparent (ignored) for sentence-boundary rules.
#[inline(always)]
fn is_ignorable(cls: SentenceBreakClass) -> bool {
    const MASK: u64 = (1u64 << SentenceBreakClass::Extend as i32) | (1u64 << SentenceBreakClass::Format as i32);
    ((1u64 << cls as i32) & MASK) != 0
}

#[inline(always)]
fn is_upper_or_lower(cls: SentenceBreakClass) -> bool {
    const MASK: u64 = (1u64 << SentenceBreakClass::Upper as i32) | (1u64 << SentenceBreakClass::Lower as i32);
    ((1u64 << cls as i32) & MASK) != 0
}

// SB8 forward-scan blocker set: classes that terminate the lookahead without
// matching Lower. Lower itself is the TARGET and is NOT included here.
#[inline(always)]
fn is_sb8_blocker(cls: SentenceBreakClass) -> bool {
    const MASK: u64 = (1u64 << SentenceBreakClass::OLetter as i32)
        | (1u64 << SentenceBreakClass::Upper as i32)
        | (1u64 << SentenceBreakClass::Sep as i32)
        | (1u64 << SentenceBreakClass::CarriageReturn as i32)
        | (1u64 << SentenceBreakClass::LineFeed as i32)
        | (1u64 << SentenceBreakClass::STerm as i32)
        | (1u64 << SentenceBreakClass::ATerm as i32);
    ((1u64 << cls as i32) & MASK) != 0
}
