//! Whitespace, word-boundary, punctuation and script analysis.

use unicode_general_category::{get_general_category, GeneralCategory};

use crate::pretext_layout_locale::try_enumerate_locale_aware_word_segments;
use crate::pretext_layout_measurement::{
    ends_with_closing_quote, get_engine_profile, text_elements, EngineProfile,
};
use crate::{SegmentBreakKind, WhiteSpaceMode, WordBreakMode};

pub(crate) const FORWARD_STICKY_GLUE: &[char] = &['\'', '’'];
pub(crate) const KINSOKU_START_CHARS: &[char] = &[
    '\u{ff0c}', '\u{ff0e}', '\u{ff01}', '\u{ff1a}', '\u{ff1b}', '\u{ff1f}', '\u{3001}', '\u{3002}',
    '\u{30fb}', '\u{ff09}', '\u{3015}', '\u{3009}', '\u{300b}', '\u{300d}', '\u{300f}', '\u{3011}',
    '\u{3017}', '\u{3019}', '\u{301b}', '\u{30fc}', '\u{3005}', '\u{303b}', '\u{309d}', '\u{309e}',
    '\u{30fd}', '\u{30fe}',
];
pub(crate) const KINSOKU_END_CHARS: &[char] = &[
    '"', '(', '[', '{', '“', '‘', '«', '‹', '\u{ff08}', '\u{3014}', '\u{3008}', '\u{300a}',
    '\u{300c}', '\u{300e}', '\u{3010}', '\u{3016}', '\u{3018}', '\u{301a}',
];
pub(crate) const LEFT_STICKY_PUNCTUATION_CHARS: &[char] = &[
    '.', ',', '!', '?', ':', ';', '\u{060c}', '\u{061b}', '\u{061f}', '\u{0964}', '\u{0965}',
    '\u{104a}', '\u{104b}', '\u{104c}', '\u{104d}', '\u{104f}', ')', ']', '}', '%', '"', '”', '’',
    '»', '›', '…',
];
pub(crate) const CLOSING_QUOTES_CHARS: &[char] = &[
    '”', '’', '»', '›', '\u{300d}', '\u{300f}', '\u{3011}', '\u{300b}', '\u{3009}', '\u{3015}',
    '\u{ff09}',
];
const KEEP_ALL_GLUE_CHARS: &[char] = &['\u{00a0}', '\u{202f}', '\u{2060}', '\u{feff}'];
const NUMERIC_JOINERS: &[char] = &['-', ':', '/', '×', ',', '.', '+', '–', '—'];

#[derive(Clone, Debug)]
pub(crate) struct AnalysisToken {
    pub(crate) text: String,
    pub(crate) kind: SegmentBreakKind,
    pub(crate) is_word_like: bool,
}

impl AnalysisToken {
    fn new(text: impl Into<String>, kind: SegmentBreakKind, is_word_like: bool) -> Self {
        Self {
            text: text.into(),
            kind,
            is_word_like,
        }
    }
}

pub(crate) fn analyze_tokens(
    text: &str,
    white_space: WhiteSpaceMode,
    word_break: WordBreakMode,
    locale: Option<&str>,
) -> Vec<AnalysisToken> {
    let normalized = if white_space == WhiteSpaceMode::PreWrap {
        normalize_whitespace_pre_wrap(text)
    } else {
        normalize_whitespace_normal(text)
    };
    if normalized.is_empty() {
        return Vec::new();
    }
    let initial = build_initial_tokens(&normalized, white_space, locale);
    let merged = build_merged_tokens(initial, get_engine_profile(), white_space);
    if word_break == WordBreakMode::KeepAll {
        merge_keep_all_text_segments(merged)
    } else {
        merged
    }
}

fn normalize_whitespace_normal(text: &str) -> String {
    let mut normalized = String::with_capacity(text.len());
    let mut pending_space = false;
    for ch in text.chars() {
        if matches!(ch, ' ' | '\t' | '\n' | '\r' | '\u{000c}') {
            pending_space |= !normalized.is_empty();
            continue;
        }
        if pending_space {
            normalized.push(' ');
            pending_space = false;
        }
        normalized.push(ch);
    }
    normalized
}

fn normalize_whitespace_pre_wrap(text: &str) -> String {
    text.replace("\r\n", "\n").replace(['\r', '\u{000c}'], "\n")
}

