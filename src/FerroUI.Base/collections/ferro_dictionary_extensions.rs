use super::ferro_list_extensions::weak_subscribe;
use super::{CollectionChangedHandler, IFerroReadOnlyDictionary, NotifyCollectionChangedAction, NotifyCollectionChangedEventArgs};
use crate::reactive::{Disposable, IDisposable};
use crate::utilities::WeakEventSender;
use std::rc::Rc;

/// Defines extension methods for working with [`FerroDictionary`](super::FerroDictionary)s.
pub struct FerroDictionaryExtensions;

impl FerroDictionaryExtensions {
    /// Invokes an action for each item in a collection and subsequently each
    /// item added or removed from the collection.
    ///
    /// `added` is called initially for each item in the collection and
    /// subsequently for each item added to the collection, with the key and
    /// the value; `removed` for each item removed from the collection;
    /// `reset` when the collection is reset, which is followed by calls to
    /// `added` for each item present in the collection after the reset.
    /// `weak_subscription` indicates if a weak subscription should be used to
    /// track changes to the collection (held by the returned disposable, see
    /// [`FerroListExtensions::for_each_item_indexed`](super::FerroListExtensions::for_each_item_indexed)).
    /// Returns a disposable used to terminate the subscription.
    pub fn for_each_item<K, V, D>(
        collection: &D,
        added: impl Fn(&K, &V) + 'static,
        removed: impl Fn(&K, &V) + 'static,
        reset: impl Fn() + 'static,
        weak_subscription: bool,
    ) -> Rc<dyn IDisposable>
    where
        K: Clone + 'static,
        V: Clone + 'static,
        D: IFerroReadOnlyDictionary<K, V> + WeakEventSender,
    {
        let add = Rc::new(move |items: &[(K, V)]| {
            for (key, value) in items {
                added(key, value);
            }
        });

        let remove = move |items: &[(K, V)]| {
            for (key, value) in items {
                removed(key, value);
            }
        };

        // The handler belongs to the collection, so it reads the collection
        // through a weak reference.
        let source = collection.downgrade_sender();
        let add_items = add.clone();
        let handler: Rc<CollectionChangedHandler<(K, V)>> =
            Rc::new(move |e: &NotifyCollectionChangedEventArgs<'_, (K, V)>| match e.action {
                NotifyCollectionChangedAction::Add => add_items(e.new_items),

                NotifyCollectionChangedAction::Move | NotifyCollectionChangedAction::Replace => {
                    remove(e.old_items);
                    add_items(e.new_items);
                }

                NotifyCollectionChangedAction::Remove => remove(e.old_items),

                NotifyCollectionChangedAction::Reset => {
                    reset();
                    if let Some(collection) = D::upgrade_sender(&source) {
                        add_items(&collection.to_vec());
                    }
                }
            });

        add(&collection.to_vec());

        if weak_subscription {
            weak_subscribe(collection, handler, D::add_collection_changed, D::remove_collection_changed)
        } else {
            let token = collection.add_collection_changed(handler);

            let collection = collection.clone();
            Disposable::create(move || {
                collection.remove_collection_changed(token);
            })
        }
    }
}

#[cfg(test)]
mod tests {
    //! Not from upstream: the managed test suite has no tests for these
    //! extensions.

    use super::*;
    use crate::collections::FerroDictionary;
    use std::cell::RefCell;

    #[test]
    fn for_each_item_reports_initial_added_replaced_and_removed_entries() {
        let dictionary = FerroDictionary::from_dictionary([("a".to_string(), 1)]);
        let log = Rc::new(RefCell::new(Vec::new()));
        let (a, r, z) = (log.clone(), log.clone(), log.clone());

        let subscription = FerroDictionaryExtensions::for_each_item(
            &dictionary,
            move |k: &String, v: &i32| a.borrow_mut().push(format!("+{k}={v}")),
            move |k: &String, v: &i32| r.borrow_mut().push(format!("-{k}={v}")),
            move || z.borrow_mut().push("reset".to_string()),
            false,
        );
        dictionary.add("b".to_string(), 2);
        dictionary.set("a".to_string(), 3);
        dictionary.remove(&"b".to_string());
        subscription.dispose();
        dictionary.add("c".to_string(), 4);

        assert_eq!(*log.borrow(), ["+a=1", "+b=2", "-a=1", "+a=3", "-b=2"].map(String::from).to_vec());
    }
}
