use super::codepoint::Codepoint;
use super::codepoint_enumerator::CodepointEnumerator;
use super::general_category::GeneralCategory;
use super::grapheme_break_class::GraphemeBreakClass;
use super::line_break::LineBreak;
use super::line_break_class::LineBreakClass;

const DOT_CIRCLE: u32 = 0x25CC;

/// Enumerates the line break opportunities of UTF-16 text (UAX #14).
#[derive(Clone, Debug)]
pub struct LineBreakEnumerator<'a> {
    text: &'a [u16],
    state: LineBreakState,
}

impl<'a> LineBreakEnumerator<'a> {
    #[inline]
    pub fn new(text: &'a [u16]) -> Self {
        Self { text, state: LineBreakState::new() }
    }

    /// The text being enumerated (a public field upstream).
    #[inline]
    pub const fn text(&self) -> &'a [u16] {
        self.text
    }

    /// Moves to the next [`LineBreak`]; returns `None` when there are no more.
    pub fn move_next(&mut self) -> Option<LineBreak> {
        if self.text.is_empty() {
            return None;
        }

        if self.state.current.end_of_text {
            return None;
        }

        loop {
            self.state.read(self.text);

            if let Some(result) = execute_rules(self.text, &mut self.state) {
                return Some(result);
            }
        }
    }
}

impl Iterator for LineBreakEnumerator<'_> {
    type Item = LineBreak;

    #[inline]
    fn next(&mut self) -> Option<LineBreak> {
        self.move_next()
    }
}

#[inline(always)]
fn is_break_class(cls: LineBreakClass) -> bool {
    const MASK: u64 = (1u64 << LineBreakClass::MandatoryBreak as i32)
        | (1u64 << LineBreakClass::LineFeed as i32)
        | (1u64 << LineBreakClass::CarriageReturn as i32)
        | (1u64 << LineBreakClass::NextLine as i32);
    ((1u64 << cls as i32) & MASK) != 0
}

/// `Codepoint.ReadAt` for the signed indices this file computes: a negative
/// index is outside of the text, exactly as upstream's unsigned comparison
/// treats it.
#[inline(always)]
fn read_at(text: &[u16], index: i32) -> (Codepoint, i32) {
    if index < 0 {
        return (Codepoint::REPLACEMENT_CODEPOINT, 1);
    }

    let (codepoint, count) = Codepoint::read_at(text, index as usize);
    (codepoint, count as i32)
}

fn get_line_break(text: &[u16], state: &mut LineBreakState, is_required: bool) -> LineBreak {
    let mut position_measure = state.current.start + state.current.length;
    let position_wrap = position_measure;

    match state.current.line_break_class {
        LineBreakClass::Space | LineBreakClass::CarriageReturn | LineBreakClass::LineFeed => {
            if state.previous_class() == LineBreakClass::CarriageReturn {
                position_measure = find_prior_non_whitespace(text, state.previous().start);
            } else {
                position_measure = find_prior_non_whitespace(text, position_measure);
            }
        }
        _ => {}
    }

    LineBreak::new(position_measure.max(0) as usize, position_wrap.max(0) as usize, is_required)
}

fn find_prior_non_whitespace(text: &[u16], from: i32) -> i32 {
    let mut from = from;

    if from > 0 {
        let (cp, count) = read_at(text, from - 1);

        let cls = cp.line_break_class();

        if is_break_class(cls) {
            from -= count;
        }
    }

    while from > 0 {
        let (cp, count) = read_at(text, from - 1);

        let cls = cp.line_break_class();

        if cls == LineBreakClass::Space {
            from -= count;
        } else {
            break;
        }
    }

    from
}

