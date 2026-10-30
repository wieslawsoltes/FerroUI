//! Shared selection modes and scalar-by-scalar typed replacement planning.
use super::*;
use ferroui_base::media::text_formatting::unicode::GraphemeEnumerator;

const STORED_OPTIONS: SelectionOptions = SelectionOptions::from_bits_retain(
    SelectionOptions::AT_END_OF_LINE.bits()
        | SelectionOptions::OVERTYPE.bits()
        | SelectionOptions::ACTIVE.bits(),
);

impl RichTextEditor {
    /// Gets the modes shared by every selection handle. A caret is always
    /// StartActive, and Replace is retained even when a caller clears that bit.
    pub fn selection_options(&self) -> SelectionOptions {
        let selection = self.selection();
        let mut options = self.selection_options.get() | SelectionOptions::REPLACE;
        options.set(
            SelectionOptions::START_ACTIVE,
            selection.active <= selection.anchor,
        );
        options
    }

    /// Updates non-content editor state without adding an undo unit or clearing
    /// redo. Undo restores text/selection and retains the current modes; a failed
    /// edit group restores its starting modes along with its other configuration.
    /// These transaction boundaries are FerroUI's editor policy.
    pub fn set_selection_options(&self, options: SelectionOptions) -> Result<(), EditError> {
        if options.bits() & !SelectionOptions::all().bits() != 0 {
            return Err(DocumentError::InvalidFormat("Unknown selection option flags.").into());
        }
        if self.in_operation.get() && !self.projecting_selection.get() {
            return Err(EditError::ReentrantEdit);
        }
        let before = self.selection();
        let range = before.range();
        let requested = if options.contains(SelectionOptions::START_ACTIVE) {
            EditorSelection::new(range.end, range.start)
        } else {
            EditorSelection::new(range.start, range.end)
        };
        let mutation = self.document.mutation_version();
        let options_generation = self.selection_options_generation.get();
        let navigation_generation = self.navigation_generation();
        let checkpoint = self.navigation_checkpoint();
        let result = self.set_selection(requested);
        if self.document.mutation_version() != mutation {
            self.observe_external_document_change();
            return Err(EditError::ReentrantEdit);
        }
        result?;
        let expected_checkpoint = checkpoint.wrapping_add(u64::from(before != requested));
        if self.selection() == requested
            && self.selection_options_generation.get() == options_generation
            && self.navigation_generation() == navigation_generation
            && self.navigation_checkpoint() == expected_checkpoint
        {
            // Even a same-value nested setter is an explicit newer decision.
            // Its generation prevents a pending outer setter from replacing it.
            self.store_selection_options(options);
        }
        Ok(())
    }

    pub fn selection_type(&self) -> SelectionType {
        if self.selection().is_empty() {
            SelectionType::InsertionPoint
        } else {
            SelectionType::Normal
        }
    }

    pub(super) fn store_selection_options(&self, options: SelectionOptions) {
        self.selection_options.set(options & STORED_OPTIONS);
        self.selection_options_generation
            .set(self.selection_options_generation.get().wrapping_add(1));
    }

    pub(super) fn clear_typing_end_of_line(&self) {
        let options = self.selection_options.get();
        if options.contains(SelectionOptions::AT_END_OF_LINE) {
            self.store_selection_options(options & !SelectionOptions::AT_END_OF_LINE);
        }
    }

    /// Enters every supplied scalar as typed input. Unlike bulk text insertion,
    /// TypeText treats CR and LF independently: a CRLF pair types two paragraph
    /// breaks. An empty call is a no-op, including during an IME preview.
    /// A selection ending at a paragraph start retains its final selected mark;
    /// a replacement crossing an earlier mark retains the trailing alignment.
    pub fn type_text(&self, text: &str) -> Result<bool, EditError> {
        if text.is_empty() {
            return Ok(false);
        }
        self.insert_text(&text.replace('\r', "\n"), EditKind::Typing)
    }

    /// Typed replacement retains the last selected paragraph mark when the
    /// selection ends exactly at the next paragraph's start. This is an actual
    /// block boundary, not an arbitrary LF inside a Run or a soft LineBreak.
    /// Native selection observations cover both directed selections, a selected
    /// mark alone and a whole first paragraph. Bulk edits and IME replacement
    /// have separate contracts and do not use this adjustment.
    pub(super) fn prepare_typing_range(
        &self,
        mut range: Range<usize>,
    ) -> Result<Range<usize>, EditError> {
        if range.is_empty() {
            return Ok(range);
        }
        let checkpoint = self.selection_format_checkpoint();
        self.document.validate_range(range.clone())?;
        let mut start = 0;
        for (index, block) in self.document.blocks().snapshot().iter().enumerate() {
            if index > 0 && start == range.end {
                range.end -= 1;
                break;
            }
            if start >= range.end {
                break;
            }
            start += block.text_length() + 1;
        }
        self.ensure_selection_format(checkpoint)?;
        Ok(range)
    }

    /// Plans one rich replacement from the original source. Native TypeText
    /// consumes one target text element per supplied Unicode scalar; an existing
    /// selection consumes the first scalar's replacement. Typed paragraph breaks
    /// and an existing paragraph mark do not consume another target element.
    /// CR, LF and VT (a LineBreak, decision D10) are protected so a structured
    /// line or paragraph break remains intact. Native U+2028 parity is uncharacterized.
    ///
    /// Compute the accepted prefix and its consumed extent together. Truncating
    /// input only after extending the range would erase targets for rejected
    /// scalars at MaxLength. The caller applies this plan in one rich transaction.
    pub(super) fn prepare_overtype_text(
        &self,
        text: &str,
        range: Range<usize>,
    ) -> Result<(Range<usize>, String), EditError> {
        let mutation = self.document.mutation_version();
        let selection = self.selection();
        let options_generation = self.selection_options_generation.get();
        let limit = self.max_length();
        let source: Vec<u16> = self.document.text().encode_utf16().collect();
        let normalized = normalize_text(text);
        let initial_end = range.end;
        let mut ends = GraphemeEnumerator::new(&source)
            .map(|grapheme| grapheme.offset() + grapheme.length())
            .filter(|end| *end > initial_end);
        let mut end = initial_end;
        let mut accepted = String::new();
        let mut accepted_length = 0;
        for (index, scalar) in normalized.chars().enumerate() {
            // Paragraph endpoint protection can shorten a selected mark to an
            // empty replacement range. It remains a selected replacement: the
            // first scalar must use the original selection's typing semantics.
            let next_end = if !(index == 0 && !selection.is_empty())
                && scalar != '\n'
                && source
                    .get(end)
                    .is_some_and(|unit| {
                        *unit != u16::from(b'\n') && *unit != u16::from(b'\r') && *unit != 0x000B
                    })
            {
                ends.next().unwrap_or(end)
            } else {
                end
            };
            let next_length = accepted_length + scalar.len_utf16();
            let retained = source.len().saturating_sub(next_end - range.start);
            if limit != 0 && next_length > limit.saturating_sub(retained) {
                break;
            }
            accepted.push(scalar);
            accepted_length = next_length;
            end = next_end;
        }
        if self.document.mutation_version() != mutation {
            self.observe_external_document_change();
            return Err(EditError::ReentrantEdit);
        }
        if self.selection() != selection
            || self.selection_options_generation.get() != options_generation
            || self.max_length() != limit
        {
            return Err(EditError::ReentrantEdit);
        }
        Ok((range.start..end, accepted))
    }
}
