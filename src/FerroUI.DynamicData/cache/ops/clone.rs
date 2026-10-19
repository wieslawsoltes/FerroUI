//! `Clone` (`Cache/ObservableCacheEx.Clone.cs`).

macro_rules! clone_methods {
    () => {
        /// The change sets, applied to `target` as they pass: an addition adds the item, an update
        /// replaces the previous item by it, and a removal removes it (`Clone(ICollection<TObject>)`;
        /// named `clone_to` beside `Clone::clone`).
        fn clone_to(&self, target: std::rc::Rc<std::cell::RefCell<Vec<T>>>) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::ChangeSet<T, K>>>
        where
            T: PartialEq,
        {
            ferroui_reactive::ObservableExt::do_(&self.change_sets(), move |changes: &crate::cache::ChangeSet<T, K>| {
                let mut target = target.borrow_mut();
                for item in changes {
                    match item.reason() {
                        crate::cache::ChangeReason::Add => target.push(item.current().clone()),
                        crate::cache::ChangeReason::Update => {
                            if let Some(previous) = item.previous() {
                                if let Some(index) = target.iter().position(|candidate| candidate == previous) {
                                    target.remove(index);
                                }
                            }
                            target.push(item.current().clone());
                        }
                        crate::cache::ChangeReason::Remove => {
                            if let Some(index) = target.iter().position(|candidate| candidate == item.current()) {
                                target.remove(index);
                            }
                        }
                        crate::cache::ChangeReason::Refresh | crate::cache::ChangeReason::Moved => {}
                    }
                }
            })
        }
    };
}
pub(crate) use clone_methods;
