//! `DeeplyNestedNotifyPropertyChangedFixture` (`DynamicData.Tests/Binding/DeeplyNestedNotifyPropertyChangedFixture.cs`).
//!
//! A path through a child that may be null (`a => a.Child!.Age`) is
//! `PropertyChain::nullable(prop!(ClassA => child)).then(prop!(ClassB => age))`; a step of an
//! object that does not notify (`ClassD.Value`) is a `PropertyPath::value`.

use ferroui_dynamic_data::binding::{AbstractNotifyPropertyChanged, NotifyPropertyChangedEx, PropertyChain, PropertyValue};
use ferroui_reactive::ObservableExt;
use ferroui_system::component_model::{INotifyPropertyChanged, PropertyPath};
use ferroui_system::events::Event;
use ferroui_system::{prop, IDisposable, ObservableError};
use std::cell::{Cell, RefCell};
use std::fmt;
use std::rc::Rc;

/// `a => a.Child!.Age`.
fn child_age() -> PropertyChain<Rc<ClassA>, i32> {
    PropertyChain::nullable(prop!(ClassA => child)).then(prop!(ClassB => age))
}

/// `x => x.Child!.Value`, where the child does not notify.
fn child_value() -> PropertyChain<Rc<ClassC>, i32> {
    PropertyChain::nullable(prop!(ClassC => child)).then(PropertyPath::value("Value", |child: &Rc<ClassD>| child.value()))
}

#[test]
fn depth_of_one() {
    let instance = ClassA::new(Some("Someone".to_owned()), None);

    let chain = instance.when_property_changed_with(prop!(ClassA => name), true);
    let result: Rc<RefCell<Option<String>>> = Rc::new(RefCell::new(None));

    let sink = result.clone();
    let _subscription = chain.subscribe_next(move |notification: PropertyValue<Rc<ClassA>, Option<String>>| *sink.borrow_mut() = notification.value().cloned().flatten());

    assert_eq!(*result.borrow(), Some("Someone".to_owned()));

    instance.set_name(Some("Else".to_owned()));
    assert_eq!(*result.borrow(), Some("Else".to_owned()));

    instance.set_name(None);
    assert_eq!(*result.borrow(), None);

    instance.set_name(Some("NotNull".to_owned()));
    assert_eq!(*result.borrow(), Some("NotNull".to_owned()));
}

/// The notifications, the error and the end of a subscription.
type Recorded<T> = (Rc<RefCell<Vec<T>>>, Rc<RefCell<Option<ObservableError>>>, Rc<Cell<bool>>);

fn recorded<T>() -> Recorded<T> {
    (Rc::new(RefCell::new(Vec::new())), Rc::new(RefCell::new(None)), Rc::new(Cell::new(false)))
}

// Covers https://github.com/reactivemarbles/DynamicData/issues/671
#[test]
fn non_observable_child_with_initial_value() {
    let source = ClassC::new(Some(ClassD::new(1)));

    let (notifications, error, is_completed) = recorded::<PropertyValue<Rc<ClassC>, i32>>();

    let (on_next, on_error, on_completed) = (notifications.clone(), error.clone(), is_completed.clone());
    let subscription = source.when_property_changed_with(child_value(), true).subscribe_all(
        move |notification| on_next.borrow_mut().push(notification),
        move |e| *on_error.borrow_mut() = Some(e),
        move || on_completed.set(true),
    );

    assert!(error.borrow().is_none());
    assert!(!is_completed.get());
    assert_eq!(notifications.borrow().len(), 1, "a notification was requested for the initial value");
    assert_eq!(notifications.borrow()[0].value(), Some(&source.child().unwrap().value()), "the child object's data value should have been published");

    source.child().unwrap().set_value(2);

    assert!(error.borrow().is_none());
    assert!(!is_completed.get());
    assert_eq!(notifications.borrow().len(), 1, "the object that was changed does not publish notifications");

    source.set_child(Some(ClassD::new(3)));

    assert!(error.borrow().is_none());
    assert!(!is_completed.get());
    assert_eq!(notifications.borrow().len(), 2, "the parent object should have published a notification for its child being changed");
    assert_eq!(notifications.borrow()[1].value(), Some(&source.child().unwrap().value()), "the child object's data value should have been published");

    subscription.dispose();
}

#[test]
fn non_observable_child_without_initial_value() {
    let source = ClassC::new(Some(ClassD::new(1)));

    let (notifications, error, is_completed) = recorded::<PropertyValue<Rc<ClassC>, i32>>();

    let (on_next, on_error, on_completed) = (notifications.clone(), error.clone(), is_completed.clone());
    let subscription = source.when_property_changed_with(child_value(), false).subscribe_all(
        move |notification| on_next.borrow_mut().push(notification),
        move |e| *on_error.borrow_mut() = Some(e),
        move || on_completed.set(true),
    );

    assert!(error.borrow().is_none());
    assert!(!is_completed.get());
    assert!(notifications.borrow().is_empty(), "no changes have been made");

    source.child().unwrap().set_value(2);

    assert!(error.borrow().is_none());
    assert!(!is_completed.get());
    assert!(notifications.borrow().is_empty(), "the object that was changed does not publish notifications");

    source.set_child(Some(ClassD::new(3)));

    assert!(error.borrow().is_none());
    assert!(!is_completed.get());
    assert_eq!(notifications.borrow().len(), 1, "the parent object should have published a notification for its child being changed");
    assert_eq!(notifications.borrow()[0].value(), Some(&source.child().unwrap().value()), "the child object's data value should have been published");

    subscription.dispose();
}

