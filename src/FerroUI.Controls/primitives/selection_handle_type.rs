/// Represents which part of the selection a text selection handle controls.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum SelectionHandleType {
    /// The handle controls the caret position.
    Caret = 0,

    /// The handle controls the start of the text selection.
    Start = 1,

    /// The handle controls the end of the text selection.
    End = 2,
}
