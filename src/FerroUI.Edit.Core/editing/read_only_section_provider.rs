use crate::document::{AnchorMovementType, DocumentError, SimpleSegment, TextAnchor, TextDocument};
use std::cell::RefCell;

pub trait IReadOnlySectionProvider {
    fn can_insert(&self, offset: usize) -> bool;
    fn get_deletable_segments(&self, segment: SimpleSegment) -> Vec<SimpleSegment>;
}

#[derive(Default)]
pub struct NoReadOnlySections;
impl IReadOnlySectionProvider for NoReadOnlySections {
    fn can_insert(&self, _: usize) -> bool {
        true
    }
    fn get_deletable_segments(&self, segment: SimpleSegment) -> Vec<SimpleSegment> {
        vec![segment]
    }
}

/// Protected segments follow document edits. Insertions at either boundary are
/// allowed; insertion strictly inside a protected segment is rejected.
pub struct TextSegmentReadOnlySectionProvider {
    document: TextDocument,
    segments: RefCell<Vec<(TextAnchor, TextAnchor)>>,
}
impl TextSegmentReadOnlySectionProvider {
    pub fn new(document: TextDocument) -> Self {
        Self {
            document,
            segments: RefCell::new(Vec::new()),
        }
    }
    pub fn add(&self, offset: usize, length: usize) -> Result<(), DocumentError> {
        let start = self.document.create_anchor(offset)?;
        let end = self.document.create_anchor(offset.saturating_add(length))?;
        start.set_movement_type(AnchorMovementType::AfterInsertion);
        end.set_movement_type(AnchorMovementType::BeforeInsertion);
        start.set_survive_deletion(true);
        end.set_survive_deletion(true);
        self.segments.borrow_mut().push((start, end));
        Ok(())
    }
    pub fn clear(&self) {
        self.segments.borrow_mut().clear();
    }
    pub fn segments(&self) -> Vec<SimpleSegment> {
        let mut segments: Vec<_> = self
            .segments
            .borrow()
            .iter()
            .filter_map(|(s, e)| {
                let (s, e) = (s.offset().ok()?, e.offset().ok()?);
                Some(SimpleSegment::new(s, e.saturating_sub(s)))
            })
            .collect();
        segments.sort_by_key(|s| s.offset);
        segments
    }
}
impl IReadOnlySectionProvider for TextSegmentReadOnlySectionProvider {
    fn can_insert(&self, offset: usize) -> bool {
        self.segments()
            .iter()
            .all(|s| offset <= s.offset || offset >= s.end_offset())
    }
    fn get_deletable_segments(&self, segment: SimpleSegment) -> Vec<SimpleSegment> {
        if segment.length == 0 {
            return if self.can_insert(segment.offset) {
                vec![segment]
            } else {
                Vec::new()
            };
        }
        let mut result = Vec::new();
        let mut cursor = segment.offset;
        let end = segment.end_offset();
        for protected in self.segments() {
            if protected.end_offset() <= cursor || protected.offset >= end {
                continue;
            }
            if protected.offset > cursor {
                result.push(SimpleSegment::new(cursor, protected.offset - cursor));
            }
            cursor = cursor.max(protected.end_offset()).min(end);
        }
        if cursor < end {
            result.push(SimpleSegment::new(cursor, end - cursor));
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn readonly_intersections_and_boundaries() {
        let document = TextDocument::from_text("0123456789");
        let provider = TextSegmentReadOnlySectionProvider::new(document.clone());
        provider.add(2, 3).unwrap();
        provider.add(4, 3).unwrap();
        assert!(provider.can_insert(2));
        assert!(!provider.can_insert(4));
        assert!(provider.can_insert(7));
        assert_eq!(
            provider.get_deletable_segments(SimpleSegment::new(0, 10)),
            vec![SimpleSegment::new(0, 2), SimpleSegment::new(7, 3)]
        );
        document.insert(2, "abc").unwrap();
        assert!(provider.can_insert(3));
        assert!(!provider.can_insert(6));
    }
}
