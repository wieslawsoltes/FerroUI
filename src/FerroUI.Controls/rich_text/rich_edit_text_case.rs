//! TOM upper/lower conversion over the existing structured document and history.
use super::*;

/// Requested case, retaining the public text-object-model numeric values.
/// Conversion uses Rust's Unicode default casing; it does not invoke a platform
/// locale service or reinterpret the document's Language formatting.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum LetterCase {
    #[default]
    Lower = 0,
    Upper = 1,
}

ferroui_base::ferro_markup_enum!(LetterCase { Lower, Upper }, { namespace: "FerroUI.Controls" });

impl LetterCase {
    fn convert(self, text: &str) -> String {
        match self {
            Self::Lower => text.to_lowercase(),
            Self::Upper => text.to_uppercase(),
        }
    }

    fn scalar_count(self, character: char) -> usize {
        match self {
            Self::Lower => character.to_lowercase().count(),
            Self::Upper => character.to_uppercase().count(),
        }
    }
}

impl RichEditTextRange {
    /// Changes the selected letters without flattening inline formatting, links,
    /// paragraph properties, soft breaks or embedded controls. Unicode expansions
    /// rebase the independent directed selection; the edit is one undo unit.
    ///
    /// The complete selected string supplies contextual lowercase rules, even
    /// across differently formatted runs. A degenerate or unchanged range returns
    /// `false` without touching composition, revision, typing format or history.
    /// An expansion exceeding MaxLength fails before replacing content rather
    /// than truncating the transformed text. Locale-specific native casing remains
    /// separate from this portable Unicode default-casing operation.
    pub fn change_case(&self, value: LetterCase) -> Result<bool, EditError> {
        if self.editor.is_read_only() {
            return Err(EditError::ReadOnly);
        }
        if self.editor.is_in_operation() {
            return Err(EditError::ReentrantEdit);
        }
        let source = self.text()?;
        let converted = value.convert(&source);
        if source == converted {
            return Ok(false);
        }
        ensure_case_length(&self.editor, &source, &converted)?;

        // Ordinary programmatic edits settle the existing composition first.
        // Its cancellation can rebase this live range, so never apply a snapshot
        // captured from the preview to the subsequently restored committed text.
        let composed = self.editor.prepare_edit()?;
        let (source, converted) = if composed {
            let source = self.text()?;
            let converted = value.convert(&source);
            if source == converted {
                return Ok(false);
            }
            (source, converted)
        } else {
            (source, converted)
        };
        let new_document_length = ensure_case_length(&self.editor, &source, &converted)?;
        let checkpoint = self.range_read_checkpoint();
        let editor_checkpoint = self.editor.navigation_checkpoint();
        let range = self.range();
        let selection = self.editor.selection();
        let mut fragment = self.snapshot()?;
        self.ensure_range_read(checkpoint)?;
        let converted_selection = convert_snapshot(
            &mut fragment,
            &source,
            &converted,
            value,
            range.clone(),
            selection,
        );
        self.ensure_range_read(checkpoint)?;
        if self.editor.navigation_checkpoint() != editor_checkpoint {
            return Err(EditError::ReentrantEdit);
        }
        let selection = converted_selection?;
        fragment.validate()?;

        self.editor.edit_group(|editor| {
            self.ensure_range_read(checkpoint)?;
            if editor.navigation_checkpoint() != editor_checkpoint {
                return Err(EditError::ReentrantEdit);
            }
            // MaxLength may have been lowered below the existing document size.
            // A nonexpanding case edit still preserves all existing characters.
            // Restore the configured limit before publishing the grouped change.
            let configured_limit = editor.max_length();
            if configured_limit > 0 && new_document_length > configured_limit {
                editor.set_max_length(new_document_length);
            }
            let result = self.set_snapshot_preserving_link_fields(&fragment);
            editor.set_max_length(configured_limit);
            result?;
            let typing_format = editor.typing_format();
            editor.set_selection(selection)?;
            if selection.is_empty() {
                editor.apply_format(&typing_format)?;
            }
            Ok(true)
        })
    }
}

