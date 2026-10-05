use std::rc::Rc;

use crate::media::text_formatting::{
    DrawableTextRun, FormattingObjectPool, LogicalTextRunEnumerator, ShapedTextRun, TextFormatterImpl, TextCharacters, TextCollapsingProperties, TextFormatter, TextLine, TextRun,
    TextRunProperties,
};
use crate::media::FlowDirection;
use crate::utilities::MathUtilities;

/// Provides text collapsing properties that replace the middle segments of a file path with an ellipsis symbol when
/// the rendered width exceeds a specified limit.
///
/// This class is typically used to display file paths in a compact form by collapsing segments near
/// the center and inserting an ellipsis, ensuring that the most relevant parts of the path remain visible. It is
/// suitable for scenarios where space is limited, such as UI elements that display file or directory paths. The
/// collapsing behavior prioritizes preserving the beginning and end segments of the path.
pub struct TextPathSegmentEllipsis {
    width: f64,
    symbol: Rc<dyn TextRun>,
    flow_direction: FlowDirection,
}

/// A stretch of the line's text: either one separator or the text between separators.
#[derive(Clone, Copy)]
struct Segment {
    start: i32,
    length: i32,
    width: f64,
    is_separator: bool,
}

impl TextPathSegmentEllipsis {
    /// Initializes a new instance with the specified ellipsis symbol, maximum
    /// width, text run properties, and flow direction.
    ///
    /// * `ellipsis` — the string to use as the ellipsis symbol when collapsing path segments.
    /// * `width` — the maximum width, in device-independent pixels, within which the text must fit.
    /// * `text_run_properties` — the text formatting properties to apply to the ellipsis symbol.
    /// * `flow_direction` — the flow direction for text layout.
    pub fn new(
        ellipsis: &str,
        width: f64,
        text_run_properties: Rc<dyn TextRunProperties>,
        flow_direction: FlowDirection,
    ) -> Self {
        Self { width, symbol: Rc::new(TextCharacters::from_str(ellipsis, text_run_properties)), flow_direction }
    }

