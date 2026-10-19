//! `DeferredPropertyObservableTests`
//! (`ReactiveUI.Binding.Tests/Observables/DeferredPropertyObservableTests.cs`).

use crate::tests::binding::test_models::RecordingObserver;
use ferroui_reactiveui::*;
use ferroui_system::{IObservable, IObserver};
use std::cell::RefCell;
use std::rc::Rc;

const FIRST_VALUE: &str = "first";
const SECOND_VALUE: &str = "second";

#[test]
fn subscribe_reads_the_property_when_each_subscriber_arrives() {
    let source = Rc::new(Holder { value: RefCell::new(FIRST_VALUE.to_string()) });
    let observable = DeferredPropertyObservable::new(source.clone(), |holder: &Rc<Holder>| holder.value.borrow().clone());
    let first = RecordingObserver::<String>::new();
    let second = RecordingObserver::<String>::new();

    let first_subscription = observable.subscribe(first.clone() as Rc<dyn IObserver<String>>);
    *source.value.borrow_mut() = SECOND_VALUE.to_string();
    let second_subscription = observable.subscribe(second.clone() as Rc<dyn IObserver<String>>);

    assert_eq!(first.values(), vec![FIRST_VALUE.to_string()]);
    assert_eq!(second.values(), vec![SECOND_VALUE.to_string()]);
    second_subscription.dispose();
    first_subscription.dispose();
}

// Not ported: Subscribe_NullObserver_Throws - argument validation

// Not ported: Constructor_NullGetter_Throws - argument validation

/// An object with a value and no change notification (`Holder`).
struct Holder {
    value: RefCell<String>,
}