fn ensure_case_length(
    editor: &RichTextEditor,
    source: &str,
    converted: &str,
) -> Result<usize, EditError> {
    let old_length = editor.document().text_length();
    let length = old_length
        .checked_sub(source.encode_utf16().count())
        .and_then(|retained| retained.checked_add(converted.encode_utf16().count()))
        .ok_or(DocumentError::InvalidFormat(
            "The converted text length is out of range.",
        ))?;
    let limit = editor.max_length();
    if limit > 0 && length > limit && length > old_length {
        return Err(DocumentError::InvalidFormat(
            "The case conversion would exceed the document's text limit.",
        )
        .into());
    }
    Ok(length)
}

/// Attribute each converted scalar to its original run. Rust's contextual final
/// sigma changes the scalar value but not the number of output scalars; full
/// mappings such as sharp-s and dotted-I can expand. Consume the actual contextual
/// output, rather than applying independent string casing to individual runs.
fn convert_snapshot(
    snapshot: &mut DocumentSnapshot,
    source: &str,
    converted: &str,
    value: LetterCase,
    range: Range<usize>,
    selection: EditorSelection,
) -> Result<EditorSelection, EditError> {
    let mut output = converted.chars();
    for (index, paragraph) in snapshot.paragraphs.iter_mut().enumerate() {
        if index != 0 {
            consume_unchanged(&mut output, '\n')?;
        }
        convert_inlines(&mut paragraph.inlines, &mut output, value)?;
    }
    if output.next().is_some() {
        return Err(case_mapping_error());
    }

    let mut offsets = vec![None; source.encode_utf16().count() + 1];
    offsets[0] = Some(0);
    let mut source_offset = 0;
    let mut converted_offset = 0;
    let mut output = converted.chars();
    for character in source.chars() {
        source_offset += character.len_utf16();
        for _ in 0..value.scalar_count(character) {
            converted_offset += output.next().ok_or_else(case_mapping_error)?.len_utf16();
        }
        offsets[source_offset] = Some(converted_offset);
    }
    snapshot.remap_link_field_offsets(&offsets)?;
    let map = |position: usize| -> Result<usize, EditError> {
        if position <= range.start {
            return Ok(position);
        }
        if position >= range.end {
            return Ok(position - range.len() + converted_offset);
        }
        offsets
            .get(position - range.start)
            .copied()
            .flatten()
            .map(|offset| range.start + offset)
            .ok_or_else(case_mapping_error)
    };
    Ok(EditorSelection::new(
        map(selection.anchor)?,
        map(selection.active)?,
    ))
}

fn convert_inlines(
    inlines: &mut [InlineSnapshot],
    output: &mut std::str::Chars<'_>,
    value: LetterCase,
) -> Result<(), EditError> {
    for inline in inlines {
        match &mut inline.kind {
            InlineSnapshotKind::Run(text) => {
                let count: usize = text
                    .chars()
                    .map(|character| value.scalar_count(character))
                    .sum();
                let mut replacement = String::with_capacity(text.len());
                for _ in 0..count {
                    replacement.push(output.next().ok_or_else(case_mapping_error)?);
                }
                *text = replacement;
            }
            InlineSnapshotKind::LineBreak => consume_unchanged(output, '\u{000B}')?,
            InlineSnapshotKind::Embedded(Some(_)) => consume_unchanged(output, '\u{fffc}')?,
            InlineSnapshotKind::Embedded(None) => {}
            _ => {
                if let Some(children) = inline.children_mut() {
                    convert_inlines(children, output, value)?;
                }
            }
        }
    }
    Ok(())
}

fn consume_unchanged(output: &mut std::str::Chars<'_>, expected: char) -> Result<(), EditError> {
    if output.next() == Some(expected) {
        Ok(())
    } else {
        Err(case_mapping_error())
    }
}

fn case_mapping_error() -> EditError {
    DocumentError::InvalidFormat("Unicode case mapping did not preserve document boundaries.")
        .into()
}

#[cfg(test)]
#[path = "rich_edit_text_case_tests.rs"]
mod tests;
