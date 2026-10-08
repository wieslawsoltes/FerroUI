//! Ported from the upstream `Xaml/BindingTests`.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use ferroui_base::data::core::plugins::ObservableValue;
use ferroui_base::data::converters::IValueConverter;
use ferroui_base::data::core::{ValueType, ValueTypes};
use ferroui_base::data::BindingError;
use ferroui_base::utilities::CultureInfo;
use ferroui_base::metadata::MarkupTyped;
use ferroui_base::reactive::IObservable;
use ferroui_base::{ferro_markup_type, BoxedValue, Ref};
use ferroui_controls::{Border, Button, ContentControl, TextBlock, Window};

use crate::support::app::*;
use crate::support::loader::*;
use crate::support::helpers::*;
use crate::support::xaml::{AttachedPropertyOwner, NonControl, TestControl};
use crate::support::TypeModule;
use crate::support_bindings::*;

// --- test types -------------------------------------------------------------

anonymous_object!(AnonymousFoo { Foo: Option<BoxedValue> => foo });
anonymous_object!(AnonymousObservable { Observable: ObservableValue => observable });

/// `new { Foo = value }`.
fn anonymous_foo(foo: Option<BoxedValue>) -> Option<BoxedValue> {
    Some(Rc::new(AnonymousFoo { foo }))
}

/// A value of type `object` that is nothing else (`new object()`).
#[derive(Clone, Copy, Debug, PartialEq)]
struct PlainObject;

pub struct WindowViewModel {
    show_in_taskbar: Cell<bool>,
    greeting1: RefCell<Option<String>>,
    greeting2: RefCell<Option<String>>,
    object: RefCell<Option<BoxedValue>>,
}

crate::test_identity_eq!(WindowViewModel);

impl WindowViewModel {
    pub fn new() -> Rc<Self> {
        Rc::new(Self {
            show_in_taskbar: Cell::new(false),
            greeting1: RefCell::new(Some("Hello".to_string())),
            greeting2: RefCell::new(Some("World".to_string())),
            object: RefCell::new(None),
        })
    }

    pub fn with_object(object: Option<BoxedValue>) -> Rc<Self> {
        let result = Self::new();
        result.set_object(object);
        result
    }

    pub fn show_in_taskbar(&self) -> bool {
        self.show_in_taskbar.get()
    }

    pub fn set_show_in_taskbar(&self, value: bool) {
        self.show_in_taskbar.set(value);
    }

    pub fn greeting1(&self) -> Option<String> {
        self.greeting1.borrow().clone()
    }

    pub fn set_greeting1(&self, value: Option<String>) {
        self.greeting1.replace(value);
    }

    pub fn greeting2(&self) -> Option<String> {
        self.greeting2.borrow().clone()
    }

    pub fn set_greeting2(&self, value: Option<String>) {
        self.greeting2.replace(value);
    }

    pub fn object(&self) -> Option<BoxedValue> {
        self.object.borrow().clone()
    }

    pub fn set_object(&self, value: Option<BoxedValue>) {
        self.object.replace(value);
    }
}

ferro_markup_type!(class WindowViewModel as "BindingTests+WindowViewModel" {
    this: Rc<WindowViewModel>,
    handles: [WindowViewModel, Rc<WindowViewModel>, Option<Rc<WindowViewModel>>],
    constructors: [() => WindowViewModel::new],
    properties: [
        ShowInTaskbar: bool { get: WindowViewModel::show_in_taskbar, set: WindowViewModel::set_show_in_taskbar },
        Greeting1: Option<String> { get: WindowViewModel::greeting1, set: WindowViewModel::set_greeting1 },
        Greeting2: Option<String> { get: WindowViewModel::greeting2, set: WindowViewModel::set_greeting2 },
        Object: Option<BoxedValue> { get: WindowViewModel::object, set: WindowViewModel::set_object },
    ],
});

/// `BindingTests.CultureAppender`: joins the value and the culture with `+`.
pub struct CultureAppender;

crate::test_identity_eq!(CultureAppender);

