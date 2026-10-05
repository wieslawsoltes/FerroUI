//! Ported from the upstream `Data/BindingTests_Method`.

use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};

use ferroui_base::data::core::ValueTypes;
use ferroui_base::data::converters::MethodToCommandConverter;
use ferroui_base::data::model::{Event, ICommand, INotifyPropertyChanged};
use ferroui_base::input::{InputElement, Key, KeyEventArgs};
use ferroui_base::logging::{LogArea, LogEventLevel};
use ferroui_base::metadata::{MarkupDelegate, MarkupTyped};
use ferroui_base::threading::Dispatcher;
use ferroui_base::{ferro_markup_type, BoxedValue, Ref};
use ferroui_controls::{Button, TextBlock, Window};

use crate::support::app::*;
use crate::support::helpers::*;
use crate::support::loader::*;
use crate::support::TypeModule;
use crate::support_bindings::*;

// --- test types -------------------------------------------------------------

/// The text of a value inside an interpolated string: empty for null.
fn display(value: &Option<BoxedValue>) -> String {
    value.as_ref().map(|value| ValueTypes::to_display_string(Some(value))).unwrap_or_default()
}

/// The base class of the view model: its virtual methods do nothing.
pub struct ViewModelBase;

crate::test_identity_eq!(ViewModelBase);

impl ViewModelBase {
    pub fn new() -> Rc<Self> {
        Rc::new(Self)
    }

    pub fn virtual_object_method(&self, _i: Option<BoxedValue>) {}

    pub fn virtual_int32_method(&self, _i: i32) {}

    pub fn virtual_string_method(&self, _i: Option<String>) {}

    pub fn method_with_new_slot(&self, _i: i32) {}
}

ferro_markup_type!(class ViewModelBase as "BindingTests_Method+ViewModelBase" {
    this: Rc<ViewModelBase>,
    handles: [ViewModelBase, Rc<ViewModelBase>, Option<Rc<ViewModelBase>>],
    constructors: [() => ViewModelBase::new],
    methods: [
        fn VirtualObjectMethod(Option<BoxedValue>) =>
            |this: &Rc<ViewModelBase>, i: Option<BoxedValue>| this.virtual_object_method(i),
        fn VirtualInt32Method(i32) => |this: &Rc<ViewModelBase>, i: i32| this.virtual_int32_method(i),
        fn VirtualStringMethod(Option<String>) =>
            |this: &Rc<ViewModelBase>, i: Option<String>| this.virtual_string_method(i),
        fn MethodWithNewSlot(i32) => |this: &Rc<ViewModelBase>, i: i32| this.method_with_new_slot(i),
    ],
});

pub struct ViewModel {
    base: Rc<ViewModelBase>,
    value: RefCell<String>,
    parameter: RefCell<Option<BoxedValue>>,
    property_changed: Event<str>,
}

crate::test_identity_eq!(ViewModel);

impl INotifyPropertyChanged for ViewModel {
    fn property_changed(&self) -> &Event<str> {
        &self.property_changed
    }
}

impl ViewModel {
    pub fn new() -> Rc<Self> {
        Rc::new(Self {
            base: ViewModelBase::new(),
            value: RefCell::new("Not called".to_string()),
            parameter: RefCell::new(None),
            property_changed: Event::new(),
        })
    }

    pub fn with_parameter(parameter: Option<BoxedValue>) -> Rc<Self> {
        let result = Self::new();
        result.set_parameter(parameter);
        result
    }

    /// The view model as its base class.
    pub fn base(&self) -> &Rc<ViewModelBase> {
        &self.base
    }

    fn set_value(&self, value: String) {
        self.value.replace(value);
    }

    pub fn method(&self) {
        self.set_value("Called".to_string());
    }

    pub fn object_method(&self, i: Option<BoxedValue>) {
        self.set_value(format!("Called ObjectMethod with {}", display(&i)));
    }

    pub fn int32_method(&self, i: i32) {
        self.set_value(format!("Called Int32Method with {i}"));
    }

    pub fn string_method(&self, i: Option<String>) {
        self.set_value(format!("Called StringMethod with {}", i.unwrap_or_default()));
    }

    pub fn method_with_overloads(&self) {
        self.set_value("Called MethodWithOverloads without parameter".to_string());
    }

