//! Ported from the upstream `BindingTests_Method`.

use super::test_support::*;
use crate::data::Binding;
use ferroui_base::data::core::Value;
use ferroui_base::data::model::Model;
use ferroui_base::ferro_model;
use std::cell::Cell;

struct TestClass {
    is_set: Cell<bool>,
}

impl TestClass {
    /// A private method: it is not part of the declared binding metadata.
    #[allow(dead_code)]
    fn my_method(&self) {
        self.is_set.set(true);
    }
}

ferro_model!(TestClass, |b| b.property::<Value<bool>>("IsSet", |o| o.is_set.get(), |o, v| o.is_set.set(v)));

#[test]
fn binding_to_private_methods_shouldnt_work() {
    let vm = Model::new_model(TestClass { is_set: Cell::new(false) });
    let target = Button::new();
    target.set_data_context(Some(vm.clone()));
    target.bind_binding(Button::command_property(), &Binding::with_path("MyMethod"));

    target.click();

    assert!(!vm.is_set.get());
}