impl CultureAppender {
    /// `CultureAppender.Instance`.
    pub fn instance() -> Rc<dyn IValueConverter> {
        thread_local! {
            static INSTANCE: Rc<CultureAppender> = Rc::new(CultureAppender);
        }
        INSTANCE.with(|instance| -> Rc<dyn IValueConverter> { instance.clone() })
    }
}

impl IValueConverter for CultureAppender {
    fn convert(
        &self,
        value: Option<&BoxedValue>,
        _target_type: ValueType,
        _parameter: Option<&BoxedValue>,
        culture: &CultureInfo,
    ) -> Result<Option<BoxedValue>, BindingError> {
        Ok(Some(Rc::new(format!("{}+{}", ValueTypes::to_display_string(value), culture))))
    }

    fn convert_back(
        &self,
        _value: Option<&BoxedValue>,
        _target_type: ValueType,
        _parameter: Option<&BoxedValue>,
        _culture: &CultureInfo,
    ) -> Result<Option<BoxedValue>, BindingError> {
        Err(BindingError::message("The method or operation is not implemented."))
    }
}

ferro_markup_type!(class CultureAppender as "BindingTests+CultureAppender" {
    this: Rc<CultureAppender>,
    handles: [CultureAppender, Rc<CultureAppender>, Option<Rc<CultureAppender>>],
    interfaces: [Rc<dyn IValueConverter>],
    fields: [Instance: Rc<dyn IValueConverter> => CultureAppender::instance],
});

/// The test types of this file.
pub(crate) const MODULE: TypeModule = TypeModule {
    types: &[],
    markup_types: &[
        <CultureAppender as MarkupTyped>::MARKUP,
        <AnonymousFoo as MarkupTyped>::MARKUP,
        <AnonymousObservable as MarkupTyped>::MARKUP,
        <WindowViewModel as MarkupTyped>::MARKUP,
    ],
    value_types: || {
        ValueTypes::register_reference::<CultureAppender>();
        ValueTypes::register_reference::<AnonymousFoo>();
        ValueTypes::register_reference::<AnonymousObservable>();
        ValueTypes::register_reference::<WindowViewModel>();
    },
};

// --- tests ------------------------------------------------------------------

#[test]
fn binding_to_data_context_works() {
    let _app = styled_window_application();
    let xaml = r#"
<Window xmlns='https://github.com/ferroui'
        xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'
        xmlns:local='clr-namespace:FerroUI.Markup.Xaml.UnitTests.Xaml;assembly=FerroUI.Markup.Xaml.UnitTests'>
    <Button Name='button' Content='{Binding Foo}'/>
</Window>"#;
    let window: Ref<Window> = load_as(xaml);
    let button = window.get_control::<Button>("button");

    button.set_data_context(anonymous_foo(boxed_str("foo")));
    window.apply_template();

    assert_eq!(string_of(&button.content()).as_deref(), Some("foo"));
}

#[test]
fn longhand_binding_to_data_context_works() {
    let _app = styled_window_application();
    let xaml = r#"
<Window xmlns='https://github.com/ferroui'
        xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'
        xmlns:local='clr-namespace:FerroUI.Markup.Xaml.UnitTests.Xaml;assembly=FerroUI.Markup.Xaml.UnitTests'>
    <Button Name='button'>
        <Button.Content>
            <Binding Path='Foo'/>
        </Button.Content>
    </Button>
</Window>"#;
    let window: Ref<Window> = load_as(xaml);
    let button = window.get_control::<Button>("button");

    button.set_data_context(anonymous_foo(boxed_str("foo")));
    window.apply_template();

    assert_eq!(string_of(&button.content()).as_deref(), Some("foo"));
}

#[test]
fn can_bind_control_to_non_control() {
    let _app = styled_window_application();
    let xaml = r#"
<Window xmlns='https://github.com/ferroui'
        xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'
        xmlns:local='clr-namespace:FerroUI.Markup.Xaml.UnitTests.Xaml;assembly=FerroUI.Markup.Xaml.UnitTests'>
    <Button Name='button' Content='Foo'>
        <Button.Tag>
            <local:NonControl Control='{Binding #button}'/>
        </Button.Tag>
    </Button>
</Window>"#;
    let window: Ref<Window> = load_as(xaml);
    let button = window.get_control::<Button>("button");

    let non_control: Ref<NonControl> = cast(&button.tag().unwrap());
    assert!(non_control.control().is_some_and(|control| control == button.clone().upcast::<ferroui_controls::Control>()));
}

