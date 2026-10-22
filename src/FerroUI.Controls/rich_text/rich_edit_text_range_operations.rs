//! TOM character replacement and deletion over the shared structured document.
//!
//! Contracts: the pinned Microsoft.UI.Text.xml ITextRange.Character/Delete and
//! https://learn.microsoft.com/windows/windows-app-sdk/api/winrt/microsoft.ui.text.itextrange.delete
//! The native engine's synthetic final paragraph mark is not a model character.
use super::*;

impl RichEditTextRange {
    /// The UTF-16 code unit at StartPosition, including the leading surrogate of
    /// a supplementary character. A caret at the end of this model story returns
    /// zero; the native synthetic terminal paragraph marker is not exposed.
    pub fn character(&self) -> u16 {
        self.document()
            .text()
            .encode_utf16()
            .nth(self.start_position())
            .unwrap_or(0)
    }

    /// Returns the complete Unicode scalar at StartPosition, independently of
    /// whether this range is degenerate. None means the end of the model story.
    pub fn character_scalar(&self) -> Option<char> {
        scalar_at(&self.document().text(), self.start_position())
    }

    /// Replaces the first UTF-16 code unit without moving the range's start.
    /// Isolated surrogate input and replacement of half an existing pair are
    /// rejected before changing content, composition or history. Use
    /// [`set_character_scalar`](Self::set_character_scalar) for a complete scalar.
    pub fn set_character(&self, value: u16) -> Result<bool, EditError> {
        let value = char::from_u32(u32::from(value)).ok_or(DocumentError::InvalidFormat(
            "A character replacement cannot introduce an isolated UTF-16 surrogate.",
        ))?;
        self.replace_character(value, true)
    }

    /// Replaces the complete scalar at StartPosition. This Rust extension to the
    /// native Char16 property supports supplementary characters without exposing
    /// invalid intermediate UTF-16. It retains the replaced character's format,
    /// rebases the independent directed selection, and creates one undo unit.
    pub fn set_character_scalar(&self, value: char) -> Result<bool, EditError> {
        self.replace_character(value, false)
    }

    fn replace_character(&self, value: char, single_unit: bool) -> Result<bool, EditError> {
        ensure_range_editable(&self.editor)?;
        let previous = self.checked_character(single_unit)?;
        // Text input and native paragraph insertion share the model's LF
        // normalization; compare the effective value before touching IME/history.
        let value = if value == '\r' { '\n' } else { value };
        if previous == value {
            return Ok(false);
        }
        ensure_character_length(&self.editor, previous, value)?;

        self.editor.prepare_edit()?;
        // Cancelling a preview rebases the live range. Recapture its committed
        // character instead of restoring a stale preview-derived replacement.
        let previous = self.checked_character(single_unit)?;
        if previous == value {
            return Ok(false);
        }
        ensure_character_length(&self.editor, previous, value)?;
        let range = self.range();
        let replacement = range.start..range.start + previous.len_utf16();
        let selection = self.editor.selection();
        let typing_format = self.editor.typing_format();
        let own_typing_format = self.range_typing_format();
        let external_generation = self.editor.external_change_generation();
        let document = self.document();
        let element = document.element_at(range.start, TextGravity::Forward);
        // The document's run splice retains local/inherited formatting without
        // materializing inherited theme values. Structural replacements capture
        // the assigned format and container path of the overwritten character.
        let fragment = if value != '\n'
            && element
                .as_ref()
                .is_some_and(|element| element.is::<crate::documents::Run>())
            && previous != '\n'
        {
            None
        } else {
            Some(character_fragment(&document.snapshot(), range.start, value))
        };
        let mut buffer = [0; 4];
        let text = value.encode_utf8(&mut buffer);
        let result = self.editor.edit_group(|editor| {
            if let Some(fragment) = &fragment {
                editor
                    .document()
                    .replace_range(replacement.clone(), fragment)?;
            } else {
                editor.document().replace_text(replacement.clone(), text)?;
            }
            let selection = rebase_selection(selection, replacement.clone(), value.len_utf16());
            editor.set_selection(selection)?;
            if selection.is_empty() {
                editor.replace_range_typing_format(&typing_format)?;
            }
            // The property replaces a character; it does not navigate the range.
            // Its far endpoint continues to encompass the same remaining text.
            let end = if range.is_empty() {
                range.start
            } else {
                range.end - previous.len_utf16() + value.len_utf16()
            };
            self.set_range(range.start, end)?;
            self.set_range_typing_format(own_typing_format.clone())?;
            Ok(true)
        });
        if result.is_err() && self.editor.external_change_generation() == external_generation {
            self.set_range(range.start, range.end)?;
            self.set_range_typing_format(own_typing_format)?;
        }
        result
    }

