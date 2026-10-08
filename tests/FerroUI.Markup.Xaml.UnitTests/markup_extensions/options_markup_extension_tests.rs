//! Ported from the upstream `MarkupExtensions/OptionsMarkupExtensionTests`.

use std::cell::{Cell, RefCell};
use std::fmt;
use std::rc::Rc;

use ferroui_base::controls::{ResourceDictionary, ResourceKey};
use ferroui_base::data::core::ValueTypes;
use ferroui_base::media::{Color, Colors};
use ferroui_base::metadata::{IAddChild, IServiceProvider, MarkupTyped};
use ferroui_base::{ferro_markup_type, BoxedValue, Ref, Thickness, Visual};
use ferroui_controls::{Border, Button, ContentControl, Panel, TextBlock, UserControl};
use ferroui_markup_xaml::markup_extensions::On;

use crate::support::app::*;
use crate::support::helpers::*;
use crate::support::loader::*;
use crate::support::TypeModule;

// --- test globals -----------------------------------------------------------

// The static fields `RaisedOption` and `ObjectsCreated` of the test class.
// Every test runs on its own thread, so the fields are per thread.
thread_local! {
    static RAISED_OPTION: RefCell<Option<Rc<dyn Fn(&BoxedValue) -> bool>>> = const { RefCell::new(None) };
    static OBJECTS_CREATED: Cell<Option<i32>> = const { Cell::new(None) };
}

/// The failure of a member that dereferences null (`RaisedOption!(..)` while
/// no test set it, `throw null!`).
#[derive(Debug)]
pub struct NullReferenceException;

impl fmt::Display for NullReferenceException {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Object reference not set to an instance of an object.")
    }
}

/// `OptionsMarkupExtensionTests.RaisedOption!(option)`.
fn raised_option(option: BoxedValue) -> Result<bool, NullReferenceException> {
    let raised_option = RAISED_OPTION.with(|raised_option| raised_option.borrow().clone());
    match raised_option {
        Some(raised_option) => Ok(raised_option(&option)),
        None => Err(NullReferenceException),
    }
}

/// `OptionsMarkupExtensionTests.ObjectsCreated`.
fn objects_created() -> Option<i32> {
    OBJECTS_CREATED.with(Cell::get)
}

/// The scope of [`setup_test_globals`]; dropping it resets the globals.
struct TestGlobals;

impl Drop for TestGlobals {
    fn drop(&mut self) {
        RAISED_OPTION.with(|raised_option| raised_option.replace(None));
        OBJECTS_CREATED.with(|objects_created| objects_created.set(None));
    }
}

/// `SetupTestGlobals(acceptedOption)`.
fn setup_test_globals(accepted_option: BoxedValue) -> TestGlobals {
    RAISED_OPTION.with(|raised_option| {
        raised_option.replace(Some(Rc::new(move |o: &BoxedValue| **o == *accepted_option)));
    });
    OBJECTS_CREATED.with(|objects_created| objects_created.set(Some(0)));
    TestGlobals
}

// --- test types -------------------------------------------------------------

/// `OptionsMarkupExtensionBase<TReturn, TOn>`, and with it the classes
/// derived from it: `OptionsMarkupExtension` is the instantiation for any
/// value ([`OptionsMarkupExtension`]), `OptionsMarkupExtension<TReturn>` the
/// typed one. No test names an instantiation of the typed class through
/// `x:TypeArguments`, so none is published to markup.
pub struct OptionsMarkupExtensionBase<TReturn> {
    option_a: RefCell<Option<TReturn>>,
    option_b: RefCell<Option<TReturn>>,
    option_number: RefCell<Option<TReturn>>,
    default: RefCell<Option<TReturn>>,
}

/// `OptionsMarkupExtension : OptionsMarkupExtensionBase<object, On>`.
pub type OptionsMarkupExtension = OptionsMarkupExtensionBase<BoxedValue>;

impl<TReturn> PartialEq for OptionsMarkupExtensionBase<TReturn> {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl<TReturn: Clone> OptionsMarkupExtensionBase<TReturn> {
    pub fn new() -> Rc<Self> {
        Rc::new(Self {
            option_a: RefCell::new(None),
            option_b: RefCell::new(None),
            option_number: RefCell::new(None),
            default: RefCell::new(None),
        })
    }

    pub fn with_default(default_value: Option<TReturn>) -> Rc<Self> {
        let this = Self::new();
        this.set_default(default_value);
        this
    }

    pub fn option_a(&self) -> Option<TReturn> {
        self.option_a.borrow().clone()
    }

    pub fn set_option_a(&self, value: Option<TReturn>) {
        self.option_a.replace(value);
    }

    pub fn option_b(&self) -> Option<TReturn> {
        self.option_b.borrow().clone()
    }

    pub fn set_option_b(&self, value: Option<TReturn>) {
        self.option_b.replace(value);
    }

    pub fn option_number(&self) -> Option<TReturn> {
        self.option_number.borrow().clone()
    }

    pub fn set_option_number(&self, value: Option<TReturn>) {
        self.option_number.replace(value);
    }

    pub fn default(&self) -> Option<TReturn> {
        self.default.borrow().clone()
    }

    pub fn set_default(&self, value: Option<TReturn>) {
        self.default.replace(value);
    }

