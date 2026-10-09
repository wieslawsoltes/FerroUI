//! String-path (reflection) bindings over markup metadata: the members of
//! view models declared only with `ferro_markup_type!`.

use super::{IPropertyAccessorPlugin, InpcPropertyAccessorPlugin, MethodAccessorPlugin};
use crate::data::converters::MethodToCommandConverter;
use crate::data::core::{ValueTypes, WeakValue};
use crate::data::model::{Event, ICommand, INotifyPropertyChanged};
use crate::data::{BindingMode, BindingNotification, ReflectionBinding};
use crate::logging::{ILogSink, LogArea, LogEventLevel, Logger};
use crate::metadata::{MarkupDelegate, MarkupType, MarkupTyped};
use crate::threading::Dispatcher;
use crate::{
    ferro_class, ferro_impl_classes, ferro_markup_type, ferro_property, instantiate, BoxedValue, FerroObjectImpl,
    FerroProperty, Ref, StyledElement, StyledElementImpl, StyledProperty,
};
use std::any::Any;
use std::cell::{Cell, RefCell};
use std::fmt::Display;
use std::rc::Rc;

fn boxed<T: PartialEq + 'static>(value: T) -> BoxedValue {
    Rc::new(value)
}

// --- the view models ----------------------------------------------------

pub struct MemberBaseVm {
    base_calls: Cell<i32>,
}

impl PartialEq for MemberBaseVm {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl MemberBaseVm {
    fn inherited(&self) -> String {
        "inherited".to_string()
    }

    fn base_method(&self) {
        self.base_calls.set(self.base_calls.get() + 1);
    }
}

ferro_markup_type!(class MemberBaseVm {
    this: Rc<MemberBaseVm>,
    handles: [MemberBaseVm, Rc<MemberBaseVm>, Option<Rc<MemberBaseVm>>],
    properties: [Inherited: String { get: |vm: &Rc<MemberBaseVm>| vm.inherited() }],
    methods: [fn BaseMethod() => |vm: &Rc<MemberBaseVm>| vm.base_method()],
});

pub struct MemberVm {
    base: Rc<MemberBaseVm>,
    name: RefCell<String>,
    age: Cell<i32>,
    nick: RefCell<Option<String>>,
    child: RefCell<Option<Rc<MemberVm>>>,
    parameter: RefCell<Option<BoxedValue>>,
    items: RefCell<Vec<String>>,
    log: RefCell<Vec<String>>,
    property_changed: Event<str>,
}

impl PartialEq for MemberVm {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl INotifyPropertyChanged for MemberVm {
    fn property_changed(&self) -> &Event<str> {
        &self.property_changed
    }
}

impl MemberVm {
    fn new(name: &str) -> Rc<Self> {
        Rc::new(Self {
            base: Rc::new(MemberBaseVm { base_calls: Cell::new(0) }),
            name: RefCell::new(name.to_string()),
            age: Cell::new(30),
            nick: RefCell::new(None),
            child: RefCell::new(None),
            parameter: RefCell::new(None),
            items: RefCell::new(vec!["zero".to_string(), "one".to_string()]),
            log: RefCell::new(Vec::new()),
            property_changed: Event::new(),
        })
    }

    fn name(&self) -> String {
        self.name.borrow().clone()
    }

    fn set_name(&self, value: String) {
        self.name.replace(value);
        self.property_changed.raise("Name");
    }

    fn age(&self) -> i32 {
        self.age.get()
    }

    fn set_age(&self, value: i32) {
        self.age.set(value);
        self.property_changed.raise("Age");
    }

    fn nick(&self) -> Option<String> {
        self.nick.borrow().clone()
    }

    fn set_nick(&self, value: Option<String>) {
        self.nick.replace(value);
        self.property_changed.raise("Nick");
    }

    fn child(&self) -> Option<Rc<MemberVm>> {
        self.child.borrow().clone()
    }

    fn set_child(&self, value: Option<Rc<MemberVm>>) {
        self.child.replace(value);
        self.property_changed.raise("Child");
    }

    fn parameter(&self) -> Option<BoxedValue> {
        self.parameter.borrow().clone()
    }

