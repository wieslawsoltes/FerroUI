//! Port of `Collections/ActionCollection.cs`.

use crate::IAction;
use ferroui_base::collections::{FerroList, NotifyCollectionChangedAction, NotifyCollectionChangedEventArgs, WeakFerroList};
use ferroui_base::utilities::WeakEventSender;
use ferroui_base::{FerroObject, Ref};
use std::ops::Deref;
use std::rc::Rc;

/// Represents a collection of [`IAction`]s.
///
/// The collection is a notifying list of objects (`FerroList<FerroObject>`); every item must be
/// an action. It is a reference object: clones refer to the same collection and compare by
/// identity.
#[derive(Clone, PartialEq)]
pub struct ActionCollection(FerroList<Ref<FerroObject>>);

impl ActionCollection {
    /// Initializes a new instance of the [`ActionCollection`] class.
    pub fn new() -> Self {
        let list: FerroList<Ref<FerroObject>> = FerroList::new();
        let weak = list.downgrade_sender();
        list.add_collection_changed(Rc::new(move |e: &NotifyCollectionChangedEventArgs<'_, Ref<FerroObject>>| {
            Self::action_collection_collection_changed(&weak, e);
        }));
        Self(list)
    }

    /// The list of the collection.
    pub fn list(&self) -> &FerroList<Ref<FerroObject>> {
        &self.0
    }

    fn action_collection_collection_changed(
        this: &WeakFerroList<Ref<FerroObject>>,
        event_args: &NotifyCollectionChangedEventArgs<'_, Ref<FerroObject>>,
    ) {
        match event_args.action {
            NotifyCollectionChangedAction::Reset => {
                if let Some(this) = <FerroList<Ref<FerroObject>> as WeakEventSender>::upgrade_sender(this) {
                    for item in this.snapshot().iter() {
                        Self::verify_type(Some(item));
                    }
                }
            }
            NotifyCollectionChangedAction::Add | NotifyCollectionChangedAction::Replace => {
                let changed_item = event_args.new_items.first();
                Self::verify_type(changed_item);
            }
            _ => {}
        }
    }

    fn verify_type(item: Option<&Ref<FerroObject>>) {
        let Some(item) = item else {
            return;
        };

        if <dyn IAction>::of(item).is_none() {
            panic!("Only IAction types are supported in an ActionCollection.");
        }
    }
}

impl Default for ActionCollection {
    fn default() -> Self {
        Self::new()
    }
}

impl Deref for ActionCollection {
    type Target = FerroList<Ref<FerroObject>>;

    fn deref(&self) -> &FerroList<Ref<FerroObject>> {
        &self.0
    }
}

impl std::fmt::Debug for ActionCollection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ActionCollection({})", self.0.count())
    }
}
