use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use icu_properties::{props::IndicConjunctBreak, CodePointMapData};
use unicode_segmentation::UnicodeSegmentation;

use crate::pretext_layout_analysis::{
    can_continue_keep_all_text_run, contains_cjk, CLOSING_QUOTES_CHARS, KINSOKU_END_CHARS,
    KINSOKU_START_CHARS, LEFT_STICKY_PUNCTUATION_CHARS,
};
use crate::{IPretextTextMeasurer, PretextError, SegmentBreakKind};

#[derive(Clone, Copy, Debug)]
pub(crate) struct EngineProfile {
    pub(crate) line_fit_epsilon: f64,
    pub(crate) carry_cjk_after_closing_quote: bool,
    pub(crate) prefer_prefix_widths_for_breakable_runs: bool,
    pub(crate) prefer_early_soft_hyphen_break: bool,
}

pub(crate) const fn get_engine_profile() -> EngineProfile {
    EngineProfile {
        line_fit_epsilon: 0.005,
        carry_cjk_after_closing_quote: true,
        prefer_prefix_widths_for_breakable_runs: false,
        prefer_early_soft_hyphen_break: false,
    }
}

#[derive(Clone, Debug)]
pub(crate) struct MeasuredSegment {
    pub(crate) text: String,
    pub(crate) kind: SegmentBreakKind,
    pub(crate) is_breakable_run: bool,
    pub(crate) width: f64,
    pub(crate) breakable_widths: Option<Vec<f64>>,
    pub(crate) breakable_prefix_widths: Option<Vec<f64>>,
}

pub(crate) fn build_base_cjk_units(text: &str, carry_cjk_after_closing_quote: bool) -> Vec<String> {
    let mut elements = text_elements(text);
    let Some(first) = elements.next() else {
        return Vec::new();
    };
    let mut units = Vec::new();
    let mut current = first.to_owned();
    let mut current_contains_cjk = contains_cjk(first);
    let mut current_ends_with_closing_quote = ends_with_closing_quote(first);
    let mut current_is_single_kinsoku_end = is_single_kinsoku_end(first);

    for grapheme in elements {
        let grapheme_contains_cjk = contains_cjk(grapheme);
        if current_is_single_kinsoku_end
            || is_cjk_line_start_prohibited(grapheme)
            || is_left_sticky_cluster(grapheme)
            || (carry_cjk_after_closing_quote
                && grapheme_contains_cjk
                && current_ends_with_closing_quote)
        {
            current.push_str(grapheme);
            current_contains_cjk |= grapheme_contains_cjk;
            current_ends_with_closing_quote |= ends_with_closing_quote(grapheme);
            current_is_single_kinsoku_end = false;
            continue;
        }
        if !current_contains_cjk && !grapheme_contains_cjk {
            current.push_str(grapheme);
            current_ends_with_closing_quote = ends_with_closing_quote(grapheme);
            current_is_single_kinsoku_end = false;
            continue;
        }
        units.push(current);
        current = grapheme.to_owned();
        current_contains_cjk = grapheme_contains_cjk;
        current_ends_with_closing_quote = ends_with_closing_quote(grapheme);
        current_is_single_kinsoku_end = is_single_kinsoku_end(grapheme);
    }
    units.push(current);
    units
}

fn is_single_kinsoku_end(text: &str) -> bool {
    let mut chars = text.chars();
    chars
        .next()
        .is_some_and(|ch| KINSOKU_END_CHARS.contains(&ch))
        && chars.next().is_none()
}

pub(crate) fn merge_keep_all_text_units(units: Vec<String>) -> Vec<String> {
    let mut merged: Vec<String> = Vec::with_capacity(units.len());
    for unit in units {
        if let Some(current) = merged.last_mut() {
            if contains_cjk(current) && can_continue_keep_all_text_run(current) {
                current.push_str(&unit);
                continue;
            }
        }
        merged.push(unit);
    }
    merged
}

fn is_left_sticky_cluster(text: &str) -> bool {
    !text.is_empty()
        && text
            .chars()
            .all(|ch| LEFT_STICKY_PUNCTUATION_CHARS.contains(&ch))
}

fn is_cjk_line_start_prohibited(text: &str) -> bool {
    !text.is_empty()
        && text.chars().all(|ch| {
            KINSOKU_START_CHARS.contains(&ch) || LEFT_STICKY_PUNCTUATION_CHARS.contains(&ch)
        })
}

pub(crate) fn ends_with_closing_quote(text: &str) -> bool {
    for ch in text.chars().rev() {
        if CLOSING_QUOTES_CHARS.contains(&ch) {
            return true;
        }
        if !LEFT_STICKY_PUNCTUATION_CHARS.contains(&ch) {
            return false;
        }
    }
    false
}

pub(crate) fn get_text_elements(text: &str) -> Vec<&str> {
    text_elements(text).collect()
}