    fn checked_character(&self, single_unit: bool) -> Result<char, EditError> {
        let value = self.character_scalar().ok_or(DocumentError::InvalidFormat(
            "Character replacement requires a character before the end of the model story.",
        ))?;
        if single_unit && value.len_utf16() != 1 {
            return Err(DocumentError::InvalidUtf16Boundary {
                offset: self.start_position() + 1,
            }
            .into());
        }
        Ok(value)
    }

    /// Deletes this range and, for a nonzero count, additional logical units in
    /// the requested direction. Removing a nonempty range counts as one unit;
    /// count zero removes only that range. The return value is a nonnegative
    /// count of removed units, using usize so even i32::MIN has no abs overflow.
    ///
    /// A nonempty range is removed before resolving the remaining units: joining
    /// its surrounding text can change word, paragraph and shaped line boundaries.
    /// All changes use one undo transaction and preserve the independent editor
    /// selection. Unavailable layout-dependent units fail explicitly.
    pub fn delete(&self, unit: TextRangeUnit, count: i32) -> Result<usize, EditError> {
        ensure_range_editable(&self.editor)?;
        if !self.deletion_has_effect(unit, count)? {
            return Ok(0);
        }
        self.editor.prepare_edit()?;
        if !self.deletion_has_effect(unit, count)? {
            return Ok(0);
        }

        let original = self.range();
        let external_generation = self.editor.external_change_generation();
        let own_typing_format = self.range_typing_format();
        let result = self.editor.edit_group(|editor| {
            let mut position = self.start_position();
            let mut removed = 0;
            let mut remaining = count;
            if !self.is_degenerate() {
                position = delete_span(editor, self.range(), true)?;
                removed = 1;
                remaining -= count.signum();
            }
            if remaining != 0 {
                let units = TextUnits::resolve(editor, unit)?;
                let (end, moved) = units.move_position(position, remaining);
                if moved != 0 {
                    position = delete_span(editor, position.min(end)..position.max(end), false)?;
                    removed += moved.unsigned_abs() as usize;
                }
            }
            self.set_range(position, position)?;
            self.set_range_typing_format(own_typing_format.clone())?;
            Ok(removed)
        });
        if result.is_err() && self.editor.external_change_generation() == external_generation {
            // The shared editor restores model/history/selection. This service
            // also restores the independent range's explicitly changed endpoints.
            self.set_range(original.start, original.end)?;
            self.set_range_typing_format(own_typing_format)?;
        }
        result
    }

    fn deletion_has_effect(&self, unit: TextRangeUnit, count: i32) -> Result<bool, EditError> {
        let range = self.range();
        if !range.is_empty() && count.unsigned_abs() <= 1 {
            // Native Delete ignores Unit when only the existing range is removed.
            return Ok(true);
        }
        if range.is_empty() && count == 0 {
            return Ok(false);
        }
        // Validate display-unit availability before cancelling any IME preview.
        let units = TextUnits::resolve(&self.editor, unit)?;
        Ok(!range.is_empty() || units.move_position(range.start, count).1 != 0)
    }
}

impl RichEditTextSelection {
    /// TOM unit deletion over the shared directed selection. The existing
    /// parameterless `delete` remains the direct selection-deletion convenience.
    pub fn delete_by(&self, unit: TextRangeUnit, count: i32) -> Result<usize, EditError> {
        ensure_range_editable(&self.editor)?;
        if !self.deletion_has_effect(unit, count)? {
            return Ok(0);
        }
        self.editor.edit_group(|editor| {
            // Both layout validation and cancellation of a preview may invoke
            // application callbacks. Capture the actual selection afterwards.
            if !self.deletion_has_effect(unit, count)? {
                return Ok(0);
            }
            let range = self.range()?;
            let removed = range.delete(unit, count)?;
            if removed != 0 {
                // An independent range preserves the editor's selection. The
                // selection facade instead collapses it, even when the native
                // paragraph rule retained its selected final separator.
                editor.set_selection(EditorSelection::caret(range.start_position()))?;
            }
            Ok(removed)
        })
    }

    fn deletion_has_effect(&self, unit: TextRangeUnit, count: i32) -> Result<bool, EditError> {
        let selection = self.selection();
        if !selection.is_empty() && count.unsigned_abs() <= 1 {
            return Ok(true);
        }
        if selection.is_empty() && count == 0 {
            return Ok(false);
        }
        let units = TextUnits::resolve(&self.editor, unit)?;
        let selection = self.selection();
        Ok(!selection.is_empty() || units.move_position(selection.range().start, count).1 != 0)
    }
}