    pub fn method_with_overloads_int32(&self, i: i32) {
        self.set_value(format!("Called MethodWithOverloads with Int32 {i}"));
    }

    pub fn method_with_overloads_string(&self, i: Option<String>) {
        self.set_value(format!("Called MethodWithOverloads with String {}", i.unwrap_or_default()));
    }

    pub fn method_with_overloads_object(&self, i: Option<BoxedValue>) {
        self.set_value(format!("Called MethodWithOverloads with Object {}", display(&i)));
    }

    pub fn method_with_overloads2(&self) {
        self.set_value("Called MethodWithOverloads2 without parameter".to_string());
    }

    pub fn method_with_overloads2_int32(&self, i: i32) {
        self.set_value(format!("Called MethodWithOverloads2 with Int32 {i}"));
    }

    pub fn method_with_overloads2_string(&self, i: Option<String>) {
        self.set_value(format!("Called MethodWithOverloads2 with String {}", i.unwrap_or_default()));
    }

    pub fn method_with_overloads3(&self) {
        self.set_value("Called MethodWithOverloads3 without parameter".to_string());
    }

    pub fn method_with_overloads3_int32(&self, _a: i32, _b: i32) -> Result<(), String> {
        Err("MethodWithOverloads3 should not be called".to_string())
    }

    pub fn method_with_overloads3_string(&self, _a: Option<String>, _b: Option<String>) -> Result<(), String> {
        Err("MethodWithOverloads3 should not be called".to_string())
    }

    pub fn method_with_overloads4_int32(&self, _a: i32, _b: i32) -> Result<(), String> {
        Err("MethodWithOverloads4 should not be called".to_string())
    }

    pub fn method_with_overloads4_string(&self, _a: Option<String>, _b: Option<String>) -> Result<(), String> {
        Err("MethodWithOverloads4 should not be called".to_string())
    }

    pub fn virtual_object_method(&self, i: Option<BoxedValue>) {
        self.set_value(format!("Called VirtualObjectMethod with {}", display(&i)));
    }

    pub fn virtual_int32_method(&self, i: i32) {
        self.set_value(format!("Called VirtualInt32Method with {i}"));
    }

    pub fn virtual_string_method(&self, i: Option<String>) {
        self.set_value(format!("Called VirtualStringMethod with {}", i.unwrap_or_default()));
    }

    pub fn method_with_new_slot(&self, i: i32) {
        self.set_value(format!("Called MethodWithNewSlot with {i}"));
    }

    pub fn value(&self) -> String {
        self.value.borrow().clone()
    }

    pub fn parameter(&self) -> Option<BoxedValue> {
        self.parameter.borrow().clone()
    }

    pub fn set_parameter(&self, value: Option<BoxedValue>) {
        if *self.parameter.borrow() == value {
            return;
        }
        self.parameter.replace(value);
        self.property_changed.raise("Parameter");
    }

    pub fn do_(&self, parameter: Option<BoxedValue>) {
        self.set_value(format!("Do {}", display(&parameter)));
    }

    pub fn can_do(&self, parameter: Option<BoxedValue>) -> bool {
        parameter.is_some()
    }
}