fn build_initial_tokens(
    text: &str,
    white_space: WhiteSpaceMode,
    locale: Option<&str>,
) -> Vec<AnalysisToken> {
    if let Some(segments) = try_enumerate_locale_aware_word_segments(text, locale) {
        let mut tokens = Vec::new();
        for segment in segments {
            split_segment_by_break_kind(segment, white_space, &mut tokens);
        }
        return tokens;
    }
    build_fallback_tokens(text, white_space)
}

/// The source fallback when a dictionary segmenter is unavailable. Keeping this
/// path also makes malformed/unsupported locale hints recoverable.
fn build_fallback_tokens(text: &str, white_space: WhiteSpaceMode) -> Vec<AnalysisToken> {
    let mut tokens: Vec<AnalysisToken> = Vec::new();
    for element in text_elements(text) {
        let kind = classify_segment_break_text_element(element, white_space);
        let is_word_like = kind == SegmentBreakKind::Text && is_word_like_text(element);
        if let Some(previous) = tokens.last_mut() {
            if previous.kind == kind
                && previous.is_word_like == is_word_like
                && can_merge_adjacent_elements(kind)
            {
                previous.text.push_str(element);
                continue;
            }
        }
        tokens.push(AnalysisToken::new(element, kind, is_word_like));
    }
    tokens
}

fn split_segment_by_break_kind(
    segment: &str,
    white_space: WhiteSpaceMode,
    tokens: &mut Vec<AnalysisToken>,
) {
    let mut current: Option<AnalysisToken> = None;
    for element in text_elements(segment) {
        let kind = classify_segment_break_text_element(element, white_space);
        if let Some(previous) = current.as_mut() {
            if previous.kind == kind {
                previous.text.push_str(element);
                continue;
            }
        }
        if let Some(mut previous) = current.take() {
            previous.is_word_like =
                previous.kind == SegmentBreakKind::Text && is_word_like_text(&previous.text);
            tokens.push(previous);
        }
        current = Some(AnalysisToken::new(element, kind, false));
    }
    if let Some(mut previous) = current {
        previous.is_word_like =
            previous.kind == SegmentBreakKind::Text && is_word_like_text(&previous.text);
        tokens.push(previous);
    }
}

fn build_merged_tokens(
    initial: Vec<AnalysisToken>,
    profile: EngineProfile,
    white_space: WhiteSpaceMode,
) -> Vec<AnalysisToken> {
    let mut merged: Vec<AnalysisToken> = Vec::with_capacity(initial.len());
    for token in initial {
        if token.kind == SegmentBreakKind::Text {
            if let Some(previous) = merged
                .last_mut()
                .filter(|p| p.kind == SegmentBreakKind::Text)
            {
                let carries_closing_quote = profile.carry_cjk_after_closing_quote
                    && contains_cjk(&token.text)
                    && contains_cjk(&previous.text)
                    && ends_with_closing_quote(&previous.text);
                let carries_cjk_punctuation = is_cjk_line_start_prohibited_segment(&token.text)
                    && contains_cjk(&previous.text);
                let carries_myanmar = previous.text.ends_with('\u{104f}');
                let carries_arabic = token.is_word_like
                    && contains_arabic_script(&token.text)
                    && ends_with_arabic_no_space_punctuation(&previous.text);
                let repeated_punctuation = !token.is_word_like
                    && token.text.encode_utf16().count() == 1
                    && token.text != "-"
                    && token.text != "—"
                    && token.text.chars().next().is_some_and(|ch| {
                        !previous.text.is_empty() && previous.text.chars().all(|part| part == ch)
                    });
                let sticky_punctuation = !token.is_word_like
                    && (is_left_sticky_punctuation_segment(&token.text)
                        || (token.text == "-" && previous.is_word_like));
                if carries_closing_quote
                    || carries_cjk_punctuation
                    || carries_myanmar
                    || carries_arabic
                    || repeated_punctuation
                    || sticky_punctuation
                {
                    previous.text.push_str(&token.text);
                    previous.is_word_like |= token.is_word_like;
                    continue;
                }
            }
        }
        merged.push(token);
    }

    for index in 1..merged.len() {
        if merged[index].kind == SegmentBreakKind::Text
            && !merged[index].is_word_like
            && is_escaped_quote_cluster_segment(&merged[index].text)
            && merged[index - 1].kind == SegmentBreakKind::Text
        {
            let text = std::mem::take(&mut merged[index].text);
            let word_like = merged[index].is_word_like;
            merged[index - 1].text.push_str(&text);
            merged[index - 1].is_word_like |= word_like;
        }
    }
    for index in (0..merged.len().saturating_sub(1)).rev() {
        if merged[index].kind == SegmentBreakKind::Text
            && !merged[index].is_word_like
            && is_forward_sticky_cluster_segment(&merged[index].text)
        {
            let mut next = index + 1;
            while next < merged.len() && merged[next].text.is_empty() {
                next += 1;
            }
            if next < merged.len() && merged[next].kind == SegmentBreakKind::Text {
                let mut text = std::mem::take(&mut merged[index].text);
                text.push_str(&merged[next].text);
                merged[next].text = text;
                merged[next].is_word_like |= merged[index].is_word_like;
            }
        }
    }
    let mut merged = compact_tokens(merged);
    merged = merge_cjk_boundary_runs(merged);
    merged = merge_glue_connected_text_runs(merged);
    merged = merge_url_like_runs(merged);
    merged = merge_url_query_runs(merged);
    merged = merge_numeric_runs(merged);
    merged = split_hyphenated_numeric_runs(merged);
    merged = merge_ascii_punctuation_chains(merged);
    merged = carry_trailing_forward_sticky_across_cjk_boundary(merged);
    merged = carry_leading_cjk_start_prohibited_across_boundary(merged);
    merged = split_myanmar_runs(merged);

    for index in 0..merged.len().saturating_sub(1) {
        if let Some(marks) = split_leading_space_and_marks(&merged[index].text) {
            if merged[index + 1].kind == SegmentBreakKind::Text
                && contains_arabic_script(&merged[index + 1].text)
            {
                let kind = if merged[index].kind == SegmentBreakKind::PreservedSpace
                    || white_space == WhiteSpaceMode::PreWrap
                {
                    SegmentBreakKind::PreservedSpace
                } else {
                    SegmentBreakKind::Space
                };
                let mut text = marks.to_owned();
                text.push_str(&merged[index + 1].text);
                merged[index] = AnalysisToken::new(" ", kind, false);
                merged[index + 1].text = text;
            }
        }
    }
    compact_tokens(merged)
}

