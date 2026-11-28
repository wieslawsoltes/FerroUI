use super::{AtSpiCoordinateHelper, AtSpiNode, ITextProvider, ITextRangeProvider};
use ferroui_base::Rect;
use ferroui_base::media::text_formatting::unicode::GraphemeEnumerator;
use std::ops::Range;

#[derive(Clone, Copy)]
pub(super) struct Clip(u32);

impl Clip {
    pub(super) fn new(flags: u32) -> Option<Self> {
        (flags <= 3).then_some(Self(flags))
    }

    fn allows(self, low: f64, high: f64, minimum: f64, maximum: f64) -> bool {
        (self.0 & 1 == 0 || low >= minimum) && (self.0 & 2 == 0 || high <= maximum)
    }
}

pub(super) struct BoundsQuery<'a> {
    pub(super) node: &'a AtSpiNode,
    pub(super) provider: &'a dyn ITextProvider,
    pub(super) area: Rect,
    pub(super) x_clip: Clip,
    pub(super) y_clip: Clip,
}

impl BoundsQuery<'_> {
    pub(super) fn offsets(&self) -> Option<Vec<Range<usize>>> {
        let mut included = Vec::new();
        for visible in self.provider.get_visible_ranges().ok()? {
            let Some(span) = visible.text_offsets().ok().flatten() else { continue };
            if span.is_empty() {
                continue;
            }
            let units: Vec<_> = visible.get_text(None).ok()?.encode_utf16().collect();
            if units.len() != span.len() {
                return None;
            }
            let mut graphemes = GraphemeEnumerator::new(&units);
            let mut boundaries = vec![span.start];
            while let Some(grapheme) = graphemes.move_next() {
                let start = span.start + grapheme.offset();
                let end = start + grapheme.length();
                if matches!(units[grapheme.offset()], 10 | 13) {
                    self.collect_run(&boundaries, false, &mut included)?;
                    self.collect_run(&[start, end], true, &mut included)?;
                    boundaries.clear();
                }
                boundaries.push(end);
            }
            self.collect_run(&boundaries, false, &mut included)?;
        }

        included.sort_unstable_by_key(|span| (span.start, span.end));
        let mut merged: Vec<Range<usize>> = Vec::new();
        for span in included {
            if let Some(last) = merged.last_mut().filter(|last| span.start <= last.end) {
                last.end = last.end.max(span.end);
            } else {
                merged.push(span);
            }
        }
        Some(merged)
    }

    fn collect_run(&self, boundaries: &[usize], delimiter: bool, included: &mut Vec<Range<usize>>) -> Option<()> {
        if boundaries.len() < 2 {
            return Some(());
        }
        let range = self.provider.range_from_offsets(boundaries[0]..*boundaries.last()?).ok().flatten()?;
        self.collect(&*range, boundaries, delimiter, included)
    }

    fn collect(&self, range: &dyn ITextRangeProvider, boundaries: &[usize], delimiter: bool, included: &mut Vec<Range<usize>>) -> Option<()> {
        // The native range's selection geometry may also contain a caret
        // rectangle on the following row. The delimiter belongs to its own row.
        let rectangles: Vec<_> = range.get_bounding_rectangles().ok()?.into_iter()
            .take(if delimiter { 1 } else { usize::MAX })
            .map(|rect| AtSpiCoordinateHelper::to_screen(self.node, rect))
            .filter(|rect| !rect.is_empty())
            .collect();
        if !rectangles.iter().any(|rect| !rect.intersect(self.area).is_empty()) {
            return Some(());
        }
        let span = boundaries[0]..*boundaries.last()?;
        if rectangles.iter().all(|rect| self.area.contains_rect(*rect)) {
            included.push(span);
        } else if boundaries.len() == 2 {
            if rectangles.iter().any(|rect| self.includes(*rect)) {
                included.push(span);
            }
        } else {
            // Only query individual clusters along intersected edges. A
            // fully enclosed or excluded subrange uses one geometry query.
            let mid = boundaries.len() / 2;
            for part in [&boundaries[..=mid], &boundaries[mid..]] {
                self.collect_run(part, false, included)?;
            }
        }
        Some(())
    }

    fn includes(&self, rect: Rect) -> bool {
        !rect.intersect(self.area).is_empty()
            && self.x_clip.allows(rect.left(), rect.right(), self.area.left(), self.area.right())
            && self.y_clip.allows(rect.top(), rect.bottom(), self.area.top(), self.area.bottom())
    }
}

#[cfg(test)]
mod tests {
    use super::Clip;

    #[test]
    fn each_axis_can_reject_either_partial_edge() {
        for flags in 0..=3 {
            let clip = Clip::new(flags).unwrap();
            assert!(clip.allows(1.0, 3.0, 1.0, 3.0));
            assert_eq!(clip.allows(0.0, 2.0, 1.0, 3.0), flags & 1 == 0);
            assert_eq!(clip.allows(2.0, 4.0, 1.0, 3.0), flags & 2 == 0);
        }
        assert!(Clip::new(4).is_none());
    }
}
