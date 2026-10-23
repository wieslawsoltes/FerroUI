//! Private endpoint-pair tracking for the independently measured range SetText
//! origin. The document's public change records and ordinary TextPointer
//! affinity remain based on the visible minimal replacement.
use crate::documents::{
    DocumentError, DocumentTextChange, RichTextDocument, TextGravity, TextPosition, TextRange,
};
use ferroui_base::reactive::IDisposable;
use std::cell::Cell;
use std::ops::Range;
use std::rc::Rc;

pub(super) struct RangePosition {
    position: Rc<Cell<TextRange>>,
    // Keep this counter alive with its coordinate identity. Failed edit groups
    // restore only identities which participated in an owned paired mutation.
    _paired_generation: Rc<Cell<u64>>,
    subscription: Rc<dyn IDisposable>,
}

impl RangePosition {
    pub(super) fn new(
        document: &RichTextDocument,
        range: Range<usize>,
    ) -> Result<Self, DocumentError> {
        document.validate_range(range.clone())?;
        let position = Rc::new(Cell::new(TextRange::new(
            TextPosition::new(
                range.start,
                if range.is_empty() {
                    TextGravity::Forward
                } else {
                    TextGravity::Backward
                },
            ),
            TextPosition::new(range.end, TextGravity::Forward),
        )));
        let paired_generation = Rc::new(Cell::new(0u64));
        let weak_position = Rc::downgrade(&position);
        let weak_generation = Rc::downgrade(&paired_generation);
        let subscription = document.range_position_changed(
            &position,
            &paired_generation,
            move |change, operation| {
                let (Some(position), Some(generation)) =
                    (weak_position.upgrade(), weak_generation.upgrade())
                else {
                    return;
                };
                let before = position.get();
                let after = if let Some(operation) = operation {
                    let mapped = map_set_text_pair(before.range(), operation);
                    // Affinity is retained for subsequent unmeasured origins.
                    // Even an unchanged pair participated in this semantic edit.
                    generation.set(generation.get().wrapping_add(1));
                    TextRange::new(
                        TextPosition {
                            offset: mapped.start,
                            ..before.start
                        },
                        TextPosition {
                            offset: mapped.end,
                            ..before.end
                        },
                    )
                } else {
                    TextRange::new(
                        change.map_position(before.start),
                        change.map_position(before.end),
                    )
                };
                position.set(after);
            },
        );
        Ok(Self {
            position,
            _paired_generation: paired_generation,
            subscription,
        })
    }

    pub(super) fn range(&self) -> Range<usize> {
        self.position.get().range()
    }
}

impl Drop for RangePosition {
    fn drop(&mut self) {
        self.subscription.dispose();
    }
}

/// Measured single-edit passive pair behavior. A caret stays before an insertion
/// at itself, and replacement coverage is evaluated for the whole range rather
/// than applying independent endpoint gravity. Only explicit plain range
/// SetText supplies this operation span. Other origins use their existing map.
fn map_set_text_pair(range: Range<usize>, change: &DocumentTextChange) -> Range<usize> {
    let edit = &change.range;
    let inserted_end = edit.start + change.inserted_length;
    let after = |position: usize| position - edit.len() + change.inserted_length;
    if range.is_empty() {
        let offset = if range.start <= edit.start {
            range.start
        } else if range.start <= edit.end {
            edit.start
        } else {
            after(range.start)
        };
        return offset..offset;
    }
    if range.end <= edit.start {
        return range;
    }
    if !edit.is_empty() && edit.end <= range.start {
        return after(range.start)..after(range.end);
    }
    if edit.is_empty() {
        return if edit.start < range.start {
            after(range.start)..after(range.end)
        } else {
            range.start..after(range.end)
        };
    }
    if edit.start <= range.start && range.end <= edit.end {
        return if range == *edit {
            edit.start..inserted_end
        } else {
            edit.start..edit.start
        };
    }
    if edit.start < range.start {
        return inserted_end..after(range.end);
    }
    if range.end < edit.end {
        return range.start..edit.start;
    }
    range.start..after(range.end)
}

#[cfg(test)]
#[path = "rich_edit_text_range_tracking_tests.rs"]
mod tests;
