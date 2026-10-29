//! Port of `Cursor/Contract/ICursorProvider.cs`.

use ferroui_base::input::Cursor;
use std::rc::Rc;

/// Provides a custom cursor instance.
pub trait ICursorProvider {
    /// Creates a cursor instance.
    ///
    /// Returns the created [`Cursor`].
    fn create_cursor(&self) -> Rc<Cursor>;
}

/// Providers compare by identity (reference equality), so that they can be held in property and
/// untyped values.
impl PartialEq for dyn ICursorProvider {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        std::ptr::addr_eq(self as *const Self, other as *const Self)
    }
}

ferroui_base::ferro_markup_type!(interface dyn ICursorProvider as "ICursorProvider" {
    handles: [Rc<dyn ICursorProvider>, Option<Rc<dyn ICursorProvider>>],
});
