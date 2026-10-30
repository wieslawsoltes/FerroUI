// Copyright (c) Microsoft Corporation. All rights reserved.
// Licensed under the MIT License. See NOTICE.md in the containing module.

// The line optimization routines from LinedFlowLayout.cpp are separated from
// host callbacks here. All candidate evaluation works on plain measured data.

use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug)]
pub(super) struct ItemMetrics {
    pub index: i32,
    pub width: f64,
    pub minimum: f64,
    pub maximum: f64,
}

#[derive(Clone, Debug)]
pub(super) struct LinedFlowLine {
    pub index: i32,
    pub first: i32,
    pub widths: Vec<f64>,
    pub desired_width: f64,
}
impl LinedFlowLine {
    pub fn end(&self) -> i32 {
        self.first + self.widths.len() as i32
    }
}

#[derive(Clone)]
struct Candidate {
    counts: Vec<usize>,
    widths: Vec<f64>,
    starts: Vec<usize>,
    available: f64,
    drawback: f64,
    smallest_head: f64,
    smallest_tail: f64,
    best_head: Option<(i32, i32, f64)>,
    best_tail: Option<(i32, i32, f64)>,
}

#[derive(Clone, Copy)]
pub(super) struct LineOptions {
    pub available: f64,
    pub spacing: f64,
    pub stretch: bool,
    pub first_line: i32,
    /// Offset in the complete sized-line vector, as passed to the source's
    /// equalization-cost helper (distinct from the absolute source line).
    pub begin_line_vector_index: i32,
    pub line_count: usize,
    pub total_item_count: i32,
    pub average_items_per_line: f64,
    pub lines_per_viewport: i32,
    /// A block preceding frozen lines is filled from the frozen edge backwards.
    pub forward: bool,
    pub wrap_multiplier: f64,
}

/// GetFirstAndLastDisplayedLineIndexes for partially displayed lines with no
/// padding. Lines touching an edge or hidden entirely in spacing do not count.
pub(super) fn displayed_line_range(
    viewport: f64,
    mut offset: f64,
    height: f64,
    spacing: f64,
    line_count: i32,
) -> (i32, i32) {
    if line_count <= 0 {
        return (-1, -1);
    }
    if viewport == f64::INFINITY {
        return (0, line_count - 1);
    }
    let stride = height + spacing;
    if viewport <= 0.0 || stride <= 0.0 {
        return (-1, -1);
    }
    let extent = line_count as f64 * stride - spacing;
    let maximum_offset = (extent - viewport).max(0.0);
    offset = offset.min(maximum_offset);
    if offset.abs() < 0.01 {
        offset = 0.0;
    }
    // Zero padding still accounts for a temporary negative scroll offset.
    let near = offset + offset.min(0.0);
    let far = offset + viewport - (maximum_offset - offset).min(0.0);
    if far <= 0.0 || near >= extent {
        return (-1, -1);
    }
    let spacing_portion = spacing / stride;
    let fractional_first = near.max(0.0) / stride;
    let mut first_displayed = fractional_first as i32;
    if first_displayed as f64 + 1.0 - fractional_first <= spacing_portion
        && first_displayed + 1 < line_count
    {
        first_displayed += 1;
    }
    let fractional_last = (far.min(extent) + spacing) / stride;
    let mut last_displayed = fractional_last as i32;
    if fractional_last - last_displayed as f64 <= spacing_portion {
        last_displayed = (last_displayed - 1).max(0);
    }
    if first_displayed > last_displayed {
        (-1, -1)
    } else {
        (first_displayed, last_displayed)
    }
}

/// ComputeFrozenItemsRange. Frozen lines cover the viewport plus the larger
/// of 80% of a viewport or 40% of each adjacent sized region.
pub(super) fn frozen_line_range(
    viewport: f64,
    offset: f64,
    height: f64,
    spacing: f64,
    line_count: i32,
    sized: (i32, i32),
) -> (i32, i32) {
    let (first_sized, last_sized) = sized;
    if line_count <= 0 || first_sized > last_sized {
        return (-1, -1);
    }
    if viewport == f64::INFINITY {
        return sized;
    }
    let displayed = displayed_line_range(viewport, offset, height, spacing, line_count);
    if displayed.0 < 0 {
        return (-1, -1);
    }
    let first_displayed = displayed.0.max(first_sized);
    let last_displayed = displayed.1.min(last_sized);
    if first_displayed > last_displayed {
        return (-1, -1);
    }
    let stride = height + spacing;
    let viewport_lines = (viewport / stride) as i32;
    let viewport_frozen = (0.8 * viewport_lines as f64) as i32 + 1;
    let near_sized = first_displayed - first_sized;
    let far_sized = last_sized - last_displayed;
    let near_frozen = viewport_frozen
        .min(near_sized)
        .max((0.4 * near_sized as f64) as i32);
    let far_frozen = viewport_frozen
        .min(far_sized)
        .max((0.4 * far_sized as f64) as i32);
    (first_displayed - near_frozen, last_displayed + far_frozen)
}