fn compact_tokens(mut tokens: Vec<AnalysisToken>) -> Vec<AnalysisToken> {
    tokens.retain(|token| !token.text.is_empty());
    tokens
}

fn merge_keep_all_text_segments(tokens: Vec<AnalysisToken>) -> Vec<AnalysisToken> {
    let mut merged: Vec<AnalysisToken> = Vec::with_capacity(tokens.len());
    for token in tokens {
        if token.kind == SegmentBreakKind::Text {
            if let Some(previous) = merged
                .last_mut()
                .filter(|p| p.kind == SegmentBreakKind::Text)
            {
                if contains_cjk(&previous.text) && can_continue_keep_all_text_run(&previous.text) {
                    previous.text.push_str(&token.text);
                    previous.is_word_like |= token.is_word_like;
                    continue;
                }
            }
        }
        merged.push(token);
    }
    merged
}

fn merge_glue_connected_text_runs(tokens: Vec<AnalysisToken>) -> Vec<AnalysisToken> {
    let mut merged = Vec::with_capacity(tokens.len());
    let mut tokens = tokens.into_iter().peekable();
    while let Some(mut token) = tokens.next() {
        if token.kind == SegmentBreakKind::Glue {
            while tokens
                .peek()
                .is_some_and(|next| next.kind == SegmentBreakKind::Glue)
            {
                token
                    .text
                    .push_str(&tokens.next().expect("peeked token").text);
            }
            if tokens
                .peek()
                .is_some_and(|next| next.kind == SegmentBreakKind::Text)
            {
                let next = tokens.next().expect("peeked token");
                token.text.push_str(&next.text);
                token.kind = SegmentBreakKind::Text;
                token.is_word_like = next.is_word_like;
            } else {
                merged.push(token);
                continue;
            }
        }
        if token.kind == SegmentBreakKind::Text {
            while tokens
                .peek()
                .is_some_and(|next| next.kind == SegmentBreakKind::Glue)
            {
                while tokens
                    .peek()
                    .is_some_and(|next| next.kind == SegmentBreakKind::Glue)
                {
                    token
                        .text
                        .push_str(&tokens.next().expect("peeked token").text);
                }
                if tokens
                    .peek()
                    .is_some_and(|next| next.kind == SegmentBreakKind::Text)
                {
                    let next = tokens.next().expect("peeked token");
                    token.text.push_str(&next.text);
                    token.is_word_like |= next.is_word_like;
                }
            }
        }
        merged.push(token);
    }
    merged
}

