use std::cell::OnceCell;
use std::rc::Rc;

use crate::pretext_layout_analysis::{analyze_tokens, contains_cjk, AnalysisToken};
use crate::pretext_layout_bidi::compute_segment_levels;
use crate::pretext_layout_measurement::{
    build_base_cjk_units, get_engine_profile, merge_keep_all_text_units, EngineProfile, FontState,
    MeasuredSegment,
};
use crate::{
    PrepareOptions, PrepareProfile, PreparedLineChunk, PreparedText, PreparedTextWithSegments,
    PretextError, PretextLayout, SegmentBreakKind, WordBreakMode,
};

struct PreparedContent {
    base: PreparedText,
    segments: Vec<String>,
    segment_levels: Option<Vec<i8>>,
}

impl PretextLayout {
    /// Measures normalized text once for repeated allocation-free layout.
    pub fn prepare(
        &self,
        text: &str,
        font: &str,
        options: PrepareOptions,
    ) -> Result<PreparedText, PretextError> {
        Ok(self.prepare_core(text, font, options, false)?.base)
    }

    /// Retains normalized source segments in addition to reusable metrics.
    pub fn prepare_with_segments(
        &self,
        text: &str,
        font: &str,
        options: PrepareOptions,
    ) -> Result<Rc<PreparedTextWithSegments>, PretextError> {
        let content = self.prepare_core(text, font, options, true)?;
        let grapheme_offsets = (0..content.segments.len())
            .map(|_| OnceCell::new())
            .collect();
        Ok(Rc::new(PreparedTextWithSegments {
            base: content.base,
            segments: content.segments,
            segment_levels: content.segment_levels,
            grapheme_offsets,
        }))
    }

    /// Profiles analysis and measurement using a caller-supplied monotonic clock
    /// measured in milliseconds. This works with native clocks and browser
    /// `performance.now()` without introducing a platform dependency.
    pub fn profile_prepare(
        &self,
        text: &str,
        font: &str,
        options: PrepareOptions,
        mut now_ms: impl FnMut() -> f64,
    ) -> Result<PrepareProfile, PretextError> {
        let start = now_ms();
        let locale = self.locale();
        let tokens = analyze_tokens(
            text,
            options.white_space,
            options.word_break,
            locale.as_deref(),
        );
        let analysis_end = now_ms();
        let state = self.get_font_state(font)?;
        let mut prepared_segments = 0;
        let mut breakable_segments = 0;
        for token in &tokens {
            visit_prepared_segments(
                token,
                &state,
                get_engine_profile(),
                options.word_break,
                |segment| {
                    prepared_segments += 1;
                    if segment.is_breakable_run
                        && segment
                            .breakable_widths
                            .as_ref()
                            .is_some_and(|widths| widths.len() > 1)
                    {
                        breakable_segments += 1;
                    }
                },
            )?;
        }
        let end = now_ms();
        if !start.is_finite()
            || !analysis_end.is_finite()
            || !end.is_finite()
            || analysis_end < start
            || end < analysis_end
        {
            return Err(PretextError::InvalidInput(
                "profiling clock must return finite, monotonic milliseconds".into(),
            ));
        }
        Ok(PrepareProfile {
            analysis_ms: analysis_end - start,
            measure_ms: end - analysis_end,
            total_ms: end - start,
            analysis_segments: tokens.len(),
            prepared_segments,
            breakable_segments,
        })
    }

    fn prepare_core(
        &self,
        text: &str,
        font: &str,
        options: PrepareOptions,
        include_segments: bool,
    ) -> Result<PreparedContent, PretextError> {
        let state = self.get_font_state(font)?;
        let locale = self.locale();
        let tokens = analyze_tokens(
            text,
            options.white_space,
            options.word_break,
            locale.as_deref(),
        );
        let capacity = tokens.len();
        let mut base = PreparedText {
            font: font.to_owned(),
            white_space: options.white_space,
            widths: Vec::with_capacity(capacity),
            line_end_fit_advances: Vec::with_capacity(capacity),
            line_end_paint_advances: Vec::with_capacity(capacity),
            kinds: Vec::with_capacity(capacity),
            breakable_widths: Vec::with_capacity(capacity),
            breakable_prefix_widths: Vec::with_capacity(capacity),
            discretionary_hyphen_width: state.hyphen_width,
            tab_stop_advance: state.tab_stop_advance,
            chunks: Vec::new(),
            simple_line_walk_fast_path: true,
        };
        let mut segments = if include_segments {
            Vec::with_capacity(capacity)
        } else {
            Vec::new()
        };
        let mut normalized = String::new();
        let mut starts = Vec::new();
        let mut utf16_length = 0;
        for token in &tokens {
            visit_prepared_segments(
                token,
                &state,
                get_engine_profile(),
                options.word_break,
                |segment| {
                    base.simple_line_walk_fast_path &= matches!(
                        segment.kind,
                        SegmentBreakKind::Text
                            | SegmentBreakKind::Space
                            | SegmentBreakKind::ZeroWidthBreak
                    );
                    base.widths.push(segment.width);
                    base.line_end_fit_advances
                        .push(get_line_end_fit_advance(&segment, state.hyphen_width));
                    base.line_end_paint_advances
                        .push(get_line_end_paint_advance(&segment, state.hyphen_width));
                    base.kinds.push(segment.kind);
                    base.breakable_widths.push(segment.breakable_widths);
                    base.breakable_prefix_widths
                        .push(segment.breakable_prefix_widths);
                    if include_segments {
                        starts.push(utf16_length);
                        utf16_length += segment.text.encode_utf16().count();
                        normalized.push_str(&segment.text);
                        segments.push(segment.text);
                    }
                },
            )?;
        }
        base.chunks = build_chunks(&base.kinds);
        base.simple_line_walk_fast_path &= base.chunks.len() <= 1;
        let segment_levels = if include_segments {
            compute_segment_levels(&normalized, &starts)
        } else {
            None
        };
        Ok(PreparedContent {
            base,
            segments,
            segment_levels,
        })
    }

