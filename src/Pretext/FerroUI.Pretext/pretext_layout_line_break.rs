//! Prepared line walking and incremental wrapping.

use crate::pretext_layout_line_text::materialize_line;
use crate::pretext_layout_measurement::{get_engine_profile, EngineProfile};
use crate::{
    LayoutCursor, LayoutLine, LayoutLineRange, LayoutLinesResult, LayoutResult, LineStats,
    PreparedText, PreparedTextWithSegments, PretextLayout, SegmentBreakKind,
};

#[derive(Clone, Copy, Debug)]
pub(crate) struct InternalLine {
    pub start: LayoutCursor,
    pub visible_end: LayoutCursor,
    pub end: LayoutCursor,
    pub width: f64,
    pub append_hyphen: bool,
}

#[derive(Clone, Copy)]
struct PendingBreak {
    end_segment_index: usize,
    fit_width: f64,
    paint_width: f64,
    kind: SegmentBreakKind,
}

/// The state shared by the batched and incremental walkers. All state is local
/// to a walk; a cursor can resume a prepared paragraph at a different width.
struct LineState {
    start: LayoutCursor,
    end: LayoutCursor,
    width: f64,
    has_content: bool,
    pending: Option<PendingBreak>,
}

impl LineState {
    fn new(start: LayoutCursor) -> Self {
        Self {
            start,
            end: start,
            width: 0.0,
            has_content: false,
            pending: None,
        }
    }

    fn start_segment(&mut self, segment_index: usize, width: f64, reset_start: bool) {
        if reset_start {
            self.start = cursor(segment_index, 0);
        }
        self.end = cursor(segment_index + 1, 0);
        self.width = width;
        self.has_content = true;
    }

    fn start_grapheme(
        &mut self,
        segment_index: usize,
        grapheme_index: usize,
        width: f64,
        reset_start: bool,
    ) {
        if reset_start {
            self.start = cursor(segment_index, grapheme_index);
        }
        self.end = cursor(segment_index, grapheme_index + 1);
        self.width = width;
        self.has_content = true;
    }

    fn append_whole(&mut self, segment_index: usize, width: f64, reset_start: bool) {
        if self.has_content {
            self.width += width;
            self.end = cursor(segment_index + 1, 0);
        } else {
            self.start_segment(segment_index, width, reset_start);
        }
    }

    fn update_pending(
        &mut self,
        prepared: &PreparedText,
        segment_index: usize,
        width: f64,
        simple: bool,
    ) {
        let kind = prepared.kinds[segment_index];
        if !can_break_after(kind) {
            return;
        }
        let (fit, paint) = if simple {
            (0.0, 0.0)
        } else if kind == SegmentBreakKind::Tab {
            (0.0, width)
        } else {
            (
                prepared.line_end_fit_advances[segment_index],
                prepared.line_end_paint_advances[segment_index],
            )
        };
        self.pending = Some(PendingBreak {
            end_segment_index: segment_index + 1,
            fit_width: self.width - width + fit,
            paint_width: self.width - width + paint,
            kind,
        });
    }

    fn finish(
        &self,
        end: Option<LayoutCursor>,
        width: Option<f64>,
        append_hyphen: bool,
    ) -> Option<InternalLine> {
        self.has_content.then(|| {
            let end = end.unwrap_or(self.end);
            InternalLine {
                start: self.start,
                visible_end: end,
                end,
                width: width.unwrap_or(self.width),
                append_hyphen,
            }
        })
    }

    fn emit(
        &mut self,
        end: Option<LayoutCursor>,
        width: Option<f64>,
        append_hyphen: bool,
        on_line: &mut impl FnMut(InternalLine),
        count: &mut usize,
    ) {
        if let Some(line) = self.finish(end, width, append_hyphen) {
            *count += 1;
            on_line(line);
        }
        self.width = 0.0;
        self.has_content = false;
        self.pending = None;
    }

    fn is_after_pending(&self, pending: PendingBreak) -> bool {
        self.end.segment_index > pending.end_segment_index
            || (self.end.segment_index == pending.end_segment_index && self.end.grapheme_index > 0)
    }
}

const fn cursor(segment_index: usize, grapheme_index: usize) -> LayoutCursor {
    LayoutCursor {
        segment_index,
        grapheme_index,
    }
}

