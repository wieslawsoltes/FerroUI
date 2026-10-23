//! Viewport metrics and document/display offset conversion. These structures
//! have no platform resources and can be exercised without a render backend.

use std::collections::BTreeMap;

/// Sparse Fenwick tree: unmeasured logical lines take the default height.
/// Storage is proportional to measured lines, not the document's line count.
pub(crate) struct HeightTree {
    count: usize,
    default_height: f64,
    heights: BTreeMap<usize, f64>,
    sums: BTreeMap<usize, f64>,
    collapsed: Vec<(usize, usize, usize)>,
}

impl HeightTree {
    pub fn new(count: usize, default_height: f64) -> Self {
        Self {
            count: count.max(1),
            default_height,
            heights: BTreeMap::new(),
            sums: BTreeMap::new(),
            collapsed: Vec::new(),
        }
    }

    pub fn reset(&mut self, count: usize, default_height: f64) {
        *self = Self::new(count, default_height);
    }

    pub fn set_height(&mut self, line: usize, height: f64) {
        if line >= self.count
            || self.next_visible(line) != line
            || !height.is_finite()
            || height <= 0.0
        {
            return;
        }
        let old = self
            .heights
            .insert(line, height)
            .unwrap_or(self.default_height);
        let delta = height - old;
        if delta.abs() < 0.001 {
            return;
        }
        let mut i = line + 1;
        while i <= self.count {
            *self.sums.entry(i).or_default() += delta;
            let Some(next) = i.checked_add(i & i.wrapping_neg()) else {
                break;
            };
            i = next;
        }
    }

    pub fn top(&self, line: usize) -> f64 {
        let mut i = line.min(self.count);
        let mut value = (i - self.hidden_before(i)) as f64 * self.default_height;
        while i > 0 {
            value += self.sums.get(&i).copied().unwrap_or_default();
            i &= i - 1;
        }
        value
    }

    pub fn total(&self) -> f64 {
        self.top(self.count)
    }

    /// Collapsed ranges are half-open logical line intervals. A fold hiding a
    /// million lines needs one interval rather than a million zero heights.
    pub fn set_collapsed(&mut self, ranges: &[(usize, usize)]) {
        self.heights.clear();
        self.sums.clear();
        self.collapsed.clear();
        let mut sorted = ranges.to_vec();
        sorted.sort_unstable();
        for (start, end) in sorted {
            let end = end.min(self.count);
            if start >= end {
                continue;
            }
            if let Some(last) = self.collapsed.last_mut() {
                if start <= last.1 {
                    last.1 = last.1.max(end);
                    continue;
                }
            }
            self.collapsed.push((start, end, 0));
        }
        let mut total = 0;
        for range in &mut self.collapsed {
            range.2 = total;
            total += range.1 - range.0;
        }
    }

    /// Reconcile newly discovered multiline visual elements without discarding
    /// the already shaped heights of all other visible lines.
    pub fn set_collapsed_preserving_heights(&mut self, ranges: &[(usize, usize)]) {
        let measured = std::mem::take(&mut self.heights);
        self.set_collapsed(ranges);
        for (line, height) in measured {
            self.set_height(line, height);
        }
    }

    fn hidden_before(&self, line: usize) -> usize {
        let index = self.collapsed.partition_point(|r| r.0 < line);
        if index == 0 {
            return 0;
        }
        let (start, end, before) = self.collapsed[index - 1];
        before + line.min(end) - start
    }

    pub fn next_visible(&self, line: usize) -> usize {
        let index = self.collapsed.partition_point(|r| r.0 <= line);
        if index == 0 {
            return line;
        }
        let (_, end, _) = self.collapsed[index - 1];
        line.max(end)
    }

    pub fn visible_owner(&self, line: usize) -> usize {
        let index = self.collapsed.partition_point(|r| r.0 <= line);
        if index == 0 {
            return line;
        }
        let (start, end, _) = self.collapsed[index - 1];
        if line < end {
            start.saturating_sub(1)
        } else {
            line
        }
    }

    pub fn line_at(&self, y: f64) -> usize {
        let (mut low, mut high) = (0, self.count);
        while low < high {
            let mid = low + (high - low) / 2;
            if self.top(mid + 1) <= y {
                low = mid + 1;
            } else {
                high = mid;
            }
        }
        self.visible_owner(low.min(self.count - 1))
    }
}

/// Tab expansion keeps both directions of the UTF-16 index mapping. Hit

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sparse_heights_support_wrap_and_deep_scrolling() {
        let mut heights = HeightTree::new(1_000_000, 20.0);
        heights.set_height(2, 60.0);
        heights.set_height(900_000, 40.0);
        assert_eq!(heights.top(3), 100.0);
        assert_eq!(heights.line_at(99.0), 2);
        assert_eq!(heights.line_at(100.0), 3);
        assert_eq!(heights.total(), 20_000_060.0);
        assert!(heights.sums.len() < 50);
    }

    #[test]
    fn hits_past_a_collapsed_eof_resolve_to_the_visible_owner() {
        let mut heights = HeightTree::new(5, 20.0);
        heights.set_height(0, 40.0);
        heights.set_height(4, 60.0);
        heights.set_collapsed_preserving_heights(&[(1, 5)]);
        assert_eq!(heights.total(), 40.0);
        assert_eq!(heights.line_at(39.0), 0);
        assert_eq!(heights.line_at(40.0), 0);
        assert_eq!(heights.line_at(500.0), 0);
        assert_eq!(heights.heights.len(), 1);
    }

    #[test]
    fn collapsed_ranges_do_not_allocate_per_hidden_line() {
        let mut heights = HeightTree::new(1_000_000, 20.0);
        heights.set_collapsed(&[(1, 900_000), (4, 8), (900_010, 999_999)]);
        assert_eq!(heights.total(), 240.0);
        assert_eq!(heights.line_at(20.0), 900_000);
        assert_eq!(heights.line_at(220.0), 999_999);
        assert_eq!(heights.next_visible(5), 900_000);
        assert_eq!(heights.collapsed.len(), 2);
        heights.set_height(900_001, 40.0);
        assert_eq!(heights.line_at(79.0), 900_001);
        assert_eq!(heights.line_at(80.0), 900_002);
    }
}