    /// The part of upstream's `Collapse` that runs with the logical runs
    /// rented; the caller returns `logical_runs` afterwards.
    fn collapse_core(
        &self,
        text_line: &dyn TextLine,
        logical_runs: &[Rc<dyn TextRun>],
        shaped_symbol: Rc<ShapedTextRun>,
        total_width: f64,
        object_pool: &FormattingObjectPool,
    ) -> Option<Vec<Rc<dyn TextRun>>> {
        // Pre-compute cumulative run start char positions so that
        // measure_segment_width can binary-search to the first overlapping
        // run instead of re-scanning from index 0 on every call. Built
        // once per collapse; reused by every segment-width measurement.
        // run_start_chars[i] = sum of lengths of runs 0..i-1;
        // run_start_chars[count] = total char length (sentinel).
        let mut run_start_chars = vec![0i32; logical_runs.len() + 1];

        for (i, run) in logical_runs.iter().enumerate() {
            run_start_chars[i + 1] = run_start_chars[i] + run.length();
        }

        // Segment ranges
        let mut segments: Vec<Segment> = Vec::new();
        let mut candidate_segment_indices: Vec<usize> = Vec::new();
        let mut global_index = 0i32;
        let mut current_seg_start = 0i32;
        let mut in_separator = false;

        for run in logical_runs {
            if run.is::<ShapedTextRun>() {
                for &ch in run.text_span() {
                    if Self::is_separator(ch) {
                        // finish previous non-separator segment
                        if !in_separator && global_index - current_seg_start > 0 {
                            let segment_width = Self::measure_segment_width(
                                logical_runs,
                                &run_start_chars,
                                current_seg_start,
                                global_index - current_seg_start,
                            );

                            segments.push(Segment {
                                start: current_seg_start,
                                length: global_index - current_seg_start,
                                width: segment_width,
                                is_separator: false,
                            });
                        }

                        let separator_width =
                            Self::measure_segment_width(logical_runs, &run_start_chars, global_index, 1);

                        // separator as its own segment
                        segments.push(Segment { start: global_index, length: 1, width: separator_width, is_separator: true });

                        // next segment starts after separator
                        current_seg_start = global_index + 1;
                        in_separator = true;
                    } else if in_separator {
                        // start of a non-separator segment
                        current_seg_start = global_index;
                        in_separator = false;
                    }

                    global_index += 1;
                }
            } else {
                // Non shaped run is treated as non-separator
                if in_separator {
                    current_seg_start = global_index;
                    in_separator = false;
                }

                global_index += run.length();
            }
        }

        // Add last pending segment if any
        if global_index - current_seg_start > 0 {
            let segment_width = Self::measure_segment_width(
                logical_runs,
                &run_start_chars,
                current_seg_start,
                global_index - current_seg_start,
            );

            segments.push(Segment {
                start: current_seg_start,
                length: global_index - current_seg_start,
                width: segment_width,
                is_separator: false,
            });
        }

        if segments.is_empty() {
            // Nothing to collapse
            return None;
        }

        let mut prefix = vec![0f64; segments.len() + 1];

        // Measure segment widths
        for (i, segment) in segments.iter().enumerate() {
            if !segment.is_separator {
                candidate_segment_indices.push(i);
            }

            prefix[i + 1] = prefix[i] + segment.width;
        }

        // Determine center character index to prefer collapsing ranges near the middle.
        let mid_char = global_index / 2;

        // Find candidate whose center is closest to mid_char
        let mut center_candidate_idx = 0i32;
        let mut best_dist = i64::MAX;

        for (i, &candidate_segment_index) in candidate_segment_indices.iter().enumerate() {
            let segment = segments[candidate_segment_index];
            let seg_center = segment.start + segment.length / 2;
            let dist = (seg_center - mid_char).abs() as i64;

            if dist < best_dist {
                best_dist = dist;
                center_candidate_idx = i as i32;
            }
        }

        // Expand windows around center_candidate_idx.
        let candidate_count = candidate_segment_indices.len() as i32;

        if candidate_count > 0 {
            let mut window_starts: Vec<i32> = Vec::new();

            for window_size in 1..=candidate_count {
                // For a given window_size, try all windows of that size centered as close as possible to center_candidate_idx.
                // Compute start index of window such that center is as near as possible.
                let half = (window_size - 1) / 2;
                let start = center_candidate_idx - half;
                // For even window sizes, prefer left-leaning start, also try shifting the window across the center.
                window_starts.clear();

                // clamp start range
                let min_start = 0.max(center_candidate_idx - (window_size - 1));
                let max_start = (candidate_count - window_size).min(center_candidate_idx + (window_size - 1));

                // Left side first
                let mut s = start;
                while s >= min_start {
                    window_starts.push(s);
                    s -= 1;
                }

                // Right side next
                let mut s = start + 1;
                while s <= max_start {
                    window_starts.push(s);
                    s += 1;
                }

                for &window_start in &window_starts {
                    if window_start < 0 || window_start + window_size > candidate_count {
                        continue;
                    }

                    let left_cand = window_start;
                    let right_cand = window_start + window_size - 1;

                    // Map candidate window to segments range (in segments list)
                    let seg_start_index = candidate_segment_indices[left_cand as usize];
                    let seg_end_index = candidate_segment_indices[right_cand as usize];

                    // Ensure that we leave at least one character on each side (prefer middle-only removal)
                    let left_remaining = segments[seg_start_index].start;
                    let right_remaining = global_index - (segments[seg_end_index].start + segments[seg_end_index].length);

                    if left_remaining <= 0 || right_remaining <= 0 {
                        continue;
                    }

                    let trimmed_width = prefix[seg_end_index + 1] - prefix[seg_start_index];

                    if MathUtilities::less_than_or_close(
                        total_width - trimmed_width + shaped_symbol.size().width,
                        self.width,
                    ) {
                        // perform split using character indices
                        let remove_start = segments[seg_start_index].start;
                        let remove_length =
                            (segments[seg_end_index].start + segments[seg_end_index].length) - remove_start;

                        let (first, remainder) =
                            TextFormatterImpl::split_text_runs(logical_runs, remove_start, object_pool).deconstruct();

                        let Some(remainder) = remainder else {
                            // We reached the end
                            object_pool.text_run_lists.return_optional(first);

                            return None;
                        };

                        let (middle, last) =
                            TextFormatterImpl::split_text_runs(&remainder, remove_length, object_pool).deconstruct();

                        // Build resulting runs
                        // first + shaped_symbol + last
                        let mut result: Vec<Rc<dyn TextRun>> = Vec::with_capacity(
                            first.as_ref().map_or(0, |runs| runs.len()) + 1 + last.as_ref().map_or(0, |runs| runs.len()),
                        );

                        if let Some(first) = &first {
                            result.extend(first.iter().cloned());
                        }

                        result.push(shaped_symbol);

                        if let Some(last) = &last {
                            result.extend(last.iter().cloned());
                        }

                        // Return rented lists
                        object_pool.text_run_lists.return_optional(first);
                        object_pool.text_run_lists.return_list(remainder);
                        object_pool.text_run_lists.return_optional(middle);
                        object_pool.text_run_lists.return_optional(last);

                        return Some(result);
                    }
                }
            }
        }

        // Fallback - try to trim at segment boundaries from start
        let mut current_length = 0;
        let mut remaining_width = text_line.width_including_trailing_whitespace();

        for (segment_index, segment) in segments.iter().enumerate() {
            if segment_index < segments.len() - 1
                && MathUtilities::greater_than(remaining_width - segment.width, self.width)
            {
                remaining_width -= segment.width;
                current_length += segment.length;

                continue;
            }

            // Split before current segment
            let (first, second): (Option<Vec<Rc<dyn TextRun>>>, Option<Vec<Rc<dyn TextRun>>>) =
                TextFormatterImpl::split_text_runs(logical_runs, current_length, object_pool).deconstruct();

            let mut trimmed_run: Option<Rc<dyn TextRun>> = None;
            let mut remaining_run_count = 0;

            if let Some(second) = &second {
                if !second.is_empty() {
                    remaining_run_count = second.len() - 1;

                    let run = &second[0];

                    if let Some(shaped_run) = run.downcast_ref::<ShapedTextRun>() {
                        let measure_width = self.width - shaped_symbol.size().width;

                        if let Some((length, _)) = shaped_run.try_measure_characters_backwards(measure_width) {
                            let split_at = shaped_run.length() - length;

                            if split_at > 0 {
                                trimmed_run = shaped_run.split(split_at).second.map(|run| run as Rc<dyn TextRun>);
                            } else if length > 0 {
                                // The whole run fits in the remaining budget — no split needed,
                                // use the run as-is. (Splitting at 0 panics.)
                                trimmed_run = Some(run.clone());
                            }
                            // else: length == 0 → nothing of this run survives; trimmed_run stays None.
                        }
                    }
                }
            }

            let run_count = usize::from(trimmed_run.is_some()) + 1 + remaining_run_count;

            let mut result: Vec<Rc<dyn TextRun>> = Vec::with_capacity(run_count);

            // Append symbol
            result.push(shaped_symbol);

            // Append trimmed run if any
            if let Some(trimmed_run) = trimmed_run {
                result.push(trimmed_run);
            }

            // Append remaining runs
            if let Some(second) = &second {
                result.extend(second.iter().skip(1).cloned());
            }

            // Return rented lists
            object_pool.text_run_lists.return_optional(first);
            object_pool.text_run_lists.return_optional(second);

            return Some(result);
        }

        // No suitable segment found
        None
    }