impl PretextLayout {
    /// Counts lines without materializing text or allocating line objects.
    pub fn layout(prepared: &PreparedText, max_width: f64, line_height: f64) -> LayoutResult {
        let line_count = count_prepared_lines(prepared, max_width);
        LayoutResult {
            line_count,
            height: line_count as f64 * line_height,
        }
    }

    /// Materializes the text and geometry of each line.
    pub fn layout_with_lines(
        prepared: &PreparedTextWithSegments,
        max_width: f64,
        line_height: f64,
    ) -> LayoutLinesResult {
        let mut lines = Vec::new();
        let line_count = walk_prepared_lines(prepared, max_width, |line| {
            lines.push(materialize_line(prepared, line))
        });
        LayoutLinesResult {
            line_count,
            height: line_count as f64 * line_height,
            lines,
        }
    }

    /// Resumes at a segment/grapheme cursor. Every returned line advances the
    /// cursor, including when an individual grapheme exceeds the width.
    pub fn layout_next_line(
        prepared: &PreparedTextWithSegments,
        start: LayoutCursor,
        max_width: f64,
    ) -> Option<LayoutLine> {
        try_step_line(prepared, start, max_width).map(|line| materialize_line(prepared, line))
    }

    /// Computes the next line without allocating text.
    pub fn layout_next_line_range(
        prepared: &PreparedTextWithSegments,
        start: LayoutCursor,
        max_width: f64,
    ) -> Option<LayoutLineRange> {
        try_step_line(prepared, start, max_width).map(|line| LayoutLineRange {
            width: line.width,
            start: line.start,
            end: line.end,
        })
    }

    /// Materializes a previously computed range. The cursor carries enough
    /// information to recover discretionary hyphens without hidden walk state.
    pub fn materialize_line_range(
        prepared: &PreparedTextWithSegments,
        line: &LayoutLineRange,
    ) -> LayoutLine {
        materialize_line(
            prepared,
            InternalLine {
                start: line.start,
                visible_end: line.end,
                end: line.end,
                width: line.width,
                append_hyphen: false,
            },
        )
    }

    pub fn walk_line_ranges(
        prepared: &PreparedTextWithSegments,
        max_width: f64,
        mut on_line: impl FnMut(LayoutLineRange),
    ) -> usize {
        let mut start = cursor(0, 0);
        let mut count = 0;
        while let Some(line) = Self::layout_next_line_range(prepared, start, max_width) {
            start = line.end;
            count += 1;
            on_line(line);
        }
        count
    }

    pub fn measure_line_stats(prepared: &PreparedTextWithSegments, max_width: f64) -> LineStats {
        let mut max_line_width = 0.0_f64;
        let line_count = walk_prepared_lines(prepared, max_width, |line| {
            max_line_width = max_line_width.max(line.width)
        });
        LineStats {
            line_count,
            max_line_width,
        }
    }

    pub fn measure_natural_width(prepared: &PreparedTextWithSegments) -> f64 {
        Self::measure_line_stats(prepared, f64::INFINITY).max_line_width
    }
}

pub(crate) fn count_prepared_lines(prepared: &PreparedText, max_width: f64) -> usize {
    walk_prepared_lines(prepared, max_width, |_| {})
}

/// A generic callback lets the count-only caller compile away output entirely.
pub(crate) fn walk_prepared_lines(
    prepared: &PreparedText,
    max_width: f64,
    mut on_line: impl FnMut(InternalLine),
) -> usize {
    if prepared.simple_line_walk_fast_path {
        walk_prepared_lines_simple(prepared, max_width, &mut on_line)
    } else {
        walk_prepared_lines_general(prepared, max_width, &mut on_line)
    }
}