fn sum_width(items: &[ItemMetrics], spacing: f64) -> f64 {
    items.iter().map(|item| item.width).sum::<f64>()
        + spacing * items.len().saturating_sub(1) as f64
}

/// Constraint-aware scale calculation. Clamped items are taken out of the
/// remaining budget and the factor is recomputed for the other items.
pub(super) fn line_scale(items: &[ItemMetrics], available: f64, spacing: f64, expand: bool) -> f64 {
    if items.is_empty() {
        return 1.0;
    }
    let mut budget = (available - spacing * items.len().saturating_sub(1) as f64).max(0.0);
    let mut desired = items.iter().map(|item| item.width).sum::<f64>();
    if desired <= 0.0 {
        return 1.0;
    }
    let mut ignored = vec![false; items.len()];
    let mut factor = 1.0;
    loop {
        if desired <= 0.0 {
            return if expand {
                factor
            } else if budget > 0.0 {
                1.0
            } else {
                0.0
            };
        }
        factor = budget / desired;
        let next = items.iter().enumerate().find(|(i, item)| {
            !ignored[*i]
                && if expand {
                    item.maximum < item.width * factor
                } else {
                    // ComputeLineShrinkFactor applies MaxWidth before
                    // MinWidth when probing constraints, unlike the initial
                    // arrange-width calculation. Preserve that ordering for
                    // metadata with conflicting limits.
                    item.minimum > item.width.max(item.minimum) * factor
                }
        });
        let Some((index, item)) = next else {
            return factor;
        };
        budget = (budget - if expand { item.maximum } else { item.minimum }).max(0.0);
        desired = (desired
            - if expand {
                item.width
            } else {
                item.width.max(item.minimum)
            })
        .max(0.0);
        ignored[index] = true;
        if budget <= 0.0 {
            return if expand { factor } else { 0.0 };
        }
    }
}

fn apply_scale(items: &[ItemMetrics], factor: f64) -> Vec<f64> {
    items
        .iter()
        .map(|item| {
            if factor < 1.0 {
                (item.width * factor).max(item.minimum)
            } else {
                (item.width * factor).min(item.maximum)
            }
            .max(0.0)
        })
        .collect()
}

pub(super) fn finish_line(
    items: &[ItemMetrics],
    index: i32,
    options: LineOptions,
    is_last: bool,
) -> LinedFlowLine {
    let desired = sum_width(items, options.spacing);
    let factor = if desired > options.available && options.available > 0.0 {
        line_scale(items, options.available, options.spacing, false)
    } else if desired < options.available && options.stretch && !is_last && desired > 0.0 {
        line_scale(items, options.available, options.spacing, true)
    } else {
        1.0
    };
    let widths = apply_scale(items, if factor == 0.0 { 1.0 } else { factor });
    let desired_width =
        widths.iter().sum::<f64>() + options.spacing * widths.len().saturating_sub(1) as f64;
    LinedFlowLine {
        index,
        first: items.first().map_or(0, |item| item.index),
        widths,
        desired_width,
    }
}

/// Fast path: compare the distortion needed to expand the current line with
/// the distortion needed to include the next item and shrink the line.
pub(super) fn compute_fast_path(items: &[ItemMetrics], options: LineOptions) -> Vec<LinedFlowLine> {
    let mut result = Vec::new();
    let mut start = 0;
    let mut current = 0.0;
    for index in 0..items.len() {
        let next_width =
            current + if index == start { 0.0 } else { options.spacing } + items[index].width;
        if index > start && next_width > options.available {
            let expand = line_scale(
                &items[start..index],
                options.available,
                options.spacing,
                true,
            );
            let shrink = line_scale(
                &items[start..=index],
                options.available,
                options.spacing,
                false,
            );
            if expand - 1.0 < 1.0 - shrink || shrink == 0.0 {
                result.push(finish_line(
                    &items[start..index],
                    result.len() as i32,
                    options,
                    false,
                ));
                start = index;
                current = items[index].width;
            } else {
                result.push(finish_line(
                    &items[start..=index],
                    result.len() as i32,
                    options,
                    index + 1 == items.len(),
                ));
                start = index + 1;
                current = 0.0;
            }
        } else {
            current = next_width;
        }
        // A full line is committed immediately, including one oversized
        // item. Otherwise a following zero-width item would be incorrectly
        // appended to a line already completed by the upstream fast path.
        if start <= index && current >= options.available {
            result.push(finish_line(
                &items[start..=index],
                result.len() as i32,
                options,
                index + 1 == items.len(),
            ));
            start = index + 1;
            current = 0.0;
        }
    }
    if start < items.len() {
        result.push(finish_line(
            &items[start..],
            result.len() as i32,
            options,
            true,
        ));
    }
    result
}