    pub fn should_provide_option(
        &self,
        _service_provider: Rc<dyn IServiceProvider>,
        option: String,
    ) -> Result<bool, NullReferenceException> {
        raised_option(Rc::new(option))
    }

    pub fn provide_value(&self, _service_provider: Rc<dyn IServiceProvider>) -> Result<Option<TReturn>, NullReferenceException> {
        Err(NullReferenceException)
    }
}

impl<TReturn> IAddChild<Rc<On<TReturn>>> for OptionsMarkupExtensionBase<TReturn> {
    fn add_child(&self, _child: Rc<On<TReturn>>) {}
}

ferro_markup_type!(class OptionsMarkupExtension as "OptionsMarkupExtension" {
    this: Rc<OptionsMarkupExtension>,
    handles: [OptionsMarkupExtension, Rc<OptionsMarkupExtension>, Option<Rc<OptionsMarkupExtension>>],
    interfaces: [Rc<dyn IAddChild<Rc<On>>>],
    constructors: [
        () => OptionsMarkupExtension::new,
        (Option<BoxedValue>) => OptionsMarkupExtension::with_default,
    ],
    content: Default,
    properties: [
        OptionA: Option<BoxedValue> { get: OptionsMarkupExtension::option_a, set: OptionsMarkupExtension::set_option_a }
            [MarkupExtensionOption("option 1")],
        OptionB: Option<BoxedValue> { get: OptionsMarkupExtension::option_b, set: OptionsMarkupExtension::set_option_b }
            [MarkupExtensionOption("option 2")],
        OptionNumber: Option<BoxedValue> {
            get: OptionsMarkupExtension::option_number,
            set: OptionsMarkupExtension::set_option_number
        } [MarkupExtensionOption(3)],
        Default: Option<BoxedValue> { get: OptionsMarkupExtension::default, set: OptionsMarkupExtension::set_default }
            [MarkupExtensionDefaultOption],
    ],
    methods: [
        try fn ShouldProvideOption(Rc<dyn IServiceProvider>, String) -> bool => OptionsMarkupExtension::should_provide_option,
        try fn ProvideValue(Rc<dyn IServiceProvider>) -> Option<BoxedValue> => OptionsMarkupExtension::provide_value,
        fn AddChild(Rc<On>) => |this: &Rc<OptionsMarkupExtension>, child: Rc<On>| IAddChild::add_child(&**this, child),
    ],
});

pub struct OptionsMarkupExtensionNoServiceProvider {
    option_a: RefCell<Option<BoxedValue>>,
    option_b: RefCell<Option<BoxedValue>>,
    default: RefCell<Option<BoxedValue>>,
}

crate::test_identity_eq!(OptionsMarkupExtensionNoServiceProvider);

impl OptionsMarkupExtensionNoServiceProvider {
    pub fn new() -> Rc<Self> {
        Rc::new(Self { option_a: RefCell::new(None), option_b: RefCell::new(None), default: RefCell::new(None) })
    }

    pub fn option_a(&self) -> Option<BoxedValue> {
        self.option_a.borrow().clone()
    }

    pub fn set_option_a(&self, value: Option<BoxedValue>) {
        self.option_a.replace(value);
    }

    pub fn option_b(&self) -> Option<BoxedValue> {
        self.option_b.borrow().clone()
    }

    pub fn set_option_b(&self, value: Option<BoxedValue>) {
        self.option_b.replace(value);
    }

    pub fn default(&self) -> Option<BoxedValue> {
        self.default.borrow().clone()
    }

    pub fn set_default(&self, value: Option<BoxedValue>) {
        self.default.replace(value);
    }

    pub fn should_provide_option(option: i32) -> Result<bool, NullReferenceException> {
        raised_option(Rc::new(option))
    }

    pub fn provide_value(&self, _service_provider: Rc<dyn IServiceProvider>) -> Result<Option<BoxedValue>, NullReferenceException> {
        Err(NullReferenceException)
    }
}

ferro_markup_type!(class OptionsMarkupExtensionNoServiceProvider {
    this: Rc<OptionsMarkupExtensionNoServiceProvider>,
    handles: [
        OptionsMarkupExtensionNoServiceProvider,
        Rc<OptionsMarkupExtensionNoServiceProvider>,
        Option<Rc<OptionsMarkupExtensionNoServiceProvider>>,
    ],
    constructors: [() => OptionsMarkupExtensionNoServiceProvider::new],
    content: Default,
    properties: [
        OptionA: Option<BoxedValue> {
            get: OptionsMarkupExtensionNoServiceProvider::option_a,
            set: OptionsMarkupExtensionNoServiceProvider::set_option_a
        } [MarkupExtensionOption(1)],
        OptionB: Option<BoxedValue> {
            get: OptionsMarkupExtensionNoServiceProvider::option_b,
            set: OptionsMarkupExtensionNoServiceProvider::set_option_b
        } [MarkupExtensionOption(2)],
        Default: Option<BoxedValue> {
            get: OptionsMarkupExtensionNoServiceProvider::default,
            set: OptionsMarkupExtensionNoServiceProvider::set_default
        } [MarkupExtensionDefaultOption],
    ],
    methods: [
        static try fn ShouldProvideOption(i32) -> bool => OptionsMarkupExtensionNoServiceProvider::should_provide_option,
        try fn ProvideValue(Rc<dyn IServiceProvider>) -> Option<BoxedValue> =>
            OptionsMarkupExtensionNoServiceProvider::provide_value,
    ],
});

pub struct OptionsMarkupExtensionMinimal {
    option_a: Cell<bool>,
}

crate::test_identity_eq!(OptionsMarkupExtensionMinimal);

impl OptionsMarkupExtensionMinimal {
    pub fn new() -> Rc<Self> {
        Rc::new(Self { option_a: Cell::new(false) })
    }