ferro_markup_type!(class ViewModel as "BindingTests_Method+ViewModel" {
    this: Rc<ViewModel>,
    handles: [ViewModel, Rc<ViewModel>, Option<Rc<ViewModel>>],
    base: Rc<ViewModelBase>,
    constructors: [() => ViewModel::new],
    properties: [
        Value: String { get: ViewModel::value },
        Parameter: Option<BoxedValue> { get: ViewModel::parameter, set: ViewModel::set_parameter },
    ],
    methods: [
        fn Method() => ViewModel::method,
        fn ObjectMethod(Option<BoxedValue>) => ViewModel::object_method,
        fn Int32Method(i32) => ViewModel::int32_method,
        fn StringMethod(Option<String>) => ViewModel::string_method,
        fn MethodWithOverloads() => ViewModel::method_with_overloads,
        fn MethodWithOverloads(i32) => ViewModel::method_with_overloads_int32,
        fn MethodWithOverloads(Option<String>) => ViewModel::method_with_overloads_string,
        fn MethodWithOverloads(Option<BoxedValue>) => ViewModel::method_with_overloads_object,
        fn MethodWithOverloads2() => ViewModel::method_with_overloads2,
        fn MethodWithOverloads2(i32) => ViewModel::method_with_overloads2_int32,
        fn MethodWithOverloads2(Option<String>) => ViewModel::method_with_overloads2_string,
        fn MethodWithOverloads3() => ViewModel::method_with_overloads3,
        try fn MethodWithOverloads3(i32, i32) => ViewModel::method_with_overloads3_int32,
        try fn MethodWithOverloads3(Option<String>, Option<String>) => ViewModel::method_with_overloads3_string,
        try fn MethodWithOverloads4(i32, i32) => ViewModel::method_with_overloads4_int32,
        try fn MethodWithOverloads4(Option<String>, Option<String>) => ViewModel::method_with_overloads4_string,
        fn VirtualObjectMethod(Option<BoxedValue>) => ViewModel::virtual_object_method,
        fn VirtualInt32Method(i32) => ViewModel::virtual_int32_method,
        fn VirtualStringMethod(Option<String>) => ViewModel::virtual_string_method,
        fn MethodWithNewSlot(i32) => ViewModel::method_with_new_slot,
        fn Do(Option<BoxedValue>) => ViewModel::do_,
        fn CanDo(Option<BoxedValue>) -> bool => ViewModel::can_do [DependsOn("Parameter")],
    ],
    notify_property_changed: ViewModel,
});

/// The test types of this file.
pub(crate) const MODULE: TypeModule = TypeModule {
    types: &[],
    markup_types: &[<ViewModelBase as MarkupTyped>::MARKUP, <ViewModel as MarkupTyped>::MARKUP],
    value_types: || {
        ValueTypes::register_reference::<ViewModelBase>();
        ValueTypes::register_reference::<ViewModel>();
        ValueTypes::register_cast::<ViewModel, Rc<ViewModelBase>>(|view_model| view_model.base().clone());
    },
};

fn perform_click(button: &Ref<Button>) {
    let mut e = KeyEventArgs::new();
    e.set_routed_event(Some(InputElement::key_down_event()));
    e.key = Key::Enter;
    button.raise_event(&e);
}

// --- tests ------------------------------------------------------------------

#[test]
fn binding_method_to_command_works() {
    let _app = styled_window_application();
    let xaml = r#"
<Window xmlns='https://github.com/ferroui'
        xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'
        xmlns:local='clr-namespace:FerroUI.Markup.Xaml.UnitTests.Xaml;assembly=FerroUI.Markup.Xaml.UnitTests'>
    <Button Name='button' Command='{Binding Method}'/>
</Window>"#;
    let window: Ref<Window> = load_as(xaml);
    let button = window.get_control::<Button>("button");
    let vm = ViewModel::new();

    button.set_data_context(Some(vm.clone()));
    window.apply_template();

    assert!(button.command().is_some());
    perform_click(&button);
    assert_eq!(vm.value(), "Called");
}

#[test]
fn binding_method_with_parameter_to_command_uses_single_parameter_overload() {
    let rows = [
        ("ObjectMethod", "<x:String>hello</x:String>", "Called ObjectMethod with hello"),
        ("StringMethod", "<x:String>hello</x:String>", "Called StringMethod with hello"),
        ("Int32Method", "<x:Int32>42</x:Int32>", "Called Int32Method with 42"),
        ("Int32Method", "<x:String>42</x:String>", "Called Int32Method with 42"),
        ("VirtualObjectMethod", "<x:String>hello</x:String>", "Called VirtualObjectMethod with hello"),
        ("VirtualStringMethod", "<x:String>hello</x:String>", "Called VirtualStringMethod with hello"),
        ("VirtualStringMethod", "<x:Null />", "Called VirtualStringMethod with "),
        ("VirtualInt32Method", "<x:Int32>42</x:Int32>", "Called VirtualInt32Method with 42"),
        ("MethodWithNewSlot", "<x:Int32>42</x:Int32>", "Called MethodWithNewSlot with 42"),
    ];
    for (method_name, xaml_parameter, expected) in rows {
        let _app = styled_window_application();

        let window: Ref<Window> = load_as(&format!(
            r#"<Window xmlns='https://github.com/ferroui'
        xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'>
    <Button Name='button' Command='{{Binding {method_name}}}'>
      <Button.CommandParameter>
        {xaml_parameter}
      </Button.CommandParameter>
    </Button>
</Window>"#
        ));
        let button = window.get_control::<Button>("button");
        let vm = ViewModel::new();

        button.set_data_context(Some(vm.clone()));
        window.apply_template();

        assert!(button.command().is_some(), "row ({method_name}, {xaml_parameter})");
        perform_click(&button);
        assert_eq!(vm.value(), expected, "row ({method_name}, {xaml_parameter})");
    }
}

