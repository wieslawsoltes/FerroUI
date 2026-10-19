//! `TransformToTree` (`Cache/ObservableCacheEx.TransformToTree.cs`).

macro_rules! transform_to_tree_methods {
    () => {
        /// The items as the nodes of a tree, each the child of the node whose key `pivot_on`
        /// gives for it; the sequence has the roots
        /// (`TransformToTree(Func<TObject, TKey>, IObservable<Func<Node<TObject, TKey>, bool>>? predicateChanged = null)`).
        fn transform_to_tree(
            &self,
            pivot_on: impl Fn(&T) -> K + 'static,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::ChangeSet<std::rc::Rc<crate::cache::Node<T, K>>, K>>> {
            crate::cache::internal::tree_builder(self.change_sets(), std::rc::Rc::new(pivot_on), None)
        }

        /// [`transform_to_tree`](Self::transform_to_tree) whose sequence has the nodes that the
        /// latest predicate of `predicate_changed` accepts, instead of the roots
        /// (`TransformToTree(Func<TObject, TKey>, IObservable<Func<Node<TObject, TKey>, bool>>)`).
        fn transform_to_tree_with(
            &self,
            pivot_on: impl Fn(&T) -> K + 'static,
            predicate_changed: std::rc::Rc<dyn ferroui_system::IObservable<std::rc::Rc<dyn Fn(&std::rc::Rc<crate::cache::Node<T, K>>) -> bool>>>,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::cache::ChangeSet<std::rc::Rc<crate::cache::Node<T, K>>, K>>> {
            crate::cache::internal::tree_builder(self.change_sets(), std::rc::Rc::new(pivot_on), Some(predicate_changed))
        }
    };
}
pub(crate) use transform_to_tree_methods;
