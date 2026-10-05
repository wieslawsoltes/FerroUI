//! The test types declared by the upstream test file `Xaml/XamlIlTests.cs (namespace of the project root)`.

use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};

use ferroui_base::collections::FerroList;
use ferroui_base::data::converters::IMultiValueConverter;
use ferroui_base::data::core::{ClrPropertyInfo, IPropertyInfo, ValueType, ValueTypes};
use ferroui_base::data::model::{Event, INotifyPropertyChanged};
use ferroui_base::data::BindingError;
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::media::IBrush;
use ferroui_base::metadata::{from_markup_value, IServiceProvider, MarkupTyped};
use ferroui_base::utilities::{CultureInfo, EventArgs};
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, ferro_markup_type, ferro_properties, ferro_static_type,
    instantiate, AttachedProperty, BoxedValue, FerroObject, FerroObjectImpl, FerroProperty, ObjectType, Ref, StaticType,
    StyledElementImpl, StyledProperty, VisualImpl,
};
use ferroui_controls::primitives::TemplatedControlImpl;
use ferroui_controls::platform::PlatformManager;
use ferroui_controls::{
    ContentControlImpl, Control, ControlImpl, ItemsControl, ItemsSource, TopLevelImpl, UserControl, Window,
    WindowBaseImpl, WindowImpl,
};
use ferroui_markup_xaml_loader::FerroRuntimeXamlLoader;
use ferroui_markup_xaml::converters::{ITypeDescriptorContext, TypeConverter};
use ferroui_markup_xaml::{FerroXamlLoader, IProvideValueTarget, ServiceProviderExtensions, XamlLoadException};

use crate::support::TypeModule;

// --- XamlIlBugTestsBrushToColorConverter -------------------------------------

/// A multi-value converter that returns the color of the first value, when
/// that value is a solid color brush.
pub struct XamlIlBugTestsBrushToColorConverter {
    this: Weak<XamlIlBugTestsBrushToColorConverter>,
}

crate::test_identity_eq!(XamlIlBugTestsBrushToColorConverter);

impl XamlIlBugTestsBrushToColorConverter {
    pub fn new() -> Rc<Self> {
        Rc::new_cyclic(|this| Self { this: this.clone() })
    }

    fn as_multi_value_converter(&self) -> Rc<dyn IMultiValueConverter> {
        self.this.upgrade().expect("the converter is alive while it is used")
    }
}

impl IMultiValueConverter for XamlIlBugTestsBrushToColorConverter {
    fn convert(
        &self,
        values: &[Option<BoxedValue>],
        _target_type: ValueType,
        _parameter: Option<&BoxedValue>,
    ) -> Result<Option<BoxedValue>, BindingError> {
        let brush = from_markup_value::<Rc<dyn IBrush>>(&values[0]);
        let color = brush.and_then(|brush| brush.as_solid_color_brush().map(|brush| brush.color()));
        Ok(color.map(|color| Rc::new(color) as BoxedValue))
    }
}

ferro_markup_type!(class XamlIlBugTestsBrushToColorConverter {
    this: Rc<XamlIlBugTestsBrushToColorConverter>,
    handles: [
        XamlIlBugTestsBrushToColorConverter,
        Rc<XamlIlBugTestsBrushToColorConverter>,
        Option<Rc<XamlIlBugTestsBrushToColorConverter>>
    ],
    namespace: "FerroUI.Markup.Xaml.UnitTests",
    interfaces: [Rc<dyn IMultiValueConverter>],
    constructors: [() => XamlIlBugTestsBrushToColorConverter::new],
});

// --- XamlIlBugTestsDataContext -----------------------------------------------

/// A data context that only notifies property changes.
pub struct XamlIlBugTestsDataContext {
    property_changed: Event<str>,
}

crate::test_identity_eq!(XamlIlBugTestsDataContext);

impl INotifyPropertyChanged for XamlIlBugTestsDataContext {
    fn property_changed(&self) -> &Event<str> {
        &self.property_changed
    }
}