    pub(crate) fn get_font_state(&self, font: &str) -> Result<Rc<FontState>, PretextError> {
        let cached = self.font_states.borrow().get(font).cloned();
        if let Some(cached) = cached {
            return Ok(cached);
        }
        let generation = self.cache_generation.get();
        let factory = self.text_measurer_factory();
        if !factory.is_supported() {
            return Err(PretextError::Backend(format!(
                "text measurement backend '{}' is unsupported",
                factory.name()
            )));
        }
        let state = Rc::new(FontState::new(factory.create(font)?)?);
        // Backend callbacks can replace the configured factory or clear caches.
        // The current preparation owns its snapshot, but must not repopulate a
        // newer generation with a stale backend.
        if self.cache_generation.get() == generation {
            self.font_states
                .borrow_mut()
                .insert(font.to_owned(), state.clone());
        }
        Ok(state)
    }
}

fn visit_prepared_segments(
    token: &AnalysisToken,
    state: &FontState,
    profile: EngineProfile,
    word_break: WordBreakMode,
    mut on_segment: impl FnMut(MeasuredSegment),
) -> Result<(), PretextError> {
    if matches!(
        token.kind,
        SegmentBreakKind::HardBreak
            | SegmentBreakKind::ZeroWidthBreak
            | SegmentBreakKind::SoftHyphen
            | SegmentBreakKind::Tab
    ) {
        on_segment(MeasuredSegment {
            text: token.text.clone(),
            kind: token.kind,
            is_breakable_run: false,
            width: 0.0,
            breakable_widths: None,
            breakable_prefix_widths: None,
        });
        return Ok(());
    }
    if token.kind == SegmentBreakKind::Text && contains_cjk(&token.text) {
        let units = build_base_cjk_units(&token.text, profile.carry_cjk_after_closing_quote);
        let units = if word_break == WordBreakMode::KeepAll {
            merge_keep_all_text_units(units)
        } else {
            units
        };
        for unit in units {
            let is_breakable_run = word_break == WordBreakMode::KeepAll || !contains_cjk(&unit);
            on_segment(state.measure_segment(&unit, token.kind, is_breakable_run)?);
        }
    } else {
        let is_breakable_run = token.kind == SegmentBreakKind::Text
            && token.is_word_like
            && token.text.encode_utf16().count() > 1;
        on_segment(state.measure_segment(&token.text, token.kind, is_breakable_run)?);
    }
    Ok(())
}

fn get_line_end_fit_advance(segment: &MeasuredSegment, hyphen_width: f64) -> f64 {
    match segment.kind {
        SegmentBreakKind::Space
        | SegmentBreakKind::PreservedSpace
        | SegmentBreakKind::ZeroWidthBreak
        | SegmentBreakKind::Tab
        | SegmentBreakKind::HardBreak => 0.0,
        SegmentBreakKind::SoftHyphen => hyphen_width,
        _ => segment.width,
    }
}

fn get_line_end_paint_advance(segment: &MeasuredSegment, hyphen_width: f64) -> f64 {
    match segment.kind {
        SegmentBreakKind::Space
        | SegmentBreakKind::ZeroWidthBreak
        | SegmentBreakKind::Tab
        | SegmentBreakKind::HardBreak => 0.0,
        SegmentBreakKind::SoftHyphen => hyphen_width,
        _ => segment.width,
    }
}

fn build_chunks(kinds: &[SegmentBreakKind]) -> Vec<PreparedLineChunk> {
    let mut chunks = Vec::new();
    let mut start = 0;
    for (index, &kind) in kinds.iter().enumerate() {
        if kind == SegmentBreakKind::HardBreak {
            chunks.push(PreparedLineChunk {
                start_segment_index: start,
                end_segment_index: index,
                consumed_end_segment_index: index + 1,
            });
            start = index + 1;
        }
    }
    if start < kinds.len() {
        chunks.push(PreparedLineChunk {
            start_segment_index: start,
            end_segment_index: kinds.len(),
            consumed_end_segment_index: kinds.len(),
        });
    }
    chunks
}