fn merge_cjk_boundary_runs(tokens: Vec<AnalysisToken>) -> Vec<AnalysisToken> {
    let mut merged: Vec<AnalysisToken> = Vec::with_capacity(tokens.len());
    for token in tokens {
        if token.kind == SegmentBreakKind::Text && is_cjk_line_start_prohibited_segment(&token.text)
        {
            if let Some(previous) = merged
                .last_mut()
                .filter(|p| p.kind == SegmentBreakKind::Text && contains_cjk(&p.text))
            {
                previous.text.push_str(&token.text);
                previous.is_word_like |= token.is_word_like;
                continue;
            }
        }
        merged.push(token);
    }
    merged
}

fn merge_url_like_runs(tokens: Vec<AnalysisToken>) -> Vec<AnalysisToken> {
    let mut merged = Vec::with_capacity(tokens.len());
    let mut index = 0;
    while index < tokens.len() {
        let token = &tokens[index];
        if token.kind == SegmentBreakKind::Text && is_url_like_run_start(&tokens, index) {
            let mut text = token.text.clone();
            let mut next = index + 1;
            while next < tokens.len() && !is_text_run_boundary(tokens[next].kind) {
                text.push_str(&tokens[next].text);
                let ends_query_prefix = tokens[next].text.contains('?');
                next += 1;
                if ends_query_prefix {
                    break;
                }
            }
            merged.push(AnalysisToken::new(text, SegmentBreakKind::Text, true));
            index = next;
        } else {
            merged.push(token.clone());
            index += 1;
        }
    }
    merged
}

fn merge_url_query_runs(tokens: Vec<AnalysisToken>) -> Vec<AnalysisToken> {
    let mut merged = Vec::with_capacity(tokens.len());
    let mut index = 0;
    while index < tokens.len() {
        let token = &tokens[index];
        merged.push(token.clone());
        index += 1;
        if !is_url_query_boundary_token(&token.text)
            || index >= tokens.len()
            || is_text_run_boundary(tokens[index].kind)
        {
            continue;
        }
        let mut query_text = String::new();
        while index < tokens.len() && !is_text_run_boundary(tokens[index].kind) {
            query_text.push_str(&tokens[index].text);
            index += 1;
        }
        if !query_text.is_empty() {
            merged.push(AnalysisToken::new(query_text, SegmentBreakKind::Text, true));
        }
    }
    merged
}

fn merge_numeric_runs(tokens: Vec<AnalysisToken>) -> Vec<AnalysisToken> {
    let mut merged = Vec::with_capacity(tokens.len());
    let mut tokens = tokens.into_iter().peekable();
    while let Some(mut token) = tokens.next() {
        if token.kind == SegmentBreakKind::Text
            && is_numeric_run_segment(&token.text)
            && segment_contains_decimal_digit(&token.text)
        {
            while tokens.peek().is_some_and(|next| {
                next.kind == SegmentBreakKind::Text && is_numeric_run_segment(&next.text)
            }) {
                token
                    .text
                    .push_str(&tokens.next().expect("peeked token").text);
            }
            token.is_word_like = true;
        }
        merged.push(token);
    }
    merged
}

fn split_hyphenated_numeric_runs(tokens: Vec<AnalysisToken>) -> Vec<AnalysisToken> {
    let mut split = Vec::with_capacity(tokens.len());
    for token in tokens {
        if token.kind == SegmentBreakKind::Text
            && token.text.contains('-')
            && token.text.split('-').all(|part| {
                !part.is_empty()
                    && segment_contains_decimal_digit(part)
                    && is_numeric_run_segment(part)
            })
        {
            for part in token.text.split_inclusive('-') {
                split.push(AnalysisToken::new(part, SegmentBreakKind::Text, true));
            }
        } else {
            split.push(token);
        }
    }
    split
}