impl XamlIlBugTestsDataContext {
    pub fn new() -> Rc<Self> {
        Rc::new(Self { property_changed: Event::new() })
    }

    pub fn on_property_changed(&self, property_name: &str) {
        self.property_changed.raise(property_name);
    }
}

ferro_markup_type!(class XamlIlBugTestsDataContext {
    this: Rc<XamlIlBugTestsDataContext>,
    handles: [XamlIlBugTestsDataContext, Rc<XamlIlBugTestsDataContext>, Option<Rc<XamlIlBugTestsDataContext>>],
    namespace: "FerroUI.Markup.Xaml.UnitTests",
    constructors: [() => XamlIlBugTestsDataContext::new],
    notify_property_changed: XamlIlBugTestsDataContext,
});

// --- XamlIlBugTestsStaticClassWithAttachedProperty ---------------------------

/// A static type that owns the attached property `TestInt` of controls.
pub struct XamlIlBugTestsStaticClassWithAttachedProperty;

ferro_static_type!(XamlIlBugTestsStaticClassWithAttachedProperty);

ferro_properties! {
    impl XamlIlBugTestsStaticClassWithAttachedProperty {
        pub fn test_int_property() -> AttachedProperty<i32> {
            FerroProperty::register_attached::<XamlIlBugTestsStaticClassWithAttachedProperty, Control, _>("TestInt", 0)
        }
    }
}

impl XamlIlBugTestsStaticClassWithAttachedProperty {
    pub fn set_test_int(control: &Control, value: i32) {
        control.set_value(Self::test_int_property(), value)
    }

    pub fn get_test_int(control: &Control) -> i32 {
        control.get_value(Self::test_int_property())
    }
}

ferro_markup_type!(static XamlIlBugTestsStaticClassWithAttachedProperty {
    type_info: XamlIlBugTestsStaticClassWithAttachedProperty,
    namespace: "FerroUI.Markup.Xaml.UnitTests",
    methods: [
        static fn SetTestInt(Ref<Control>, i32) => |control: Ref<Control>, value: i32| {
            XamlIlBugTestsStaticClassWithAttachedProperty::set_test_int(&control, value)
        },
        static fn GetTestInt(Ref<Control>) -> i32 =>
            |control: Ref<Control>| XamlIlBugTestsStaticClassWithAttachedProperty::get_test_int(&control),
    ],
});

// --- XamlIlCheckClrPropertyInfoExtension -------------------------------------

/// A markup extension that reads the current value of its target property
/// through the plain-property description of the provide-value target and
/// provides that value plus one.
pub struct XamlIlCheckClrPropertyInfoExtension {
    expected_property_name: RefCell<Option<String>>,
}

crate::test_identity_eq!(XamlIlCheckClrPropertyInfoExtension);

impl XamlIlCheckClrPropertyInfoExtension {
    pub fn new() -> Rc<Self> {
        Rc::new(Self { expected_property_name: RefCell::new(None) })
    }

    pub fn expected_property_name(&self) -> Option<String> {
        self.expected_property_name.borrow().clone()
    }

    pub fn set_expected_property_name(&self, value: Option<String>) {
        *self.expected_property_name.borrow_mut() = value;
    }

    pub fn provide_value(&self, prov: &Rc<dyn IServiceProvider>) -> Option<BoxedValue> {
        let pvt = prov.get_required_service::<Rc<dyn IProvideValueTarget>>();
        // `(ClrPropertyInfo)pvt.TargetProperty`.
        let info = from_markup_value::<Rc<dyn IPropertyInfo>>(&pvt.target_property())
            .expect("Unable to cast the target property to type 'ClrPropertyInfo'.");
        let info = info
            .as_any()
            .and_then(|info| info.downcast_ref::<ClrPropertyInfo>())
            .expect("Unable to cast the target property to type 'ClrPropertyInfo'.");
        let target = pvt.target_object().expect("the provide-value target has a target object");
        let v = from_markup_value::<i32>(&info.get_boxed(&target)).expect("the property holds an integer");
        Some(Rc::new(v + 1))
    }
}