#[test]
fn can_bind_to_data_context_of_anchor_on_non_control() {
    let _app = mock_windowing_platform_application();
    let xaml = r#"
<Window xmlns='https://github.com/ferroui'
        xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'
        xmlns:local='clr-namespace:FerroUI.Markup.Xaml.UnitTests.Xaml;assembly=FerroUI.Markup.Xaml.UnitTests'>
    <Button Name='button'>
        <Button.Tag>
            <local:NonControl String='{Binding Foo}'/>
        </Button.Tag>
    </Button>
</Window>"#;
    let window: Ref<Window> = load_as(xaml);
    let button = window.get_control::<Button>("button");

    button.set_data_context(anonymous_foo(boxed_str("foo")));

    let non_control: Ref<NonControl> = cast(&button.tag().unwrap());
    assert_eq!(non_control.string().as_deref(), Some("foo"));
}

#[test]
fn binding_to_window_works() {
    let _app = styled_window_application();
    let xaml = r#"
<Window xmlns='https://github.com/ferroui'
        xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'
        Title='{Binding Foo}'>
</Window>"#;
    let window: Ref<Window> = load_as(xaml);

    window.set_data_context(anonymous_foo(boxed_str("foo")));
    window.apply_template();

    assert_eq!(window.title().as_deref(), Some("foo"));
}

#[test]
fn binding_data_context_to_inherited_data_context_works() {
    let _app = styled_window_application();
    let xaml = r#"
<Window xmlns='https://github.com/ferroui'
        xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'>
    <Border DataContext='{Binding Foo}'/>
</Window>"#;
    let window: Ref<Window> = load_as(xaml);
    let border = object_of::<Border>(&window.content());

    window.set_data_context(anonymous_foo(boxed_str("foo")));
    window.apply_template();
    window.presenter().unwrap().apply_template();

    assert_eq!(string_of(&border.data_context()).as_deref(), Some("foo"));
}

#[test]
fn binding_to_self_works() {
    let _app = mock_windowing_platform_application();
    let xaml = r#"
<Window xmlns='https://github.com/ferroui'
        xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'>
    <TextBlock Name='textblock' Text='{Binding Tag, RelativeSource={RelativeSource Self}}'/>
</Window>"#;

    let window: Ref<ContentControl> = parse(xaml);
    let text_block = object_of::<TextBlock>(&window.content());

    text_block.set_tag(boxed_str("foo"));

    assert_eq!(text_block.text().as_deref(), Some("foo"));
}

#[test]
fn longform_binding_to_self_works() {
    let _app = mock_windowing_platform_application();
    let xaml = r#"
<Window xmlns='https://github.com/ferroui'
        xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'>
    <TextBlock Name='textblock' Tag='foo'>
        <TextBlock.Text>
            <Binding RelativeSource='{RelativeSource Self}' Path='Tag'/>
        </TextBlock.Text>
    </TextBlock>
</Window>"#;
    let window: Ref<Window> = load_as(xaml);
    let text_block = object_of::<TextBlock>(&window.content());

    window.apply_template();

    assert_eq!(text_block.text().as_deref(), Some("foo"));
}

#[test]
fn binding_to_self_in_style_works() {
    let _app = styled_window_application();
    let xaml = r#"
<Window xmlns='https://github.com/ferroui'
        xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'>

    <Window.Styles>
        <Style Selector='Button'>
            <Setter Property='IsVisible' Value='{Binding $self.IsEnabled}' />
        </Style>
    </Window.Styles>

    <Button Name='button' />
</Window>"#;
    let window: Ref<Window> = load_as(xaml);
    let button = window.get_control::<Button>("button");

    window.apply_template();
    window.presenter().unwrap().apply_template();

    assert!(button.is_visible());

    button.set_is_enabled(false);

    assert!(!button.is_visible());
}

