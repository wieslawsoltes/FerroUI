//! The list editing of the rich edit control (#1257): a list is the `ListType` of each of its
//! paragraphs (`ParagraphFormat`), so applying, changing and removing one is a paragraph
//! format of the selection. These are the commands the keyboard adds: a level deeper or
//! shallower, and the end of a list on Enter or Backspace.

use super::{EditError, RichTextEditor};
use crate::documents::{MarkerType, Paragraph, ParagraphFormat, MAX_LIST_LEVEL};
use ferroui_base::Ref;
use std::ops::Range;

impl RichTextEditor {
    /// The paragraphs a paragraph format of `range` applies to, with their ranges: the
    /// paragraph of the start through the paragraph of the end, without the paragraph that a
    /// non-empty range only reaches the start of.
    fn paragraphs_of(&self, range: Range<usize>) -> Vec<(Range<usize>, Ref<Paragraph>)> {
        let mut blocks = Vec::new();
        let mut offset = 0;
        let mut start = None;
        let mut end = None;
        for block in self.document().blocks().snapshot().iter() {
            let block_end = offset + block.text_length();
            if start.is_none() && range.start <= block_end {
                start = Some(blocks.len());
            }
            let index = blocks.len();
            blocks.push((offset..block_end, block.cast::<Paragraph>()));
            if range.end <= block_end {
                // A non-empty range that ends at the start of a paragraph does not format it.
                let reaches_start_only = index > start.unwrap_or(index) && !range.is_empty() && range.end == offset;
                end = Some(if reaches_start_only { index - 1 } else { index });
                break;
            }
            offset = block_end + 1;
        }
        let (Some(start), Some(end)) = (start, end) else {
            return Vec::new();
        };
        blocks
            .drain(start..=end)
            .filter_map(|(range, paragraph)| Some((range, paragraph?)))
            .collect()
    }

    /// The list paragraph of an empty selection, with its range.
    fn caret_list_paragraph(&self) -> Option<(Range<usize>, Ref<Paragraph>)> {
        let selection = self.selection();
        if !selection.is_empty() {
            return None;
        }
        self.paragraphs_of(selection.range())
            .into_iter()
            .next()
            .filter(|(_, paragraph)| paragraph.list_type() != MarkerType::None)
    }

    /// Moves every list paragraph of the selection `delta` levels deeper (positive) or
    /// shallower, between level 1 and 9, as one undo unit; the paragraphs of the selection that
    /// are not in a list stay as they are. Returns whether a level changed.
    pub fn change_list_level(&self, delta: i32) -> Result<bool, EditError> {
        self.prepare_edit()?;
        let changes: Vec<(Range<usize>, i32)> = self
            .paragraphs_of(self.selection().range())
            .into_iter()
            .filter(|(_, paragraph)| paragraph.list_type() != MarkerType::None)
            .filter_map(|(range, paragraph)| {
                let level = paragraph.list_level_index().max(1);
                let next = level.saturating_add(delta).clamp(1, MAX_LIST_LEVEL);
                (next != paragraph.list_level_index()).then_some((range, next))
            })
            .collect();
        if changes.is_empty() {
            return Ok(false);
        }
        self.edit_group(|editor| {
            for (range, level) in &changes {
                editor.document().apply_paragraph_format(
                    range.clone(),
                    &ParagraphFormat {
                        list_level_index: Some(*level),
                        ..Default::default()
                    },
                )?;
            }
            Ok(true)
        })
    }

    /// Whether Tab and Shift+Tab change the list level instead of typing a tab: a selection
    /// that starts at the start of a list paragraph (a caret there, or a selection of the
    /// item), or a selection over more than one paragraph with a list paragraph.
    pub fn tab_changes_list_level(&self) -> bool {
        let range = self.selection().range();
        let paragraphs = self.paragraphs_of(range.clone());
        let is_list = |paragraph: &Ref<Paragraph>| paragraph.list_type() != MarkerType::None;
        if paragraphs.len() > 1 {
            return paragraphs.iter().any(|(_, paragraph)| is_list(paragraph));
        }
        paragraphs
            .first()
            .is_some_and(|(paragraph_range, paragraph)| paragraph_range.start == range.start && is_list(paragraph))
    }

    /// Enter in an empty list paragraph with an empty selection ends the list there: a nested
    /// paragraph moves one level up, a paragraph at level 1 leaves the list. Returns false
    /// where Enter inserts a paragraph instead.
    pub fn end_list_on_enter(&self) -> Result<bool, EditError> {
        self.prepare_edit()?;
        let Some((range, paragraph)) = self.caret_list_paragraph().filter(|(range, _)| range.is_empty()) else {
            return Ok(false);
        };
        if paragraph.list_level_index() > 1 {
            return self.change_list_level(-1);
        }
        self.remove_list(range)
    }

    /// Backspace at the start of a list paragraph with an empty selection removes the
    /// paragraph's marker and keeps its text. Returns false where Backspace deletes instead.
    pub fn remove_list_on_backspace(&self) -> Result<bool, EditError> {
        self.prepare_edit()?;
        let selection = self.selection();
        let Some((range, _)) = self
            .caret_list_paragraph()
            .filter(|(range, _)| range.start == selection.active)
        else {
            return Ok(false);
        };
        self.remove_list(range)
    }

    fn remove_list(&self, range: Range<usize>) -> Result<bool, EditError> {
        debug_assert!(range.start <= self.selection().active && self.selection().active <= range.end);
        self.apply_paragraph_format(&ParagraphFormat {
            list_type: Some(MarkerType::None),
            list_level_index: Some(0),
            ..Default::default()
        })?;
        Ok(true)
    }
}
