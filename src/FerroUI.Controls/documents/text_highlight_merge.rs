//! Priority-preserving interval splitting from WinUI TextHighlightMerge.cpp.
//! Source endpoints are inclusive; FerroUI uses half-open UTF-16 ranges.

use ferroui_base::media::IBrush;
use std::collections::BTreeMap;
use std::fmt;
use std::ops::Range;
use std::rc::Rc;

/// One visual layer after resolving its theme defaults.
#[derive(Clone, PartialEq)]
pub struct TextHighlightRegion {
    pub range: Range<usize>,
    pub foreground: Option<Rc<dyn IBrush>>,
    pub background: Option<Rc<dyn IBrush>>,
}

impl TextHighlightRegion {
    pub fn new(
        range: Range<usize>,
        foreground: Option<Rc<dyn IBrush>>,
        background: Option<Rc<dyn IBrush>>,
    ) -> Self {
        Self {
            range,
            foreground,
            background,
        }
    }
}

impl fmt::Debug for TextHighlightRegion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TextHighlightRegion")
            .field("range", &self.range)
            .field(
                "foreground",
                &self
                    .foreground
                    .as_ref()
                    .and_then(|b| b.as_solid_color_brush())
                    .map(|b| b.color()),
            )
            .field(
                "background",
                &self
                    .background
                    .as_ref()
                    .and_then(|b| b.as_solid_color_brush())
                    .map(|b| b.color()),
            )
            .finish()
    }
}

/// Splits overlapping layers into ordered, disjoint regions.
///
/// Later additions replace the complete foreground/background pair. Selection
/// therefore has highest priority when a view adds it after its highlighters.
/// Adjacent regions remain separate, matching the source merge algorithm.
#[derive(Clone, Debug, Default)]
pub struct TextHighlightMerge {
    regions: BTreeMap<usize, TextHighlightRegion>,
}

impl TextHighlightMerge {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn from_regions(regions: impl IntoIterator<Item = TextHighlightRegion>) -> Self {
        let mut result = Self::new();
        for region in regions {
            result.add_region(region);
        }
        result
    }

    pub fn merge(&mut self, region: TextHighlightRegion) {
        self.add_region(region);
    }

    pub fn add_region(&mut self, region: TextHighlightRegion) {
        if region.range.is_empty() {
            return;
        }
        let start = region.range.start;
        let end = region.range.end;

        let preceding = self
            .regions
            .range(..start)
            .next_back()
            .filter(|(_, previous)| previous.range.end > start)
            .map(|(key, _)| *key);
        if let Some(key) = preceding {
            if let Some(mut previous) = self.regions.remove(&key) {
                if previous.range.end > end {
                    let mut tail = previous.clone();
                    tail.range.start = end;
                    self.regions.insert(end, tail);
                }
                previous.range.end = start;
                self.regions.insert(previous.range.start, previous);
            }
        }

        let covered: Vec<_> = self
            .regions
            .range(start..end)
            .map(|(key, _)| *key)
            .collect();
        for key in covered {
            if let Some(mut previous) = self.regions.remove(&key) {
                if previous.range.end > end {
                    previous.range.start = end;
                    self.regions.insert(end, previous);
                }
            }
        }
        self.regions.insert(start, region);
    }

    pub fn iter(
        &self,
    ) -> impl DoubleEndedIterator<Item = &TextHighlightRegion> + ExactSizeIterator {
        self.regions.values()
    }
    pub fn regions(&self) -> Vec<TextHighlightRegion> {
        self.regions.values().cloned().collect()
    }
    pub fn into_regions(self) -> Vec<TextHighlightRegion> {
        self.regions.into_values().collect()
    }
    pub fn is_empty(&self) -> bool {
        self.regions.is_empty()
    }
    pub fn len(&self) -> usize {
        self.regions.len()
    }
    pub fn clear(&mut self) {
        self.regions.clear();
    }
}