fn ensure_range_editable(editor: &RichTextEditor) -> Result<(), EditError> {
    if editor.is_read_only() {
        Err(EditError::ReadOnly)
    } else if editor.is_in_operation() {
        Err(EditError::ReentrantEdit)
    } else {
        Ok(())
    }
}

fn scalar_at(text: &str, position: usize) -> Option<char> {
    let mut offset = 0;
    text.chars().find(|character| {
        let current = offset;
        offset += character.len_utf16();
        current == position
    })
}

fn character_fragment(
    snapshot: &DocumentSnapshot,
    position: usize,
    value: char,
) -> DocumentSnapshot {
    let (index, offset) = snapshot.locate(position);
    let source = &snapshot.paragraphs[index];
    let text = if value == '\n' {
        String::new()
    } else {
        value.to_string()
    };
    let inline = character_context(&source.inlines, offset, &text);
    let paragraph = crate::documents::ParagraphSnapshot {
        format: source.format.clone(),
        inlines: vec![inline],
    };
    let mut paragraphs = if value == '\n' {
        vec![paragraph.clone(), paragraph]
    } else {
        vec![paragraph]
    };
    // replace_range retains the destination's first paragraph. Its assigned
    // character defaults must stay inherited, rather than being copied to a span
    // around the new character. A newly split paragraph keeps those defaults.
    paragraphs[0].format.character = CharacterFormat::default();
    DocumentSnapshot { paragraphs }
}

fn character_context(inlines: &[InlineSnapshot], offset: usize, text: &str) -> InlineSnapshot {
    let mut position = 0;
    for (index, inline) in inlines.iter().enumerate() {
        let end = position + inline.text_length();
        // Forward gravity chooses the overwritten leaf at a formatting boundary.
        // At a paragraph mark, use the preceding paragraph's final inline context.
        if offset < end || index + 1 == inlines.len() {
            let mut result = inline.clone();
            if let Some(children) = inline.children() {
                let child = character_context(children, offset.saturating_sub(position), text);
                *result
                    .children_mut()
                    .expect("A captured container retains children") = vec![child];
            } else {
                result.kind = InlineSnapshotKind::Run(text.to_owned());
            }
            return result;
        }
        position = end;
    }
    InlineSnapshot::run(text)
}

fn ensure_character_length(
    editor: &RichTextEditor,
    previous: char,
    value: char,
) -> Result<(), EditError> {
    let current = editor.document().text_length();
    let length = current
        .checked_sub(previous.len_utf16())
        .and_then(|retained| retained.checked_add(value.len_utf16()))
        .ok_or(DocumentError::InvalidFormat(
            "The replaced character's document length is out of range.",
        ))?;
    let limit = editor.max_length();
    if limit != 0 && length > limit && length > current {
        Err(DocumentError::InvalidFormat(
            "The character replacement would exceed the document's text limit.",
        )
        .into())
    } else {
        Ok(())
    }
}

pub(super) fn delete_span(
    editor: &RichTextEditor,
    mut range: Range<usize>,
    selected: bool,
) -> Result<usize, EditError> {
    let document = editor.document();
    let snapshot = document.snapshot();
    let (first, start) = snapshot.locate(range.start);
    let (last, end) = snapshot.locate(range.end);
    let first_length = snapshot.paragraphs[first].text_length();
    // A partial paragraph selection ending at its mark leaves that mark in place.
    // A mark selected by itself, and a whole paragraph, remain deletable.
    if selected && last == first + 1 && end == 0 && start > 0 && start < first_length {
        range.end -= 1;
    }
    let following_format = (first != last && (start == 0 || end > 0))
        .then(|| snapshot.paragraphs[last].format.clone());
    let selection = editor.selection();
    let typing_format = editor.typing_format();
    if first != last {
        // Structural deletion has no inserted text that needs an insertion
        // format. replace_text would synthesize an empty carrier with the old
        // paragraph's defaults, keeping deleted formatting at the new caret.
        document.replace_range(range.clone(), &DocumentSnapshot::default())?;
    } else {
        document.replace_text(range.clone(), "")?;
    }
    if let Some(mut format) = following_format {
        // Paragraph formatting follows the surviving following paragraph. If a
        // prefix survives, retain its paragraph character defaults independently
        // of the native paragraph-format transfer. Deleting the whole paragraph
        // removes its character defaults together with its content.
        let paragraph = document.paragraphs()[first].clone();
        if start != 0 {
            format.character = CharacterFormat::from_element(&paragraph);
        }
        format.replace_on(&paragraph);
    }
    let selection = rebase_selection(selection, range.clone(), 0);
    editor.set_selection(selection)?;
    if selection.is_empty() {
        editor.replace_range_typing_format(&typing_format)?;
    }
    Ok(range.start)
}

#[cfg(test)]
#[path = "rich_edit_text_range_operations_tests.rs"]
mod tests;
