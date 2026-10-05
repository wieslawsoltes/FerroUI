//! Tests of the accessors compiled binding paths are built from.

use super::*;
use crate::register_types;
use crate::test_support::boxed;
use ferroui_base::data::core::plugins::{AccessorListener, IStreamPlugin, TaskValue, TaskValueSource};
use ferroui_base::data::core::{ClrPropertyInfo, IPropertyInfo, Value, ValueType, WeakValue};
use ferroui_base::data::model::{BindableList, Event, INotifyPropertyChanged, ModelTypes};
use ferroui_base::data::{BindingNotification, BindingPriority};
use ferroui_base::metadata::{MarkupTyped, MarkupTypeKind};
use ferroui_base::reactive::{IObserver, AnonymousObserver};
use ferroui_base::{BoxedValue, FerroProperty};
use ferroui_controls::{Border, Control};
use std::cell::RefCell;
use std::rc::Rc;

struct Person {
    name: RefCell<String>,
    changed: Event<str>,
}

impl Person {
    fn new(name: &str) -> Rc<Self> {
        if !ModelTypes::is_registered(std::any::TypeId::of::<Person>()) {
            ModelTypes::register::<Person>(|b| b.notify_property_changed());
        }
        Rc::new(Self { name: RefCell::new(name.to_string()), changed: Event::new() })
    }

    fn set_name(&self, value: &str) {
        *self.name.borrow_mut() = value.to_string();
        self.changed.raise("Name");
    }
}

