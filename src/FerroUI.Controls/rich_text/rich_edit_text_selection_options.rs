//! Native-valued selection contracts shared by every live selection facade.
use super::*;

bitflags::bitflags! {
    /// Text-object-model selection flags, with the Microsoft.UI.Text values.
    ///
    /// Direction is derived from the accepted selection. Replace remains set,
    /// matching the characterized MSFTEDIT setter. Overtype controls actual typed
    /// replacement. Active and AtEndOfLine retain explicit caller state; automatic
    /// focus synchronization and ambiguous visual-caret affinity are not yet
    /// provided by these flags.
    #[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
    pub struct SelectionOptions: u32 {
        const NONE = 0;
        const START_ACTIVE = 1;
        const AT_END_OF_LINE = 2;
        const OVERTYPE = 4;
        const ACTIVE = 8;
        const REPLACE = 16;
    }
}

ferroui_base::ferro_markup_enum!(flags SelectionOptions {
    None = SelectionOptions::NONE,
    StartActive = SelectionOptions::START_ACTIVE,
    AtEndOfLine = SelectionOptions::AT_END_OF_LINE,
    Overtype = SelectionOptions::OVERTYPE,
    Active = SelectionOptions::ACTIVE,
    Replace = SelectionOptions::REPLACE,
}, { namespace: "FerroUI.Controls" });

/// Native selection classifications. The current editor always owns one text
/// selection and reports InsertionPoint or Normal. None and the shape values
/// preserve the public enum contract; object/shape selection detection requires
/// a separate model contract and is not inferred from an embedded control.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum SelectionType {
    None = 0,
    InsertionPoint = 1,
    Normal = 2,
    InlineShape = 7,
    Shape = 8,
}

ferroui_base::ferro_markup_enum!(SelectionType {
    None, InsertionPoint, Normal, InlineShape, Shape,
}, { namespace: "FerroUI.Controls" });

impl RichEditTextSelection {
    /// Reads the shared editor modes and the accepted selection direction.
    pub fn options(&self) -> SelectionOptions {
        self.editor.selection_options()
    }

    /// Changes direction and modes through the control's selection projection.
    /// A cancelled or redirected direction request does not install stale modes;
    /// a newer nested request keeps ownership. Unknown flag bits are rejected.
    pub fn set_options(&self, options: SelectionOptions) -> Result<(), EditError> {
        self.editor.set_selection_options(options)
    }

    pub fn selection_type(&self) -> SelectionType {
        self.editor.selection_type()
    }
}