fn merge_ascii_punctuation_chains(tokens: Vec<AnalysisToken>) -> Vec<AnalysisToken> {
    let mut merged = Vec::with_capacity(tokens.len());
    let mut tokens = tokens.into_iter().peekable();
    while let Some(mut token) = tokens.next() {
        if token.kind == SegmentBreakKind::Text
            && token.is_word_like
            && is_ascii_punctuation_chain_segment(&token.text)
        {
            while has_ascii_punctuation_chain_trailing_joiners(&token.text)
                && tokens.peek().is_some_and(|next| {
                    next.kind == SegmentBreakKind::Text
                        && next.is_word_like
                        && is_ascii_punctuation_chain_segment(&next.text)
                })
            {
                token
                    .text
                    .push_str(&tokens.next().expect("peeked token").text);
            }
        }
        merged.push(token);
    }
    merged
}

fn carry_trailing_forward_sticky_across_cjk_boundary(
    mut tokens: Vec<AnalysisToken>,
) -> Vec<AnalysisToken> {
    for index in 0..tokens.len().saturating_sub(1) {
        if tokens[index].kind != SegmentBreakKind::Text
            || tokens[index + 1].kind != SegmentBreakKind::Text
            || !contains_cjk(&tokens[index].text)
            || !contains_cjk(&tokens[index + 1].text)
        {
            continue;
        }
        if let Some(split) = split_trailing_forward_sticky_cluster(&tokens[index].text) {
            let mut tail = tokens[index].text.split_off(split);
            tail.push_str(&tokens[index + 1].text);
            tokens[index + 1].text = tail;
        }
    }
    compact_tokens(tokens)
}

fn carry_leading_cjk_start_prohibited_across_boundary(
    mut tokens: Vec<AnalysisToken>,
) -> Vec<AnalysisToken> {
    for index in 1..tokens.len() {
        if tokens[index - 1].kind != SegmentBreakKind::Text
            || tokens[index].kind != SegmentBreakKind::Text
            || !contains_cjk(&tokens[index - 1].text)
        {
            continue;
        }
        if let Some(split) = split_leading_cjk_start_prohibited_prefix(&tokens[index].text) {
            let tail = tokens[index].text.split_off(split);
            let prefix = std::mem::replace(&mut tokens[index].text, tail);
            tokens[index - 1].text.push_str(&prefix);
        }
    }
    compact_tokens(tokens)
}

fn split_myanmar_runs(tokens: Vec<AnalysisToken>) -> Vec<AnalysisToken> {
    let mut result = Vec::with_capacity(tokens.len());
    for token in tokens {
        if token.kind == SegmentBreakKind::Text && contains_myanmar_script(&token.text) {
            if let Some(index) = try_split_myanmar_run(&token.text) {
                result.push(AnalysisToken::new(
                    &token.text[..index],
                    SegmentBreakKind::Text,
                    true,
                ));
                result.push(AnalysisToken::new(
                    &token.text[index..],
                    SegmentBreakKind::Text,
                    true,
                ));
                continue;
            }
        }
        result.push(token);
    }
    result
}

fn classify_segment_break_text_element(
    element: &str,
    white_space: WhiteSpaceMode,
) -> SegmentBreakKind {
    use SegmentBreakKind::*;
    match element {
        " " if white_space == WhiteSpaceMode::PreWrap => PreservedSpace,
        "\t" if white_space == WhiteSpaceMode::PreWrap => Tab,
        "\n" if white_space == WhiteSpaceMode::PreWrap => HardBreak,
        " " => Space,
        "\u{00a0}" | "\u{202f}" | "\u{2060}" | "\u{feff}" => Glue,
        "\u{200b}" => ZeroWidthBreak,
        "\u{00ad}" => SoftHyphen,
        _ => Text,
    }
}

pub(crate) fn is_word_like_text(text: &str) -> bool {
    !text.is_empty() && text.chars().all(is_word_like_code_point)
}

fn is_text_run_boundary(kind: SegmentBreakKind) -> bool {
    matches!(
        kind,
        SegmentBreakKind::Space
            | SegmentBreakKind::PreservedSpace
            | SegmentBreakKind::ZeroWidthBreak
            | SegmentBreakKind::HardBreak
    )
}

fn can_merge_adjacent_elements(kind: SegmentBreakKind) -> bool {
    !matches!(
        kind,
        SegmentBreakKind::Tab
            | SegmentBreakKind::HardBreak
            | SegmentBreakKind::ZeroWidthBreak
            | SegmentBreakKind::SoftHyphen
    )
}