#[test]
fn notifies_initial_value_with_fallback() {
    let instance = ClassA::new(None, Some(ClassB::new(10)));

    // provide a fallback so a value can always be obtained
    let fallback: Rc<dyn Fn() -> i32> = Rc::new(|| -1);
    let chain = instance.when_changed_with(child_age(), |_sender, a| a, Some(fallback));

    let result: Rc<Cell<Option<i32>>> = Rc::new(Cell::new(None));

    let sink = result.clone();
    let _subscription = chain.subscribe_next(move |age: i32| sink.set(Some(age)));

    assert_eq!(result.get(), Some(10));

    instance.child().unwrap().set_age(22);
    assert_eq!(result.get(), Some(22));

    instance.set_child(Some(ClassB::new(25)));
    assert_eq!(result.get(), Some(25));

    instance.child().unwrap().set_age(26);
    assert_eq!(result.get(), Some(26));
    instance.set_child(None);
    assert_eq!(result.get(), Some(-1));

    instance.set_child(Some(ClassB::new(21)));
    assert_eq!(result.get(), Some(21));
}

/// The value of a notification of `a => a.Child!.Age` (`notification?.Value`).
fn subscribe_result(chain: Rc<dyn ferroui_system::IObservable<PropertyValue<Rc<ClassA>, i32>>>) -> (Rc<Cell<Option<i32>>>, Rc<dyn IDisposable>) {
    let result: Rc<Cell<Option<i32>>> = Rc::new(Cell::new(None));
    let sink = result.clone();
    let subscription = chain.subscribe_next(move |notification: PropertyValue<Rc<ClassA>, i32>| sink.set(Some(notification.value().copied().unwrap_or_default())));
    (result, subscription)
}

#[test]
fn notifies_initial_value_and_null_child() {
    let instance = ClassA::new(None, None);

    let chain = instance.when_property_changed_with(child_age(), true);

    let (result, _subscription) = subscribe_result(chain);
    assert_eq!(result.get(), None);
    instance.set_child(Some(ClassB::new(10)));

    assert_eq!(result.get(), Some(10));

    instance.child().unwrap().set_age(22);
    assert_eq!(result.get(), Some(22));

    instance.set_child(Some(ClassB::new(25)));
    assert_eq!(result.get(), Some(25));

    instance.child().unwrap().set_age(26);
    assert_eq!(result.get(), Some(26));
    instance.set_child(None);
}

#[test]
fn null_child_with_initial_value() {
    let instance = ClassA::new(None, None);

    let chain = instance.when_property_changed_with(child_age(), true);

    let (result, _subscription) = subscribe_result(chain);

    assert_eq!(result.get(), None);

    instance.set_child(Some(ClassB::new(21)));
    assert_eq!(result.get(), Some(21));

    instance.child().unwrap().set_age(22);
    assert_eq!(result.get(), Some(22));

    instance.set_child(Some(ClassB::new(25)));
    assert_eq!(result.get(), Some(25));

    instance.child().unwrap().set_age(30);
    assert_eq!(result.get(), Some(30));
}

#[test]
fn null_child_without_initial_value() {
    let instance = ClassA::new(None, None);

    let chain = instance.when_property_changed_with(child_age(), false);

    let (result, _subscription) = subscribe_result(chain);

    assert_eq!(result.get(), None);

    instance.set_child(Some(ClassB::new(21)));
    assert_eq!(result.get(), Some(21));

    instance.child().unwrap().set_age(22);
    assert_eq!(result.get(), Some(22));

    instance.set_child(Some(ClassB::new(25)));
    assert_eq!(result.get(), Some(25));

    instance.child().unwrap().set_age(30);
    assert_eq!(result.get(), Some(30));
}

#[test]
fn without_initial_value() {
    let instance = ClassA::new(Some("TestClass".to_owned()), Some(ClassB::new(10)));

    let chain = instance.when_property_changed_with(child_age(), false);

    let (result, _subscription) = subscribe_result(chain);

    assert_eq!(result.get(), None);

    instance.child().unwrap().set_age(22);
    assert_eq!(result.get(), Some(22));

    instance.set_child(Some(ClassB::new(25)));
    assert_eq!(result.get(), Some(25));
    instance.child().unwrap().set_age(30);
    assert_eq!(result.get(), Some(30));
}

// Not ported: StressIt - a manual benchmark, not a test upstream (its [Fact] is commented out)

