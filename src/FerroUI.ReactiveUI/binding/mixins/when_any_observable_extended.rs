//! `ReactiveUIBindingExtensions.WhenAnyObservable.Extended.WideArity.cs`: `WhenAnyObservable` of
//! paths that hold observables of different types, combined by a selector.

use super::observable_chain_helpers::{switch_latest, ObservableLeaf};
use crate::binding::observables::CombineLatestObservable;
use ferroui_reactive::ObservableExt;
use ferroui_system::component_model::PropertyPath;
use ferroui_system::IObservable;
use std::rc::Rc;

/// The paths of one `WhenAnyObservable` with a selector: a tuple of 2 to 16 paths to properties
/// that hold observables, whose latest values combine.
// Deviation (DEVIATIONS.md, ReactiveUI): the arity is a tuple of paths, from 2 to 16, where
// upstream has overloads of 2 to 16 expressions.
pub trait WhenAnyObservableCombinePaths<T: 'static>: 'static {
    /// The tuple of the latest values of the observables.
    type Items: Clone + 'static;

    /// The latest values of the latest observable each path holds on `source`, combined.
    fn observe_combined(self, source: &Rc<T>) -> Rc<dyn IObservable<Self::Items>>;
}

macro_rules! when_any_observable_combine_paths_tuple {
    ($($L:ident $path:ident),+) => {
        impl<T: 'static, $($L: ObservableLeaf),+> WhenAnyObservableCombinePaths<T> for ($(PropertyPath<Rc<T>, $L>,)+)
        where
            $($L::Item: Clone,)+
        {
            type Items = ($($L::Item,)+);

            fn observe_combined(self, source: &Rc<T>) -> Rc<dyn IObservable<Self::Items>> {
                let ($($path,)+) = self;
                CombineLatestObservable::create_tuple(($(switch_latest(source, $path),)+))
            }
        }
    };
}

when_any_observable_combine_paths_tuple!(L1 path1, L2 path2);
when_any_observable_combine_paths_tuple!(L1 path1, L2 path2, L3 path3);
when_any_observable_combine_paths_tuple!(L1 path1, L2 path2, L3 path3, L4 path4);
when_any_observable_combine_paths_tuple!(L1 path1, L2 path2, L3 path3, L4 path4, L5 path5);
when_any_observable_combine_paths_tuple!(L1 path1, L2 path2, L3 path3, L4 path4, L5 path5, L6 path6);
when_any_observable_combine_paths_tuple!(L1 path1, L2 path2, L3 path3, L4 path4, L5 path5, L6 path6, L7 path7);
when_any_observable_combine_paths_tuple!(L1 path1, L2 path2, L3 path3, L4 path4, L5 path5, L6 path6, L7 path7, L8 path8);
when_any_observable_combine_paths_tuple!(L1 path1, L2 path2, L3 path3, L4 path4, L5 path5, L6 path6, L7 path7, L8 path8, L9 path9);
when_any_observable_combine_paths_tuple!(L1 path1, L2 path2, L3 path3, L4 path4, L5 path5, L6 path6, L7 path7, L8 path8, L9 path9, L10 path10);
when_any_observable_combine_paths_tuple!(L1 path1, L2 path2, L3 path3, L4 path4, L5 path5, L6 path6, L7 path7, L8 path8, L9 path9, L10 path10, L11 path11);
when_any_observable_combine_paths_tuple!(L1 path1, L2 path2, L3 path3, L4 path4, L5 path5, L6 path6, L7 path7, L8 path8, L9 path9, L10 path10, L11 path11, L12 path12);
when_any_observable_combine_paths_tuple!(L1 path1, L2 path2, L3 path3, L4 path4, L5 path5, L6 path6, L7 path7, L8 path8, L9 path9, L10 path10, L11 path11, L12 path12, L13 path13);
when_any_observable_combine_paths_tuple!(L1 path1, L2 path2, L3 path3, L4 path4, L5 path5, L6 path6, L7 path7, L8 path8, L9 path9, L10 path10, L11 path11, L12 path12, L13 path13, L14 path14);
when_any_observable_combine_paths_tuple!(L1 path1, L2 path2, L3 path3, L4 path4, L5 path5, L6 path6, L7 path7, L8 path8, L9 path9, L10 path10, L11 path11, L12 path12, L13 path13, L14 path14, L15 path15);
when_any_observable_combine_paths_tuple!(L1 path1, L2 path2, L3 path3, L4 path4, L5 path5, L6 path6, L7 path7, L8 path8, L9 path9, L10 path10, L11 path11, L12 path12, L13 path13, L14 path14, L15 path15, L16 path16);

/// The latest values of the latest observable that each path holds on `source`, combined by
/// `selector`, which takes them as one tuple (`WhenAnyObservable(..., Func<T1?, ..., TRet>)`).
pub(crate) fn when_any_observable_select<T: 'static, P: WhenAnyObservableCombinePaths<T>, R: 'static>(
    source: &Rc<T>,
    paths: P,
    selector: impl Fn(P::Items) -> R + 'static,
) -> Rc<dyn IObservable<R>> {
    paths.observe_combined(source).select(selector)
}