fn append_breakable_walk(
    state: &mut LineState,
    prepared: &PreparedText,
    start: LayoutCursor,
    max_width: f64,
    profile: EngineProfile,
    on_line: &mut impl FnMut(InternalLine),
    count: &mut usize,
) {
    let segment_index = start.segment_index;
    let start_grapheme_index = start.grapheme_index;
    let Some(widths) = prepared.breakable_widths[segment_index].as_deref() else {
        return;
    };
    let prefixes = prepared.breakable_prefix_widths[segment_index].as_deref();
    for grapheme_index in start_grapheme_index..widths.len() {
        let width = get_breakable_advance(
            widths,
            prefixes,
            grapheme_index,
            profile.prefer_prefix_widths_for_breakable_runs,
        );
        if !state.has_content {
            state.start_grapheme(segment_index, grapheme_index, width, true);
        } else if state.width + width > max_width + profile.line_fit_epsilon {
            state.emit(None, None, false, on_line, count);
            state.start_grapheme(segment_index, grapheme_index, width, true);
        } else {
            state.width += width;
            state.end = cursor(segment_index, grapheme_index + 1);
        }
    }
    if state.has_content && state.end == cursor(segment_index, widths.len()) {
        state.end = cursor(segment_index + 1, 0);
    }
}

fn walk_prepared_lines_simple(
    prepared: &PreparedText,
    max_width: f64,
    on_line: &mut impl FnMut(InternalLine),
) -> usize {
    let profile = get_engine_profile();
    let epsilon = profile.line_fit_epsilon;
    let mut count = 0;
    let mut state = LineState::new(cursor(0, 0));
    let mut index = 0;
    while index < prepared.widths.len() {
        if !state.has_content {
            while index < prepared.widths.len() && is_skippable_at_line_start(prepared.kinds[index])
            {
                index += 1;
            }
            if index == prepared.widths.len() {
                break;
            }
        }
        let width = prepared.widths[index];
        let kind = prepared.kinds[index];
        if !state.has_content {
            if width > max_width + epsilon && prepared.breakable_widths[index].is_some() {
                append_breakable_walk(
                    &mut state,
                    prepared,
                    cursor(index, 0),
                    max_width,
                    profile,
                    on_line,
                    &mut count,
                );
            } else {
                state.start_segment(index, width, true);
            }
            state.update_pending(prepared, index, width, true);
            index += 1;
            continue;
        }
        if state.width + width > max_width + epsilon {
            if can_break_after(kind) {
                state.append_whole(index, width, true);
                state.emit(
                    Some(cursor(index + 1, 0)),
                    Some(state.width - width),
                    false,
                    on_line,
                    &mut count,
                );
                index += 1;
                continue;
            }
            if let Some(pending) = state.pending {
                if state.is_after_pending(pending) {
                    state.emit(None, None, false, on_line, &mut count);
                } else {
                    state.emit(
                        Some(cursor(pending.end_segment_index, 0)),
                        Some(pending.paint_width),
                        false,
                        on_line,
                        &mut count,
                    );
                }
                continue;
            }
            state.emit(None, None, false, on_line, &mut count);
            if width > max_width + epsilon && prepared.breakable_widths[index].is_some() {
                append_breakable_walk(
                    &mut state,
                    prepared,
                    cursor(index, 0),
                    max_width,
                    profile,
                    on_line,
                    &mut count,
                );
                index += 1;
            }
            continue;
        }
        state.append_whole(index, width, true);
        state.update_pending(prepared, index, width, true);
        index += 1;
    }
    state.emit(None, None, false, on_line, &mut count);
    count
}

fn continue_soft_hyphen_walk(
    state: &mut LineState,
    prepared: &PreparedText,
    index: usize,
    max_width: f64,
    profile: EngineProfile,
    on_line: &mut impl FnMut(InternalLine),
    count: &mut usize,
) -> bool {
    if !matches!(
        state.pending,
        Some(PendingBreak {
            kind: SegmentBreakKind::SoftHyphen,
            ..
        })
    ) {
        return false;
    }
    let Some(widths) = prepared.breakable_widths[index].as_deref() else {
        return false;
    };
    let prefixes = prepared.breakable_prefix_widths[index]
        .as_deref()
        .filter(|_| profile.prefer_prefix_widths_for_breakable_runs);
    let (fit_count, fitted_width) = fit_soft_hyphen_break(
        prefixes.unwrap_or(widths),
        state.width,
        max_width,
        profile.line_fit_epsilon,
        prepared.discretionary_hyphen_width,
        prefixes.is_some(),
    );
    if fit_count == 0 {
        return false;
    }
    state.width = fitted_width;
    state.end = cursor(index, fit_count);
    state.pending = None;
    if fit_count == widths.len() {
        state.end = cursor(index + 1, 0);
    } else {
        state.emit(
            Some(cursor(index, fit_count)),
            Some(fitted_width + prepared.discretionary_hyphen_width),
            true,
            on_line,
            count,
        );
        append_breakable_walk(
            state,
            prepared,
            cursor(index, fit_count),
            max_width,
            profile,
            on_line,
            count,
        );
    }
    true
}

