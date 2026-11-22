use crate::TextArea;
use ferroui_base::Ref;
use ferroui_controls::Control;
use std::fmt;

/// A rejected change to the editor's native margins or layers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EditorAdornmentError {
    AlreadyParented,
    DifferentTextView,
    InvalidIndex,
    InvalidLayerPosition,
    Busy,
}

impl fmt::Display for EditorAdornmentError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::AlreadyParented => "The control already belongs to a visual or logical tree",
            Self::DifferentTextView => "The margin belongs to a different text view",
            Self::InvalidIndex => "The index is outside the editor's adornment collection",
            Self::InvalidLayerPosition => "The background layer only accepts controls above it",
            Self::Busy => "The editor is recording, laying out or changing its adornments",
        })
    }
}

impl std::error::Error for EditorAdornmentError {}

impl TextArea {
    pub(crate) fn validate_adornment(
        &self,
        control: &Ref<Control>,
    ) -> Result<(), EditorAdornmentError> {
        let logical_ancestors = std::iter::successors(self.parent(), |node| node.parent());
        if control.visual_parent().is_some()
            || control.parent().is_some()
            || control.ptr_eq(&self.to_ref())
            || control.is_visual_ancestor_of(self)
            || self.editor().is_some_and(|editor| control.ptr_eq(&editor))
            || logical_ancestors
                .into_iter()
                .any(|node| control.ptr_eq(&node))
        {
            Err(EditorAdornmentError::AlreadyParented)
        } else {
            Ok(())
        }
    }
}
