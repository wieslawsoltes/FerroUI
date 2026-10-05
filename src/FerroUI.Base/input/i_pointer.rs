use super::InputElement;
use crate::Ref;
use std::any::Any;

/// Identifies a specific input pointer, e.g. a mouse, a touch contact or a
/// pen.
pub trait IPointer {
    /// The unique identifier of the pointer.
    fn id(&self) -> i32;

    /// Captures pointer input to the specified control.
    ///
    /// When an element captures the pointer, it receives pointer input
    /// whether the cursor is within the control's bounds or not. The current
    /// pointer capture control is exposed by [`captured`](Self::captured).
    fn capture(&self, control: Option<&Ref<InputElement>>);

    /// The control that is currently capturing the pointer, if any.
    fn captured(&self) -> Option<Ref<InputElement>>;

    /// The type of the pointer.
    fn type_(&self) -> PointerType;

    /// Whether the pointer is the primary pointer of its type.
    fn is_primary(&self) -> bool;

    /// The pointer as [`Any`], for downcasting to the concrete pointer.
    fn as_any(&self) -> &dyn Any;
}

/// Enumerates the types of pointer devices.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum PointerType {
    /// The pointer device is a mouse.
    #[default]
    Mouse,
    /// The pointer device is a touch contact.
    Touch,
    /// The pointer device is a pen.
    Pen,
}
