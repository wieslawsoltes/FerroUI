//! `SummaryViewModel` (`ReactiveUI.Binding.GeneratedCode.TestModels/TestModels/SummaryViewModel.cs`).

use ferroui_reactiveui::*;
use ferroui_system::{prop, IObservable};
use std::cell::OnceCell;
use std::rc::Rc;

/// A view model with two output properties that may hold no value, each backed by
/// `to_property` (`SummaryViewModel`).
///
/// Upstream declares them with `[ObservableAsProperty]`, whose generated getters read the helpers;
/// here the view model stores the helpers and its getters read them.
// Deviation (DEVIATIONS.md, ReactiveUI): `to_property` raises through `IReactiveObject`, so the
// view model, a plain `INotifyPropertyChanged` upstream, holds a `ReactiveObject`.
#[derive(Default)]
pub struct SummaryViewModel {
    reactive: ReactiveObject,
    full_name: OnceCell<ObservableAsPropertyHelper<Option<String>>>,
    age: OnceCell<ObservableAsPropertyHelper<Option<i32>>>,
}
reactive_object!(SummaryViewModel);

impl SummaryViewModel {
    /// The view model whose `FullName` follows `names` and whose `Age` follows `ages`.
    pub fn new(names: Rc<dyn IObservable<Option<String>>>, ages: Rc<dyn IObservable<Option<i32>>>) -> Rc<Self> {
        let vm = Rc::new(Self::default());
        let full_name = names.to_property(&vm, prop!(SummaryViewModel => full_name));
        let _ = vm.full_name.set(full_name);
        let age = ages.to_property(&vm, prop!(SummaryViewModel => age));
        let _ = vm.age.set(age);
        vm
    }

    /// `FullName`: the latest name.
    pub fn full_name(&self) -> Option<String> {
        self.full_name.get().and_then(|helper| helper.value())
    }

    /// `Age`: the latest age.
    pub fn age(&self) -> Option<i32> {
        self.age.get().and_then(|helper| helper.value())
    }
}