#[test]
fn stream_binding_to_observable_works() {
    let _app = mock_windowing_platform_application();
    let xaml = r#"
<Window xmlns='https://github.com/ferroui'
        xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'>
    <TextBlock Name='textblock' Text='{Binding Observable^}'/>
</Window>"#;
    let window: Ref<Window> = load_as(xaml);
    let text_block = object_of::<TextBlock>(&window.content());
    let observable = BehaviorSubject::new("foo".to_string());

    let source: Rc<dyn IObservable<String>> = observable.clone();
    window.set_data_context(Some(Rc::new(AnonymousObservable { observable: ObservableValue::new(source) })));
    window.apply_template();

    assert_eq!(text_block.text().as_deref(), Some("foo"));
    observable.on_next("bar".to_string());
    assert_eq!(text_block.text().as_deref(), Some("bar"));
}

#[test]
fn binding_to_namespaced_attached_property_works() {
    let _app = mock_windowing_platform_application();
    let xaml = r#"
<Window xmlns='https://github.com/ferroui'
        xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'
        xmlns:local='clr-namespace:FerroUI.Markup.Xaml.UnitTests.Xaml;assembly=FerroUI.Markup.Xaml.UnitTests'>
    <TextBlock local:AttachedPropertyOwner.Double='{Binding}'/>
</Window>"#;
    let window: Ref<Window> = load_as(xaml);
    let text_block = object_of::<TextBlock>(&window.content());

    window.set_data_context(obj(5.6_f64));
    window.apply_template();

    assert_eq!(AttachedPropertyOwner::get_double(&text_block), 5.6);
}

#[test]
fn binding_to_add_ownered_attached_property_works() {
    let _app = mock_windowing_platform_application();
    let xaml = r#"
<Window xmlns='https://github.com/ferroui'
        xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'
        xmlns:local='clr-namespace:FerroUI.Markup.Xaml.UnitTests.Xaml;assembly=FerroUI.Markup.Xaml.UnitTests'>
    <local:TestControl Double='{Binding}'/>
</Window>"#;
    let window: Ref<Window> = load_as(xaml);
    let test_control = object_of::<TestControl>(&window.content());

    window.set_data_context(obj(5.6_f64));
    window.apply_template();

    assert_eq!(test_control.double(), 5.6);
}

#[test]
fn binding_to_attached_property_using_add_ownered_type_works() {
    let _app = mock_windowing_platform_application();
    let xaml = r#"
<Window xmlns='https://github.com/ferroui'
        xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'
        xmlns:local='clr-namespace:FerroUI.Markup.Xaml.UnitTests.Xaml;assembly=FerroUI.Markup.Xaml.UnitTests'>
    <TextBlock local:TestControl.Double='{Binding}'/>
</Window>"#;
    let window: Ref<Window> = load_as(xaml);
    let text_block = object_of::<TextBlock>(&window.content());

    window.set_data_context(obj(5.6_f64));
    window.apply_template();

    assert_eq!(AttachedPropertyOwner::get_double(&text_block), 5.6);
}

#[test]
fn binding_to_attached_property_in_style_works() {
    let _app = styled_window_application();
    let xaml = r#"
<Window xmlns='https://github.com/ferroui'
        xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'
        xmlns:local='clr-namespace:FerroUI.Markup.Xaml.UnitTests.Xaml;assembly=FerroUI.Markup.Xaml.UnitTests'>
    <Window.Styles>
        <Style Selector='TextBlock'>
            <Setter Property='local:TestControl.Double' Value='{Binding}'/>
        </Style>
    </Window.Styles>
    <TextBlock/>
</Window>"#;
    let window: Ref<Window> = load_as(xaml);
    let text_block = object_of::<TextBlock>(&window.content());

    window.set_data_context(obj(5.6_f64));
    window.apply_template();

    assert_eq!(AttachedPropertyOwner::get_double(&text_block), 5.6);
}