#[test]
fn binding_method_with_parameter_to_command_prefers_object_overload() {
    let _app = styled_window_application();

    let window: Ref<Window> = load_as(
        r#"<Window xmlns='https://github.com/ferroui'
        xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'>
    <Button Name='button' Command='{Binding MethodWithOverloads}' CommandParameter='foo' />
</Window>"#,
    );
    let button = window.get_control::<Button>("button");
    let vm = ViewModel::new();

    button.set_data_context(Some(vm.clone()));
    window.apply_template();

    assert!(button.command().is_some());
    perform_click(&button);
    assert_eq!(vm.value(), "Called MethodWithOverloads with Object foo");
}

#[test]
fn binding_method_with_parameter_to_command_fails_with_multiple_single_parameter_overloads_without_object() {
    assert_binding_fails(
        "MethodWithOverloads2",
        concat!(
            "Unable to resolve method of name 'MethodWithOverloads2' on type ",
            "'FerroUI.Markup.Xaml.UnitTests.Data.BindingTests_Method+ViewModel'. ",
            "Found 2 overloads accepting one parameter: 'System.Int32', 'System.String'. ",
            "Expected either a single overload with one parameter, or an overload accepting System.Object."
        ),
    );
}

#[test]
fn binding_method_with_parameter_to_command_uses_parameterless_overload_when_no_overloads_with_parameter_exist() {
    let _app = styled_window_application();

    let window: Ref<Window> = load_as(
        r#"<Window xmlns='https://github.com/ferroui'
        xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'>
    <Button Name='button' Command='{Binding MethodWithOverloads3}' CommandParameter='foo' />
</Window>"#,
    );
    let button = window.get_control::<Button>("button");
    let vm = ViewModel::new();

    button.set_data_context(Some(vm.clone()));
    window.apply_template();

    assert!(button.command().is_some());
    perform_click(&button);
    assert_eq!(vm.value(), "Called MethodWithOverloads3 without parameter");
}

#[test]
fn binding_method_with_parameter_to_command_fails_without_valid_overloads() {
    assert_binding_fails(
        "MethodWithOverloads4",
        concat!(
            "Unable to resolve method of name 'MethodWithOverloads4' on type ",
            "'FerroUI.Markup.Xaml.UnitTests.Data.BindingTests_Method+ViewModel'. ",
            "Found 2 overloads accepting more than one parameter. ",
            "Expected a method with zero or one parameter."
        ),
    );
}

fn assert_binding_fails(method_name: &str, expected_error: &str) {
    let _app = styled_window_application();

    let errors: Rc<RefCell<Vec<String>>> = Rc::new(RefCell::new(Vec::new()));
    let _log_sink = TestLogSink::start({
        let errors = errors.clone();
        move |level, area, template, values| {
            if level >= LogEventLevel::Warning && area == LogArea::BINDING {
                errors.borrow_mut().push(format!("{template} {}", values.join(" ")));
            }
        }
    });

    let window: Ref<Window> = load_as(&format!(
        r#"<Window xmlns='https://github.com/ferroui'
        xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'>
    <Button Name='button' Command='{{Binding {method_name}}}' CommandParameter='foo' />
</Window>"#
    ));
    let button = window.get_control::<Button>("button");
    let vm = ViewModel::new();

    button.set_data_context(Some(vm));
    window.apply_template();

    assert!(button.command().is_none());
    assert!(
        errors.borrow().iter().any(|error| error.contains(expected_error)),
        "no logged binding error contains {expected_error:?}: {:?}",
        errors.borrow()
    );
}

