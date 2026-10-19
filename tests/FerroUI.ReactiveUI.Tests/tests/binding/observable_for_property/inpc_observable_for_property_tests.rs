//! `INPCObservableForPropertyTests` (`ReactiveUI.Binding.Tests/ObservableForProperty/INPCObservableForPropertyTests.cs`).
//!
//! Upstream passes a `Type` to `GetAffinityForObject` and an object with an expression to
//! `GetNotificationForProperty`; the port passes the object as a `PropertyOwner` and the name of
//! the property. The `null` property name of .NET's `PropertyChangedEventArgs` (every property
//! changed) is the empty name. The tests of indexer expressions are not ported: a property path
//! has no indexers.

use crate::tests::binding::test_models::{PocoModel, TestViewModel};
use ferroui_reactive::ObservableExt;
use ferroui_reactiveui::*;
use ferroui_system::component_model::{INotifyPropertyChanged, INotifyPropertyChanging, PropertyOwner};
use ferroui_system::events::Event;
use ferroui_system::{IDisposable, IObservable};
use std::any::Any;
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;

/// The affinity returned for the objects that raise `PropertyChanged`.
const INPC_AFFINITY: i32 = 5;

/// A sample name value assigned during notification tests.
const SAMPLE_NAME: &str = "Alice";

/// A sample indexer value used during notification tests.
const SAMPLE_VALUE: &str = "value";

/// The name of the property that is not an indexer used in the `PropertyChanging` tests.
const SOME_PROP_NAME: &str = "SomeProp";

/// A sample age value assigned to an unrelated property.
const SAMPLE_AGE: i32 = 30;

/// The expected number of emissions when distinct filtering is disabled and the initial value is
/// not skipped: the initial value plus the three subsequent property changes.
const EXPECTED_NON_DISTINCT_COUNT: usize = 4;

/// The owner of the properties of an object that raises `PropertyChanged`.
fn notifying_owner<M: INotifyPropertyChanged + 'static>(object: &Rc<M>) -> PropertyOwner {
    PropertyOwner { object: Some(object.clone() as Rc<dyn Any>), notify: Some(object.clone() as Rc<dyn INotifyPropertyChanged>) }
}

/// The owner of the properties of an object without change notification.
fn poco_owner<M: 'static>(object: &Rc<M>) -> PropertyOwner {
    PropertyOwner { object: Some(object.clone() as Rc<dyn Any>), notify: None }
}

/// Counts the notifications of `source` until the subscription is disposed.
fn count_notifications(source: &Rc<dyn IObservable<ObservedChange<PropertyOwner, ()>>>) -> (Rc<Cell<usize>>, Rc<dyn IDisposable>) {
    let count = Rc::new(Cell::new(0));
    let counter = count.clone();
    let subscription = source.subscribe_next(move |_| counter.set(counter.get() + 1));
    (count, subscription)
}

#[test]
fn get_affinity_for_object_inpc_type_returns5() {
    let sut = INPCObservableForProperty::new();

    let affinity = sut.get_affinity_for_object_after_change(&notifying_owner(&TestViewModel::new()), "Name");

    assert_eq!(affinity, INPC_AFFINITY);
}

#[test]
fn get_affinity_for_object_inpc_changing_type_returns5_for_before_changed() {
    let sut = INPCObservableForProperty::new();

    let affinity = sut.get_affinity_for_object(&notifying_owner(&TestViewModel::new()), "Name", true);

    assert_eq!(affinity, INPC_AFFINITY);
}

#[test]
fn get_affinity_for_object_poco_type_returns0() {
    let sut = INPCObservableForProperty::new();

    let affinity = sut.get_affinity_for_object_after_change(&poco_owner(&PocoModel::new()), "Value");

    assert_eq!(affinity, 0);
}

#[test]
fn get_notification_for_property_property_changed_emits_notification() {
    let sut = INPCObservableForProperty::new();
    let vm = TestViewModel::new();

    let (count, subscription) = count_notifications(&sut.get_notification_for_property_after_change(&notifying_owner(&vm), "Name"));

    vm.set_name(SAMPLE_NAME.to_string());

    assert!(count.get() > 0);
    subscription.dispose();
}