/// The source runtime's text-element boundaries predate UAX #29 rule GB9c.
/// Preserve those cursor boundaries while retaining the current Unicode data
/// for all other rules (including emoji ZWJ sequences and combining marks).
///
/// Offsets include both zero and the end of the string, including for an empty
/// string. They are UTF-8 offsets; public shaping clusters remain UTF-16 based.
pub(crate) fn text_element_offsets(text: &str) -> impl Iterator<Item = usize> + '_ {
    let conjunct_break = CodePointMapData::<IndicConjunctBreak>::new();
    text.grapheme_indices(true)
        .flat_map(move |(base, cluster)| {
            let mut consonant = false;
            let mut linker = false;
            cluster.char_indices().filter_map(move |(offset, ch)| {
                let property = conjunct_break.get(ch);
                let boundary = offset == 0
                    || (consonant && linker && property == IndicConjunctBreak::Consonant);
                match property {
                    IndicConjunctBreak::Consonant => {
                        consonant = true;
                        linker = false;
                    }
                    IndicConjunctBreak::Linker => linker |= consonant,
                    IndicConjunctBreak::Extend => {}
                    _ => {
                        consonant = false;
                        linker = false;
                    }
                }
                boundary.then_some(base + offset)
            })
        })
        .chain(std::iter::once(text.len()))
}

pub(crate) fn text_elements(text: &str) -> impl Iterator<Item = &str> {
    text_element_offsets(text).skip(1).scan(0, |start, end| {
        let element = &text[*start..end];
        *start = end;
        Some(element)
    })
}

type SegmentCache = HashMap<(SegmentBreakKind, bool), HashMap<String, MeasuredSegment>>;

pub(crate) struct FontState {
    text_measurer: Rc<dyn IPretextTextMeasurer>,
    pub(crate) hyphen_width: f64,
    pub(crate) tab_stop_advance: f64,
    segment_cache: RefCell<SegmentCache>,
}

impl FontState {
    pub(crate) fn new(text_measurer: Rc<dyn IPretextTextMeasurer>) -> Result<Self, PretextError> {
        let space_width = validate_measurement(text_measurer.measure_text(" "))?;
        let hyphen_width = validate_measurement(text_measurer.measure_text("-"))?;
        let tab_stop_advance = space_width * 8.0;
        if !tab_stop_advance.is_finite() {
            return Err(PretextError::Backend(
                "text measurement produced an overflowing tab stop".into(),
            ));
        }
        Ok(Self {
            text_measurer,
            hyphen_width,
            tab_stop_advance,
            segment_cache: RefCell::new(HashMap::new()),
        })
    }

    pub(crate) fn measure_segment(
        &self,
        text: &str,
        kind: SegmentBreakKind,
        is_breakable_run: bool,
    ) -> Result<MeasuredSegment, PretextError> {
        let cached = self
            .segment_cache
            .borrow()
            .get(&(kind, is_breakable_run))
            .and_then(|cache| cache.get(text))
            .cloned();
        if let Some(cached) = cached {
            return Ok(cached);
        }
        // Never retain a RefCell borrow while invoking user-provided measurers.
        let mut grapheme_widths = None;
        let mut prefix_widths = None;
        if is_breakable_run {
            let graphemes = get_text_elements(text);
            if graphemes.len() > 1 {
                let mut widths = Vec::with_capacity(graphemes.len());
                let mut prefixes = Vec::with_capacity(graphemes.len());
                let mut prefix_length = 0;
                let mut previous = 0.0;
                for grapheme in graphemes {
                    prefix_length += grapheme.len();
                    let width = self.measure_text(&text[..prefix_length])?;
                    prefixes.push(width);
                    widths.push(width - previous);
                    previous = width;
                }
                grapheme_widths = Some(widths);
                prefix_widths = Some(prefixes);
            }
        }
        let width = self.measure_text(text)?;
        let segment = MeasuredSegment {
            text: text.to_owned(),
            kind,
            is_breakable_run,
            width,
            breakable_widths: grapheme_widths,
            breakable_prefix_widths: prefix_widths,
        };
        self.segment_cache
            .borrow_mut()
            .entry((kind, is_breakable_run))
            .or_default()
            .insert(text.to_owned(), segment.clone());
        Ok(segment)
    }

    fn measure_text(&self, text: &str) -> Result<f64, PretextError> {
        if text.is_empty() {
            Ok(0.0)
        } else {
            validate_measurement(self.text_measurer.measure_text(text))
        }
    }
}

fn validate_measurement(width: f64) -> Result<f64, PretextError> {
    if width.is_finite() && width >= 0.0 {
        Ok(width)
    } else {
        Err(PretextError::Backend(
            "text measurement must return a finite, non-negative advance".into(),
        ))
    }
}
