//! The tests of `ReactiveUI.Binding.GeneratedCode.Tests`: upstream checks at run time the code its
//! source generator writes for `WhenAnyValue`, `WhenAny`, `WhenAnyObservable`, `WhenChanged`,
//! `WhenChanging` and `ToProperty`. The port has no generator: these tests check what the property
//! paths (`prop!`), the observation methods over them and `to_property` produce, on the models of
//! `ReactiveUI.Binding.GeneratedCode.TestModels` ([`test_models`]).
//!
//! Not ported here: the other tests of `Binding/` (`Bind*`, `InvokeCommand`,
//! `GeneratedBindingHook`, `OneWayBindCompat`) and `View/`, which belong to other work packages.

mod binding;
mod test_models;
mod when_any;
mod when_any_observable;
mod when_any_value;
mod when_changed;
