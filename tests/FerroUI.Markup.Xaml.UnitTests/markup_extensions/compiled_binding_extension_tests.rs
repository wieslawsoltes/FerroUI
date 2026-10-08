//! Ported from the upstream `MarkupExtensions/CompiledBindingExtensionTests`:
//! the test types of the file and its tests.

use ferroui_base::utilities::CultureInfo;
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::{Rc, Weak};

use ferroui_base::data::converters::IValueConverter;
use ferroui_base::data::core::plugins::{ObservableValue, TaskValue};
use ferroui_base::data::core::{BindingExpression, TypedBindingExpression, ValueType, ValueTypes, INDEXER_NAME};
use ferroui_base::data::model::{BindableArray, BindableList, Event, INotifyCollectionChanged, INotifyPropertyChanged};
use ferroui_base::data::{
    BindingBase, BindingError, BindingOperations, CompiledBinding, CompiledBindingPathElementKind,
};
use ferroui_base::input::{InputElement, InputElementImpl, Key, KeyEventArgs};
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::media::immutable::ImmutableSolidColorBrush;
use ferroui_base::media::{Brushes, Color, Colors, SolidColorBrush};
use ferroui_base::metadata::{MarkupDelegate, MarkupTyped};
use ferroui_base::threading::Dispatcher;
use ferroui_base::utilities::Decimal;
use ferroui_base::reactive::{IObservable, IObserver, LightweightSubject};
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, ferro_markup_type, ferro_properties, instantiate, BoxedValue,
    DirectProperty, FerroObjectImpl, FerroProperty, Ref, Size, StyledElement, StyledElementImpl, Visual, VisualImpl,
};
use ferroui_controls::presenters::ContentPresenter;
use ferroui_controls::templates::{IDataTemplate, ITemplateWithParam};
use ferroui_controls::{
    Button, ComboBox, ContentControl, Control, ControlImpl, Grid, GridLength, GridUnitType, IItemsList,
    ItemsControl, ItemsSource, Panel, TextBlock, TextBox, Window,
};
use ferroui_markup::markup::data::DelayedBinding;
use ferroui_markup_xaml::templates::TemplateContent;
use ferroui_markup_xaml::RuntimeXamlLoaderConfiguration;
use xamlx::exceptions::XamlError;

use crate::support::app::*;
use crate::support::helpers::*;
use crate::support::loader::*;
use crate::support::TypeModule;
use crate::support_bindings::*;

// --- test types -------------------------------------------------------------
//
// What the declarations cannot state today (see the report of the port):
// * static properties: declared as static values (`fields:`).

/// The text of an untyped value in an interpolated string (`$"{value}"`):
/// null is empty.
fn display(value: &Option<BoxedValue>) -> String {
    match value {
        Some(value) => ValueTypes::to_display_string(Some(value)),
        None => String::new(),
    }
}

/// The message of the exception a dictionary throws for a missing key.
fn key_not_found(key: &str) -> String {
    format!("The given key '{key}' was not present in the dictionary.")
}

/// `INonIntegerIndexer`: an indexer with a string key.
pub trait INonIntegerIndexer {
    /// `this[key]` (get), or `None` if the key is missing.
    fn try_get(&self, key: &str) -> Option<String>;

    /// `this[key] = value`.
    fn set(&self, key: &str, value: String);

    /// `this[key]` (get). Panics if the key is missing.
    fn get(&self, key: &str) -> String {
        self.try_get(key).unwrap_or_else(|| panic!("{}", key_not_found(key)))
    }
}

/// `INonIntegerIndexerDerived`.
pub trait INonIntegerIndexerDerived: INonIntegerIndexer {}

/// `IHasProperty`.
pub trait IHasProperty {
    fn string_property(&self) -> Option<String>;
    fn set_string_property(&self, value: Option<String>);
}

/// `IHasPropertyDerived`.
pub trait IHasPropertyDerived: IHasProperty {}

/// `IHasExplicitProperty`.
pub trait IHasExplicitProperty {
    fn explicit_property(&self) -> String;
}

crate::test_identity_eq!(
    dyn INonIntegerIndexer,
    dyn INonIntegerIndexerDerived,
    dyn IHasProperty,
    dyn IHasPropertyDerived,
    dyn IHasExplicitProperty,
);

ferro_markup_type!(interface dyn INonIntegerIndexer as "INonIntegerIndexer" {
    this: Rc<dyn INonIntegerIndexer>,
    handles: [Rc<dyn INonIntegerIndexer>, Option<Rc<dyn INonIntegerIndexer>>],
    indexers: [
        (String) -> String {
            try_get: |this: &Rc<dyn INonIntegerIndexer>, key: String| {
                this.try_get(&key).ok_or_else(|| key_not_found(&key))
            },
            set: |this: &Rc<dyn INonIntegerIndexer>, key: String, value: String| this.set(&key, value)
        },
    ],
});

ferro_markup_type!(interface dyn INonIntegerIndexerDerived as "INonIntegerIndexerDerived" {
    this: Rc<dyn INonIntegerIndexerDerived>,
    handles: [Rc<dyn INonIntegerIndexerDerived>, Option<Rc<dyn INonIntegerIndexerDerived>>],
    interfaces: [Rc<dyn INonIntegerIndexer>],
});

ferro_markup_type!(interface dyn IHasProperty as "IHasProperty" {
    this: Rc<dyn IHasProperty>,
    handles: [Rc<dyn IHasProperty>, Option<Rc<dyn IHasProperty>>],
    properties: [
        StringProperty: Option<String> {
            get: |this: &Rc<dyn IHasProperty>| this.string_property(),
            set: |this: &Rc<dyn IHasProperty>, value: Option<String>| this.set_string_property(value)
        },
    ],
});

ferro_markup_type!(interface dyn IHasPropertyDerived as "IHasPropertyDerived" {
    this: Rc<dyn IHasPropertyDerived>,
    handles: [Rc<dyn IHasPropertyDerived>, Option<Rc<dyn IHasPropertyDerived>>],
    interfaces: [Rc<dyn IHasProperty>],
});

ferro_markup_type!(interface dyn IHasExplicitProperty as "IHasExplicitProperty" {
    this: Rc<dyn IHasExplicitProperty>,
    handles: [Rc<dyn IHasExplicitProperty>, Option<Rc<dyn IHasExplicitProperty>>],
    properties: [
        ExplicitProperty: String { get: |this: &Rc<dyn IHasExplicitProperty>| this.explicit_property() },
    ],
});

/// `AppendConverter`: joins the value, the parameter and the culture with
/// `+`.
pub struct AppendConverter {
    this: Weak<AppendConverter>,
}

crate::test_identity_eq!(AppendConverter);

impl AppendConverter {
    pub fn new() -> Rc<AppendConverter> {
        Rc::new_cyclic(|this| Self { this: this.clone() })
    }

    /// `AppendConverter.Instance`.
    pub fn instance() -> Rc<dyn IValueConverter> {
        thread_local! {
            static INSTANCE: Rc<AppendConverter> = AppendConverter::new();
        }
        INSTANCE.with(|instance| instance.as_value_converter())
    }

    /// The converter as the value converter contract.
    pub fn as_value_converter(&self) -> Rc<dyn IValueConverter> {
        self.this.upgrade().expect("the converter is alive while it is used")
    }
}

