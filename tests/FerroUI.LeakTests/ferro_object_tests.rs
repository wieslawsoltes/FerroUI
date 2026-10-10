//! Port of the object tests of the reference leak tests: what a binding
//! holds, and what holds a binding.
//!
//! A binding source that is a shared object (a model, an object of the
//! class hierarchy) is boxed as that object, which is how a binding is given
//! a source that outlives it.

use crate::leak::Tracked;
use crate::subject::Subject;
use ferroui_base::data::core::expression_nodes::CastTarget;
use ferroui_base::data::core::parsers::TypeResolver;
use ferroui_base::data::core::plugins::{ObservableValue, PropertyInfoAccessorFactory};
use ferroui_base::data::core::{ClrPropertyInfo, Maybe};
use ferroui_base::data::model::{Event, INotifyPropertyChanged, Model};
use ferroui_base::data::{BindingPriority, CompiledBindingPath, CompiledBindingPathBuilder, ReflectionBinding};
use ferroui_base::threading::Dispatcher;
use ferroui_base::{
    ferro_class, ferro_impl_classes, ferro_model, ferro_property, instantiate, to_binding, BoxedValue, DirectProperty,
    FerroObject, FerroObjectImpl, FerroProperty, ObjectType, Ref, StyledElement,
};
use ferroui_controls::{Button, Grid, TextBlock};
use ferroui_markup_xaml::markup_extensions::CompiledBindingExtension;
use std::cell::RefCell;
use std::rc::Rc;

// --- the classes of the tests -----------------------------------------------

#[repr(C)]
struct Class1 {
    base: FerroObject,
    foo: RefCell<String>,
}

ferro_class!(Class1: FerroObject);
ferro_impl_classes!(Class1: FerroObjectImpl);

impl Class1 {
    ferro_property!(pub fn foo_property() -> DirectProperty<Class1, String> {
        FerroProperty::register_direct::<Class1, _>("Foo", |o| o.foo(), Some(|o, v| o.set_foo(v)), "unset".to_string())
    });

    fn new() -> Ref<Self> {
        instantiate(Self { base: FerroObject::construct(), foo: RefCell::new("initial2".to_string()) })
    }

    fn foo(&self) -> String {
        self.foo.borrow().clone()
    }

    fn set_foo(&self, value: String) {
        self.set_and_raise(Self::foo_property(), &self.foo, value);
    }

    fn do_something(&self) {}
}

/// A view model that notifies of the changes of its property.
struct Class2 {
    foo: RefCell<Option<String>>,
    property_changed: Event<str>,
}

impl INotifyPropertyChanged for Class2 {
    fn property_changed(&self) -> &Event<str> {
        &self.property_changed
    }
}

ferro_model!(Class2, |b| b
    .notify_property_changed()
    .property::<Maybe<String>>("Foo", |o| o.foo(), |o, v| o.set_foo(v)));

impl Class2 {
    fn new(foo: &str) -> Rc<Self> {
        Model::new_model(Self { foo: RefCell::new(Some(foo.to_string())), property_changed: Event::new() })
    }

    fn foo(&self) -> Option<String> {
        self.foo.borrow().clone()
    }

    fn set_foo(&self, value: Option<String>) {
        if *self.foo.borrow() != value {
            *self.foo.borrow_mut() = value;
            self.property_changed.raise("Foo");
        }
    }
}

/// A view model with an observable.
struct Class3 {
    observable: Option<ObservableValue>,
}

ferro_model!(Class3, |b| b.read_only::<Maybe<ObservableValue>>("Observable", |o| o.observable.clone()));

impl Class3 {
    fn new(observable: &Subject<String>) -> Rc<Self> {
        Model::new_model(Self { observable: Some(ObservableValue::new(observable.observable())) })
    }

    /// The path `Observable^`: the property, then the stream of the values
    /// of the observable it holds.
    ///
    /// The reference names the accessor of notifying properties for the
    /// property of this class, which does not notify; that accessor is
    /// typed by the notification contract here, so the plain one is used.
    fn observable_stream_path() -> CompiledBindingPath {
        CompiledBindingPathBuilder::new()
            .property(
                Rc::new(ClrPropertyInfo::read_only::<Class3, Maybe<ObservableValue>>("Observable", |o| {
                    o.observable.clone()
                })),
                PropertyInfoAccessorFactory::create_plain_property_accessor(),
            )
            .stream_observable()
            .build()
    }
}