    fn set_parameter(&self, value: Option<BoxedValue>) {
        self.parameter.replace(value);
        self.property_changed.raise("Parameter");
    }

    fn save(&self) {
        self.log.borrow_mut().push("Save".to_string());
    }

    fn do_(&self, parameter: Option<BoxedValue>) {
        self.log.borrow_mut().push(format!("Do {}", ValueTypes::to_display_string(parameter.as_ref())));
    }

    fn can_do(&self, parameter: Option<BoxedValue>) -> bool {
        parameter.is_some()
    }

    fn count(&self, value: i32) {
        self.log.borrow_mut().push(format!("Count {value}"));
    }

    fn item(&self, index: i32) -> Result<String, String> {
        usize::try_from(index)
            .ok()
            .and_then(|index| self.items.borrow().get(index).cloned())
            .ok_or_else(|| "Index was out of range.".to_string())
    }

    fn set_item(&self, index: i32, value: String) -> Result<(), String> {
        let index = usize::try_from(index).map_err(|_| "Index was out of range.".to_string())?;
        match self.items.borrow_mut().get_mut(index) {
            Some(item) => *item = value,
            None => return Err("Index was out of range.".to_string()),
        }
        self.property_changed.raise("Item");
        Ok(())
    }

    fn log(&self) -> Vec<String> {
        self.log.borrow().clone()
    }
}

ferro_markup_type!(class MemberVm {
    this: Rc<MemberVm>,
    handles: [MemberVm, Rc<MemberVm>, Option<Rc<MemberVm>>],
    base: Rc<MemberBaseVm>,
    properties: [
        Name: String { get: MemberVm::name, set: MemberVm::set_name },
        Age: i32 { get: MemberVm::age, set: MemberVm::set_age },
        Nick: Option<String> { get: MemberVm::nick, set: MemberVm::set_nick },
        Child: Option<Rc<MemberVm>> { get: MemberVm::child, set: MemberVm::set_child },
        Parameter: Option<BoxedValue> { get: MemberVm::parameter, set: MemberVm::set_parameter },
    ],
    indexers: [(i32) -> String { try_get: MemberVm::item, try_set: MemberVm::set_item }],
    methods: [
        fn Save() => MemberVm::save,
        fn Do(Option<BoxedValue>) => MemberVm::do_,
        fn CanDo(Option<BoxedValue>) -> bool => MemberVm::can_do [DependsOn("Parameter")],
        fn Count(i32) => MemberVm::count,
        fn Ambiguous(i32) => MemberVm::count,
        fn Ambiguous(String) => (|_: &Rc<MemberVm>, _: String| {}),
        fn TooMany(i32, i32) => (|_: &Rc<MemberVm>, _: i32, _: i32| {}),
    ],
    notify_property_changed: MemberVm,
});

/// What the declaring crate registers once (`ValueTypes::register_global`):
/// the view models are reference types, and the derived one is its base.
fn register() {
    MarkupType::register_all(&[<MemberBaseVm as MarkupTyped>::MARKUP, <MemberVm as MarkupTyped>::MARKUP]);
    ValueTypes::register_reference::<MemberBaseVm>();
    ValueTypes::register_reference::<MemberVm>();
    ValueTypes::register_cast::<MemberVm, Rc<MemberBaseVm>>(|vm| vm.base.clone());
}

// --- the binding target ---------------------------------------------------

#[repr(C)]
pub struct MemberTarget {
    base: StyledElement,
}

ferro_class!(MemberTarget: StyledElement);
ferro_impl_classes!(MemberTarget: StyledElementImpl);

impl FerroObjectImpl for MemberTarget {}

impl MemberTarget {
    ferro_property!(pub fn text_property() -> StyledProperty<String> {
        FerroProperty::register::<MemberTarget, _>("Text", "default".to_string())
    });
    ferro_property!(pub fn maybe_text_property() -> StyledProperty<Option<String>> {
        FerroProperty::register::<MemberTarget, _>("MaybeText", None)
    });
    ferro_property!(pub fn command_property() -> StyledProperty<Option<Rc<dyn ICommand>>> {
        FerroProperty::register::<MemberTarget, _>("Command", None)
    });
    ferro_property!(pub fn tag_property() -> StyledProperty<Option<BoxedValue>> {
        FerroProperty::register::<MemberTarget, _>("Tag", None)
    });

