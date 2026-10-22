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
        low.min(self.count - 1)
    }
}

/// Tab expansion keeps both directions of the UTF-16 index mapping. Hit
/// testing inside the second half of a tab selects its trailing boundary.
pub(crate) struct DisplayLine {
    pub text: Vec<u16>,
    positions: Vec<(usize, usize)>,
}

impl DisplayLine {
    pub fn new(text: &[u16], tab_size: usize) -> Self {
        let mut result = Self::empty();
        result.push_text(text, 0, tab_size);
        result
    }

    pub fn empty() -> Self {
        Self {
            text: Vec::new(),
            positions: Vec::new(),
        }
    }

    pub fn push_text(&mut self, text: &[u16], document_offset: usize, tab_size: usize) {
        let tab_size = tab_size.clamp(1, 256);
        for (i, &unit) in text.iter().enumerate() {
            self.push_position(document_offset + i);
            if unit == 9 {
                let spaces = tab_size - self.text.len() % tab_size;
                self.text.resize(self.text.len() + spaces, 32);
            } else {
                self.text.push(unit);
            }
        }
        self.push_position(document_offset + text.len());
    }

    pub fn push_fold(&mut self, start: usize, end: usize, title: &str) {
        self.push_position(start);
        self.text.extend(title.encode_utf16());
        self.push_position(end);
    }

    /// Projects IME composition without changing document storage. Both the
    /// inserted composition and the removed selection still map to document
    /// boundaries; only its visual text is transient.
    pub fn apply_preedit(
        &mut self,
        start: usize,
        end: usize,
        text: &str,
        virtual_padding: usize,
    ) -> Result<(usize, usize), ferroui_edit_core::editing::EditError> {
        use ferroui_edit_core::editing::EditError;
        if end < start {
            return Err(EditError::InvalidSelection);
        }
        let first = self.visual_offset(start);
        let last = self.visual_offset(end);
        let composition_length = text.encode_utf16().count();
        let length = virtual_padding
            .checked_add(composition_length)
            .ok_or(EditError::VirtualSpaceTooLarge)?;
        let projected_length = self
            .text
            .len()
            .checked_sub(last - first)
            .and_then(|length_without_selection| length_without_selection.checked_add(length))
            .filter(|&length| length <= i32::MAX as usize)
            .ok_or(EditError::VirtualSpaceTooLarge)?;
        let mut units = Vec::new();
        units
            .try_reserve_exact(length)
            .map_err(|_| EditError::VirtualSpaceTooLarge)?;
        units.resize(virtual_padding, 32);
        units.extend(text.encode_utf16());
        let mut positions = Vec::new();
        positions
            .try_reserve_exact(self.positions.len().saturating_add(2))
            .map_err(|_| EditError::VirtualSpaceTooLarge)?;
        self.text
            .try_reserve(projected_length.saturating_sub(self.text.len()))
            .map_err(|_| EditError::VirtualSpaceTooLarge)?;
        self.text.splice(first..last, units);
        positions.extend(self.positions.iter().copied().filter(|p| p.0 < start));
        positions.push((start, first));
        positions.push((end, first + length));
        positions.extend(
            self.positions
                .iter()
                .filter(|p| p.0 > end)
                .map(|&(d, v)| (d, v - (last - first) + length)),
        );
        self.positions = positions;
        Ok((first + virtual_padding, composition_length))
    }

    fn push_position(&mut self, offset: usize) {
        let position = (offset, self.text.len());
        if self.positions.last() != Some(&position) {
            self.positions.push(position);
        }
    }

    pub fn visual_offset(&self, document_offset: usize) -> usize {
        let index = self.positions.partition_point(|p| p.0 <= document_offset);
        if index == 0 {
            0
        } else {
            self.positions[index - 1].1
        }
    }

    pub fn document_offset(&self, visual_offset: usize) -> usize {
        let right = self.positions.partition_point(|p| p.1 < visual_offset);
        if right == 0 {
            return 0;
        }
        if right >= self.positions.len() {
            return self.positions.last().map_or(0, |p| p.0);
        }
        let left = self.positions[right - 1];
        let right = self.positions[right];
        if visual_offset - left.1 < right.1 - visual_offset {
            left.0
        } else {
            right.0
        }
    }

    pub fn whitespace_markers(&self) -> impl Iterator<Item = (usize, bool)> + '_ {
        self.positions.windows(2).filter_map(|pair| {
            let (a, b) = (pair[0], pair[1]);
            if b.0 != a.0 + 1 || self.text.get(a.1) != Some(&32) {
                return None;
            }
            Some((a.1, b.1 > a.1 + 1))
        })
    }
}

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
    fn tabs_roundtrip_utf16_offsets_including_surrogate_pairs() {
        let input: Vec<u16> = "a\t😀\tz".encode_utf16().collect();
        let display = DisplayLine::new(&input, 4);
        assert_eq!(String::from_utf16_lossy(&display.text), "a   😀  z");
        for offset in 0..=input.len() {
            assert_eq!(
                display.document_offset(display.visual_offset(offset)),
                offset
            );
        }
        assert_eq!(display.document_offset(2), 1);
        assert_eq!(display.document_offset(3), 2);
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

    #[test]
    fn folded_display_maps_both_boundaries_without_copying_hidden_text() {
        let mut display = DisplayLine::new(&[65, 9], 4);
        display.push_fold(2, 1_000_002, "...");
        display.push_text(&[66], 1_000_002, 4);
        assert_eq!(String::from_utf16_lossy(&display.text), "A   ...B");
        assert_eq!(display.visual_offset(1_000_002), 7);
        assert_eq!(display.document_offset(5), 2);
        assert_eq!(display.document_offset(6), 1_000_002);
        assert!(display.positions.len() < 10);
    }

    #[test]
    fn preedit_replaces_visual_selection_and_retains_document_mapping() {
        let mut display = DisplayLine::new(&"a😀z".encode_utf16().collect::<Vec<_>>(), 4);
        assert_eq!(display.apply_preedit(1, 3, "日本語", 0).unwrap(), (1, 3));
        assert_eq!(String::from_utf16_lossy(&display.text), "a日本語z");
        assert_eq!(display.visual_offset(4), 5);
        assert_eq!(display.document_offset(4), 3);
        assert_eq!(display.document_offset(5), 4);
    }

    #[test]
    fn virtual_preedit_padding_maps_back_to_eol_and_rejects_overflow_atomically() {
        let mut display = DisplayLine::new(&[65], 4);
        let old_text = display.text.clone();
        let old_positions = display.positions.clone();
        assert!(display.apply_preedit(1, 1, "X", usize::MAX).is_err());
        assert_eq!(display.text, old_text);
        assert_eq!(display.positions, old_positions);
        assert_eq!(display.apply_preedit(1, 1, "😀", 3).unwrap(), (4, 2));
        assert_eq!(String::from_utf16_lossy(&display.text), "A   😀");
        for index in 1..=6 {
            assert_eq!(display.document_offset(index), 1);
        }
    }
}
