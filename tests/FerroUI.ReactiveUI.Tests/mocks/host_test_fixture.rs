//! `HostTestFixture` (`ReactiveUI.Tests/WhenAny/Mockups/HostTestFixture.cs`): the view model of
//! the tests of the observation of nested properties.

use super::{NonObservableTestFixture, OwnerClass, TestFixture};
use ferroui_reactive::ObservableExt;
use ferroui_reactiveui::{
    reactive_object, reactive_properties, IReactiveObject, ObservableAsPropertyHelper, ObservableMixins, ObservedProperty, ReactiveObject,
    ReactiveUIBindingExtensions, ToPropertyMixins,
};
use ferroui_system::{prop, IObservable};
use std::cell::RefCell;
use std::rc::Rc;

reactive_properties! {
    /// A view model holding a child fixture, an owner whose name it follows, an object that does
    /// not notify, and a number (`HostTestFixture`).
    #[derive(Default)]
    pub struct HostTestFixture {
        reactive: ReactiveObject,
        child: RefCell<Option<Rc<TestFixture>>>,
        #[reactive]
        owner: Option<Rc<OwnerClass>>,
        #[reactive]
        poco_child: Option<Rc<NonObservableTestFixture>>,
        #[reactive]
        some_other_param: i32,
        owner_name: RefCell<Option<ObservableAsPropertyHelper<Option<String>>>>,
    }
}
reactive_object!(HostTestFixture);

/// Fixtures compare by identity, as the reference types of upstream do
/// (`EqualityComparer<HostTestFixture>.Default`).
impl PartialEq for HostTestFixture {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl HostTestFixture {
    /// A fixture whose `OwnerName` follows the name of its owner, shared as view models are
    /// (the constructor `HostTestFixture()`).
    ///
    /// The helper is made once the fixture is shared, since the observation takes the `Rc` that
    /// upstream's constructor has as `this`.
    pub fn new() -> Rc<Self> {
        let fixture = Rc::new(Self::default());
        let names: Rc<dyn IObservable<Option<Rc<dyn IObservable<Option<String>>>>>> = fixture
            .when_any_value(prop!(HostTestFixture => owner))
            .where_not_null()
            .select(|owner: Rc<OwnerClass>| Some(owner.when_any_value(prop!(OwnerClass => name))));
        let helper = ObservedProperty::switch(names).to_property(&fixture, prop!(HostTestFixture => owner_name));
        *fixture.owner_name.borrow_mut() = Some(helper);
        fixture
    }

    /// `OwnerName`: the name of the owner, `None` while there is no owner.
    pub fn owner_name(&self) -> Option<String> {
        self.owner_name.borrow().as_ref().and_then(|helper| helper.value())
    }

    /// `Child`.
    pub fn child(&self) -> Option<Rc<TestFixture>> {
        self.child.borrow().clone()
    }

    /// Sets `Child`, unless `value` is the fixture it holds: [`TestFixture`] has no equality, and
    /// upstream's `RaiseAndSetIfChanged` compares the references of a class without `Equals`.
    pub fn set_child(&self, value: Option<Rc<TestFixture>>) {
        let same = match (&*self.child.borrow(), &value) {
            (Some(current), Some(value)) => Rc::ptr_eq(current, value),
            (None, None) => true,
            _ => false,
        };
        if same {
            return;
        }
        IReactiveObject::raise_property_changing(self, "Child");
        *self.child.borrow_mut() = value;
        IReactiveObject::raise_property_changed(self, "Child");
    }
}