ferro_markup_type!(class XamlIlCheckClrPropertyInfoExtension {
    this: Rc<XamlIlCheckClrPropertyInfoExtension>,
    handles: [
        XamlIlCheckClrPropertyInfoExtension,
        Rc<XamlIlCheckClrPropertyInfoExtension>,
        Option<Rc<XamlIlCheckClrPropertyInfoExtension>>
    ],
    namespace: "FerroUI.Markup.Xaml.UnitTests",
    constructors: [() => XamlIlCheckClrPropertyInfoExtension::new],
    properties: [
        ExpectedPropertyName: Option<String> {
            get: |this: &Rc<XamlIlCheckClrPropertyInfoExtension>| this.expected_property_name(),
            set: |this: &Rc<XamlIlCheckClrPropertyInfoExtension>, value: Option<String>| {
                this.set_expected_property_name(value)
            }
        },
    ],
    methods: [
        fn ProvideValue(Rc<dyn IServiceProvider>) -> Option<BoxedValue> =>
            |this: &Rc<XamlIlCheckClrPropertyInfoExtension>, prov: Rc<dyn IServiceProvider>| this.provide_value(&prov),
    ],
});

// --- XamlIlClassWithClrPropertyWithValue -------------------------------------

/// A plain class with a plain property that starts with a value.
pub struct XamlIlClassWithClrPropertyWithValue {
    count: Cell<i32>,
}

crate::test_identity_eq!(XamlIlClassWithClrPropertyWithValue);

impl XamlIlClassWithClrPropertyWithValue {
    pub fn new() -> Rc<Self> {
        Rc::new(Self { count: Cell::new(5) })
    }

    pub fn count(&self) -> i32 {
        self.count.get()
    }

    pub fn set_count(&self, value: i32) {
        self.count.set(value)
    }
}

ferro_markup_type!(class XamlIlClassWithClrPropertyWithValue {
    this: Rc<XamlIlClassWithClrPropertyWithValue>,
    handles: [
        XamlIlClassWithClrPropertyWithValue,
        Rc<XamlIlClassWithClrPropertyWithValue>,
        Option<Rc<XamlIlClassWithClrPropertyWithValue>>
    ],
    namespace: "FerroUI.Markup.Xaml.UnitTests",
    constructors: [() => XamlIlClassWithClrPropertyWithValue::new],
    properties: [
        Count: i32 {
            get: |this: &Rc<XamlIlClassWithClrPropertyWithValue>| this.count(),
            set: |this: &Rc<XamlIlClassWithClrPropertyWithValue>, value: i32| this.set_count(value)
        },
    ],
});

// --- XamlIlClassWithTypeConverterOnFerroProperty -----------------------------

/// The item type of the property with the converter (the nested class
/// `MyType` of the class below).
pub struct MyType {
    value: String,
}

crate::test_identity_eq!(MyType);

impl MyType {
    pub fn new(value: &str) -> Rc<Self> {
        Rc::new(Self { value: value.to_string() })
    }

    pub fn value(&self) -> String {
        self.value.clone()
    }
}

ferro_markup_type!(class MyType {
    this: Rc<MyType>,
    handles: [MyType, Rc<MyType>, Option<Rc<MyType>>],
    namespace: "FerroUI.Markup.Xaml.UnitTests",
    constructors: [(String) => |value: String| MyType::new(&value)],
    properties: [
        Value: String { get: |this: &Rc<MyType>| this.value() },
    ],
});

/// The value of the property with the converter: a sequence of items.
pub type MyTypes = Vec<Rc<MyType>>;

/// The converter base, for the conversions the converter below leaves to
/// its base class.
struct BaseTypeConverter;

impl TypeConverter for BaseTypeConverter {}