#[test]
fn get_notification_for_property_before_changed_emits_notification() {
    let sut = INPCObservableForProperty::new();
    let vm = TestViewModel::new();

    let (count, subscription) = count_notifications(&sut.get_notification_for_property_with_warnings(&notifying_owner(&vm), "Name", true));

    vm.set_name(SAMPLE_NAME.to_string());

    assert!(count.get() > 0);
    subscription.dispose();
}

#[test]
fn get_notification_for_property_different_property_does_not_emit() {
    let sut = INPCObservableForProperty::new();
    let vm = TestViewModel::new();

    let (count, subscription) = count_notifications(&sut.get_notification_for_property_after_change(&notifying_owner(&vm), "Name"));

    // Change Age, not Name.
    vm.set_age(SAMPLE_AGE);

    assert_eq!(count.get(), 0);
    subscription.dispose();
}

#[test]
fn get_notification_for_property_poco_type_returns_never() {
    let sut = INPCObservableForProperty::new();
    let model = PocoModel::new();
    model.set_value("test".to_string());

    let (count, subscription) = count_notifications(&sut.get_notification_for_property_after_change(&poco_owner(&model), "Value"));

    model.set_value("changed".to_string());

    assert_eq!(count.get(), 0);
    subscription.dispose();
}

#[test]
fn get_notification_for_property_null_property_name_emits_notification() {
    let sut = INPCObservableForProperty::new();
    let vm = NullPropertyNameViewModel::new();

    let (count, subscription) = count_notifications(&sut.get_notification_for_property_after_change(&notifying_owner(&vm), "Name"));

    vm.raise_all_properties_changed();

    assert!(count.get() > 0);
    subscription.dispose();
}

#[test]
fn get_affinity_for_object_non_changing_type_returns0_for_before_changed() {
    let sut = INPCObservableForProperty::new();

    let affinity = sut.get_affinity_for_object(&notifying_owner(&NonChangingViewModel::new()), "Name", true);

    assert_eq!(affinity, 0);
}

// Not ported: GetNotificationForProperty_IndexExpression_PropertyChanged_EmitsNotification -
// expression trees (an indexer expression; a property path has no indexers).
// Not ported: GetNotificationForProperty_IndexExpression_BeforeChanged_EmitsNotification -
// expression trees (same reason).
// Not ported: GetNotificationForProperty_IndexExpression_DifferentPropertyName_DoesNotEmit -
// expression trees (same reason).
// Not ported: GetNotificationForProperty_IndexExpression_NullPropertyName_EmitsNotification -
// expression trees (same reason).
// Not ported: ObservableForProperty_ByName_InvalidProperty_StillCreatesObservable - reflection (the
// string form takes the getter of the property, so there is no property name that does not exist).

#[test]
fn observable_for_property_by_name_skip_initial_false_emits_initial_value() {
    let vm = TestViewModel::new();
    vm.set_name("Initial".to_string());

    let received: Rc<RefCell<Option<String>>> = Rc::new(RefCell::new(None));
    let slot = received.clone();
    let _subscription = vm
        .observable_for_property_named("Name", |vm: &Rc<TestViewModel>| vm.name(), false, false, true)
        .subscribe_next(move |change| *slot.borrow_mut() = Some(change.value));

    assert_eq!(received.borrow().as_deref(), Some("Initial"));
}

#[test]
fn observable_for_property_by_name_is_distinct_false_allows_duplicates() {
    let vm = TestViewModel::new();

    let values: Rc<RefCell<Vec<String>>> = Rc::new(RefCell::new(Vec::new()));
    let sink = values.clone();
    let _subscription = vm
        .observable_for_property_named("Name", |vm: &Rc<TestViewModel>| vm.name(), false, false, false)
        .subscribe_next(move |change| sink.borrow_mut().push(change.value));

    vm.set_name(SAMPLE_NAME.to_string());
    vm.set_name(String::new());
    vm.set_name(SAMPLE_NAME.to_string());

    assert_eq!(values.borrow().len(), EXPECTED_NON_DISTINCT_COUNT);
}