/// `ClassA`: a name and a child that may be null, raising their names when they change. Two are
/// equal when their names and children are.
pub(crate) struct ClassA {
    notify: AbstractNotifyPropertyChanged,
    child: RefCell<Option<Rc<ClassB>>>,
    name: RefCell<Option<String>>,
}

impl ClassA {
    /// `new ClassA { Name = name, Child = child }`.
    pub(crate) fn new(name: Option<String>, child: Option<Rc<ClassB>>) -> Rc<Self> {
        Rc::new(Self { notify: AbstractNotifyPropertyChanged::new(), child: RefCell::new(child), name: RefCell::new(name) })
    }

    /// `Child`.
    pub(crate) fn child(&self) -> Option<Rc<ClassB>> {
        self.child.borrow().clone()
    }

    /// Sets `Child`, raising it when it changes.
    pub(crate) fn set_child(&self, value: Option<Rc<ClassB>>) {
        self.notify.set_and_raise(&self.child, value, "Child");
    }

    /// `Name`.
    pub(crate) fn name(&self) -> Option<String> {
        self.name.borrow().clone()
    }

    /// Sets `Name`, raising it when it changes.
    pub(crate) fn set_name(&self, value: Option<String>) {
        self.notify.set_and_raise(&self.name, value, "Name");
    }
}

impl INotifyPropertyChanged for ClassA {
    fn property_changed(&self) -> &Event<str> {
        self.notify.property_changed()
    }
}

impl PartialEq for ClassA {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other) || (*self.name.borrow() == *other.name.borrow() && *self.child.borrow() == *other.child.borrow())
    }
}

impl fmt::Display for ClassA {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ClassA: Name={}, Child: ", self.name.borrow().as_deref().unwrap_or(""))?;
        match &*self.child.borrow() {
            Some(child) => write!(f, "{child}"),
            None => Ok(()),
        }
    }
}

/// `ClassB`: an age that raises `Age` when it changes. Two are equal when their ages are.
pub(crate) struct ClassB {
    notify: AbstractNotifyPropertyChanged,
    age: Cell<i32>,
}

impl ClassB {
    /// `new ClassB { Age = age }`.
    pub(crate) fn new(age: i32) -> Rc<Self> {
        Rc::new(Self { notify: AbstractNotifyPropertyChanged::new(), age: Cell::new(age) })
    }

    /// `Age`.
    pub(crate) fn age(&self) -> i32 {
        self.age.get()
    }

    /// Sets `Age`, raising it when it changes.
    pub(crate) fn set_age(&self, value: i32) {
        self.notify.set_and_raise(&self.age, value, "Age");
    }
}

impl INotifyPropertyChanged for ClassB {
    fn property_changed(&self) -> &Event<str> {
        self.notify.property_changed()
    }
}

impl PartialEq for ClassB {
    fn eq(&self, other: &Self) -> bool {
        self.age.get() == other.age.get()
    }
}

impl fmt::Display for ClassB {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Age: {}", self.age.get())
    }
}

/// `ClassC`: a child that may be null and does not notify, raising `Child` when another one is
/// set (by reference, as `ClassD` has no `Equals`).
pub(crate) struct ClassC {
    notify: AbstractNotifyPropertyChanged,
    child: RefCell<Option<Rc<ClassD>>>,
}

impl ClassC {
    /// `new ClassC { Child = child }`.
    pub(crate) fn new(child: Option<Rc<ClassD>>) -> Rc<Self> {
        Rc::new(Self { notify: AbstractNotifyPropertyChanged::new(), child: RefCell::new(child) })
    }

    /// `Child`.
    pub(crate) fn child(&self) -> Option<Rc<ClassD>> {
        self.child.borrow().clone()
    }

    /// Sets `Child`, raising it when it is another object.
    pub(crate) fn set_child(&self, value: Option<Rc<ClassD>>) {
        self.notify.set_and_raise_with(
            &self.child,
            value,
            |current, value| match (current, value) {
                (Some(current), Some(value)) => Rc::ptr_eq(current, value),
                (None, None) => true,
                _ => false,
            },
            "Child",
        );
    }
}

impl INotifyPropertyChanged for ClassC {
    fn property_changed(&self) -> &Event<str> {
        self.notify.property_changed()
    }
}

impl fmt::Display for ClassC {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ClassC: Child=")?;
        match &*self.child.borrow() {
            Some(child) => write!(f, "{child}"),
            None => Ok(()),
        }
    }
}

/// `ClassD`: a value that does not notify its changes.
pub(crate) struct ClassD {
    value: Cell<i32>,
}

impl ClassD {
    /// `new ClassD { Value = value }`.
    pub(crate) fn new(value: i32) -> Rc<Self> {
        Rc::new(Self { value: Cell::new(value) })
    }

    /// `Value`.
    pub(crate) fn value(&self) -> i32 {
        self.value.get()
    }

    /// Sets `Value`, without a notification.
    pub(crate) fn set_value(&self, value: i32) {
        self.value.set(value);
    }
}

impl fmt::Display for ClassD {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ClassD: Value={}", self.value.get())
    }
}