fn walk_prepared_lines_general(
    prepared: &PreparedText,
    max_width: f64,
    on_line: &mut impl FnMut(InternalLine),
) -> usize {
    let profile = get_engine_profile();
    let epsilon = profile.line_fit_epsilon;
    let mut count = 0;
    for chunk in &prepared.chunks {
        if chunk.start_segment_index == chunk.end_segment_index {
            let end = cursor(chunk.consumed_end_segment_index, 0);
            on_line(InternalLine {
                start: cursor(chunk.start_segment_index, 0),
                visible_end: end,
                end,
                width: 0.0,
                append_hyphen: false,
            });
            count += 1;
            continue;
        }
        let mut state = LineState::new(cursor(chunk.start_segment_index, 0));
        let mut index = chunk.start_segment_index;
        while index < chunk.end_segment_index {
            let kind = prepared.kinds[index];
            let mut width = segment_advance(prepared, index, state.width);
            if kind == SegmentBreakKind::SoftHyphen {
                if state.has_content {
                    state.end = cursor(index + 1, 0);
                    let hyphen_width = state.width + prepared.discretionary_hyphen_width;
                    state.pending = Some(PendingBreak {
                        end_segment_index: index + 1,
                        fit_width: hyphen_width,
                        paint_width: hyphen_width,
                        kind,
                    });
                }
                index += 1;
                continue;
            }
            if !state.has_content {
                while index < chunk.end_segment_index
                    && is_skippable_at_line_start(prepared.kinds[index])
                {
                    index += 1;
                }
                if index == chunk.end_segment_index {
                    break;
                }
                width = segment_advance(prepared, index, state.width);
                if width > max_width + epsilon && prepared.breakable_widths[index].is_some() {
                    append_breakable_walk(
                        &mut state,
                        prepared,
                        cursor(index, 0),
                        max_width,
                        profile,
                        on_line,
                        &mut count,
                    );
                } else {
                    state.start_segment(index, width, true);
                }
                state.update_pending(prepared, index, width, false);
                index += 1;
                continue;
            }
            if state.width + width > max_width + epsilon {
                let current_fit = state.width
                    + if kind == SegmentBreakKind::Tab {
                        0.0
                    } else {
                        prepared.line_end_fit_advances[index]
                    };
                let current_paint = state.width
                    + if kind == SegmentBreakKind::Tab {
                        width
                    } else {
                        prepared.line_end_paint_advances[index]
                    };
                if let Some(pending) = state.pending.filter(|p| {
                    p.kind == SegmentBreakKind::SoftHyphen
                        && profile.prefer_early_soft_hyphen_break
                        && p.fit_width <= max_width + epsilon
                }) {
                    state.emit(
                        Some(cursor(pending.end_segment_index, 0)),
                        Some(pending.paint_width),
                        true,
                        on_line,
                        &mut count,
                    );
                    continue;
                }
                if continue_soft_hyphen_walk(
                    &mut state, prepared, index, max_width, profile, on_line, &mut count,
                ) {
                    index += 1;
                    continue;
                }
                if can_break_after(kind) && current_fit <= max_width + epsilon {
                    state.append_whole(index, width, true);
                    state.emit(
                        Some(cursor(index + 1, 0)),
                        Some(current_paint),
                        false,
                        on_line,
                        &mut count,
                    );
                    index += 1;
                    continue;
                }
                if let Some(pending) = state.pending.filter(|p| p.fit_width <= max_width + epsilon)
                {
                    if state.is_after_pending(pending) {
                        state.emit(None, None, false, on_line, &mut count);
                    } else {
                        state.emit(
                            Some(cursor(pending.end_segment_index, 0)),
                            Some(pending.paint_width),
                            pending.kind == SegmentBreakKind::SoftHyphen,
                            on_line,
                            &mut count,
                        );
                    }
                    continue;
                }
                state.emit(None, None, false, on_line, &mut count);
                if width > max_width + epsilon && prepared.breakable_widths[index].is_some() {
                    append_breakable_walk(
                        &mut state,
                        prepared,
                        cursor(index, 0),
                        max_width,
                        profile,
                        on_line,
                        &mut count,
                    );
                    index += 1;
                }
                continue;
            }
            state.append_whole(index, width, true);
            state.update_pending(prepared, index, width, false);
            index += 1;
        }
        let paint = state
            .pending
            .filter(|p| p.end_segment_index == chunk.consumed_end_segment_index)
            .map_or(state.width, |p| p.paint_width);
        state.emit(
            Some(cursor(chunk.consumed_end_segment_index, 0)),
            Some(paint),
            false,
            on_line,
            &mut count,
        );
    }
    count
}