    /// Whether the character separates path segments. Upstream lists the
    /// platform's directory separators next to `/` and `\`; on every platform
    /// that is this set.
    fn is_separator(ch: u16) -> bool {
        ch == b'/' as u16 || ch == b'\\' as u16
    }

    fn measure_segment_width(
        runs: &[Rc<dyn TextRun>],
        run_start_chars: &[i32],
        segment_start: i32,
        segment_length: i32,
    ) -> f64 {
        if segment_length <= 0 {
            return 0.0;
        }

        let segment_end = segment_start + segment_length;

        // Binary search run_start_chars for the largest i with run_start_chars[i] <= segment_start.
        // That's the first run whose range can overlap the segment.
        let mut i = Self::find_first_overlapping_run(run_start_chars, segment_start);

        let mut width = 0.0;

        while i < runs.len() {
            let run_start = run_start_chars[i];

            if run_start >= segment_end {
                break;
            }

            let run = &runs[i];
            let run_end = run_start + run.length();

            let overlap_start = segment_start.max(run_start);
            let overlap_end = segment_end.min(run_end);

            if overlap_end > overlap_start {
                if let Some(shaped) = run.downcast_ref::<ShapedTextRun>() {
                    // The shaped buffer answers this from its cluster cache; O(log clusters).
                    width += shaped.shaped_buffer().get_char_range_width(overlap_start - run_start, overlap_end - run_start);
                } else if let Some(drawable) = run.as_drawable() {
                    // Drawables are atomic: count full width when they completely overlap.
                    if overlap_end - overlap_start >= run.length() {
                        width += drawable.size().width;
                    }
                }
            }

            i += 1;
        }

        width
    }

    fn find_first_overlapping_run(run_start_chars: &[i32], char_index: i32) -> usize {
        if char_index <= 0 {
            return 0;
        }

        // Upper bound excludes the sentinel entry; we want a run index, not a boundary.
        if run_start_chars.len() < 2 {
            return 0;
        }

        let mut lo = 0usize;
        let mut hi = run_start_chars.len() - 2;

        while lo < hi {
            let mid = (lo + hi + 1) >> 1;

            if run_start_chars[mid] <= char_index {
                lo = mid;
            } else {
                hi = mid - 1;
            }
        }

        lo
    }
}

impl TextCollapsingProperties for TextPathSegmentEllipsis {
    fn width(&self) -> f64 {
        self.width
    }

    fn symbol(&self) -> &Rc<dyn TextRun> {
        &self.symbol
    }

    fn flow_direction(&self) -> FlowDirection {
        self.flow_direction
    }

    fn collapse(&self, text_line: &dyn TextLine) -> Option<Vec<Rc<dyn TextRun>>> {
        if text_line.text_runs().is_empty() {
            return None;
        }

        let object_pool = FormattingObjectPool::instance();

        let shaped_symbol = <dyn TextFormatter>::create_symbol(&*self.symbol, self.flow_direction);

        if MathUtilities::less_than(self.width, shaped_symbol.size().width) {
            // Nothing to collapse
            return None;
        }

        let total_width = text_line.width();

        if MathUtilities::less_than_or_close(total_width, self.width) {
            // Nothing to collapse
            return None;
        }

        // Extract logical runs from the line
        let mut logical_runs = object_pool.text_run_lists.rent();

        {
            let mut enumerator = LogicalTextRunEnumerator::new(text_line);

            while let Some(run) = enumerator.move_next() {
                logical_runs.push(run);
            }
        }

        let result = self.collapse_core(text_line, &logical_runs, shaped_symbol, total_width, object_pool);

        object_pool.text_run_lists.return_list(logical_runs);

        result
    }
}