impl PartialEq for Person {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl INotifyPropertyChanged for Person {
    fn property_changed(&self) -> &Event<str> {
        &self.changed
    }
}

fn name_property() -> Rc<dyn IPropertyInfo> {
    Rc::new(ClrPropertyInfo::read_write::<Person, Value<String>>(
        "Name",
        |p| p.name.borrow().clone(),
        |p, v| *p.name.borrow_mut() = v,
    ))
}

fn recorder() -> (Rc<RefCell<Vec<Option<BoxedValue>>>>, AccessorListener) {
    let values: Rc<RefCell<Vec<Option<BoxedValue>>>> = Rc::new(RefCell::new(Vec::new()));
    let sink = values.clone();
    (values, Rc::new(move |value| sink.borrow_mut().push(value)))
}

fn strings(values: &Rc<RefCell<Vec<Option<BoxedValue>>>>) -> Vec<String> {
    values.borrow().iter().map(|v| v.as_ref().and_then(|v| v.downcast_ref::<String>().cloned()).unwrap_or_default()).collect()
}

#[test]
fn notifying_property_accessor_reads_writes_and_follows_changes() {
    let person = Person::new("Ann");
    let source: BoxedValue = person.clone();
    let accessor = PropertyInfoAccessorFactory::create_inpc_property_accessor(&WeakValue::new(&source), name_property());

    assert_eq!(accessor.property_type(), Some(ValueType::of::<String>()));
    assert_eq!(accessor.value().unwrap().downcast_ref::<String>().unwrap(), "Ann");

    let (values, listener) = recorder();
    accessor.subscribe(listener);
    assert_eq!(strings(&values), ["Ann"]);

    person.set_name("Bob");
    assert_eq!(strings(&values), ["Ann", "Bob"]);
    // Other properties are ignored, an empty name means all properties.
    person.changed.raise("Other");
    person.changed.raise("");
    assert_eq!(strings(&values), ["Ann", "Bob", "Bob"]);

    // Writing publishes the current value.
    assert_eq!(accessor.set_value(Some(&boxed("Cat".to_string())), BindingPriority::LocalValue), Ok(true));
    assert_eq!(*person.name.borrow(), "Cat");
    assert_eq!(strings(&values).last().unwrap(), "Cat");

    accessor.unsubscribe();
    person.set_name("Dan");
    assert_eq!(strings(&values).last().unwrap(), "Cat");
    assert!(!person.changed.has_handlers());
}

#[test]
fn notifying_property_accessor_of_a_dead_or_read_only_source() {
    let read_only: Rc<dyn IPropertyInfo> =
        Rc::new(ClrPropertyInfo::read_only::<Person, Value<String>>("Name", |p| p.name.borrow().clone()));
    let person = Person::new("Ann");
    let source: BoxedValue = person.clone();
    let accessor = PropertyInfoAccessorFactory::create_inpc_property_accessor(&WeakValue::new(&source), read_only);
    assert_eq!(accessor.set_value(Some(&boxed("x".to_string())), BindingPriority::LocalValue), Ok(false));

    let weak = WeakValue::new(&source);
    let accessor = PropertyInfoAccessorFactory::create_inpc_property_accessor(&weak, name_property());
    drop(source);
    drop(person);
    assert!(accessor.value().is_none());
    assert_eq!(accessor.set_value(Some(&boxed("x".to_string())), BindingPriority::LocalValue), Ok(false));
    let (values, listener) = recorder();
    accessor.subscribe(listener);
    assert_eq!(values.borrow().len(), 1);
    assert!(values.borrow()[0].is_none());
    accessor.dispose();
}

#[test]
fn a_failing_getter_is_published_as_a_binding_error() {
    let person = Person::new("Ann");
    let source: BoxedValue = boxed(5i32);
    let _ = person;
    // The property belongs to another type than the source.
    let accessor = PropertyInfoAccessorFactory::create_inpc_property_accessor(&WeakValue::new(&source), name_property());
    let (values, listener) = recorder();
    accessor.subscribe(listener);
    let published = values.borrow()[0].clone().unwrap();
    assert!(published.is::<BindingNotification>());
}

#[test]
fn registered_property_accessor_reads_writes_and_follows_changes() {
    let border = Border::new();
    border.set_tag(Some(boxed("one".to_string())));
    let source: BoxedValue = boxed(border.clone());
    let property: &'static FerroProperty = Control::tag_property();
    let accessor = PropertyInfoAccessorFactory::create_ferro_property_accessor(&WeakValue::new(&source), property.as_property_info());

    assert_eq!(accessor.property_type(), Some(ValueType::of::<Option<BoxedValue>>()));
    assert_eq!(accessor.value().unwrap().downcast_ref::<String>().unwrap(), "one");

    let (values, listener) = recorder();
    accessor.subscribe(listener);
    border.set_tag(Some(boxed("two".to_string())));
    // Changes of other properties are ignored.
    border.set_is_visible(false);
    assert_eq!(strings(&values), ["one", "two"]);

    assert_eq!(accessor.set_value(Some(&boxed("three".to_string())), BindingPriority::LocalValue), Ok(true));
    assert_eq!(border.tag().unwrap().downcast_ref::<String>().unwrap(), "three");
    assert_eq!(strings(&values), ["one", "two", "three"]);

    let subscribers = border.property_changed_subscriber_count();
    accessor.unsubscribe();
    assert_eq!(border.property_changed_subscriber_count(), subscribers - 1);
    border.set_tag(None);
    assert_eq!(values.borrow().len(), 3);

    // A value of the wrong type is an error, as the setter throwing.
    let width: &'static FerroProperty = ferroui_base::layout::Layoutable::width_property();
    let accessor = PropertyInfoAccessorFactory::create_ferro_property_accessor(&WeakValue::new(&source), width.as_property_info());
    assert!(accessor.set_value(Some(&boxed("wide".to_string())), BindingPriority::LocalValue).is_err());
    assert_eq!(accessor.set_value(Some(&boxed(12.0f64)), BindingPriority::LocalValue), Ok(true));
    assert_eq!(border.width(), 12.0);
}

#[test]
fn registered_property_accessor_of_something_that_is_not_an_object() {
    let source: BoxedValue = boxed(5i32);
    let property: &'static FerroProperty = Control::tag_property();
    let accessor = PropertyInfoAccessorFactory::create_ferro_property_accessor(&WeakValue::new(&source), property.as_property_info());
    assert!(accessor.value().is_none());
    // The accessor of the base library: a write to a reference without a
    // target does nothing and reports success, and subscribing publishes
    // the (null) current value.
    assert_eq!(accessor.set_value(Some(&boxed(1i32)), BindingPriority::LocalValue), Ok(true));
    let (values, listener) = recorder();
    accessor.subscribe(listener);
    assert_eq!(values.borrow().len(), 1);
    assert!(values.borrow()[0].is_none());
    accessor.dispose();

    // A property description that is not a registered property is an error.
    let error = PropertyInfoAccessorFactory::try_create_ferro_property_accessor(&WeakValue::new(&source), name_property())
        .err()
        .unwrap();
    assert_eq!(error.message(), "Unable to cast the property 'Name' to type 'FerroProperty'.");
}

#[test]
fn indexer_accessor_follows_the_collection_changes_that_affect_its_index() {
    let list = BindableList::new([10, 20, 30]);
    let source: BoxedValue = list.clone();
    let item: Rc<dyn IPropertyInfo> = Rc::new(ClrPropertyInfo::read_only::<BindableList<i32>, Value<i32>>("Item", |l| {
        l.items().try_get(1).unwrap_or(-1)
    }));
    let accessor = PropertyInfoAccessorFactory::create_indexer_property_accessor(&WeakValue::new(&source), item, 1);

    let (values, listener) = recorder();
    accessor.subscribe(listener);
    let seen = || -> Vec<i32> { values.borrow().iter().map(|v| *v.as_ref().unwrap().downcast_ref::<i32>().unwrap()).collect() };
    assert_eq!(seen(), [20]);
    assert_eq!(accessor.property_type(), Some(ValueType::of::<i32>()));

    // An item added after the index does not affect it.
    list.items().add(40);
    assert_eq!(seen(), [20]);
    // An item inserted before it does.
    list.items().insert(0, 5);
    assert_eq!(seen(), [20, 10]);
    // Replacing the item at the index, and another one.
    list.items().set(1, 11);
    assert_eq!(seen(), [20, 10, 11]);
    list.items().set(3, 33);
    assert_eq!(seen(), [20, 10, 11]);
    // Removing an item after the index, then one before it.
    list.items().remove_at(4);
    assert_eq!(seen(), [20, 10, 11]);
    list.items().remove_at(0);
    assert_eq!(seen(), [20, 10, 11, 20]);
    // A reset always affects it.
    list.items().clear();
    assert_eq!(seen(), [20, 10, 11, 20, -1]);

    accessor.unsubscribe();
    list.items().add(1);
    list.items().add(2);
    assert_eq!(seen().len(), 5);
    assert_eq!(accessor.value().unwrap().downcast_ref::<i32>(), Some(&2));
    accessor.dispose();
}

#[test]
fn task_stream_plugin_streams_the_outcome_of_a_task() {
    let plugin = TaskStreamPlugin::<i32>::new();
    let not_a_task: BoxedValue = boxed(1i32);
    assert!(!plugin.match_(&WeakValue::new(&not_a_task)));

    let completed: BoxedValue = boxed(TaskValue::from_result(7i32));
    let reference = WeakValue::new(&completed);
    assert!(plugin.match_(&reference));
    let seen: Rc<RefCell<Vec<Option<BoxedValue>>>> = Rc::new(RefCell::new(Vec::new()));
    let sink = seen.clone();
    let observer: Rc<dyn IObserver<Option<BoxedValue>>> =
        Rc::new(AnonymousObserver::new(move |value: Option<BoxedValue>| sink.borrow_mut().push(value)));
    let _subscription = plugin.start(&reference).unwrap().subscribe(observer);
    assert_eq!(seen.borrow()[0].as_ref().unwrap().downcast_ref::<i32>(), Some(&7));

    let running: BoxedValue = boxed(TaskValueSource::new().task());
    assert!(plugin.match_(&WeakValue::new(&running)));
}

#[test]
fn the_factory_is_published_for_the_compiler() {
    use ferroui_base::data::core::plugins::IPropertyAccessor;
    use ferroui_base::metadata::{from_markup_value, into_markup_value};

    register_types();
    let markup = <PropertyInfoAccessorFactory as MarkupTyped>::MARKUP;
    assert_eq!(markup.kind, MarkupTypeKind::Static);
    assert_eq!(
        markup.full_name(),
        "FerroUI.Markup.Xaml.MarkupExtensions.CompiledBindings.PropertyInfoAccessorFactory"
    );
    let method = |name: &str| markup.find_methods(name).next().unwrap();

    let person = Person::new("Ann");
    let source: BoxedValue = person.clone();
    let accessor = (method("CreateInpcPropertyAccessor").invoke)(&[
        into_markup_value(WeakValue::new(&source)),
        into_markup_value(name_property()),
    ])
    .unwrap();
    let accessor = from_markup_value::<Rc<dyn IPropertyAccessor>>(&accessor).unwrap();
    assert_eq!(accessor.value().unwrap().downcast_ref::<String>().unwrap(), "Ann");

    let border = Border::new();
    border.set_tag(Some(boxed(4i32)));
    let target: BoxedValue = boxed(border);
    let tag: &'static FerroProperty = Control::tag_property();
    let accessor = (method("CreateFerroPropertyAccessor").invoke)(&[
        into_markup_value(WeakValue::new(&target)),
        into_markup_value(tag.as_property_info()),
    ])
    .unwrap();
    let accessor = from_markup_value::<Rc<dyn IPropertyAccessor>>(&accessor).unwrap();
    assert_eq!(accessor.value().unwrap().downcast_ref::<i32>(), Some(&4));

    let list = BindableList::new([1, 2]);
    let items: BoxedValue = list.clone();
    let item: Rc<dyn IPropertyInfo> =
        Rc::new(ClrPropertyInfo::read_only::<BindableList<i32>, Value<i32>>("Item", |l| l.items().try_get(0).unwrap_or(-1)));
    let accessor = (method("CreateIndexerPropertyAccessor").invoke)(&[
        into_markup_value(WeakValue::new(&items)),
        into_markup_value(item),
        into_markup_value(0i32),
    ])
    .unwrap();
    let accessor = from_markup_value::<Rc<dyn IPropertyAccessor>>(&accessor).unwrap();
    assert_eq!(accessor.value().unwrap().downcast_ref::<i32>(), Some(&1));
}

// View models that state their notifications in markup metadata.

struct DeclaredViewModel {
    title: RefCell<String>,
    changed: Event<str>,
}

impl PartialEq for DeclaredViewModel {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl INotifyPropertyChanged for DeclaredViewModel {
    fn property_changed(&self) -> &Event<str> {
        &self.changed
    }
}

ferroui_base::ferro_markup_type!(class DeclaredViewModel {
    handles: [DeclaredViewModel],
    namespace: "Tests.ViewModels",
    notify_property_changed: DeclaredViewModel,
});

/// Raises the same event, but does not declare it.
struct SilentViewModel {
    title: RefCell<String>,
    changed: Event<str>,
}

impl PartialEq for SilentViewModel {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

ferroui_base::ferro_markup_type!(class SilentViewModel {
    handles: [SilentViewModel],
    namespace: "Tests.ViewModels",
});

#[test]
fn a_view_model_that_declares_its_notifications_updates_a_bound_target() {
    use ferroui_base::data::{CompiledBinding, CompiledBindingPathBuilder};
    use ferroui_base::metadata::MarkupType;

    MarkupType::register(<DeclaredViewModel as MarkupTyped>::MARKUP);
    let model = Rc::new(DeclaredViewModel { title: RefCell::new("first".to_string()), changed: Event::new() });
    let source: BoxedValue = model.clone();
    let title: Rc<dyn IPropertyInfo> = Rc::new(ClrPropertyInfo::read_write::<DeclaredViewModel, Value<String>>(
        "Title",
        |m| m.title.borrow().clone(),
        |m, v| *m.title.borrow_mut() = v,
    ));

    // Through the accessor.
    let accessor = PropertyInfoAccessorFactory::create_inpc_property_accessor(&WeakValue::new(&source), title.clone());
    let (values, listener) = recorder();
    accessor.subscribe(listener);
    assert!(model.changed.has_handlers());
    *model.title.borrow_mut() = "second".to_string();
    model.changed.raise("Title");
    assert_eq!(strings(&values), ["first", "second"]);
    accessor.unsubscribe();
    assert!(!model.changed.has_handlers());

    // Through a compiled binding whose path uses the accessor factory.
    let path = CompiledBindingPathBuilder::new()
        .property(title, Rc::new(|target, property| PropertyInfoAccessorFactory::create_inpc_property_accessor(target, property)))
        .build();
    let binding = CompiledBinding::new(path).with_source(Some(source));
    let border = Border::new();
    let tag: &'static FerroProperty = Control::tag_property();
    let expression = border.bind_binding(tag, &binding);
    assert_eq!(border.tag().unwrap().downcast_ref::<String>().unwrap(), "second");

    *model.title.borrow_mut() = "third".to_string();
    model.changed.raise("Title");
    assert_eq!(border.tag().unwrap().downcast_ref::<String>().unwrap(), "third");
    expression.dispose();
    assert!(!model.changed.has_handlers());
}

#[test]
fn a_view_model_without_declared_notifications_is_read_once() {
    use ferroui_base::metadata::MarkupType;

    MarkupType::register(<SilentViewModel as MarkupTyped>::MARKUP);
    let model = Rc::new(SilentViewModel { title: RefCell::new("first".to_string()), changed: Event::new() });
    let source: BoxedValue = model.clone();
    let title: Rc<dyn IPropertyInfo> =
        Rc::new(ClrPropertyInfo::read_only::<SilentViewModel, Value<String>>("Title", |m| m.title.borrow().clone()));

    let accessor = PropertyInfoAccessorFactory::create_inpc_property_accessor(&WeakValue::new(&source), title);
    let (values, listener) = recorder();
    accessor.subscribe(listener);
    assert!(!model.changed.has_handlers());

    *model.title.borrow_mut() = "second".to_string();
    model.changed.raise("Title");
    assert_eq!(strings(&values), ["first"]);
    // The current value is still read on demand.
    assert_eq!(accessor.value().unwrap().downcast_ref::<String>().unwrap(), "second");
    accessor.unsubscribe();
}