fn penalty(width: f64, available: f64, is_final: bool) -> f64 {
    let delta = width - available;
    if delta > 0.0 {
        delta * delta * delta
    } else if is_final {
        0.0
    } else {
        delta * delta
    }
}
fn improvement(
    amount: f64,
    available: f64,
    from: f64,
    to: f64,
    from_final: bool,
    to_final: bool,
) -> f64 {
    let contribution = |width: f64, final_line: bool| {
        let delta = width - available;
        if final_line && delta < 0.0 {
            0.0
        } else {
            delta * delta
        }
    };
    contribution(from, from_final) + contribution(to, to_final)
        - contribution(from - amount, from_final)
        - contribution(to + amount, to_final)
}

fn get_items_layout(
    items: &[ItemMetrics],
    options: LineOptions,
    adjusted: f64,
    average_width: f64,
    public_locks: &BTreeMap<i32, i32>,
    internal_locks: &BTreeMap<i32, i32>,
) -> Candidate {
    let line_count = options.line_count.min(items.len()).max(1);
    let last_line = options.first_line + line_count as i32 - 1;
    let direction = if options.forward { 1 } else { -1 };
    let mut counts = vec![0; line_count];
    let mut widths = vec![0.0; line_count];
    let mut starts = vec![0; line_count];
    let mut head = vec![None; line_count];
    let mut tail = vec![None; line_count];
    let mut line = if options.forward { 0 } else { line_count - 1 };
    let mut cumulative_lines = 0.0;
    let mut previous_movable = None;
    let source_final = items
        .last()
        .is_some_and(|item| item.index == options.total_item_count - 1);
    let in_lines = |line: i32| line >= options.first_line && line <= last_line;
    let lock_for = |index: i32| {
        public_locks
            .get(&index)
            .or_else(|| internal_locks.get(&index))
            .copied()
            .filter(|line| in_lines(*line))
    };
    // Merging two ordered lock maps preserves the upstream closest-item rule.
    let next_lock = |index: i32, include_internal: bool| {
        public_locks
            .iter()
            .chain(internal_locks.iter().filter(|_| include_internal))
            .filter(|(item, line)| in_lines(**line) && (**item - index) * direction > 0)
            .min_by_key(|(item, _)| (**item - index) * direction)
            .map(|(item, line)| (*item, *line))
    };
    for step in 0..items.len() {
        let position = if options.forward {
            step
        } else {
            items.len() - step - 1
        };
        let item = &items[position];
        let absolute_line = options.first_line + line as i32;
        let locked_line = lock_for(item.index);
        let upcoming_lock = next_lock(item.index, true);
        let same_line_locked_ahead =
            public_locks
                .iter()
                .chain(internal_locks.iter())
                .any(|(index, target)| {
                    *target == absolute_line && (*index - item.index) * direction >= 0
                });
        let remaining_lines = if options.forward {
            line_count - line - 1
        } else {
            line
        };
        let must_cumulate = counts[line] == 0 || same_line_locked_ahead || remaining_lines == 0;
        let can_cumulate = widths[line]
            + if counts[line] == 0 {
                0.0
            } else {
                options.spacing
            }
            + item.width
            <= adjusted;
        let mut must_not = false;
        let mut should_not = false;
        let mut should = false;
        if !must_cumulate {
            must_not = items.len() - step - 1 < remaining_lines
                || locked_line.is_some_and(|locked| locked != absolute_line)
                || upcoming_lock.is_some_and(|(next, next_line)| {
                    (next - item.index) * direction < (next_line - absolute_line) * direction
                });
            if !must_not {
                if can_cumulate && upcoming_lock.is_some() {
                    if let Some((next_index, next_line)) = next_lock(item.index, false) {
                        let lines_to_fill = (next_line - absolute_line) * direction;
                        if lines_to_fill <= options.lines_per_viewport {
                            let items_available =
                                (next_index - item.index) * direction + counts[line] as i32;
                            should_not = items_available
                                < (options.average_items_per_line * lines_to_fill as f64) as i32;
                        }
                    }
                }
                if average_width > 0.0 && ((can_cumulate && !should_not) || !can_cumulate) {
                    let processed_lines = if options.forward {
                        line + 1
                    } else {
                        line_count - line
                    };
                    let expected = average_width * processed_lines as f64;
                    let actual = cumulative_lines + widths[line] + item.width;
                    if can_cumulate {
                        should_not = actual - expected > options.wrap_multiplier * item.width;
                    } else {
                        should = expected - actual > options.wrap_multiplier * item.width;
                    }
                }
            }
        }
        if must_cumulate || ((can_cumulate || should) && !must_not && !should_not) {
            previous_movable = if locked_line.is_some() || step == 0 || step + 1 == items.len() {
                None
            } else {
                Some(position)
            };
            if counts[line] != 0 {
                widths[line] += options.spacing;
            }
            if counts[line] == 0 || !options.forward {
                starts[line] = position;
            }
            widths[line] += item.width;
            counts[line] += 1;
        } else {
            tail[line] = previous_movable.take();
            cumulative_lines += widths[line];
            line = if let Some(locked) = locked_line {
                (locked - options.first_line) as usize
            } else if step + 1 == items.len() {
                if options.forward {
                    line_count - 1
                } else {
                    0
                }
            } else {
                (line as i32 + direction) as usize
            };
            starts[line] = position;
            counts[line] = 1;
            widths[line] = item.width;
            let remaining_lines = if options.forward {
                line_count - line
            } else {
                line + 1
            };
            if locked_line.is_none()
                && step + 1 < items.len()
                && items.len() - step - 1 >= remaining_lines
            {
                head[line] = Some(position);
            }
        }
    }
    let mut candidate = Candidate {
        counts,
        widths,
        starts,
        available: adjusted,
        drawback: 0.0,
        smallest_head: f64::MAX,
        smallest_tail: f64::MAX,
        best_head: None,
        best_tail: None,
    };
    for line in 0..line_count {
        let from_final = line + 1 == line_count && source_final;
        candidate.drawback += penalty(candidate.widths[line], options.available, from_final);
        // Preserve the pinned implementation's vector-index comparison here:
        // GetItemDrawbackImprovement receives beginLineVectorIndex + offset
        // and compares that value with the global final-line index. The main
        // candidate drawback above independently uses the actual final item.
        let last_global_line = ((options.total_item_count - 1) as f64
            / options.average_items_per_line.max(1.0)) as i32;
        let from_equalization_final =
            options.begin_line_vector_index + line as i32 == last_global_line;
        if let Some(index) = head[line] {
            let width = items[index].width;
            if width > 0.0 {
                candidate.smallest_head = candidate.smallest_head.min(width);
            }
            let destination = line as i32 - direction;
            if width > 0.0
                && candidate.counts[line] > 1
                && destination >= 0
                && destination < line_count as i32
            {
                let to_final = options.begin_line_vector_index + destination == last_global_line;
                let gain = improvement(
                    width + options.spacing,
                    adjusted,
                    candidate.widths[line],
                    candidate.widths[destination as usize],
                    from_equalization_final,
                    to_final,
                );
                if gain > candidate.best_head.map_or(0.0, |(_, _, gain)| gain) {
                    candidate.best_head =
                        Some((items[index].index, options.first_line + destination, gain));
                }
            }
        }
        if let Some(index) = tail[line] {
            let width = items[index].width;
            if candidate.widths[line] <= adjusted && width > 0.0 {
                candidate.smallest_tail = candidate
                    .smallest_tail
                    .min(width + adjusted - candidate.widths[line]);
            }
            let destination = line as i32 + direction;
            if width > 0.0
                && candidate.counts[line] > 1
                && destination >= 0
                && destination < line_count as i32
            {
                let to_final = options.begin_line_vector_index + destination == last_global_line;
                let gain = improvement(
                    width + options.spacing,
                    adjusted,
                    candidate.widths[line],
                    candidate.widths[destination as usize],
                    from_equalization_final,
                    to_final,
                );
                if gain > candidate.best_tail.map_or(0.0, |(_, _, gain)| gain) {
                    candidate.best_tail =
                        Some((items[index].index, options.first_line + destination, gain));
                }
            }
        }
    }
    candidate
}

