//! `ToPropertyUnsafeTests` (`ReactiveUI.Binding.Tests/Bindings/ToPropertyUnsafeTests.cs`).
//!
//! Upstream's `ToPropertyUnsafe` finds by reflection how its owner raises its notifications (a
//! public `RaisePropertyChanged(PropertyChangedEventArgs)`, a protected `OnPropertyChanged(string)`,
//! inherited field-like events, ReactiveUI's extension methods). The port's `to_property` takes a
//! reactive object, which raises them through `IReactiveObject`: each test that checks what the
//! owner sees is ported on a reactive object, and the notifications are read from its `changing()`
//! and `changed()`. The tests of what reflection refuses are not ported.

use ferroui_reactive::subjects::{BehaviorSubject, Subject, SubjectBase};
use ferroui_reactive::ObservableExt;
use ferroui_reactiveui::*;
use ferroui_system::prop;
use ferroui_system::IObserver;
use std::cell::RefCell;
use std::rc::Rc;

const FIRST: &str = "first";
const SECOND: &str = "second";
const INITIAL: &str = "initial";
const CHANGING_NAME: &str = "changing:Name";
const CHANGED_NAME: &str = "changed:Name";
const PROPERTY_NAME: &str = "Name";

fn some(value: &str) -> Option<String> {
    Some(value.to_string())
}

/// A reactive object whose `Name` is an output property (upstream's models, each with its own way
/// of raising).
#[derive(Default)]
struct NameModel {
    reactive: ReactiveObject,
    helper: RefCell<Option<ObservableAsPropertyHelper<Option<String>>>>,
}
reactive_object!(NameModel);

impl NameModel {
    fn new() -> Rc<Self> {
        Rc::new(Self::default())
    }

    /// `Name`: the value of the helper, `None` before it is set.
    fn name(&self) -> Option<String> {
        let helper = self.helper.borrow().clone();
        helper.and_then(|helper| helper.value())
    }

    fn set_helper(&self, helper: ObservableAsPropertyHelper<Option<String>>) {
        *self.helper.borrow_mut() = Some(helper);
    }
}

/// The names of the properties that `model` reports changed (`Raised`).
fn record_changed(model: &Rc<NameModel>) -> Rc<RefCell<Vec<String>>> {
    let raised: Rc<RefCell<Vec<String>>> = Rc::new(RefCell::new(Vec::new()));
    let sink = raised.clone();
    let _ = model.changed().subscribe_next(move |args| sink.borrow_mut().push(args.property_name));
    raised
}

/// The notifications of `model`, each as `changing:Name` or `changed:Name`, in the order raised.
fn record_both(model: &Rc<NameModel>) -> Rc<RefCell<Vec<String>>> {
    let raised: Rc<RefCell<Vec<String>>> = Rc::new(RefCell::new(Vec::new()));
    let (changing, changed) = (raised.clone(), raised.clone());
    let _ = model.changing().subscribe_next(move |args| changing.borrow_mut().push(format!("changing:{}", args.property_name)));
    let _ = model.changed().subscribe_next(move |args| changed.borrow_mut().push(format!("changed:{}", args.property_name)));
    raised
}

#[test]
fn public_event_args_raise_method_raises_changed() {
    let source = Subject::<Option<String>>::new();
    let model = NameModel::new();
    let raised = record_changed(&model);
    model.set_helper(source.as_observable().to_property(&model, prop!(NameModel => name)));

    // A helper that subscribes straight away announces its initial value; only later values are
    // counted here.
    raised.borrow_mut().clear();
    source.on_next(some(FIRST));

    assert_eq!(model.name(), some(FIRST));
    assert_eq!(*raised.borrow(), vec![PROPERTY_NAME.to_string()]);
}

#[test]
fn protected_name_raise_method_with_runtime_name_raises_changed() {
    let source = Subject::<Option<String>>::new();
    let model = NameModel::new();
    let raised = record_changed(&model);
    model.set_helper(source.as_observable().to_property_by_name(&model, PROPERTY_NAME, ObservableAsPropertyInitialValue::Value(some(INITIAL)), false, None));

    assert_eq!(model.name(), some(INITIAL));
    raised.borrow_mut().clear();
    source.on_next(some(FIRST));

    assert_eq!(model.name(), some(FIRST));
    assert_eq!(*raised.borrow(), vec![PROPERTY_NAME.to_string()]);
}

#[test]
fn inherited_field_like_events_raise_changing_then_changed() {
    let source = Subject::<Option<String>>::new();
    let model = NameModel::new();
    let helper = source.as_observable().to_property(&model, prop!(NameModel => name));
    model.set_helper(helper);
    let raised = record_both(&model);

    source.on_next(some(FIRST));
    source.on_next(some(SECOND));

    assert_eq!(model.name(), some(SECOND));
    assert_eq!(*raised.borrow(), vec![CHANGING_NAME, CHANGED_NAME, CHANGING_NAME, CHANGED_NAME]);
}

#[test]
fn reactive_object_raises_through_extensions() {
    let source = Subject::<Option<String>>::new();
    let model = NameModel::new();
    model.set_helper(source.as_observable().to_property_with(
        &model,
        prop!(NameModel => name),
        ObservableAsPropertyInitialValue::factory(|| some(INITIAL)),
        false,
        None,
    ));

    assert_eq!(model.name(), some(INITIAL));
    let raised = record_both(&model);
    source.on_next(some(FIRST));

    assert!(raised.borrow().iter().any(|name| name == CHANGED_NAME));
    assert!(raised.borrow().iter().any(|name| name == CHANGING_NAME));
}

#[test]
fn deferred_subscribes_on_first_read() {
    let source = BehaviorSubject::new(some(FIRST));
    let model = NameModel::new();
    model.set_helper(source.as_observable().to_property_with(
        &model,
        prop!(NameModel => name),
        ObservableAsPropertyInitialValue::Value(some(INITIAL)),
        true,
        None,
    ));

    assert!(!source.has_observers());
    assert_eq!(model.name(), some(FIRST));
    assert!(source.has_observers());
}

// Not ported: NoNotification_Throws - reflection (an owner without change notification is refused
// by the type checker: `to_property` takes a reactive object)

// Not ported: IndirectSelector_Throws - expression trees (a property path names its property; there
// is no selector to inspect)
