//! `ReactiveUIBindingExtensions.WhenAnyValue.WideArity.cs`: `WhenAnyValue`, and the arity of the
//! observation of several paths ([`WhenAnyPaths`]).

use crate::binding::interfaces::ObservedChange;
use crate::binding::observable_for_property::{ExpressionChainParameters, ExpressionChainSink};
use crate::binding::observables::CombineLatestObservable;
use ferroui_reactive::ObservableExt;
use ferroui_system::component_model::PropertyPath;
use ferroui_system::IObservable;
use std::rc::Rc;

/// The property paths of one observation of an object of type `T`: one [`PropertyPath`], or a
/// tuple of 2 to 12 of them.
///
/// Upstream overloads `WhenAnyValue`, `WhenAny`, `WhenChanged` and `WhenChanging` by the number of
/// expressions (1 to 16); here the paths are one argument, and this trait gives the type of their
/// values (`V`, or the tuple `(V1, ..., Vn)` for upstream's `PropertyValues<T1, ..., Tn>`) and of
/// their changes.
// Deviation (DEVIATIONS.md, ReactiveUI): the arity is a tuple of paths, from 1 to 12, where
// upstream has overloads of 1 to 16 expressions.
pub trait WhenAnyPaths<T: 'static>: 'static {
    /// The values of the paths: `V` for one path, `(V1, ..., Vn)` for a tuple.
    type Values: Clone + 'static;
    /// The changes of the paths: `ObservedChange<Rc<T>, V>` for one path, a tuple of them for a
    /// tuple.
    type Changes: Clone + 'static;

    /// The latest values of the paths on `source`, after each change or before it.
    fn observe_values(self, source: &Rc<T>, before_change: bool) -> Rc<dyn IObservable<Self::Values>>;

    /// The latest changes of the paths on `source`, after each change or before it.
    fn observe_changes(self, source: &Rc<T>, before_change: bool) -> Rc<dyn IObservable<Self::Changes>>;
}

impl<T: 'static, V: Clone + PartialEq + 'static> WhenAnyPaths<T> for PropertyPath<Rc<T>, V> {
    type Values = V;
    type Changes = ObservedChange<Rc<T>, V>;

    /// `SubscribeToExpressionChain(property.Body, skipInitial: false)` and the value of each change.
    fn observe_values(self, source: &Rc<T>, before_change: bool) -> Rc<dyn IObservable<V>> {
        self.observe_changes(source, before_change).select(|change: ObservedChange<Rc<T>, V>| change.value)
    }

    fn observe_changes(self, source: &Rc<T>, before_change: bool) -> Rc<dyn IObservable<ObservedChange<Rc<T>, V>>> {
        // Before a change of one property, each notification is a change of its own, even with the
        // value of the last one (upstream's generated `WhenChanging` observes `PropertyChanging`
        // without a distinct gate); after a change, and before a change of a deeper path, repeats
        // are dropped, as upstream's generated code does.
        let one_step_before_change = before_change && self.steps().len() == 1;
        let mut parameters = ExpressionChainParameters::new(source, self, before_change, false, !one_step_before_change, false);
        parameters.keeps_repeat_after_attachment = one_step_before_change;
        Rc::new(ExpressionChainSink::new(parameters))
    }
}

macro_rules! when_any_paths_tuple {
    ($($V:ident $path:ident),+) => {
        impl<T: 'static, $($V: Clone + PartialEq + 'static),+> WhenAnyPaths<T> for ($(PropertyPath<Rc<T>, $V>,)+) {
            type Values = ($($V,)+);
            type Changes = ($(ObservedChange<Rc<T>, $V>,)+);

            fn observe_values(self, source: &Rc<T>, before_change: bool) -> Rc<dyn IObservable<Self::Values>> {
                let ($($path,)+) = self;
                CombineLatestObservable::create_tuple(($(WhenAnyPaths::observe_values($path, source, before_change),)+))
            }

            fn observe_changes(self, source: &Rc<T>, before_change: bool) -> Rc<dyn IObservable<Self::Changes>> {
                let ($($path,)+) = self;
                CombineLatestObservable::create_tuple(($(WhenAnyPaths::observe_changes($path, source, before_change),)+))
            }
        }
    };
}

when_any_paths_tuple!(V1 path1, V2 path2);
when_any_paths_tuple!(V1 path1, V2 path2, V3 path3);
when_any_paths_tuple!(V1 path1, V2 path2, V3 path3, V4 path4);
when_any_paths_tuple!(V1 path1, V2 path2, V3 path3, V4 path4, V5 path5);
when_any_paths_tuple!(V1 path1, V2 path2, V3 path3, V4 path4, V5 path5, V6 path6);
when_any_paths_tuple!(V1 path1, V2 path2, V3 path3, V4 path4, V5 path5, V6 path6, V7 path7);
when_any_paths_tuple!(V1 path1, V2 path2, V3 path3, V4 path4, V5 path5, V6 path6, V7 path7, V8 path8);
when_any_paths_tuple!(V1 path1, V2 path2, V3 path3, V4 path4, V5 path5, V6 path6, V7 path7, V8 path8, V9 path9);
when_any_paths_tuple!(V1 path1, V2 path2, V3 path3, V4 path4, V5 path5, V6 path6, V7 path7, V8 path8, V9 path9, V10 path10);
when_any_paths_tuple!(V1 path1, V2 path2, V3 path3, V4 path4, V5 path5, V6 path6, V7 path7, V8 path8, V9 path9, V10 path10, V11 path11);
when_any_paths_tuple!(V1 path1, V2 path2, V3 path3, V4 path4, V5 path5, V6 path6, V7 path7, V8 path8, V9 path9, V10 path10, V11 path11, V12 path12);
when_any_paths_tuple!(V1 path1, V2 path2, V3 path3, V4 path4, V5 path5, V6 path6, V7 path7, V8 path8, V9 path9, V10 path10, V11 path11, V12 path12, V13 path13);
when_any_paths_tuple!(V1 path1, V2 path2, V3 path3, V4 path4, V5 path5, V6 path6, V7 path7, V8 path8, V9 path9, V10 path10, V11 path11, V12 path12, V13 path13, V14 path14);
when_any_paths_tuple!(V1 path1, V2 path2, V3 path3, V4 path4, V5 path5, V6 path6, V7 path7, V8 path8, V9 path9, V10 path10, V11 path11, V12 path12, V13 path13, V14 path14, V15 path15);
when_any_paths_tuple!(V1 path1, V2 path2, V3 path3, V4 path4, V5 path5, V6 path6, V7 path7, V8 path8, V9 path9, V10 path10, V11 path11, V12 path12, V13 path13, V14 path14, V15 path15, V16 path16);

/// The latest values of `paths` on `source`: the current values on subscription, then the values
/// after each change of any step of any path, skipping repeats (`WhenAnyValue`).
pub(crate) fn when_any_value<T: 'static, P: WhenAnyPaths<T>>(source: &Rc<T>, paths: P) -> Rc<dyn IObservable<P::Values>> {
    paths.observe_values(source, false)
}
