//! `WhenAnyArityTestViewModel` (`ReactiveUI.Tests/WhenAny/WhenAnyArityTestViewModel.cs`): the view
//! model of the tests of the arities of `WhenAny`, `WhenAnyValue` and `WhenAnyObservable`.

use ferroui_reactiveui::{reactive_object, reactive_properties, IReactiveObject, ReactiveObject};
use ferroui_system::IObservable;
use std::cell::RefCell;
use std::rc::Rc;

reactive_properties! {
    /// Twelve string properties and twelve properties that hold observables
    /// (`WhenAnyArityTestViewModel`).
    #[derive(Default)]
    pub struct WhenAnyArityTestViewModel {
        reactive: ReactiveObject,
        #[reactive]
        property1: Option<String>,
        #[reactive]
        property2: Option<String>,
        #[reactive]
        property3: Option<String>,
        #[reactive]
        property4: Option<String>,
        #[reactive]
        property5: Option<String>,
        #[reactive]
        property6: Option<String>,
        #[reactive]
        property7: Option<String>,
        #[reactive]
        property8: Option<String>,
        #[reactive]
        property9: Option<String>,
        #[reactive]
        property10: Option<String>,
        #[reactive]
        property11: Option<String>,
        #[reactive]
        property12: Option<String>,
        observable_property1: RefCell<Option<Rc<dyn IObservable<String>>>>,
        observable_property2: RefCell<Option<Rc<dyn IObservable<String>>>>,
        observable_property3: RefCell<Option<Rc<dyn IObservable<String>>>>,
        observable_property4: RefCell<Option<Rc<dyn IObservable<String>>>>,
        observable_property5: RefCell<Option<Rc<dyn IObservable<String>>>>,
        observable_property6: RefCell<Option<Rc<dyn IObservable<String>>>>,
        observable_property7: RefCell<Option<Rc<dyn IObservable<String>>>>,
        observable_property8: RefCell<Option<Rc<dyn IObservable<String>>>>,
        observable_property9: RefCell<Option<Rc<dyn IObservable<String>>>>,
        observable_property10: RefCell<Option<Rc<dyn IObservable<String>>>>,
        observable_property11: RefCell<Option<Rc<dyn IObservable<String>>>>,
        observable_property12: RefCell<Option<Rc<dyn IObservable<String>>>>,
    }
}
reactive_object!(WhenAnyArityTestViewModel);

/// The accessors of the properties that hold observables. An observable has no equality, so these
/// are written here rather than by `#[reactive]`; the setter skips the observable the property
/// holds, as upstream's `RaiseAndSetIfChanged` compares the references.
macro_rules! observable_properties {
    ($($field:ident / $setter:ident : $name:literal,)*) => {
        impl WhenAnyArityTestViewModel {
            $(
                #[doc = concat!("`", $name, "`.")]
                pub fn $field(&self) -> Option<Rc<dyn IObservable<String>>> {
                    self.$field.borrow().clone()
                }

                #[doc = concat!("Sets `", $name, "`, unless `value` is the observable it holds.")]
                pub fn $setter(&self, value: Option<Rc<dyn IObservable<String>>>) {
                    let same = match (&*self.$field.borrow(), &value) {
                        (Some(current), Some(value)) => Rc::ptr_eq(current, value),
                        (None, None) => true,
                        _ => false,
                    };
                    if same {
                        return;
                    }
                    IReactiveObject::raise_property_changing(self, $name);
                    *self.$field.borrow_mut() = value;
                    IReactiveObject::raise_property_changed(self, $name);
                }
            )*
        }
    };
}

observable_properties! {
    observable_property1 / set_observable_property1: "ObservableProperty1",
    observable_property2 / set_observable_property2: "ObservableProperty2",
    observable_property3 / set_observable_property3: "ObservableProperty3",
    observable_property4 / set_observable_property4: "ObservableProperty4",
    observable_property5 / set_observable_property5: "ObservableProperty5",
    observable_property6 / set_observable_property6: "ObservableProperty6",
    observable_property7 / set_observable_property7: "ObservableProperty7",
    observable_property8 / set_observable_property8: "ObservableProperty8",
    observable_property9 / set_observable_property9: "ObservableProperty9",
    observable_property10 / set_observable_property10: "ObservableProperty10",
    observable_property11 / set_observable_property11: "ObservableProperty11",
    observable_property12 / set_observable_property12: "ObservableProperty12",
}

impl WhenAnyArityTestViewModel {
    /// A view model whose properties are unset, shared as view models are.
    pub fn new() -> Rc<Self> {
        Rc::new(Self::default())
    }
}
