//! TOM range formatting affinity over the shared visible UTF-16 document.
//!
//! Contract: pinned Microsoft.UI.Text.xml ITextRange.Gravity and RangeGravity.
//! Measured system-MSFTEDIT behavior is recorded in native-msftedit-links-gravity.json.
//! This mode is independent of TextPointer's insertion tracking policy.
use super::*;

/// Character-format affinity at a collapsed range boundary, with the native
/// Microsoft.UI.Text numeric values. UIBehavior follows the preceding run except
/// at the start of a paragraph. Backward can use the preceding paragraph's last
/// character; Forward and collapsed Inward use the following run.
///
/// The characterized MSFTEDIT setter canonicalizes Outward to UIBehavior. All
/// five constants remain available; the getter reports the accepted mode. This
/// does not change FerroUI's separate live-pointer rebasing policy.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum RangeGravity {
    #[default]
    UIBehavior = 0,
    Backward = 1,
    Forward = 2,
    Inward = 3,
    Outward = 4,
}

ferroui_base::ferro_markup_enum!(RangeGravity {
    UIBehavior, Backward, Forward, Inward, Outward,
}, { namespace: "FerroUI.Controls" });

impl TryFrom<i32> for RangeGravity {
    type Error = DocumentError;

    fn try_from(value: i32) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::UIBehavior),
            1 => Ok(Self::Backward),
            2 => Ok(Self::Forward),
            3 => Ok(Self::Inward),
            4 => Ok(Self::Outward),
            _ => Err(DocumentError::InvalidFormat("Unknown text range gravity.")),
        }
    }
}

impl RangeGravity {
    fn canonical(self) -> Self {
        if self == Self::Outward {
            Self::UIBehavior
        } else {
            self
        }
    }
}

impl RichEditTextRange {
    /// Reads this independent range's accepted formatting affinity.
    pub fn gravity(&self) -> RangeGravity {
        self.data.borrow().gravity
    }

    /// Changes range metadata without changing text, selection, composition or
    /// undo/redo. Rust Clone shares this setting; get_clone() starts with UIBehavior.
    /// Endpoint setters retain it. Successful text undo leaves the current mode.
    ///
    /// A real mode change retires pending asynchronous range operations, even
    /// when a later setter returns to the original mode. Outward is accepted as
    /// UIBehavior, matching the measured native setter readback.
    pub fn set_gravity(&self, gravity: RangeGravity) {
        let gravity = gravity.canonical();
        let mut data = self.data.borrow_mut();
        if data.gravity != gravity {
            data.gravity = gravity;
            data.generation = data.generation.wrapping_add(1);
        }
    }

    /// Checked numeric boundary for native/serialized values. Invalid values
    /// leave all state untouched; the typed setter cannot receive them.
    pub fn set_gravity_value(&self, value: i32) -> Result<(), EditError> {
        self.set_gravity(RangeGravity::try_from(value)?);
        Ok(())
    }

    pub(super) fn range_read_checkpoint(&self) -> (u64, u64) {
        (
            self.data.borrow().generation,
            self.document().mutation_version(),
        )
    }

    pub(super) fn ensure_range_generation(&self, generation: u64) -> Result<(), EditError> {
        if self.data.borrow().generation != generation {
            return Err(EditError::ReentrantEdit);
        }
        Ok(())
    }

    pub(super) fn ensure_range_read(&self, (generation, mutation): (u64, u64)) -> Result<(), EditError> {
        if self.document().mutation_version() != mutation {
            // A format/brush service can execute application code. Its direct
            // model write is external to an enclosing editor transaction.
            self.editor.observe_external_document_change();
            return Err(EditError::ReentrantEdit);
        }
        self.ensure_range_generation(generation)
    }

    pub(super) fn range_typing_format(&self) -> CharacterFormat {
        // Copy only the immutable snapshot's Rc while borrowing RangeData.
        // Deep copying brushes/collections is an application callback boundary.
        let format = self.data.borrow().typing_format.clone();
        (*format).clone()
    }

    pub(super) fn caret_character_format(&self) -> Result<CharacterFormat, EditError> {
        let checkpoint = self.range_read_checkpoint();
        let range = self.range();
        debug_assert!(range.is_empty());
        let (gravity, typing) = {
            let data = self.data.borrow();
            (data.gravity, data.typing_format.clone())
        };
        let document = self.document();
        let element = match gravity {
            RangeGravity::Backward if range.start > 0 => {
                // This is an element lookup, not a text slice or an editable
                // endpoint. A code unit inside a surrogate still identifies its run.
                document.element_at(range.start - 1, TextGravity::Forward)
            }
            RangeGravity::Forward | RangeGravity::Inward
                if range.start == document.text_length() =>
            {
                // Native forward affinity can sample its synthetic terminal EOP.
                // The visible model has no such character: use the final
                // paragraph's effective defaults, without adding a position.
                document
                    .paragraphs()
                    .last()
                    .map(|paragraph| paragraph.clone().upcast())
            }
            RangeGravity::Forward | RangeGravity::Inward => {
                document.element_at(range.start, TextGravity::Forward)
            }
            _ => document.element_at(range.start, TextGravity::Backward),
        };
        let format = element
            .map(|element| CharacterFormat::effective(&element))
            .unwrap_or_else(|| self.editor.insertion_defaults().character)
            .overlay(&typing);
        self.ensure_range_read(checkpoint)?;
        Ok(format)
    }
}

#[cfg(test)]
#[path = "rich_edit_text_range_gravity_tests.rs"]
mod tests;
