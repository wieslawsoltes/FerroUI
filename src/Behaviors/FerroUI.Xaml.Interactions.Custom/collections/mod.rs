//! The actions that change a collection and the behavior and the trigger of the changes of a
//! collection (`Collections/`).
//!
//! The lists of the reference (`IList`, `IEnumerable`, `INotifyCollectionChanged`) are untyped
//! lists of the framework here ([`ItemsSource`](ferroui_controls::ItemsSource)), which a binding
//! makes of a `FerroList` or an `ObservableCollection` as it does for the items of an items
//! control.

mod add_range_action;
mod clear_collection_action;
mod collection_changed_behavior;
mod collection_changed_trigger;
mod remove_range_action;

pub use add_range_action::AddRangeAction;
pub use clear_collection_action::ClearCollectionAction;
pub use collection_changed_behavior::CollectionChangedBehavior;
pub use collection_changed_trigger::CollectionChangedTrigger;
pub use remove_range_action::RemoveRangeAction;

/// Not from the reference: the subscription to the changes of a collection
/// (`collection.CollectionChanged += handler` and `-=`).
pub(crate) mod collection_changed {
    use ferroui_base::data::model::CollectionChange;
    use ferroui_base::reactive::{Disposable, IDisposable};
    use ferroui_controls::{ItemsChangedEventArgs, ItemsChangedHandler, ItemsSource};
    use std::rc::Rc;

    /// Subscribes `handler` to the changes of `collection`; the subscription removes it. `None`
    /// when the collection does not notify its changes (it is not an `INotifyCollectionChanged`).
    ///
    /// Deviation (DEVIATIONS.md, Behaviors): the handler receives an untyped description of the
    /// change (`CollectionChange`: the action, the starting indexes and the numbers of items),
    /// which the actions get as their parameter; the reference passes the
    /// `NotifyCollectionChangedEventArgs`, whose items are views of the list here and cannot
    /// outlive the notification.
    pub(crate) fn subscribe(
        collection: &ItemsSource,
        handler: impl Fn(CollectionChange) + 'static,
    ) -> Option<Rc<dyn IDisposable>> {
        let handler: Rc<ItemsChangedHandler> = Rc::new(move |e: &ItemsChangedEventArgs<'_>| {
            handler(CollectionChange {
                action: e.action,
                new_starting_index: e.new_starting_index,
                new_count: e.new_items.len(),
                old_starting_index: e.old_starting_index,
                old_count: e.old_items.len(),
            });
        });
        let token = collection.list().add_collection_changed(handler)?;
        let collection = collection.clone();
        Some(Disposable::create(move || collection.list().remove_collection_changed(token)))
    }
}
