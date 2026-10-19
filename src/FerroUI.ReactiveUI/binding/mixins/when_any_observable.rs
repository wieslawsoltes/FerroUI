//! `ReactiveUIBindingExtensions.WhenAnyObservable.WideArity.cs`: `WhenAnyObservable` of paths that
//! hold observables of one type, merged.

use super::observable_chain_helpers::{switch_latest, ObservableLeaf};
use ferroui_reactive::linq::Observable;
use ferroui_system::component_model::PropertyPath;
use ferroui_system::IObservable;
use std::rc::Rc;

/// The paths of one `WhenAnyObservable` of an object of type `T`: one path to a property that
/// holds an observable, or a tuple of 2 to 16 of them whose observables produce one type.
// Deviation (DEVIATIONS.md, ReactiveUI): the arity is a tuple of paths, from 1 to 16, where
// upstream has overloads of 1 to 16 expressions.
pub trait WhenAnyObservablePaths<T: 'static>: 'static {
    /// The type of the values of the observables.
    type Item: 'static;

    /// The values of the latest observable each path holds on `source`, merged.
    fn observe(self, source: &Rc<T>) -> Rc<dyn IObservable<Self::Item>>;
}

impl<T: 'static, L: ObservableLeaf> WhenAnyObservablePaths<T> for PropertyPath<Rc<T>, L> {
    type Item = L::Item;

    fn observe(self, source: &Rc<T>) -> Rc<dyn IObservable<L::Item>> {
        switch_latest(source, self)
    }
}

macro_rules! when_any_observable_paths_tuple {
    ($L1:ident $path1:ident $(, $L:ident $path:ident)+) => {
        impl<T: 'static, $L1: ObservableLeaf $(, $L: ObservableLeaf<Item = $L1::Item>)+> WhenAnyObservablePaths<T>
            for (PropertyPath<Rc<T>, $L1>, $(PropertyPath<Rc<T>, $L>,)+)
        {
            type Item = $L1::Item;

            fn observe(self, source: &Rc<T>) -> Rc<dyn IObservable<Self::Item>> {
                let ($path1, $($path,)+) = self;
                Observable::merge_all(vec![switch_latest(source, $path1) $(, switch_latest(source, $path))+])
            }
        }
    };
}

when_any_observable_paths_tuple!(L1 path1, L2 path2);
when_any_observable_paths_tuple!(L1 path1, L2 path2, L3 path3);
when_any_observable_paths_tuple!(L1 path1, L2 path2, L3 path3, L4 path4);
when_any_observable_paths_tuple!(L1 path1, L2 path2, L3 path3, L4 path4, L5 path5);
when_any_observable_paths_tuple!(L1 path1, L2 path2, L3 path3, L4 path4, L5 path5, L6 path6);
when_any_observable_paths_tuple!(L1 path1, L2 path2, L3 path3, L4 path4, L5 path5, L6 path6, L7 path7);
when_any_observable_paths_tuple!(L1 path1, L2 path2, L3 path3, L4 path4, L5 path5, L6 path6, L7 path7, L8 path8);
when_any_observable_paths_tuple!(L1 path1, L2 path2, L3 path3, L4 path4, L5 path5, L6 path6, L7 path7, L8 path8, L9 path9);
when_any_observable_paths_tuple!(L1 path1, L2 path2, L3 path3, L4 path4, L5 path5, L6 path6, L7 path7, L8 path8, L9 path9, L10 path10);
when_any_observable_paths_tuple!(L1 path1, L2 path2, L3 path3, L4 path4, L5 path5, L6 path6, L7 path7, L8 path8, L9 path9, L10 path10, L11 path11);
when_any_observable_paths_tuple!(L1 path1, L2 path2, L3 path3, L4 path4, L5 path5, L6 path6, L7 path7, L8 path8, L9 path9, L10 path10, L11 path11, L12 path12);
when_any_observable_paths_tuple!(L1 path1, L2 path2, L3 path3, L4 path4, L5 path5, L6 path6, L7 path7, L8 path8, L9 path9, L10 path10, L11 path11, L12 path12, L13 path13);
when_any_observable_paths_tuple!(L1 path1, L2 path2, L3 path3, L4 path4, L5 path5, L6 path6, L7 path7, L8 path8, L9 path9, L10 path10, L11 path11, L12 path12, L13 path13, L14 path14);
when_any_observable_paths_tuple!(L1 path1, L2 path2, L3 path3, L4 path4, L5 path5, L6 path6, L7 path7, L8 path8, L9 path9, L10 path10, L11 path11, L12 path12, L13 path13, L14 path14, L15 path15);
when_any_observable_paths_tuple!(L1 path1, L2 path2, L3 path3, L4 path4, L5 path5, L6 path6, L7 path7, L8 path8, L9 path9, L10 path10, L11 path11, L12 path12, L13 path13, L14 path14, L15 path15, L16 path16);

/// The values of the latest observable that each path holds on `source`, merged (`WhenAnyObservable`).
pub(crate) fn when_any_observable<T: 'static, P: WhenAnyObservablePaths<T>>(source: &Rc<T>, paths: P) -> Rc<dyn IObservable<P::Item>> {
    paths.observe(source)
}
