//! The view models of `SharedScenarios/ToProperty` (one module per scenario folder, each with its
//! `MyViewModel.cs` and `Scenario.cs`), which the generated-code tests of `ToProperty` build.
//!
//! Upstream's scenarios differ in how the generated code raises the owner's notifications: through
//! the owner's own `PropertyChanged` event (`PartialEvent`, `NameOf`, `InitialValueDeferScheduler`,
//! `FactoryOutResult`), through a base class's protected methods (`PartialProtectedBase`), through
//! a public raise method (`PublicRaiseMethod`) or through ReactiveUI (`ReactiveObject`). Each view
//! model stores the helper `to_property` returns and reads its value in the getter.
// Deviation (DEVIATIONS.md, ReactiveUI): `to_property` raises through `IReactiveObject`, so every
// view model holds a `ReactiveObject`; the strategies of the generator collapse into that one, and
// the scenarios keep their options (the name, the initial value or its factory, the deferred
// subscription, the scheduler).

use ferroui_reactive::concurrency::IScheduler;
use ferroui_reactiveui::*;
use ferroui_system::{prop, IObservable};
use std::cell::OnceCell;
use std::rc::Rc;

/// `PartialEvent`: a view model that raises its own event (`ToProperty(this, x => x.FullName)`).
pub mod partial_event {
    use super::*;

    /// The view model of the scenario (`MyViewModel`).
    #[derive(Default)]
    pub struct MyViewModel {
        reactive: ReactiveObject,
        full_name: OnceCell<ObservableAsPropertyHelper<String>>,
    }
    reactive_object!(MyViewModel);

    impl MyViewModel {
        /// The view model whose `FullName` follows `names`.
        pub fn new(names: Rc<dyn IObservable<String>>) -> Rc<Self> {
            let vm = Rc::new(Self::default());
            let helper = names.to_property(&vm, prop!(MyViewModel => full_name));
            let _ = vm.full_name.set(helper);
            vm
        }

        /// `FullName`.
        pub fn full_name(&self) -> String {
            self.full_name.get().map(|helper| helper.value()).unwrap_or_default()
        }
    }

    /// `Scenario.Execute`.
    pub fn execute(names: Rc<dyn IObservable<String>>) -> Rc<MyViewModel> {
        MyViewModel::new(names)
    }
}

/// `PartialProtectedBase`: a view model that raises through the protected methods of its base
/// class, `PropertyChanging` before `PropertyChanged`.
pub mod partial_protected_base {
    use super::*;

    /// The view model of the scenario (`MyViewModel : ViewModelBase`); the base class's events are
    /// the reactive object's.
    #[derive(Default)]
    pub struct MyViewModel {
        reactive: ReactiveObject,
        full_name: OnceCell<ObservableAsPropertyHelper<String>>,
    }
    reactive_object!(MyViewModel);

    impl MyViewModel {
        /// The view model whose `FullName` follows `names`.
        pub fn new(names: Rc<dyn IObservable<String>>) -> Rc<Self> {
            let vm = Rc::new(Self::default());
            let helper = names.to_property(&vm, prop!(MyViewModel => full_name));
            let _ = vm.full_name.set(helper);
            vm
        }

        /// `FullName`.
        pub fn full_name(&self) -> String {
            self.full_name.get().map(|helper| helper.value()).unwrap_or_default()
        }
    }

    /// `Scenario.Execute`.
    pub fn execute(names: Rc<dyn IObservable<String>>) -> Rc<MyViewModel> {
        MyViewModel::new(names)
    }
}

/// `PublicRaiseMethod`: a view model raised through its public `RaisePropertyChanged`, here
/// `IReactiveObject::raise_property_changed`.
pub mod public_raise_method {
    use super::*;

    /// The view model of the scenario (`MyViewModel`).
    #[derive(Default)]
    pub struct MyViewModel {
        reactive: ReactiveObject,
        count: OnceCell<ObservableAsPropertyHelper<i32>>,
    }
    reactive_object!(MyViewModel);

    impl MyViewModel {
        /// The view model whose `Count` follows `counts`.
        pub fn new(counts: Rc<dyn IObservable<i32>>) -> Rc<Self> {
            let vm = Rc::new(Self::default());
            let helper = counts.to_property(&vm, prop!(MyViewModel => count));
            let _ = vm.count.set(helper);
            vm
        }

        /// `Count`.
        pub fn count(&self) -> i32 {
            self.count.get().map(|helper| helper.value()).unwrap_or_default()
        }
    }

    /// `Scenario.Execute`.
    pub fn execute(counts: Rc<dyn IObservable<i32>>) -> Rc<MyViewModel> {
        MyViewModel::new(counts)
    }
}

/// `ReactiveObject`: a view model deriving from `ReactiveObject`, raised through ReactiveUI.
pub mod reactive_object {
    use super::*;

    /// The view model of the scenario (`MyViewModel : ReactiveObject`).
    #[derive(Default)]
    pub struct MyViewModel {
        reactive: ReactiveObject,
        full_name: OnceCell<ObservableAsPropertyHelper<String>>,
    }
    reactive_object!(MyViewModel);