fn execute_rules(text: &[u16], state: &mut LineBreakState) -> Option<LineBreak> {
    // Rules are invoked directly (not through a function table) so the compiler can
    // inline trivial early-exit rules and eliminate per-rule indirect-call
    // overhead. Order is significant: LB21a must run before LB21.
    macro_rules! rule {
        ($done:lifetime, $rule:ident) => {
            let res = $rule(text, state);
            if res != RuleResult::Pass {
                break $done res;
            }
        };
    }

    let res = 'done: {
        rule!('done, regional_indicator);
        rule!('done, lb03);
        rule!('done, lb04);
        rule!('done, lb05);
        rule!('done, lb06);
        rule!('done, lb07);
        rule!('done, lb08);
        rule!('done, lb08a);
        rule!('done, lb09);
        rule!('done, lb10);
        rule!('done, lb11);
        rule!('done, lb12);
        rule!('done, lb12a);
        rule!('done, lb13);
        rule!('done, lb14);
        rule!('done, lb15a);
        rule!('done, lb15b);
        rule!('done, lb15c);
        rule!('done, lb15d);
        rule!('done, lb16);
        rule!('done, lb17);
        rule!('done, lb18);
        rule!('done, lb19);
        rule!('done, lb20);
        rule!('done, lb20a);
        rule!('done, lb21a);
        rule!('done, lb21);
        rule!('done, lb21b);
        rule!('done, lb22);
        rule!('done, lb23);
        rule!('done, lb23a);
        rule!('done, lb24);
        rule!('done, lb25);
        rule!('done, lb26);
        rule!('done, lb27);
        rule!('done, lb28);
        rule!('done, lb28a);
        rule!('done, lb29);
        rule!('done, lb30);
        rule!('done, lb30a);
        rule!('done, lb30b);
        lb31(text, state) // always returns MayBreak
    };

    match res {
        RuleResult::NoBreak => None,
        RuleResult::MayBreak | RuleResult::MustBreak => {
            let is_required = is_break_class(state.current.line_break_class);
            Some(get_line_break(text, state, is_required))
        }
        RuleResult::Pass => None,
    }
}

fn regional_indicator(_text: &[u16], state: &mut LineBreakState) -> RuleResult {
    if state.current.inherited {
        return RuleResult::Pass;
    }

    if state.current.line_break_class == LineBreakClass::RegionalIndicator {
        state.regional_indicator += 1;
        if state.regional_indicator % 2 == 0 {
            state.regional_indicator = 0;
        }
    }

    RuleResult::Pass
}

/// LB3: Always break at the end of text.
#[inline(always)]
fn lb03(_text: &[u16], state: &mut LineBreakState) -> RuleResult {
    if state.current.end_of_text {
        return RuleResult::MustBreak;
    }

    RuleResult::Pass
}

/// LB4: Always break after hard line breaks.
#[inline(always)]
fn lb04(_text: &[u16], state: &mut LineBreakState) -> RuleResult {
    // BK !
    if state.current.line_break_class == LineBreakClass::MandatoryBreak {
        return RuleResult::MustBreak;
    }

    RuleResult::Pass
}

/// LB5: Treat CR followed by LF, as well as CR, LF, and NL as hard line
fn lb05(_text: &[u16], state: &mut LineBreakState) -> RuleResult {
    match state.current.line_break_class {
        LineBreakClass::CarriageReturn => {
            if state.next_class() == LineBreakClass::LineFeed {
                return RuleResult::NoBreak; // CR × LF
            }

            RuleResult::MustBreak // CR !
        }
        // LF !
        // NL !
        LineBreakClass::LineFeed | LineBreakClass::NextLine => RuleResult::MustBreak,
        _ => RuleResult::Pass,
    }
}

/// LB6: Do not break before hard line breaks.
#[inline(always)]
fn lb06(_text: &[u16], state: &mut LineBreakState) -> RuleResult {
    // × ( BK | CR | LF | NL )
    if is_break_class(state.next_class()) {
        return RuleResult::NoBreak;
    }

    RuleResult::Pass
}

/// LB7: Do not break before spaces or zero width space.
#[inline(always)]
fn lb07(_text: &[u16], state: &mut LineBreakState) -> RuleResult {
    // × SP
    // × ZW
    match state.next_class() {
        LineBreakClass::Space | LineBreakClass::ZWSpace => RuleResult::NoBreak,
        _ => RuleResult::Pass,
    }
}

/// LB8: Break before any character following a zero-width space, even if one or more spaces intervene.
fn lb08(_text: &[u16], state: &mut LineBreakState) -> RuleResult {
    if state.last_before_space.line_break_class == LineBreakClass::ZWSpace && state.next_class() != LineBreakClass::Space {
        return RuleResult::MayBreak;
    }

    RuleResult::Pass
}

/// LB8a: Do not break after a zero width joiner.
#[inline(always)]
fn lb08a(_text: &[u16], state: &mut LineBreakState) -> RuleResult {
    // ZWJ ×
    if state.current.line_break_class == LineBreakClass::ZWJ {
        return RuleResult::NoBreak;
    }

    RuleResult::Pass
}

/// LB9: Do not break a combining character sequence;
/// treat it as if it has the line breaking class of the base character in all of the following rules.
/// Treat ZWJ as if it were CM.
fn lb09(text: &[u16], state: &mut LineBreakState) -> RuleResult {
    // Treat X (CM | ZWJ)* as if it were X.
    // where X is any line break class except BK, CR, LF, NL, SP, or ZW.
    let cls = state.current.line_break_class;
    if is_break_class(cls) || cls == LineBreakClass::Space || cls == LineBreakClass::ZWSpace {
        return RuleResult::Pass;
    }

    match state.next_class() {
        LineBreakClass::CombiningMark | LineBreakClass::ZWJ => {
            state.ignore_next(text);
            RuleResult::NoBreak
        }
        _ => RuleResult::Pass,
    }
}

/// LB10: Treat any remaining combining mark or ZWJ as AL.
fn lb10(text: &[u16], state: &mut LineBreakState) -> RuleResult {
    if state.current.line_break_class == LineBreakClass::CombiningMark {
        state.current = BreakUnit { line_break_class: LineBreakClass::Alphabetic, inherited: true, ..state.current };
    }

    let next = state.next(text);

    if next.line_break_class == LineBreakClass::CombiningMark {
        state.replace_next(BreakUnit { line_break_class: LineBreakClass::Alphabetic, inherited: true, ..next });
    }

    RuleResult::Pass
}

/// LB11: Do not break before or after Word joiner and related characters.
#[inline(always)]
fn lb11(_text: &[u16], state: &mut LineBreakState) -> RuleResult {
    if state.next_class() == LineBreakClass::WordJoiner /* × WJ */
        || state.current.line_break_class == LineBreakClass::WordJoiner
    /* WJ × */
    {
        return RuleResult::NoBreak;
    }

    RuleResult::Pass
}

/// LB12: Do not break after NBSP and related characters.
#[inline(always)]
fn lb12(_text: &[u16], state: &mut LineBreakState) -> RuleResult {
    // GL ×
    if state.current.line_break_class == LineBreakClass::Glue {
        return RuleResult::NoBreak;
    }

    RuleResult::Pass
}

/// LB12a: Do not break before NBSP and related characters, except after spaces and hyphens.
fn lb12a(_text: &[u16], state: &mut LineBreakState) -> RuleResult {
    // [^SP BA HY] × GL
    if state.next_class() == LineBreakClass::Glue {
        return match state.current.line_break_class {
            LineBreakClass::Space
            | LineBreakClass::BreakAfter
            | LineBreakClass::Hyphen
            | LineBreakClass::UnambiguousHyphen => RuleResult::Pass,
            _ => RuleResult::NoBreak,
        };
    }

    RuleResult::Pass
}

/// LB13: Do not break before ‘]’ or ‘!’ or ‘;’ or ‘/’, even after spaces.
#[inline(always)]
fn lb13(_text: &[u16], state: &mut LineBreakState) -> RuleResult {
    // × CL
    // × CP
    // × EX
    // × SY
    match state.next_class() {
        LineBreakClass::ClosePunctuation
        | LineBreakClass::CloseParenthesis
        | LineBreakClass::Exclamation
        | LineBreakClass::BreakSymbols => RuleResult::NoBreak,
        _ => RuleResult::Pass,
    }
}

/// LB14: Do not break after ‘[’, even after spaces.
#[inline(always)]
fn lb14(_text: &[u16], state: &mut LineBreakState) -> RuleResult {
    // OP SP* ×
    if state.last_before_whitespace.line_break_class == LineBreakClass::OpenPunctuation {
        return RuleResult::NoBreak;
    }

    RuleResult::Pass
}

/// LB15a: Do not break after an unresolved initial punctuation that lies at the start of the line,
/// after a space, after opening punctuation, or after an unresolved quotation mark, even after spaces.
fn lb15a(text: &[u16], state: &mut LineBreakState) -> RuleResult {
    fn is_start_like(unit: BreakUnit) -> bool {
        if (unit.start_of_text && unit.length == 0) || is_break_class(unit.line_break_class) {
            return true;
        }

        matches!(
            unit.line_break_class,
            LineBreakClass::OpenPunctuation
                | LineBreakClass::Quotation
                | LineBreakClass::Glue
                | LineBreakClass::Space
                | LineBreakClass::ZWSpace
        )
    }

    // (sot | BK | CR | LF | NL | OP | QU | GL | SP | ZW) [\p{Pi}&QU] SP* ×
    if state.last_before_whitespace.codepoint.general_category() == GeneralCategory::InitialPunctuation
        && state.last_before_whitespace.line_break_class == LineBreakClass::Quotation
        && is_start_like(LineBreakState::before(text, state.last_before_whitespace))
    {
        return RuleResult::NoBreak;
    }

    RuleResult::Pass
}

/// LB15b: Do not break before an unresolved final punctuation that lies at the end of the line,
/// before a space, before a prohibited break, or before an unresolved quotation mark, even after spaces.
fn lb15b(text: &[u16], state: &mut LineBreakState) -> RuleResult {
    // × [\p{Pf}&QU] ( SP | GL | WJ | CL | QU | CP | EX | IS | SY | BK | CR | LF | NL | ZW | eot)
    if state.next(text).codepoint.general_category() == GeneralCategory::FinalPunctuation
        && (state.next_class() == LineBreakClass::Quotation)
    {
        let after = LineBreakState::after(text, state.next(text));

        if after.end_of_text {
            // Only on eot
            return RuleResult::NoBreak;
        }

        if is_break_class(after.line_break_class) {
            return RuleResult::NoBreak;
        }

        if matches!(
            after.line_break_class,
            LineBreakClass::Space
                | LineBreakClass::Glue
                | LineBreakClass::WordJoiner
                | LineBreakClass::ClosePunctuation
                | LineBreakClass::Quotation
                | LineBreakClass::CloseParenthesis
                | LineBreakClass::Exclamation
                | LineBreakClass::InfixNumeric
                | LineBreakClass::BreakSymbols
                | LineBreakClass::ZWSpace
        ) {
            return RuleResult::NoBreak;
        }
    }

    RuleResult::Pass
}

/// LB15c: Break before a decimal mark that follows a space, for instance, in ‘subtract .5’.
fn lb15c(text: &[u16], state: &mut LineBreakState) -> RuleResult {
    // SP ÷ IS NU
    if state.current.line_break_class == LineBreakClass::Space
        && state.next_class() == LineBreakClass::InfixNumeric
        && LineBreakState::after(text, state.next(text)).line_break_class == LineBreakClass::Numeric
    {
        return RuleResult::MayBreak;
    }

    RuleResult::Pass
}

/// LB15d: Otherwise, do not break before ‘;’, ‘,’, or ‘.’, even after spaces.
#[inline(always)]
fn lb15d(_text: &[u16], state: &mut LineBreakState) -> RuleResult {
    // × IS
    if state.next_class() == LineBreakClass::InfixNumeric {
        return RuleResult::NoBreak;
    }

    RuleResult::Pass
}

/// LB16: Do not break between closing punctuation and a nonstarter (lb=NS),
/// even with intervening spaces.
fn lb16(text: &[u16], state: &mut LineBreakState) -> RuleResult {
    if matches!(
        state.last_before_whitespace.line_break_class,
        LineBreakClass::ClosePunctuation | LineBreakClass::CloseParenthesis
    ) {
        let class_after_spaces = LineBreakState::class_after_spaces(text, state.current);

        if matches!(class_after_spaces, LineBreakClass::ConditionalJapaneseStarter | LineBreakClass::Nonstarter) {
            return RuleResult::NoBreak;
        }
    }

    RuleResult::Pass
}

/// LB17: Do not break within ‘——’, even with intervening spaces.
fn lb17(text: &[u16], state: &mut LineBreakState) -> RuleResult {
    // B2 SP* × B2
    if state.last_before_whitespace.line_break_class == LineBreakClass::BreakBoth
        && LineBreakState::class_after_spaces(text, state.current) == LineBreakClass::BreakBoth
    {
        return RuleResult::NoBreak;
    }

    RuleResult::Pass
}

/// LB18: Break after spaces.
#[inline(always)]
fn lb18(_text: &[u16], state: &mut LineBreakState) -> RuleResult {
    // SP ÷
    if state.current.line_break_class == LineBreakClass::Space {
        return RuleResult::MayBreak;
    }

    RuleResult::Pass
}

/// LB19: Do not break before or after quotation marks.
fn lb19(text: &[u16], state: &mut LineBreakState) -> RuleResult {
    let next = state.next(text);

    if next.line_break_class == LineBreakClass::Quotation
        && next.codepoint.general_category() != GeneralCategory::InitialPunctuation
    {
        return RuleResult::NoBreak;
    }

    if state.current.line_break_class == LineBreakClass::Quotation
        && state.current.codepoint.general_category() != GeneralCategory::FinalPunctuation
    {
        return RuleResult::NoBreak;
    }

    if !state.current.codepoint.is_east_asian() && next.line_break_class == LineBreakClass::Quotation {
        return RuleResult::NoBreak;
    }

    if next.line_break_class == LineBreakClass::Quotation {
        let after = LineBreakState::after(text, next);

        if after.end_of_text || !after.codepoint.is_east_asian() {
            return RuleResult::NoBreak;
        }
    }

    if state.current.line_break_class == LineBreakClass::Quotation && !next.codepoint.is_east_asian() {
        return RuleResult::NoBreak;
    }

    let previous = state.previous();

    if ((previous.start_of_text && previous.length == 0) || !previous.codepoint.is_east_asian())
        && state.current.line_break_class == LineBreakClass::Quotation
    {
        return RuleResult::NoBreak;
    }

    RuleResult::Pass
}

/// LB20: Break before and after unresolved CB.
#[inline(always)]
fn lb20(_text: &[u16], state: &mut LineBreakState) -> RuleResult {
    // ÷ CB
    // CB ÷
    if (state.current.line_break_class == LineBreakClass::ContingentBreak)
        || (state.next_class() == LineBreakClass::ContingentBreak)
    {
        return RuleResult::MayBreak;
    }

    RuleResult::Pass
}

/// LB20a: Do not break after a word-initial hyphen.
fn lb20a(text: &[u16], state: &mut LineBreakState) -> RuleResult {
    fn is_match(unit: BreakUnit) -> bool {
        if unit.start_of_text && unit.length == 0 {
            return true;
        }

        if is_break_class(unit.line_break_class) {
            return true;
        }

        matches!(
            unit.line_break_class,
            LineBreakClass::Space | LineBreakClass::ZWSpace | LineBreakClass::ContingentBreak | LineBreakClass::Glue
        )
    }

    let current = if state.current.inherited { state.last_before_whitespace } else { state.current };
    let previous = if state.current.inherited { LineBreakState::before(text, current) } else { state.previous() };

    // (sot | BK | CR | LF | NL | SP | ZW | CB | GL)(HY | HH) × (AL | HL)
    if is_match(previous)
        && matches!(state.next_class(), LineBreakClass::Alphabetic | LineBreakClass::HebrewLetter)
        && matches!(current.line_break_class, LineBreakClass::Hyphen | LineBreakClass::UnambiguousHyphen)
    {
        return RuleResult::NoBreak;
    }

    RuleResult::Pass
}

/// LB21: Do not break before hyphen-minus, other hyphens, fixed-width spaces, small kana, and other non-starters, or after acute accents.
fn lb21(_text: &[u16], state: &mut LineBreakState) -> RuleResult {
    // × (BA | HY | NS)
    if matches!(
        state.next_class(),
        // [21.01]
        LineBreakClass::BreakAfter
        // [21.02]
        | LineBreakClass::UnambiguousHyphen
        // [21.01]
        | LineBreakClass::Hyphen
        // [21.01]
        | LineBreakClass::Nonstarter
    ) {
        return RuleResult::NoBreak;
    }

    // [21.04] BB ×
    if state.current.line_break_class == LineBreakClass::BreakBefore {
        return RuleResult::NoBreak;
    }

    RuleResult::Pass
}

/// LB21a: Don't break after Hebrew + Hyphen.
fn lb21a(_text: &[u16], state: &mut LineBreakState) -> RuleResult {
    if state.next_class() != LineBreakClass::HebrewLetter {
        // [21.1] HL(HY|HH) × [^HL]
        if state.previous_class() == LineBreakClass::HebrewLetter
            && matches!(state.current.line_break_class, LineBreakClass::Hyphen | LineBreakClass::UnambiguousHyphen)
        {
            return RuleResult::NoBreak;
        }
    }

    RuleResult::Pass
}

/// LB21b: Don’t break between Solidus and Hebrew letters.
#[inline(always)]
fn lb21b(_text: &[u16], state: &mut LineBreakState) -> RuleResult {
    // [21.2] SY × HL
    if (state.current.line_break_class == LineBreakClass::BreakSymbols) && (state.next_class() == LineBreakClass::HebrewLetter)
    {
        return RuleResult::NoBreak;
    }

    RuleResult::Pass
}

/// LB22: Do not break before ellipses.
#[inline(always)]
fn lb22(_text: &[u16], state: &mut LineBreakState) -> RuleResult {
    // × IN
    if state.next_class() == LineBreakClass::Inseparable {
        return RuleResult::NoBreak;
    }

    RuleResult::Pass
}

/// LB23: Do not break between digits and letters.
fn lb23(_text: &[u16], state: &mut LineBreakState) -> RuleResult {
    match state.current.line_break_class {
        LineBreakClass::Alphabetic | LineBreakClass::HebrewLetter => {
            // (AL | HL) × NU
            if state.next_class() == LineBreakClass::Numeric {
                return RuleResult::NoBreak;
            }
        }
        LineBreakClass::Numeric => {
            // NU × (AL | HL)
            if matches!(state.next_class(), LineBreakClass::Alphabetic | LineBreakClass::HebrewLetter) {
                return RuleResult::NoBreak;
            }
        }
        _ => {}
    }

    RuleResult::Pass
}

#[inline(always)]
fn is_id_eb_em(cls: LineBreakClass) -> bool {
    cls == LineBreakClass::Ideographic || cls == LineBreakClass::EBase || cls == LineBreakClass::EModifier
}

#[inline(always)]
fn is_pr_po(cls: LineBreakClass) -> bool {
    cls == LineBreakClass::PrefixNumeric || cls == LineBreakClass::PostfixNumeric
}

/// LB23a: Do not break between numeric prefixes and ideographs, or between
/// ideographs and numeric postfixes.
#[inline(always)]
fn lb23a(_text: &[u16], state: &mut LineBreakState) -> RuleResult {
    let current_class = state.current.line_break_class;
    let next_class = state.next_class();

    // LB23a: PR × (ID | EB | EM)
    if current_class == LineBreakClass::PrefixNumeric && is_id_eb_em(next_class) {
        return RuleResult::NoBreak;
    }

    //  (ID | EB | EM) × PO
    if next_class == LineBreakClass::PostfixNumeric && is_id_eb_em(current_class) {
        return RuleResult::NoBreak;
    }

    RuleResult::Pass
}

/// LB24: Do not break between numeric prefix/postfix and letters, or between
/// letters and prefix/postfix.
#[inline(always)]
fn lb24(_text: &[u16], state: &mut LineBreakState) -> RuleResult {
    let current_class = state.current.line_break_class;
    let next_class = state.next_class();

    // LB24: (PR | PO) × (AL | HL);
    if is_pr_po(current_class) && (next_class == LineBreakClass::Alphabetic || next_class == LineBreakClass::HebrewLetter) {
        return RuleResult::NoBreak;
    }

    // (AL | HL) × (PR | PO)
    if (current_class == LineBreakClass::Alphabetic || current_class == LineBreakClass::HebrewLetter) && is_pr_po(next_class)
    {
        return RuleResult::NoBreak;
    }

    RuleResult::Pass
}

/// LB25: Do not break between the following pairs of classes relevant to numbers
fn lb25(text: &[u16], state: &mut LineBreakState) -> RuleResult {
    match state.next_class() {
        // [25.06] NU(SY|IS)* x PR
        LineBreakClass::PrefixNumeric => {
            match state.current.line_break_class {
                // [25.04] NU(SY|IS)* CP × PR
                LineBreakClass::CloseParenthesis => match state.previous_class() {
                    LineBreakClass::Numeric => {
                        return RuleResult::NoBreak;
                    }
                    LineBreakClass::BreakSymbols | LineBreakClass::InfixNumeric => {
                        let previous = state.previous();
                        if LineBreakState::before(text, previous).line_break_class == LineBreakClass::Numeric {
                            return RuleResult::NoBreak;
                        }
                    }
                    _ => {}
                },
                LineBreakClass::Numeric => {
                    return RuleResult::NoBreak;
                }
                LineBreakClass::BreakSymbols | LineBreakClass::InfixNumeric => {
                    if state.previous_class() == LineBreakClass::Numeric {
                        return RuleResult::NoBreak;
                    }
                }
                // [25.03] NU(SY|IS)* CL × PR
                LineBreakClass::ClosePunctuation => match state.previous_class() {
                    LineBreakClass::Numeric => {
                        return RuleResult::NoBreak;
                    }
                    LineBreakClass::BreakSymbols | LineBreakClass::InfixNumeric => {
                        // Upstream re-tests the previous class here (it can never be
                        // Numeric in this arm); kept as is.
                        if state.previous_class() == LineBreakClass::Numeric {
                            return RuleResult::NoBreak;
                        }
                    }
                    _ => {}
                },
                _ => {}
            }
        }
        // [25.15] NU(SY|IS)* ×	NU
        LineBreakClass::Numeric => match state.current.line_break_class {
            LineBreakClass::Numeric => {
                return RuleResult::NoBreak;
            }
            LineBreakClass::BreakSymbols | LineBreakClass::InfixNumeric => {
                if state.previous_class() == LineBreakClass::Numeric {
                    return RuleResult::NoBreak;
                }
            }
            _ => {}
        },
        LineBreakClass::PostfixNumeric => {
            match state.current.line_break_class {
                // [25.01] NU(SY|IS)* CL × PO
                LineBreakClass::ClosePunctuation => match state.previous_class() {
                    LineBreakClass::Numeric => {
                        return RuleResult::NoBreak;
                    }
                    LineBreakClass::BreakSymbols | LineBreakClass::InfixNumeric => {
                        // Upstream re-tests the previous class here (it can never be
                        // Numeric in this arm); kept as is.
                        if state.previous_class() == LineBreakClass::Numeric {
                            return RuleResult::NoBreak;
                        }
                    }
                    _ => {}
                },
                // [25.05] NU(SY|IS)* ×	PO
                LineBreakClass::Numeric => {
                    return RuleResult::NoBreak;
                }
                LineBreakClass::BreakSymbols | LineBreakClass::InfixNumeric => {
                    if state.previous_class() == LineBreakClass::Numeric {
                        return RuleResult::NoBreak;
                    }
                }
                _ => {}
            }
        }
        _ => {}
    }

    if state.current.line_break_class == LineBreakClass::PrefixNumeric {
        match state.next_class() {
            LineBreakClass::OpenPunctuation => {
                let after_next = LineBreakState::after(text, state.next(text));

                // [25.1] PR × OP NU
                if after_next.line_break_class == LineBreakClass::Numeric {
                    return RuleResult::NoBreak;
                }

                // PR × OP IS NU
                if after_next.line_break_class == LineBreakClass::InfixNumeric
                    && LineBreakState::after(text, after_next).line_break_class == LineBreakClass::Numeric
                {
                    return RuleResult::NoBreak;
                }
            }
            // PR × NU
            LineBreakClass::Numeric => {
                return RuleResult::NoBreak;
            }
            _ => {}
        }
    }

    if state.current.line_break_class == LineBreakClass::PostfixNumeric {
        match state.next_class() {
            LineBreakClass::OpenPunctuation => {
                let after_next = LineBreakState::after(text, state.next(text));

                // PO × OP NU
                if after_next.line_break_class == LineBreakClass::Numeric {
                    return RuleResult::NoBreak;
                }

                // PO × OP IS NU
                if after_next.line_break_class == LineBreakClass::InfixNumeric
                    && LineBreakState::after(text, after_next).line_break_class == LineBreakClass::Numeric
                {
                    return RuleResult::NoBreak;
                }
            }
            // PO × NU
            LineBreakClass::Numeric => {
                return RuleResult::NoBreak;
            }
            _ => {}
        }
    }

    // HY × NU
    // [25.14] IS × NU
    if matches!(state.current.line_break_class, LineBreakClass::Hyphen | LineBreakClass::InfixNumeric)
        && state.next_class() == LineBreakClass::Numeric
    {
        return RuleResult::NoBreak;
    }

    RuleResult::Pass
}

/// LB26: Do not break a Korean syllable.
fn lb26(_text: &[u16], state: &mut LineBreakState) -> RuleResult {
    match state.current.line_break_class {
        LineBreakClass::JL => {
            // JL × (JL | JV | H2 | H3)
            if matches!(
                state.next_class(),
                LineBreakClass::JL | LineBreakClass::JV | LineBreakClass::H2 | LineBreakClass::H3
            ) {
                return RuleResult::NoBreak;
            }
        }
        LineBreakClass::JV | LineBreakClass::H2 => {
            // (JV | H2) × (JV | JT)
            if matches!(state.next_class(), LineBreakClass::JV | LineBreakClass::JT) {
                return RuleResult::NoBreak;
            }
        }
        LineBreakClass::JT | LineBreakClass::H3 => {
            // (JT | H3) × JT
            if state.next_class() == LineBreakClass::JT {
                return RuleResult::NoBreak;
            }
        }
        _ => {}
    }

    RuleResult::Pass
}

/// LB27: Treat a Korean Syllable Block the same as ID.
fn lb27(_text: &[u16], state: &mut LineBreakState) -> RuleResult {
    match state.current.line_break_class {
        LineBreakClass::JL | LineBreakClass::JV | LineBreakClass::JT | LineBreakClass::H2 | LineBreakClass::H3 => {
            // (JL | JV | JT | H2 | H3) × PO
            if state.next_class() == LineBreakClass::PostfixNumeric {
                return RuleResult::NoBreak;
            }
        }
        LineBreakClass::PrefixNumeric => {
            // PR × (JL | JV | JT | H2 | H3)
            if matches!(
                state.next_class(),
                LineBreakClass::JL | LineBreakClass::JV | LineBreakClass::JT | LineBreakClass::H2 | LineBreakClass::H3
            ) {
                return RuleResult::NoBreak;
            }
        }
        _ => {}
    }

    RuleResult::Pass
}

/// LB28: Do not break between alphabetics (“at”).
#[inline(always)]
fn lb28(_text: &[u16], state: &mut LineBreakState) -> RuleResult {
    // [28.0] (AL | HL) × (AL | HL)
    if matches!(state.current.line_break_class, LineBreakClass::Alphabetic | LineBreakClass::HebrewLetter)
        && matches!(state.next_class(), LineBreakClass::Alphabetic | LineBreakClass::HebrewLetter)
    {
        return RuleResult::NoBreak;
    }

    RuleResult::Pass
}

/// LB28a: Do not break inside the orthographic syllables of Brahmic scripts.
fn lb28a(text: &[u16], state: &mut LineBreakState) -> RuleResult {
    // (AK | DottedCircle | AS)
    fn is_match(chr: BreakUnit) -> bool {
        (chr.line_break_class == LineBreakClass::Aksara)
            || (chr.codepoint.value() == DOT_CIRCLE)
            || (chr.line_break_class == LineBreakClass::AksaraStart)
    }

    let current = if state.current.inherited { state.last_before_whitespace } else { state.current };
    let previous = if state.current.inherited { LineBreakState::before(text, current) } else { state.previous() };

    // [28.11] AP × (AK | DottedCircle | AS)
    if (current.line_break_class == LineBreakClass::AksaraPrebase) && is_match(state.next(text)) {
        return RuleResult::NoBreak;
    }

    // [28.12] (AK | DottedCircle | AS) × (VF | VI)
    if is_match(current)
        && ((state.next_class() == LineBreakClass::ViramaFinal) || (state.next_class() == LineBreakClass::Virama))
    {
        return RuleResult::NoBreak;
    }

    // [28.13] (AK | DottedCircle| AS) VI × (AK | DottedCircle)
    if is_match(previous)
        && current.line_break_class == LineBreakClass::Virama
        && ((state.next_class() == LineBreakClass::Aksara) || (state.next(text).codepoint.value() == DOT_CIRCLE))
    {
        return RuleResult::NoBreak;
    }

    // [28.14] (AK | DottedCircle | AS) × (AK | DottedCircle | AS) VF
    if is_match(current)
        && is_match(state.next(text))
        && (LineBreakState::after(text, state.next(text)).line_break_class == LineBreakClass::ViramaFinal)
    {
        return RuleResult::NoBreak;
    }

    RuleResult::Pass
}

/// LB29: Do not break between numeric punctuation and alphabetics (“e.g.”).
#[inline(always)]
fn lb29(_text: &[u16], state: &mut LineBreakState) -> RuleResult {
    // IS × (AL | HL)
    if (state.current.line_break_class == LineBreakClass::InfixNumeric)
        && ((state.next_class() == LineBreakClass::Alphabetic) || (state.next_class() == LineBreakClass::HebrewLetter))
    {
        return RuleResult::NoBreak;
    }

    RuleResult::Pass
}

/// LB30: Do not break between letters, numbers, or ordinary symbols and opening or closing parentheses.
fn lb30(text: &[u16], state: &mut LineBreakState) -> RuleResult {
    match state.current.line_break_class {
        LineBreakClass::Alphabetic | LineBreakClass::HebrewLetter | LineBreakClass::Numeric => {
            let next = state.next(text);

            // (AL | HL | NU) × [OP-[\p{ea=F}\p{ea=W}\p{ea=H}]]
            if (next.line_break_class == LineBreakClass::OpenPunctuation) && !next.codepoint.is_east_asian() {
                return RuleResult::NoBreak;
            }
        }
        LineBreakClass::CloseParenthesis => {
            // [CP-[\p{ea=F}\p{ea=W}\p{ea=H}]] × (AL | HL | NU)
            if !state.current.codepoint.is_east_asian()
                && matches!(
                    state.next_class(),
                    LineBreakClass::Alphabetic | LineBreakClass::HebrewLetter | LineBreakClass::Numeric
                )
            {
                return RuleResult::NoBreak;
            }
        }
        _ => {}
    }

    RuleResult::Pass
}

/// LB30a: Break between two regional indicator symbols if and only if there
/// are an even number of regional indicators preceding the position of the
/// break.
fn lb30a(_text: &[u16], state: &mut LineBreakState) -> RuleResult {
    if state.regional_indicator > 0
        && state.next_class() == LineBreakClass::RegionalIndicator
        && state.regional_indicator + 1 == 2
    {
        return RuleResult::NoBreak;
    }

    RuleResult::Pass
}

/// LB30b: Do not break between an emoji base (or potential emoji) and an emoji modifier.
fn lb30b(_text: &[u16], state: &mut LineBreakState) -> RuleResult {
    let current = if state.current.inherited { state.previous() } else { state.current };

    // EB × EM
    if (current.line_break_class == LineBreakClass::EBase) && (state.next_class() == LineBreakClass::EModifier) {
        return RuleResult::NoBreak;
    }

    // [\p{Extended_Pictographic}&&\p{Cn}] × EM
    //
    // The Extended_Pictographic property is used to customize segmentation (as
    // described in [UAX29] and [UAX14]) so that possible future emoji ZWJ
    // sequences will not break grapheme clusters, words, or lines. Unassigned
    // codepoints with Line_Break=ID in some blocks are also assigned the
    // Extended_Pictographic property. Those blocks are intended for future
    // allocation of emoji characters.
    if state.next_class() == LineBreakClass::EModifier
        && current.codepoint.grapheme_break_class() == GraphemeBreakClass::ExtendedPictographic
        && current.codepoint.general_category() == GeneralCategory::Unassigned
    {
        return RuleResult::NoBreak;
    }

    RuleResult::Pass
}

/// LB31: Break everywhere else.
#[inline(always)]
fn lb31(_text: &[u16], _state: &mut LineBreakState) -> RuleResult {
    RuleResult::MayBreak
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum RuleResult {
    Pass,
    NoBreak,
    MayBreak,
    MustBreak,
}

/// One codepoint of the text with its resolved line break class.
///
/// `start` and `length` are signed like upstream: looking before the first
/// unit yields a unit that starts at -1.
#[derive(Clone, Copy, Debug)]
struct BreakUnit {
    start: i32,
    length: i32,
    codepoint: Codepoint,
    end_of_text: bool,
    start_of_text: bool,
    line_break_class: LineBreakClass,
    ignored: bool,
    inherited: bool,
}

impl BreakUnit {
    /// The parameterless constructor upstream: everything zero, class Unknown.
    const fn empty() -> Self {
        Self {
            start: 0,
            length: 0,
            codepoint: Codepoint::new(0),
            end_of_text: false,
            start_of_text: false,
            line_break_class: LineBreakClass::Unknown,
            ignored: false,
            inherited: false,
        }
    }

    /// `default(BreakUnit)` upstream: everything zero, including the class.
    const DEFAULT: BreakUnit = BreakUnit { line_break_class: LineBreakClass::OpenPunctuation, ..Self::empty() };

    const SOT: BreakUnit = BreakUnit { start_of_text: true, ..Self::empty() };
    const EOT: BreakUnit = BreakUnit { end_of_text: true, ..Self::empty() };

    #[inline]
    fn new(codepoint: Codepoint, start: i32, length: i32) -> Self {
        Self { codepoint, start, length, line_break_class: Self::map_class(codepoint), ..Self::empty() }
    }

    fn map_class(cp: Codepoint) -> LineBreakClass {
        if cp.value() == 327685 {
            return LineBreakClass::Alphabetic;
        }

        // LB 1
        // ==========================================
        // Resolved Original    General_Category
        // ==========================================
        // AL       AI, SG, XX  Any
        // CM       SA          Only Mn or Mc
        // AL       SA          Any except Mn and Mc
        // NS       CJ          Any
        let cls = cp.line_break_class();

        const SPECIAL_MASK: u64 = (1u64 << LineBreakClass::Ambiguous as i32)
            | (1u64 << LineBreakClass::Surrogate as i32)
            | (1u64 << LineBreakClass::Unknown as i32)
            | (1u64 << LineBreakClass::ComplexContext as i32)
            | (1u64 << LineBreakClass::ConditionalJapaneseStarter as i32);

        if ((1u64 << cls as i32) & SPECIAL_MASK) != 0 {
            match cls {
                LineBreakClass::Ambiguous | LineBreakClass::Surrogate | LineBreakClass::Unknown => {
                    return LineBreakClass::Alphabetic;
                }
                LineBreakClass::ComplexContext => {
                    return if matches!(
                        cp.general_category(),
                        GeneralCategory::NonspacingMark | GeneralCategory::SpacingMark
                    ) {
                        LineBreakClass::CombiningMark
                    } else {
                        LineBreakClass::Alphabetic
                    };
                }
                LineBreakClass::ConditionalJapaneseStarter => {
                    return LineBreakClass::Nonstarter;
                }
                _ => {}
            }
        }

        cls
    }
}

#[derive(Clone, Debug)]
struct LineBreakState {
    // next is resolved lazily on first read; subsequent reads eagerly
    // pre-peek so that next_class is a plain field load inside every rule
    // (avoiding the BreakUnit copy that reading the next unit
    // would otherwise incur per call).
    next: BreakUnit,
    has_next: bool,
    next_class: LineBreakClass,
    previous: BreakUnit,
    previous_class: LineBreakClass,

    current: BreakUnit,
    position: i32,
    regional_indicator: i32,
    last_before_whitespace: BreakUnit,
    last_before_space: BreakUnit,
}

impl LineBreakState {
    fn new() -> Self {
        Self {
            has_next: false,
            next: BreakUnit::DEFAULT,
            next_class: LineBreakClass::Unknown,
            previous: BreakUnit::SOT,
            previous_class: BreakUnit::SOT.line_break_class,
            current: BreakUnit::SOT,
            position: 0,
            regional_indicator: 0,
            last_before_space: BreakUnit::SOT,
            last_before_whitespace: BreakUnit::SOT,
        }
    }

    /// Cached line break class of the previous unit. Updated whenever
    /// the previous unit would be reassigned (in `read`
    /// or via the Ignored/Inherited fall-through here).
    #[inline]
    fn previous_class(&mut self) -> LineBreakClass {
        if self.previous.ignored || self.previous.inherited {
            self.previous = self.last_before_whitespace;
            self.previous_class = self.last_before_whitespace.line_break_class;
        }
        self.previous_class
    }

    #[inline]
    fn previous(&mut self) -> BreakUnit {
        if self.previous.ignored || self.previous.inherited {
            self.previous = self.last_before_whitespace;
            self.previous_class = self.last_before_whitespace.line_break_class;
        }
        self.previous
    }

    #[inline]
    fn next(&mut self, text: &[u16]) -> BreakUnit {
        if !self.has_next {
            self.next = self.peek(text);
            self.next_class = self.next.line_break_class;
            self.has_next = true;
        }

        self.next
    }

    /// Cached line break class of the next unit. Faster than
    /// reading the next unit because it avoids the
    /// BreakUnit copy. `read` is responsible for ensuring has_next
    /// is set before any rule sees this value.
    #[inline(always)]
    fn next_class(&self) -> LineBreakClass {
        self.next_class
    }

    fn after(text: &[u16], current: BreakUnit) -> BreakUnit {
        if current.end_of_text {
            return BreakUnit::EOT;
        }

        Self::peek_at(text, current.start + current.length)
    }

    fn before(text: &[u16], current: BreakUnit) -> BreakUnit {
        if current.start_of_text {
            return BreakUnit::SOT;
        }

        let position = current.start - 1;

        Self::peek_at(text, position)
    }

    fn ignore_next(&mut self, text: &[u16]) {
        let n = BreakUnit { ignored: true, ..self.next(text) };
        self.next = n;
        self.next_class = n.line_break_class;
        self.has_next = true;
    }

    fn replace_next(&mut self, next: BreakUnit) {
        self.next = next;
        self.next_class = next.line_break_class;
        self.has_next = true;
    }

    fn peek_at(text: &[u16], index: i32) -> BreakUnit {
        if text.is_empty() {
            return BreakUnit { start_of_text: true, ..BreakUnit::new(Codepoint::REPLACEMENT_CODEPOINT, index, 0) };
        }

        if index >= text.len() as i32 {
            return BreakUnit::EOT;
        }

        let (codepoint, count) = read_at(text, index);

        BreakUnit {
            end_of_text: index + count == text.len() as i32,
            start_of_text: index == 0,
            ..BreakUnit::new(codepoint, index, count)
        }
    }

    #[inline]
    fn peek(&self, text: &[u16]) -> BreakUnit {
        Self::peek_at(text, self.position)
    }

    fn read(&mut self, text: &[u16]) -> BreakUnit {
        self.previous = self.current;
        self.previous_class = self.current.line_break_class;

        let next = self.next(text);

        self.current = next;

        self.position += next.length;

        // Eagerly peek the new "next" so the cached next_class is valid
        // for every rule in the upcoming execute_rules pass.
        self.next = self.peek(text);
        self.next_class = self.next.line_break_class;
        self.has_next = true;

        // LB9 ignored marks do not become the prior item for the next real boundary.
        if self.previous.ignored || self.previous.inherited {
            self.previous = self.last_before_whitespace;
            self.previous_class = self.last_before_whitespace.line_break_class;
        }

        if self.current.ignored {
            let line_break_class = self.previous().line_break_class;
            self.current = BreakUnit { line_break_class, inherited: true, ..self.current };
        }

        let current = if self.current.inherited { self.previous() } else { self.current };

        if !self.current.codepoint.is_white_space() {
            self.last_before_whitespace = current;
        }

        if self.current.line_break_class != LineBreakClass::Space {
            self.last_before_space = current;
        }

        next
    }

    fn class_after_spaces(text: &[u16], current: BreakUnit) -> LineBreakClass {
        let position = current.start + current.length;

        if position >= text.len() as i32 {
            return current.line_break_class;
        }

        // A negative position cannot be sliced (upstream throws there as well).
        let mut enumerator = CodepointEnumerator::new(&text[position as usize..]);

        loop {
            match enumerator.move_next() {
                Some(cp) => {
                    if cp.line_break_class() != LineBreakClass::Space {
                        return cp.line_break_class();
                    }
                }
                // At the end the enumerator leaves the replacement codepoint in its out value.
                None => return Codepoint::REPLACEMENT_CODEPOINT.line_break_class(),
            }
        }
    }
}
