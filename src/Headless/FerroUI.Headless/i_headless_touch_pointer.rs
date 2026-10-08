//! Port of `IHeadlessTouchPointer.cs`.

use ferroui_base::reactive::IDisposable;
use std::any::Any;

/// Represents an active touch contact simulated on a headless window/toplevel.
/// Use [`touch_move`](crate::HeadlessWindowExtensions::touch_move) and
/// [`touch_end`](crate::HeadlessWindowExtensions::touch_end) to drive the
/// contact.
///
/// Disposing the touch pointer cancels the contact if it hasn't been
/// released with `touch_end` yet.
///
/// Not client implementable: the extensions only accept the touch pointers
/// they created.
pub trait IHeadlessTouchPointer: IDisposable {
    /// The touch pointer as its concrete type.
    fn as_any(&self) -> &dyn Any;
}
