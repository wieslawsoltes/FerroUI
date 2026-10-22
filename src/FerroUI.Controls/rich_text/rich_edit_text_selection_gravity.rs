//! Gravity on the actual editor selection, distinct from an independent range.
//! Native observations: native-msftedit-selection-gravity.json (.33648).
use super::*;

impl RichEditTextSelection {
    /// Reads the accepted mode shared by this editor's live selection handles.
    /// A fresh editor starts with UIBehavior. Selecting another range, moving,
    /// typing and obtaining an independent range retain the live mode.
    pub fn gravity(&self) -> RangeGravity {
        self.editor.selection_gravity()
    }

    /// Changes character-format affinity at a collapsed insertion boundary.
    /// Outward reads back as UIBehavior. The explicit typing patch still wins;
    /// a resolved neighboring format is never stored as a new typing patch.
    ///
    /// Changing affinity does not move endpoints, modify text, end composition
    /// or add history. Undo/redo keep the current mode; a failed grouped edit
    /// restores its starting mode. The latter are FerroUI transaction policies.
    /// Native synthetic terminal EOP styling is not part of the visible model.
    pub fn set_gravity(&self, gravity: RangeGravity) {
        self.editor.set_selection_gravity(gravity);
    }

    /// Validates a native/serialized numeric value before changing any state.
    pub fn set_gravity_value(&self, value: i32) -> Result<(), EditError> {
        self.set_gravity(RangeGravity::try_from(value)?);
        Ok(())
    }
}

#[cfg(test)]
#[path = "rich_edit_text_selection_gravity_tests.rs"]
mod tests;