pub(crate) fn try_step_line(
    prepared: &PreparedText,
    start: LayoutCursor,
    max_width: f64,
) -> Option<InternalLine> {
    let normalized = normalize_line_start(prepared, start)?;
    if prepared.simple_line_walk_fast_path {
        try_step_line_simple(prepared, normalized, max_width)
    } else {
        try_step_line_general(prepared, normalized, max_width)
    }
}

fn append_breakable_step(
    state: &mut LineState,
    prepared: &PreparedText,
    index: usize,
    first: usize,
    max_width: f64,
    profile: EngineProfile,
) -> Option<InternalLine> {
    let widths = prepared.breakable_widths[index].as_deref()?;
    let prefixes = prepared.breakable_prefix_widths[index].as_deref();
    for grapheme_index in first..widths.len() {
        let width = get_breakable_advance(
            widths,
            prefixes,
            grapheme_index,
            profile.prefer_prefix_widths_for_breakable_runs,
        );
        if !state.has_content {
            state.start_grapheme(index, grapheme_index, width, false);
        } else if state.width + width > max_width + profile.line_fit_epsilon {
            return state.finish(None, None, false);
        } else {
            state.width += width;
            state.end = cursor(index, grapheme_index + 1);
        }
    }
    if state.has_content && state.end == cursor(index, widths.len()) {
        state.end = cursor(index + 1, 0);
    }
    None
}

fn try_step_line_simple(
    prepared: &PreparedText,
    start: LayoutCursor,
    max_width: f64,
) -> Option<InternalLine> {
    let profile = get_engine_profile();
    let epsilon = profile.line_fit_epsilon;
    let mut state = LineState::new(start);
    for index in start.segment_index..prepared.widths.len() {
        let width = prepared.widths[index];
        let kind = prepared.kinds[index];
        let first = if index == start.segment_index {
            start.grapheme_index
        } else {
            0
        };
        if !state.has_content {
            if first > 0
                || (width > max_width + epsilon && prepared.breakable_widths[index].is_some())
            {
                if let Some(line) =
                    append_breakable_step(&mut state, prepared, index, first, max_width, profile)
                {
                    return Some(line);
                }
            } else {
                state.start_segment(index, width, false);
            }
            state.update_pending(prepared, index, width, true);
            continue;
        }
        if state.width + width > max_width + epsilon {
            if can_break_after(kind) {
                state.append_whole(index, width, false);
                return state.finish(Some(cursor(index + 1, 0)), Some(state.width - width), false);
            }
            if let Some(pending) = state.pending {
                return if state.is_after_pending(pending) {
                    state.finish(None, None, false)
                } else {
                    state.finish(
                        Some(cursor(pending.end_segment_index, 0)),
                        Some(pending.paint_width),
                        false,
                    )
                };
            }
            return state.finish(None, None, false);
        }
        state.append_whole(index, width, false);
        state.update_pending(prepared, index, width, true);
    }
    state.finish(None, None, false)
}