/// Converts comma-separated text to items (the nested class
/// `MyTypeConverter` of the class below).
pub struct MyTypeConverter {
    this: Weak<MyTypeConverter>,
}

crate::test_identity_eq!(MyTypeConverter);

impl MyTypeConverter {
    pub fn new() -> Rc<Self> {
        Rc::new_cyclic(|this| Self { this: this.clone() })
    }

    fn as_type_converter(&self) -> Rc<dyn TypeConverter> {
        self.this.upgrade().expect("the converter is alive while it is used")
    }
}

impl TypeConverter for MyTypeConverter {
    fn can_convert_from(&self, _context: Option<&Rc<dyn ITypeDescriptorContext>>, source_type: ValueType) -> bool {
        source_type.is_string()
    }

    fn convert_from(
        &self,
        context: Option<&Rc<dyn ITypeDescriptorContext>>,
        culture: Option<&CultureInfo>,
        value: Option<&BoxedValue>,
    ) -> Result<Option<BoxedValue>, XamlLoadException> {
        if let Some(s) = value.and_then(|value| value.downcast_ref::<String>()) {
            let items: MyTypes = s.split(',').filter(|x| !x.is_empty()).map(|x| MyType::new(x.trim())).collect();
            // The value of the property: a (non-null) sequence of items.
            return Ok(Some(Rc::new(Some(items))));
        }
        BaseTypeConverter.convert_from(context, culture, value)
    }
}

ferro_markup_type!(class MyTypeConverter {
    this: Rc<MyTypeConverter>,
    handles: [MyTypeConverter, Rc<MyTypeConverter>, Option<Rc<MyTypeConverter>>],
    namespace: "FerroUI.Markup.Xaml.UnitTests",
    base: Rc<dyn TypeConverter>,
    constructors: [() => MyTypeConverter::new],
    methods: [
        fn CanConvertFrom(Option<Rc<dyn ITypeDescriptorContext>>, ValueType) -> bool =>
            |this: &Rc<MyTypeConverter>, context: Option<Rc<dyn ITypeDescriptorContext>>, source_type: ValueType| {
                this.can_convert_from(context.as_ref(), source_type)
            },
        try fn ConvertFrom(Option<Rc<dyn ITypeDescriptorContext>>, Option<CultureInfo>, Option<BoxedValue>) -> Option<BoxedValue> =>
            |this: &Rc<MyTypeConverter>,
             context: Option<Rc<dyn ITypeDescriptorContext>>,
             culture: Option<CultureInfo>,
             value: Option<BoxedValue>| {
                this.convert_from(context.as_ref(), culture.as_ref(), value.as_ref())
            },
    ],
});

/// An object of the object model whose registered property states its type
/// converter with an attribute on the property.
#[repr(C)]
pub struct XamlIlClassWithTypeConverterOnFerroProperty {
    base: FerroObject,
}

ferro_class!(XamlIlClassWithTypeConverterOnFerroProperty: FerroObject);
ferro_impl_classes!(XamlIlClassWithTypeConverterOnFerroProperty: FerroObjectImpl);
ferro_class_info!(XamlIlClassWithTypeConverterOnFerroProperty {
    new: XamlIlClassWithTypeConverterOnFerroProperty::new,
    markup: {
        namespace: "FerroUI.Markup.Xaml.UnitTests",
        property_attributes: [MyProp: [TypeConverter(type(Rc<MyTypeConverter>))]],
    },
});

ferro_properties! {
    impl XamlIlClassWithTypeConverterOnFerroProperty {
        pub fn my_prop_property() -> StyledProperty<Option<MyTypes>> {
            FerroProperty::register::<XamlIlClassWithTypeConverterOnFerroProperty, _>("MyProp", None)
        }
    }
}

