//! Live selection formatting affinity and guarded format snapshots.
use super::*;

/// A format read may call application brush/property services. Track accepted
/// intent as well as visible values, including a mode changed away and back.
#[derive(Clone, Copy)]
pub(in crate::rich_text) struct SelectionFormatCheckpoint {
    mutation: u64,
    selection: EditorSelection,
    gravity: u64,
    options: u64,
    navigation: u64,
    checkpoint: u64,
}

impl RichTextEditor {
    /// Formatting affinity shared by every facade for this editor's selection.
    /// It does not change the separate TextPointer insertion tracking policy.
    pub fn selection_gravity(&self) -> RangeGravity {
        self.selection_gravity.get()
    }

    /// Changes only selection metadata. Outward is accepted as UIBehavior, as
    /// recorded by the native selection setter. Navigation, text replacement,
    /// composition, undo and redo retain the accepted mode. A failed edit group
    /// restores its starting mode; those history boundaries are FerroUI policy.
    ///
    /// This is permitted while read-only, composing or publishing. No callback,
    /// content mutation, typing patch, selection event or undo unit is created.
    /// An accepted same-value setter is still newer intent for pending reads.
    pub fn set_selection_gravity(&self, gravity: RangeGravity) {
        self.selection_gravity.set(gravity.canonical());
        self.selection_gravity_generation
            .set(self.selection_gravity_generation.get().wrapping_add(1));
    }

    pub(in crate::rich_text) fn selection_format_checkpoint(&self) -> SelectionFormatCheckpoint {
        SelectionFormatCheckpoint {
            mutation: self.document.mutation_version(),
            selection: self.selection(),
            gravity: self.selection_gravity_generation.get(),
            options: self.selection_options_generation.get(),
            navigation: self.navigation_generation(),
            checkpoint: self.navigation_checkpoint(),
        }
    }

    pub(in crate::rich_text) fn ensure_selection_format(
        &self,
        checkpoint: SelectionFormatCheckpoint,
    ) -> Result<(), EditError> {
        if self.document.mutation_version() != checkpoint.mutation {
            self.observe_external_document_change();
            return Err(EditError::ReentrantEdit);
        }
        if self.selection() != checkpoint.selection
            || self.selection_gravity_generation.get() != checkpoint.gravity
            || self.selection_options_generation.get() != checkpoint.options
            || self.navigation_generation() != checkpoint.navigation
            || self.navigation_checkpoint() != checkpoint.checkpoint
        {
            return Err(EditError::ReentrantEdit);
        }
        Ok(())
    }

    /// Resolves a transient insertion overlay without changing the explicit
    /// typing patch. Ordinary insertion already inherits its backward source;
    /// preserve that path and its Run identity when affinity selects that source.
    /// A nonempty selection keeps the model's existing replacement inheritance.
    /// A caret expanded by overtype still consumes its chosen insertion source.
    /// Exact native edits outside the live caret do not consume selection affinity.
    pub(super) fn insertion_character_format(
        &self,
        range: &Range<usize>,
    ) -> Result<CharacterFormat, EditError> {
        let checkpoint = self.selection_format_checkpoint();
        let selection = self.selection();
        let gravity = self.selection_gravity();
        let format = if selection.is_empty()
            && selection.active == range.start
            && gravity != RangeGravity::UIBehavior
        {
            let source = gravity.caret_element(&self.document, range.start);
            let ordinary = self.document.element_at(range.start, TextGravity::Backward);
            let explicit = self.typing_format();
            if source != ordinary || !range.is_empty() {
                source
                    .map(|element| CharacterFormat::effective(&element))
                    .unwrap_or_else(|| self.insertion_defaults().character)
                    .overlay(&explicit)
            } else {
                explicit
            }
        } else {
            self.typing_format()
        };
        self.ensure_selection_format(checkpoint)?;
        Ok(format)
    }
}