fn is_url_like_run_start(tokens: &[AnalysisToken], index: usize) -> bool {
    let text = &tokens[index].text;
    if text
        .get(..4)
        .is_some_and(|start| start.eq_ignore_ascii_case("www."))
    {
        return true;
    }
    let Some(next) = tokens
        .get(index + 1)
        .filter(|next| next.kind == SegmentBreakKind::Text)
    else {
        return false;
    };
    if let Some(scheme) = text.strip_suffix(':') {
        if is_url_scheme(scheme) && next.text == "//" {
            return true;
        }
    }
    is_url_scheme(text) && next.text.starts_with("://")
}

fn is_url_scheme(text: &str) -> bool {
    let mut chars = text.chars();
    chars.next().is_some_and(|ch| ch.is_ascii_alphabetic())
        && chars.all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '+' | '.' | '-'))
}

fn is_url_query_boundary_token(text: &str) -> bool {
    text.contains('?')
        && (text.contains("://")
            || text
                .get(..4)
                .is_some_and(|start| start.eq_ignore_ascii_case("www.")))
}

pub(crate) fn segment_contains_decimal_digit(text: &str) -> bool {
    text.chars()
        .any(|ch| get_general_category(ch) == GeneralCategory::DecimalNumber)
}

fn is_numeric_run_segment(text: &str) -> bool {
    !text.is_empty()
        && text.chars().all(|ch| {
            get_general_category(ch) == GeneralCategory::DecimalNumber
                || NUMERIC_JOINERS.contains(&ch)
        })
}

fn is_ascii_punctuation_chain_segment(text: &str) -> bool {
    !text.is_empty()
        && text
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '_' | ',' | ':' | ';'))
}

fn has_ascii_punctuation_chain_trailing_joiners(text: &str) -> bool {
    text.ends_with([',', ':', ';'])
}

fn contains_arabic_script(text: &str) -> bool {
    text.chars().any(|ch| {
        matches!(ch as u32,
        0x0600..=0x06ff | 0x0750..=0x077f | 0x08a0..=0x08ff | 0xfb50..=0xfdff |
        0xfe70..=0xfeff | 0x10e60..=0x10e7f | 0x1ee00..=0x1eeff)
    })
}

fn contains_myanmar_script(text: &str) -> bool {
    text.chars()
        .any(|ch| matches!(ch as u32, 0x1000..=0x109f | 0xa9e0..=0xa9ff | 0xaa60..=0xaa7f))
}

fn is_combining_mark(ch: char) -> bool {
    matches!(
        get_general_category(ch),
        GeneralCategory::NonspacingMark
            | GeneralCategory::SpacingMark
            | GeneralCategory::EnclosingMark
    )
}

fn is_escaped_quote_cluster_segment(segment: &str) -> bool {
    let mut saw_quote = false;
    for ch in segment.chars() {
        if ch == '\\' || is_combining_mark(ch) {
            continue;
        }
        if KINSOKU_END_CHARS.contains(&ch)
            || LEFT_STICKY_PUNCTUATION_CHARS.contains(&ch)
            || FORWARD_STICKY_GLUE.contains(&ch)
        {
            saw_quote = true;
        } else {
            return false;
        }
    }
    saw_quote
}

fn is_left_sticky_punctuation_segment(segment: &str) -> bool {
    if is_escaped_quote_cluster_segment(segment) {
        return true;
    }
    let mut saw_punctuation = false;
    for ch in segment.chars() {
        if LEFT_STICKY_PUNCTUATION_CHARS.contains(&ch) {
            saw_punctuation = true;
        } else if !(saw_punctuation && is_combining_mark(ch)) {
            return false;
        }
    }
    saw_punctuation
}

fn is_cjk_line_start_prohibited_segment(segment: &str) -> bool {
    !segment.is_empty()
        && segment.chars().all(|ch| {
            KINSOKU_START_CHARS.contains(&ch) || LEFT_STICKY_PUNCTUATION_CHARS.contains(&ch)
        })
}

pub(crate) fn can_continue_keep_all_text_run(previous_text: &str) -> bool {
    let Some(last) = previous_text.chars().next_back() else {
        return false;
    };
    if KEEP_ALL_GLUE_CHARS.contains(&last) {
        return false;
    }
    !(KINSOKU_START_CHARS.contains(&last) || LEFT_STICKY_PUNCTUATION_CHARS.contains(&last))
        || ends_with_mixed_script_keep_all_continuation(previous_text)
}