impl XamlIlClassWithTypeConverterOnFerroProperty {
    pub fn construct() -> Self {
        Self { base: FerroObject::construct() }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    pub fn my_prop(&self) -> Option<MyTypes> {
        self.get_value(Self::my_prop_property())
    }

    pub fn set_my_prop(&self, value: Option<MyTypes>) {
        self.set_value(Self::my_prop_property(), value)
    }
}

// --- XamlIlBugTestsEventHandlerCodeBehind -------------------------------------

/// A window whose document names a handler of the class for an event of a
/// control inside a data template: the handler saves the data context of
/// its sender.
#[repr(C)]
pub struct XamlIlBugTestsEventHandlerCodeBehind {
    base: Window,
    saved_context: RefCell<Option<BoxedValue>>,
}

ferro_class!(XamlIlBugTestsEventHandlerCodeBehind: Window);
ferro_impl_classes!(
    XamlIlBugTestsEventHandlerCodeBehind: FerroObjectImpl,
    StyledElementImpl,
    VisualImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl,
    TemplatedControlImpl,
    ContentControlImpl,
    TopLevelImpl,
    WindowBaseImpl,
    WindowImpl
);
ferro_class_info!(XamlIlBugTestsEventHandlerCodeBehind {
    new: XamlIlBugTestsEventHandlerCodeBehind::new,
    markup: {
        namespace: "FerroUI.Markup.Xaml.UnitTests",
        methods: [
            fn HandleDataContextChanged(Option<BoxedValue>, EventArgs) =>
                |this: &Ref<XamlIlBugTestsEventHandlerCodeBehind>, sender: Option<BoxedValue>, args: EventArgs| {
                    this.handle_data_context_changed(&sender, &args)
                },
        ],
    },
});

impl XamlIlBugTestsEventHandlerCodeBehind {
    pub fn construct() -> Self {
        Self { base: Window::construct(PlatformManager::create_window()), saved_context: RefCell::new(None) }
    }

    /// The constructor: populates the instance from its document and gives
    /// the items control of the document its items.
    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        let root: BoxedValue = Rc::new(this.clone());
        let loaded = FerroRuntimeXamlLoader::load(
            "
<Window x:Class='FerroUI.Markup.Xaml.UnitTests.XamlIlBugTestsEventHandlerCodeBehind'
  xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'
  xmlns='https://github.com/ferroui'
  xmlns:local='clr-namespace:FerroUI.Markup.Xaml.UnitTests;assembly=FerroUI.Markup.Xaml.UnitTests'
>
  <ItemsControl>
    <ItemsControl.ItemTemplate>
      <DataTemplate>
        <Button DataContextChanged='HandleDataContextChanged' Content='{Binding .}' />
      </DataTemplate>
    </ItemsControl.ItemTemplate>
  </ItemsControl>
</Window>
",
            Some(&crate::ASSEMBLY),
            Some(root),
            None,
            false,
        );
        if let Err(error) = loaded {
            panic!("{}", crate::support::loader::describe(&error));
        }
        let items_control = crate::support::helpers::object_of::<ItemsControl>(&this.content());
        items_control.set_items_source(Some(ItemsSource::from_strs(["123"])));
        this
    }

    /// `SavedContext`.
    pub fn saved_context(&self) -> Option<BoxedValue> {
        self.saved_context.borrow().clone()
    }

    pub fn handle_data_context_changed(&self, sender: &Option<BoxedValue>, _args: &EventArgs) {
        let sender = crate::support::helpers::object_of::<Control>(sender);
        *self.saved_context.borrow_mut() = sender.data_context();
    }
}

// --- classes with compiled markup ---------------------------------------------

/// Populates a new instance of a class with compiled markup from the
/// document of the class: the call the XAML compiler rewrites to the
/// generated populate method of the class.
fn load_compiled_markup<T: ObjectType>(this: &Ref<T>) {
    let object: BoxedValue = Rc::new(this.clone());
    if let Err(error) = FerroXamlLoader::load_object(&object) {
        panic!("{}", error.message());
    }
}