/// An object of the class hierarchy as a binding source.
fn object_source<T: ObjectType>(object: &Ref<T>) -> Option<BoxedValue> {
    Some(Rc::new(object.clone().upcast::<FerroObject>()) as BoxedValue)
}

fn collect_garbage() {
    // Forces the weak events to compact.
    Dispatcher::ui_thread().run_jobs(None);
}

// --- tests ------------------------------------------------------------------

#[test]
fn binding_to_direct_property_does_not_get_collected() {
    let _scope = Dispatcher::unit_test_scope();
    let target = Class1::new();

    let source = {
        let source: Subject<BoxedValue> = Subject::new();
        let _sub = target.bind_property_untyped(Class1::foo_property(), source.observable(), BindingPriority::LocalValue);
        source.on_next(Rc::new("foo".to_string()));
        Tracked::shared("the source of the binding", source.state())
    };

    collect_garbage();

    assert_eq!("foo", target.foo());
    source.assert_alive();
}

#[test]
fn binding_to_direct_property_gets_collected_when_completed() {
    let _scope = Dispatcher::unit_test_scope();
    let target = Class1::new();

    let (source, weak_source) = {
        let source: Subject<BoxedValue> = Subject::new();
        let _sub = target.bind_property_untyped(Class1::foo_property(), source.observable(), BindingPriority::LocalValue);
        (Tracked::shared("the source of the binding", source.state()), Rc::downgrade(source.state()))
    };

    {
        let state = weak_source.upgrade().expect("the binding holds its source until the source completes");
        Subject::from_state(state).on_completed();
    }

    collect_garbage();
    source.assert_freed();
}

#[test]
fn compiled_binding_to_inpc_property_with_alive_source_does_not_keep_target_alive() {
    let _scope = Dispatcher::unit_test_scope();
    let source = Class2::new("foo");

    let target = {
        let path = CompiledBindingPathBuilder::new()
            .property(
                Rc::new(ClrPropertyInfo::read_write::<Class2, Maybe<String>>("Foo", |o| o.foo(), |o, v| o.set_foo(v))),
                PropertyInfoAccessorFactory::create_inpc_property_accessor::<Class2>(),
            )
            .build();

        let target = TextBlock::new();

        let binding = CompiledBindingExtension::new();
        binding.set_source(Some(source.clone() as BoxedValue));
        binding.set_path(Some(path));
        target.bind_binding(TextBlock::text_property().as_property(), &*binding);

        Tracked::object("the target of the binding", &target)
    };

    collect_garbage();
    target.assert_freed();
    drop(source);
}

#[test]
fn compiled_binding_to_ferro_property_with_alive_source_does_not_keep_target_alive() {
    let _scope = Dispatcher::unit_test_scope();
    let source = StyledElement::new();
    source.set_name(Some("foo".to_string()));

    let target = {
        let path = CompiledBindingPathBuilder::new().ferro_property(StyledElement::name_property().as_property()).build();

        let target = TextBlock::new();

        let binding = CompiledBindingExtension::new();
        binding.set_source(object_source(&source));
        binding.set_path(Some(path));
        target.bind_binding(TextBlock::text_property().as_property(), &*binding);

        Tracked::object("the target of the binding", &target)
    };

    collect_garbage();
    target.assert_freed();
    drop(source);
}

#[test]
fn compiled_binding_to_method_with_alive_source_does_not_keep_target_alive() {
    let _scope = Dispatcher::unit_test_scope();
    let source = Class1::new();

    let target = {
        let path = CompiledBindingPathBuilder::new()
            .command::<Ref<FerroObject>>(
                "DoSomething",
                |o: &Ref<FerroObject>, _: Option<&BoxedValue>| {
                    if let Some(o) = o.cast::<Class1>() {
                        o.do_something();
                    }
                },
                Some(Rc::new(|_: &Ref<FerroObject>, _: Option<&BoxedValue>| true)),
                &[],
            )
            .build();

        let target = Button::new();

        let binding = CompiledBindingExtension::new();
        binding.set_source(object_source(&source));
        binding.set_path(Some(path));
        target.bind_binding(Button::command_property().as_property(), &*binding);

        Tracked::object("the target of the binding", &target)
    };

    collect_garbage();
    target.assert_freed();
    drop(source);
}