fn maybe_finish_at_soft_hyphen(
    state: &mut LineState,
    prepared: &PreparedText,
    index: usize,
    max_width: f64,
    profile: EngineProfile,
) -> Option<InternalLine> {
    let pending = state
        .pending
        .filter(|p| p.kind == SegmentBreakKind::SoftHyphen)?;
    if let Some(widths) = prepared.breakable_widths[index].as_deref() {
        let prefixes = prepared.breakable_prefix_widths[index]
            .as_deref()
            .filter(|_| profile.prefer_prefix_widths_for_breakable_runs);
        let (fit_count, fitted_width) = fit_soft_hyphen_break(
            prefixes.unwrap_or(widths),
            state.width,
            max_width,
            profile.line_fit_epsilon,
            prepared.discretionary_hyphen_width,
            prefixes.is_some(),
        );
        if fit_count == widths.len() {
            state.width = fitted_width;
            state.end = cursor(index + 1, 0);
            state.pending = None;
            return None;
        }
        if fit_count > 0 {
            return state.finish(
                Some(cursor(index, fit_count)),
                Some(fitted_width + prepared.discretionary_hyphen_width),
                true,
            );
        }
    }
    if pending.fit_width <= max_width + profile.line_fit_epsilon {
        return state.finish(
            Some(cursor(pending.end_segment_index, 0)),
            Some(pending.paint_width),
            true,
        );
    }
    None
}

fn try_step_line_general(
    prepared: &PreparedText,
    start: LayoutCursor,
    max_width: f64,
) -> Option<InternalLine> {
    let chunk = &prepared.chunks[find_chunk_index_for_start(prepared, start.segment_index)?];
    if chunk.start_segment_index == chunk.end_segment_index {
        let end = cursor(chunk.consumed_end_segment_index, 0);
        return Some(InternalLine {
            start: cursor(chunk.start_segment_index, 0),
            visible_end: end,
            end,
            width: 0.0,
            append_hyphen: false,
        });
    }
    let profile = get_engine_profile();
    let epsilon = profile.line_fit_epsilon;
    let mut state = LineState::new(start);
    for index in start.segment_index..chunk.end_segment_index {
        let kind = prepared.kinds[index];
        let first = if index == start.segment_index {
            start.grapheme_index
        } else {
            0
        };
        let width = segment_advance(prepared, index, state.width);
        if kind == SegmentBreakKind::SoftHyphen && first == 0 {
            if state.has_content {
                state.end = cursor(index + 1, 0);
                let hyphen_width = state.width + prepared.discretionary_hyphen_width;
                state.pending = Some(PendingBreak {
                    end_segment_index: index + 1,
                    fit_width: hyphen_width,
                    paint_width: hyphen_width,
                    kind,
                });
            }
            continue;
        }
        if !state.has_content {
            if first > 0
                || (width > max_width + epsilon && prepared.breakable_widths[index].is_some())
            {
                if let Some(line) =
                    append_breakable_step(&mut state, prepared, index, first, max_width, profile)
                {
                    return Some(line);
                }
            } else {
                state.start_segment(index, width, false);
            }
            state.update_pending(prepared, index, width, false);
            continue;
        }
        if state.width + width > max_width + epsilon {
            let current_fit = state.width
                + if kind == SegmentBreakKind::Tab {
                    0.0
                } else {
                    prepared.line_end_fit_advances[index]
                };
            let current_paint = state.width
                + if kind == SegmentBreakKind::Tab {
                    width
                } else {
                    prepared.line_end_paint_advances[index]
                };
            if let Some(pending) = state.pending.filter(|p| {
                p.kind == SegmentBreakKind::SoftHyphen
                    && profile.prefer_early_soft_hyphen_break
                    && p.fit_width <= max_width + epsilon
            }) {
                return state.finish(
                    Some(cursor(pending.end_segment_index, 0)),
                    Some(pending.paint_width),
                    true,
                );
            }
            if let Some(line) =
                maybe_finish_at_soft_hyphen(&mut state, prepared, index, max_width, profile)
            {
                return Some(line);
            }
            if can_break_after(kind) && current_fit <= max_width + epsilon {
                state.append_whole(index, width, false);
                return state.finish(Some(cursor(index + 1, 0)), Some(current_paint), false);
            }
            if let Some(pending) = state.pending.filter(|p| p.fit_width <= max_width + epsilon) {
                return if state.is_after_pending(pending) {
                    state.finish(None, None, false)
                } else {
                    state.finish(
                        Some(cursor(pending.end_segment_index, 0)),
                        Some(pending.paint_width),
                        pending.kind == SegmentBreakKind::SoftHyphen,
                    )
                };
            }
            return state.finish(None, None, false);
        }
        state.append_whole(index, width, false);
        state.update_pending(prepared, index, width, false);
    }
    let paint = state
        .pending
        .filter(|p| {
            p.end_segment_index == chunk.consumed_end_segment_index && state.end.grapheme_index == 0
        })
        .map_or(state.width, |p| p.paint_width);
    state.finish(
        Some(cursor(chunk.consumed_end_segment_index, 0)),
        Some(paint),
        false,
    )
}

