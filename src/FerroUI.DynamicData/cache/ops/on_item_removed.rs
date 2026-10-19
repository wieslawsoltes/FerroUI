//! `OnItemRemoved` (`Cache/ObservableCacheEx.OnItemRemoved.cs`).

macro_rules! on_item_removed_methods {
    () => {
        /// Runs `remove_action` for each item removed, before the change set is forwarded, and for
        /// every item left when the subscription ends (`OnItemRemoved(Action<TObject>, bool
        /// invokeOnUnsubscribe = true)`).
        fn on_item_removed(&self, remove_action: impl Fn(&T) + 'static) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::ChangeSet<T, K>>> {
            crate::cache::internal::on_being_removed(self.change_sets(), std::rc::Rc::new(move |item: &T, _: &K| remove_action(item)))
        }

        /// [`on_item_removed`](Self::on_item_removed) with the key
        /// (`OnItemRemoved(Action<TObject, TKey>, bool invokeOnUnsubscribe = true)`).
        fn on_item_removed_with_key(&self, remove_action: impl Fn(&T, &K) + 'static) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::ChangeSet<T, K>>> {
            crate::cache::internal::on_being_removed(self.change_sets(), std::rc::Rc::new(remove_action))
        }

        /// [`on_item_removed_with_key`](Self::on_item_removed_with_key), for the items left when the
        /// subscription ends only when `invoke_on_unsubscribe` is true
        /// (`OnItemRemoved(Action<TObject, TKey>, bool invokeOnUnsubscribe)`).
        fn on_item_removed_with(
            &self,
            remove_action: impl Fn(&T, &K) + 'static,
            invoke_on_unsubscribe: bool,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::ChangeSet<T, K>>> {
            if invoke_on_unsubscribe {
                crate::cache::internal::on_being_removed(self.change_sets(), std::rc::Rc::new(remove_action))
            } else {
                crate::cache::internal::on_change_reason(self.change_sets(), crate::cache::ChangeReason::Remove, remove_action)
            }
        }
    };
}
pub(crate) use on_item_removed_methods;