macro_rules! user_control_class {
    ($class:ident) => {
        ferro_class!($class: UserControl);
        ferro_impl_classes!(
            $class: FerroObjectImpl,
            StyledElementImpl,
            VisualImpl,
            LayoutableImpl,
            InteractiveImpl,
            InputElementImpl,
            ControlImpl,
            TemplatedControlImpl,
            ContentControlImpl
        );
    };
}

/// A user control with a plain property; its document
/// (`Xaml/XamlIlClassWithCustomProperty.xaml`) sets the property.
#[repr(C)]
pub struct XamlIlClassWithCustomProperty {
    base: UserControl,
    test: RefCell<Option<String>>,
}

user_control_class!(XamlIlClassWithCustomProperty);
ferro_class_info!(XamlIlClassWithCustomProperty {
    new: XamlIlClassWithCustomProperty::new,
    markup: {
        namespace: "FerroUI.Markup.Xaml.UnitTests",
        properties: [
            Test: Option<String> {
                get: XamlIlClassWithCustomProperty::test,
                set: XamlIlClassWithCustomProperty::set_test
            },
        ],
    },
});

impl XamlIlClassWithCustomProperty {
    pub fn construct() -> Self {
        Self { base: UserControl::construct(), test: RefCell::new(None) }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        load_compiled_markup(&this);
        this
    }

    pub fn test(&self) -> Option<String> {
        self.test.borrow().clone()
    }

    pub fn set_test(&self, value: Option<String>) {
        *self.test.borrow_mut() = value;
    }
}

/// A user control whose only content is its document
/// (`Xaml/XamlIlClassWithPrecompiledXaml.xaml`): the constructor of a class
/// with compiled markup populates the instance from it.
#[repr(C)]
pub struct XamlIlClassWithPrecompiledXaml {
    base: UserControl,
}

user_control_class!(XamlIlClassWithPrecompiledXaml);
ferro_class_info!(XamlIlClassWithPrecompiledXaml {
    new: XamlIlClassWithPrecompiledXaml::new,
    markup: { namespace: "FerroUI.Markup.Xaml.UnitTests" },
});

impl XamlIlClassWithPrecompiledXaml {
    pub fn construct() -> Self {
        Self { base: UserControl::construct() }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        load_compiled_markup(&this);
        this
    }
}

// --- CompiledBindingRootMock, CompiledBindingItemMock ------------------------

/// The read-only list of items of the compiled binding mocks.
pub type CompiledBindingItemMocks = Rc<FerroList<Rc<CompiledBindingItemMock>>>;

/// The item of the compiled binding tests.
pub struct CompiledBindingItemMock {
    name: RefCell<String>,
    inner_items: RefCell<CompiledBindingItemMocks>,
}

crate::test_identity_eq!(CompiledBindingItemMock);

impl CompiledBindingItemMock {
    pub fn new() -> Rc<Self> {
        Rc::new(Self { name: RefCell::new(String::new()), inner_items: RefCell::new(Rc::new(FerroList::new())) })
    }

    pub fn name(&self) -> String {
        self.name.borrow().clone()
    }

    pub fn set_name(&self, value: String) {
        *self.name.borrow_mut() = value;
    }

    pub fn inner_items(&self) -> CompiledBindingItemMocks {
        self.inner_items.borrow().clone()
    }

    pub fn set_inner_items(&self, value: CompiledBindingItemMocks) {
        *self.inner_items.borrow_mut() = value;
    }
}

ferro_markup_type!(class CompiledBindingItemMock {
    this: Rc<CompiledBindingItemMock>,
    handles: [CompiledBindingItemMock, Rc<CompiledBindingItemMock>, Option<Rc<CompiledBindingItemMock>>],
    namespace: "FerroUI.Markup.Xaml.UnitTests",
    constructors: [() => CompiledBindingItemMock::new],
    properties: [
        Name: String {
            get: |this: &Rc<CompiledBindingItemMock>| this.name(),
            set: |this: &Rc<CompiledBindingItemMock>, value: String| this.set_name(value)
        },
        InnerItems: CompiledBindingItemMocks {
            get: |this: &Rc<CompiledBindingItemMock>| this.inner_items(),
            set: |this: &Rc<CompiledBindingItemMock>, value: CompiledBindingItemMocks| this.set_inner_items(value)
        },
    ],
});