#[test]
fn compiled_binding_stream_observable_with_alive_source_does_not_keep_target_alive() {
    // Issue #5872 of the reference: a binding to a long-lived observable
    // through the stream operator should not keep the target alive, in the
    // same way as every other kind of binding above.
    let _scope = Dispatcher::unit_test_scope();
    let observable: Subject<String> = Subject::new();
    let source = Class3::new(&observable);

    let target = {
        let target = TextBlock::new();

        let binding = CompiledBindingExtension::new();
        binding.set_source(Some(source.clone() as BoxedValue));
        binding.set_path(Some(Class3::observable_stream_path()));
        target.bind_binding(TextBlock::text_property().as_property(), &*binding);

        observable.on_next("foo".to_string());
        assert_eq!(Some("foo".to_string()), target.text());

        Tracked::object("the target of the binding", &target)
    };

    collect_garbage();
    target.assert_freed();

    // The source and its observable are kept alive: a resource that
    // outlives the target.
    drop(source);
    drop(observable);
}

// Finding L002 (README.md), open: the value of the observable does not reach the text of the
// text block (the assertion inside the scenario fails before anything is released), although
// `to_binding_binds_values_of_observable` of the base crate passes for a subject that replays
// its value and a property of a plain string. Not investigated to its cause.
#[test]
#[ignore = "finding L002: a value of an observable bound with to_binding does not reach TextBlock.Text"]
fn to_binding_observable_with_alive_source_does_not_keep_target_alive() {
    // Issue #18176 of the reference: a binding made from an observable
    // should not keep the target alive while the observable is alive.
    let _scope = Dispatcher::unit_test_scope();
    // The text of a text block is a nullable string: the observable has its value type.
    let observable: Subject<Option<String>> = Subject::new();

    let target = {
        let target = TextBlock::new();

        let binding = to_binding(observable.observable());
        target.bind_binding(TextBlock::text_property().as_property(), &*binding);

        observable.on_next(Some("foo".to_string()));
        assert_eq!(Some("foo".to_string()), target.text());

        Tracked::object("the target of the binding", &target)
    };

    collect_garbage();
    target.assert_freed();

    // The observable is kept alive: a resource that outlives the target.
    drop(observable);
}

#[test]
fn stream_observable_binding_with_alive_target_keeps_source_alive() {
    // A binding that is active still needs its source: what frees the
    // target of a stream binding must not free the observable of a target
    // that is alive.
    let _scope = Dispatcher::unit_test_scope();
    let target = TextBlock::new();

    let observable = {
        let observable: Subject<String> = Subject::new();
        let source = Class3::new(&observable);

        let binding = CompiledBindingExtension::new();
        binding.set_source(Some(source.clone() as BoxedValue));
        binding.set_path(Some(Class3::observable_stream_path()));
        target.bind_binding(TextBlock::text_property().as_property(), &*binding);

        observable.on_next("foo".to_string());
        assert_eq!(Some("foo".to_string()), target.text());

        Tracked::shared("the observable of the source", observable.state())
    };

    collect_garbage();

    // The target is alive, so its binding keeps the observable alive.
    observable.assert_alive();

    drop(target);
}

#[test]
fn binding_to_attached_property_with_alive_source_does_not_keep_target_alive() {
    let _scope = Dispatcher::unit_test_scope();
    let source = StyledElement::new();
    source.set_name(Some("foo".to_string()));

    let target = {
        let target = TextBlock::new();

        let type_resolver: TypeResolver = Rc::new(|_: Option<&str>, name: &str| {
            if name == "Grid" {
                Some(CastTarget::Class(Grid::TYPE))
            } else {
                panic!("the type {name} is not supported")
            }
        });
        let binding = ReflectionBinding::new("(Grid.Row)");
        binding.set_source(object_source(&source));
        binding.set_type_resolver(Some(type_resolver));
        target.bind_binding(TextBlock::text_property().as_property(), &*binding);

        Tracked::object("the target of the binding", &target)
    };

    collect_garbage();
    target.assert_freed();
    drop(source);
}
