//! Lazy line materialization from segment/grapheme cursor ranges.

use crate::pretext_layout_line_break::InternalLine;
use crate::pretext_layout_measurement::text_element_offsets;
use crate::{LayoutCursor, LayoutLine, PreparedTextWithSegments, SegmentBreakKind};

pub(crate) fn materialize_line(
    prepared: &PreparedTextWithSegments,
    line: InternalLine,
) -> LayoutLine {
    // The consumed end includes whitespace and hard breaks. Invisible kinds are
    // removed below; preserved whitespace remains in the materialized string.
    debug_assert!(line.visible_end <= line.end);
    LayoutLine {
        text: build_line_text_from_range(prepared, line.start, line.end, line.append_hyphen),
        width: line.width,
        start: line.start,
        end: line.end,
    }
}

pub(crate) fn build_line_text_from_range(
    prepared: &PreparedTextWithSegments,
    start: LayoutCursor,
    end: LayoutCursor,
    append_hyphen: bool,
) -> String {
    if start > end || start.segment_index >= prepared.segments.len() {
        return String::new();
    }
    let append_hyphen =
        append_hyphen || line_has_discretionary_hyphen(prepared, start, end.segment_index);
    if start == end && !append_hyphen {
        return String::new();
    }
    let mut text = String::with_capacity(estimate_line_text_capacity(
        prepared,
        start.segment_index,
        end.segment_index,
        append_hyphen,
    ));
    let mut first_grapheme = start.grapheme_index;
    for index in start.segment_index..prepared.segments.len() {
        if index > end.segment_index
            || (index == end.segment_index && first_grapheme == end.grapheme_index)
        {
            break;
        }
        if matches!(
            prepared.kinds[index],
            SegmentBreakKind::ZeroWidthBreak
                | SegmentBreakKind::SoftHyphen
                | SegmentBreakKind::HardBreak
        ) {
            first_grapheme = 0;
            continue;
        }
        if index == end.segment_index {
            if append_hyphen {
                text.push('-');
            }
            append_segment_slice(
                &mut text,
                prepared,
                index,
                first_grapheme,
                Some(end.grapheme_index),
            );
            break;
        }
        append_segment_slice(&mut text, prepared, index, first_grapheme, None);
        first_grapheme = 0;
    }
    if append_hyphen && end.grapheme_index == 0 {
        text.push('-');
    }
    text
}

fn estimate_line_text_capacity(
    prepared: &PreparedTextWithSegments,
    start: usize,
    end: usize,
    append_hyphen: bool,
) -> usize {
    prepared
        .segments
        .iter()
        .enumerate()
        .skip(start)
        .take(end.saturating_sub(start).saturating_add(1))
        .filter(|(index, _)| {
            !matches!(
                prepared.kinds[*index],
                SegmentBreakKind::ZeroWidthBreak
                    | SegmentBreakKind::SoftHyphen
                    | SegmentBreakKind::HardBreak
            )
        })
        .map(|(_, segment)| segment.len())
        .sum::<usize>()
        + usize::from(append_hyphen)
}

pub(crate) fn line_has_discretionary_hyphen(
    prepared: &PreparedTextWithSegments,
    start: LayoutCursor,
    end_segment_index: usize,
) -> bool {
    end_segment_index
        .checked_sub(1)
        .and_then(|index| prepared.kinds.get(index))
        == Some(&SegmentBreakKind::SoftHyphen)
        && !(start.segment_index == end_segment_index && start.grapheme_index > 0)
}

fn append_segment_slice(
    text: &mut String,
    prepared: &PreparedTextWithSegments,
    index: usize,
    start: usize,
    end: Option<usize>,
) {
    let Some(segment) = prepared.segments.get(index) else {
        return;
    };
    if start == 0 && end.is_none() {
        text.push_str(segment);
        return;
    }
    let offsets = segment_grapheme_offsets(prepared, index);
    let first = offsets.get(start).copied().unwrap_or(segment.len());
    let last = offsets
        .get(end.unwrap_or(offsets.len().saturating_sub(1)))
        .copied()
        .unwrap_or(segment.len());
    if first < last {
        text.push_str(&segment[first..last]);
    }
}

/// UTF-8 offsets are cached in the prepared object, rather than in global weak
/// tables. They are computed only for segments requiring a partial text slice.
pub(crate) fn segment_grapheme_offsets(
    prepared: &PreparedTextWithSegments,
    index: usize,
) -> &[usize] {
    let Some(segment) = prepared.segments.get(index) else {
        return &[];
    };
    let Some(cache) = prepared.grapheme_offsets.get(index) else {
        return &[];
    };
    cache.get_or_init(|| text_element_offsets(segment).collect())
}
