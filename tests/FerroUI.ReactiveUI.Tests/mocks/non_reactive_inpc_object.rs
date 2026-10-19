//! `NonReactiveInpcObject` (`ReactiveUI.Tests/WhenAny/Mockups/NonReactiveInpcObject.cs`).

use super::TestFixture;
use ferroui_system::component_model::INotifyPropertyChanged;
use ferroui_system::events::Event;
use std::cell::RefCell;
use std::rc::Rc;

/// An object that raises `PropertyChanged` without being a reactive object
/// (`NonReactiveInpcObject`).
pub struct NonReactiveInpcObject {
    property_changed: Event<str>,
    inpc_property: RefCell<Rc<TestFixture>>,
}

impl Default for NonReactiveInpcObject {
    fn default() -> Self {
        Self { property_changed: Event::new(), inpc_property: RefCell::new(TestFixture::new()) }
    }
}

impl INotifyPropertyChanged for NonReactiveInpcObject {
    fn property_changed(&self) -> &Event<str> {
        &self.property_changed
    }
}

impl NonReactiveInpcObject {
    /// An object holding a new fixture, shared as the objects of upstream are.
    pub fn new() -> Rc<Self> {
        Rc::new(Self::default())
    }

    /// `InpcProperty`.
    pub fn inpc_property(&self) -> Rc<TestFixture> {
        self.inpc_property.borrow().clone()
    }

    /// Sets `InpcProperty` and raises `PropertyChanged`, unless `value` is the fixture it holds
    /// (upstream compares the references, `field == value`).
    pub fn set_inpc_property(&self, value: Rc<TestFixture>) {
        if Rc::ptr_eq(&*self.inpc_property.borrow(), &value) {
            return;
        }
        *self.inpc_property.borrow_mut() = value;
        self.property_changed.raise("InpcProperty");
    }
}