#[test]
fn observable_for_property_by_name_null_property_value_returns_default() {
    let vm = NullablePropertyViewModel::new();

    let received: Rc<RefCell<Option<Option<String>>>> = Rc::new(RefCell::new(None));
    let slot = received.clone();
    let _subscription = vm
        .observable_for_property_named("NullableName", |vm: &Rc<NullablePropertyViewModel>| vm.nullable_name(), false, false, true)
        .subscribe_next(move |change| *slot.borrow_mut() = Some(change.value));

    // A change was received, and its value is absent.
    assert_eq!(*received.borrow(), Some(None));
}

// Not ported: GetNotificationForProperty_IndexExpression_BeforeChanged_NullPropertyName_EmitsNotification -
// expression trees (an indexer expression; a property path has no indexers).
// Not ported: GetNotificationForProperty_IndexExpression_BeforeChanged_DifferentName_DoesNotEmit -
// expression trees (same reason).

#[test]
fn get_notification_for_property_non_index_before_changed_null_property_name_emits_notification() {
    let sut = INPCObservableForProperty::new();
    let vm = IndexableViewModel::new();

    let (count, subscription) = count_notifications(&sut.get_notification_for_property_with_warnings(&notifying_owner(&vm), SOME_PROP_NAME, true));

    vm.raise_all_property_changing();

    assert!(count.get() > 0);
    subscription.dispose();
}

#[test]
fn get_notification_for_property_non_index_before_changed_different_property_does_not_emit() {
    let sut = INPCObservableForProperty::new();
    let vm = IndexableViewModel::new();

    let (count, subscription) = count_notifications(&sut.get_notification_for_property_with_warnings(&notifying_owner(&vm), SOME_PROP_NAME, true));

    vm.set_item("key", SAMPLE_VALUE.to_string()); // Raises PropertyChanging for "Item[]", not "SomeProp".

    assert_eq!(count.get(), 0);
    subscription.dispose();
}

// Not ported: GetNotificationForProperty_IndexExpression_PropertyChanged_NullPropertyName_EmitsNotification_Variant -
// expression trees (an indexer expression; a property path has no indexers).

#[test]
fn get_notification_for_property_non_index_property_changed_null_property_name_emits_notification() {
    let sut = INPCObservableForProperty::new();
    let vm = IndexableViewModel::new();

    let (count, subscription) = count_notifications(&sut.get_notification_for_property_after_change(&notifying_owner(&vm), SOME_PROP_NAME));

    vm.raise_all_properties_changed();

    assert_eq!(count.get(), 1);
    subscription.dispose();
}

/// `PropertyChanged` with the empty name matches the observation of every property (the empty and
/// the null name both mean "all properties changed").
#[test]
fn property_changed_empty_property_name_matches_all_properties() {
    let sut = INPCObservableForProperty::new();
    let vm = IndexableViewModel::new();

    let (count, subscription) = count_notifications(&sut.get_notification_for_property_after_change(&notifying_owner(&vm), SOME_PROP_NAME));

    vm.raise_property_changed_with_name("");

    assert_eq!(count.get(), 1);
    subscription.dispose();
}

/// `PropertyChanging` with the empty name matches the observation of every property.
#[test]
fn property_changing_empty_property_name_matches_all_properties() {
    let sut = INPCObservableForProperty::new();
    let vm = IndexableViewModel::new();

    let (count, subscription) = count_notifications(&sut.get_notification_for_property_with_warnings(&notifying_owner(&vm), SOME_PROP_NAME, true));

    vm.raise_property_changing_with_name("");

    assert_eq!(count.get(), 1);
    subscription.dispose();
}

/// A model with an indexer that raises `PropertyChanged` and `PropertyChanging`
/// (`IndexableViewModel`); the indexer is `item` / `set_item`, which raise the name `Item[]`.
struct IndexableViewModel {
    items: RefCell<HashMap<String, String>>,
    some_prop: RefCell<String>,
    property_changed: Event<str>,
    property_changing: Event<str>,
}