    pub fn option_a(&self) -> bool {
        self.option_a.get()
    }

    pub fn set_option_a(&self, value: bool) {
        self.option_a.set(value);
    }

    pub fn should_provide_option(option: f64) -> bool {
        option > 0.0
    }

    pub fn provide_value(&self) -> Result<Option<BoxedValue>, NullReferenceException> {
        Err(NullReferenceException)
    }
}

// The option of `OptionA` is the floating-point number 11.0 upstream.
// Attribute arguments of markup metadata have no floating-point form, so it
// is declared as the integer 11: the compiler reads an option through its
// text, which is "11" for both.
ferro_markup_type!(class OptionsMarkupExtensionMinimal {
    this: Rc<OptionsMarkupExtensionMinimal>,
    handles: [OptionsMarkupExtensionMinimal, Rc<OptionsMarkupExtensionMinimal>, Option<Rc<OptionsMarkupExtensionMinimal>>],
    constructors: [() => OptionsMarkupExtensionMinimal::new],
    properties: [
        OptionA: bool { get: OptionsMarkupExtensionMinimal::option_a, set: OptionsMarkupExtensionMinimal::set_option_a }
            [MarkupExtensionOption(11)],
    ],
    methods: [
        static fn ShouldProvideOption(f64) -> bool => OptionsMarkupExtensionMinimal::should_provide_option,
        try fn ProvideValue() -> Option<BoxedValue> => OptionsMarkupExtensionMinimal::provide_value,
    ],
});

pub struct OptionsMarkupExtensionWithProperty {
    option_a: Cell<bool>,
    property: Cell<i32>,
}

crate::test_identity_eq!(OptionsMarkupExtensionWithProperty);

impl OptionsMarkupExtensionWithProperty {
    pub fn new() -> Rc<Self> {
        Rc::new(Self { option_a: Cell::new(false), property: Cell::new(0) })
    }

    pub fn option_a(&self) -> bool {
        self.option_a.get()
    }

    pub fn set_option_a(&self, value: bool) {
        self.option_a.set(value);
    }

    pub fn property(&self) -> i32 {
        self.property.get()
    }

    pub fn set_property(&self, value: i32) {
        self.property.set(value);
    }

    pub fn should_provide_option(&self, option: i32) -> bool {
        option == self.property()
    }

    pub fn provide_value(&self) -> Result<Option<BoxedValue>, NullReferenceException> {
        Err(NullReferenceException)
    }
}

ferro_markup_type!(class OptionsMarkupExtensionWithProperty {
    this: Rc<OptionsMarkupExtensionWithProperty>,
    handles: [
        OptionsMarkupExtensionWithProperty,
        Rc<OptionsMarkupExtensionWithProperty>,
        Option<Rc<OptionsMarkupExtensionWithProperty>>,
    ],
    constructors: [() => OptionsMarkupExtensionWithProperty::new],
    properties: [
        OptionA: bool {
            get: OptionsMarkupExtensionWithProperty::option_a,
            set: OptionsMarkupExtensionWithProperty::set_option_a
        } [MarkupExtensionOption(5)],
        Property: i32 {
            get: OptionsMarkupExtensionWithProperty::property,
            set: OptionsMarkupExtensionWithProperty::set_property
        },
    ],
    methods: [
        fn ShouldProvideOption(i32) -> bool => OptionsMarkupExtensionWithProperty::should_provide_option,
        try fn ProvideValue() -> Option<BoxedValue> => OptionsMarkupExtensionWithProperty::provide_value,
    ],
});

/// `OptionsMarkupExtensionWithGeneric<TResult>`. The tests name the
/// instantiation for `Thickness`, which is the one published to markup.
pub struct OptionsMarkupExtensionWithGeneric<TResult> {
    option_a: RefCell<TResult>,
    default: RefCell<TResult>,
}

impl<TResult> PartialEq for OptionsMarkupExtensionWithGeneric<TResult> {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl<TResult: Clone + Default> OptionsMarkupExtensionWithGeneric<TResult> {
    pub fn new() -> Rc<Self> {
        Rc::new(Self { option_a: RefCell::new(TResult::default()), default: RefCell::new(TResult::default()) })
    }

    pub fn option_a(&self) -> TResult {
        self.option_a.borrow().clone()
    }

    pub fn set_option_a(&self, value: TResult) {
        self.option_a.replace(value);
    }

    pub fn default(&self) -> TResult {
        self.default.borrow().clone()
    }

    pub fn set_default(&self, value: TResult) {
        self.default.replace(value);
    }

    pub fn should_provide_option(&self, option: String) -> Result<bool, NullReferenceException> {
        raised_option(Rc::new(option))
    }