#[test]
fn binding_to_text_block_text_with_string_converter_works() {
    for fmt in [r"Hello \{0\}", "'Hello {0}'", "Hello {0}"] {
        let _app = styled_window_application();
        let xaml = format!(
            "{}{}{}",
            r#"
<Window xmlns='https://github.com/ferroui'
        xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'
        xmlns:local='clr-namespace:FerroUI.Markup.Xaml.UnitTests.Xaml;assembly=FerroUI.Markup.Xaml.UnitTests'>
    <TextBlock Name='textBlock' Text="{Binding Foo, StringFormat="#,
            fmt,
            r#"}"/>
</Window>"#
        );
        let window: Ref<Window> = load_as(&xaml);
        let text_block = window.get_control::<TextBlock>("textBlock");

        text_block.set_data_context(anonymous_foo(boxed_str("world")));
        window.apply_template();

        assert_eq!(text_block.text().as_deref(), Some("Hello world"), "StringFormat={fmt}");
    }
}

#[test]
fn multi_binding_to_text_block_text_with_string_converter_works() {
    for fmt in ["{}{0} {1}!", r"\{0\} \{1\}!"] {
        let _app = styled_window_application();
        let xaml = format!(
            "{}{}{}",
            r#"
<Window xmlns='https://github.com/ferroui'
        xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'
        xmlns:local='clr-namespace:FerroUI.Markup.Xaml.UnitTests.Xaml;assembly=FerroUI.Markup.Xaml.UnitTests'>
    <TextBlock Name='textBlock'>
        <TextBlock.Text>
            <MultiBinding StringFormat='"#,
            fmt,
            r#"'>
                <Binding Path='Greeting1'/>
                <Binding Path='Greeting2'/>
            </MultiBinding>
        </TextBlock.Text>
    </TextBlock>
</Window>"#
        );
        let window: Ref<Window> = load_as(&xaml);
        let text_block = window.get_control::<TextBlock>("textBlock");

        text_block.set_data_context(Some(WindowViewModel::new()));
        window.apply_template();

        assert_eq!(text_block.text().as_deref(), Some("Hello World!"), "StringFormat={fmt}");
    }
}

#[test]
fn binding_one_way_to_source_works() {
    let _app = styled_window_application();
    let xaml = r#"
<Window xmlns='https://github.com/ferroui'
        xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'
        ShowInTaskbar='{Binding ShowInTaskbar, Mode=OneWayToSource}'>
</Window>"#;
    let window: Ref<Window> = load_as(xaml);
    let view_model = WindowViewModel::new();

    window.set_data_context(Some(view_model.clone()));
    window.apply_template();

    assert!(window.show_in_taskbar());
    assert!(view_model.show_in_taskbar());
}

/// The rows of the negation theories: the value and its negation (`None`
/// where the value cannot be converted to a boolean).
fn negation_data() -> Vec<(&'static str, Option<BoxedValue>, Option<bool>)> {
    vec![
        ("true", obj(true), Some(false)),
        ("false", obj(false), Some(true)),
        ("null", None, Some(true)),
        ("new object()", obj(PlainObject), None),
        ("\"foo\"", boxed_str("foo"), None),
        ("\"true\"", boxed_str("true"), Some(false)),
        ("\"false\"", boxed_str("false"), Some(true)),
        ("0", obj(0_i32), Some(true)),
        ("1", obj(1_i32), Some(false)),
        ("2", obj(2_i32), Some(false)),
        ("-1", obj(-1_i32), Some(false)),
        ("0.0", obj(0.0_f64), Some(true)),
        ("1.0", obj(1.0_f64), Some(false)),
        ("2.0", obj(2.0_f64), Some(false)),
        ("-1.0", obj(-1.0_f64), Some(false)),
        ("double.NaN", obj(f64::NAN), Some(false)),
        ("double.PositiveInfinity", obj(f64::INFINITY), Some(false)),
        ("double.NegativeInfinity", obj(f64::NEG_INFINITY), Some(false)),
    ]
}

#[test]
fn negating_object_returns_correct_value() {
    for (row, value, expected) in negation_data() {
        let _app = styled_window_application();
        let xaml = r#"
<Window xmlns='https://github.com/ferroui'
        xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'
        Tag='{Binding !Object}'>
</Window>"#;
        let window: Ref<Window> = load_as(xaml);
        let view_model = WindowViewModel::with_object(value);

        window.set_data_context(Some(view_model));
        window.apply_template();

        assert_eq!(window.tag().is_some(), expected.is_some(), "row {row}");
        assert_eq!(value_of::<bool>(&window.tag()), expected, "row {row}");
    }
}