/// The root of the compiled binding tests: a user control with plain
/// read-only properties.
#[repr(C)]
pub struct CompiledBindingRootMock {
    base: UserControl,
    items: CompiledBindingItemMocks,
}

user_control_class!(CompiledBindingRootMock);
ferro_class_info!(CompiledBindingRootMock {
    new: CompiledBindingRootMock::new,
    markup: {
        namespace: "FerroUI.Markup.Xaml.UnitTests",
        properties: [
            Greeting: String { get: CompiledBindingRootMock::greeting },
            RootProperty: String { get: CompiledBindingRootMock::root_property },
            Items: CompiledBindingItemMocks { get: CompiledBindingRootMock::items },
        ],
    },
});

impl CompiledBindingRootMock {
    pub fn construct() -> Self {
        let inner = CompiledBindingItemMock::new();
        inner.set_name("Inner".to_string());
        let outer = CompiledBindingItemMock::new();
        outer.set_name("Outer".to_string());
        outer.set_inner_items(Rc::new(FerroList::from_items([inner])));
        Self { base: UserControl::construct(), items: Rc::new(FerroList::from_items([outer])) }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    pub fn greeting(&self) -> String {
        "Hello".to_string()
    }

    pub fn root_property(&self) -> String {
        "RootValue".to_string()
    }

    pub fn items(&self) -> CompiledBindingItemMocks {
        self.items.clone()
    }
}

pub(crate) const MODULE: TypeModule = TypeModule {
    types: &[
        <XamlIlBugTestsStaticClassWithAttachedProperty as StaticType>::TYPE,
        XamlIlClassWithTypeConverterOnFerroProperty::TYPE,
        XamlIlClassWithCustomProperty::TYPE,
        XamlIlClassWithPrecompiledXaml::TYPE,
        CompiledBindingRootMock::TYPE,
        XamlIlBugTestsEventHandlerCodeBehind::TYPE,
    ],
    markup_types: &[
        <XamlIlBugTestsBrushToColorConverter as MarkupTyped>::MARKUP,
        <XamlIlBugTestsDataContext as MarkupTyped>::MARKUP,
        <XamlIlBugTestsStaticClassWithAttachedProperty as MarkupTyped>::MARKUP,
        <XamlIlCheckClrPropertyInfoExtension as MarkupTyped>::MARKUP,
        <XamlIlClassWithClrPropertyWithValue as MarkupTyped>::MARKUP,
        <MyType as MarkupTyped>::MARKUP,
        <MyTypeConverter as MarkupTyped>::MARKUP,
        <CompiledBindingItemMock as MarkupTyped>::MARKUP,
    ],
    value_types: || {
        ValueTypes::register_reference::<XamlIlBugTestsBrushToColorConverter>();
        ValueTypes::register_reference::<XamlIlBugTestsDataContext>();
        ValueTypes::register_reference::<XamlIlCheckClrPropertyInfoExtension>();
        ValueTypes::register_reference::<XamlIlClassWithClrPropertyWithValue>();
        ValueTypes::register_reference::<MyType>();
        ValueTypes::register_reference::<MyTypeConverter>();
        ValueTypes::register_reference::<CompiledBindingItemMock>();
        ValueTypes::register_nullable::<MyTypes>();
        ItemsSource::register_binding_conversion::<CompiledBindingItemMocks>();
        ValueTypes::register_cast::<XamlIlBugTestsBrushToColorConverter, Rc<dyn IMultiValueConverter>>(
            XamlIlBugTestsBrushToColorConverter::as_multi_value_converter,
        );
        ValueTypes::register_cast::<MyTypeConverter, Rc<dyn TypeConverter>>(MyTypeConverter::as_type_converter);
    },
};
