//! Port of `DropHandlerBase.cs`.

use crate::IDropHandler;
use ferroui_base::collections::FerroList;
use ferroui_base::input::{DragDropEffects, DragEventArgs};
use ferroui_base::interactivity::RoutedEventArgs;
use ferroui_base::BoxedValue;

/// Provides helper methods for custom drop handlers.
///
/// Deviation (DEVIATIONS.md, Behaviors): the abstract class of the reference is a trait whose
/// methods have the bodies of the virtual methods of the class, and every type that implements it
/// is an [`IDropHandler`]. A handler overrides [`validate`](Self::validate) and
/// [`execute`](Self::execute) (and the others when it needs to); an override calls the body of the
/// base class through the `base_*` method of the same name. The `IList<T>` helpers work on a
/// [`FerroList`], and the overloads of `MoveItem` and `SwapItem` that take two collections are
/// [`move_item_between`](Self::move_item_between) and [`swap_item_between`](Self::swap_item_between).
/// Call the methods with the trait named (`DropHandlerBase::enter(handler, ..)`)
/// where [`IDropHandler`] is in scope too: the two traits have the same method names.
pub trait DropHandlerBase {
    /// Moves an item within a collection.
    fn move_item<T: Clone>(&self, items: &FerroList<T>, source_index: usize, target_index: usize)
    where
        Self: Sized,
    {
        if source_index < target_index {
            let item = items.get(source_index);
            items.remove_at(source_index);
            items.insert(target_index, item);
        } else {
            let remove_index = source_index + 1;
            if items.count() + 1 > remove_index {
                let item = items.get(source_index);
                items.remove_at(remove_index - 1);
                items.insert(target_index, item);
            }
        }
    }

    /// Moves an item from one collection to another.
    fn move_item_between<T: Clone>(
        &self,
        source_items: &FerroList<T>,
        target_items: &FerroList<T>,
        source_index: usize,
        target_index: usize,
    ) where
        Self: Sized,
    {
        let item = source_items.get(source_index);
        source_items.remove_at(source_index);
        target_items.insert(target_index, item);
    }

    /// Swaps two items inside a collection.
    fn swap_item<T: Clone + PartialEq>(&self, items: &FerroList<T>, source_index: usize, target_index: usize)
    where
        Self: Sized,
    {
        let item1 = items.get(source_index);
        let item2 = items.get(target_index);
        items.set(target_index, item1);
        items.set(source_index, item2);
    }

    /// Swaps an item of one collection with an item of another.
    fn swap_item_between<T: Clone + PartialEq>(
        &self,
        source_items: &FerroList<T>,
        target_items: &FerroList<T>,
        source_index: usize,
        target_index: usize,
    ) where
        Self: Sized,
    {
        let item1 = source_items.get(source_index);
        let item2 = target_items.get(target_index);
        target_items.set(target_index, item1);
        source_items.set(source_index, item2);
    }

    /// Inserts an item into a collection at the specified index.
    fn insert_item<T: Clone>(&self, items: &FerroList<T>, item: T, index: usize)
    where
        Self: Sized,
    {
        items.insert(index, item);
    }

    /// Called when a drag enters the target.
    fn enter(
        &self,
        sender: Option<&BoxedValue>,
        e: &DragEventArgs,
        source_context: Option<&BoxedValue>,
        target_context: Option<&BoxedValue>,
    ) {
        DropHandlerBase::base_enter(self, sender, e, source_context, target_context);
    }

    /// The body of [`enter`](Self::enter) of the base class: the effects the target accepts, from
    /// [`validate`](Self::validate).
    fn base_enter(
        &self,
        sender: Option<&BoxedValue>,
        e: &DragEventArgs,
        source_context: Option<&BoxedValue>,
        target_context: Option<&BoxedValue>,
    ) {
        if !DropHandlerBase::validate(self, sender, e, source_context, target_context, None) {
            e.set_drag_effects(DragDropEffects::NONE);
            e.set_handled(true);
        } else {
            e.set_drag_effects(e.drag_effects() | DragDropEffects::COPY | DragDropEffects::MOVE | DragDropEffects::LINK);
            e.set_handled(true);
        }
    }

    /// Called continuously while a drag is over the target.
    fn over(
        &self,
        sender: Option<&BoxedValue>,
        e: &DragEventArgs,
        source_context: Option<&BoxedValue>,
        target_context: Option<&BoxedValue>,
    ) {
        DropHandlerBase::base_over(self, sender, e, source_context, target_context);
    }

