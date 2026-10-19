//! `ReactiveViewControlsBindingTests` (`ReactiveViewControlsBindingTests.cs`): the synchronization
//! of a reactive user control's view model and data context.

use super::view_support::{same, start};
use crate::ReactiveUserControl;
use ferroui_base::BoxedValue;
use ferroui_reactiveui::{reactive_object, ReactiveObject};
use std::rc::Rc;

/// A test view model (`VM`).
struct Vm {
    reactive: ReactiveObject,
}
reactive_object!(Vm);

impl PartialEq for Vm {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl Vm {
    fn new() -> Rc<Self> {
        Rc::new(Self { reactive: ReactiveObject::new() })
    }
}

/// A test view for `Vm` (`View : ReactiveUserControl<VM>`).
type View = ReactiveUserControl<Vm>;

#[test]
fn reactive_user_control_view_model_data_context_syncs() {
    let _app = start();
    let v = View::new();
    let vm = Vm::new();

    v.set_data_context(Some(vm.clone() as BoxedValue));
    assert!(v.view_model().is_some_and(|view_model| Rc::ptr_eq(&view_model, &vm)));

    let vm2 = Vm::new();
    v.set_view_model(Some(vm2.clone()));
    assert!(v.data_context().is_some_and(|data_context| same(&data_context, &vm2)));
}