    impl MyViewModel {
        /// The view model whose `FullName` follows `names`.
        pub fn new(names: Rc<dyn IObservable<String>>) -> Rc<Self> {
            let vm = Rc::new(Self::default());
            let helper = names.to_property(&vm, prop!(MyViewModel => full_name));
            let _ = vm.full_name.set(helper);
            vm
        }

        /// `FullName`.
        pub fn full_name(&self) -> String {
            self.full_name.get().map(|helper| helper.value()).unwrap_or_default()
        }
    }

    /// `Scenario.Execute`.
    pub fn execute(names: Rc<dyn IObservable<String>>) -> Rc<MyViewModel> {
        MyViewModel::new(names)
    }
}

/// `NameOf`: the property named by a string (`ToProperty(this, nameof(FullName))`).
pub mod name_of {
    use super::*;

    /// The view model of the scenario (`MyViewModel`).
    #[derive(Default)]
    pub struct MyViewModel {
        reactive: ReactiveObject,
        full_name: OnceCell<ObservableAsPropertyHelper<String>>,
    }
    reactive_object!(MyViewModel);

    impl MyViewModel {
        /// The view model whose `FullName` follows `names`.
        pub fn new(names: Rc<dyn IObservable<String>>) -> Rc<Self> {
            let vm = Rc::new(Self::default());
            let helper = names.to_property_by_name(&vm, "FullName", ObservableAsPropertyInitialValue::Value(String::new()), false, None);
            let _ = vm.full_name.set(helper);
            vm
        }

        /// `FullName`.
        pub fn full_name(&self) -> String {
            self.full_name.get().map(|helper| helper.value()).unwrap_or_default()
        }
    }

    /// `Scenario.Execute`.
    pub fn execute(names: Rc<dyn IObservable<String>>) -> Rc<MyViewModel> {
        MyViewModel::new(names)
    }
}

/// `InitialValueDeferScheduler`: an initial value, a deferred subscription and a scheduler
/// (`ToProperty(this, x => x.FullName, "(none)", true, scheduler)`).
pub mod initial_value_defer_scheduler {
    use super::*;

    /// The initial value of `FullName`.
    const INITIAL_FULL_NAME: &str = "(none)";

    /// The view model of the scenario (`MyViewModel`).
    #[derive(Default)]
    pub struct MyViewModel {
        reactive: ReactiveObject,
        full_name: OnceCell<ObservableAsPropertyHelper<String>>,
    }
    reactive_object!(MyViewModel);

    impl MyViewModel {
        /// The view model whose `FullName` follows `names` from `"(none)"`, subscribing on the
        /// first read and delivering on `scheduler`.
        pub fn new(names: Rc<dyn IObservable<String>>, scheduler: Option<Rc<dyn IScheduler>>) -> Rc<Self> {
            let vm = Rc::new(Self::default());
            let helper = names.to_property_with(
                &vm,
                prop!(MyViewModel => full_name),
                ObservableAsPropertyInitialValue::Value(INITIAL_FULL_NAME.to_string()),
                true,
                scheduler,
            );
            let _ = vm.full_name.set(helper);
            vm
        }

        /// `FullName`.
        pub fn full_name(&self) -> String {
            self.full_name.get().map(|helper| helper.value()).unwrap_or_default()
        }
    }

    /// `Scenario.Execute`.
    pub fn execute(names: Rc<dyn IObservable<String>>, scheduler: Option<Rc<dyn IScheduler>>) -> Rc<MyViewModel> {
        MyViewModel::new(names, scheduler)
    }
}

/// `FactoryOutResult`: the helper returned through upstream's `out` parameter, with a factory of
/// the initial value (`ToProperty(this, x => x.Count, out _count, () => -1)`).
pub mod factory_out_result {
    use super::*;

    /// The initial value of `Count`.
    const INITIAL_COUNT: i32 = -1;

    /// The view model of the scenario (`MyViewModel`).
    #[derive(Default)]
    pub struct MyViewModel {
        reactive: ReactiveObject,
        count: OnceCell<ObservableAsPropertyHelper<i32>>,
    }
    reactive_object!(MyViewModel);

    impl MyViewModel {
        /// The view model whose `Count` follows `counts` from the value of a factory.
        pub fn new(counts: Rc<dyn IObservable<i32>>) -> Rc<Self> {
            let vm = Rc::new(Self::default());
            let helper = counts.to_property_with(&vm, prop!(MyViewModel => count), ObservableAsPropertyInitialValue::factory(|| INITIAL_COUNT), false, None);
            let _ = vm.count.set(helper);
            vm
        }

        /// `Count`.
        pub fn count(&self) -> i32 {
            self.count.get().map(|helper| helper.value()).unwrap_or_default()
        }
    }

    /// `Scenario.Execute`.
    pub fn execute(counts: Rc<dyn IObservable<i32>>) -> Rc<MyViewModel> {
        MyViewModel::new(counts)
    }
}