    pub fn new() -> Ref<Self> {
        instantiate(Self { base: StyledElement::construct() })
    }

    fn text(&self) -> String {
        self.get_value(Self::text_property())
    }
}

fn target_for(vm: &Rc<MemberVm>) -> Ref<MemberTarget> {
    register();
    let target = MemberTarget::new();
    target.set_data_context(Some(vm.clone()));
    target
}

struct ErrorLog {
    messages: RefCell<Vec<String>>,
}

impl ErrorLog {
    fn start() -> Rc<Self> {
        let result = Rc::new(Self { messages: RefCell::new(Vec::new()) });
        Logger::set_thread_sink(Some(result.clone()));
        result
    }

    fn contains(&self, text: &str) -> bool {
        self.messages.borrow().iter().any(|message| message.contains(text))
    }
}

impl ILogSink for ErrorLog {
    fn is_enabled(&self, level: LogEventLevel, area: &str) -> bool {
        level >= LogEventLevel::Warning && area == LogArea::BINDING
    }

    fn log(&self, _level: LogEventLevel, _area: &str, _source: Option<&dyn Any>, message_template: &str) {
        self.messages.borrow_mut().push(message_template.to_string());
    }

    fn log_with_values(
        &self,
        _level: LogEventLevel,
        _area: &str,
        _source: Option<&dyn Any>,
        message_template: &str,
        property_values: &[&dyn Display],
    ) {
        let values: Vec<String> = property_values.iter().map(|value| value.to_string()).collect();
        self.messages.borrow_mut().push(format!("{message_template} {}", values.join(" ")));
    }
}

// --- properties -------------------------------------------------------------

#[test]
fn one_way_binding_reads_a_metadata_property_and_follows_changes() {
    let vm = MemberVm::new("Ann");
    let target = target_for(&vm);

    target.bind_binding(MemberTarget::text_property(), &ReflectionBinding::new("Name"));
    assert_eq!(target.text(), "Ann");

    vm.set_name("Bob".to_string());
    assert_eq!(target.text(), "Bob");

    // A number is converted to the text of the target.
    target.bind_binding(MemberTarget::maybe_text_property(), &ReflectionBinding::new("Age"));
    assert_eq!(target.get_value(MemberTarget::maybe_text_property()).as_deref(), Some("30"));
    vm.set_age(31);
    assert_eq!(target.get_value(MemberTarget::maybe_text_property()).as_deref(), Some("31"));
}

#[test]
fn two_way_binding_writes_a_metadata_property() {
    let vm = MemberVm::new("Ann");
    let target = target_for(&vm);

    target.bind_binding(MemberTarget::text_property(), &ReflectionBinding::new("Name").with_mode(BindingMode::TwoWay));
    target.set_value(MemberTarget::text_property(), "Zed".to_string());
    assert_eq!(vm.name(), "Zed");

    // Text is converted to the type of the property.
    target.bind_binding(MemberTarget::text_property(), &ReflectionBinding::new("Age").with_mode(BindingMode::TwoWay));
    assert_eq!(target.text(), "30");
    target.set_value(MemberTarget::text_property(), "41".to_string());
    assert_eq!(vm.age(), 41);

    // A nullable property takes null and a value.
    target.bind_binding(
        MemberTarget::maybe_text_property(),
        &ReflectionBinding::new("Nick").with_mode(BindingMode::TwoWay),
    );
    target.set_value(MemberTarget::maybe_text_property(), Some("Annie".to_string()));
    assert_eq!(vm.nick().as_deref(), Some("Annie"));
    target.set_value(MemberTarget::maybe_text_property(), None);
    assert_eq!(vm.nick(), None);
}

#[test]
fn nested_path_follows_every_link() {
    let vm = MemberVm::new("Ann");
    let child = MemberVm::new("Kid");
    let grandchild = MemberVm::new("Baby");
    let target = target_for(&vm);

    target.bind_binding(MemberTarget::text_property(), &ReflectionBinding::new("Child.Child.Name"));
    assert_eq!(target.text(), "default");

    child.set_child(Some(grandchild.clone()));
    vm.set_child(Some(child.clone()));
    assert_eq!(target.text(), "Baby");

    grandchild.set_name("Toddler".to_string());
    assert_eq!(target.text(), "Toddler");

    child.set_child(Some(MemberVm::new("Other")));
    assert_eq!(target.text(), "Other");
}

#[test]
fn members_of_the_base_type_are_found() {
    let vm = MemberVm::new("Ann");
    let target = target_for(&vm);

    target.bind_binding(MemberTarget::text_property(), &ReflectionBinding::new("Inherited"));
    assert_eq!(target.text(), "inherited");

    target.bind_binding(MemberTarget::command_property(), &ReflectionBinding::new("BaseMethod"));
    let command = target.get_value(MemberTarget::command_property()).expect("a command");
    command.execute(None);
    assert_eq!(vm.base.base_calls.get(), 1);
}

#[test]
fn a_source_held_as_a_handle_is_read_like_the_object() {
    let vm = MemberVm::new("Ann");
    register();
    let target = MemberTarget::new();
    // The handle form: the box holds an `Rc<MemberVm>`.
    target.set_data_context(Some(boxed(vm.clone())));

    target.bind_binding(MemberTarget::text_property(), &ReflectionBinding::new("Name").with_mode(BindingMode::TwoWay));
    assert_eq!(target.text(), "Ann");
    vm.set_name("Bob".to_string());
    assert_eq!(target.text(), "Bob");
    target.set_value(MemberTarget::text_property(), "Cid".to_string());
    assert_eq!(vm.name(), "Cid");
}

#[test]
fn a_view_model_without_value_registration_is_read_through_its_handle() {
    pub struct Plain {
        text: RefCell<String>,
        property_changed: Event<str>,
    }
    impl PartialEq for Plain {
        fn eq(&self, other: &Self) -> bool {
            std::ptr::eq(self, other)
        }
    }
    impl INotifyPropertyChanged for Plain {
        fn property_changed(&self) -> &Event<str> {
            &self.property_changed
        }
    }
    ferro_markup_type!(class Plain {
        this: Rc<Plain>,
        handles: [Plain, Rc<Plain>, Option<Rc<Plain>>],
        properties: [Text: String { get: |vm: &Rc<Plain>| vm.text.borrow().clone() }],
        notify_property_changed: Plain,
    });
    MarkupType::register(<Plain as MarkupTyped>::MARKUP);

    let vm = Rc::new(Plain { text: RefCell::new("a".to_string()), property_changed: Event::new() });
    // Without `ValueTypes::register_reference` a view model travels as its
    // handle (the object form is what that registration defines).
    for source in [boxed(vm.clone()), boxed(vm.clone())] {
        let target = MemberTarget::new();
        target.set_data_context(Some(source));
        target.bind_binding(MemberTarget::text_property(), &ReflectionBinding::new("Text"));
        assert_eq!(target.text(), *vm.text.borrow());

        vm.text.replace(format!("{}b", vm.text.borrow().clone()));
        vm.property_changed.raise("Text");
        assert_eq!(target.text(), *vm.text.borrow());
    }
}

#[test]
fn missing_member_is_the_binding_error_of_the_managed_original() {
    let vm = MemberVm::new("Ann");
    let log = ErrorLog::start();
    let target = target_for(&vm);

    target.bind_binding(MemberTarget::text_property(), &ReflectionBinding::new("Missing"));
    assert_eq!(target.text(), "default");
    let type_name = <MemberVm as MarkupTyped>::MARKUP.full_name();
    let expected = format!("Could not find a matching property accessor for 'Missing' on '{type_name}'.");
    assert!(log.contains(&expected), "{:?}", log.messages.borrow());
    Logger::set_thread_sink(None);

    // The plugin itself reports the member it was asked for.
    let source: BoxedValue = vm.clone();
    let accessor = InpcPropertyAccessorPlugin.start(&WeakValue::new(&source), "Missing").expect("an accessor");
    let received: Rc<RefCell<Vec<Option<BoxedValue>>>> = Rc::new(RefCell::new(Vec::new()));
    accessor.subscribe({
        let received = received.clone();
        Rc::new(move |value| received.borrow_mut().push(value))
    });
    let value = received.borrow()[0].clone().expect("an error");
    let notification = value.downcast_ref::<BindingNotification>().expect("a notification");
    assert_eq!(
        notification.error().map(|error| error.to_string()),
        Some(format!("Could not find CLR property 'Missing' on '{type_name}'"))
    );
    assert!(!InpcPropertyAccessorPlugin.match_(&*source, "Missing"));
    assert!(InpcPropertyAccessorPlugin.match_(&*source, "Name"));
    assert!(InpcPropertyAccessorPlugin.match_(&*source, "Inherited"));
}

// --- methods ----------------------------------------------------------------

#[test]
fn method_binds_as_a_delegate() {
    let vm = MemberVm::new("Ann");
    let target = target_for(&vm);

    target.bind_binding(MemberTarget::tag_property(), &ReflectionBinding::new("Count"));
    let tag = target.get_value(MemberTarget::tag_property()).expect("a delegate");
    let delegate = tag.downcast_ref::<MarkupDelegate>().expect("a delegate");
    assert_eq!(delegate.method().map(|method| method.name), Some("Count"));
    assert!(delegate.target().is_some());
    assert_eq!(delegate.to_string(), "System.Action`1[System.Int32]");

    delegate.invoke(&[Some(boxed(3))]);
    assert_eq!(vm.log(), ["Count 3"]);
    // A failure is reported by the fallible form only.
    assert!(delegate.try_invoke(&[Some(boxed("x".to_string()))]).is_err());
    assert_eq!(delegate.invoke(&[Some(boxed("x".to_string()))]), None);

    // The text of a delegate is the name of its type.
    target.bind_binding(MemberTarget::text_property(), &ReflectionBinding::new("Save"));
    assert_eq!(target.text(), "System.Action");
    target.bind_binding(MemberTarget::text_property(), &ReflectionBinding::new("CanDo"));
    assert_eq!(target.text(), "System.Func`2[System.Object,System.Boolean]");
}

#[test]
fn method_binds_as_a_command() {
    let vm = MemberVm::new("Ann");
    let target = target_for(&vm);

    target.bind_binding(MemberTarget::command_property(), &ReflectionBinding::new("Save"));
    let command = target.get_value(MemberTarget::command_property()).expect("a command");
    assert!(command.can_execute(None));
    command.execute(Some(&boxed("ignored".to_string())));
    assert_eq!(vm.log(), ["Save"]);

    // The command parameter is converted to the type of the parameter.
    target.bind_binding(MemberTarget::command_property(), &ReflectionBinding::new("Count"));
    let command = target.get_value(MemberTarget::command_property()).expect("a command");
    command.execute(Some(&boxed("42".to_string())));
    command.execute(Some(&boxed(7)));
    assert_eq!(vm.log(), ["Save", "Count 42", "Count 7"]);
}

#[test]
fn command_asks_the_can_method_and_requeries_on_dependent_property_changes() {
    let _scope = Dispatcher::unit_test_scope();
    let vm = MemberVm::new("Ann");
    let target = target_for(&vm);

    target.bind_binding(MemberTarget::command_property(), &ReflectionBinding::new("Do"));
    let command = target.get_value(MemberTarget::command_property()).expect("a command");
    assert!(!command.can_execute(None));
    assert!(command.can_execute(Some(&boxed(1))));
    command.execute(Some(&boxed("A".to_string())));
    assert_eq!(vm.log(), ["Do A"]);

    let raised = Rc::new(Cell::new(0));
    let subscription = command.can_execute_changed({
        let raised = raised.clone();
        Rc::new(move || raised.set(raised.get() + 1))
    });

    vm.set_parameter(Some(boxed(1)));
    assert_eq!(raised.get(), 0, "the notification is posted to the dispatcher");
    Dispatcher::current_dispatcher().run_jobs(None);
    assert_eq!(raised.get(), 1);

    // A property the method does not depend on.
    vm.set_name("Bob".to_string());
    Dispatcher::current_dispatcher().run_jobs(None);
    assert_eq!(raised.get(), 1);

    // An empty name stands for every property.
    vm.property_changed.raise("");
    Dispatcher::current_dispatcher().run_jobs(None);
    assert_eq!(raised.get(), 2);

    subscription.dispose();
    vm.set_parameter(None);
    Dispatcher::current_dispatcher().run_jobs(None);
    assert_eq!(raised.get(), 2);
}

#[test]
fn method_to_command_converter_is_made_from_a_delegate_and_does_not_outlive_its_use() {
    let _scope = Dispatcher::unit_test_scope();
    register();
    let vm = MemberVm::new("Ann");
    let weak = Rc::downgrade(&vm);
    let markup = <MemberVm as MarkupTyped>::MARKUP;
    let method = markup.find_methods("Do").next().expect("the method");
    {
        let source: BoxedValue = vm.clone();
        let action = MarkupDelegate::for_method(Some(source), markup, method);
        let command = MethodToCommandConverter::new(&action);
        let count = Rc::new(Cell::new(0));
        let _subscription = command.can_execute_changed({
            let count = count.clone();
            Rc::new(move || count.set(count.get() + 1))
        });
        vm.set_parameter(Some(boxed(0)));
        Dispatcher::current_dispatcher().run_jobs(None);
        vm.set_parameter(None);
        Dispatcher::current_dispatcher().run_jobs(None);
        assert_eq!(count.get(), 2);
        assert_eq!(vm.property_changed.handler_count(), 1);
    }
    // The command is gone: so is its subscription, and nothing holds the view model.
    assert_eq!(vm.property_changed.handler_count(), 0);
    drop(vm);
    assert!(weak.upgrade().is_none());
}

#[test]
fn overloads_that_cannot_be_chosen_are_binding_errors() {
    let vm = MemberVm::new("Ann");
    register();
    let source: BoxedValue = vm.clone();
    let type_name = <MemberVm as MarkupTyped>::MARKUP.full_name();

    let error_of = |name: &str| {
        assert!(MethodAccessorPlugin.match_(&*source, name));
        let accessor = MethodAccessorPlugin.start(&WeakValue::new(&source), name).expect("an accessor");
        let received: Rc<RefCell<Vec<Option<BoxedValue>>>> = Rc::new(RefCell::new(Vec::new()));
        accessor.subscribe({
            let received = received.clone();
            Rc::new(move |value| received.borrow_mut().push(value))
        });
        let value = received.borrow()[0].clone().expect("an error");
        let notification = value.downcast_ref::<BindingNotification>().expect("a notification");
        notification.error().map(|error| error.to_string()).unwrap_or_default()
    };

    assert_eq!(
        error_of("Ambiguous"),
        format!(
            "Unable to resolve method of name 'Ambiguous' on type '{type_name}'. \
             Found 2 overloads accepting one parameter: 'System.Int32', 'System.String'. \
             Expected either a single overload with one parameter, or an overload accepting System.Object."
        )
    );
    assert_eq!(
        error_of("TooMany"),
        format!(
            "Unable to resolve method of name 'TooMany' on type '{type_name}'. \
             Found 1 overloads accepting more than one parameter. \
             Expected a method with zero or one parameter."
        )
    );
    assert!(!MethodAccessorPlugin.match_(&*source, "Missing"));

    // The command property stays unset.
    let target = target_for(&vm);
    target.bind_binding(MemberTarget::command_property(), &ReflectionBinding::new("Ambiguous"));
    assert!(target.get_value(MemberTarget::command_property()).is_none());
}

// --- indexers ---------------------------------------------------------------

#[test]
fn indexer_of_the_metadata_is_read_written_and_followed() {
    let vm = MemberVm::new("Ann");
    let target = target_for(&vm);

    target.bind_binding(MemberTarget::text_property(), &ReflectionBinding::new("[1]").with_mode(BindingMode::TwoWay));
    assert_eq!(target.text(), "one");

    // The owner announces a change of its indexer.
    vm.set_item(1, "uno".to_string()).unwrap();
    assert_eq!(target.text(), "uno");

    target.set_value(MemberTarget::text_property(), "eins".to_string());
    assert_eq!(vm.items.borrow()[1], "eins");

    // Behind a property.
    let parent = MemberVm::new("Parent");
    parent.set_child(Some(vm.clone()));
    let target = target_for(&parent);
    target.bind_binding(MemberTarget::text_property(), &ReflectionBinding::new("Child[0]"));
    assert_eq!(target.text(), "zero");
}

#[test]
fn indexer_errors_are_the_ones_of_the_managed_original() {
    let vm = MemberVm::new("Ann");
    let log = ErrorLog::start();
    let target = target_for(&vm);

    target.bind_binding(MemberTarget::text_property(), &ReflectionBinding::new("[1,2]"));
    assert!(log.contains("Wrong number of arguments for indexer: expected 1, got 2."), "{:?}", log.messages.borrow());

    target.bind_binding(MemberTarget::text_property(), &ReflectionBinding::new("[x]"));
    assert!(log.contains("Could not convert list index '0' of type 'x' to"), "{:?}", log.messages.borrow());

    // The getter fails.
    target.bind_binding(MemberTarget::text_property(), &ReflectionBinding::new("[9]"));
    assert!(log.contains("Index was out of range."), "{:?}", log.messages.borrow());
    assert_eq!(target.text(), "default");

    // A type without an indexer.
    target.set_data_context(Some(vm.base.clone()));
    target.bind_binding(MemberTarget::text_property(), &ReflectionBinding::new("[0]"));
    let type_name = <MemberBaseVm as MarkupTyped>::MARKUP.full_name();
    assert!(log.contains(&format!("Type '{type_name}' does not have an indexer.")), "{:?}", log.messages.borrow());
    Logger::set_thread_sink(None);
}

// --- the default value converter: a delegate to a command ---------------------

// The two tests of upstream's `DefaultValueConverterTests` that convert a delegate. Upstream's delegates are
// lambdas; a delegate that converts to a command here is the delegate of a method that metadata declares (the
// converter reads the parameters of the method, which the delegate of a closure does not have), so the tests
// are written on the view model of this file and assert what its methods record.

#[test]
fn can_convert_from_delegate_to_command() {
    let _scope = Dispatcher::unit_test_scope();
    register();
    let vm = MemberVm::new("Ann");
    let markup = <MemberVm as MarkupTyped>::MARKUP;
    let method = markup.find_methods("Count").next().expect("the method");
    let source: BoxedValue = vm.clone();
    let action: BoxedValue = Rc::new(MarkupDelegate::for_method(Some(source), markup, method));

    let result = crate::data::converters::DefaultValueConverter::instance()
        .convert(
            Some(&action),
            crate::data::core::ValueType::of::<Rc<dyn ICommand>>(),
            None,
            &crate::utilities::CultureInfo::invariant_culture(),
        )
        .expect("the conversion does not fail");

    let result = result.expect("a command");
    let command = result.downcast_ref::<Rc<dyn ICommand>>().expect("the result is a command").clone();

    command.execute(Some(&boxed(5)));

    assert_eq!(vm.log(), ["Count 5"]);
}

#[test]
fn can_convert_from_delegate_to_command_no_parameters() {
    let _scope = Dispatcher::unit_test_scope();
    register();
    let vm = MemberVm::new("Ann");
    let markup = <MemberVm as MarkupTyped>::MARKUP;
    let method = markup.find_methods("Save").next().expect("the method");
    let source: BoxedValue = vm.clone();
    let action: BoxedValue = Rc::new(MarkupDelegate::for_method(Some(source), markup, method));

    let result = crate::data::converters::DefaultValueConverter::instance()
        .convert(
            Some(&action),
            crate::data::core::ValueType::of::<Rc<dyn ICommand>>(),
            None,
            &crate::utilities::CultureInfo::invariant_culture(),
        )
        .expect("the conversion does not fail");

    let result = result.expect("a command");
    let command = result.downcast_ref::<Rc<dyn ICommand>>().expect("the result is a command").clone();

    command.execute(None);

    assert_eq!(vm.log(), ["Save"]);
}