#[allow(dead_code)]
impl IndexableViewModel {
    fn new() -> Rc<Self> {
        Rc::new(Self {
            items: RefCell::new(HashMap::new()),
            some_prop: RefCell::new(String::new()),
            property_changed: Event::new(),
            property_changing: Event::new(),
        })
    }

    /// A property that is not an indexer (`SomeProp`).
    fn some_prop(&self) -> String {
        self.some_prop.borrow().clone()
    }

    /// The value of `key` (`this[key]`).
    fn item(&self, key: &str) -> String {
        self.items.borrow().get(key).cloned().unwrap_or_default()
    }

    /// Sets the value of `key`, raising `Item[]` (`this[key] = value`).
    fn set_item(&self, key: &str, value: String) {
        self.property_changing.raise("Item[]");
        self.items.borrow_mut().insert(key.to_string(), value);
        self.property_changed.raise("Item[]");
    }

    /// Raises `PropertyChanged` for every property (`RaiseAllPropertiesChanged`).
    fn raise_all_properties_changed(&self) {
        self.property_changed.raise("");
    }

    /// Raises `PropertyChanged` with `name` (`RaisePropertyChangedWithName`).
    fn raise_property_changed_with_name(&self, name: &str) {
        self.property_changed.raise(name);
    }

    /// Raises `PropertyChanging` for every property (`RaiseAllPropertyChanging`).
    fn raise_all_property_changing(&self) {
        self.property_changing.raise("");
    }

    /// Raises `PropertyChanging` with `name` (`RaisePropertyChangingWithName`).
    fn raise_property_changing_with_name(&self, name: &str) {
        self.property_changing.raise(name);
    }
}

impl INotifyPropertyChanged for IndexableViewModel {
    fn property_changed(&self) -> &Event<str> {
        &self.property_changed
    }

    fn as_notify_property_changing(&self) -> Option<&dyn INotifyPropertyChanging> {
        Some(self)
    }
}

impl INotifyPropertyChanging for IndexableViewModel {
    fn property_changing(&self) -> &Event<str> {
        &self.property_changing
    }
}

/// A model with a property that may hold nothing (`NullablePropertyViewModel`).
struct NullablePropertyViewModel {
    nullable_name: RefCell<Option<String>>,
    property_changed: Event<str>,
}

impl NullablePropertyViewModel {
    fn new() -> Rc<Self> {
        Rc::new(Self { nullable_name: RefCell::new(None), property_changed: Event::new() })
    }

    fn nullable_name(&self) -> Option<String> {
        self.nullable_name.borrow().clone()
    }
}

impl INotifyPropertyChanged for NullablePropertyViewModel {
    fn property_changed(&self) -> &Event<str> {
        &self.property_changed
    }
}

/// A model that raises `PropertyChanged` but not `PropertyChanging` (`NonChangingViewModel`).
struct NonChangingViewModel {
    property_changed: Event<str>,
}

impl NonChangingViewModel {
    fn new() -> Rc<Self> {
        Rc::new(Self { property_changed: Event::new() })
    }
}

impl INotifyPropertyChanged for NonChangingViewModel {
    fn property_changed(&self) -> &Event<str> {
        &self.property_changed
    }
}

/// A model that raises `PropertyChanged` for every property (`NullPropertyNameViewModel`).
struct NullPropertyNameViewModel {
    property_changed: Event<str>,
}

impl NullPropertyNameViewModel {
    fn new() -> Rc<Self> {
        Rc::new(Self { property_changed: Event::new() })
    }

    /// Raises `PropertyChanged` for every property (`RaiseAllPropertiesChanged`).
    fn raise_all_properties_changed(&self) {
        self.property_changed.raise("");
    }
}

impl INotifyPropertyChanged for NullPropertyNameViewModel {
    fn property_changed(&self) -> &Event<str> {
        &self.property_changed
    }
}