#[test]
fn double_negating_object_returns_correct_value() {
    for (row, value, negated) in negation_data() {
        let _app = styled_window_application();
        let xaml = r#"
<Window xmlns='https://github.com/ferroui'
        xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'
        Tag='{Binding !!Object}'>
</Window>"#;
        let window: Ref<Window> = load_as(xaml);
        let view_model = WindowViewModel::with_object(value);

        window.set_data_context(Some(view_model));
        window.apply_template();

        let expected = negated.map(|negated| !negated);
        assert_eq!(window.tag().is_some(), expected.is_some(), "row {row}");
        assert_eq!(value_of::<bool>(&window.tag()), expected, "row {row}");
    }
}

#[test]
fn negating_object_returns_correct_value_when_bound_to_bool() {
    for (row, value, expected) in negation_data() {
        let _app = styled_window_application();
        let xaml = r#"
<Window xmlns='https://github.com/ferroui'
        xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'
        IsVisible='{Binding !Object}'>
</Window>"#;
        let window: Ref<Window> = load_as(xaml);
        let view_model = WindowViewModel::with_object(value);

        window.set_data_context(Some(view_model));
        window.apply_template();

        assert_eq!(window.is_visible(), expected.unwrap_or(false), "row {row}");
    }
}

#[test]
fn double_negating_object_returns_correct_value_when_bound_to_bool() {
    for (row, value, negated) in negation_data() {
        let _app = styled_window_application();
        let xaml = r#"
<Window xmlns='https://github.com/ferroui'
        xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'
        IsVisible='{Binding !!Object}'>
</Window>"#;
        let window: Ref<Window> = load_as(xaml);
        let view_model = WindowViewModel::with_object(value);

        window.set_data_context(Some(view_model));
        window.apply_template();

        let expected = negated.map(|negated| !negated).unwrap_or(false);
        assert_eq!(window.is_visible(), expected, "row {row}");
    }
}

#[test]
fn converter_culture_can_be_specified_by_ietf_language_tag() {
    let _app = styled_window_application();
    let xaml = r#"
<Window xmlns='https://github.com/ferroui'
        xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'
        xmlns:local='clr-namespace:FerroUI.Markup.Xaml.UnitTests.Xaml;assembly=FerroUI.Markup.Xaml.UnitTests'>
  <TextBlock Name='textBlock' Text='{Binding Greeting1, Converter={x:Static local:BindingTests+CultureAppender.Instance}, ConverterCulture=ar-SA}'/>
</Window>"#;
    let window: Ref<Window> = load_as(xaml);
    let text_block = object_of::<TextBlock>(&window.content());

    window.set_data_context(Some(WindowViewModel::new()));
    window.apply_template();

    assert_eq!(Some("Hello+ar-SA"), text_block.text().as_deref());
}

#[test]
fn binding_classes_works() {
    let _app = styled_window_application();
    // Note, this test also checks `Classes` reordering, so it should be kept AFTER the last single class
    let xaml = r#"
<Window xmlns='https://github.com/ferroui'
        xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'
        xmlns:local='clr-namespace:FerroUI.Markup.Xaml.UnitTests.Xaml;assembly=FerroUI.Markup.Xaml.UnitTests'>
    <Button Name='button' Classes.MyClass='{Binding Foo}' Classes.MySecondClass='True' Classes='foo bar'/>
</Window>"#;
    let window: Ref<Window> = load_as(xaml);
    let button = window.get_control::<Button>("button");

    button.set_data_context(anonymous_foo(obj(true)));
    window.apply_template();

    assert!(button.classes().contains("MyClass"));
    assert!(button.classes().contains("MySecondClass"));
    assert!(button.classes().contains("foo"));
    assert!(button.classes().contains("bar"));

    button.set_data_context(anonymous_foo(obj(false)));

    assert!(!button.classes().contains("MyClass"));
    assert!(button.classes().contains("MySecondClass"));
    assert!(button.classes().contains("foo"));
    assert!(button.classes().contains("bar"));
}