    pub fn provide_value(&self) -> Result<TResult, NullReferenceException> {
        Err(NullReferenceException)
    }
}

ferro_markup_type!(class OptionsMarkupExtensionWithGeneric<Thickness> as "OptionsMarkupExtensionWithGeneric`1" {
    this: Rc<OptionsMarkupExtensionWithGeneric<Thickness>>,
    handles: [
        OptionsMarkupExtensionWithGeneric<Thickness>,
        Rc<OptionsMarkupExtensionWithGeneric<Thickness>>,
        Option<Rc<OptionsMarkupExtensionWithGeneric<Thickness>>>,
    ],
    generic: "OptionsMarkupExtensionWithGeneric`1" [Thickness],
    constructors: [() => OptionsMarkupExtensionWithGeneric::<Thickness>::new],
    content: Default,
    properties: [
        OptionA: Thickness {
            get: OptionsMarkupExtensionWithGeneric::<Thickness>::option_a,
            set: OptionsMarkupExtensionWithGeneric::<Thickness>::set_option_a
        } [MarkupExtensionOption("option 1")],
        Default: Thickness {
            get: OptionsMarkupExtensionWithGeneric::<Thickness>::default,
            set: OptionsMarkupExtensionWithGeneric::<Thickness>::set_default
        } [MarkupExtensionDefaultOption],
    ],
    methods: [
        try fn ShouldProvideOption(String) -> bool => OptionsMarkupExtensionWithGeneric::<Thickness>::should_provide_option,
        try fn ProvideValue() -> Thickness => OptionsMarkupExtensionWithGeneric::<Thickness>::provide_value,
    ],
});

pub struct ChildObject {
    name: RefCell<Option<String>>,
}

crate::test_identity_eq!(ChildObject);

impl ChildObject {
    pub fn new() -> Rc<Self> {
        OBJECTS_CREATED.with(|objects_created| objects_created.set(objects_created.get().map(|count| count + 1)));
        Rc::new(Self { name: RefCell::new(None) })
    }

    pub fn name(&self) -> Option<String> {
        self.name.borrow().clone()
    }