impl IValueConverter for AppendConverter {
    fn convert(
        &self,
        value: Option<&BoxedValue>,
        _target_type: ValueType,
        parameter: Option<&BoxedValue>,
        culture: &CultureInfo,
    ) -> Result<Option<BoxedValue>, BindingError> {
        let text = format!("{}+{}+{}", display(&value.cloned()), display(&parameter.cloned()), culture);
        Ok(Some(Rc::new(text)))
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

ferro_markup_type!(class AppendConverter {
    this: Rc<AppendConverter>,
    handles: [AppendConverter, Rc<AppendConverter>, Option<Rc<AppendConverter>>],
    interfaces: [Rc<dyn IValueConverter>],
    constructors: [() => AppendConverter::new],
    fields: [Instance: Rc<dyn IValueConverter> => AppendConverter::instance],
});

/// `TestData`.
pub struct TestData {
    string_property: RefCell<Option<String>>,
}

crate::test_identity_eq!(TestData);

impl TestData {
    pub fn new() -> Rc<TestData> {
        Rc::new(Self { string_property: RefCell::new(None) })
    }

    pub fn string_property(&self) -> Option<String> {
        self.string_property.borrow().clone()
    }

    pub fn set_string_property(&self, value: Option<String>) {
        self.string_property.replace(value);
    }
}

ferro_markup_type!(class TestData {
    this: Rc<TestData>,
    handles: [TestData, Rc<TestData>, Option<Rc<TestData>>],
    constructors: [() => TestData::new],
    properties: [
        StringProperty: Option<String> { get: TestData::string_property, set: TestData::set_string_property },
    ],
});

/// `OuterClass`.
pub struct OuterClass;

crate::test_identity_eq!(OuterClass);

impl OuterClass {
    pub fn new() -> Rc<OuterClass> {
        Rc::new(Self)
    }
}

ferro_markup_type!(class OuterClass {
    this: Rc<OuterClass>,
    handles: [OuterClass, Rc<OuterClass>, Option<Rc<OuterClass>>],
    constructors: [() => OuterClass::new],
});

/// `OuterClass.NestedClass`.
pub struct NestedClass {
    nested_property: RefCell<String>,
}

crate::test_identity_eq!(NestedClass);

impl NestedClass {
    pub fn new() -> Rc<NestedClass> {
        Rc::new(Self { nested_property: RefCell::new("nested value".to_string()) })
    }

    pub fn nested_property(&self) -> String {
        self.nested_property.borrow().clone()
    }

    pub fn set_nested_property(&self, value: String) {
        self.nested_property.replace(value);
    }
}

ferro_markup_type!(class NestedClass as "OuterClass+NestedClass" {
    this: Rc<NestedClass>,
    handles: [NestedClass, Rc<NestedClass>, Option<Rc<NestedClass>>],
    constructors: [() => NestedClass::new],
    properties: [
        NestedProperty: String { get: NestedClass::nested_property, set: NestedClass::set_nested_property },
    ],
});

/// `TestDataContextBaseClass`.
pub struct TestDataContextBaseClass;

crate::test_identity_eq!(TestDataContextBaseClass);

impl TestDataContextBaseClass {
    pub fn new() -> Rc<TestDataContextBaseClass> {
        Rc::new(Self)
    }
}

ferro_markup_type!(class TestDataContextBaseClass {
    this: Rc<TestDataContextBaseClass>,
    handles: [TestDataContextBaseClass, Rc<TestDataContextBaseClass>, Option<Rc<TestDataContextBaseClass>>],
    constructors: [() => TestDataContextBaseClass::new],
});

/// `TestItemsCollectionDataContext`.
pub struct TestItemsCollectionDataContext {
    base: Rc<TestDataContextBaseClass>,
    items: Rc<BindableList<Rc<TestData>>>,
}

crate::test_identity_eq!(TestItemsCollectionDataContext);

impl TestItemsCollectionDataContext {
    pub fn new() -> Rc<TestItemsCollectionDataContext> {
        Rc::new(Self { base: TestDataContextBaseClass::new(), items: BindableList::new(Vec::new()) })
    }

    /// The part of the object that is the base class.
    pub fn base(&self) -> &Rc<TestDataContextBaseClass> {
        &self.base
    }

    pub fn items(&self) -> Rc<BindableList<Rc<TestData>>> {
        self.items.clone()
    }
}

ferro_markup_type!(class TestItemsCollectionDataContext {
    this: Rc<TestItemsCollectionDataContext>,
    handles: [
        TestItemsCollectionDataContext,
        Rc<TestItemsCollectionDataContext>,
        Option<Rc<TestItemsCollectionDataContext>>
    ],
    base: Rc<TestDataContextBaseClass>,
    constructors: [() => TestItemsCollectionDataContext::new],
    properties: [
        Items: Rc<BindableList<Rc<TestData>>> { get: TestItemsCollectionDataContext::items },
    ],
});

/// `TestDataContext.NonIntegerIndexer`: an indexer with a string key that
/// raises a property change notification for the indexer when a value is
/// set.
pub struct NonIntegerIndexer {
    this: Weak<NonIntegerIndexer>,
    storage: RefCell<HashMap<String, String>>,
    property_changed: Event<str>,
}

crate::test_identity_eq!(NonIntegerIndexer);

impl INotifyPropertyChanged for NonIntegerIndexer {
    fn property_changed(&self) -> &Event<str> {
        &self.property_changed
    }
}

impl NonIntegerIndexer {
    pub fn new() -> Rc<NonIntegerIndexer> {
        Rc::new_cyclic(|this| Self {
            this: this.clone(),
            storage: RefCell::new(HashMap::new()),
            property_changed: Event::new(),
        })
    }

    /// `this[key]` (get). Panics if the key is missing.
    pub fn get(&self, key: &str) -> String {
        INonIntegerIndexer::get(self, key)
    }

    /// `this[key] = value`.
    pub fn set(&self, key: &str, value: String) {
        INonIntegerIndexer::set(self, key, value)
    }

    /// The indexer as its contracts.
    pub fn as_non_integer_indexer_derived(&self) -> Rc<dyn INonIntegerIndexerDerived> {
        self.this.upgrade().expect("the indexer is alive while it is used")
    }

    pub fn as_non_integer_indexer(&self) -> Rc<dyn INonIntegerIndexer> {
        self.this.upgrade().expect("the indexer is alive while it is used")
    }
}

impl INonIntegerIndexer for NonIntegerIndexer {
    fn try_get(&self, key: &str) -> Option<String> {
        self.storage.borrow().get(key).cloned()
    }

    fn set(&self, key: &str, value: String) {
        self.storage.borrow_mut().insert(key.to_string(), value);
        self.property_changed.raise(INDEXER_NAME);
    }
}

impl INonIntegerIndexerDerived for NonIntegerIndexer {}

ferro_markup_type!(class NonIntegerIndexer as "TestDataContext+NonIntegerIndexer" {
    this: Rc<NonIntegerIndexer>,
    handles: [NonIntegerIndexer, Rc<NonIntegerIndexer>, Option<Rc<NonIntegerIndexer>>],
    interfaces: [Rc<dyn INonIntegerIndexerDerived>],
    constructors: [() => NonIntegerIndexer::new],
    indexers: [
        (String) -> String {
            try_get: |this: &Rc<NonIntegerIndexer>, key: String| {
                this.try_get(&key).ok_or_else(|| key_not_found(&key))
            },
            set: |this: &Rc<NonIntegerIndexer>, key: String, value: String| this.set(&key, value)
        },
    ],
    notify_property_changed: NonIntegerIndexer,
});

/// `TestDataContext.NestedGeneric<string>`.
pub struct NestedGenericString {
    value: RefCell<Option<String>>,
}

crate::test_identity_eq!(NestedGenericString);

impl NestedGenericString {
    pub fn new() -> Rc<NestedGenericString> {
        Rc::new(Self { value: RefCell::new(None) })
    }

    pub fn value(&self) -> Option<String> {
        self.value.borrow().clone()
    }

    pub fn set_value(&self, value: Option<String>) {
        self.value.replace(value);
    }
}

ferro_markup_type!(class NestedGenericString as "TestDataContext+NestedGeneric`1" {
    this: Rc<NestedGenericString>,
    handles: [NestedGenericString, Rc<NestedGenericString>, Option<Rc<NestedGenericString>>],
    generic: "TestDataContext+NestedGeneric`1" [String],
    constructors: [() => NestedGenericString::new],
    properties: [
        Value: Option<String> { get: NestedGenericString::value, set: NestedGenericString::set_value },
    ],
});

/// `ListItemCollectionView<int>`: a list of integers with a current item.
pub struct ListItemCollectionViewInt32 {
    items: RefCell<Vec<i32>>,
    current_item: Cell<i32>,
}

crate::test_identity_eq!(ListItemCollectionViewInt32);

impl ListItemCollectionViewInt32 {
    pub fn new() -> Rc<ListItemCollectionViewInt32> {
        Rc::new(Self { items: RefCell::new(Vec::new()), current_item: Cell::new(0) })
    }

    /// `List<int>.Add`.
    pub fn add(&self, item: i32) {
        self.items.borrow_mut().push(item);
    }

    /// `List<int>.Count`.
    pub fn count(&self) -> usize {
        self.items.borrow().len()
    }

    /// `this[index]`. Panics if out of range.
    pub fn get(&self, index: usize) -> i32 {
        self.items.borrow()[index]
    }

    pub fn current_item(&self) -> i32 {
        self.current_item.get()
    }

    pub fn set_current_item(&self, value: i32) {
        self.current_item.set(value);
    }
}

impl IItemsList for ListItemCollectionViewInt32 {
    fn count(&self) -> usize {
        ListItemCollectionViewInt32::count(self)
    }

    fn get_at(&self, index: usize) -> Option<BoxedValue> {
        Some(Rc::new(self.get(index)))
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

ferro_markup_type!(class ListItemCollectionViewInt32 as "ListItemCollectionView`1" {
    this: Rc<ListItemCollectionViewInt32>,
    handles: [
        ListItemCollectionViewInt32,
        Rc<ListItemCollectionViewInt32>,
        Option<Rc<ListItemCollectionViewInt32>>
    ],
    generic: "ListItemCollectionView`1" [i32],
    constructors: [() => ListItemCollectionViewInt32::new],
    properties: [
        CurrentItem: i32 {
            get: ListItemCollectionViewInt32::current_item,
            set: ListItemCollectionViewInt32::set_current_item
        },
        Count: i32 { get: |this: &Rc<ListItemCollectionViewInt32>| this.count() as i32 },
    ],
    methods: [fn Add(i32) => ListItemCollectionViewInt32::add],
});

/// `TestDataContext`.
pub struct TestDataContext {
    this: Weak<TestDataContext>,
    base: Rc<TestDataContextBaseClass>,
    bool_property: Cell<bool>,
    string_property: RefCell<Option<String>>,
    task_property: RefCell<Option<TaskValue>>,
    observable_property: RefCell<Option<ObservableValue>>,
    observable_collection_property: RefCell<Rc<BindableList<String>>>,
    array_property: RefCell<Option<Rc<BindableArray<String>>>>,
    objects_array_property: RefCell<Option<Rc<BindableArray<Option<BoxedValue>>>>>,
    list_property: RefCell<Rc<BindableList<String>>>,
    non_integer_indexer_property: RefCell<Rc<NonIntegerIndexer>>,
    nested_generic_string: RefCell<Option<Rc<NestedGenericString>>>,
    generic_property: Rc<ListItemCollectionViewInt32>,
    decimal_value: Cell<Decimal>,
}

crate::test_identity_eq!(TestDataContext);

impl TestDataContext {
    pub fn new() -> Rc<TestDataContext> {
        Rc::new_cyclic(|this| Self {
            this: this.clone(),
            base: TestDataContextBaseClass::new(),
            bool_property: Cell::new(false),
            string_property: RefCell::new(None),
            task_property: RefCell::new(None),
            observable_property: RefCell::new(None),
            observable_collection_property: RefCell::new(BindableList::new(Vec::new())),
            array_property: RefCell::new(None),
            objects_array_property: RefCell::new(None),
            list_property: RefCell::new(BindableList::new(Vec::new())),
            non_integer_indexer_property: RefCell::new(NonIntegerIndexer::new()),
            nested_generic_string: RefCell::new(None),
            generic_property: ListItemCollectionViewInt32::new(),
            decimal_value: Cell::new(Self::expected_decimal()),
        })
    }

    fn this(&self) -> Rc<TestDataContext> {
        self.this.upgrade().expect("the data context is alive while it is used")
    }

    /// The part of the object that is the base class.
    pub fn base(&self) -> &Rc<TestDataContextBaseClass> {
        &self.base
    }

    /// The object as its contracts.
    pub fn as_has_property(&self) -> Rc<dyn IHasProperty> {
        self.this()
    }

    pub fn as_has_property_derived(&self) -> Rc<dyn IHasPropertyDerived> {
        self.this()
    }

    pub fn as_has_explicit_property(&self) -> Rc<dyn IHasExplicitProperty> {
        self.this()
    }

    pub fn bool_property(&self) -> bool {
        self.bool_property.get()
    }

    pub fn set_bool_property(&self, value: bool) {
        self.bool_property.set(value);
    }

    pub fn string_property(&self) -> Option<String> {
        self.string_property.borrow().clone()
    }

    pub fn set_string_property(&self, value: Option<String>) {
        self.string_property.replace(value);
    }

    /// `Task<string>? TaskProperty`.
    pub fn task_property(&self) -> Option<TaskValue> {
        self.task_property.borrow().clone()
    }

    pub fn set_task_property(&self, value: Option<TaskValue>) {
        self.task_property.replace(value);
    }

    /// `IObservable<string>? ObservableProperty`.
    pub fn observable_property(&self) -> Option<ObservableValue> {
        self.observable_property.borrow().clone()
    }

    pub fn set_observable_property(&self, value: Option<ObservableValue>) {
        self.observable_property.replace(value);
    }

    /// `ObservableCollection<string> ObservableCollectionProperty`.
    pub fn observable_collection_property(&self) -> Rc<BindableList<String>> {
        self.observable_collection_property.borrow().clone()
    }

    pub fn set_observable_collection_property(&self, value: Rc<BindableList<String>>) {
        self.observable_collection_property.replace(value);
    }

    /// `string[]? ArrayProperty`.
    pub fn array_property(&self) -> Option<Rc<BindableArray<String>>> {
        self.array_property.borrow().clone()
    }

    pub fn set_array_property(&self, value: Option<Rc<BindableArray<String>>>) {
        self.array_property.replace(value);
    }

    /// `object[]? ObjectsArrayProperty`.
    pub fn objects_array_property(&self) -> Option<Rc<BindableArray<Option<BoxedValue>>>> {
        self.objects_array_property.borrow().clone()
    }

    pub fn set_objects_array_property(&self, value: Option<Rc<BindableArray<Option<BoxedValue>>>>) {
        self.objects_array_property.replace(value);
    }

    /// `List<string> ListProperty`.
    pub fn list_property(&self) -> Rc<BindableList<String>> {
        self.list_property.borrow().clone()
    }

    pub fn set_list_property(&self, value: Rc<BindableList<String>>) {
        self.list_property.replace(value);
    }

    pub fn non_integer_indexer_property(&self) -> Rc<NonIntegerIndexer> {
        self.non_integer_indexer_property.borrow().clone()
    }

    pub fn set_non_integer_indexer_property(&self, value: Rc<NonIntegerIndexer>) {
        self.non_integer_indexer_property.replace(value);
    }

    pub fn non_integer_indexer_interface_property(&self) -> Rc<dyn INonIntegerIndexerDerived> {
        self.non_integer_indexer_property()
    }

    /// `NestedGeneric<string>? NestedGenericString { get; init; }`.
    pub fn nested_generic_string(&self) -> Option<Rc<NestedGenericString>> {
        self.nested_generic_string.borrow().clone()
    }

    pub fn set_nested_generic_string(&self, value: Option<Rc<NestedGenericString>>) {
        self.nested_generic_string.replace(value);
    }

    /// `ExplicitProperty` of the class itself (the explicit implementation
    /// of the interface is [`IHasExplicitProperty::explicit_property`]).
    pub fn explicit_property(&self) -> String {
        "Bye".to_string()
    }

    /// `static string StaticProperty`.
    pub fn static_property() -> String {
        "World".to_string()
    }

    /// `const decimal ExpectedDecimal = 15.756m`.
    pub fn expected_decimal() -> Decimal {
        Decimal::from_parts(15756, 3, false)
    }

    pub fn decimal_value(&self) -> Decimal {
        self.decimal_value.get()
    }

    pub fn set_decimal_value(&self, value: Decimal) {
        self.decimal_value.set(value);
    }

    pub fn generic_property(&self) -> Rc<ListItemCollectionViewInt32> {
        self.generic_property.clone()
    }
}

impl IHasProperty for TestDataContext {
    fn string_property(&self) -> Option<String> {
        TestDataContext::string_property(self)
    }

    fn set_string_property(&self, value: Option<String>) {
        TestDataContext::set_string_property(self, value)
    }
}

impl IHasPropertyDerived for TestDataContext {}

impl IHasExplicitProperty for TestDataContext {
    fn explicit_property(&self) -> String {
        "Hello".to_string()
    }
}

ferro_markup_type!(class TestDataContext {
    this: Rc<TestDataContext>,
    handles: [TestDataContext, Rc<TestDataContext>, Option<Rc<TestDataContext>>],
    base: Rc<TestDataContextBaseClass>,
    interfaces: [Rc<dyn IHasPropertyDerived>, Rc<dyn IHasProperty>, Rc<dyn IHasExplicitProperty>],
    constructors: [() => TestDataContext::new],
    properties: [
        BoolProperty: bool { get: TestDataContext::bool_property, set: TestDataContext::set_bool_property },
        StringProperty: Option<String> {
            get: TestDataContext::string_property,
            set: TestDataContext::set_string_property
        },
        TaskProperty: Option<TaskValue> {
            get: TestDataContext::task_property,
            set: TestDataContext::set_task_property
        },
        ObservableProperty: Option<ObservableValue> {
            get: TestDataContext::observable_property,
            set: TestDataContext::set_observable_property
        },
        ObservableCollectionProperty: Rc<BindableList<String>> {
            get: TestDataContext::observable_collection_property,
            set: TestDataContext::set_observable_collection_property
        },
        ArrayProperty: Option<Rc<BindableArray<String>>> {
            get: TestDataContext::array_property,
            set: TestDataContext::set_array_property
        },
        ObjectsArrayProperty: Option<Rc<BindableArray<Option<BoxedValue>>>> {
            get: TestDataContext::objects_array_property,
            set: TestDataContext::set_objects_array_property
        },
        ListProperty: Rc<BindableList<String>> {
            get: TestDataContext::list_property,
            set: TestDataContext::set_list_property
        },
        NonIntegerIndexerProperty: Rc<NonIntegerIndexer> {
            get: TestDataContext::non_integer_indexer_property,
            set: TestDataContext::set_non_integer_indexer_property
        },
        NonIntegerIndexerInterfaceProperty: Rc<dyn INonIntegerIndexerDerived> {
            get: TestDataContext::non_integer_indexer_interface_property
        },
        NestedGenericString: Option<Rc<NestedGenericString>> {
            get: TestDataContext::nested_generic_string,
            set: TestDataContext::set_nested_generic_string
        },
        ExplicitProperty: String { get: TestDataContext::explicit_property },
        GenericProperty: Rc<ListItemCollectionViewInt32> { get: TestDataContext::generic_property },
        DecimalValue: Decimal { get: TestDataContext::decimal_value, set: TestDataContext::set_decimal_value },
    ],
    static_properties: [StaticProperty: String { get: TestDataContext::static_property }],
});

// The instantiations of generic types of the runtime library the properties
// of the test types use. The run-time type system knows an instantiation
// only if it is declared, and a declaration belongs to a type of this crate:
// the unit types below only carry the declarations.

/// The message of the exception a list throws for an index outside of it.
const INDEX_OUT_OF_RANGE: &str =
    "Index was out of range. Must be non-negative and less than the size of the collection. (Parameter 'index')";

/// `list[index]` (get).
fn list_item<T: ferroui_base::PropertyValue>(list: &Rc<BindableList<T>>, index: i32) -> Result<T, String> {
    usize::try_from(index)
        .ok()
        .and_then(|index| list.items().try_get(index))
        .ok_or_else(|| INDEX_OUT_OF_RANGE.to_string())
}

/// `list[index] = value`.
fn set_list_item<T: ferroui_base::PropertyValue>(list: &Rc<BindableList<T>>, index: i32, value: T) -> Result<(), String> {
    match usize::try_from(index) {
        Ok(index) if index < list.items().count() => {
            list.items().set(index, value);
            Ok(())
        }
        _ => Err(INDEX_OUT_OF_RANGE.to_string()),
    }
}

/// `List<string>` (also what `ObservableCollection<string>` is declared as:
/// both are a `BindableList<String>`).
pub struct ListOfString;

ferro_markup_type!(class ListOfString as "List`1" {
    namespace: "System.Collections.Generic",
    this: Rc<BindableList<String>>,
    handles: [BindableList<String>, Rc<BindableList<String>>, Option<Rc<BindableList<String>>>],
    generic: "List`1" [String],
    // The list of the port is a notifying list (`ObservableCollection<T>`).
    interfaces: [Rc<dyn INotifyCollectionChanged>],
    constructors: [() => || BindableList::<String>::new(Vec::new())],
    indexers: [
        (i32) -> String {
            try_get: |list: &Rc<BindableList<String>>, index: i32| list_item(list, index),
            try_set: |list: &Rc<BindableList<String>>, index: i32, value: String| set_list_item(list, index, value)
        },
    ],
    methods: [fn Add(String) => |list: &Rc<BindableList<String>>, item: String| list.items().add(item)],
});

/// `ObservableCollection<TestData>`, declared as `List<TestData>`.
pub struct ListOfTestData;

ferro_markup_type!(class ListOfTestData as "List`1" {
    namespace: "System.Collections.Generic",
    this: Rc<BindableList<Rc<TestData>>>,
    handles: [
        BindableList<Rc<TestData>>,
        Rc<BindableList<Rc<TestData>>>,
        Option<Rc<BindableList<Rc<TestData>>>>
    ],
    generic: "List`1" [Rc<TestData>],
    interfaces: [Rc<dyn INotifyCollectionChanged>],
    constructors: [() => || BindableList::<Rc<TestData>>::new(Vec::new())],
    indexers: [
        (i32) -> Rc<TestData> {
            try_get: |list: &Rc<BindableList<Rc<TestData>>>, index: i32| list_item(list, index),
            try_set: |list: &Rc<BindableList<Rc<TestData>>>, index: i32, value: Rc<TestData>| {
                set_list_item(list, index, value)
            }
        },
    ],
    methods: [
        fn Add(Rc<TestData>) => |list: &Rc<BindableList<Rc<TestData>>>, item: Rc<TestData>| list.items().add(item),
    ],
});

/// `Task<string>`: the type of the task values of the test types.
pub struct TaskOfString;

ferro_markup_type!(class TaskOfString as "Task`1" {
    namespace: "System.Threading.Tasks",
    this: TaskValue,
    handles: [TaskValue, Option<TaskValue>],
    generic: "Task`1" [String],
});

/// `IObservable<string>`: the type of the observable values of the test
/// types.
pub struct ObservableOfString;

ferro_markup_type!(interface ObservableOfString as "IObservable`1" {
    namespace: "System",
    this: ObservableValue,
    handles: [ObservableValue, Option<ObservableValue>],
    generic: "IObservable`1" [String],
});

/// `MethodDataContext`.
pub struct MethodDataContext;

crate::test_identity_eq!(MethodDataContext);

impl MethodDataContext {
    pub fn new() -> Rc<MethodDataContext> {
        Rc::new(Self)
    }

    pub fn action(&self) {}

    pub fn func(&self) -> Option<BoxedValue> {
        Some(Rc::new(1_i32))
    }

    pub fn func2(&self, i: Option<BoxedValue>) -> Option<BoxedValue> {
        i
    }

    pub fn custom_delegate_type_void(&self, _i: Option<BoxedValue>) {}

    pub fn custom_delegate_type_int(&self, i: Option<BoxedValue>) -> Option<BoxedValue> {
        i
    }
}

ferro_markup_type!(class MethodDataContext {
    this: Rc<MethodDataContext>,
    handles: [MethodDataContext, Rc<MethodDataContext>, Option<Rc<MethodDataContext>>],
    constructors: [() => MethodDataContext::new],
    methods: [
        fn Action() => MethodDataContext::action,
        fn Func() -> Option<BoxedValue> => MethodDataContext::func,
        fn Func2(Option<BoxedValue>) -> Option<BoxedValue> => MethodDataContext::func2,
        fn CustomDelegateTypeVoid(Option<BoxedValue>) => MethodDataContext::custom_delegate_type_void,
        fn CustomDelegateTypeInt(Option<BoxedValue>) -> Option<BoxedValue> =>
            MethodDataContext::custom_delegate_type_int,
    ],
});

/// `MethodAsCommandDataContextBase`: the virtual methods do nothing.
pub struct MethodAsCommandDataContextBase;

crate::test_identity_eq!(MethodAsCommandDataContextBase);

impl MethodAsCommandDataContextBase {
    pub fn new() -> Rc<MethodAsCommandDataContextBase> {
        Rc::new(Self)
    }

    pub fn virtual_object_method(&self, _i: Option<BoxedValue>) {}

    pub fn virtual_int32_method(&self, _i: i32) {}

    pub fn virtual_string_method(&self, _i: Option<String>) {}

    pub fn method_with_new_slot(&self, _i: i32) {}
}

ferro_markup_type!(class MethodAsCommandDataContextBase {
    this: Rc<MethodAsCommandDataContextBase>,
    handles: [
        MethodAsCommandDataContextBase,
        Rc<MethodAsCommandDataContextBase>,
        Option<Rc<MethodAsCommandDataContextBase>>
    ],
    constructors: [() => MethodAsCommandDataContextBase::new],
    methods: [
        fn VirtualObjectMethod(Option<BoxedValue>) => MethodAsCommandDataContextBase::virtual_object_method,
        fn VirtualInt32Method(i32) => MethodAsCommandDataContextBase::virtual_int32_method,
        fn VirtualStringMethod(Option<String>) => MethodAsCommandDataContextBase::virtual_string_method,
        fn MethodWithNewSlot(i32) => MethodAsCommandDataContextBase::method_with_new_slot,
    ],
});

/// `MethodAsCommandDataContext`: every method records that it was called in
/// `Value`. The overloads of the original are the methods with a suffix
/// that names the parameter type (`_int32`, `_string`, `_object`).
pub struct MethodAsCommandDataContext {
    base: Rc<MethodAsCommandDataContextBase>,
    value: RefCell<String>,
    parameter: RefCell<Option<BoxedValue>>,
    property_changed: Event<str>,
}

crate::test_identity_eq!(MethodAsCommandDataContext);

impl INotifyPropertyChanged for MethodAsCommandDataContext {
    fn property_changed(&self) -> &Event<str> {
        &self.property_changed
    }
}

/// The exception of the overloads that must never be called.
fn should_not_be_called(method: &str) -> Result<(), String> {
    Err(format!("{method} should not be called"))
}

impl MethodAsCommandDataContext {
    pub fn new() -> Rc<MethodAsCommandDataContext> {
        Rc::new(Self {
            base: MethodAsCommandDataContextBase::new(),
            value: RefCell::new("Not called".to_string()),
            parameter: RefCell::new(None),
            property_changed: Event::new(),
        })
    }

    /// The part of the object that is the base class.
    pub fn base(&self) -> &Rc<MethodAsCommandDataContextBase> {
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
        should_not_be_called("MethodWithOverloads3")
    }

    pub fn method_with_overloads3_string(&self, _a: Option<String>, _b: Option<String>) -> Result<(), String> {
        should_not_be_called("MethodWithOverloads3")
    }

    pub fn method_with_overloads4_int32(&self, _a: i32, _b: i32) -> Result<(), String> {
        should_not_be_called("MethodWithOverloads4")
    }

    pub fn method_with_overloads4_string(&self, _a: Option<String>, _b: Option<String>) -> Result<(), String> {
        should_not_be_called("MethodWithOverloads4")
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

    /// `Value` (its setter is private).
    pub fn value(&self) -> String {
        self.value.borrow().clone()
    }

    pub fn parameter(&self) -> Option<BoxedValue> {
        self.parameter.borrow().clone()
    }

    pub fn set_parameter(&self, value: Option<BoxedValue>) {
        if ValueTypes::identity_equals(self.parameter.borrow().as_ref(), value.as_ref()) {
            return;
        }
        self.parameter.replace(value);
        self.property_changed.raise("Parameter");
    }

    pub fn do_(&self, parameter: Option<BoxedValue>) {
        self.set_value(format!("Do {}", display(&parameter)));
    }

    /// `[DependsOn(nameof(Parameter))] bool CanDo(object parameter)`.
    pub fn can_do(&self, _parameter: Option<BoxedValue>) -> bool {
        self.parameter.borrow().is_some()
    }
}

ferro_markup_type!(class MethodAsCommandDataContext {
    this: Rc<MethodAsCommandDataContext>,
    handles: [MethodAsCommandDataContext, Rc<MethodAsCommandDataContext>, Option<Rc<MethodAsCommandDataContext>>],
    base: Rc<MethodAsCommandDataContextBase>,
    constructors: [() => MethodAsCommandDataContext::new],
    properties: [
        Value: String { get: MethodAsCommandDataContext::value },
        Parameter: Option<BoxedValue> {
            get: MethodAsCommandDataContext::parameter,
            set: MethodAsCommandDataContext::set_parameter
        },
    ],
    methods: [
        fn Method() => MethodAsCommandDataContext::method,
        fn ObjectMethod(Option<BoxedValue>) => MethodAsCommandDataContext::object_method,
        fn Int32Method(i32) => MethodAsCommandDataContext::int32_method,
        fn StringMethod(Option<String>) => MethodAsCommandDataContext::string_method,
        fn MethodWithOverloads() => MethodAsCommandDataContext::method_with_overloads,
        fn MethodWithOverloads(i32) => MethodAsCommandDataContext::method_with_overloads_int32,
        fn MethodWithOverloads(Option<String>) => MethodAsCommandDataContext::method_with_overloads_string,
        fn MethodWithOverloads(Option<BoxedValue>) => MethodAsCommandDataContext::method_with_overloads_object,
        fn MethodWithOverloads2() => MethodAsCommandDataContext::method_with_overloads2,
        fn MethodWithOverloads2(i32) => MethodAsCommandDataContext::method_with_overloads2_int32,
        fn MethodWithOverloads2(Option<String>) => MethodAsCommandDataContext::method_with_overloads2_string,
        fn MethodWithOverloads3() => MethodAsCommandDataContext::method_with_overloads3,
        try fn MethodWithOverloads3(i32, i32) => MethodAsCommandDataContext::method_with_overloads3_int32,
        try fn MethodWithOverloads3(Option<String>, Option<String>) => MethodAsCommandDataContext::method_with_overloads3_string,
        try fn MethodWithOverloads4(i32, i32) => MethodAsCommandDataContext::method_with_overloads4_int32,
        try fn MethodWithOverloads4(Option<String>, Option<String>) => MethodAsCommandDataContext::method_with_overloads4_string,
        fn VirtualObjectMethod(Option<BoxedValue>) => MethodAsCommandDataContext::virtual_object_method,
        fn VirtualInt32Method(i32) => MethodAsCommandDataContext::virtual_int32_method,
        fn VirtualStringMethod(Option<String>) => MethodAsCommandDataContext::virtual_string_method,
        fn MethodWithNewSlot(i32) => MethodAsCommandDataContext::method_with_new_slot,
        fn Do(Option<BoxedValue>) => MethodAsCommandDataContext::do_,
        fn CanDo(Option<BoxedValue>) -> bool => MethodAsCommandDataContext::can_do [DependsOn("Parameter")],
    ],
    notify_property_changed: MethodAsCommandDataContext,
});

/// `CustomDataTemplate`: a data template whose data type property has
/// another name than the one of the data templates of the framework.
pub struct CustomDataTemplate {
    this: Weak<CustomDataTemplate>,
    fancy_data_type: RefCell<Option<ValueType>>,
    content: RefCell<Option<BoxedValue>>,
}

crate::test_identity_eq!(CustomDataTemplate);

impl CustomDataTemplate {
    pub fn new() -> Rc<CustomDataTemplate> {
        Rc::new_cyclic(|this| Self {
            this: this.clone(),
            fancy_data_type: RefCell::new(None),
            content: RefCell::new(None),
        })
    }

    pub fn fancy_data_type(&self) -> Option<ValueType> {
        *self.fancy_data_type.borrow()
    }

    pub fn set_fancy_data_type(&self, value: Option<ValueType>) {
        self.fancy_data_type.replace(value);
    }

    pub fn content(&self) -> Option<BoxedValue> {
        self.content.borrow().clone()
    }

    pub fn set_content(&self, value: Option<BoxedValue>) {
        self.content.replace(value);
    }

    /// `FancyDataType?.IsInstanceOfType(data) ?? true`.
    pub fn match_(&self, data: Option<&BoxedValue>) -> bool {
        match (self.fancy_data_type(), data) {
            (None, _) => true,
            (Some(data_type), Some(data)) => ValueTypes::is_assignable(ValueType::of_value(&**data), data_type),
            (Some(_), None) => false,
        }
    }

    /// `TemplateContent.Load(Content)?.Result`.
    pub fn build(&self, _data: Option<&BoxedValue>) -> Option<Ref<Control>> {
        TemplateContent::load(self.content().as_ref()).map(|result| result.result().clone())
    }

    /// The template as the data template contract.
    pub fn as_data_template(&self) -> Rc<dyn IDataTemplate> {
        self.this.upgrade().expect("the template is alive while it is used")
    }
}

impl ITemplateWithParam<Option<BoxedValue>, Option<Ref<Control>>> for CustomDataTemplate {
    fn build(&self, param: &Option<BoxedValue>) -> Option<Ref<Control>> {
        CustomDataTemplate::build(self, param.as_ref())
    }
}

impl IDataTemplate for CustomDataTemplate {
    fn match_(&self, data: Option<&BoxedValue>) -> bool {
        CustomDataTemplate::match_(self, data)
    }

    fn as_any(&self) -> Option<&dyn std::any::Any> {
        Some(self)
    }
}

ferro_markup_type!(class CustomDataTemplate {
    this: Rc<CustomDataTemplate>,
    handles: [CustomDataTemplate, Rc<CustomDataTemplate>, Option<Rc<CustomDataTemplate>>],
    interfaces: [Rc<dyn IDataTemplate>],
    constructors: [() => CustomDataTemplate::new],
    content: Content,
    properties: [
        FancyDataType: Option<ValueType> {
            get: CustomDataTemplate::fancy_data_type,
            set: CustomDataTemplate::set_fancy_data_type
        } [DataType],
        Content: Option<BoxedValue> {
            get: CustomDataTemplate::content,
            set: CustomDataTemplate::set_content
        } [TemplateContent],
    ],
    methods: [
        fn Match(Option<BoxedValue>) -> bool => |this: &Rc<CustomDataTemplate>, data: Option<BoxedValue>| {
            this.match_(data.as_ref())
        },
        try fn Build(Option<BoxedValue>) -> Option<Ref<Control>> =>
            |this: &Rc<CustomDataTemplate>, _data: Option<BoxedValue>| {
                TemplateContent::try_load_as::<Ref<Control>>(this.content().as_ref())
                    .map(|result| result.map(|result| result.result().clone()))
                    .map_err(|error| error.message().to_string())
            },
    ],
});

/// `CustomDataTemplateInherit`: the data type property is inherited.
pub struct CustomDataTemplateInherit {
    base: Rc<CustomDataTemplate>,
}

crate::test_identity_eq!(CustomDataTemplateInherit);

impl CustomDataTemplateInherit {
    pub fn new() -> Rc<CustomDataTemplateInherit> {
        Rc::new(Self { base: CustomDataTemplate::new() })
    }

    /// The part of the object that is the base class.
    pub fn base(&self) -> &Rc<CustomDataTemplate> {
        &self.base
    }
}

ferro_markup_type!(class CustomDataTemplateInherit {
    this: Rc<CustomDataTemplateInherit>,
    handles: [CustomDataTemplateInherit, Rc<CustomDataTemplateInherit>, Option<Rc<CustomDataTemplateInherit>>],
    base: Rc<CustomDataTemplate>,
    constructors: [() => CustomDataTemplateInherit::new],
});

/// `AssignBindingControl`: a control with a plain property that is assigned
/// the binding given for it in markup.
#[repr(C)]
pub struct AssignBindingControl {
    base: Control,
    x: RefCell<Option<Rc<dyn BindingBase>>>,
}

ferro_class!(AssignBindingControl: Control);
ferro_impl_classes!(
    AssignBindingControl: FerroObjectImpl,
    StyledElementImpl,
    VisualImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl
);
ferro_class_info!(AssignBindingControl {
    new: AssignBindingControl::new,
    markup: {
        properties: [
            X: Option<Rc<dyn BindingBase>> {
                get: |this: &Ref<AssignBindingControl>| this.x(),
                set: |this: &Ref<AssignBindingControl>, value: Option<Rc<dyn BindingBase>>| this.set_x(value)
            } [AssignBinding],
        ],
    },
});

impl AssignBindingControl {
    pub fn construct() -> Self {
        Self { base: Control::construct(), x: RefCell::new(None) }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    pub fn x(&self) -> Option<Rc<dyn BindingBase>> {
        self.x.borrow().clone()
    }

    pub fn set_x(&self, value: Option<Rc<dyn BindingBase>>) {
        self.x.replace(value);
    }
}

/// `DataGridLikeColumn`: its binding and its template inherit their data
/// type from the items of the control that owns the column.
pub struct DataGridLikeColumn {
    binding: RefCell<Option<Rc<dyn BindingBase>>>,
    template: RefCell<Option<Rc<dyn IDataTemplate>>>,
}

crate::test_identity_eq!(DataGridLikeColumn);

impl DataGridLikeColumn {
    pub fn new() -> Rc<DataGridLikeColumn> {
        Rc::new(Self { binding: RefCell::new(None), template: RefCell::new(None) })
    }

    pub fn binding(&self) -> Option<Rc<dyn BindingBase>> {
        self.binding.borrow().clone()
    }

    pub fn set_binding(&self, value: Option<Rc<dyn BindingBase>>) {
        self.binding.replace(value);
    }

    pub fn template(&self) -> Option<Rc<dyn IDataTemplate>> {
        self.template.borrow().clone()
    }

    pub fn set_template(&self, value: Option<Rc<dyn IDataTemplate>>) {
        self.template.replace(value);
    }
}

ferro_markup_type!(class DataGridLikeColumn {
    this: Rc<DataGridLikeColumn>,
    handles: [DataGridLikeColumn, Rc<DataGridLikeColumn>, Option<Rc<DataGridLikeColumn>>],
    constructors: [() => DataGridLikeColumn::new],
    properties: [
        Binding: Option<Rc<dyn BindingBase>> {
            get: DataGridLikeColumn::binding,
            set: DataGridLikeColumn::set_binding
        } [AssignBinding, InheritDataTypeFromItems("Items", AncestorType = type(Ref<DataGridLikeControl>))],
        Template: Option<Rc<dyn IDataTemplate>> {
            get: DataGridLikeColumn::template,
            set: DataGridLikeColumn::set_template
        } [InheritDataTypeFromItems("Items", AncestorType = type(Ref<DataGridLikeControl>))],
    ],
});

/// The columns of a [`DataGridLikeControl`]: a `FerroList<DataGridLikeColumn>`.
pub struct DataGridLikeColumns {
    items: RefCell<Vec<Rc<DataGridLikeColumn>>>,
}

crate::test_identity_eq!(DataGridLikeColumns);

impl DataGridLikeColumns {
    pub fn new() -> Rc<DataGridLikeColumns> {
        Rc::new(Self { items: RefCell::new(Vec::new()) })
    }

    pub fn add(&self, item: Rc<DataGridLikeColumn>) {
        self.items.borrow_mut().push(item);
    }

    pub fn count(&self) -> usize {
        self.items.borrow().len()
    }

    /// `this[index]`. Panics if out of range.
    pub fn get(&self, index: usize) -> Rc<DataGridLikeColumn> {
        self.items.borrow()[index].clone()
    }

    /// `Enumerable.Single()`. Panics unless there is exactly one column.
    pub fn single(&self) -> Rc<DataGridLikeColumn> {
        match self.count() {
            1 => self.get(0),
            0 => panic!("Sequence contains no elements"),
            _ => panic!("Sequence contains more than one element"),
        }
    }
}

ferro_markup_type!(class DataGridLikeColumns as "FerroList`1" {
    namespace: "FerroUI.Collections",
    this: Rc<DataGridLikeColumns>,
    handles: [DataGridLikeColumns, Rc<DataGridLikeColumns>, Option<Rc<DataGridLikeColumns>>],
    generic: "FerroList`1" [Rc<DataGridLikeColumn>],
    constructors: [() => DataGridLikeColumns::new],
    properties: [
        Count: i32 { get: |this: &Rc<DataGridLikeColumns>| this.count() as i32 },
    ],
    methods: [fn Add(Rc<DataGridLikeColumn>) => DataGridLikeColumns::add],
});

/// `DataGridLikeControl`: a control with items and columns, as a data grid
/// has them.
#[repr(C)]
pub struct DataGridLikeControl {
    base: Control,
    items: RefCell<Option<ItemsSource>>,
    columns: Rc<DataGridLikeColumns>,
}

ferro_class!(DataGridLikeControl: Control);
ferro_impl_classes!(
    DataGridLikeControl: FerroObjectImpl,
    StyledElementImpl,
    VisualImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl
);
ferro_class_info!(DataGridLikeControl {
    new: DataGridLikeControl::new,
    markup: {
        properties: [
            Columns: Rc<DataGridLikeColumns> { get: |this: &Ref<DataGridLikeControl>| this.columns() },
        ],
    },
});

ferro_properties! {
    impl DataGridLikeControl {
        pub fn items_property() -> DirectProperty<DataGridLikeControl, Option<ItemsSource>> {
            FerroProperty::register_direct::<DataGridLikeControl, _>(
                "Items",
                |o| o.items(),
                Some(|o: &DataGridLikeControl, v| o.set_items(v)),
                None,
            )
        }
    }
}

impl DataGridLikeControl {
    pub fn construct() -> Self {
        Self { base: Control::construct(), items: RefCell::new(None), columns: DataGridLikeColumns::new() }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// `IEnumerable? Items`.
    pub fn items(&self) -> Option<ItemsSource> {
        self.items.borrow().clone()
    }

    pub fn set_items(&self, value: Option<ItemsSource>) {
        self.set_and_raise(Self::items_property(), &self.items, value);
    }

    pub fn columns(&self) -> Rc<DataGridLikeColumns> {
        self.columns.clone()
    }
}

/// `DataGridLikeControlInheritor`.
#[repr(C)]
pub struct DataGridLikeControlInheritor {
    base: DataGridLikeControl,
}

ferro_class!(DataGridLikeControlInheritor: DataGridLikeControl);
ferro_impl_classes!(
    DataGridLikeControlInheritor: FerroObjectImpl,
    StyledElementImpl,
    VisualImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl
);
ferro_class_info!(DataGridLikeControlInheritor { new: DataGridLikeControlInheritor::new });

impl DataGridLikeControlInheritor {
    pub fn construct() -> Self {
        Self { base: DataGridLikeControl::construct() }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
}

/// `ImplicitConvertible`: a value with an implicit conversion to a color.
pub struct ImplicitConvertible {
    value: String,
}

crate::test_identity_eq!(ImplicitConvertible);

impl ImplicitConvertible {
    pub fn new(value: String) -> Rc<ImplicitConvertible> {
        Rc::new(Self { value })
    }

    pub fn value(&self) -> String {
        self.value.clone()
    }

    /// `implicit operator Color(ImplicitConvertible value)`: an error is the
    /// exception of the parse.
    pub fn to_color(&self) -> Result<Color, String> {
        Color::parse(&self.value).map_err(|error| error.to_string())
    }
}

ferro_markup_type!(class ImplicitConvertible {
    this: Rc<ImplicitConvertible>,
    handles: [ImplicitConvertible, Rc<ImplicitConvertible>, Option<Rc<ImplicitConvertible>>],
    constructors: [(String) => ImplicitConvertible::new],
    properties: [
        Value: String { get: ImplicitConvertible::value },
    ],
    methods: [
        static try fn op_Implicit(Rc<ImplicitConvertible>) -> Color =>
            |value: Rc<ImplicitConvertible>| value.to_color(),
    ],
});

/// The test types of this file.
pub(crate) const MODULE: TypeModule = TypeModule {
    types: &[AssignBindingControl::TYPE, DataGridLikeControl::TYPE, DataGridLikeControlInheritor::TYPE],
    markup_types: &[
        <dyn INonIntegerIndexer as MarkupTyped>::MARKUP,
        <dyn INonIntegerIndexerDerived as MarkupTyped>::MARKUP,
        <dyn IHasProperty as MarkupTyped>::MARKUP,
        <dyn IHasPropertyDerived as MarkupTyped>::MARKUP,
        <dyn IHasExplicitProperty as MarkupTyped>::MARKUP,
        <AppendConverter as MarkupTyped>::MARKUP,
        <TestData as MarkupTyped>::MARKUP,
        <OuterClass as MarkupTyped>::MARKUP,
        <NestedClass as MarkupTyped>::MARKUP,
        <TestDataContextBaseClass as MarkupTyped>::MARKUP,
        <TestItemsCollectionDataContext as MarkupTyped>::MARKUP,
        <NonIntegerIndexer as MarkupTyped>::MARKUP,
        <NestedGenericString as MarkupTyped>::MARKUP,
        <ListItemCollectionViewInt32 as MarkupTyped>::MARKUP,
        <TestDataContext as MarkupTyped>::MARKUP,
        <ListOfString as MarkupTyped>::MARKUP,
        <ListOfTestData as MarkupTyped>::MARKUP,
        <TaskOfString as MarkupTyped>::MARKUP,
        <ObservableOfString as MarkupTyped>::MARKUP,
        <MethodDataContext as MarkupTyped>::MARKUP,
        <MethodAsCommandDataContextBase as MarkupTyped>::MARKUP,
        <MethodAsCommandDataContext as MarkupTyped>::MARKUP,
        <CustomDataTemplate as MarkupTyped>::MARKUP,
        <CustomDataTemplateInherit as MarkupTyped>::MARKUP,
        <DataGridLikeColumn as MarkupTyped>::MARKUP,
        <DataGridLikeColumns as MarkupTyped>::MARKUP,
        <ImplicitConvertible as MarkupTyped>::MARKUP,
    ],
    value_types: || {
        ValueTypes::register_reference::<AppendConverter>();
        ValueTypes::register_reference::<TestData>();
        ValueTypes::register_reference::<OuterClass>();
        ValueTypes::register_reference::<NestedClass>();
        ValueTypes::register_reference::<TestDataContextBaseClass>();
        ValueTypes::register_reference::<TestItemsCollectionDataContext>();
        ValueTypes::register_reference::<NonIntegerIndexer>();
        ValueTypes::register_reference::<NestedGenericString>();
        ValueTypes::register_reference::<ListItemCollectionViewInt32>();
        ValueTypes::register_reference::<TestDataContext>();
        ValueTypes::register_reference::<BindableList<String>>();
        ValueTypes::register_reference::<BindableList<Rc<TestData>>>();
        ValueTypes::register_reference::<BindableArray<String>>();
        ValueTypes::register_reference::<BindableArray<Option<BoxedValue>>>();
        // `string[]` and `object[]`: the arrays the array-element nodes of binding paths index.
        ferroui_markup_xaml_loader::runtime::type_system::register_bindable_array::<String>();
        ferroui_markup_xaml_loader::runtime::type_system::register_bindable_array::<Option<BoxedValue>>();
        ValueTypes::register_reference::<MethodDataContext>();
        ValueTypes::register_reference::<MethodAsCommandDataContextBase>();
        ValueTypes::register_reference::<MethodAsCommandDataContext>();
        ValueTypes::register_reference::<CustomDataTemplate>();
        ValueTypes::register_reference::<CustomDataTemplateInherit>();
        ValueTypes::register_reference::<DataGridLikeColumn>();
        ValueTypes::register_reference::<DataGridLikeColumns>();
        ValueTypes::register_reference::<ImplicitConvertible>();

        ValueTypes::register_nullable::<TaskValue>();
        ValueTypes::register_nullable::<ObservableValue>();
        ValueTypes::register_nullable::<Rc<dyn INonIntegerIndexer>>();
        ValueTypes::register_nullable::<Rc<dyn INonIntegerIndexerDerived>>();
        ValueTypes::register_nullable::<Rc<dyn IHasProperty>>();
        ValueTypes::register_nullable::<Rc<dyn IHasPropertyDerived>>();
        ValueTypes::register_nullable::<Rc<dyn IHasExplicitProperty>>();

        // Base classes.
        ValueTypes::register_cast::<TestItemsCollectionDataContext, Rc<TestDataContextBaseClass>>(|data_context| {
            data_context.base().clone()
        });
        ValueTypes::register_cast::<TestDataContext, Rc<TestDataContextBaseClass>>(|data_context| {
            data_context.base().clone()
        });
        ValueTypes::register_cast::<MethodAsCommandDataContext, Rc<MethodAsCommandDataContextBase>>(
            |data_context| data_context.base().clone(),
        );
        ValueTypes::register_cast::<CustomDataTemplateInherit, Rc<CustomDataTemplate>>(|template| {
            template.base().clone()
        });

        // Contracts.
        ValueTypes::register_cast::<AppendConverter, Rc<dyn IValueConverter>>(AppendConverter::as_value_converter);
        ValueTypes::register_cast::<TestDataContext, Rc<dyn IHasProperty>>(TestDataContext::as_has_property);
        ValueTypes::register_cast::<TestDataContext, Rc<dyn IHasPropertyDerived>>(
            TestDataContext::as_has_property_derived,
        );
        ValueTypes::register_cast::<TestDataContext, Rc<dyn IHasExplicitProperty>>(
            TestDataContext::as_has_explicit_property,
        );
        ValueTypes::register_cast::<Rc<dyn IHasPropertyDerived>, Rc<dyn IHasProperty>>(|derived| derived.clone());
        ValueTypes::register_cast::<NonIntegerIndexer, Rc<dyn INonIntegerIndexerDerived>>(
            NonIntegerIndexer::as_non_integer_indexer_derived,
        );
        ValueTypes::register_cast::<NonIntegerIndexer, Rc<dyn INonIntegerIndexer>>(
            NonIntegerIndexer::as_non_integer_indexer,
        );
        ValueTypes::register_cast::<Rc<dyn INonIntegerIndexerDerived>, Rc<dyn INonIntegerIndexer>>(|derived| {
            derived.clone()
        });
        ValueTypes::register_cast::<CustomDataTemplate, Rc<dyn IDataTemplate>>(CustomDataTemplate::as_data_template);
        ValueTypes::register_cast::<CustomDataTemplateInherit, Rc<dyn IDataTemplate>>(|template| {
            template.base().as_data_template()
        });

        // `implicit operator Color(ImplicitConvertible value)` for the value conversions of
        // bindings (the compiler finds the operator in the metadata of the type).
        ValueTypes::register_conversion::<ImplicitConvertible, Color>(|value| value.to_color().ok());

        // Collections as the items of items controls: as the handle and as the reference
        // object (the untyped form in which a property declared in metadata delivers them).
        ItemsSource::register_binding_conversion::<Rc<BindableList<String>>>();
        ItemsSource::register_binding_conversion::<Rc<BindableList<Rc<TestData>>>>();
        register_list_object_conversion::<String>();
        register_list_object_conversion::<Rc<TestData>>();
        ValueTypes::register_conversion::<ListItemCollectionViewInt32, Option<ItemsSource>>(|_| None);
    },
};

/// Lets bindings deliver a list that is held as a reference object (the
/// untyped form of `Rc<BindableList<T>>`) to an items source property.
fn register_list_object_conversion<T: ferroui_base::PropertyValue>() {
    fn items_source<T: ferroui_base::PropertyValue>(list: &BoxedValue) -> Option<ItemsSource> {
        value_of::<Rc<BindableList<T>>>(&Some(list.clone())).map(ItemsSource::from)
    }
    ValueTypes::register_boxed_conversion::<BindableList<T>, ItemsSource>(items_source::<T>);
    ValueTypes::register_boxed_conversion::<BindableList<T>, Option<ItemsSource>>(|list| Some(items_source::<T>(list)));
}

/// `PerformClick(button)`: the key press that clicks a button.
pub(crate) fn perform_click(button: &Button) {
    let mut e = KeyEventArgs::new();
    e.set_routed_event(Some(InputElement::key_down_event()));
    e.key = Key::Enter;
    button.raise_event(&e);
}

// --- tests ------------------------------------------------------------------

/// `descendants.OfType<ContentPresenter>().First()`.
fn first_content_presenter(descendants: Vec<Ref<Visual>>) -> Ref<ContentPresenter> {
    descendants
        .into_iter()
        .find_map(|visual| visual.cast::<ContentPresenter>())
        .expect("Sequence contains no elements")
}

/// `ReferenceEquals(a, b)` of two untyped values.
fn is_same_box(a: &Option<BoxedValue>, b: &Option<BoxedValue>) -> bool {
    match (a, b) {
        (Some(a), Some(b)) => Rc::ptr_eq(a, b),
        (None, None) => true,
        _ => false,
    }
}

#[test]
fn resolves_clr_property_based_on_data_context_type() {
    let _app = styled_window_application();
    let xaml = r#"
<Window xmlns='https://github.com/ferroui'
        xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'
        xmlns:local='clr-namespace:FerroUI.Markup.Xaml.UnitTests.MarkupExtensions;assembly=FerroUI.Markup.Xaml.UnitTests'
        x:DataType='local:TestDataContext'>
    <TextBlock Text='{CompiledBinding StringProperty}' Name='textBlock' />
</Window>"#;
    let window: Ref<Window> = load_as(xaml);
    let text_block = window.get_control::<TextBlock>("textBlock");

    let data_context = TestDataContext::new();
    data_context.set_string_property(Some("foobar".to_string()));

    window.set_data_context(Some(data_context.clone()));

    assert_eq!(data_context.string_property(), text_block.text());
}

#[test]
fn resolves_clr_property_based_on_data_context_type_interface_inheritance() {
    let _app = styled_window_application();
    let xaml = r#"
<Window xmlns='https://github.com/ferroui'
        xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'
        xmlns:local='clr-namespace:FerroUI.Markup.Xaml.UnitTests.MarkupExtensions;assembly=FerroUI.Markup.Xaml.UnitTests'
        x:DataType='local:IHasPropertyDerived'>
    <TextBlock Text='{CompiledBinding StringProperty}' Name='textBlock' />
</Window>"#;
    let window: Ref<Window> = load_as(xaml);
    let text_block = window.get_control::<TextBlock>("textBlock");

    let data_context = TestDataContext::new();
    data_context.set_string_property(Some("foobar".to_string()));

    window.set_data_context(Some(data_context.clone()));

    assert_eq!(data_context.string_property(), text_block.text());
}

#[test]
fn resolves_path_passed_by_property() {
    let _app = styled_window_application();
    let xaml = r#"
<Window xmlns='https://github.com/ferroui'
        xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'
        xmlns:local='clr-namespace:FerroUI.Markup.Xaml.UnitTests.MarkupExtensions;assembly=FerroUI.Markup.Xaml.UnitTests'
        x:DataType='local:TestDataContext'>
    <TextBlock Text='{CompiledBinding Path=StringProperty}' Name='textBlock' />
</Window>"#;
    let window: Ref<Window> = load_as(xaml);
    let text_block = window.get_control::<TextBlock>("textBlock");

    let data_context = TestDataContext::new();
    data_context.set_string_property(Some("foobar".to_string()));

    window.set_data_context(Some(data_context.clone()));

    assert_eq!(data_context.string_property(), text_block.text());
}

#[test]
fn resolves_path_passed_by_property_with_inner_item_template() {
    let _app = styled_window_application();
    let xaml = r#"
<Window xmlns='https://github.com/ferroui'
        xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'
        xmlns:local='clr-namespace:FerroUI.Markup.Xaml.UnitTests.MarkupExtensions;assembly=FerroUI.Markup.Xaml.UnitTests'
        x:DataType='local:TestDataContext'>
    <ItemsControl Name='itemsControl' ItemsSource='{CompiledBinding Path=ListProperty}'>
	    <ItemsControl.ItemTemplate>
		    <DataTemplate>
			    <TextBlock />
		    </DataTemplate>
	    </ItemsControl.ItemTemplate>
    </ItemsControl>
</Window>"#;
    let window: Ref<Window> = load_as(xaml);
    let text_block = window.get_control::<ItemsControl>("itemsControl");

    let data_context = TestDataContext::new();
    data_context.list_property().items().add("Hello".to_string());

    window.set_data_context(Some(data_context.clone()));

    let expected = ItemsSource::from(data_context.list_property());
    assert!(
        text_block.items_source().is_some_and(|items_source| items_source.ptr_eq(&expected)),
        "the items source is {:?}",
        text_block.items_source()
    );
}

#[test]
fn resolves_static_clr_property_based() {
    let _app = styled_window_application();
    let xaml = r#"
<Window xmlns='https://github.com/ferroui'
        xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'
        xmlns:local='clr-namespace:FerroUI.Markup.Xaml.UnitTests.MarkupExtensions;assembly=FerroUI.Markup.Xaml.UnitTests'
        x:DataType='local:TestDataContext'>
    <TextBlock Text='{CompiledBinding StaticProperty}' Name='textBlock' />
</Window>"#;
    let window: Ref<Window> = load_as(xaml);
    let text_block = window.get_control::<TextBlock>("textBlock");

    text_block.set_data_context(Some(TestDataContext::new()));

    assert_eq!(Some(TestDataContext::static_property()), text_block.text());
}

#[test]
fn resolves_data_type_from_binding_property() {
    let _app = styled_window_application();
    let xaml = r#"
<Window xmlns='https://github.com/ferroui'
        xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'
        xmlns:local='clr-namespace:FerroUI.Markup.Xaml.UnitTests.MarkupExtensions;assembly=FerroUI.Markup.Xaml.UnitTests'>
    <TextBlock Text='{CompiledBinding StringProperty, DataType=local:TestDataContext}' Name='textBlock' />
</Window>"#;
    let window: Ref<Window> = load_as(xaml);
    let text_block = window.get_control::<TextBlock>("textBlock");

    let data_context = TestDataContext::new();
    data_context.set_string_property(Some("foobar".to_string()));

    window.set_data_context(Some(data_context.clone()));

    assert_eq!(data_context.string_property(), text_block.text());
}

#[test]
fn resolves_data_type_from_binding_property_type_extension() {
    let _app = styled_window_application();
    let xaml = r#"
<Window xmlns='https://github.com/ferroui'
        xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'
        xmlns:local='clr-namespace:FerroUI.Markup.Xaml.UnitTests.MarkupExtensions;assembly=FerroUI.Markup.Xaml.UnitTests'>
    <TextBlock Text='{CompiledBinding StringProperty, DataType={x:Type local:TestDataContext}}' Name='textBlock' />
</Window>"#;
    let window: Ref<Window> = load_as(xaml);
    let text_block = window.get_control::<TextBlock>("textBlock");

    let data_context = TestDataContext::new();
    data_context.set_string_property(Some("foobar".to_string()));

    window.set_data_context(Some(data_context.clone()));

    assert_eq!(data_context.string_property(), text_block.text());
}

#[test]
fn resolves_stream_task_binding_correctly() {
    let _app = styled_window_application();
    let xaml = r#"
<Window xmlns='https://github.com/ferroui'
        xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'
        xmlns:local='clr-namespace:FerroUI.Markup.Xaml.UnitTests.MarkupExtensions;assembly=FerroUI.Markup.Xaml.UnitTests'
        x:DataType='local:TestDataContext'>
    <TextBlock Text='{CompiledBinding TaskProperty^}' Name='textBlock' />
</Window>"#;
    let window: Ref<Window> = load_as(xaml);
    let text_block = window.get_control::<TextBlock>("textBlock");

    let data_context = TestDataContext::new();
    data_context.set_task_property(Some(TaskValue::from_result("foobar".to_string())));

    window.set_data_context(Some(data_context.clone()));

    assert_eq!(Some("foobar"), text_block.text().as_deref());
}

#[test]
fn resolves_stream_observable_binding_correctly() {
    let _app = styled_window_application();
    let xaml = r#"
<Window xmlns='https://github.com/ferroui'
        xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'
        xmlns:local='clr-namespace:FerroUI.Markup.Xaml.UnitTests.MarkupExtensions;assembly=FerroUI.Markup.Xaml.UnitTests'
        x:DataType='local:TestDataContext'>
    <TextBlock Text='{CompiledBinding ObservableProperty^}' Name='textBlock' />
</Window>"#;
    let window: Ref<Window> = load_as(xaml);
    let text_block = window.get_control::<TextBlock>("textBlock");

    DelayedBinding::apply_bindings(&text_block);

    let subject = LightweightSubject::<String>::new();
    let observable: Rc<dyn IObservable<String>> = Rc::new(subject.clone());
    let data_context = TestDataContext::new();
    data_context.set_observable_property(Some(ObservableValue::new(observable)));

    window.set_data_context(Some(data_context.clone()));

    IObserver::on_next(&subject, "foobar".to_string());

    assert_eq!(Some("foobar"), text_block.text().as_deref());
}

#[test]
fn resolves_indexer_binding_correctly() {
    let _app = styled_window_application();
    let xaml = r#"
<Window xmlns='https://github.com/ferroui'
        xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'
        xmlns:local='clr-namespace:FerroUI.Markup.Xaml.UnitTests.MarkupExtensions;assembly=FerroUI.Markup.Xaml.UnitTests'
        x:DataType='local:TestDataContext'>
    <TextBlock Text='{CompiledBinding ListProperty[3]}' Name='textBlock' />
</Window>"#;
    let window: Ref<Window> = load_as(xaml);
    let text_block = window.get_control::<TextBlock>("textBlock");

    let data_context = TestDataContext::new();
    data_context.list_property().items().add_range(["A", "B", "C", "D", "E"].map(String::from));

    window.set_data_context(Some(data_context.clone()));

    assert_eq!(Some(data_context.list_property().items().get(3)), text_block.text());
}

#[test]
fn indexer_setter_binds_correctly() {
    let _app = styled_window_application();
    let window: Ref<Window> = load_as(
        r#"
<Window xmlns='https://github.com/ferroui'
        xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'
        xmlns:local='clr-namespace:FerroUI.Markup.Xaml.UnitTests.MarkupExtensions;assembly=FerroUI.Markup.Xaml.UnitTests'
        x:DataType='local:TestDataContext'>
    <TextBox Text='{CompiledBinding ListProperty[3], Mode=TwoWay}' Name='textBox' />
</Window>"#,
    );
    let text_box = window.get_control::<TextBox>("textBox");

    let data_context = TestDataContext::new();
    data_context.list_property().items().add_range(["A", "B", "C", "D", "E"].map(String::from));

    window.set_data_context(Some(data_context.clone()));

    assert_eq!(Some(data_context.list_property().items().get(3)), text_box.text());

    text_box.set_text(Some("Z"));

    assert_eq!("Z", data_context.list_property().items().get(3));
    assert_eq!(Some(data_context.list_property().items().get(3)), text_box.text());
}

#[test]
fn resolves_array_indexer_binding_correctly() {
    let _app = styled_window_application();
    let xaml = r#"
<Window xmlns='https://github.com/ferroui'
        xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'
        xmlns:local='clr-namespace:FerroUI.Markup.Xaml.UnitTests.MarkupExtensions;assembly=FerroUI.Markup.Xaml.UnitTests'
        x:DataType='local:TestDataContext'>
    <TextBlock Text='{CompiledBinding ArrayProperty[3]}' Name='textBlock' />
</Window>"#;
    let window: Ref<Window> = load_as(xaml);
    let text_block = window.get_control::<TextBlock>("textBlock");

    let data_context = TestDataContext::new();
    data_context.set_array_property(Some(BindableArray::new(["A", "B", "C", "D", "E"].map(String::from))));

    window.set_data_context(Some(data_context.clone()));

    assert_eq!(Some(data_context.array_property().unwrap().get(&[3])), text_block.text());
}

#[test]
fn resolves_observable_indexer_binding_correctly() {
    let _app = styled_window_application();
    let xaml = r#"
<Window xmlns='https://github.com/ferroui'
        xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'
        xmlns:local='clr-namespace:FerroUI.Markup.Xaml.UnitTests.MarkupExtensions;assembly=FerroUI.Markup.Xaml.UnitTests'
        x:DataType='local:TestDataContext'>
    <TextBlock Text='{CompiledBinding ObservableCollectionProperty[3]}' Name='textBlock' />
</Window>"#;
    let window: Ref<Window> = load_as(xaml);
    let text_block = window.get_control::<TextBlock>("textBlock");

    let data_context = TestDataContext::new();
    data_context.observable_collection_property().items().add_range(["A", "B", "C", "D", "E"].map(String::from));

    window.set_data_context(Some(data_context.clone()));

    assert_eq!(Some(data_context.observable_collection_property().items().get(3)), text_block.text());

    data_context.observable_collection_property().items().set(3, "New Value".to_string());

    assert_eq!(Some(data_context.observable_collection_property().items().get(3)), text_block.text());
}

#[test]
fn infers_compiled_binding_data_context_from_data_context_binding() {
    let _app = styled_window_application();
    let xaml = r#"
<Window xmlns='https://github.com/ferroui'
        xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'
        xmlns:local='clr-namespace:FerroUI.Markup.Xaml.UnitTests.MarkupExtensions;assembly=FerroUI.Markup.Xaml.UnitTests'
        x:DataType='local:TestDataContext'>
    <TextBlock DataContext='{CompiledBinding StringProperty}' Text='{CompiledBinding}' Name='textBlock' />
</Window>"#;
    let window: Ref<Window> = load_as(xaml);
    let text_block = window.get_control::<TextBlock>("textBlock");

    window.apply_template();
    window.presenter().unwrap().apply_template();

    let data_context = TestDataContext::new();
    data_context.set_string_property(Some("A".to_string()));

    window.set_data_context(Some(data_context.clone()));

    assert_eq!(data_context.string_property(), text_block.text());
}

#[test]
fn resolves_non_integer_indexer_binding_correctly() {
    let _app = styled_window_application();
    let xaml = r#"
<Window xmlns='https://github.com/ferroui'
        xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'
        xmlns:local='clr-namespace:FerroUI.Markup.Xaml.UnitTests.MarkupExtensions;assembly=FerroUI.Markup.Xaml.UnitTests'
        x:DataType='local:TestDataContext'>
    <TextBlock Text='{CompiledBinding NonIntegerIndexerProperty[Test]}' Name='textBlock' />
</Window>"#;
    let window: Ref<Window> = load_as(xaml);
    let text_block = window.get_control::<TextBlock>("textBlock");

    let data_context = TestDataContext::new();

    data_context.non_integer_indexer_property().set("Test", "Initial Value".to_string());

    window.set_data_context(Some(data_context.clone()));

    assert_eq!(Some(data_context.non_integer_indexer_property().get("Test")), text_block.text());

    data_context.non_integer_indexer_property().set("Test", "New Value".to_string());

    assert_eq!(Some(data_context.non_integer_indexer_property().get("Test")), text_block.text());
}

#[test]
#[ignore = "base metadata: the change notifier of a value held as an interface handle (Rc<dyn INonIntegerIndexerDerived>) is looked up in the metadata of the interface, which cannot state one, not on the object behind the handle: a change the object raises for its indexer is not observed"]
fn resolves_non_integer_indexer_binding_from_parent_interface_correctly() {
    let _app = styled_window_application();
    let xaml = r#"
<Window xmlns='https://github.com/ferroui'
        xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'
        xmlns:local='clr-namespace:FerroUI.Markup.Xaml.UnitTests.MarkupExtensions;assembly=FerroUI.Markup.Xaml.UnitTests'
        x:DataType='local:TestDataContext'>
    <TextBlock Text='{CompiledBinding NonIntegerIndexerInterfaceProperty[Test]}' Name='textBlock' />
</Window>"#;
    let window: Ref<Window> = load_as(xaml);
    let text_block = window.get_control::<TextBlock>("textBlock");

    let data_context = TestDataContext::new();

    data_context.non_integer_indexer_interface_property().set("Test", "Initial Value".to_string());

    window.set_data_context(Some(data_context.clone()));

    assert_eq!(Some(data_context.non_integer_indexer_interface_property().get("Test")), text_block.text());

    data_context.non_integer_indexer_interface_property().set("Test", "New Value".to_string());

    assert_eq!(Some(data_context.non_integer_indexer_interface_property().get("Test")), text_block.text());
}

#[test]
fn infers_data_template_type_from_data_type_property() {
    let _app = styled_window_application();
    let xaml = r#"
<Window xmlns='https://github.com/ferroui'
        xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'
        xmlns:local='clr-namespace:FerroUI.Markup.Xaml.UnitTests.MarkupExtensions;assembly=FerroUI.Markup.Xaml.UnitTests'
        x:DataType='local:TestDataContext'>
    <Window.DataTemplates>
        <DataTemplate DataType='{x:Type x:String}'>
            <TextBlock Text='{CompiledBinding}' Name='textBlock' />
        </DataTemplate>
    </Window.DataTemplates>
    <ContentControl Name='target' Content='{CompiledBinding StringProperty}' />
</Window>"#;
    let window: Ref<Window> = load_as(xaml);
    let target = window.get_control::<ContentControl>("target");

    let data_context = TestDataContext::new();

    data_context.set_string_property(Some("Initial Value".to_string()));

    window.set_data_context(Some(data_context.clone()));

    window.apply_template();
    target.apply_template();
    let presenter = target.presenter().unwrap();
    presenter.update_child();

    let child = presenter.child().unwrap().cast::<TextBlock>().unwrap();
    assert_eq!(data_context.string_property(), child.text());
}

#[test]
fn throws_on_uninferrable_loose_data_template_no_data_type_with_compiled_binding_path() {
    let _app = styled_window_application();
    let xaml = r#"
<Window xmlns='https://github.com/ferroui'
        xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'
        xmlns:local='clr-namespace:FerroUI.Markup.Xaml.UnitTests.MarkupExtensions;assembly=FerroUI.Markup.Xaml.UnitTests'
        x:DataType='local:TestDataContext'>
    <Window.DataTemplates>
        <DataTemplate>
            <TextBlock Text='{CompiledBinding StringProperty}' Name='textBlock' />
        </DataTemplate>
    </Window.DataTemplates>
    <ContentControl Name='target' Content='{CompiledBinding}' />
</Window>"#;
    assert_throws_xaml_diagnostic(
        try_load(xaml),
        "FRN2000",
        "Unable to resolve property or method of name 'StringProperty' on type 'XamlX.TypeSystem.XamlPseudoType'. Line 8, position 24.",
    );
}

#[test]
fn throws_on_uninferrable_data_type_from_non_compiled_data_context_binding_with_compiled_binding_path() {
    let _app = styled_window_application();
    let xaml = r#"
<Window xmlns='https://github.com/ferroui'
        xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'
        xmlns:local='clr-namespace:FerroUI.Markup.Xaml.UnitTests.MarkupExtensions;assembly=FerroUI.Markup.Xaml.UnitTests'
        x:DataType='local:TestDataContext'>
    <ContentControl Name='target' DataContext='{Binding}'>
        <TextBlock Text='{CompiledBinding StringProperty}' Name='textBlock' />
    </ContentControl>
</Window>"#;
    assert_throws_xaml_diagnostic(
        try_load(xaml),
        "FRN2000",
        "Unable to resolve property or method of name 'StringProperty' on type 'XamlX.TypeSystem.XamlPseudoType'. Line 7, position 20.",
    );
}

#[test]
fn reports_multiple_errors_on_data_context_and_binding_path_errors() {
    let _app = styled_window_application();
    let xaml = r#"
<Window xmlns='https://github.com/ferroui'
        xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'
        xmlns:local='clr-namespace:FerroUI.Markup.Xaml.UnitTests.MarkupExtensions;assembly=FerroUI.Markup.Xaml.UnitTests'>
    <ContentControl Content='{CompiledBinding NoDataContext}'
                    Tag='{CompiledBinding NonExistentProp, DataType=local:TestDataContext}'
                    Height='{CompiledBinding invalid.}' />
</Window>"#;
    let error = match try_load(xaml) {
        Ok(_) => panic!("Expected an AggregateException"),
        Err(error) => error,
    };
    let inner = match xaml_error(&error) {
        Some(XamlError::Aggregate(inner)) => inner,
        _ => panic!("Expected an AggregateException: {}", describe(&error)),
    };
    assert_eq!(3, inner.len());
    for inner in inner {
        assert!(inner.is_xml_exception(), "{} is not an XML exception", inner.type_name());
    }
}

#[test]
#[ignore = "themes: needs the Simple theme (upstream runs this test under it)"]
fn infers_data_template_type_from_parent_collection_items_type() {
    let _app = styled_window_application();
    let xaml = r#"
<Window xmlns='https://github.com/ferroui'
        xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'
        xmlns:local='clr-namespace:FerroUI.Markup.Xaml.UnitTests.MarkupExtensions;assembly=FerroUI.Markup.Xaml.UnitTests'
        x:DataType='local:TestDataContext'>
    <ItemsControl ItemsSource='{CompiledBinding ListProperty}' Name='target'>
        <ItemsControl.ItemTemplate>
            <DataTemplate>
                <TextBlock Text='{CompiledBinding}' Name='textBlock' />
            </DataTemplate>
        </ItemsControl.ItemTemplate>
    </ItemsControl>
</Window>"#;
    let window: Ref<Window> = load_as(xaml);
    let target = window.get_control::<ItemsControl>("target");

    let data_context = TestDataContext::new();

    data_context.list_property().items().add("Test".to_string());

    window.set_data_context(Some(data_context.clone()));

    window.apply_template();
    target.apply_template();
    let presenter = target.presenter().unwrap();
    presenter.apply_template();

    let container = presenter.panel().unwrap().children().get(0).cast::<ContentPresenter>().unwrap();
    assert_eq!(Some(data_context.list_property().items().get(0)), string_of(&container.content()));
}

/// `Assert.IsAssignableFrom<PropertyElement>(Assert.Single(((CompiledBinding)binding).Path!.Elements))`:
/// the type and the name of the property the single element of the path of
/// a compiled binding reads (`node.Property.PropertyType`, `node.Property.Name`).
#[track_caller]
fn single_property_element(binding: &Rc<dyn BindingBase>) -> (ValueType, String) {
    let compiled = assert_binding_is_type::<CompiledBinding>(binding);
    let compiled_path = compiled.path().expect("the path of the binding is null");
    assert_eq!(1, compiled_path.len(), "the collection does not contain exactly one element");
    let node = compiled_path.element(0).expect("the path has an element");
    assert_eq!(CompiledBindingPathElementKind::Property, node.kind());
    (
        node.property_type().expect("the property element has no property type"),
        node.name().expect("the property element has no name").to_string(),
    )
}

#[test]
fn infers_data_type_from_parent_data_grid_items_type_in_case_of_control_inheritance() {
    let _app = styled_window_application();
    let window: Ref<Window> = load_as(
        r#"
<Window xmlns='https://github.com/ferroui'
        xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'
        xmlns:local='clr-namespace:FerroUI.Markup.Xaml.UnitTests.MarkupExtensions;assembly=FerroUI.Markup.Xaml.UnitTests'
        x:DataType='local:TestItemsCollectionDataContext'>
    <local:DataGridLikeControlInheritor Items='{CompiledBinding Items}' Name='target'>
        <local:DataGridLikeControlInheritor.Columns>
            <local:DataGridLikeColumn Binding='{CompiledBinding StringProperty}'>
            </local:DataGridLikeColumn>
        </local:DataGridLikeControlInheritor.Columns>
    </local:DataGridLikeControlInheritor>
</Window>"#,
    );
    let target = window.get_control::<DataGridLikeControl>("target");
    let column = target.columns().single();

    let data_context = TestItemsCollectionDataContext::new();
    let item = TestData::new();
    item.set_string_property(Some("Test".to_string()));
    data_context.items().items().add(item);
    window.set_data_context(Some(data_context));

    window.apply_template();
    target.apply_template();

    // Assert DataGridLikeColumn.Binding data type.
    let binding = column.binding().expect("the binding of the column is null");
    let (property_type, name) = single_property_element(&binding);
    assert!(property_type.is_string(), "the property type is {}", property_type.name());
    assert_eq!("StringProperty", name);
}

#[test]
fn infers_data_template_type_from_parent_data_grid_items_type() {
    let _app = styled_window_application();
    let window: Ref<Window> = load_as(
        r#"
<Window xmlns='https://github.com/ferroui'
        xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'
        xmlns:local='clr-namespace:FerroUI.Markup.Xaml.UnitTests.MarkupExtensions;assembly=FerroUI.Markup.Xaml.UnitTests'
        x:DataType='local:TestDataContext'>
    <local:DataGridLikeControl Items='{CompiledBinding ListProperty}' Name='target'>
        <local:DataGridLikeControl.Columns>
            <local:DataGridLikeColumn Binding='{CompiledBinding Length}'>
                <local:DataGridLikeColumn.Template>
                    <DataTemplate>
                        <TextBlock Text='{CompiledBinding Length}' />
                    </DataTemplate>
                </local:DataGridLikeColumn.Template>
            </local:DataGridLikeColumn>
        </local:DataGridLikeControl.Columns>
    </local:DataGridLikeControl>
</Window>"#,
    );
    let target = window.get_control::<DataGridLikeControl>("target");
    let column = target.columns().single();

    let data_context = TestDataContext::new();
    data_context.list_property().items().add("Test".to_string());
    window.set_data_context(Some(data_context.clone()));

    window.apply_template();
    target.apply_template();

    // Assert DataGridLikeColumn.Binding data type.
    let binding = column.binding().expect("the binding of the column is null");
    let (property_type, _) = single_property_element(&binding);
    assert!(property_type.is::<i32>(), "the property type is {}", property_type.name());

    // Assert DataGridLikeColumn.Template data type by evaluating the template.
    let first_item = data_context.list_property().items().get(0);
    let template = column.template().expect("the template of the column is null");
    let text_block_from_template = template
        .build(&Some(boxed(first_item.clone())))
        .expect("the template built no control")
        .cast::<TextBlock>()
        .expect("the control the template built is not a TextBlock");
    text_block_from_template.set_data_context(Some(boxed(first_item.clone())));
    assert_eq!(Some(first_item.encode_utf16().count().to_string()), text_block_from_template.text());
}

#[test]
fn explicit_data_type_still_works_on_data_grid_like_controls() {
    let _app = styled_window_application();
    let window: Ref<Window> = load_as(
        r#"
<Window xmlns='https://github.com/ferroui'
        xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'
        xmlns:local='clr-namespace:FerroUI.Markup.Xaml.UnitTests.MarkupExtensions;assembly=FerroUI.Markup.Xaml.UnitTests'
        x:DataType='local:TestDataContext'>
    <local:DataGridLikeControl Name='target'>
        <local:DataGridLikeControl.Columns>
            <local:DataGridLikeColumn Binding='{CompiledBinding Length}' x:DataType='x:String'>
                <local:DataGridLikeColumn.Template>
                    <DataTemplate x:DataType='x:String'>
                        <TextBlock Text='{CompiledBinding Length}' />
                    </DataTemplate>
                </local:DataGridLikeColumn.Template>
            </local:DataGridLikeColumn>
        </local:DataGridLikeControl.Columns>
    </local:DataGridLikeControl>
</Window>"#,
    );
    let target = window.get_control::<DataGridLikeControl>("target");
    let column = target.columns().single();

    let data_context = TestDataContext::new();
    data_context.list_property().items().add("Test".to_string());
    target.set_items(Some(ItemsSource::from(data_context.list_property())));

    window.apply_template();
    target.apply_template();

    // Assert DataGridLikeColumn.Binding data type.
    let binding = column.binding().expect("the binding of the column is null");
    let (property_type, _) = single_property_element(&binding);
    assert!(property_type.is::<i32>(), "the property type is {}", property_type.name());

    // Assert DataGridLikeColumn.Template data type by evaluating the template.
    let first_item = data_context.list_property().items().get(0);
    let template = column.template().expect("the template of the column is null");
    let text_block_from_template = template
        .build(&Some(boxed(first_item.clone())))
        .expect("the template built no control")
        .cast::<TextBlock>()
        .expect("the control the template built is not a TextBlock");
    text_block_from_template.set_data_context(Some(boxed(first_item.clone())));
    assert_eq!(Some(first_item.encode_utf16().count().to_string()), text_block_from_template.text());
}

#[test]
fn throws_on_uninferrable_data_template_in_items_control_without_items_binding() {
    let _app = styled_window_application();
    let xaml = r#"
<Window xmlns='https://github.com/ferroui'
        xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'
        xmlns:local='clr-namespace:FerroUI.Markup.Xaml.UnitTests.MarkupExtensions;assembly=FerroUI.Markup.Xaml.UnitTests'
        x:DataType='local:TestDataContext'>
    <ItemsControl Name='target'>
        <ItemsControl.DataTemplates>
            <DataTemplate>
                <TextBlock Text='{CompiledBinding Property}' Name='textBlock' />
            </DataTemplate>
        </ItemsControl.DataTemplates>
    </ItemsControl>
</Window>"#;
    assert_throws_xaml_diagnostic(
        try_load(xaml),
        "FRN2000",
        "Unable to resolve property or method of name 'Property' on type 'XamlX.TypeSystem.XamlPseudoType'. Line 9, position 28.",
    );
}

#[test]
fn ignores_data_template_type_from_data_type_property_if_x_data_type_defined() {
    let _app = styled_window_application();
    let xaml = r#"
<Window xmlns='https://github.com/ferroui'
        xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'
        xmlns:local='clr-namespace:FerroUI.Markup.Xaml.UnitTests.MarkupExtensions;assembly=FerroUI.Markup.Xaml.UnitTests'>
    <Window.DataTemplates>
        <DataTemplate DataType='local:TestDataContextBaseClass' x:DataType='local:TestDataContext'>
            <TextBlock Text='{CompiledBinding StringProperty}' Name='textBlock' />
        </DataTemplate>
    </Window.DataTemplates>
    <ContentControl x:DataType='local:TestDataContext' Name='target' Content='{CompiledBinding}' />
</Window>"#;
    let window: Ref<Window> = load_as(xaml);
    let target = window.get_control::<ContentControl>("target");

    let data_context = TestDataContext::new();

    data_context.set_string_property(Some("Initial Value".to_string()));

    window.set_data_context(Some(data_context.clone()));

    window.apply_template();
    target.apply_template();
    let presenter = target.presenter().unwrap();
    presenter.update_child();

    let child = presenter.child().unwrap().cast::<TextBlock>().unwrap();
    assert_eq!(data_context.string_property(), child.text());
}

#[test]
fn infers_custom_data_template_based_on_attribute() {
    let _app = styled_window_application();
    let xaml = r#"
<Window xmlns='https://github.com/ferroui'
        xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'
        xmlns:local='clr-namespace:FerroUI.Markup.Xaml.UnitTests.MarkupExtensions;assembly=FerroUI.Markup.Xaml.UnitTests'>
    <Window.DataTemplates>
        <local:CustomDataTemplate FancyDataType='local:TestDataContext'>
            <TextBlock Text='{CompiledBinding StringProperty}' Name='textBlock' />
        </local:CustomDataTemplate>
    </Window.DataTemplates>
    <ContentControl x:DataType='local:TestDataContext' Name='target' Content='{CompiledBinding}' />
</Window>"#;
    let window: Ref<Window> = load_as(xaml);
    let target = window.get_control::<ContentControl>("target");

    let data_context = TestDataContext::new();

    data_context.set_string_property(Some("Initial Value".to_string()));

    window.set_data_context(Some(data_context.clone()));

    window.apply_template();
    target.apply_template();
    let presenter = target.presenter().unwrap();
    presenter.update_child();

    let child = presenter.child().unwrap().cast::<TextBlock>().unwrap();
    assert_eq!(data_context.string_property(), child.text());
}

#[test]
fn infers_custom_data_template_based_on_attribute_from_base_class() {
    let _app = styled_window_application();
    let xaml = r#"
<Window xmlns='https://github.com/ferroui'
        xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'
        xmlns:local='clr-namespace:FerroUI.Markup.Xaml.UnitTests.MarkupExtensions;assembly=FerroUI.Markup.Xaml.UnitTests'>
    <Window.DataTemplates>
        <local:CustomDataTemplateInherit FancyDataType='local:TestDataContext'>
            <TextBlock Text='{CompiledBinding StringProperty}' Name='textBlock' />
        </local:CustomDataTemplateInherit>
    </Window.DataTemplates>
    <ContentControl x:DataType='local:TestDataContext' Name='target' Content='{CompiledBinding}' />
</Window>"#;
    let window: Ref<Window> = load_as(xaml);
    let target = window.get_control::<ContentControl>("target");

    let data_context = TestDataContext::new();

    data_context.set_string_property(Some("Initial Value".to_string()));

    window.set_data_context(Some(data_context.clone()));

    window.apply_template();
    target.apply_template();
    let presenter = target.presenter().unwrap();
    presenter.update_child();

    let child = presenter.child().unwrap().cast::<TextBlock>().unwrap();
    assert_eq!(data_context.string_property(), child.text());
}

#[test]
fn resolves_element_name_binding() {
    let _app = styled_window_application();
    let xaml = r#"
<Window xmlns='https://github.com/ferroui'
        xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'
        xmlns:local='clr-namespace:FerroUI.Markup.Xaml.UnitTests.MarkupExtensions;assembly=FerroUI.Markup.Xaml.UnitTests'
        x:DataType='local:TestDataContext'>
    <StackPanel>
        <TextBlock Text='{CompiledBinding StringProperty}' x:Name='text' />
        <TextBlock Text='{CompiledBinding #text.Text}' x:Name='text2' />
    </StackPanel>
</Window>"#;
    let window: Ref<Window> = load_as(xaml);
    let text_block = window.get_control::<TextBlock>("text2");

    let data_context = TestDataContext::new();
    data_context.set_string_property(Some("foobar".to_string()));

    window.set_data_context(Some(data_context.clone()));

    assert_eq!(data_context.string_property(), text_block.text());
}

#[test]
fn resolves_element_name_binding_from_long_form() {
    let _app = styled_window_application();
    let xaml = r#"
<Window xmlns='https://github.com/ferroui'
        xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'
        xmlns:local='clr-namespace:FerroUI.Markup.Xaml.UnitTests.MarkupExtensions;assembly=FerroUI.Markup.Xaml.UnitTests'
        x:DataType='local:TestDataContext'>
    <StackPanel>
        <TextBlock Text='{CompiledBinding StringProperty}' x:Name='text' />
        <TextBlock Text='{CompiledBinding Text, ElementName=text}' x:Name='text2' />
    </StackPanel>
</Window>"#;
    let window: Ref<Window> = load_as(xaml);
    let text_block = window.get_control::<TextBlock>("text2");

    let data_context = TestDataContext::new();
    data_context.set_string_property(Some("foobar".to_string()));

    window.set_data_context(Some(data_context.clone()));

    assert_eq!(data_context.string_property(), text_block.text());
}

#[test]
fn resolves_element_name_binding_from_long_form_without_path() {
    let _app = styled_window_application();
    let xaml = r#"
<Window xmlns='https://github.com/ferroui'
        xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'
        xmlns:local='clr-namespace:FerroUI.Markup.Xaml.UnitTests.MarkupExtensions;assembly=FerroUI.Markup.Xaml.UnitTests'
        x:DataType='local:TestDataContext'>
    <StackPanel>
        <TextBlock Text='{CompiledBinding StringProperty}' x:Name='text' />
        <TextBlock Text='{CompiledBinding ElementName=text}' x:Name='text2' />
    </StackPanel>
</Window>"#;
    let window: Ref<Window> = load_as(xaml);
    let text_block = window.get_control::<TextBlock>("text2");

    assert_eq!(Some("FerroUI.Controls.TextBlock"), text_block.text().as_deref());
}

#[test]
fn resolves_relative_source_binding_long_form() {
    let _app = styled_window_application();
    let xaml = r#"
<Window xmlns='https://github.com/ferroui'
        xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'
        xmlns:local='clr-namespace:FerroUI.Markup.Xaml.UnitTests.MarkupExtensions;assembly=FerroUI.Markup.Xaml.UnitTests'
        x:DataType='local:TestDataContext'
        Title='test'>
    <TextBlock Text='{CompiledBinding Title, RelativeSource={RelativeSource AncestorType=Window}}' x:Name='text'/>
</Window>"#;
    let window: Ref<Window> = load_as(xaml);
    let target = window.get_control::<TextBlock>("text");

    window.apply_template();
    window.presenter().unwrap().apply_template();
    target.apply_template();

    assert_eq!(Some("test"), target.text().as_deref());
}

#[test]
fn resolves_relative_source_binding_even_longer_form() {
    let _app = styled_window_application();
    let xaml = r#"
<Window xmlns='https://github.com/ferroui'
        xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'
        xmlns:local='clr-namespace:FerroUI.Markup.Xaml.UnitTests.MarkupExtensions;assembly=FerroUI.Markup.Xaml.UnitTests'
        x:DataType='local:TestDataContext'
        Title='test'>
    <TextBlock Text='{CompiledBinding Title, RelativeSource={RelativeSource AncestorType={x:Type Window}}}' x:Name='text'/>
</Window>"#;
    let window: Ref<Window> = load_as(xaml);
    let target = window.get_control::<TextBlock>("text");

    window.apply_template();
    window.presenter().unwrap().apply_template();
    target.apply_template();

    // The assertion is commented out in the original:
    // assert_eq!(Some("test"), target.text().as_deref());
}

#[test]
fn resolves_relative_source_binding_from_template() {
    let _app = styled_window_application();
    let xaml = r#"
<ContentControl xmlns='https://github.com/ferroui'
                Focusable='True'>
    <ContentControl.Styles>
        <Style Selector='ContentControl'>
            <Setter Property='Template'>
                <ControlTemplate>
                    <ContentPresenter Focusable='{CompiledBinding !Focusable, RelativeSource={RelativeSource TemplatedParent}}' />
                </ControlTemplate>
            </Setter>
        </Style>
    </ContentControl.Styles>
</ContentControl>"#;

    let content_control: Ref<ContentControl> = parse(xaml);
    content_control.set_data_context(Some(TestDataContext::new())); // should be ignored
    content_control.measure(Size::new(10.0, 10.0));

    let result = first_content_presenter(content_control.get_template_descendants());
    assert!(!result.focusable());
}

#[test]
fn resolves_relative_source_binding_from_style_selector() {
    let _app = styled_window_application();
    let xaml = r#"
<TextBox xmlns='https://github.com/ferroui'
         xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'
         InnerLeftContent='Hello'>
    <TextBox.Styles>
        <Style Selector='TextBox'>
            <Setter Property='Template'>
                <ControlTemplate>
                    <StackPanel>
                        <ContentPresenter x:Name='Content' />
                        <TextPresenter x:Name='PART_TextPresenter' />
                    </StackPanel>
                </ControlTemplate>
            </Setter>
            <Style Selector='^ /template/ ContentPresenter#Content'>
                <Setter Property='Content' Value='{CompiledBinding InnerLeftContent, RelativeSource={RelativeSource TemplatedParent}}' />
            </Style>
        </Style>
    </TextBox.Styles>
</TextBox>"#;

    let text_box: Ref<TextBox> = parse(xaml);
    text_box.set_data_context(Some(TestDataContext::new())); // should be ignored
    text_box.measure(Size::new(10.0, 10.0));

    let result = first_content_presenter(text_box.get_template_descendants());
    assert!(text_box.inner_left_content() == result.content());
}

#[test]
fn binds_to_templated_parent_from_non_control() {
    let _app = styled_window_application();
    let xaml = r#"
<Window xmlns='https://github.com/ferroui'
        xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'
        xmlns:local='clr-namespace:FerroUI.Markup.Xaml.UnitTests.Xaml;assembly=FerroUI.Markup.Xaml.UnitTests'>
    <Button Name='button'>
      <Button.Template>
        <ControlTemplate>
          <Grid>
            <Grid.ColumnDefinitions>
              <ColumnDefinition Width='{CompiledBinding RelativeSource={RelativeSource TemplatedParent}, Path=Tag}'/>
            </Grid.ColumnDefinitions>
          </Grid>
        </ControlTemplate>
      </Button.Template>
    </Button>
</Window>"#;
    let window: Ref<Window> = load_as(xaml);
    let button = window.get_control::<Button>("button");

    button.set_tag(Some(boxed(GridLength::new(5.0, GridUnitType::Star))));

    window.apply_template();
    button.apply_template();

    let grid = button.get_template_descendants().into_iter().find_map(|visual| visual.cast::<Grid>()).unwrap();
    assert_eq!(value_of::<GridLength>(&button.tag()), Some(grid.column_definitions().get(0).width()));
}

#[test]
fn resolves_element_name_in_template() {
    let _app = styled_window_application();
    let xaml = r#"
<ContentControl xmlns='https://github.com/ferroui'
                Content='Hello'>
    <ContentControl.Styles>
        <Style Selector='ContentControl'>
            <Setter Property='Template'>
                <ControlTemplate>
                    <Panel>
                        <TextBox Name='InnerTextBox' Text='Hello' />
                        <ContentPresenter Content='{CompiledBinding Text, ElementName=InnerTextBox}' />
                    </Panel>
                </ControlTemplate>
            </Setter>
        </Style>
    </ContentControl.Styles>
</ContentControl>"#;

    let content_control: Ref<ContentControl> = parse(xaml);
    content_control.measure(Size::new(10.0, 10.0));

    let result = first_content_presenter(content_control.get_template_descendants());

    assert_string("Hello", &result.content());
}

#[test]
fn binds_to_source() {
    let _app = styled_window_application();
    let xaml = r#"
<Window xmlns='https://github.com/ferroui'
        xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'
        xmlns:local='clr-namespace:FerroUI.Markup.Xaml.UnitTests.MarkupExtensions;assembly=FerroUI.Markup.Xaml.UnitTests'
        x:DataType='local:TestDataContext'
        Title='test'>
    <TextBlock Text='{CompiledBinding Length, Source=Test}' x:Name='text'/>
</Window>"#;
    let window: Ref<Window> = load_as(xaml);
    let target = window.get_control::<TextBlock>("text");

    window.apply_template();
    window.presenter().unwrap().apply_template();
    target.apply_template();

    assert_eq!(Some("Test".len().to_string()), target.text());
}

#[test]
fn binds_to_source_static_resource() {
    let _app = styled_window_application();
    let xaml = r#"
<Window xmlns='https://github.com/ferroui'
             xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'
             xmlns:local='using:FerroUI.Markup.Xaml.UnitTests.MarkupExtensions'
             x:CompileBindings='True'>
    <Window.Resources>
        <local:TestDataContext x:Key='dataKey' StringProperty='foobar'/>
    </Window.Resources>
    <TextBlock Name='textBlock' Text='{Binding StringProperty, Source={StaticResource dataKey}}'/>
</Window>"#;

    let window: Ref<Window> = load_as(xaml);
    let text_block = window.get_control::<TextBlock>("textBlock");

    assert_eq!(Some("foobar"), text_block.text().as_deref());
}

#[test]
fn binds_to_source_static_resource1() {
    let _app = styled_window_application();
    let xaml = r#"
<Window xmlns='https://github.com/ferroui'
             xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'
             xmlns:local='using:FerroUI.Markup.Xaml.UnitTests.MarkupExtensions'
             x:CompileBindings='True'>
    <Window.Resources>
        <local:TestDataContext x:Key='dataKey' StringProperty='foobar'/>
        <x:String x:Key='otherObjectKey'>test</x:String>
    </Window.Resources>
    <TextBlock Name='textBlock' Text='{Binding StringProperty, Source={StaticResource dataKey}}'/>
</Window>"#;

    let window: Ref<Window> = load_as(xaml);
    let text_block = window.get_control::<TextBlock>("textBlock");

    assert_eq!(Some("foobar"), text_block.text().as_deref());
}

#[test]
fn binds_to_source_static_resource_in_resource_dictionary() {
    let _app = styled_window_application();
    let xaml = r#"
<Window xmlns='https://github.com/ferroui'
             xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'
             xmlns:local='using:FerroUI.Markup.Xaml.UnitTests.MarkupExtensions'
             x:DataType='local:TestDataContext' x:CompileBindings='True'>
    <Window.Resources>
        <ResourceDictionary>
            <local:TestDataContext x:Key='dataKey' StringProperty='foobar'/>
        </ResourceDictionary>
    </Window.Resources>
    <TextBlock Name='textBlock' Text='{Binding StringProperty, Source={StaticResource dataKey}}'/>
</Window>"#;

    let window: Ref<Window> = load_as(xaml);
    let text_block = window.get_control::<TextBlock>("textBlock");

    assert_eq!(Some("foobar"), text_block.text().as_deref());
}

#[test]
fn binds_to_source_static_resource_in_resource_dictionary1() {
    let _app = styled_window_application();
    let xaml = r#"
<Window xmlns='https://github.com/ferroui'
             xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'
             xmlns:local='using:FerroUI.Markup.Xaml.UnitTests.MarkupExtensions'
             x:DataType='local:TestDataContext' x:CompileBindings='True'>
    <Window.Resources>
        <ResourceDictionary>
            <local:TestDataContext x:Key='dataKey' StringProperty='foobar'/>
            <x:String x:Key='otherObjectKey'>test</x:String>
        </ResourceDictionary>
    </Window.Resources>
    <TextBlock Name='textBlock' Text='{Binding StringProperty, Source={StaticResource dataKey}}'/>
</Window>"#;

    let window: Ref<Window> = load_as(xaml);
    let text_block = window.get_control::<TextBlock>("textBlock");

    assert_eq!(Some("foobar"), text_block.text().as_deref());
}

#[test]
fn binds_to_source_x_static() {
    let _app = styled_window_application();
    let xaml = r#"
<Window xmlns='https://github.com/ferroui'
             xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'
             xmlns:local='using:FerroUI.Markup.Xaml.UnitTests.MarkupExtensions'
             x:CompileBindings='True'>
    <ContentControl Name='contentControl' Content='{Binding Color, Source={x:Static Brushes.Red}}'/>
</Window>"#;

    let window: Ref<Window> = load_as(xaml);
    let content_control = window.get_control::<ContentControl>("contentControl");

    assert_value(Brushes::red().color(), &content_control.content());
}

#[test]
fn compiles_binding_when_requested() {
    let _app = styled_window_application();
    let xaml = r#"
<Window xmlns='https://github.com/ferroui'
        xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'
        xmlns:local='clr-namespace:FerroUI.Markup.Xaml.UnitTests.MarkupExtensions;assembly=FerroUI.Markup.Xaml.UnitTests'
        x:DataType='local:TestDataContext'
        x:CompileBindings='true'>
    <TextBlock Text='{Binding StringProperty}' Name='textBlock' />
</Window>"#;
    let window: Ref<Window> = load_as(xaml);
    let text_block = window.get_control::<TextBlock>("textBlock");

    let data_context = TestDataContext::new();
    data_context.set_string_property(Some("foobar".to_string()));

    window.set_data_context(Some(data_context.clone()));

    assert_eq!(data_context.string_property(), text_block.text());
}

#[test]
fn bool_property_getter_uses_cached_boxes() {
    let _app = styled_window_application();
    let xaml = r#"
<Window xmlns='https://github.com/ferroui'
        xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'
        xmlns:local='clr-namespace:FerroUI.Markup.Xaml.UnitTests.MarkupExtensions;assembly=FerroUI.Markup.Xaml.UnitTests'
        x:DataType='local:TestDataContext'>
    <TextBlock Tag='{CompiledBinding BoolProperty}' Name='textBlock' />
</Window>"#;
    let window: Ref<Window> = load_as(xaml);
    let text_block = window.get_control::<TextBlock>("textBlock");

    let with_bool_property = |value: bool| {
        let data_context = TestDataContext::new();
        data_context.set_bool_property(value);
        data_context
    };

    window.set_data_context(Some(with_bool_property(true)));
    let boxed_true = text_block.tag();
    window.set_data_context(Some(with_bool_property(false)));
    let boxed_false = text_block.tag();

    assert_value(true, &boxed_true);
    assert_value(false, &boxed_false);

    // The getter must return the cached boxes instead of allocating a new box per read.
    window.set_data_context(Some(with_bool_property(true)));
    assert!(is_same_box(&boxed_true, &text_block.tag()));
    window.set_data_context(Some(with_bool_property(false)));
    assert!(is_same_box(&boxed_false, &text_block.tag()));
}

#[test]
fn throws_on_invalid_binding_path_on_compiled_binding_enabled_via_directive() {
    let _app = styled_window_application();
    let xaml = r#"
<Window xmlns='https://github.com/ferroui'
        xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'
        xmlns:local='clr-namespace:FerroUI.Markup.Xaml.UnitTests.MarkupExtensions;assembly=FerroUI.Markup.Xaml.UnitTests'
        x:DataType='local:TestDataContext'
        x:CompileBindings='true'>
    <TextBlock Text='{Binding InvalidPath}' Name='textBlock' />
</Window>"#;
    assert_throws_xaml_diagnostic(
        try_load(xaml),
        "FRN2000",
        "Unable to resolve property or method of name 'InvalidPath' on type 'FerroUI.Markup.Xaml.UnitTests.MarkupExtensions.TestDataContext'. Line 7, position 16.",
    );
}

#[test]
fn support_parent_in_path() {
    let _app = styled_window_application();
    let xaml = r#"
<Window xmlns='https://github.com/ferroui'
        xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'
        xmlns:local='clr-namespace:FerroUI.Markup.Xaml.UnitTests.MarkupExtensions;assembly=FerroUI.Markup.Xaml.UnitTests'
        Title='foo'>
    <ContentControl Content='{CompiledBinding $parent.Title}' Name='contentControl' />
</Window>"#;
    let window: Ref<Window> = load_as(xaml);
    let content_control = window.get_control::<ContentControl>("contentControl");

    assert_string("foo", &content_control.content());
}

#[test]
fn supports_parent_in_path_with_type_and_level_filter() {
    let _app = styled_window_application();
    let window: Ref<Window> = load_as(
        r#"
<Window xmlns='https://github.com/ferroui'
        xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'>
    <Border x:Name='p2'>
        <Border x:Name='p1'>
            <Button x:Name='p0'>
                <TextBlock x:Name='textBlock' Text='{CompiledBinding $parent[Control;1].Name}' />
            </Button>
        </Border>
    </Border>
</Window>"#,
    );
    let text_block = window.get_control::<TextBlock>("textBlock");

    assert_eq!(Some("p1"), text_block.text().as_deref());
}

#[test]
fn support_converter_with_parameter() {
    let _app = styled_window_application();
    let xaml = r#"
<Window xmlns='https://github.com/ferroui'
        xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'
        xmlns:local='clr-namespace:FerroUI.Markup.Xaml.UnitTests.MarkupExtensions;assembly=FerroUI.Markup.Xaml.UnitTests'
        x:DataType='local:TestDataContext' x:CompileBindings='True'>
    <TextBlock Name='textBlock' Text='{Binding StringProperty, Converter={x:Static local:AppendConverter.Instance}, ConverterParameter=Bar}'/>
</Window>"#;

    let window: Ref<Window> = load_as(xaml);
    let text_block = window.get_control::<TextBlock>("textBlock");

    let data_context = TestDataContext::new();
    data_context.set_string_property(Some("Foo".to_string()));
    window.set_data_context(Some(data_context));

    assert_eq!(Some(format!("Foo+Bar+{}", CultureInfo::current_culture())), text_block.text());
}

#[test]
fn support_converter_with_culture() {
    let _app = styled_window_application();
    let xaml = r#"
<Window xmlns='https://github.com/ferroui'
        xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'
        xmlns:local='clr-namespace:FerroUI.Markup.Xaml.UnitTests.MarkupExtensions;assembly=FerroUI.Markup.Xaml.UnitTests'
        x:DataType='local:TestDataContext' x:CompileBindings='True'>
    <TextBlock Name='textBlock' Text='{Binding StringProperty, Converter={x:Static local:AppendConverter.Instance}, ConverterCulture=ar-SA}'/>
</Window>"#;

    let window: Ref<Window> = load_as(xaml);
    let text_block = window.get_control::<TextBlock>("textBlock");

    let data_context = TestDataContext::new();
    data_context.set_string_property(Some("Foo".to_string()));
    window.set_data_context(Some(data_context));

    assert_eq!(Some("Foo++ar-SA"), text_block.text().as_deref());
}

/// `Assert.Equal(expected, value)` for an untyped value that must be the
/// shared object `expected` (reference equality).
fn is_same_instance<T: PartialEq + 'static>(value: &Option<BoxedValue>, expected: &Rc<T>) -> bool {
    value_of::<Rc<T>>(value).is_some_and(|value| Rc::ptr_eq(&value, expected))
}

#[test]
fn throws_on_invalid_compile_bindings_directive() {
    let _app = styled_window_application();
    let xaml = r#"
<Window xmlns='https://github.com/ferroui'
        xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'
        xmlns:local='clr-namespace:FerroUI.Markup.Xaml.UnitTests.MarkupExtensions;assembly=FerroUI.Markup.Xaml.UnitTests'
        x:DataType='local:TestDataContext'
        x:CompileBindings='notabool'>
</Window>"#;
    assert_throws_xaml_diagnostic(
        try_load(xaml),
        "FRN2100",
        "The value of x:CompileBindings must be a literal boolean value. Line 6, position 9.",
    );
}

#[test]
fn support_cast_to_type_in_expression() {
    let _app = styled_window_application();
    let xaml = r#"
<Window xmlns='https://github.com/ferroui'
        xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'
        xmlns:local='using:FerroUI.Markup.Xaml.UnitTests.MarkupExtensions'
        x:DataType='local:TestDataContext'>
    <ContentControl Content='{CompiledBinding $parent.((local:TestDataContext)DataContext)}' Name='contentControl' />
</Window>"#;
    let window: Ref<Window> = load_as(xaml);
    let content_control = window.get_control::<ContentControl>("contentControl");

    let data_context = TestDataContext::new();

    window.set_data_context(Some(data_context.clone()));

    assert!(is_same_instance(&content_control.content(), &data_context));
}

#[test]
fn support_cast_to_nested_type_in_expression() {
    let _app = styled_window_application();
    let xaml = r#"
<Window xmlns='https://github.com/ferroui'
        xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'
        xmlns:local='using:FerroUI.Markup.Xaml.UnitTests.MarkupExtensions'>
    <ContentControl Content='{CompiledBinding $parent.((local:OuterClass+NestedClass)DataContext).NestedProperty}' Name='contentControl' />
</Window>"#;
    let window: Ref<Window> = load_as(xaml);
    let content_control = window.get_control::<ContentControl>("contentControl");

    let data_context = NestedClass::new();
    data_context.set_nested_property("hello".to_string());
    window.set_data_context(Some(data_context));

    assert_string("hello", &content_control.content());
}

#[test]
fn support_cast_to_type_in_expression_different_type_evaluates_to_null() {
    let _app = styled_window_application();
    let xaml = r#"
<Window xmlns='https://github.com/ferroui'
        xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'
        xmlns:local='using:FerroUI.Markup.Xaml.UnitTests.MarkupExtensions'
        x:DataType='local:TestDataContext'>
    <ContentControl Content='{CompiledBinding $parent.((local:TestDataContext)DataContext)}' Name='contentControl' />
</Window>"#;
    let window: Ref<Window> = load_as(xaml);
    let content_control = window.get_control::<ContentControl>("contentControl");

    let data_context = boxed_str("foo");

    window.set_data_context(data_context);

    assert!(content_control.content().is_none());
}

#[test]
fn support_cast_to_type_in_expression_with_property() {
    let _app = styled_window_application();
    let xaml = r#"
<Window xmlns='https://github.com/ferroui'
        xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'
        xmlns:local='using:FerroUI.Markup.Xaml.UnitTests.MarkupExtensions'
        x:DataType='local:TestDataContext'>
    <ContentControl Content='{CompiledBinding $parent.((local:TestDataContext)DataContext).StringProperty}' Name='contentControl' />
</Window>"#;
    let window: Ref<Window> = load_as(xaml);
    let content_control = window.get_control::<ContentControl>("contentControl");

    let data_context = TestDataContext::new();
    data_context.set_string_property(Some("foobar".to_string()));

    window.set_data_context(Some(data_context.clone()));

    assert_eq!(data_context.string_property(), string_of(&content_control.content()));
}

#[test]
fn support_cast_to_type_in_expression_with_property1() {
    let _app = styled_window_application();
    let xaml = r#"
<Window xmlns='https://github.com/ferroui'
        xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'
        xmlns:local='using:FerroUI.Markup.Xaml.UnitTests.MarkupExtensions'
        x:DataType='local:TestDataContext'>
    <ContentControl Content='{CompiledBinding $parent.DataContext(local:TestDataContext).StringProperty}' Name='contentControl' />
</Window>"#;
    let window: Ref<Window> = load_as(xaml);
    let content_control = window.get_control::<ContentControl>("contentControl");

    let data_context = TestDataContext::new();
    data_context.set_string_property(Some("foobar".to_string()));

    window.set_data_context(Some(data_context.clone()));

    assert_eq!(data_context.string_property(), string_of(&content_control.content()));
}

#[test]
fn support_cast_to_type_in_expression_with_property_indexer() {
    let _app = styled_window_application();
    let xaml = r#"
<Window xmlns='https://github.com/ferroui'
        xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'
        xmlns:local='using:FerroUI.Markup.Xaml.UnitTests.MarkupExtensions'
        x:DataType='local:TestDataContext'>
    <ContentControl Content='{CompiledBinding ((local:TestData)ObjectsArrayProperty[0]).StringProperty}' Name='contentControl' />
</Window>"#;
    let window: Ref<Window> = load_as(xaml);
    let content_control = window.get_control::<ContentControl>("contentControl");

    let data = TestData::new();
    data.set_string_property(Some("Foo".to_string()));
    let data_context = TestDataContext::new();
    let objects: Vec<Option<BoxedValue>> = vec![Some(data.clone())];
    data_context.set_objects_array_property(Some(BindableArray::new(objects)));

    window.set_data_context(Some(data_context));

    assert_eq!(data.string_property(), string_of(&content_control.content()));
}

#[test]
fn support_cast_to_type_in_expression_with_property_different_type_evaluates_to_null() {
    let _app = styled_window_application();
    let xaml = r#"
<Window xmlns='https://github.com/ferroui'
        xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'
        xmlns:local='using:FerroUI.Markup.Xaml.UnitTests.MarkupExtensions'
        x:DataType='local:TestDataContext'>
    <ContentControl Content='{CompiledBinding $parent.((local:TestDataContext)DataContext).StringProperty}' Name='contentControl' />
</Window>"#;
    let window: Ref<Window> = load_as(xaml);
    let content_control = window.get_control::<ContentControl>("contentControl");

    let data_context = TestDataContext::new();
    data_context.set_string_property(Some("foobar".to_string()));

    window.set_data_context(Some(data_context.clone()));

    assert_eq!(data_context.string_property(), string_of(&content_control.content()));

    window.set_data_context(boxed_str("foo"));

    assert!(content_control.content().is_none());
}

#[test]
fn supports_empty_path() {
    let _app = styled_window_application();
    let xaml = r#"
<Window xmlns='https://github.com/ferroui'
        xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'
        xmlns:local='clr-namespace:FerroUI.Markup.Xaml.UnitTests.MarkupExtensions;assembly=FerroUI.Markup.Xaml.UnitTests'
        x:DataType='local:TestDataContext'>
    <TextBlock Text='{CompiledBinding}' Name='textBlock' />
</Window>"#;
    let window: Ref<Window> = load_as(xaml);
    let text_block = window.get_control::<TextBlock>("textBlock");

    let data_context = TestDataContext::new();
    data_context.set_string_property(Some("foobar".to_string()));

    window.set_data_context(Some(data_context.clone()));

    assert_eq!(Some(<TestDataContext as MarkupTyped>::MARKUP.full_name()), text_block.text());
}

#[test]
fn supports_empty_path_with_string_format() {
    let _app = styled_window_application();
    let xaml = r#"
<Window xmlns='https://github.com/ferroui'
        xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'
        xmlns:local='clr-namespace:FerroUI.Markup.Xaml.UnitTests.MarkupExtensions;assembly=FerroUI.Markup.Xaml.UnitTests'
        x:DataType='local:TestDataContext'>
    <TextBlock Text='{CompiledBinding StringFormat=bar-\{0\}}' Name='textBlock' />
</Window>"#;
    let window: Ref<Window> = load_as(xaml);
    let text_block = window.get_control::<TextBlock>("textBlock");

    let data_context = TestDataContext::new();
    data_context.set_string_property(Some("foobar".to_string()));

    window.set_data_context(Some(data_context.clone()));

    assert_eq!(Some(format!("bar-{}", <TestDataContext as MarkupTyped>::MARKUP.full_name())), text_block.text());
}

#[test]
fn supports_dot_path() {
    let _app = styled_window_application();
    let xaml = r#"
<Window xmlns='https://github.com/ferroui'
        xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'
        xmlns:local='clr-namespace:FerroUI.Markup.Xaml.UnitTests.MarkupExtensions;assembly=FerroUI.Markup.Xaml.UnitTests'
        x:DataType='local:TestDataContext'>
    <TextBlock Text='{CompiledBinding .}' Name='textBlock' />
</Window>"#;
    let window: Ref<Window> = load_as(xaml);
    let text_block = window.get_control::<TextBlock>("textBlock");

    let data_context = TestDataContext::new();
    data_context.set_string_property(Some("foobar".to_string()));

    window.set_data_context(Some(data_context.clone()));

    assert_eq!(Some(<TestDataContext as MarkupTyped>::MARKUP.full_name()), text_block.text());
}

#[test]
fn supports_explicit_dot_path_with_string_format() {
    let _app = styled_window_application();
    let xaml = r#"
<Window xmlns='https://github.com/ferroui'
        xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'
        xmlns:local='clr-namespace:FerroUI.Markup.Xaml.UnitTests.MarkupExtensions;assembly=FerroUI.Markup.Xaml.UnitTests'
        x:DataType='local:TestDataContext'>
    <TextBlock Text='{CompiledBinding Path=., StringFormat=bar-\{0\}}' Name='textBlock' />
</Window>"#;
    let window: Ref<Window> = load_as(xaml);
    let text_block = window.get_control::<TextBlock>("textBlock");

    let data_context = TestDataContext::new();
    data_context.set_string_property(Some("foobar".to_string()));

    window.set_data_context(Some(data_context.clone()));

    assert_eq!(Some(format!("bar-{}", <TestDataContext as MarkupTyped>::MARKUP.full_name())), text_block.text());
}

#[test]
fn support_cast_to_type_in_expression_with_property_explicit_property_cast() {
    let _app = styled_window_application();
    let xaml = r#"
<Window xmlns='https://github.com/ferroui'
        xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'
        xmlns:local='using:FerroUI.Markup.Xaml.UnitTests.MarkupExtensions'>
    <ContentControl Content='{CompiledBinding $parent.((local:IHasExplicitProperty)DataContext).ExplicitProperty}' Name='contentControl' />
</Window>"#;
    let window: Ref<Window> = load_as(xaml);
    let content_control = window.get_control::<ContentControl>("contentControl");

    let data_context = TestDataContext::new();

    window.set_data_context(Some(data_context.clone()));

    assert_eq!(
        Some(data_context.as_has_explicit_property().explicit_property()),
        string_of(&content_control.content())
    );
}

#[test]
fn binds_to_self() {
    let _app = styled_window_application();
    let xaml = r#"
<Window xmlns='https://github.com/ferroui'
        xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'
        xmlns:local='clr-namespace:FerroUI.Markup.Xaml.UnitTests.MarkupExtensions;assembly=FerroUI.Markup.Xaml.UnitTests'
        x:DataType='local:TestDataContext'>
    <TextBlock Name='textBlock' Text='{CompiledBinding $self}' />
</Window>"#;
    let window: Ref<Window> = load_as(xaml);
    let text_block = window.get_control::<TextBlock>("textBlock");

    window.apply_template();
    window.presenter().unwrap().apply_template();

    assert_eq!(Some("FerroUI.Controls.TextBlock"), text_block.text().as_deref());
}

#[test]
fn binds_to_self_without_data_type() {
    let _app = styled_window_application();
    let xaml = r#"
<Window xmlns='https://github.com/ferroui'
        xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'
        xmlns:local='clr-namespace:FerroUI.Markup.Xaml.UnitTests.MarkupExtensions;assembly=FerroUI.Markup.Xaml.UnitTests'>
    <TextBlock Name='textBlock' Text='{CompiledBinding $self.Name}' />
</Window>"#;
    let window: Ref<Window> = load_as(xaml);
    let text_block = window.get_control::<TextBlock>("textBlock");

    window.apply_template();
    window.presenter().unwrap().apply_template();

    assert_eq!(text_block.name(), text_block.text());
}

#[test]
fn binds_to_self_in_style() {
    let _app = styled_window_application();
    let xaml = r#"
<Window xmlns='https://github.com/ferroui'
        xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'>

    <Window.Styles>
        <Style Selector='Button'>
            <Setter Property='IsVisible' Value='{CompiledBinding $self.IsEnabled}' />
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
fn binds_to_relative_source_self() {
    let _app = styled_window_application();
    let xaml = r#"
<Window xmlns='https://github.com/ferroui'
        xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'
        xmlns:local='clr-namespace:FerroUI.Markup.Xaml.UnitTests.MarkupExtensions;assembly=FerroUI.Markup.Xaml.UnitTests'
        x:DataType='local:TestDataContext'>
    <TextBlock Name='textBlock' Text='{CompiledBinding RelativeSource={RelativeSource Self}}' />
</Window>"#;
    let window: Ref<Window> = load_as(xaml);
    let text_block = window.get_control::<TextBlock>("textBlock");

    window.apply_template();
    window.presenter().unwrap().apply_template();

    assert_eq!(Some("FerroUI.Controls.TextBlock"), text_block.text().as_deref());
}

#[test]
fn binds_to_relative_source_self_in_multi_binding() {
    for compile_bindings in [false, true] {
        let _app = styled_window_application();
        let xaml = r#"
<Window xmlns='https://github.com/ferroui'
        xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'
        x:CompileBindings='{compileBindings}'>
  <StackPanel>
    <TextBlock Name='textBlock'>
      <TextBlock.Text>
        <MultiBinding StringFormat="{} $self = {0}, $parent = {1}">
          <Binding Path="$self.FontStyle"/>
          <Binding Path="$parent.Orientation"/>
        </MultiBinding>
      </TextBlock.Text>
    </TextBlock>
  </StackPanel>
</Window>"#
        .replace("{compileBindings}", if compile_bindings { "True" } else { "False" });
        let window: Ref<Window> = load_as(&xaml);
        let text_block = window.get_control::<TextBlock>("textBlock");

        let data_context = TestDataContext::new();
        window.set_data_context(Some(data_context));

        assert_eq!(
            Some(" $self = Normal, $parent = Vertical"),
            text_block.get_value(TextBlock::text_property()).as_deref(),
            "compileBindings: {compile_bindings}"
        );
    }
}

#[test]
fn binds_to_relative_source_self_in_multi_binding_in_style() {
    for compile_bindings in [false, true] {
        let _app = styled_window_application();
        let xaml = r#"
<Window xmlns='https://github.com/ferroui'
        xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'
        x:CompileBindings='{compileBindings}'>
  <Window.Styles>
    <Style Selector='TextBlock'>
        <Setter Property='Text'>
          <MultiBinding StringFormat="{} $self = {0}">
            <Binding Path="$self.FontStyle"/>
          </MultiBinding>
        </Setter>
    </Style>
  </Window.Styles>
  <StackPanel>
    <TextBlock Name='textBlock'/>
  </StackPanel>
</Window>"#
        .replace("{compileBindings}", if compile_bindings { "True" } else { "False" });
        let window: Ref<Window> = load_as(&xaml);
        let text_block = window.get_control::<TextBlock>("textBlock");

        let data_context = TestDataContext::new();
        window.set_data_context(Some(data_context));

        assert_eq!(
            Some(" $self = Normal"),
            text_block.get_value(TextBlock::text_property()).as_deref(),
            "compileBindings: {compile_bindings}"
        );
    }
}

#[test]
fn supports_method_binding_as_delegate() {
    let _app = styled_window_application();
    let xaml = r#"
<Window xmlns='https://github.com/ferroui'
        xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'
        xmlns:local='clr-namespace:FerroUI.Markup.Xaml.UnitTests.MarkupExtensions;assembly=FerroUI.Markup.Xaml.UnitTests'
        x:DataType='local:MethodDataContext'>
    <StackPanel>
        <ContentControl Content='{CompiledBinding Action}' Name='action' />
        <ContentControl Content='{CompiledBinding Func}' Name='func' />
        <ContentControl Content='{CompiledBinding Func2}' Name='func2' />
        <ContentControl Content='{CompiledBinding CustomDelegateTypeVoid}' Name='customvoid' />
        <ContentControl Content='{CompiledBinding CustomDelegateTypeInt}' Name='customint' />
    </StackPanel>
</Window>"#;
    let window: Ref<Window> = load_as(xaml);
    window.set_data_context(Some(MethodDataContext::new()));

    // The port has one delegate type, the untyped delegate of markup metadata, so the
    // delegate types of the original (`Action`, `Func<object>`, `Func<object, object>`) are
    // told apart by their signature: the parameters the delegate takes and what it returns.
    let delegate_of = |name: &str| {
        let content = window.get_control::<ContentControl>(name).content();
        assert!(is_type::<MarkupDelegate>(&content), "the content of '{name}' is not a delegate");
        value_of::<MarkupDelegate>(&content).expect("a delegate")
    };
    let argument: Option<BoxedValue> = Some(Rc::new("argument".to_string()));

    // `Action`: no parameters, no result.
    assert!(delegate_of("action").invoke(&[]).is_none());
    // `Func<object>`: no parameters, the result of `Func()`.
    assert_value(1_i32, &delegate_of("func").invoke(&[]));
    // `Func<object, object>`: one parameter, the result of `Func2(i)` (its argument).
    assert_string("argument", &delegate_of("func2").invoke(&[argument]));
    // The custom delegate types are only required to be delegates.
    assert!(is_type::<MarkupDelegate>(&window.get_control::<ContentControl>("customvoid").content()));
    assert!(is_type::<MarkupDelegate>(&window.get_control::<ContentControl>("customint").content()));
}

#[test]
fn binding_method_to_command_works() {
    let _app = styled_window_application();
    let xaml = r#"
<Window xmlns='https://github.com/ferroui'
        xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'
        xmlns:local='clr-namespace:FerroUI.Markup.Xaml.UnitTests.MarkupExtensions;assembly=FerroUI.Markup.Xaml.UnitTests'
        x:DataType='local:MethodAsCommandDataContext'>
    <Button Name='button' Command='{CompiledBinding Method}'/>
</Window>"#;
    let window: Ref<Window> = load_as(xaml);
    let button = window.get_control::<Button>("button");
    let vm = MethodAsCommandDataContext::new();

    button.set_data_context(Some(vm.clone()));
    window.apply_template();

    assert!(button.command().is_some());
    perform_click(&button);
    assert_eq!("Called", vm.value());
}

#[test]
fn binding_method_with_parameter_to_command_uses_single_parameter_overload() {
    let rows = [
        ("ObjectMethod", "<x:String>hello</x:String>", "Called ObjectMethod with hello"),
        ("StringMethod", "<x:String>hello</x:String>", "Called StringMethod with hello"),
        ("StringMethod", "<x:Null />", "Called StringMethod with "),
        ("Int32Method", "<x:Int32>42</x:Int32>", "Called Int32Method with 42"),
        ("VirtualObjectMethod", "<x:String>hello</x:String>", "Called VirtualObjectMethod with hello"),
        ("VirtualStringMethod", "<x:String>hello</x:String>", "Called VirtualStringMethod with hello"),
        ("VirtualStringMethod", "<x:Null />", "Called VirtualStringMethod with "),
        ("VirtualInt32Method", "<x:Int32>42</x:Int32>", "Called VirtualInt32Method with 42"),
        ("MethodWithNewSlot", "<x:Int32>42</x:Int32>", "Called MethodWithNewSlot with 42"),
    ];
    for (method_name, xaml_parameter, expected) in rows {
        let row = format!("row ({method_name}, {xaml_parameter})");
        let _app = styled_window_application();

        let xaml = r#"<Window xmlns='https://github.com/ferroui'
        xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'
        xmlns:local='clr-namespace:FerroUI.Markup.Xaml.UnitTests.MarkupExtensions;assembly=FerroUI.Markup.Xaml.UnitTests'
        x:DataType='local:MethodAsCommandDataContext'>
    <Button Name='button' Command='{CompiledBinding {{methodName}}}'>
      <Button.CommandParameter>
        {{xamlParameter}}
      </Button.CommandParameter>
    </Button>
</Window>"#
        .replace("{{methodName}}", method_name)
        .replace("{{xamlParameter}}", xaml_parameter);
        let window: Ref<Window> = load_as(&xaml);
        let button = window.get_control::<Button>("button");
        let vm = MethodAsCommandDataContext::new();

        button.set_data_context(Some(vm.clone()));
        window.apply_template();

        assert!(button.command().is_some(), "{row}");
        perform_click(&button);
        assert_eq!(expected, vm.value(), "{row}");
    }
}

/// The body of the theory `Binding_Method_With_Parameter_To_Command_With_Single_Parameter_Overload_
/// Throws_At_Runtime_If_Mismatched_Types`: the click must throw (an exception that escapes the
/// click is a panic whose message names the exception; each row is a test that expects it).
fn mismatched_types_row(method_name: &str, xaml_parameter: &str) {
    let _app = styled_window_application();

    let xaml = r#"<Window xmlns='https://github.com/ferroui'
        xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'
        xmlns:local='clr-namespace:FerroUI.Markup.Xaml.UnitTests.MarkupExtensions;assembly=FerroUI.Markup.Xaml.UnitTests'
        x:DataType='local:MethodAsCommandDataContext'>
    <Button Name='button' Command='{CompiledBinding {{methodName}}}'>
      <Button.CommandParameter>
        {{xamlParameter}}
      </Button.CommandParameter>
    </Button>
</Window>"#
    .replace("{{methodName}}", method_name)
    .replace("{{xamlParameter}}", xaml_parameter);
    let window: Ref<Window> = load_as(&xaml);
    let button = window.get_control::<Button>("button");
    let vm = MethodAsCommandDataContext::new();

    button.set_data_context(Some(vm.clone()));
    window.apply_template();

    assert!(button.command().is_some());
    perform_click(&button);
}

#[test]
#[should_panic(expected = "InvalidCastException")]
fn binding_method_with_parameter_to_command_with_single_parameter_overload_throws_at_runtime_if_mismatched_types_int32_method_string(
) {
    mismatched_types_row("Int32Method", "<x:String>hello</x:String>");
}

#[test]
#[should_panic(expected = "NullReferenceException")]
fn binding_method_with_parameter_to_command_with_single_parameter_overload_throws_at_runtime_if_mismatched_types_int32_method_null(
) {
    mismatched_types_row("Int32Method", "<x:Null />");
}

#[test]
#[should_panic(expected = "InvalidCastException")]
fn binding_method_with_parameter_to_command_with_single_parameter_overload_throws_at_runtime_if_mismatched_types_string_method_int32(
) {
    mismatched_types_row("StringMethod", "<x:Int32>42</x:Int32>");
}

#[test]
fn binding_method_with_parameter_to_command_prefers_object_overload() {
    let _app = styled_window_application();

    let xaml = r#"<Window xmlns='https://github.com/ferroui'
        xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'
        xmlns:local='clr-namespace:FerroUI.Markup.Xaml.UnitTests.MarkupExtensions;assembly=FerroUI.Markup.Xaml.UnitTests'
        x:DataType='local:MethodAsCommandDataContext'>
    <Button Name='button' Command='{CompiledBinding MethodWithOverloads}' CommandParameter="foo" />
</Window>"#;
    let window: Ref<Window> = load_as(xaml);
    let button = window.get_control::<Button>("button");
    let vm = MethodAsCommandDataContext::new();

    button.set_data_context(Some(vm.clone()));
    window.apply_template();

    assert!(button.command().is_some());
    perform_click(&button);
    assert_eq!("Called MethodWithOverloads with Object foo", vm.value());
}

#[test]
fn binding_method_with_parameter_to_command_fails_with_multiple_single_parameter_overloads_without_object() {
    let _app = styled_window_application();

    let exception = assert_throws_xml_exception(try_load(
        r#"<Window xmlns='https://github.com/ferroui'
        xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'
        xmlns:local='clr-namespace:FerroUI.Markup.Xaml.UnitTests.MarkupExtensions;assembly=FerroUI.Markup.Xaml.UnitTests'
        x:DataType='local:MethodAsCommandDataContext'>
    <Button Name='button' Command='{CompiledBinding MethodWithOverloads2}' CommandParameter="foo" />
</Window>"#,
    ));

    let message = xaml_error(&exception).expect("the error of the compiler").message();
    let expected = concat!(
        "Unable to resolve method of name 'MethodWithOverloads2' on type 'FerroUI.Markup.Xaml.UnitTests.MarkupExtensions.MethodAsCommandDataContext'. ",
        "Found 2 overloads accepting one parameter: 'System.Int32', 'System.String'. ",
        "Expected either a single overload with one parameter, or an overload accepting System.Object.",
    );
    assert!(message.starts_with(expected), "{message}");
}

#[test]
fn binding_method_with_parameter_to_command_uses_parameterless_overload_when_no_overloads_with_parameter_exist() {
    let _app = styled_window_application();

    let xaml = r#"<Window xmlns='https://github.com/ferroui'
        xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'
        xmlns:local='clr-namespace:FerroUI.Markup.Xaml.UnitTests.MarkupExtensions;assembly=FerroUI.Markup.Xaml.UnitTests'
        x:DataType='local:MethodAsCommandDataContext'>
    <Button Name='button' Command='{CompiledBinding MethodWithOverloads3}' CommandParameter="foo" />
</Window>"#;
    let window: Ref<Window> = load_as(xaml);
    let button = window.get_control::<Button>("button");
    let vm = MethodAsCommandDataContext::new();

    button.set_data_context(Some(vm.clone()));
    window.apply_template();

    assert!(button.command().is_some());
    perform_click(&button);
    assert_eq!("Called MethodWithOverloads3 without parameter", vm.value());
}

#[test]
fn binding_method_with_parameter_to_command_fails_without_valid_overloads() {
    let _app = styled_window_application();

    let exception = assert_throws_xml_exception(try_load(
        r#"<Window xmlns='https://github.com/ferroui'
        xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'
        xmlns:local='clr-namespace:FerroUI.Markup.Xaml.UnitTests.MarkupExtensions;assembly=FerroUI.Markup.Xaml.UnitTests'
        x:DataType='local:MethodAsCommandDataContext'>
    <Button Name='button' Command='{CompiledBinding MethodWithOverloads4}' CommandParameter="foo" />
</Window>"#,
    ));

    let message = xaml_error(&exception).expect("the error of the compiler").message();
    let expected = concat!(
        "Unable to resolve method of name 'MethodWithOverloads4' on type 'FerroUI.Markup.Xaml.UnitTests.MarkupExtensions.MethodAsCommandDataContext'. ",
        "Found 2 overloads accepting more than one parameter. ",
        "Expected a method with zero or one parameter. ",
    );
    assert!(message.starts_with(expected), "{message}");
}

#[test]
fn binding_method_to_text_block_text_works() {
    let _app = styled_window_application();
    let xaml = r#"
<Window xmlns='https://github.com/ferroui'
        xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'
        xmlns:local='clr-namespace:FerroUI.Markup.Xaml.UnitTests.MarkupExtensions;assembly=FerroUI.Markup.Xaml.UnitTests'
        x:DataType='local:MethodAsCommandDataContext'>
    <TextBlock Name='textBlock' Text='{CompiledBinding Method}'/>
</Window>"#;
    let window: Ref<Window> = load_as(xaml);
    let text_block = window.get_control::<TextBlock>("textBlock");
    let vm = MethodAsCommandDataContext::new();

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
        xmlns:local='clr-namespace:FerroUI.Markup.Xaml.UnitTests.MarkupExtensions;assembly=FerroUI.Markup.Xaml.UnitTests'
        x:DataType='local:MethodAsCommandDataContext'>
    <Button Name='button' Command='{CompiledBinding Do}' CommandParameter='{CompiledBinding Parameter, Mode=OneTime}'/>
</Window>"#;
        let window: Ref<Window> = load_as(xaml);
        let button = window.get_control::<Button>("button");
        let vm = MethodAsCommandDataContext::new();
        vm.set_parameter(command_parameter);

        button.set_data_context(Some(vm.clone()));
        window.apply_template();

        assert!(button.command().is_some(), "row {result}");
        perform_click(&button);
        assert_eq!(vm.value(), result, "row {result}");
    }
}

#[test]
fn binding_method_with_parameter_to_command_can_execute_depends_on() {
    let _app = styled_window_application();
    let xaml = r#"
<Window xmlns='https://github.com/ferroui'
        xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'
        xmlns:local='clr-namespace:FerroUI.Markup.Xaml.UnitTests.MarkupExtensions;assembly=FerroUI.Markup.Xaml.UnitTests'
        x:DataType='local:MethodAsCommandDataContext'>
    <Button Name='button' Command='{CompiledBinding Do}'/>
</Window>"#;
    let window: Ref<Window> = load_as(xaml);
    let button = window.get_control::<Button>("button");
    let vm = MethodAsCommandDataContext::new();
    vm.set_parameter(None);

    button.set_data_context(Some(vm.clone()));
    window.apply_template();

    assert!(button.command().is_some());

    assert_eq!(button.is_effectively_enabled(), false);

    vm.set_parameter(obj(true));
    Dispatcher::ui_thread().run_jobs(None);

    assert_eq!(button.is_effectively_enabled(), true);
}

#[test]
fn binding_method_to_command_in_style_works() {
    let _app = styled_window_application();
    let xaml = r#"
<Window xmlns='https://github.com/ferroui'
        xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'
        xmlns:local='clr-namespace:FerroUI.Markup.Xaml.UnitTests.MarkupExtensions;assembly=FerroUI.Markup.Xaml.UnitTests'
        x:DataType='local:MethodAsCommandDataContext'>
    <Window.Styles>
        <Style Selector='Button'>
            <Setter Property='Command' Value='{CompiledBinding Method}'/>
        </Style>
    </Window.Styles>
    <Button Name='button'/>
</Window>"#;
    let window: Ref<Window> = load_as(xaml);
    let button = window.get_control::<Button>("button");
    let vm = MethodAsCommandDataContext::new();

    button.set_data_context(Some(vm.clone()));
    window.apply_template();

    assert!(button.command().is_some());
    perform_click(&button);
    assert_eq!("Called", vm.value());
}

#[test]
fn resolves_data_type_for_assign_binding() {
    let _app = styled_window_application();
    let xaml = r#"
<local:AssignBindingControl xmlns='https://github.com/ferroui'
        xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'
        xmlns:local='clr-namespace:FerroUI.Markup.Xaml.UnitTests.MarkupExtensions;assembly=FerroUI.Markup.Xaml.UnitTests'
        x:DataType='local:TestDataContext'
        X='{CompiledBinding StringProperty}' />"#;
    let control: Ref<AssignBindingControl> = load_as(xaml);
    let binding = control.x().expect("the binding is null");
    let (property_type, _) = single_property_element(&binding);
    assert!(property_type.is_string(), "the property type is {}", property_type.name());
}

#[test]
fn resolves_data_type_for_assign_binding_from_binding_property() {
    let _app = styled_window_application();
    let xaml = r#"
<local:AssignBindingControl xmlns='https://github.com/ferroui'
        xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'
        xmlns:local='clr-namespace:FerroUI.Markup.Xaml.UnitTests.MarkupExtensions;assembly=FerroUI.Markup.Xaml.UnitTests'
        X='{CompiledBinding StringProperty, DataType=local:TestDataContext}' />"#;
    let control: Ref<AssignBindingControl> = load_as(xaml);
    let binding = control.x().expect("the binding is null");
    let (property_type, _) = single_property_element(&binding);
    assert!(property_type.is_string(), "the property type is {}", property_type.name());
}

#[test]
fn uses_runtime_loader_configuration_to_enabled_compiled() {
    let _app = styled_window_application();
    let xaml = r#"
<local:AssignBindingControl xmlns='https://github.com/ferroui'
        xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'
        xmlns:local='clr-namespace:FerroUI.Markup.Xaml.UnitTests.MarkupExtensions;assembly=FerroUI.Markup.Xaml.UnitTests'
        X='{CompiledBinding StringProperty, DataType=local:TestDataContext}' />"#;
    let mut configuration = RuntimeXamlLoaderConfiguration::new();
    configuration.use_compiled_bindings_by_default = true;
    let control: Ref<AssignBindingControl> = cast(&load_document(document_without_uri(xaml), Some(configuration)));
    let binding = control.x().expect("the binding is null");
    let (property_type, _) = single_property_element(&binding);
    assert!(property_type.is_string(), "the property type is {}", property_type.name());
}

#[test]
fn should_bind_to_nested_generic_property() {
    let _app = styled_window_application();
    let xaml = r#"
<Window xmlns='https://github.com/ferroui'
        xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'
        xmlns:local='clr-namespace:FerroUI.Markup.Xaml.UnitTests.MarkupExtensions;assembly=FerroUI.Markup.Xaml.UnitTests'
        x:DataType='local:TestDataContext'
        x:CompileBindings='True'>
    <ComboBox x:Name='comboBox' ItemsSource='{Binding GenericProperty}' SelectedItem='{Binding GenericProperty.CurrentItem}' />
</Window>"#;
    let window: Ref<Window> = load_as(xaml);
    let combo_box = window.get_control::<ComboBox>("comboBox");

    let data_context = TestDataContext::new();
    data_context.generic_property().add(123);
    data_context.generic_property().set_current_item(123);
    window.set_data_context(Some(data_context));

    assert_eq!(Some(123), value_of::<i32>(&combo_box.selected_item()));
}

/// The body of the theory `Should_Use_StringFormat_Without_Braces`.
#[track_caller]
fn should_use_string_format_without_braces(compile_bindings: bool) {
    let _app = styled_window_application();
    let compile_bindings = if compile_bindings { "True" } else { "False" };
    let xaml = format!(
        r#"
<Window xmlns='https://github.com/ferroui'
        xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'
        xmlns:local='clr-namespace:FerroUI.Markup.Xaml.UnitTests.MarkupExtensions;assembly=FerroUI.Markup.Xaml.UnitTests'
        x:DataType='local:TestDataContext'
        x:CompileBindings='{compile_bindings}'>
    <TextBlock Name='textBlock' Text='{{Binding DecimalValue, StringFormat=c2}}'/>
</Window>"#
    );
    let window: Ref<Window> = load_as(&xaml);
    let text_block = window.get_control::<TextBlock>("textBlock");

    let data_context = TestDataContext::new();
    window.set_data_context(Some(data_context));

    // `string.Format("{0:c2}", TestDataContext.ExpectedDecimal)`.
    let expected = TestDataContext::expected_decimal().to_string_with("c2", None).expect("c2 is a valid format");
    assert_eq!(Some(expected), text_block.get_value(TextBlock::text_property()));
}

/// `[InlineData(false)]`.
#[test]
fn should_use_string_format_without_braces_false() {
    should_use_string_format_without_braces(false);
}

/// `[InlineData(true)]`.
#[test]
fn should_use_string_format_without_braces_true() {
    should_use_string_format_without_braces(true);
}

#[test]
fn should_negate_boolean_value() {
    for value in [true, false] {
        let _app = styled_window_application();
        let xaml = r#"
<Window xmlns='https://github.com/ferroui'
        xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'
        xmlns:local='clr-namespace:FerroUI.Markup.Xaml.UnitTests.MarkupExtensions;assembly=FerroUI.Markup.Xaml.UnitTests'
        x:DataType='local:TestDataContext'
        x:CompileBindings='True'>
    <TextBlock Name='textBlock' Tag='{Binding !BoolProperty}'/>
</Window>"#;
        let window: Ref<Window> = load_as(xaml);
        let text_block = window.get_control::<TextBlock>("textBlock");

        let data_context = TestDataContext::new();
        data_context.set_bool_property(value);
        window.set_data_context(Some(data_context));

        let tag = text_block.tag();
        assert!(is_type::<bool>(&tag), "row {value}");
        let result = value_of::<bool>(&tag);
        assert_eq!(Some(!value), result, "row {value}");
    }
}

#[test]
fn can_use_implicit_conversions() {
    let _app = styled_window_application();
    let xaml = r#"
<Window xmlns='https://github.com/ferroui'
        xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'
        xmlns:local='clr-namespace:FerroUI.Markup.Xaml.UnitTests.MarkupExtensions;assembly=FerroUI.Markup.Xaml.UnitTests'
        x:DataType='local:ImplicitConvertible'
        x:CompileBindings='True'>
    <TextBlock Name='textBlock'>
        <TextBlock.Background>
            <SolidColorBrush Color='{Binding}'/>
        </TextBlock.Background>
    </TextBlock>
</Window>"#;
    let window: Ref<Window> = load_as(xaml);
    let text_block = window.get_control::<TextBlock>("textBlock");

    let data_context = ImplicitConvertible::new("Green".to_string());
    window.set_data_context(Some(data_context));

    let background = text_block.background().expect("the background is null");
    let brush = background
        .as_object()
        .and_then(|object| object.downcast_ref::<SolidColorBrush>())
        .expect("the background is not a SolidColorBrush");
    assert_eq!(Colors::GREEN, brush.color());
}

#[test]
fn can_bind_brush_to_hex_string() {
    let _app = styled_window_application();
    let xaml = r#"
<Window xmlns='https://github.com/ferroui'
        xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'
        xmlns:local='clr-namespace:FerroUI.Markup.Xaml.UnitTests.MarkupExtensions;assembly=FerroUI.Markup.Xaml.UnitTests'
        x:DataType='local:TestData'
        x:CompileBindings='True'>
    <TextBlock Name='textBlock' Background='{Binding StringProperty}'/>
</Window>"#;
    let window: Ref<Window> = load_as(xaml);
    let text_block = window.find_control::<TextBlock>("textBlock");

    let data_context = TestData::new();
    data_context.set_string_property(Some("#ff0000".to_string()));
    window.set_data_context(Some(data_context));

    let background = text_block.unwrap().background().expect("the background is null");
    let brush = background
        .as_any()
        .downcast_ref::<ImmutableSolidColorBrush>()
        .expect("the background is not an ImmutableSolidColorBrush");
    assert_eq!(Colors::RED, brush.color());
}

#[test]
fn resolves_element_name_data_context_type_based_on_context() {
    let _app = styled_window_application();
    let xaml = r#"
<Window xmlns='https://github.com/ferroui'
        xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'
        xmlns:local='clr-namespace:FerroUI.Markup.Xaml.UnitTests.MarkupExtensions;assembly=FerroUI.Markup.Xaml.UnitTests'
        x:DataType='local:TestDataContext'
        x:Name='MyWindow'>
    <TextBlock Text='{CompiledBinding ElementName=MyWindow, Path=DataContext.StringProperty}' Name='textBlock' />
</Window>"#;
    let window: Ref<Window> = load_as(xaml);
    let text_block = window.get_control::<TextBlock>("textBlock");

    let data_context = TestDataContext::new();
    data_context.set_string_property(Some("foobar".to_string()));

    window.set_data_context(Some(data_context.clone()));

    assert_eq!(data_context.string_property(), text_block.text());
}

#[test]
fn resolves_element_name_data_context_type_based_on_context_short_syntax() {
    let _app = styled_window_application();
    let xaml = r#"
<Window xmlns='https://github.com/ferroui'
        xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'
        xmlns:local='clr-namespace:FerroUI.Markup.Xaml.UnitTests.MarkupExtensions;assembly=FerroUI.Markup.Xaml.UnitTests'
        x:DataType='local:TestDataContext'
        x:Name='MyWindow'>
    <TextBlock Text='{CompiledBinding #MyWindow.DataContext.StringProperty}' Name='textBlock' />
</Window>"#;
    let window: Ref<Window> = load_as(xaml);
    let text_block = window.get_control::<TextBlock>("textBlock");

    let data_context = TestDataContext::new();
    data_context.set_string_property(Some("foobar".to_string()));

    window.set_data_context(Some(data_context.clone()));

    assert_eq!(data_context.string_property(), text_block.text());
}

#[test]
fn type_cast_works_with_element_name_data_context() {
    // By default, DataContext will infer DataType from the XAML context, which will be local:TestDataContext here.
    // But developer should be able to re-define this type via type casing, if they know better.
    let _app = styled_window_application();
    let xaml = r#"
<Window xmlns='https://github.com/ferroui'
        xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'
        xmlns:local='clr-namespace:FerroUI.Markup.Xaml.UnitTests.MarkupExtensions;assembly=FerroUI.Markup.Xaml.UnitTests'
        x:DataType='local:TestDataContext'
        x:Name='MyWindow'>
    <Panel>
        <TextBlock Text='{CompiledBinding $parent.((Button)DataContext).Tag}' Name='textBlock' />
    </Panel>
</Window>"#;
    let window: Ref<Window> = load_as(xaml);
    let text_block = window.get_control::<TextBlock>("textBlock");

    let panel_data_context = Button::new();
    panel_data_context.set_tag(boxed_str("foo"));
    object_of::<Panel>(&window.content()).set_data_context(Some(Control::boxed(panel_data_context.clone())));

    assert_eq!(string_of(&panel_data_context.tag()), text_block.text());
}

#[test]
fn resolves_parent_data_context_type_based_on_context() {
    let _app = styled_window_application();
    let xaml = r#"
<Window xmlns='https://github.com/ferroui'
        xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'
        xmlns:local='clr-namespace:FerroUI.Markup.Xaml.UnitTests.MarkupExtensions;assembly=FerroUI.Markup.Xaml.UnitTests'
        x:DataType='local:TestDataContext'
        x:Name='MyWindow'>
    <Panel>
        <TextBlock Text='{CompiledBinding $parent[Panel].DataContext.StringProperty}' Name='textBlock' />
    </Panel>
</Window>"#;
    let window: Ref<Window> = load_as(xaml);
    let text_block = window.get_control::<TextBlock>("textBlock");

    let data_context = TestDataContext::new();
    data_context.set_string_property(Some("foobar".to_string()));

    window.set_data_context(Some(data_context.clone()));

    assert_eq!(data_context.string_property(), text_block.text());
}

#[test]
fn resolves_parent_data_context_type_based_on_context_short_syntax() {
    let _app = styled_window_application();
    let xaml = r#"
<Window xmlns='https://github.com/ferroui'
        xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'
        xmlns:local='clr-namespace:FerroUI.Markup.Xaml.UnitTests.MarkupExtensions;assembly=FerroUI.Markup.Xaml.UnitTests'
        x:DataType='local:TestDataContext'
        x:Name='MyWindow'>
    <Panel>
        <TextBlock Text='{CompiledBinding $parent.DataContext.StringProperty}' Name='textBlock' />
    </Panel>
</Window>"#;
    let window: Ref<Window> = load_as(xaml);
    let text_block = window.get_control::<TextBlock>("textBlock");

    let data_context = TestDataContext::new();
    data_context.set_string_property(Some("foobar".to_string()));

    window.set_data_context(Some(data_context.clone()));

    assert_eq!(data_context.string_property(), text_block.text());
}

#[test]
fn resolves_nested_generic_data_types() {
    let _app = styled_window_application();
    let xaml = r#"
<Window xmlns='https://github.com/ferroui'
        xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'
        xmlns:local='clr-namespace:FerroUI.Markup.Xaml.UnitTests.MarkupExtensions;assembly=FerroUI.Markup.Xaml.UnitTests'
        x:DataType='{x:Type local:TestDataContext+NestedGeneric, x:TypeArguments=x:String}'
        x:Name='MyWindow'>
    <Panel>
        <TextBlock Text='{CompiledBinding Value}' Name='textBlock' />
    </Panel>
</Window>"#;
    let window: Ref<Window> = load_as(xaml);
    let text_block = window.get_control::<TextBlock>("textBlock");

    let data_context = TestDataContext::new();
    let nested = NestedGenericString::new();
    nested.set_value(Some("10".to_string()));
    data_context.set_nested_generic_string(Some(nested));

    window.set_data_context(Some(data_context.nested_generic_string().unwrap()));

    assert_eq!(data_context.nested_generic_string().unwrap().value(), text_block.text());
}

#[test]
fn emits_typed_binding_expression_for_simple_data_context_binding() {
    let _app = styled_window_application();
    let xaml = r#"
<Window xmlns='https://github.com/ferroui'
        xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'
        xmlns:local='clr-namespace:FerroUI.Markup.Xaml.UnitTests.MarkupExtensions;assembly=FerroUI.Markup.Xaml.UnitTests'
        x:DataType='local:TestDataContext'>
    <TextBlock Text='{CompiledBinding StringProperty}' Name='textBlock' />
</Window>"#;
    let window: Ref<Window> = load_as(xaml);
    let text_block = window.get_control::<TextBlock>("textBlock");
    let data_context = TestDataContext::new();
    data_context.set_string_property(Some("hello".to_string()));
    window.set_data_context(Some(data_context));

    let expression = BindingOperations::get_binding_expression_base(&text_block, TextBlock::text_property())
        .expect("the property has no binding expression");
    assert!(expression.as_any().is::<TypedBindingExpression<TestDataContext, Option<String>>>());
    assert_eq!(Some("hello"), text_block.text().as_deref());
}

#[test]
fn emits_typed_binding_expression_for_all_standard_modes() {
    for mode in ["OneWay", "TwoWay", "OneWayToSource", "OneTime"] {
        let _app = styled_window_application();
        let xaml = r#"
<Window xmlns='https://github.com/ferroui'
        xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'
        xmlns:local='clr-namespace:FerroUI.Markup.Xaml.UnitTests.MarkupExtensions;assembly=FerroUI.Markup.Xaml.UnitTests'
        x:DataType='local:TestDataContext'>
    <TextBlock Text='{CompiledBinding StringProperty, Mode={mode}}' Name='textBlock' />
</Window>"#
        .replace("{mode}", mode);
        let window: Ref<Window> = load_as(&xaml);
        let text_block = window.get_control::<TextBlock>("textBlock");
        let data_context = TestDataContext::new();
        data_context.set_string_property(Some("x".to_string()));
        window.set_data_context(Some(data_context));

        let expression = BindingOperations::get_binding_expression_base(&text_block, TextBlock::text_property())
            .unwrap_or_else(|| panic!("Mode={mode}: the property has no binding expression"));
        assert!(expression.as_any().is::<TypedBindingExpression<TestDataContext, Option<String>>>(), "Mode={mode}");
    }
}

#[test]
fn emits_typed_binding_expression_for_binding_with_compile_bindings_true() {
    let _app = styled_window_application();
    let xaml = r#"
<Window xmlns='https://github.com/ferroui'
        xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'
        xmlns:local='clr-namespace:FerroUI.Markup.Xaml.UnitTests.MarkupExtensions;assembly=FerroUI.Markup.Xaml.UnitTests'
        x:DataType='local:TestDataContext'
        x:CompileBindings='True'>
    <TextBlock Text='{Binding StringProperty}' Name='textBlock' />
</Window>"#;
    let window: Ref<Window> = load_as(xaml);
    let text_block = window.get_control::<TextBlock>("textBlock");
    let data_context = TestDataContext::new();
    data_context.set_string_property(Some("hi".to_string()));
    window.set_data_context(Some(data_context));

    let expression = BindingOperations::get_binding_expression_base(&text_block, TextBlock::text_property())
        .expect("the property has no binding expression");
    assert!(expression.as_any().is::<TypedBindingExpression<TestDataContext, Option<String>>>());
}

#[test]
fn falls_back_to_binding_expression_for_nested_path() {
    let _app = styled_window_application();
    let xaml = r#"
<Window xmlns='https://github.com/ferroui'
        xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'
        xmlns:local='clr-namespace:FerroUI.Markup.Xaml.UnitTests.MarkupExtensions;assembly=FerroUI.Markup.Xaml.UnitTests'
        x:DataType='local:TestDataContext'>
    <TextBlock Text='{CompiledBinding NestedGenericString.Value}' Name='textBlock' />
</Window>"#;
    let window: Ref<Window> = load_as(xaml);
    let text_block = window.get_control::<TextBlock>("textBlock");
    let data_context = TestDataContext::new();
    let nested = NestedGenericString::new();
    nested.set_value(Some("v".to_string()));
    data_context.set_nested_generic_string(Some(nested));
    window.set_data_context(Some(data_context));

    let expression = BindingOperations::get_binding_expression_base(&text_block, TextBlock::text_property())
        .expect("the property has no binding expression");
    assert!(expression.as_any().is::<BindingExpression>());
}

#[test]
fn falls_back_to_binding_expression_when_converter_set() {
    let _app = styled_window_application();
    let xaml = r#"
<Window xmlns='https://github.com/ferroui'
        xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'
        xmlns:local='clr-namespace:FerroUI.Markup.Xaml.UnitTests.MarkupExtensions;assembly=FerroUI.Markup.Xaml.UnitTests'
        x:DataType='local:TestDataContext'>
    <TextBlock Text='{CompiledBinding StringProperty, Converter={x:Static local:AppendConverter.Instance}, ConverterParameter=suffix}' Name='textBlock' />
</Window>"#;
    let window: Ref<Window> = load_as(xaml);
    let text_block = window.get_control::<TextBlock>("textBlock");
    let data_context = TestDataContext::new();
    data_context.set_string_property(Some("x".to_string()));
    window.set_data_context(Some(data_context));

    let expression = BindingOperations::get_binding_expression_base(&text_block, TextBlock::text_property())
        .expect("the property has no binding expression");
    assert!(expression.as_any().is::<BindingExpression>());
}

#[test]
fn falls_back_to_binding_expression_for_data_validation_enabled_property() {
    let _app = styled_window_application();
    // TextBox.Text enables data validation, which TypedBindingExpression does not
    // support, so the binding must fall back to the untyped BindingExpression even
    // though it is otherwise eligible for the typed path.
    let xaml = r#"
<Window xmlns='https://github.com/ferroui'
        xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'
        xmlns:local='clr-namespace:FerroUI.Markup.Xaml.UnitTests.MarkupExtensions;assembly=FerroUI.Markup.Xaml.UnitTests'
        x:DataType='local:TestDataContext'>
    <TextBox Text='{CompiledBinding StringProperty}' Name='textBox' />
</Window>"#;
    let window: Ref<Window> = load_as(xaml);
    let text_box = window.get_control::<TextBox>("textBox");
    let data_context = TestDataContext::new();
    data_context.set_string_property(Some("hello".to_string()));
    window.set_data_context(Some(data_context));

    let expression = BindingOperations::get_binding_expression_base(&text_box, TextBox::text_property())
        .expect("the property has no binding expression");
    assert!(expression.as_any().is::<BindingExpression>());
    assert_eq!(Some("hello"), text_box.text().as_deref());
}

#[test]
fn falls_back_to_binding_expression_when_string_format_set() {
    let _app = styled_window_application();
    let xaml = r#"
<Window xmlns='https://github.com/ferroui'
        xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'
        xmlns:local='clr-namespace:FerroUI.Markup.Xaml.UnitTests.MarkupExtensions;assembly=FerroUI.Markup.Xaml.UnitTests'
        x:DataType='local:TestDataContext'>
    <TextBlock Text='{CompiledBinding DecimalValue, StringFormat=c2}' Name='textBlock' />
</Window>"#;
    let window: Ref<Window> = load_as(xaml);
    let text_block = window.get_control::<TextBlock>("textBlock");
    window.set_data_context(Some(TestDataContext::new()));

    let expression = BindingOperations::get_binding_expression_base(&text_block, TextBlock::text_property())
        .expect("the property has no binding expression");
    assert!(expression.as_any().is::<BindingExpression>());
}

#[test]
fn falls_back_to_binding_expression_for_negated_binding() {
    let _app = styled_window_application();
    let xaml = r#"
<Window xmlns='https://github.com/ferroui'
        xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'
        xmlns:local='clr-namespace:FerroUI.Markup.Xaml.UnitTests.MarkupExtensions;assembly=FerroUI.Markup.Xaml.UnitTests'
        x:DataType='local:TestDataContext'>
    <TextBlock Tag='{CompiledBinding !BoolProperty}' Name='textBlock' />
</Window>"#;
    let window: Ref<Window> = load_as(xaml);
    let text_block = window.get_control::<TextBlock>("textBlock");
    let data_context = TestDataContext::new();
    data_context.set_bool_property(true);
    window.set_data_context(Some(data_context));

    let expression = BindingOperations::get_binding_expression_base(&text_block, Control::tag_property())
        .expect("the property has no binding expression");
    assert!(expression.as_any().is::<BindingExpression>());
}

#[test]
fn falls_back_to_binding_expression_for_data_context_target() {
    let _app = styled_window_application();
    let xaml = r#"
<Window xmlns='https://github.com/ferroui'
        xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'
        xmlns:local='clr-namespace:FerroUI.Markup.Xaml.UnitTests.MarkupExtensions;assembly=FerroUI.Markup.Xaml.UnitTests'
        x:DataType='local:TestDataContext'>
    <TextBlock DataContext='{CompiledBinding StringProperty}' Name='textBlock' />
</Window>"#;
    let window: Ref<Window> = load_as(xaml);
    let text_block = window.get_control::<TextBlock>("textBlock");
    let data_context = TestDataContext::new();
    data_context.set_string_property(Some("x".to_string()));
    window.set_data_context(Some(data_context));

    let expression = BindingOperations::get_binding_expression_base(&text_block, StyledElement::data_context_property())
        .expect("the property has no binding expression");
    assert!(expression.as_any().is::<BindingExpression>());
}
