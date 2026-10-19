//! `WhenPropertyChangedFixture.UnitTests` (`DynamicData.Tests/Binding/WhenPropertyChangedFixture.UnitTests.Shallow.cs`):
//! the observation of a property of the object itself.

use super::when_property_changed_fixture::TestModel;
use ferroui_dynamic_data::binding::{NotifyPropertyChangedEx, PropertyValue};
use ferroui_reactive::ObservableExt;
use ferroui_system::prop;
use std::cell::RefCell;
use std::rc::Rc;

// Not ported: Shallow_InitialGetterThrows_DefaultErrorHandler_DetachesHandler - a getter cannot fail

// Not ported: Shallow_InitialGetterThrows_ErrorIsRecordedAndHandlerDetached - a getter cannot fail

// Not ported: Shallow_InitialObserverThrows_DetachesHandler - an observer cannot throw (a panic unwinds through the subscription; it is not an exception that Subscribe rethrows after detaching)

#[test]
fn shallow_notify_initial_false_does_not_dedup_same_valued_events() {
    let model = TestModel::new(10);
    let emissions = Rc::new(RefCell::new(Vec::new()));

    let sink = emissions.clone();
    let sub = model
        .when_property_changed_with(prop!(TestModel => value), false)
        .subscribe_next(move |pv: PropertyValue<Rc<TestModel>, i32>| sink.borrow_mut().push(pv.value().copied().unwrap_or_default()));

    model.set_value(42);
    model.set_value(42);

    assert_eq!(*emissions.borrow(), vec![42, 42]);
    sub.dispose();
}

#[test]
fn shallow_notify_initial_false_subscribes_handler_before_returning() {
    // notifyOnInitialValue=false: Subscribe must return only after the PropertyChanged handler
    // is attached. A setter that fires immediately after Subscribe returns must reach the
    // observer.
    let model = TestModel::new(10);
    let emissions = Rc::new(RefCell::new(Vec::new()));

    let sink = emissions.clone();
    let sub = model
        .when_property_changed_with(prop!(TestModel => value), false)
        .subscribe_next(move |pv: PropertyValue<Rc<TestModel>, i32>| sink.borrow_mut().push(pv.value().copied().unwrap_or_default()));

    model.set_value(20);

    assert_eq!(*emissions.borrow(), vec![20]);
    sub.dispose();
}

#[test]
fn shallow_notify_initial_true_does_not_dedup_same_valued_events() {
    let model = TestModel::new(10);
    let emissions = Rc::new(RefCell::new(Vec::new()));

    let sink = emissions.clone();
    let sub = model
        .when_property_changed_with(prop!(TestModel => value), true)
        .subscribe_next(move |pv: PropertyValue<Rc<TestModel>, i32>| sink.borrow_mut().push(pv.value().copied().unwrap_or_default()));

    model.set_value(10);
    model.set_value(10);
    model.set_value(10);

    assert_eq!(*emissions.borrow(), vec![10, 10, 10, 10]);
    sub.dispose();
}