    pub fn set_name(&self, value: Option<String>) {
        self.name.replace(value);
    }
}

ferro_markup_type!(class ChildObject {
    this: Rc<ChildObject>,
    handles: [ChildObject, Rc<ChildObject>, Option<Rc<ChildObject>>],
    constructors: [() => ChildObject::new],
    properties: [
        Name: Option<String> { get: ChildObject::name, set: ChildObject::set_name },
    ],
});

/// The test types of this file.
pub(crate) const MODULE: TypeModule = TypeModule {
    types: &[],
    markup_types: &[
        <OptionsMarkupExtension as MarkupTyped>::MARKUP,
        <OptionsMarkupExtensionNoServiceProvider as MarkupTyped>::MARKUP,
        <OptionsMarkupExtensionMinimal as MarkupTyped>::MARKUP,
        <OptionsMarkupExtensionWithProperty as MarkupTyped>::MARKUP,
        <OptionsMarkupExtensionWithGeneric<Thickness> as MarkupTyped>::MARKUP,
        <ChildObject as MarkupTyped>::MARKUP,
    ],
    value_types: || {
        ValueTypes::register_reference::<OptionsMarkupExtension>();
        ValueTypes::register_reference::<OptionsMarkupExtensionNoServiceProvider>();
        ValueTypes::register_reference::<OptionsMarkupExtensionMinimal>();
        ValueTypes::register_reference::<OptionsMarkupExtensionWithProperty>();
        ValueTypes::register_reference::<OptionsMarkupExtensionWithGeneric<Thickness>>();
        ValueTypes::register_reference::<ChildObject>();
    },
};

// --- tests ------------------------------------------------------------------

#[test]
fn resolve_default_value() {
    let _base = xaml_test_base();
    let _globals = setup_test_globals(boxed("default".to_string()));

    let xaml = r#"
<TextBlock xmlns='https://github.com/ferroui'
           xmlns:local='clr-namespace:FerroUI.Markup.Xaml.UnitTests.MarkupExtensions;assembly=FerroUI.Markup.Xaml.UnitTests'
           xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'
           Text='{local:OptionsMarkupExtension Default="Hello World"}' />"#;

    let text_block: Ref<TextBlock> = load_as(xaml);

    assert_eq!(text_block.text().as_deref(), Some("Hello World"));
}

#[test]
fn resolve_default_value_from_ctor() {
    let _base = xaml_test_base();
    let _globals = setup_test_globals(boxed("default".to_string()));

    let xaml = r#"
<TextBlock xmlns='https://github.com/ferroui'
           xmlns:local='clr-namespace:FerroUI.Markup.Xaml.UnitTests.MarkupExtensions;assembly=FerroUI.Markup.Xaml.UnitTests'
           xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'
           Text='{local:OptionsMarkupExtension "Hello World", OptionB="Im Android"}' />"#;

    let text_block: Ref<TextBlock> = load_as(xaml);

    assert_eq!(text_block.text().as_deref(), Some("Hello World"));
}

#[test]
fn resolve_implicit_default_value_ref_type() {
    let _base = xaml_test_base();
    let _globals = setup_test_globals(boxed("default".to_string()));

    let xaml = r#"
<UserControl xmlns='https://github.com/ferroui'
             xmlns:local='clr-namespace:FerroUI.Markup.Xaml.UnitTests.MarkupExtensions;assembly=FerroUI.Markup.Xaml.UnitTests'
             xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'
             Tag='{local:OptionsMarkupExtension OptionA="Hello World", x:DataType=x:String}' />"#;

    let user_control: Ref<UserControl> = load_as(xaml);

    assert!(user_control.tag().is_none());
}

#[test]
fn resolve_implicit_default_value_val_type() {
    let _base = xaml_test_base();
    let _globals = setup_test_globals(boxed("default".to_string()));

    let xaml = r#"
<UserControl xmlns='https://github.com/ferroui'
             xmlns:local='clr-namespace:FerroUI.Markup.Xaml.UnitTests.MarkupExtensions;assembly=FerroUI.Markup.Xaml.UnitTests'
             xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'
             Height='{local:OptionsMarkupExtension OptionA=10}' />"#;

    let user_control: Ref<UserControl> = load_as(xaml);

    assert_eq!(0.0, user_control.height());
}

#[test]
fn resolve_implicit_default_value_ferro_val_type() {
    let _base = xaml_test_base();
    let _globals = setup_test_globals(boxed("default".to_string()));

    let xaml = r#"
<UserControl xmlns='https://github.com/ferroui'
             xmlns:local='clr-namespace:FerroUI.Markup.Xaml.UnitTests.MarkupExtensions;assembly=FerroUI.Markup.Xaml.UnitTests'
             xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'
             Margin='{local:OptionsMarkupExtension OptionA=10}' />"#;

    let user_control: Ref<UserControl> = load_as(xaml);

    assert_eq!(Thickness::uniform(0.0), user_control.margin());
}

#[test]
fn resolve_expected_value_per_option() {
    for (option, expected_result) in [
        ("option 1", "Im Option 1"),
        ("option 2", "Im Option 2"),
        ("3", "Im Option 3"),
        ("unknown", "Default value"),
    ] {
        let _base = xaml_test_base();
        let _globals = setup_test_globals(boxed(option.to_string()));

        let xaml = r#"
<TextBlock xmlns='https://github.com/ferroui'
           xmlns:local='clr-namespace:FerroUI.Markup.Xaml.UnitTests.MarkupExtensions;assembly=FerroUI.Markup.Xaml.UnitTests'
           xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'
           Text='{local:OptionsMarkupExtension "Default value",
                OptionA="Im Option 1", OptionB="Im Option 2",
                OptionNumber="Im Option 3"}' />"#;

        let text_block: Ref<TextBlock> = load_as(xaml);

        assert_eq!(text_block.text().as_deref(), Some(expected_result), "row ({option:?}, {expected_result:?})");
    }
}

#[test]
fn resolve_expected_value_per_option_create_single_object() {
    for (option, expected_result) in [
        ("option 1", "Im Option 1"),
        ("option 2", "Im Option 2"),
        ("3", "Im Option 3"),
        ("unknown", "Default value"),
    ] {
        let _base = xaml_test_base();
        let _globals = setup_test_globals(boxed(option.to_string()));

        let xaml = r#"
<ContentControl xmlns='https://github.com/ferroui'
           xmlns:local='clr-namespace:FerroUI.Markup.Xaml.UnitTests.MarkupExtensions;assembly=FerroUI.Markup.Xaml.UnitTests'
           xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'>
    <ContentControl.Content>
        <local:OptionsMarkupExtension>
            <local:OptionsMarkupExtension.Default>
                <local:ChildObject Name="Default value" />
            </local:OptionsMarkupExtension.Default>
            <local:OptionsMarkupExtension.OptionA>
                <local:ChildObject Name="Im Option 1" />
            </local:OptionsMarkupExtension.OptionA>
            <local:OptionsMarkupExtension.OptionB>
                <local:ChildObject Name="Im Option 2" />
            </local:OptionsMarkupExtension.OptionB>
            <local:OptionsMarkupExtension.OptionNumber>
                <local:ChildObject Name="Im Option 3" />
            </local:OptionsMarkupExtension.OptionNumber>
        </local:OptionsMarkupExtension>
    </ContentControl.Content>
</ContentControl>"#;

        let content_control: Ref<ContentControl> = load_as(xaml);
        let content = content_control.content();
        assert!(is_type::<ChildObject>(&content), "row ({option:?}, {expected_result:?}): the content is not a ChildObject");
        let obj = value_of::<Rc<ChildObject>>(&content).expect("the content is a ChildObject");

        assert_eq!(obj.name().as_deref(), Some(expected_result), "row ({option:?}, {expected_result:?})");
        assert_eq!(Some(1), objects_created(), "row ({option:?}, {expected_result:?})");
    }
}

#[test]
fn convert_bcl_type() {
    let _base = xaml_test_base();
    let _globals = setup_test_globals(boxed("option 1".to_string()));

    let xaml = r#"
<Border xmlns='https://github.com/ferroui'
        xmlns:local='clr-namespace:FerroUI.Markup.Xaml.UnitTests.MarkupExtensions;assembly=FerroUI.Markup.Xaml.UnitTests'
        xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'
        Height='{local:OptionsMarkupExtension OptionA=50.1}' />"#;

    let border: Ref<Border> = load_as(xaml);

    assert_eq!(50.1, border.height());
}

#[test]
fn convert_ferro_type() {
    let _base = xaml_test_base();
    let _globals = setup_test_globals(boxed("option 1".to_string()));

    let xaml = r#"
<Border xmlns='https://github.com/ferroui'
        xmlns:local='clr-namespace:FerroUI.Markup.Xaml.UnitTests.MarkupExtensions;assembly=FerroUI.Markup.Xaml.UnitTests'
        xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'
        Padding='{local:OptionsMarkupExtension OptionA="10, 8, 10, 8"}' />"#;

    let border: Ref<Border> = load_as(xaml);

    assert_eq!(Thickness::new(10.0, 8.0, 10.0, 8.0), border.padding());
}

// Upstream runs this test on Windows and Linux only (its run-time code emitter fails for
// type arguments on macOS); the reason does not apply here, so it runs everywhere.
#[test]
fn respect_custom_type_argument() {
    let _base = xaml_test_base();
    let _globals = setup_test_globals(boxed("option 1".to_string()));

    let xaml = r#"
<TextBlock xmlns='https://github.com/ferroui'
           xmlns:local='clr-namespace:FerroUI.Markup.Xaml.UnitTests.MarkupExtensions;assembly=FerroUI.Markup.Xaml.UnitTests'
           xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'
           Tag='{local:OptionsMarkupExtensionWithGeneric Default=20, OptionA="10, 10, 10, 10", x:TypeArguments=Thickness}' />"#;

    let text_block: Ref<TextBlock> = load_as(xaml);

    assert_value(Thickness::new(10.0, 10.0, 10.0, 10.0), &text_block.tag());
}

#[test]
fn allow_nester_markup_extensions() {
    let _base = xaml_test_base();
    let _globals = setup_test_globals(boxed("option 1".to_string()));

    let xaml = r#"
<UserControl xmlns='https://github.com/ferroui'
             xmlns:local='clr-namespace:FerroUI.Markup.Xaml.UnitTests.MarkupExtensions;assembly=FerroUI.Markup.Xaml.UnitTests'
             xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'>
    <UserControl.Resources>
        <SolidColorBrush x:Key='brush'>#ff506070</SolidColorBrush>
    </UserControl.Resources>
    <Border Background='{local:OptionsMarkupExtension OptionA={StaticResource brush}}'/>
</UserControl>"#;

    let user_control: Ref<UserControl> = load_as(xaml);
    let border = object_of::<Border>(&user_control.content());

    let background = border.background().expect("the background is set");
    let brush = background.as_solid_color_brush().expect("the background is an ISolidColorBrush");
    assert_eq!(Color::parse("#ff506070").unwrap(), brush.color());
}

#[test]
fn allow_nester_on_platform_markup_extensions() {
    let _base = xaml_test_base();
    let _globals = setup_test_globals(boxed("option 1".to_string()));

    let xaml = r#"
<Border xmlns='https://github.com/ferroui'
        xmlns:local='clr-namespace:FerroUI.Markup.Xaml.UnitTests.MarkupExtensions;assembly=FerroUI.Markup.Xaml.UnitTests'
        xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'
        Margin='{local:OptionsMarkupExtension OptionA={local:OptionsMarkupExtensionMinimal OptionA="10,10,10,10"}}' />"#;

    let border: Ref<Border> = load_as(xaml);

    assert_eq!(Thickness::uniform(10.0), border.margin());
}

#[test]
fn support_xml_syntax() {
    let _base = xaml_test_base();
    let _globals = setup_test_globals(boxed("option 1".to_string()));

    let xaml = r#"
<Border xmlns='https://github.com/ferroui'
             xmlns:local='clr-namespace:FerroUI.Markup.Xaml.UnitTests.MarkupExtensions;assembly=FerroUI.Markup.Xaml.UnitTests'
             xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'>
    <Border.Background>
        <local:OptionsMarkupExtension>
            <local:OptionsMarkupExtension.OptionA>
                <SolidColorBrush Color='#ff506070' />
            </local:OptionsMarkupExtension.OptionA>
        </local:OptionsMarkupExtension>
    </Border.Background>
</Border>"#;

    let border: Ref<Border> = load_as(xaml);

    let background = border.background().expect("the background is set");
    let brush = background.as_solid_color_brush().expect("the background is an ISolidColorBrush");
    assert_eq!(Color::parse("#ff506070").unwrap(), brush.color());
}

// Upstream runs this test on Windows and Linux only (its run-time code emitter fails for
// type arguments on macOS); the reason does not apply here, so it runs everywhere.
#[test]
fn support_xml_syntax_with_custom_type_arguments() {
    let _base = xaml_test_base();
    let _globals = setup_test_globals(boxed("option 1".to_string()));

    let xaml = r#"
<Border xmlns='https://github.com/ferroui'
        xmlns:local='clr-namespace:FerroUI.Markup.Xaml.UnitTests.MarkupExtensions;assembly=FerroUI.Markup.Xaml.UnitTests'
        xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'>
    <Border.Tag>
        <local:OptionsMarkupExtensionWithGeneric x:TypeArguments='Thickness' OptionA='10, 10, 10, 10' Default='20' />
    </Border.Tag>
</Border>"#;

    let border: Ref<Border> = load_as(xaml);

    assert_value(Thickness::new(10.0, 10.0, 10.0, 10.0), &border.tag());
}

#[test]
fn support_special_on_syntax() {
    for (option, color) in [("option 1", "#ff506070"), ("3", "#000")] {
        let _base = xaml_test_base();
        let _globals = setup_test_globals(boxed(option.to_string()));

        let xaml = r#"
<Border xmlns='https://github.com/ferroui'
        xmlns:local='clr-namespace:FerroUI.Markup.Xaml.UnitTests.MarkupExtensions;assembly=FerroUI.Markup.Xaml.UnitTests'
        xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'>
    <Border.Background>
        <local:OptionsMarkupExtension>
            <On Options='OptionA, OptionB'>
                <SolidColorBrush Color='#ff506070' />
            </On>
            <On Options=' OptionNumber '>
                <SolidColorBrush Color='#000' />
            </On>
        </local:OptionsMarkupExtension>
    </Border.Background>
</Border>"#;

        let border: Ref<Border> = load_as(xaml);

        let background = border.background().expect("the background is set");
        let brush = background.as_solid_color_brush().expect("the background is an ISolidColorBrush");
        assert_eq!(Color::parse(color).unwrap(), brush.color(), "row ({option:?}, {color:?})");
    }
}

#[test]
fn support_control_inside_xml_syntax() {
    let _base = xaml_test_base();
    let _globals = setup_test_globals(boxed("option 1".to_string()));

    let xaml = r#"
<UserControl xmlns='https://github.com/ferroui'
             xmlns:local='clr-namespace:FerroUI.Markup.Xaml.UnitTests.MarkupExtensions;assembly=FerroUI.Markup.Xaml.UnitTests'
             xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'>
    <local:OptionsMarkupExtension>
        <local:OptionsMarkupExtension.OptionA>
            <Button Content='Hello World' />
        </local:OptionsMarkupExtension.OptionA>
    </local:OptionsMarkupExtension>
</UserControl>"#;

    let user_control: Ref<UserControl> = load_as(xaml);
    let button = object_of::<Button>(&user_control.content());

    assert_string("Hello World", &button.content());
}

#[test]
fn support_default_control_inside_xml_syntax() {
    let _base = xaml_test_base();
    let _globals = setup_test_globals(boxed("unknown".to_string()));

    let xaml = r#"
<UserControl xmlns='https://github.com/ferroui'
             xmlns:local='clr-namespace:FerroUI.Markup.Xaml.UnitTests.MarkupExtensions;assembly=FerroUI.Markup.Xaml.UnitTests'
             xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'>
    <local:OptionsMarkupExtension>
        <local:OptionsMarkupExtension.Default>
            <Button Content='Hello World' />
        </local:OptionsMarkupExtension.Default>
    </local:OptionsMarkupExtension>
</UserControl>"#;

    let user_control: Ref<UserControl> = load_as(xaml);
    let button = object_of::<Button>(&user_control.content());

    assert_string("Hello World", &button.content());
}

#[test]
fn support_complex_property_setters_dictionary() {
    let _base = xaml_test_base();
    let _globals = setup_test_globals(boxed("option 1".to_string()));

    let xaml = r#"
<ResourceDictionary xmlns='https://github.com/ferroui'
                    xmlns:local='clr-namespace:FerroUI.Markup.Xaml.UnitTests.MarkupExtensions;assembly=FerroUI.Markup.Xaml.UnitTests'
                    xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'>
    <Color x:Key='Color1'>Black</Color>
    <local:OptionsMarkupExtension x:Key='MyKey'>
        <local:OptionsMarkupExtension.OptionA>
            <Button Content='Hello World' />
        </local:OptionsMarkupExtension.OptionA>
    </local:OptionsMarkupExtension>
    <Color x:Key='Color2'>White</Color>
</ResourceDictionary>"#;

    let resource_dictionary: Ref<ResourceDictionary> = load_as(xaml);
    let my_key = resource_dictionary.get(&ResourceKey::from("MyKey"));
    let button = assert_value_is_type::<Button>(&my_key);
    assert_string("Hello World", &button.content());
    assert_value(Colors::BLACK, &resource_dictionary.get(&ResourceKey::from("Color1")));
    assert_value(Colors::WHITE, &resource_dictionary.get(&ResourceKey::from("Color2")));
}

#[test]
fn support_complex_property_setters_list() {
    let _base = xaml_test_base();
    let _globals = setup_test_globals(boxed("option 1".to_string()));

    let xaml = r#"
<Panel xmlns='https://github.com/ferroui'
       xmlns:local='clr-namespace:FerroUI.Markup.Xaml.UnitTests.MarkupExtensions;assembly=FerroUI.Markup.Xaml.UnitTests'
       xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'>
    <TextBlock />
    <local:OptionsMarkupExtension>
        <local:OptionsMarkupExtension.OptionA>
            <Button Content='Hello World' />
        </local:OptionsMarkupExtension.OptionA>
    </local:OptionsMarkupExtension>
    <TextBox />
</Panel>"#;

    let panel: Ref<Panel> = load_as(xaml);
    assert_eq!(3, panel.children().count());
    assert_is_type::<Button>(&panel.children().get(1));
}

#[test]
fn binding_extension_works_inside_of_options_markup_extension() {
    for (option, expected) in [("option 1", "foo"), ("option 2", "bar")] {
        let _base = xaml_test_base();
        let _globals = setup_test_globals(boxed(option.to_string()));

        let xaml = r#"
<UserControl xmlns='https://github.com/ferroui'
             xmlns:local='clr-namespace:FerroUI.Markup.Xaml.UnitTests.MarkupExtensions;assembly=FerroUI.Markup.Xaml.UnitTests'
             xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'>
    <UserControl.Resources>
        <x:String x:Key='text'>foo</x:String>
    </UserControl.Resources>

    <TextBlock Name='textBlock' Text='{local:OptionsMarkupExtension OptionA={CompiledBinding Source={StaticResource text}}, OptionB=bar}'/>
</UserControl>"#;

        let window: Ref<UserControl> = load_as(xaml);
        let text_block = window.get_control::<TextBlock>("textBlock");

        assert_eq!(text_block.text().as_deref(), Some(expected), "row ({option:?}, {expected:?})");
    }
}

#[test]
fn resolve_expected_value_with_method_without_service_provider() {
    let _base = xaml_test_base();
    let _globals = setup_test_globals(boxed(2i32));

    let xaml = r#"
<TextBlock xmlns='https://github.com/ferroui'
           xmlns:local='clr-namespace:FerroUI.Markup.Xaml.UnitTests.MarkupExtensions;assembly=FerroUI.Markup.Xaml.UnitTests'
           xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'
           Text='{local:OptionsMarkupExtensionNoServiceProvider OptionB="Im Option 2", OptionA="Im Option 1"}' />"#;

    let text_block: Ref<TextBlock> = load_as(xaml);

    assert_eq!(text_block.text().as_deref(), Some("Im Option 2"));
}

#[test]
fn resolve_expected_value_minimal_extension() {
    let _base = xaml_test_base();
    let xaml = r#"
<TextBlock xmlns='https://github.com/ferroui'
           xmlns:local='clr-namespace:FerroUI.Markup.Xaml.UnitTests.MarkupExtensions;assembly=FerroUI.Markup.Xaml.UnitTests'
           xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'
           IsVisible='{local:OptionsMarkupExtensionMinimal OptionA=True}' />"#;

    let text_block: Ref<TextBlock> = load_as(xaml);

    assert!(text_block.is_set(Visual::is_visible_property()));
    assert!(text_block.is_visible());
}

#[test]
fn resolve_expected_value_extension_with_property() {
    let _base = xaml_test_base();
    let xaml = r#"
<TextBlock xmlns='https://github.com/ferroui'
           xmlns:local='clr-namespace:FerroUI.Markup.Xaml.UnitTests.MarkupExtensions;assembly=FerroUI.Markup.Xaml.UnitTests'
           xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'
           IsVisible='{local:OptionsMarkupExtensionWithProperty OptionA=True, Property=5}' />"#;

    let text_block: Ref<TextBlock> = load_as(xaml);

    assert!(text_block.is_set(Visual::is_visible_property()));
    assert!(text_block.is_visible());
}

// --- Tests this port adds: upstream has none of the element form of `OnPlatform`. ---

/// `OnPlatform` written as an element with `On` children, as the platform information page
/// of the control catalog has it: the extension has no content property, the children are
/// found through `IAddChild<On>`.
#[test]
fn on_platform_element_with_on_children() {
    let _base = xaml_test_base();
    let xaml = r#"
<Border xmlns='https://github.com/ferroui'>
    <Border.Background>
        <OnPlatform Default='Gray'>
            <On Options='macOS, Linux, Windows' Content='Green' />
        </OnPlatform>
    </Border.Background>
</Border>"#;

    let border: Ref<Border> = load_as(xaml);

    let background = border.background().expect("the background is set");
    let brush = background.as_solid_color_brush().expect("the background is an ISolidColorBrush");
    let expected = if cfg!(any(target_os = "macos", target_os = "linux", target_os = "windows")) {
        Colors::GREEN
    } else {
        Colors::GRAY
    };
    assert_eq!(expected, brush.color());
}

/// The same form with the branch as the content of `On`.
#[test]
fn on_platform_element_with_on_content_elements() {
    let _base = xaml_test_base();
    let xaml = r#"
<Border xmlns='https://github.com/ferroui'>
    <Border.Background>
        <OnPlatform>
            <OnPlatform.Default>
                <SolidColorBrush Color='Gray' />
            </OnPlatform.Default>
            <On Options='macOS, Linux, Windows'>
                <SolidColorBrush Color='Green' />
            </On>
        </OnPlatform>
    </Border.Background>
</Border>"#;

    let border: Ref<Border> = load_as(xaml);

    let background = border.background().expect("the background is set");
    let brush = background.as_solid_color_brush().expect("the background is an ISolidColorBrush");
    let expected = if cfg!(any(target_os = "macos", target_os = "linux", target_os = "windows")) {
        Colors::GREEN
    } else {
        Colors::GRAY
    };
    assert_eq!(expected, brush.color());
}
