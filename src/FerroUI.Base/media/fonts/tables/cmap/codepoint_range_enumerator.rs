use std::rc::Rc;

use super::cmap_format::CmapFormat;
use super::cmap_format12_or13_table::CmapFormat12Or13Table;
use super::cmap_format4_table::CmapFormat4Table;
use super::codepoint_range::CodepointRange;

/// Enumerates the code point ranges a character map covers.
///
/// Usable either through [`move_next`](CodepointRangeEnumerator::move_next) /
/// [`current`](CodepointRangeEnumerator::current) or as an [`Iterator`].
#[derive(Clone)]
pub struct CodepointRangeEnumerator {
    format: CmapFormat,
    f4: Option<Rc<CmapFormat4Table>>,
    f12_or_13: Option<Rc<CmapFormat12Or13Table>>,
    index: i32,
    current: CodepointRange,
}

impl CodepointRangeEnumerator {
    pub(crate) fn new(
        format: CmapFormat,
        f4: Option<Rc<CmapFormat4Table>>,
        f12_or_13: Option<Rc<CmapFormat12Or13Table>>,
    ) -> Self {
        Self { format, f4, f12_or_13, index: -1, current: CodepointRange::default() }
    }

    #[inline]
    pub fn current(&self) -> CodepointRange {
        self.current
    }

    #[inline]
    pub fn move_next(&mut self) -> bool {
        self.index = self.index.saturating_add(1);

        let range = if self.format == CmapFormat::Format4 {
            self.f4.as_ref().and_then(|table| table.try_get_range(self.index))
        } else if self.format == CmapFormat::Format12 || self.format == CmapFormat::Format13 {
            self.f12_or_13.as_ref().and_then(|table| table.try_get_range(self.index))
        } else {
            None
        };

        self.current = range.unwrap_or_default();

        range.is_some()
    }
}

impl Iterator for CodepointRangeEnumerator {
    type Item = CodepointRange;

    fn next(&mut self) -> Option<CodepointRange> {
        if self.move_next() {
            Some(self.current)
        } else {
            None
        }
    }
}