fn ends_with_mixed_script_keep_all_continuation(text: &str) -> bool {
    if !text.ends_with(['.', ',', ':', ';', '!', '?', ')', ']', '}', '%', '"', '\'']) {
        return false;
    }
    let mut saw_cjk = false;
    let mut saw_non_cjk_word = false;
    for ch in text.chars() {
        if is_cjk_code_point(ch as u32) {
            saw_cjk = true;
        } else if is_word_like_code_point(ch) {
            saw_non_cjk_word = true;
        }
        if saw_cjk && saw_non_cjk_word {
            return true;
        }
    }
    false
}

fn is_word_like_code_point(ch: char) -> bool {
    matches!(
        get_general_category(ch),
        GeneralCategory::UppercaseLetter
            | GeneralCategory::LowercaseLetter
            | GeneralCategory::TitlecaseLetter
            | GeneralCategory::ModifierLetter
            | GeneralCategory::OtherLetter
            | GeneralCategory::DecimalNumber
            | GeneralCategory::LetterNumber
            | GeneralCategory::OtherNumber
            | GeneralCategory::NonspacingMark
            | GeneralCategory::SpacingMark
            | GeneralCategory::EnclosingMark
            | GeneralCategory::ConnectorPunctuation
    ) || ch == '_'
}

pub(crate) fn contains_cjk(text: &str) -> bool {
    text.chars().any(|ch| is_cjk_code_point(ch as u32))
}

pub(crate) fn is_cjk_code_point(code: u32) -> bool {
    matches!(code,
        0x4e00..=0x9fff | 0x3400..=0x4dbf | 0x20000..=0x2a6df |
        0x2a700..=0x2b73f | 0x2b740..=0x2b81f | 0x2b820..=0x2ceaf |
        0x2ceb0..=0x2ebef | 0x2ebf0..=0x2ee5d | 0x30000..=0x3134f |
        0x31350..=0x323af | 0x323b0..=0x33479 | 0xf900..=0xfaff |
        0x2f800..=0x2fa1f | 0x3000..=0x303f | 0x3040..=0x309f |
        0x30a0..=0x30ff | 0xac00..=0xd7af | 0xff00..=0xffef)
}

fn is_forward_sticky_cluster_segment(segment: &str) -> bool {
    is_escaped_quote_cluster_segment(segment)
        || (!segment.is_empty()
            && segment.chars().all(|ch| {
                KINSOKU_END_CHARS.contains(&ch)
                    || FORWARD_STICKY_GLUE.contains(&ch)
                    || is_combining_mark(ch)
            }))
}

fn ends_with_arabic_no_space_punctuation(segment: &str) -> bool {
    contains_arabic_script(segment) && segment.ends_with([':', '.', '\u{060c}', '\u{061b}'])
}

fn split_leading_space_and_marks(segment: &str) -> Option<&str> {
    let marks = segment.strip_prefix(' ')?;
    (!marks.is_empty() && marks.chars().all(is_combining_mark)).then_some(marks)
}

fn split_trailing_forward_sticky_cluster(text: &str) -> Option<usize> {
    let mut split = text.len();
    for (index, ch) in text.char_indices().rev() {
        if is_combining_mark(ch)
            || KINSOKU_END_CHARS.contains(&ch)
            || FORWARD_STICKY_GLUE.contains(&ch)
        {
            split = index;
        } else {
            break;
        }
    }
    (split > 0 && split < text.len()).then_some(split)
}

fn split_leading_cjk_start_prohibited_prefix(text: &str) -> Option<usize> {
    let mut split = 0;
    for (index, ch) in text.char_indices() {
        if KINSOKU_START_CHARS.contains(&ch) || LEFT_STICKY_PUNCTUATION_CHARS.contains(&ch) {
            split = index + ch.len_utf8();
        } else {
            break;
        }
    }
    (split > 0).then_some(split)
}

fn try_split_myanmar_run(text: &str) -> Option<usize> {
    for suffix in ["ဖြင့်", "ကြ၏။"] {
        if text.len() > suffix.len() && text.ends_with(suffix) {
            return Some(text.len() - suffix.len());
        }
    }
    text.find("ချီ၍").filter(|&index| index > 0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fallback_keeps_explicit_breaks_independent() {
        let tokens = build_fallback_tokens("a\t\t\n\n\u{200b}\u{00ad}b", WhiteSpaceMode::PreWrap);
        assert_eq!(
            tokens
                .iter()
                .map(|token| token.text.as_str())
                .collect::<Vec<_>>(),
            ["a", "\t", "\t", "\n", "\n", "\u{200b}", "\u{00ad}", "b"]
        );
    }
}