/// The six-phase regular-path optimizer, including the 30% candidate window,
/// asymmetric overflow cost, and internal locks for head/tail equalization.
pub(super) fn compute_regular_path(
    items: &[ItemMetrics],
    options: LineOptions,
    locks: &BTreeMap<i32, i32>,
) -> Vec<LinedFlowLine> {
    if items.is_empty() {
        return Vec::new();
    }
    let mut internal = BTreeMap::new();
    let initial = get_items_layout(items, options, options.available, -1.0, locks, &internal);
    let average_width = initial.widths.iter().sum::<f64>() / initial.widths.len() as f64;
    let mut best = initial;
    let mut processed = vec![options.available];
    if options.line_count > 1 && average_width > 0.0 {
        let mut pending = vec![average_width];
        let mut lower = average_width;
        let mut upper = average_width;
        while !pending.is_empty() {
            pending.sort_by(f64::total_cmp);
            let adjusted = pending.remove(0);
            let candidate =
                get_items_layout(items, options, adjusted, average_width, locks, &internal);
            processed.push(adjusted);
            let movable = candidate.counts.iter().any(|count| *count > 1);
            if movable
                && candidate.smallest_head != f64::MAX
                && candidate.smallest_head > 0.0
                && candidate.widths.iter().any(|width| *width > adjusted)
            {
                let next = adjusted + options.spacing + candidate.smallest_head;
                if (next - average_width).abs() < 0.3 * average_width
                    && next > upper
                    && !processed.contains(&next)
                    && !pending.contains(&next)
                {
                    upper = next;
                    pending.push(next);
                }
            }
            if movable && candidate.smallest_tail != f64::MAX && candidate.smallest_tail > 0.0 {
                let next = adjusted - options.spacing - candidate.smallest_tail;
                if next > 0.0
                    && (next - average_width).abs() < 0.3 * average_width
                    && next < lower
                    && !processed.contains(&next)
                    && !pending.contains(&next)
                {
                    lower = next;
                    pending.push(next);
                }
            }
            if candidate.drawback < best.drawback {
                best = candidate;
            }
        }
        let midpoint = (best.available + average_width) / 2.0;
        if !processed.contains(&midpoint) {
            let candidate =
                get_items_layout(items, options, midpoint, average_width, locks, &internal);
            if candidate.drawback < best.drawback {
                best = candidate;
            }
        }
        let mut current = best.clone();
        let mut no_improvement = 0;
        loop {
            let next = match (current.best_head, current.best_tail) {
                (Some(head), Some(tail)) => {
                    if head.2 > tail.2 {
                        Some(head)
                    } else if tail.2 > head.2 {
                        Some(tail)
                    } else {
                        None
                    }
                }
                (head, tail) => head.or(tail),
            };
            let Some((index, target_line, gain)) = next else {
                break;
            };
            if gain <= 0.0
                || internal.get(&index) == Some(&target_line)
                || locks.contains_key(&index)
            {
                break;
            }
            let total_lines = ((options.total_item_count - 1) as f64
                / options.average_items_per_line.max(1.0)) as i32
                + 1;
            let last_line = options.first_line + best.counts.len() as i32 - 1;
            let first_item = items.first().expect("nonempty item range").index;
            let last_item = items.last().expect("nonempty item range").index;
            if target_line > index
                || total_lines - target_line > options.total_item_count - index
                || (options.first_line - target_line).abs() > (first_item - index).abs()
                || (last_line - target_line).abs() > (last_item - index).abs()
            {
                break;
            }
            // A new internal lock must leave one item for every intervening line.
            let near = locks
                .range(..index)
                .chain(internal.range(..index))
                .max_by_key(|(index, _)| **index);
            let far = locks
                .range(index + 1..)
                .chain(internal.range(index + 1..))
                .min_by_key(|(index, _)| **index);
            if near.is_some_and(|(near, line)| {
                target_line < *line || index - *near < target_line - *line
            }) || far.is_some_and(|(far, line)| {
                target_line > *line || *far - index < *line - target_line
            }) {
                break;
            }
            internal.insert(index, target_line);
            let candidate = get_items_layout(
                items,
                options,
                current.available,
                average_width,
                locks,
                &internal,
            );
            if candidate.drawback < best.drawback {
                best = candidate.clone();
                no_improvement = 0;
            } else {
                no_improvement += 1;
            }
            current = candidate;
            if no_improvement >= best.counts.len() / 2 {
                break;
            }
        }
    }
    let mut result = Vec::with_capacity(best.counts.len());
    for (line, count) in best.counts.iter().enumerate() {
        if *count == 0 {
            continue;
        }
        let start = best.starts[line];
        let end = start + count;
        result.push(finish_line(
            &items[start..end],
            options.first_line + line as i32,
            options,
            items[end - 1].index == options.total_item_count - 1,
        ));
    }
    result
}