    /// The body of [`over`](Self::over) of the base class: the effects the target accepts, from
    /// [`validate`](Self::validate).
    fn base_over(
        &self,
        sender: Option<&BoxedValue>,
        e: &DragEventArgs,
        source_context: Option<&BoxedValue>,
        target_context: Option<&BoxedValue>,
    ) {
        if !DropHandlerBase::validate(self, sender, e, source_context, target_context, None) {
            e.set_drag_effects(DragDropEffects::NONE);
            e.set_handled(true);
        } else {
            e.set_drag_effects(e.drag_effects() | DragDropEffects::COPY | DragDropEffects::MOVE | DragDropEffects::LINK);
            e.set_handled(true);
        }
    }

    /// Called when the drag is dropped on the target.
    fn drop(
        &self,
        sender: Option<&BoxedValue>,
        e: &DragEventArgs,
        source_context: Option<&BoxedValue>,
        target_context: Option<&BoxedValue>,
    ) {
        DropHandlerBase::base_drop(self, sender, e, source_context, target_context);
    }

    /// The body of [`drop`](Self::drop) of the base class: the effects of the drop, from
    /// [`execute`](Self::execute).
    fn base_drop(
        &self,
        sender: Option<&BoxedValue>,
        e: &DragEventArgs,
        source_context: Option<&BoxedValue>,
        target_context: Option<&BoxedValue>,
    ) {
        if !DropHandlerBase::execute(self, sender, e, source_context, target_context, None) {
            e.set_drag_effects(DragDropEffects::NONE);
            e.set_handled(true);
        } else {
            e.set_drag_effects(e.drag_effects() | DragDropEffects::COPY | DragDropEffects::MOVE | DragDropEffects::LINK);
            e.set_handled(true);
        }
    }

    /// Called when the drag leaves the target.
    fn leave(&self, sender: Option<&BoxedValue>, e: &RoutedEventArgs) {
        DropHandlerBase::base_leave(self, sender, e);
    }

    /// The body of [`leave`](Self::leave) of the base class: [`cancel`](Self::cancel).
    fn base_leave(&self, sender: Option<&BoxedValue>, e: &RoutedEventArgs) {
        DropHandlerBase::cancel(self, sender, e);
    }

    /// Validates whether the drag operation can be performed.
    fn validate(
        &self,
        _sender: Option<&BoxedValue>,
        _e: &DragEventArgs,
        _source_context: Option<&BoxedValue>,
        _target_context: Option<&BoxedValue>,
        _state: Option<&BoxedValue>,
    ) -> bool {
        false
    }

    /// Executes the drop operation.
    fn execute(
        &self,
        _sender: Option<&BoxedValue>,
        _e: &DragEventArgs,
        _source_context: Option<&BoxedValue>,
        _target_context: Option<&BoxedValue>,
        _state: Option<&BoxedValue>,
    ) -> bool {
        false
    }

    /// Cancels the drag operation.
    fn cancel(&self, _sender: Option<&BoxedValue>, _e: &RoutedEventArgs) {}
}

/// Every drop handler base is a drop handler, with the methods of the class.
impl<H: DropHandlerBase> IDropHandler for H {
    fn enter(
        &self,
        sender: Option<&BoxedValue>,
        e: &DragEventArgs,
        source_context: Option<&BoxedValue>,
        target_context: Option<&BoxedValue>,
    ) {
        DropHandlerBase::enter(self, sender, e, source_context, target_context)
    }

    fn over(
        &self,
        sender: Option<&BoxedValue>,
        e: &DragEventArgs,
        source_context: Option<&BoxedValue>,
        target_context: Option<&BoxedValue>,
    ) {
        DropHandlerBase::over(self, sender, e, source_context, target_context)
    }

    fn drop(
        &self,
        sender: Option<&BoxedValue>,
        e: &DragEventArgs,
        source_context: Option<&BoxedValue>,
        target_context: Option<&BoxedValue>,
    ) {
        DropHandlerBase::drop(self, sender, e, source_context, target_context)
    }

    fn leave(&self, sender: Option<&BoxedValue>, e: &RoutedEventArgs) {
        DropHandlerBase::leave(self, sender, e)
    }

    fn validate(
        &self,
        sender: Option<&BoxedValue>,
        e: &DragEventArgs,
        source_context: Option<&BoxedValue>,
        target_context: Option<&BoxedValue>,
        state: Option<&BoxedValue>,
    ) -> bool {
        DropHandlerBase::validate(self, sender, e, source_context, target_context, state)
    }

    fn execute(
        &self,
        sender: Option<&BoxedValue>,
        e: &DragEventArgs,
        source_context: Option<&BoxedValue>,
        target_context: Option<&BoxedValue>,
        state: Option<&BoxedValue>,
    ) -> bool {
        DropHandlerBase::execute(self, sender, e, source_context, target_context, state)
    }

    fn cancel(&self, sender: Option<&BoxedValue>, e: &RoutedEventArgs) {
        DropHandlerBase::cancel(self, sender, e)
    }
}
