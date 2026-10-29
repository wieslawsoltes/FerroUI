//! Port of `Contract/IDropHandler.cs`.

use ferroui_base::input::DragEventArgs;
use ferroui_base::interactivity::RoutedEventArgs;
use ferroui_base::BoxedValue;

/// Defines callbacks for handling drag-and-drop operations.
///
/// `sender` is the drop target (an untyped value holding the object), `source_context` the data
/// dragged and `target_context` the context of the target.
pub trait IDropHandler {
    /// Called when a drag enters the target.
    fn enter(
        &self,
        sender: Option<&BoxedValue>,
        e: &DragEventArgs,
        source_context: Option<&BoxedValue>,
        target_context: Option<&BoxedValue>,
    );

    /// Called repeatedly as the drag moves over the target.
    fn over(
        &self,
        sender: Option<&BoxedValue>,
        e: &DragEventArgs,
        source_context: Option<&BoxedValue>,
        target_context: Option<&BoxedValue>,
    );

    /// Called when the drag is dropped on the target.
    fn drop(
        &self,
        sender: Option<&BoxedValue>,
        e: &DragEventArgs,
        source_context: Option<&BoxedValue>,
        target_context: Option<&BoxedValue>,
    );

    /// Called when the drag leaves the target.
    fn leave(&self, sender: Option<&BoxedValue>, e: &RoutedEventArgs);

    /// Validates whether the drag can be executed.
    fn validate(
        &self,
        sender: Option<&BoxedValue>,
        e: &DragEventArgs,
        source_context: Option<&BoxedValue>,
        target_context: Option<&BoxedValue>,
        state: Option<&BoxedValue>,
    ) -> bool;

    /// Executes the drop logic.
    fn execute(
        &self,
        sender: Option<&BoxedValue>,
        e: &DragEventArgs,
        source_context: Option<&BoxedValue>,
        target_context: Option<&BoxedValue>,
        state: Option<&BoxedValue>,
    ) -> bool;

    /// Cancels the drag operation.
    fn cancel(&self, sender: Option<&BoxedValue>, e: &RoutedEventArgs);
}

/// Handlers compare by identity (reference equality), so that they can be held in property and
/// untyped values.
impl PartialEq for dyn IDropHandler {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        std::ptr::addr_eq(self as *const Self, other as *const Self)
    }
}
