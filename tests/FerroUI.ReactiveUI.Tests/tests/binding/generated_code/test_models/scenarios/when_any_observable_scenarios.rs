//! `WhenAnyObservableScenarios` (`ReactiveUI.Binding.GeneratedCode.TestModels/Scenarios/WhenAnyObservableScenarios.cs`).

use crate::tests::binding::generated_code::test_models::ObservablePropertyViewModel;
use ferroui_reactiveui::*;
use ferroui_system::{prop, IObservable};
use std::rc::Rc;

/// The values of the latest observable `MyCommand` holds (`SingleObservable_Switch`).
pub fn single_observable_switch(vm: &Rc<ObservablePropertyViewModel>) -> Rc<dyn IObservable<String>> {
    vm.when_any_observable(prop!(ObservablePropertyViewModel => my_command))
}

/// The values of the observables of `MyCommand` and `OtherCommand`, merged
/// (`TwoObservables_Merge`).
pub fn two_observables_merge(vm: &Rc<ObservablePropertyViewModel>) -> Rc<dyn IObservable<String>> {
    vm.when_any_observable((prop!(ObservablePropertyViewModel => my_command), prop!(ObservablePropertyViewModel => other_command)))
}
