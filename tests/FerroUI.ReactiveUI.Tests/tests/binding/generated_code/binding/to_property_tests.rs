//! `ToPropertyTests` (`ReactiveUI.Binding.GeneratedCode.Tests/Binding/ToPropertyTests.cs`).
//!
//! The view models are those of `SharedScenarios/ToProperty`
//! ([`to_property_scenarios`](crate::tests::binding::generated_code::test_models::scenarios::to_property_scenarios)),
//! all reactive objects here: upstream's generator raises the owner's notifications in the way
//! each view model offers, the port's `to_property` through `IReactiveObject`.

use crate::tests::binding::generated_code::test_models::scenarios::to_property_scenarios::{
    factory_out_result, initial_value_defer_scheduler, name_of, partial_event, partial_protected_base, public_raise_method, reactive_object,
};
use crate::tests::binding::generated_code::test_models::SummaryViewModel;
use ferroui_reactive::concurrency::ImmediateScheduler;
use ferroui_reactive::subjects::{BehaviorSubject, Subject, SubjectBase};
use ferroui_reactiveui::*;
use ferroui_system::component_model::{INotifyPropertyChanged, INotifyPropertyChanging};
use ferroui_system::{IObservable, IObserver};
use std::cell::RefCell;
use std::rc::{Rc, Weak};

/// The derived name property the scenarios back.
const FULL_NAME: &str = "FullName";

/// A value pushed into a numeric property.
const PUSHED_VALUE: i32 = 3;

/// An age pushed into the summary view model.
const PUSHED_AGE: i32 = 36;

/// Records the property names a view model raises `PropertyChanged` for (`Record`).
fn record(source: &dyn INotifyPropertyChanged) -> Rc<RefCell<Vec<String>>> {
    let raised = Rc::new(RefCell::new(Vec::new()));
    let log = raised.clone();
    source.property_changed().add(Rc::new(move |name: &str| log.borrow_mut().push(name.to_string())));
    raised
}

/// A subject as the observable a view model follows.
fn source<T: Clone + 'static>(subject: &Rc<Subject<T>>) -> Rc<dyn IObservable<T>> {
    subject.clone() as Rc<dyn IObservable<T>>
}

#[test]
fn partial_event_raises_property_changed_per_distinct_value() {
    let names = Subject::<String>::new();
    let vm = partial_event::execute(source(&names));
    let raised = record(&*vm);

    names.on_next("Ada".to_string());
    names.on_next("Ada".to_string());
    names.on_next("Grace".to_string());

    assert_eq!(vm.full_name(), "Grace");
    assert_eq!(*raised.borrow(), vec![FULL_NAME.to_string(), FULL_NAME.to_string()]);
}

#[test]
fn partial_protected_base_raises_changing_then_changed() {
    let names = Subject::<String>::new();
    let vm = partial_protected_base::execute(source(&names));
    let order = Rc::new(RefCell::new(Vec::<String>::new()));

    // The handlers hold the view model weakly, as a handler on its own object must not keep it.
    let log = order.clone();
    let weak: Weak<partial_protected_base::MyViewModel> = Rc::downgrade(&vm);
    vm.property_changing().add(Rc::new(move |name: &str| {
        let full_name = weak.upgrade().map(|vm| vm.full_name()).unwrap_or_default();
        log.borrow_mut().push(format!("changing:{name}:{full_name}"));
    }));
    let log = order.clone();
    let weak: Weak<partial_protected_base::MyViewModel> = Rc::downgrade(&vm);
    vm.property_changed().add(Rc::new(move |name: &str| {
        let full_name = weak.upgrade().map(|vm| vm.full_name()).unwrap_or_default();
        log.borrow_mut().push(format!("changed:{name}:{full_name}"));
    }));

    names.on_next("Ada".to_string());

    assert_eq!(*order.borrow(), vec!["changing:FullName:".to_string(), "changed:FullName:Ada".to_string()]);
}

#[test]
fn public_raise_method_raises_property_changed() {
    let counts = Subject::<i32>::new();
    let vm = public_raise_method::execute(source(&counts));
    let raised = record(&*vm);

    counts.on_next(PUSHED_VALUE);

    assert_eq!(vm.count(), PUSHED_VALUE);
    assert_eq!(*raised.borrow(), vec!["Count".to_string()]);
}

#[test]
fn reactive_object_honours_suppressed_notifications() {
    let names = Subject::<String>::new();
    let vm = reactive_object::execute(source(&names));
    let raised = record(&*vm);

    let suppression = vm.suppress_change_notifications();
    names.on_next("hidden".to_string());
    suppression.dispose();

    names.on_next("shown".to_string());

    assert_eq!(vm.full_name(), "shown");
    assert_eq!(*raised.borrow(), vec![FULL_NAME.to_string()]);
}

#[test]
fn name_of_raises_property_changed() {
    let names = Subject::<String>::new();
    let vm = name_of::execute(source(&names));
    let raised = record(&*vm);

    names.on_next("Ada".to_string());

    assert_eq!(vm.full_name(), "Ada");
    assert_eq!(*raised.borrow(), vec![FULL_NAME.to_string()]);
}

#[test]
fn initial_value_deferred_subscribes_on_first_read() {
    let names = BehaviorSubject::new("Ada".to_string());
    let vm = initial_value_defer_scheduler::execute(names.clone() as Rc<dyn IObservable<String>>, None);

    assert!(!names.has_observers());
    assert_eq!(vm.full_name(), "Ada");
    assert!(names.has_observers());
}

#[test]
fn initial_value_deferred_immediate_scheduler_delivers_inline() {
    let names = Subject::<String>::new();
    let vm = initial_value_defer_scheduler::execute(source(&names), Some(ImmediateScheduler::instance()));

    assert_eq!(vm.full_name(), "(none)");

    names.on_next("Ada".to_string());

    assert_eq!(vm.full_name(), "Ada");
}

#[test]
fn factory_out_result_uses_factory_and_assigns_helper() {
    let counts = Subject::<i32>::new();
    let vm = factory_out_result::execute(source(&counts));

    assert_eq!(vm.count(), -1);

    counts.on_next(PUSHED_VALUE);

    assert_eq!(vm.count(), PUSHED_VALUE);
}

#[test]
fn observable_as_property_reads_helper_and_round_trips_null() {
    let names = Subject::<Option<String>>::new();
    let ages = Subject::<Option<i32>>::new();
    let vm = SummaryViewModel::new(source(&names), source(&ages));
    let raised = record(&*vm);

    names.on_next(Some("Ada".to_string()));
    ages.on_next(Some(PUSHED_AGE));
    names.on_next(None);

    assert_eq!(vm.full_name(), None);
    assert_eq!(vm.age(), Some(PUSHED_AGE));
    assert_eq!(*raised.borrow(), vec![FULL_NAME.to_string(), "Age".to_string(), FULL_NAME.to_string()]);
}
