//! `TestFixture` (`ReactiveUI.Tests/ReactiveObjects/Mocks/TestFixture.cs`): the view model of the
//! tests of reactive objects and of the observation of properties.

use ferroui_reactiveui::{reactive_object, reactive_properties, IReactiveObject, ReactiveObject};
use ferroui_system::collections::object_model::ObservableCollection;
use std::cell::RefCell;
use std::rc::Rc;

reactive_properties! {
    /// A view model with notifying properties of several types, a property that does not notify
    /// (`PocoProperty`) and a collection (`TestFixture`).
    ///
    /// The serialization attributes of upstream (`[DataMember]`, `[JsonRequired]`) have no
    /// counterpart: the tests of serialization are not ported.
    #[derive(Default)]
    pub struct TestFixture {
        reactive: ReactiveObject,
        #[reactive]
        is_not_null_string: Option<String>,
        #[reactive]
        is_only_one_word: Option<String>,
        #[reactive]
        not_serialized: Option<String>,
        #[reactive]
        nullable_int: Option<i32>,
        #[reactive]
        uses_expr_raise_set: Option<String>,
        poco_property: RefCell<Option<String>>,
        stack_overflow_trigger: RefCell<Option<Vec<String>>>,
        test_collection: ObservableCollection<i32>,
    }
}
reactive_object!(TestFixture);

impl TestFixture {
    /// A fixture whose properties are unset, shared as view models are.
    pub fn new() -> Rc<Self> {
        Rc::new(Self::default())
    }

    /// `PocoProperty`, which does not notify its changes.
    pub fn poco_property(&self) -> Option<String> {
        self.poco_property.borrow().clone()
    }

    /// Sets `PocoProperty`, without notification.
    pub fn set_poco_property(&self, value: Option<String>) {
        *self.poco_property.borrow_mut() = value;
    }

    /// `StackOverflowTrigger`.
    pub fn stack_overflow_trigger(&self) -> Option<Vec<String>> {
        self.stack_overflow_trigger.borrow().clone()
    }

    /// Sets `StackOverflowTrigger` to a copy of `value`. Upstream stores a new list
    /// (`value?.ToList()`), which never equals the old one by reference, so every assignment
    /// notifies.
    pub fn set_stack_overflow_trigger(&self, value: Option<Vec<String>>) {
        self.raise_property_changing("StackOverflowTrigger");
        *self.stack_overflow_trigger.borrow_mut() = value;
        self.raise_property_changed("StackOverflowTrigger");
    }

    /// `TestCollection`.
    pub fn test_collection(&self) -> &ObservableCollection<i32> {
        &self.test_collection
    }
}