fn find_chunk_index_for_start(prepared: &PreparedText, segment_index: usize) -> Option<usize> {
    // Chunks are sorted and disjoint. Binary search preserves source behavior
    // while avoiding a scan from paragraph start for every streamed line.
    let index = prepared
        .chunks
        .partition_point(|chunk| chunk.consumed_end_segment_index <= segment_index);
    (index < prepared.chunks.len()).then_some(index)
}

fn normalize_line_start(prepared: &PreparedText, start: LayoutCursor) -> Option<LayoutCursor> {
    if start.segment_index >= prepared.widths.len() {
        return None;
    }
    if start.grapheme_index > 0 {
        let widths = prepared.breakable_widths[start.segment_index].as_ref()?;
        return (start.grapheme_index < widths.len()).then_some(start);
    }
    let chunk = &prepared.chunks[find_chunk_index_for_start(prepared, start.segment_index)?];
    let mut index = start.segment_index.max(chunk.start_segment_index);
    if chunk.start_segment_index == chunk.end_segment_index && index == chunk.start_segment_index {
        return Some(cursor(index, 0));
    }
    while index < chunk.end_segment_index {
        if !is_skippable_at_line_start(prepared.kinds[index]) {
            return Some(cursor(index, 0));
        }
        index += 1;
    }
    (chunk.consumed_end_segment_index < prepared.widths.len())
        .then_some(cursor(chunk.consumed_end_segment_index, 0))
}

pub(crate) fn is_skippable_at_line_start(kind: SegmentBreakKind) -> bool {
    matches!(
        kind,
        SegmentBreakKind::Space | SegmentBreakKind::ZeroWidthBreak | SegmentBreakKind::SoftHyphen
    )
}

fn get_breakable_advance(
    widths: &[f64],
    prefixes: Option<&[f64]>,
    index: usize,
    prefer_prefixes: bool,
) -> f64 {
    if let Some(prefixes) = prefixes.filter(|_| prefer_prefixes) {
        prefixes[index] - if index > 0 { prefixes[index - 1] } else { 0.0 }
    } else {
        widths[index]
    }
}

fn fit_soft_hyphen_break(
    widths: &[f64],
    initial_width: f64,
    max_width: f64,
    epsilon: f64,
    hyphen_width: f64,
    cumulative: bool,
) -> (usize, f64) {
    let mut fit_count = 0;
    let mut fitted_width = initial_width;
    while fit_count < widths.len() {
        let next_width = if cumulative {
            initial_width + widths[fit_count]
        } else {
            fitted_width + widths[fit_count]
        };
        let next_line_width = if fit_count + 1 < widths.len() {
            next_width + hyphen_width
        } else {
            next_width
        };
        if next_line_width > max_width + epsilon {
            break;
        }
        fitted_width = next_width;
        fit_count += 1;
    }
    (fit_count, fitted_width)
}

pub(crate) fn can_break_after(kind: SegmentBreakKind) -> bool {
    matches!(
        kind,
        SegmentBreakKind::Space
            | SegmentBreakKind::PreservedSpace
            | SegmentBreakKind::Tab
            | SegmentBreakKind::ZeroWidthBreak
            | SegmentBreakKind::SoftHyphen
    )
}

fn segment_advance(prepared: &PreparedText, index: usize, line_width: f64) -> f64 {
    if prepared.kinds[index] == SegmentBreakKind::Tab {
        get_tab_advance(line_width, prepared.tab_stop_advance)
    } else {
        prepared.widths[index]
    }
}

fn get_tab_advance(line_width: f64, stop: f64) -> f64 {
    if stop <= 0.0 {
        return 0.0;
    }
    let remainder = line_width % stop;
    if remainder.abs() <= 1e-6 {
        stop
    } else {
        stop - remainder
    }
}