#[test]
fn binding_method_to_text_block_text_works() {
    let _app = styled_window_application();
    let xaml = r#"
<Window xmlns='https://github.com/ferroui'
        xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'
        xmlns:local='clr-namespace:FerroUI.Markup.Xaml.UnitTests.Xaml;assembly=FerroUI.Markup.Xaml.UnitTests'>
    <TextBlock Name='textBlock' Text='{Binding Method}'/>
</Window>"#;
    let window: Ref<Window> = load_as(xaml);
    let text_block = window.get_control::<TextBlock>("textBlock");
    let vm = ViewModel::new();

    text_block.set_data_context(Some(vm));
    window.apply_template();

    assert!(text_block.text().is_some());
}

#[test]
fn binding_method_with_parameter_to_command_can_execute() {
    let rows: [(Option<BoxedValue>, &str); 2] = [(None, "Not called"), (boxed_str("A"), "Do A")];
    for (command_parameter, result) in rows {
        let _app = styled_window_application();
        let xaml = r#"
<Window xmlns='https://github.com/ferroui'
        xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'
        xmlns:local='clr-namespace:FerroUI.Markup.Xaml.UnitTests.Xaml;assembly=FerroUI.Markup.Xaml.UnitTests'>
    <Button Name='button' Command='{Binding Do}' CommandParameter='{Binding Parameter, Mode=OneTime}'/>
</Window>"#;
        let window: Ref<Window> = load_as(xaml);
        let button = window.get_control::<Button>("button");
        let vm = ViewModel::with_parameter(command_parameter);

        button.set_data_context(Some(vm.clone()));
        window.apply_template();

        assert!(button.command().is_some(), "row {result:?}");
        perform_click(&button);
        assert_eq!(vm.value(), result, "row {result:?}");
    }
}

#[test]
fn binding_method_with_parameter_to_command_can_execute_depends_on() {
    let _app = styled_window_application();
    let xaml = r#"
<Window xmlns='https://github.com/ferroui'
        xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'
        xmlns:local='clr-namespace:FerroUI.Markup.Xaml.UnitTests.Xaml;assembly=FerroUI.Markup.Xaml.UnitTests'>
    <Button Name='button' Command='{Binding Do}' CommandParameter='{Binding Parameter, Mode=OneWay}'/>
</Window>"#;
    let window: Ref<Window> = load_as(xaml);
    let button = window.get_control::<Button>("button");
    let vm = ViewModel::with_parameter(None);

    button.set_data_context(Some(vm.clone()));
    window.apply_template();

    assert!(button.command().is_some());

    assert!(!button.is_effectively_enabled());

    vm.set_parameter(obj(true));
    Dispatcher::ui_thread().run_jobs(None);

    assert!(button.is_effectively_enabled());
}

#[test]
fn binding_method_to_command_collected() {
    let _app = unit_test_application(TestServices::mock_platform_render_interface());

    fn is_alive(reference: &Weak<ViewModel>) -> bool {
        reference.upgrade().is_some()
    }

    // Objects are released when their last reference goes away, so the collection the
    // original forces is the end of the scope that holds the view model and its command:
    // the view model is observed before that scope ends and after it.
    fn make_ref() -> (Weak<ViewModel>, bool) {
        let vm = ViewModel::with_parameter(None);
        let weak_vm = Rc::downgrade(&vm);
        let can_execute_count = Rc::new(Cell::new(0));
        // `new Action<object>(vm.Do)`: the delegate of the method `Do` of the view model.
        let markup = <ViewModel as MarkupTyped>::MARKUP;
        let do_ = markup.find_methods("Do").next().expect("the view model declares the method Do");
        let target: BoxedValue = vm.clone();
        let action = MarkupDelegate::for_method(Some(target), markup, do_);
        let command = MethodToCommandConverter::new(&action);
        let _subscription = ICommand::can_execute_changed(&*command, {
            let can_execute_count = can_execute_count.clone();
            Rc::new(move || can_execute_count.set(can_execute_count.get() + 1))
        });
        vm.set_parameter(Some(Rc::new(0_i32)));
        Dispatcher::ui_thread().run_jobs(None);
        vm.set_parameter(None);
        Dispatcher::ui_thread().run_jobs(None);
        assert_eq!(2, can_execute_count.get());
        drop(vm);
        let before_collect = is_alive(&weak_vm);
        (weak_vm, before_collect)
    }

    let (vmref, before_collect) = make_ref();

    let after_collect = is_alive(&vmref);

    assert!(before_collect, "Invalid ViewModel instance, it is already collected.");
    assert!(!after_collect, "ViewModel instance was not collected");
}
