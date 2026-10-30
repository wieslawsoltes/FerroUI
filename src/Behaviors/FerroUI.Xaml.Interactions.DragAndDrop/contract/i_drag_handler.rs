//! Port of `Contract/IDragHandler.cs`.

use ferroui_base::input::PointerEventArgs;
use ferroui_base::BoxedValue;

/// Provides callbacks for drag operations.
///
/// `sender` is the element the drag starts from (an untyped value holding the object), `e` the
/// pointer event that started the drag and `context` the data dragged.
pub trait IDragHandler {
    /// Called before the drag operation begins.
    fn before_drag_drop(&self, sender: Option<&BoxedValue>, e: &PointerEventArgs, context: Option<&BoxedValue>);

    /// Called after the drag operation completes.
    fn after_drag_drop(&self, sender: Option<&BoxedValue>, e: &PointerEventArgs, context: Option<&BoxedValue>);
}

/// Handlers compare by identity (reference equality), so that they can be held in property and
/// untyped values.
impl PartialEq for dyn IDragHandler {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        std::ptr::addr_eq(self as *const Self, other as *const Self)
    }
}
