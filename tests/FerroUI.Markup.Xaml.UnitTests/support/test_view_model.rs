//! Port of `TestViewModel.cs`.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use ferroui_base::data::model::{Event, INotifyPropertyChanged};
use ferroui_base::ferro_markup_type;

/// The view model of the binding tests: it raises a property change
/// notification from every setter.
#[derive(Default)]
pub struct TestViewModel {
    string: RefCell<Option<String>>,
    integer: Cell<i32>,
    child: RefCell<Option<Rc<TestViewModel>>>,
    boolean: Cell<bool>,
    property_changed: Event<str>,
}

crate::test_identity_eq!(TestViewModel);

impl INotifyPropertyChanged for TestViewModel {
    fn property_changed(&self) -> &Event<str> {
        &self.property_changed
    }
}

impl TestViewModel {
    pub fn new() -> Rc<Self> {
        Rc::new(Self::default())
    }

    fn raise_property_changed(&self, property_name: &str) {
        self.property_changed.raise(property_name);
    }

    pub fn integer(&self) -> i32 {
        self.integer.get()
    }

    pub fn set_integer(&self, value: i32) {
        self.integer.set(value);
        self.raise_property_changed("Integer");
    }

    pub fn string(&self) -> Option<String> {
        self.string.borrow().clone()
    }

    pub fn set_string(&self, value: Option<String>) {
        *self.string.borrow_mut() = value;
        self.raise_property_changed("String");
    }

    pub fn child(&self) -> Option<Rc<TestViewModel>> {
        self.child.borrow().clone()
    }

    pub fn set_child(&self, value: Option<Rc<TestViewModel>>) {
        *self.child.borrow_mut() = value;
        self.raise_property_changed("Child");
    }

    pub fn boolean(&self) -> bool {
        self.boolean.get()
    }

    pub fn set_boolean(&self, value: bool) {
        self.boolean.set(value);
        self.raise_property_changed("Boolean");
    }
}

ferro_markup_type!(class TestViewModel {
    this: Rc<TestViewModel>,
    handles: [TestViewModel, Rc<TestViewModel>, Option<Rc<TestViewModel>>],
    namespace: "FerroUI.Markup.Xaml.UnitTests",
    constructors: [() => TestViewModel::new],
    properties: [
        Integer: i32 {
            get: |this: &Rc<TestViewModel>| this.integer(),
            set: |this: &Rc<TestViewModel>, value: i32| this.set_integer(value)
        },
        String: Option<String> {
            get: |this: &Rc<TestViewModel>| this.string(),
            set: |this: &Rc<TestViewModel>, value: Option<String>| this.set_string(value)
        },
        Child: Option<Rc<TestViewModel>> {
            get: |this: &Rc<TestViewModel>| this.child(),
            set: |this: &Rc<TestViewModel>, value: Option<Rc<TestViewModel>>| this.set_child(value)
        },
        Boolean: bool {
            get: |this: &Rc<TestViewModel>| this.boolean(),
            set: |this: &Rc<TestViewModel>, value: bool| this.set_boolean(value)
        },
    ],
    notify_property_changed: TestViewModel,
});
