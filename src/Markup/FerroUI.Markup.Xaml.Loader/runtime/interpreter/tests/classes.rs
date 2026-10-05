//! The test classes: the counterparts of the classes of the upstream test
//! project (`tests/XamlParserTests/*.cs`), declared with the metadata
//! macros. Reference types are held in `Rc<T>` and compare by identity.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use ferroui_base::animation::TimeSpan;
use ferroui_base::data::core::ValueTypes;
use ferroui_base::metadata::{
    IServiceProvider, MarkupAssembly, MarkupType, MarkupTyped, XmlnsDefinition,
};
use ferroui_base::utilities::{CultureInfo, Uri};
use ferroui_base::{ferro_markup_enum, ferro_markup_type, BoxedValue};

use crate::runtime::interpreter::services::{register_service_types, IRootObjectProvider};
use crate::runtime::type_system::{DeferredContentFactory, ITypeDescriptorContext, RuntimeTypeValue};

pub(crate) type Object = Option<BoxedValue>;
pub(crate) type Str = Option<String>;

macro_rules! identity_eq {
    ($($type_:ty),* $(,)?) => {
        $(impl PartialEq for $type_ {
            fn eq(&self, other: &Self) -> bool {
                std::ptr::addr_eq(self, other)
            }
        })*
    };
}

/// A getter of a `RefCell` field.
macro_rules! g {
    ($type_:ty, $field:ident) => {
        |c: &Rc<$type_>| c.$field.borrow().clone()
    };
}

/// A setter of a `RefCell` field.
macro_rules! s {
    ($type_:ty, $field:ident : $value:ty) => {
        |c: &Rc<$type_>, v: $value| {
            *c.$field.borrow_mut() = v;
        }
    };
}

/// A reference type of the test namespace.
macro_rules! test_class {
    ($type_:ident { $($body:tt)* }) => {
        identity_eq!($type_);
        ferro_markup_type!(class $type_ {
            handles: [Rc<$type_>, Option<Rc<$type_>>],
            this: Rc<$type_>,
            namespace: "RtXamlParserTests",
            $($body)*
        });
    };
}

fn new<T: Default>() -> Rc<T> {
    Rc::new(T::default())
}

// --- Collections: instantiations of the runtime library collections. ------

/// `System.Collections.Generic.List<T>`.
pub(crate) struct TestList<T>(pub RefCell<Vec<T>>);

impl<T> Default for TestList<T> {
    fn default() -> Self {
        Self(RefCell::new(Vec::new()))
    }
}

impl<T> PartialEq for TestList<T> {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl<T: Clone> TestList<T> {
    pub fn items(&self) -> Vec<T> {
        self.0.borrow().clone()
    }
}

macro_rules! test_list {
    ($item:ty) => {
        ferro_markup_type!(class TestList<$item> as "List`1" {
            handles: [Rc<TestList<$item>>, Option<Rc<TestList<$item>>>],
            this: Rc<TestList<$item>>,
            namespace: "System.Collections.Generic",
            generic: "List`1" [$item],
            constructors: [() => new::<TestList<$item>>],
            methods: [fn Add($item) => |l: &Rc<TestList<$item>>, v: $item| l.0.borrow_mut().push(v)],
        });
    };
}

test_list!(Rc<SimpleSubClass>);
test_list!(Rc<ServiceProviderTestsClass>);
test_list!(i32);
test_list!(Option<BoxedValue>);

/// `System.Collections.Generic.IEnumerable<T>`: a list seen through the
/// interface. The interface handle declares the member markup calls on it.
#[derive(Clone, PartialEq)]
pub(crate) struct TestEnumerable<T>(pub Rc<TestList<T>>);

ferro_markup_type!(interface TestEnumerable<Rc<SimpleSubClass>> as "IEnumerable`1" {
    handles: [TestEnumerable<Rc<SimpleSubClass>>],
    namespace: "System.Collections.Generic",
    generic: "IEnumerable`1" [Rc<SimpleSubClass>],
    methods: [
        fn Add(Rc<SimpleSubClass>) =>
            |e: &TestEnumerable<Rc<SimpleSubClass>>, v: Rc<SimpleSubClass>| e.0.0.borrow_mut().push(v)
    ],
});

/// `System.Collections.Generic.Dictionary<TKey, TValue>`.
pub(crate) struct TestDictionary<K, V>(pub RefCell<Vec<(K, V)>>);

impl<K, V> Default for TestDictionary<K, V> {
    fn default() -> Self {
        Self(RefCell::new(Vec::new()))
    }
}

impl<K, V> PartialEq for TestDictionary<K, V> {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

macro_rules! test_dictionary {
    ($key:ty, $value:ty) => {
        ferro_markup_type!(class TestDictionary<$key, $value> as "Dictionary`2" {
            handles: [Rc<TestDictionary<$key, $value>>, Option<Rc<TestDictionary<$key, $value>>>],
            this: Rc<TestDictionary<$key, $value>>,
            namespace: "System.Collections.Generic",
            generic: "Dictionary`2" [$key, $value],
            constructors: [() => new::<TestDictionary<$key, $value>>],
            methods: [
                fn Add($key, $value) =>
                    |d: &Rc<TestDictionary<$key, $value>>, k: $key, v: $value| d.0.borrow_mut().push((k, v))
            ],
        });
    };
}

test_dictionary!(Option<BoxedValue>, Option<BoxedValue>);
test_dictionary!(String, i32);

// --- TestXamlLanguage.cs ---------------------------------------------------

pub(crate) trait ITestAddChild {
    fn add_child(&self, child: Object);
}

pub(crate) trait ITestAddChildOfString {
    fn add_child_text(&self, child: String);
}

pub(crate) trait ITestSupportInitialize {
    fn begin_init(&self);
    fn end_init(&self);
}

identity_eq!(dyn ITestAddChild, dyn ITestAddChildOfString, dyn ITestSupportInitialize);

ferro_markup_type!(interface dyn ITestAddChild as "IAddChild" {
    handles: [Rc<dyn ITestAddChild>],
    this: Rc<dyn ITestAddChild>,
    namespace: "RtXamlParserTests",
    methods: [fn AddChild(Option<BoxedValue>) => |t: &Rc<dyn ITestAddChild>, child: Object| t.add_child(child)],
});

ferro_markup_type!(interface dyn ITestAddChildOfString as "IAddChild`1" {
    handles: [Rc<dyn ITestAddChildOfString>],
    this: Rc<dyn ITestAddChildOfString>,
    namespace: "RtXamlParserTests",
    interfaces: [Rc<dyn ITestAddChild>],
    generic: "IAddChild`1" [String],
    methods: [fn AddChild(String) => |t: &Rc<dyn ITestAddChildOfString>, child: String| t.add_child_text(child)],
});

ferro_markup_type!(interface dyn ITestSupportInitialize as "ISupportInitialize" {
    handles: [Rc<dyn ITestSupportInitialize>],
    this: Rc<dyn ITestSupportInitialize>,
    namespace: "RtXamlParserTests",
    methods: [
        fn BeginInit() => |t: &Rc<dyn ITestSupportInitialize>| t.begin_init(),
        fn EndInit() => |t: &Rc<dyn ITestSupportInitialize>| t.end_init(),
    ],
});

// --- BasicCompilerTests.cs -------------------------------------------------

#[derive(Default)]
pub(crate) struct SimpleSubClass {
    pub test: RefCell<Str>,
}

test_class!(SimpleSubClass {
    constructors: [() => new::<SimpleSubClass>],
    properties: [Test: Option<String> { get: g!(SimpleSubClass, test), set: s!(SimpleSubClass, test: Str) }],
});

#[derive(Default)]
pub(crate) struct SimpleClass {
    pub test: RefCell<Str>,
    pub test2: RefCell<Str>,
    pub children: Rc<TestList<Rc<SimpleSubClass>>>,
}

test_class!(SimpleClass {
    constructors: [() => new::<SimpleClass>],
    content: Children,
    properties: [
        Test: Option<String> { get: g!(SimpleClass, test), set: s!(SimpleClass, test: Str) },
        Test2: Option<String> { get: g!(SimpleClass, test2), set: s!(SimpleClass, test2: Str) },
        Children: Rc<TestList<Rc<SimpleSubClass>>> { get: |c: &Rc<SimpleClass>| c.children.clone() },
    ],
});

#[derive(Default)]
pub(crate) struct ObjectWithAddChild {
    pub child: RefCell<Object>,
}

impl ITestAddChild for ObjectWithAddChild {
    fn add_child(&self, child: Object) {
        *self.child.borrow_mut() = child;
    }
}

test_class!(ObjectWithAddChild {
    interfaces: [Rc<dyn ITestAddChild>],
    constructors: [() => new::<ObjectWithAddChild>],
});

#[derive(Default)]
pub(crate) struct ObjectWithGenericAddChild {
    pub child: RefCell<Object>,
    pub text: RefCell<Str>,
}

impl ITestAddChild for ObjectWithGenericAddChild {
    fn add_child(&self, child: Object) {
        *self.child.borrow_mut() = child;
    }
}

impl ITestAddChildOfString for ObjectWithGenericAddChild {
    fn add_child_text(&self, child: String) {
        *self.text.borrow_mut() = Some(child);
    }
}

test_class!(ObjectWithGenericAddChild {
    interfaces: [Rc<dyn ITestAddChildOfString>],
    constructors: [() => new::<ObjectWithGenericAddChild>],
});

#[derive(Default)]
pub(crate) struct ObjectWithoutMatchingCtor {
    pub arg: RefCell<Str>,
    pub prop: RefCell<Str>,
}

test_class!(ObjectWithoutMatchingCtor {
    constructors: [
        (Option<String>) => |arg: Str| Rc::new(ObjectWithoutMatchingCtor { arg: RefCell::new(arg), prop: RefCell::new(None) })
    ],
    properties: [
        Arg: Option<String> { get: g!(ObjectWithoutMatchingCtor, arg), set: s!(ObjectWithoutMatchingCtor, arg: Str) },
        Prop: Option<String> { get: g!(ObjectWithoutMatchingCtor, prop), set: s!(ObjectWithoutMatchingCtor, prop: Str) },
    ],
});

// --- ContentAttributeTests.cs, ListTests.cs, DictionaryTests.cs ------------

#[derive(Default)]
pub(crate) struct RtClassWithContentAttribute {
    pub text: RefCell<Str>,
}

test_class!(RtClassWithContentAttribute {
    constructors: [() => new::<RtClassWithContentAttribute>],
    content: Text,
    properties: [
        Text: Option<String> { get: g!(RtClassWithContentAttribute, text), set: s!(RtClassWithContentAttribute, text: Str) },
    ],
});

#[derive(Default)]
pub(crate) struct RtSubClassWithContentAttributeOverride {
    pub base: Rc<RtClassWithContentAttribute>,
    pub other_text: RefCell<Str>,
}

test_class!(RtSubClassWithContentAttributeOverride {
    base: Rc<RtClassWithContentAttribute>,
    constructors: [() => new::<RtSubClassWithContentAttributeOverride>],
    content: OtherText,
    properties: [
        OtherText: Option<String> {
            get: g!(RtSubClassWithContentAttributeOverride, other_text),
            set: s!(RtSubClassWithContentAttributeOverride, other_text: Str)
        },
    ],
});

pub(crate) struct EnumerableContentClass {
    pub children: RefCell<TestEnumerable<Rc<SimpleSubClass>>>,
}

test_class!(EnumerableContentClass {
    constructors: [() => || Rc::new(EnumerableContentClass { children: RefCell::new(TestEnumerable(new())) })],
    content: Children,
    properties: [
        Children: TestEnumerable<Rc<SimpleSubClass>> {
            get: g!(EnumerableContentClass, children),
            set: s!(EnumerableContentClass, children: TestEnumerable<Rc<SimpleSubClass>>)
        },
    ],
});

#[derive(Default)]
pub(crate) struct SimpleClassWithDictionaryContent {
    pub test: RefCell<Str>,
    pub children: Rc<TestDictionary<Object, Object>>,
    pub non_content_children: Rc<TestDictionary<Object, Object>>,
}

test_class!(SimpleClassWithDictionaryContent {
    constructors: [() => new::<SimpleClassWithDictionaryContent>],
    content: Children,
    properties: [
        Test: Option<String> {
            get: g!(SimpleClassWithDictionaryContent, test),
            set: s!(SimpleClassWithDictionaryContent, test: Str)
        },
        Children: Rc<TestDictionary<Option<BoxedValue>, Option<BoxedValue>>> {
            get: |c: &Rc<SimpleClassWithDictionaryContent>| c.children.clone()
        },
        NonContentChildren: Rc<TestDictionary<Option<BoxedValue>, Option<BoxedValue>>> {
            get: |c: &Rc<SimpleClassWithDictionaryContent>| c.non_content_children.clone()
        },
    ],
});

// --- IntrinsicsTests.cs ----------------------------------------------------

#[derive(Default)]
pub(crate) struct IntrinsicsTestsClass {
    pub object_property: RefCell<Object>,
    pub int_property: RefCell<i32>,
    pub type_property: RefCell<Option<RuntimeTypeValue>>,
    pub bool_property: RefCell<bool>,
    pub nullable_bool_property: RefCell<Option<bool>>,
}

test_class!(IntrinsicsTestsClass {
    constructors: [() => new::<IntrinsicsTestsClass>],
    properties: [
        ObjectProperty: Option<BoxedValue> {
            get: g!(IntrinsicsTestsClass, object_property),
            set: s!(IntrinsicsTestsClass, object_property: Object)
        },
        IntProperty: i32 { get: g!(IntrinsicsTestsClass, int_property), set: s!(IntrinsicsTestsClass, int_property: i32) },
        TypeProperty: Option<RuntimeTypeValue> {
            get: g!(IntrinsicsTestsClass, type_property),
            set: s!(IntrinsicsTestsClass, type_property: Option<RuntimeTypeValue>)
        },
        BoolProperty: bool {
            get: g!(IntrinsicsTestsClass, bool_property),
            set: s!(IntrinsicsTestsClass, bool_property: bool)
        },
        NullableBoolProperty: Option<bool> {
            get: g!(IntrinsicsTestsClass, nullable_bool_property),
            set: s!(IntrinsicsTestsClass, nullable_bool_property: Option<bool>)
        },
    ],
    fields: [
        StaticProp: Option<BoxedValue> => || -> Object { Some(Rc::new("StaticPropValue".to_string())) },
        StaticField: Option<BoxedValue> => || -> Object { Some(Rc::new("StaticFieldValue".to_string())) },
        StringConstant: String => || "ConstantValue".to_string(),
        IntConstant: i32 => || 100,
        FloatConstant: f32 => || 2.0f32,
        DoubleConstant: f64 => || 3.0f64,
    ],
});

#[derive(Default)]
pub(crate) struct IntrinsicsTestsDerivedClass {
    pub base: Rc<IntrinsicsTestsClass>,
}

test_class!(IntrinsicsTestsDerivedClass {
    base: Rc<IntrinsicsTestsClass>,
    constructors: [() => new::<IntrinsicsTestsDerivedClass>],
});

#[derive(Default)]
pub(crate) struct IntrinsicsListTestsClass {
    pub add_int32_call_count: Cell<i32>,
    pub add_object_call_count: Cell<i32>,
}

test_class!(IntrinsicsListTestsClass {
    constructors: [() => new::<IntrinsicsListTestsClass>],
    methods: [
        fn Add(i32) => |c: &Rc<IntrinsicsListTestsClass>, _v: i32| c.add_int32_call_count.set(c.add_int32_call_count.get() + 1),
        fn Add(Option<BoxedValue>) =>
            |c: &Rc<IntrinsicsListTestsClass>, _v: Object| c.add_object_call_count.set(c.add_object_call_count.get() + 1),
    ],
});

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum IntrinsicsTestsEnum {
    Foo = 100500,
}

ferro_markup_enum!(IntrinsicsTestsEnum { Foo }, { namespace: "RtXamlParserTests" });

// --- InitializationTests.cs ------------------------------------------------

thread_local! {
    pub(crate) static INIT_EVENTS: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
    static INIT_NEXT_ID: Cell<i32> = const { Cell::new(0) };
}

pub(crate) fn reset_initialization() {
    INIT_EVENTS.with(|e| e.borrow_mut().clear());
    INIT_NEXT_ID.set(0);
}

pub(crate) struct InitializationTestsClass {
    pub id: i32,
    prop: RefCell<Str>,
    child: RefCell<Option<Rc<InitializationTestsClass>>>,
}

impl Default for InitializationTestsClass {
    fn default() -> Self {
        INIT_NEXT_ID.set(INIT_NEXT_ID.get() + 1);
        Self { id: INIT_NEXT_ID.get(), prop: RefCell::new(None), child: RefCell::new(None) }
    }
}

impl InitializationTestsClass {
    fn add_event(&self, event: &str) {
        INIT_EVENTS.with(|e| e.borrow_mut().push(format!("{}:{event}", self.id)));
    }
}

/// The `Children` collection of [`InitializationTestsClass`]: reports the
/// children added to it.
#[derive(Clone)]
pub(crate) struct InitializationTestsChildren(Rc<InitializationTestsClass>);

impl PartialEq for InitializationTestsChildren {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}

ferro_markup_type!(class InitializationTestsChildren {
    handles: [InitializationTestsChildren],
    namespace: "RtXamlParserTests",
    methods: [
        fn Add(Rc<InitializationTestsClass>) =>
            |c: &InitializationTestsChildren, child: Rc<InitializationTestsClass>| {
                c.0.add_event(&format!("ChildAdded:{}", child.id))
            }
    ],
});

test_class!(InitializationTestsClass {
    constructors: [() => new::<InitializationTestsClass>],
    content: Children,
    properties: [
        Children: InitializationTestsChildren {
            get: |c: &Rc<InitializationTestsClass>| InitializationTestsChildren(c.clone())
        },
        Property: Option<String> {
            get: g!(InitializationTestsClass, prop),
            set: |c: &Rc<InitializationTestsClass>, v: Str| {
                *c.prop.borrow_mut() = v;
                c.add_event("PropertySet");
            }
        },
        Child: Option<Rc<InitializationTestsClass>> {
            get: g!(InitializationTestsClass, child),
            set: |c: &Rc<InitializationTestsClass>, v: Option<Rc<InitializationTestsClass>>| {
                c.add_event(&format!("ChildAdded:{}", v.as_ref().map(|v| v.id.to_string()).unwrap_or_default()));
                *c.child.borrow_mut() = v;
            }
        },
    ],
});

#[derive(Default)]
pub(crate) struct InitializationTestsSupportInitializeClass {
    pub base: Rc<InitializationTestsClass>,
}

impl ITestSupportInitialize for InitializationTestsSupportInitializeClass {
    fn begin_init(&self) {
        self.base.add_event("BeginInit");
    }
    fn end_init(&self) {
        self.base.add_event("EndInit");
    }
}

test_class!(InitializationTestsSupportInitializeClass {
    base: Rc<InitializationTestsClass>,
    interfaces: [Rc<dyn ITestSupportInitialize>],
    constructors: [() => new::<InitializationTestsSupportInitializeClass>],
});

#[derive(Default)]
pub(crate) struct InitializationTestsTopDownClass {
    pub base: Rc<InitializationTestsSupportInitializeClass>,
}

test_class!(InitializationTestsTopDownClass {
    base: Rc<InitializationTestsSupportInitializeClass>,
    constructors: [() => new::<InitializationTestsTopDownClass>],
    attributes: [UsableDuringInitialization(true)],
});

// --- DeferredContentTests.cs, ServiceProviderTests.cs ----------------------

/// The counterpart of the upstream `CallbackExtensionCallback` delegate; a
/// service of the test service provider.
#[derive(Clone)]
pub(crate) struct Callback(pub Rc<dyn Fn(Rc<dyn IServiceProvider>) -> Object>);

#[derive(Default)]
pub(crate) struct CallbackExtension {
    pub nested: RefCell<Object>,
}

test_class!(CallbackExtension {
    constructors: [() => new::<CallbackExtension>],
    properties: [
        Nested: Option<BoxedValue> { get: g!(CallbackExtension, nested), set: s!(CallbackExtension, nested: Object) },
    ],
    methods: [
        fn ProvideValue(Rc<dyn IServiceProvider>) -> Option<BoxedValue> =>
            |_: &Rc<CallbackExtension>, provider: Rc<dyn IServiceProvider>| -> Object {
                let callback = provider.get_service_of::<Callback>().expect("the callback service");
                (callback.0)(provider)
            }
    ],
});

#[derive(Default)]
pub(crate) struct DeferredContentTestsClass {
    pub deferred_content: RefCell<Object>,
    pub object_property: RefCell<Object>,
}

test_class!(DeferredContentTestsClass {
    constructors: [() => new::<DeferredContentTestsClass>],
    content: DeferredContent,
    properties: [
        DeferredContent: Option<BoxedValue> {
            get: g!(DeferredContentTestsClass, deferred_content),
            set: s!(DeferredContentTestsClass, deferred_content: Object)
        } [DeferredContent],
        ObjectProperty: Option<BoxedValue> {
            get: g!(DeferredContentTestsClass, object_property),
            set: s!(DeferredContentTestsClass, object_property: Object)
        },
    ],
});

pub(crate) struct DeferredValue {
    pub original_factory: DeferredContentFactory,
}

test_class!(DeferredValue {});

pub(crate) struct ConstantRootObjectProvider(pub Object);

impl IRootObjectProvider for ConstantRootObjectProvider {
    fn root_object(&self) -> Object {
        self.0.clone()
    }
    fn intermediate_root_object(&self) -> Object {
        None
    }
}

fn delegate_customizer(builder: DeferredContentFactory, parent_services: Rc<dyn IServiceProvider>) -> DeferredContentFactory {
    let parent_root =
        parent_services.get_service_of::<Rc<dyn IRootObjectProvider>>().expect("root object provider").root_object();
    let callback = parent_services.get_service_of::<Callback>();
    DeferredContentFactory::new(move |provider| {
        let services = super::TestServiceProvider::new()
            .with::<Rc<dyn IRootObjectProvider>>(Rc::new(ConstantRootObjectProvider(parent_root.clone())))
            .with_parent(provider);
        let services = match &callback {
            Some(callback) => services.with(callback.clone()),
            None => services,
        };
        builder.invoke(Some(services))
    })
}

pub(crate) struct DeferredContentTests;

ferro_markup_type!(static DeferredContentTests {
    namespace: "RtXamlParserTests",
    methods: [
        static fn DelegateCustomizer(DeferredContentFactory, Rc<dyn IServiceProvider>) -> DeferredContentFactory =>
            delegate_customizer,
        static fn CustomizerWithChangedReturnType(DeferredContentFactory, Rc<dyn IServiceProvider>) -> Rc<DeferredValue> =>
            |builder: DeferredContentFactory, _: Rc<dyn IServiceProvider>| Rc::new(DeferredValue { original_factory: builder }),
    ],
});

#[derive(Default)]
pub(crate) struct ServiceProviderTestsClass {
    pub id: RefCell<Str>,
    pub property: RefCell<Object>,
    pub child: RefCell<Option<Rc<ServiceProviderTestsClass>>>,
    pub children: Rc<TestList<Rc<ServiceProviderTestsClass>>>,
}

test_class!(ServiceProviderTestsClass {
    constructors: [() => new::<ServiceProviderTestsClass>],
    content: Children,
    properties: [
        Id: Option<String> { get: g!(ServiceProviderTestsClass, id), set: s!(ServiceProviderTestsClass, id: Str) },
        Property: Option<BoxedValue> {
            get: g!(ServiceProviderTestsClass, property),
            set: s!(ServiceProviderTestsClass, property: Object)
        },
        Child: Option<Rc<ServiceProviderTestsClass>> {
            get: g!(ServiceProviderTestsClass, child),
            set: s!(ServiceProviderTestsClass, child: Option<Rc<ServiceProviderTestsClass>>)
        },
        Children: Rc<TestList<Rc<ServiceProviderTestsClass>>> {
            get: |c: &Rc<ServiceProviderTestsClass>| c.children.clone()
        },
    ],
});

/// The counterpart of the upstream `ServiceProviderTests.InnerProvider`.
pub(crate) struct InnerProvider {
    original: Rc<dyn IRootObjectProvider>,
}

impl InnerProvider {
    pub fn original_root_object(&self) -> Object {
        self.original.root_object()
    }
}

impl IRootObjectProvider for InnerProvider {
    fn root_object(&self) -> Object {
        Some(Rc::new("Definitely not the root object".to_string()))
    }
    fn intermediate_root_object(&self) -> Object {
        None
    }
}

/// The inner provider as a service provider: it only knows the root object
/// provider, which is itself.
struct InnerServiceProvider(Rc<InnerProvider>);

impl IServiceProvider for InnerServiceProvider {
    fn get_service(&self, service_type: std::any::TypeId) -> Option<Rc<dyn std::any::Any>> {
        ferroui_base::metadata::service(service_type, || self.0.clone() as Rc<dyn IRootObjectProvider>)
            .or_else(|| ferroui_base::metadata::service(service_type, || self.0.clone()))
    }
}

pub(crate) struct ServiceProviderTests;

ferro_markup_type!(static ServiceProviderTests {
    namespace: "RtXamlParserTests",
    methods: [
        static fn SetAttachedProperty(Rc<ServiceProviderTestsClass>, Option<String>) =>
            |_: Rc<ServiceProviderTestsClass>, _: Str| {},
        static fn InnerProviderFactory(Rc<dyn IServiceProvider>) -> Rc<dyn IServiceProvider> =>
            |outer: Rc<dyn IServiceProvider>| -> Rc<dyn IServiceProvider> {
                let original = outer.get_service_of::<Rc<dyn IRootObjectProvider>>().expect("root object provider");
                Rc::new(InnerServiceProvider(Rc::new(InnerProvider { original })))
            },
    ],
});

#[derive(Default)]
pub(crate) struct UnknownServiceUsageExtension {
    pub return_: RefCell<Object>,
}

test_class!(UnknownServiceUsageExtension {
    constructors: [() => new::<UnknownServiceUsageExtension>],
    properties: [
        Return: Option<BoxedValue> {
            get: g!(UnknownServiceUsageExtension, return_),
            set: s!(UnknownServiceUsageExtension, return_: Object)
        },
    ],
    methods: [
        fn ProvideValue(Rc<dyn IServiceProvider>) -> Option<BoxedValue> =>
            |e: &Rc<UnknownServiceUsageExtension>, provider: Rc<dyn IServiceProvider>| -> Object {
                assert!(provider.get_service(std::any::TypeId::of::<String>()).is_none());
                e.return_.borrow().clone()
            }
    ],
});

// --- MarkupExtensionTests.cs -----------------------------------------------

#[derive(Default)]
pub(crate) struct MarkupExtensionTestsClass {
    pub int_property: RefCell<i32>,
    pub double_property: RefCell<f64>,
    pub nullable_int_property: RefCell<Option<i32>>,
    pub string_property: RefCell<Str>,
    pub object_property: RefCell<Object>,
    pub int_list: Rc<TestList<i32>>,
    pub int_list2: RefCell<Rc<TestList<i32>>>,
    pub read_only_int_list: Rc<TestList<i32>>,
}

test_class!(MarkupExtensionTestsClass {
    constructors: [() => new::<MarkupExtensionTestsClass>],
    content: IntList,
    properties: [
        IntProperty: i32 {
            get: g!(MarkupExtensionTestsClass, int_property),
            set: s!(MarkupExtensionTestsClass, int_property: i32)
        },
        DoubleProperty: f64 {
            get: g!(MarkupExtensionTestsClass, double_property),
            set: s!(MarkupExtensionTestsClass, double_property: f64)
        },
        NullableIntProperty: Option<i32> {
            get: g!(MarkupExtensionTestsClass, nullable_int_property),
            set: s!(MarkupExtensionTestsClass, nullable_int_property: Option<i32>)
        },
        StringProperty: Option<String> {
            get: g!(MarkupExtensionTestsClass, string_property),
            set: s!(MarkupExtensionTestsClass, string_property: Str)
        },
        ObjectProperty: Option<BoxedValue> {
            get: g!(MarkupExtensionTestsClass, object_property),
            set: s!(MarkupExtensionTestsClass, object_property: Object)
        },
        IntList: Rc<TestList<i32>> { get: |c: &Rc<MarkupExtensionTestsClass>| c.int_list.clone() },
        IntList2: Rc<TestList<i32>> {
            get: g!(MarkupExtensionTestsClass, int_list2),
            set: s!(MarkupExtensionTestsClass, int_list2: Rc<TestList<i32>>)
        },
        ReadOnlyIntList: Rc<TestList<i32>> { get: |c: &Rc<MarkupExtensionTestsClass>| c.read_only_int_list.clone() },
    ],
});

#[derive(Default)]
pub(crate) struct MarkupExtensionContentDictionaryClass {
    pub int_dic: Rc<TestDictionary<String, i32>>,
}

test_class!(MarkupExtensionContentDictionaryClass {
    constructors: [() => new::<MarkupExtensionContentDictionaryClass>],
    content: IntDic,
    properties: [
        IntDic: Rc<TestDictionary<String, i32>> { get: |c: &Rc<MarkupExtensionContentDictionaryClass>| c.int_dic.clone() },
    ],
});

#[derive(Default)]
pub(crate) struct ObjectTestExtension {
    pub returned: RefCell<Object>,
}

test_class!(ObjectTestExtension {
    constructors: [() => new::<ObjectTestExtension>],
    properties: [
        Returned: Option<BoxedValue> { get: g!(ObjectTestExtension, returned), set: s!(ObjectTestExtension, returned: Object) },
    ],
    methods: [fn ProvideValue() -> Option<BoxedValue> => |e: &Rc<ObjectTestExtension>| -> Object { e.returned.borrow().clone() }],
});

/// The non-generic `GenericTestExtension`: must not be confused with the
/// generic types of the same name.
#[derive(Default)]
pub(crate) struct GenericTestExtension {
    pub returned: RefCell<Object>,
}

test_class!(GenericTestExtension {
    constructors: [() => new::<GenericTestExtension>],
    properties: [
        Returned: Option<BoxedValue> { get: g!(GenericTestExtension, returned), set: s!(GenericTestExtension, returned: Object) },
    ],
    methods: [fn ProvideValue() -> Option<BoxedValue> => |e: &Rc<GenericTestExtension>| -> Object { e.returned.borrow().clone() }],
});

/// `GenericTestExtension<TType>`.
pub(crate) struct GenericTestExtension1<T> {
    pub returned: RefCell<T>,
}

impl<T> PartialEq for GenericTestExtension1<T> {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

macro_rules! generic_test_extension {
    ($value:ty) => {
        ferro_markup_type!(class GenericTestExtension1<$value> as "GenericTestExtension`1" {
            handles: [Rc<GenericTestExtension1<$value>>, Option<Rc<GenericTestExtension1<$value>>>],
            this: Rc<GenericTestExtension1<$value>>,
            namespace: "RtXamlParserTests",
            generic: "GenericTestExtension`1" [$value],
            constructors: [() => || Rc::new(GenericTestExtension1::<$value> { returned: RefCell::new(Default::default()) })],
            properties: [
                Returned: $value {
                    get: |e: &Rc<GenericTestExtension1<$value>>| e.returned.borrow().clone(),
                    set: |e: &Rc<GenericTestExtension1<$value>>, v: $value| { *e.returned.borrow_mut() = v; }
                },
            ],
            methods: [
                fn ProvideValue() -> Option<BoxedValue> =>
                    |e: &Rc<GenericTestExtension1<$value>>| -> Object {
                        ferroui_base::metadata::into_markup_value::<$value>(e.returned.borrow().clone())
                    }
            ],
        });
    };
}

generic_test_extension!(i32);
generic_test_extension!(Option<String>);
generic_test_extension!(Option<BoxedValue>);

/// `GenericTestExtension<int, float>`.
#[derive(Default)]
pub(crate) struct GenericTestExtension2 {
    pub returned1: RefCell<i32>,
    pub returned2: RefCell<f32>,
}

identity_eq!(GenericTestExtension2);

ferro_markup_type!(class GenericTestExtension2 as "GenericTestExtension`2" {
    handles: [Rc<GenericTestExtension2>, Option<Rc<GenericTestExtension2>>],
    this: Rc<GenericTestExtension2>,
    namespace: "RtXamlParserTests",
    generic: "GenericTestExtension`2" [i32, f32],
    constructors: [() => new::<GenericTestExtension2>],
    properties: [
        Returned1: i32 { get: g!(GenericTestExtension2, returned1), set: s!(GenericTestExtension2, returned1: i32) },
        Returned2: f32 { get: g!(GenericTestExtension2, returned2), set: s!(GenericTestExtension2, returned2: f32) },
    ],
    methods: [
        fn ProvideValue() -> Option<BoxedValue> =>
            |e: &Rc<GenericTestExtension2>| -> Object { Some(Rc::new((*e.returned1.borrow(), *e.returned2.borrow()))) }
    ],
});

/// A service of the test service provider: the value the extensions below
/// provide.
#[derive(Clone)]
pub(crate) struct ExtensionValueHolder(pub Object);

fn held_value(provider: &Rc<dyn IServiceProvider>) -> Object {
    provider.get_service_of::<ExtensionValueHolder>().expect("the value holder service").0
}

macro_rules! provider_extension {
    ($type_:ident -> $result:ty => $provide:expr) => {
        #[derive(Default)]
        pub(crate) struct $type_;

        test_class!($type_ {
            constructors: [() => new::<$type_>],
            methods: [
                fn ProvideValue(Rc<dyn IServiceProvider>) -> $result =>
                    |_: &Rc<$type_>, provider: Rc<dyn IServiceProvider>| -> $result { ($provide)(held_value(&provider)) }
            ],
        });
    };
}

provider_extension!(ServiceProviderValueExtension -> Option<BoxedValue> => |value: Object| value);
provider_extension!(ServiceProviderIntValueExtension -> i32 => |value: Object| super::get::<i32>(&value));
provider_extension!(ServiceProviderIntListExtension -> Rc<TestList<i32>> => |value: Object| {
    let list = new::<TestList<i32>>();
    list.0.borrow_mut().push(super::get::<i32>(&value));
    list
});

// --- ConvertersTests.cs ----------------------------------------------------

#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct ConvertersTestValueType {
    pub value: String,
}

ferro_markup_type!(struct ConvertersTestValueType {
    handles: [ConvertersTestValueType],
    namespace: "RtXamlParserTests",
    methods: [
        static fn Parse(String, CultureInfo) -> ConvertersTestValueType =>
            |value: String, provider: CultureInfo| {
                assert!(provider == CultureInfo::invariant_culture());
                ConvertersTestValueType { value }
            }
    ],
});

/// A flags enumeration (`First = 1, Second = 2, Third = 4`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct ConvertersTestsEnum(i32);

impl ConvertersTestsEnum {
    pub const FIRST: Self = Self(1);
    pub const SECOND: Self = Self(2);
    pub const THIRD: Self = Self(4);

    pub const fn empty() -> Self {
        Self(0)
    }

    pub const fn bits(self) -> i32 {
        self.0
    }
}

impl std::ops::BitOr for ConvertersTestsEnum {
    type Output = Self;
    fn bitor(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }
}

ferro_markup_enum!(
    flags ConvertersTestsEnum {
        First = ConvertersTestsEnum::FIRST,
        Second = ConvertersTestsEnum::SECOND,
        Third = ConvertersTestsEnum::THIRD,
    },
    { namespace: "RtXamlParserTests" }
);

/// The counterpart of the runtime library's `UriKind`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum RtUriKind {
    #[default]
    RelativeOrAbsolute,
    Absolute,
    Relative,
}

ferro_markup_enum!(RtUriKind { RelativeOrAbsolute, Absolute, Relative }, { namespace: "RtXamlParserTests" });

#[derive(Default)]
pub(crate) struct ConvertersTestsClassWithConverter {
    pub value: RefCell<Str>,
}

#[derive(Default)]
pub(crate) struct ConvertersTestsClassWithoutConverter {
    pub value: RefCell<Str>,
}

fn check_converter_context(context: &Option<Rc<dyn ITypeDescriptorContext>>, culture: &CultureInfo) {
    assert!(*culture == CultureInfo::invariant_culture());
    let context: Rc<dyn IServiceProvider> = context.clone().expect("a type descriptor context");
    let root = context.get_service_of::<Rc<dyn IRootObjectProvider>>().expect("root object provider");
    assert!(root.root_object().is_some());
}

macro_rules! converter {
    ($type_:ident => $convert:expr) => {
        #[derive(Default)]
        pub(crate) struct $type_;

        test_class!($type_ {
            constructors: [() => new::<$type_>],
            methods: [
                fn ConvertFrom(Option<Rc<dyn ITypeDescriptorContext>>, CultureInfo, Option<BoxedValue>) -> Option<BoxedValue> =>
                    |_: &Rc<$type_>, context: Option<Rc<dyn ITypeDescriptorContext>>, culture: CultureInfo, value: Object| -> Object {
                        ($convert)(context, culture, super::get::<String>(&value))
                    }
            ],
        });
    };
}

converter!(TestConverter => |context, culture, value: String| -> Object {
    check_converter_context(&context, &culture);
    Some(Rc::new(Rc::new(ConvertersTestsClassWithConverter { value: RefCell::new(Some(value)) })))
});
converter!(PropertyTestConverter => |context, culture, value: String| -> Object {
    check_converter_context(&context, &culture);
    Some(Rc::new(Rc::new(ConvertersTestsClassWithoutConverter { value: RefCell::new(Some(value)) })))
});
converter!(NegativeIntConverter => |_context, _culture, value: String| -> Object {
    Some(Rc::new(-value.parse::<i32>().expect("an integer")))
});

test_class!(ConvertersTestsClassWithConverter {
    attributes: [TypeConverter(type(Rc<TestConverter>))],
});

test_class!(ConvertersTestsClassWithoutConverter {});

#[derive(Default)]
pub(crate) struct ConvertersTestClass {
    pub content_property: RefCell<Object>,
    pub int64_property: RefCell<i64>,
    pub bool_property: RefCell<bool>,
    pub double_property: RefCell<f64>,
    pub float_property: RefCell<f32>,
    pub time_span_property: RefCell<TimeSpan>,
    pub type_property: RefCell<Option<RuntimeTypeValue>>,
    pub uri_kind_property: RefCell<RtUriKind>,
    pub custom_property: RefCell<ConvertersTestValueType>,
    pub type_with_converter_property: RefCell<Option<Rc<ConvertersTestsClassWithConverter>>>,
    pub property_with_converter: RefCell<Option<Rc<ConvertersTestsClassWithoutConverter>>>,
    pub int_property_with_negative_converter: RefCell<i32>,
    pub enum_property: RefCell<ConvertersTestsEnum>,
}

test_class!(ConvertersTestClass {
    constructors: [() => new::<ConvertersTestClass>],
    content: ContentProperty,
    properties: [
        ContentProperty: Option<BoxedValue> {
            get: g!(ConvertersTestClass, content_property), set: s!(ConvertersTestClass, content_property: Object)
        },
        Int64Property: i64 { get: g!(ConvertersTestClass, int64_property), set: s!(ConvertersTestClass, int64_property: i64) },
        BoolProperty: bool { get: g!(ConvertersTestClass, bool_property), set: s!(ConvertersTestClass, bool_property: bool) },
        DoubleProperty: f64 {
            get: g!(ConvertersTestClass, double_property), set: s!(ConvertersTestClass, double_property: f64)
        },
        FloatProperty: f32 { get: g!(ConvertersTestClass, float_property), set: s!(ConvertersTestClass, float_property: f32) },
        TimeSpanProperty: TimeSpan {
            get: g!(ConvertersTestClass, time_span_property), set: s!(ConvertersTestClass, time_span_property: TimeSpan)
        },
        TypeProperty: Option<RuntimeTypeValue> {
            get: g!(ConvertersTestClass, type_property),
            set: s!(ConvertersTestClass, type_property: Option<RuntimeTypeValue>)
        },
        UriKindProperty: RtUriKind {
            get: g!(ConvertersTestClass, uri_kind_property), set: s!(ConvertersTestClass, uri_kind_property: RtUriKind)
        },
        CustomProperty: ConvertersTestValueType {
            get: g!(ConvertersTestClass, custom_property),
            set: s!(ConvertersTestClass, custom_property: ConvertersTestValueType)
        },
        TypeWithConverterProperty: Option<Rc<ConvertersTestsClassWithConverter>> {
            get: g!(ConvertersTestClass, type_with_converter_property),
            set: s!(ConvertersTestClass, type_with_converter_property: Option<Rc<ConvertersTestsClassWithConverter>>)
        },
        PropertyWithConverter: Option<Rc<ConvertersTestsClassWithoutConverter>> {
            get: g!(ConvertersTestClass, property_with_converter),
            set: s!(ConvertersTestClass, property_with_converter: Option<Rc<ConvertersTestsClassWithoutConverter>>)
        } [TypeConverter(type(Rc<PropertyTestConverter>))],
        IntPropertyWithNegativeConverter: i32 {
            get: g!(ConvertersTestClass, int_property_with_negative_converter),
            set: s!(ConvertersTestClass, int_property_with_negative_converter: i32)
        } [TypeConverter(type(Rc<NegativeIntConverter>))],
        EnumProperty: ConvertersTestsEnum {
            get: g!(ConvertersTestClass, enum_property), set: s!(ConvertersTestClass, enum_property: ConvertersTestsEnum)
        },
    ],
});

pub(crate) struct ConvertersTestsClassWithConstructor {
    pub int: i32,
    pub converted: TimeSpan,
}

test_class!(ConvertersTestsClassWithConstructor {
    constructors: [(i32, TimeSpan) => |int: i32, converted: TimeSpan| Rc::new(ConvertersTestsClassWithConstructor { int, converted })],
});

#[derive(Default)]
pub(crate) struct RtClassWithTwoContentAttributes {
    pub text: RefCell<Str>,
    pub other_text: RefCell<Str>,
}

test_class!(RtClassWithTwoContentAttributes {
    constructors: [() => new::<RtClassWithTwoContentAttributes>],
    content: Text,
    properties: [
        Text: Option<String> {
            get: g!(RtClassWithTwoContentAttributes, text), set: s!(RtClassWithTwoContentAttributes, text: Str)
        },
        OtherText: Option<String> {
            get: g!(RtClassWithTwoContentAttributes, other_text),
            set: s!(RtClassWithTwoContentAttributes, other_text: Str)
        } [Content],
    ],
});

/// A class with plain events (no upstream counterpart).
#[derive(Default)]
pub(crate) struct RtEventSource {
    pub changed: RefCell<Vec<ferroui_base::metadata::MarkupDelegate>>,
    pub pinged: RefCell<Vec<ferroui_base::metadata::MarkupDelegate>>,
}

test_class!(RtEventSource {
    constructors: [() => new::<RtEventSource>],
    events: [
        Changed(Option<BoxedValue>, String) =>
            |s: &Rc<RtEventSource>, h: ferroui_base::metadata::MarkupDelegate| s.changed.borrow_mut().push(h),
        Pinged() => |s: &Rc<RtEventSource>, h: ferroui_base::metadata::MarkupDelegate| s.pinged.borrow_mut().push(h),
        Moved(i32, i32) => |_: &Rc<RtEventSource>, _: ferroui_base::metadata::MarkupDelegate| {},
    ],
});

/// A value with the shape of a grid length: a value and a unit.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct RtLength {
    pub value: f64,
    pub unit: RtLengthUnit,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum RtLengthUnit {
    Auto,
    Pixel,
    Star,
}

ferro_markup_enum!(RtLengthUnit { Auto, Pixel, Star }, { namespace: "RtXamlParserTests" });

impl RtLength {
    fn parse(text: &str) -> Result<Self, String> {
        match text.trim() {
            "Auto" => Ok(Self { value: 1.0, unit: RtLengthUnit::Auto }),
            star if star.ends_with('*') => {
                let value = star.trim_end_matches('*');
                let value = if value.is_empty() { Ok(1.0) } else { value.parse::<f64>() };
                value.map(|value| Self { value, unit: RtLengthUnit::Star }).map_err(|e| e.to_string())
            }
            pixels => {
                pixels.parse::<f64>().map(|value| Self { value, unit: RtLengthUnit::Pixel }).map_err(|e| e.to_string())
            }
        }
    }
}

ferro_markup_type!(struct RtLength {
    handles: [RtLength],
    namespace: "RtXamlParserTests",
    parse: RtLength::parse,
    properties: [
        Value: f64 { get: |l: &RtLength| l.value },
        GridUnitType: RtLengthUnit { get: |l: &RtLength| l.unit },
    ],
});

/// A static type that owns an attached property and declares metadata for
/// it: one type for the type system.
pub(crate) struct RtAttachedOwner;

ferroui_base::ferro_static_type!(RtAttachedOwner);

ferroui_base::ferro_properties! {
    impl RtAttachedOwner {
        pub fn target_property() -> ferroui_base::AttachedProperty<i32> {
            ferroui_base::FerroProperty::register_attached::<RtAttachedOwner, ferroui_base::StyledElement, _>("Target", 0)
        }
    }
}

ferro_markup_type!(static RtAttachedOwner {
    type_info: RtAttachedOwner,
    namespace: "RtXamlParserTests",
    methods: [static fn Twice(i32) -> i32 => |x: i32| x * 2],
    property_attributes: [Target: [ResolveByName, DependsOn("Other")]],
});

// --- SpecialPropertiesTests.cs, GenericTypeWithPropertyElement.cs ----------

#[derive(Default)]
pub(crate) struct InitPropertiesTestClass {
    pub prop1: RefCell<Str>,
    pub prop2: RefCell<i32>,
}

test_class!(InitPropertiesTestClass {
    constructors: [() => new::<InitPropertiesTestClass>],
    properties: [
        Prop1: Option<String> { get: g!(InitPropertiesTestClass, prop1), set: s!(InitPropertiesTestClass, prop1: Str) },
        Prop2: i32 { get: g!(InitPropertiesTestClass, prop2), set: s!(InitPropertiesTestClass, prop2: i32) },
    ],
});

#[derive(Default)]
pub(crate) struct RootNode {
    pub children: Rc<TestList<Object>>,
}

test_class!(RootNode {
    constructors: [() => new::<RootNode>],
    content: Children,
    properties: [Children: Rc<TestList<Option<BoxedValue>>> { get: |c: &Rc<RootNode>| c.children.clone() }],
});

#[derive(Default)]
pub(crate) struct TypeArgument;

test_class!(TypeArgument { constructors: [() => new::<TypeArgument>] });

#[derive(Default)]
pub(crate) struct ItemNode;

test_class!(ItemNode { constructors: [() => new::<ItemNode>] });

/// `GenericClass<TypeArgument>`.
#[derive(Default)]
pub(crate) struct GenericClassOfTypeArgument {
    pub items: Rc<TestList<Object>>,
}

identity_eq!(GenericClassOfTypeArgument);

ferro_markup_type!(class GenericClassOfTypeArgument as "GenericClass`1" {
    handles: [Rc<GenericClassOfTypeArgument>, Option<Rc<GenericClassOfTypeArgument>>],
    this: Rc<GenericClassOfTypeArgument>,
    namespace: "RtXamlParserTests",
    generic: "GenericClass`1" [Rc<TypeArgument>],
    constructors: [() => new::<GenericClassOfTypeArgument>],
    content: Items,
    properties: [
        Items: Rc<TestList<Option<BoxedValue>>> { get: |c: &Rc<GenericClassOfTypeArgument>| c.items.clone() },
    ],
});

// --- DynamicSettersTests.cs ------------------------------------------------

/// `SpecialHandler<T>`.
pub(crate) struct SpecialHandler<T> {
    pub called: Cell<bool>,
    pub value: RefCell<T>,
}

impl<T> PartialEq for SpecialHandler<T> {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

macro_rules! special_handler {
    ($value:ty) => {
        ferro_markup_type!(class SpecialHandler<$value> as "SpecialHandler`1" {
            handles: [Rc<SpecialHandler<$value>>, Option<Rc<SpecialHandler<$value>>>],
            this: Rc<SpecialHandler<$value>>,
            namespace: "RtXamlParserTests",
            generic: "SpecialHandler`1" [$value],
            methods: [
                fn Handle($value) => |h: &Rc<SpecialHandler<$value>>, v: $value| {
                    h.called.set(true);
                    *h.value.borrow_mut() = v;
                }
            ],
        });
    };
}

special_handler!(Option<Uri>);
special_handler!(TimeSpan);
special_handler!(Option<TimeSpan>);
special_handler!(Option<String>);

/// `DynamicSettersClass<T1, T2>`.
pub(crate) struct DynamicSettersClass<T1, T2> {
    pub value: RefCell<T1>,
    pub is_value_set: Cell<bool>,
    pub special_handler: Rc<SpecialHandler<T2>>,
}

impl<T1, T2> PartialEq for DynamicSettersClass<T1, T2> {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl<T1: Default, T2: Default> Default for DynamicSettersClass<T1, T2> {
    fn default() -> Self {
        Self {
            value: RefCell::new(T1::default()),
            is_value_set: Cell::new(false),
            special_handler: Rc::new(SpecialHandler { called: Cell::new(false), value: RefCell::new(T2::default()) }),
        }
    }
}

macro_rules! dynamic_setters_class {
    ($value1:ty, $value2:ty) => {
        ferro_markup_type!(class DynamicSettersClass<$value1, $value2> as "DynamicSettersClass`2" {
            handles: [
                Rc<DynamicSettersClass<$value1, $value2>>,
                Option<Rc<DynamicSettersClass<$value1, $value2>>>
            ],
            this: Rc<DynamicSettersClass<$value1, $value2>>,
            namespace: "RtXamlParserTests",
            generic: "DynamicSettersClass`2" [$value1, $value2],
            constructors: [() => new::<DynamicSettersClass<$value1, $value2>>],
            properties: [
                Value: $value1 {
                    get: |c: &Rc<DynamicSettersClass<$value1, $value2>>| c.value.borrow().clone(),
                    set: |c: &Rc<DynamicSettersClass<$value1, $value2>>, v: $value1| {
                        *c.value.borrow_mut() = v;
                        c.is_value_set.set(true);
                    }
                },
                SpecialHandler: Rc<SpecialHandler<$value2>> {
                    get: |c: &Rc<DynamicSettersClass<$value1, $value2>>| c.special_handler.clone()
                },
            ],
        });
    };
}

dynamic_setters_class!(Option<String>, Option<Uri>);
dynamic_setters_class!(i32, TimeSpan);
dynamic_setters_class!(Option<i32>, Option<TimeSpan>);
dynamic_setters_class!(i32, Option<String>);
dynamic_setters_class!(Option<i32>, Option<String>);

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum ProvidedValueType {
    #[default]
    Null,
    String,
    Uri,
    Int32,
    TimeSpan,
    DateTime,
}

ferro_markup_enum!(ProvidedValueType { Null, String, Uri, Int32, TimeSpan, DateTime }, { namespace: "RtXamlParserTests" });

/// A value of a type none of the setters takes (upstream: a `DateTime`).
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct UnrelatedValue;

#[derive(Default)]
pub(crate) struct DynamicProvider {
    pub provided_value: RefCell<ProvidedValueType>,
}

test_class!(DynamicProvider {
    constructors: [() => new::<DynamicProvider>],
    properties: [
        ProvidedValue: ProvidedValueType {
            get: g!(DynamicProvider, provided_value), set: s!(DynamicProvider, provided_value: ProvidedValueType)
        },
    ],
    methods: [
        fn ProvideValue() -> Option<BoxedValue> => |p: &Rc<DynamicProvider>| -> Object {
            match *p.provided_value.borrow() {
                ProvidedValueType::Null => None,
                ProvidedValueType::String => Some(Rc::new("foo".to_string())),
                ProvidedValueType::Uri => Some(Rc::new(Uri::absolute("https://ferroui.net/").expect("a URI"))),
                ProvidedValueType::Int32 => Some(Rc::new(1234i32)),
                ProvidedValueType::TimeSpan => Some(Rc::new(TimeSpan::from_seconds(45296.789))),
                ProvidedValueType::DateTime => Some(Rc::new(UnrelatedValue)),
            }
        }
    ],
});

// --- Registration ------------------------------------------------------------

static ASSEMBLY: MarkupAssembly = MarkupAssembly {
    name: "RtXamlParserTests",
    crate_name: "ferroui_markup_xaml_loader",
    xmlns_definitions: &[XmlnsDefinition { xml_namespace: "rt-test", namespace: "RtXamlParserTests" }],
    xmlns_prefixes: &[],
    metadata: &[],
};

macro_rules! markup {
    ($($type_:ty),* $(,)?) => {
        &[$(<$type_ as MarkupTyped>::MARKUP),*]
    };
}

/// Registers a cast of the handle of a derived class to the handle of its
/// base class.
fn upcast<TDerived: 'static, TBase: PartialEq + 'static>(base: fn(&Rc<TDerived>) -> Rc<TBase>) {
    ValueTypes::register_cast::<Rc<TDerived>, Rc<TBase>>(base);
}

macro_rules! nullable {
    ($($type_:ty),* $(,)?) => {
        $(ValueTypes::register_nullable::<$type_>();)*
    };
}

/// Makes the test types known: the metadata once per process, the casts
/// between the handles once per thread.
pub(crate) fn register() {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| {
        ferroui_base::register_types();
        register_service_types();
        MarkupAssembly::register(&ASSEMBLY);
        ferroui_base::TypeInfo::register_namespaces(&[(module_path!(), "RtXamlParserTests")]);
        ferroui_base::TypeInfo::register(<RtAttachedOwner as ferroui_base::StaticType>::TYPE);
        let types: &[&'static MarkupType] = markup![
            TestList<Rc<SimpleSubClass>>,
            TestList<Rc<ServiceProviderTestsClass>>,
            TestList<i32>,
            TestList<Option<BoxedValue>>,
            TestEnumerable<Rc<SimpleSubClass>>,
            TestDictionary<Option<BoxedValue>, Option<BoxedValue>>,
            TestDictionary<String, i32>,
            dyn ITestAddChild,
            dyn ITestAddChildOfString,
            dyn ITestSupportInitialize,
            SimpleSubClass,
            SimpleClass,
            ObjectWithAddChild,
            ObjectWithGenericAddChild,
            ObjectWithoutMatchingCtor,
            RtClassWithContentAttribute,
            RtSubClassWithContentAttributeOverride,
            EnumerableContentClass,
            SimpleClassWithDictionaryContent,
            IntrinsicsTestsClass,
            IntrinsicsTestsDerivedClass,
            IntrinsicsListTestsClass,
            IntrinsicsTestsEnum,
            InitializationTestsChildren,
            InitializationTestsClass,
            InitializationTestsSupportInitializeClass,
            InitializationTestsTopDownClass,
            CallbackExtension,
            DeferredContentTestsClass,
            DeferredValue,
            DeferredContentTests,
            ServiceProviderTestsClass,
            ServiceProviderTests,
            UnknownServiceUsageExtension,
            MarkupExtensionTestsClass,
            MarkupExtensionContentDictionaryClass,
            ObjectTestExtension,
            GenericTestExtension,
            GenericTestExtension1<i32>,
            GenericTestExtension1<Option<String>>,
            GenericTestExtension1<Option<BoxedValue>>,
            GenericTestExtension2,
            ServiceProviderValueExtension,
            ServiceProviderIntValueExtension,
            ServiceProviderIntListExtension,
            ConvertersTestValueType,
            ConvertersTestsEnum,
            RtUriKind,
            TestConverter,
            PropertyTestConverter,
            NegativeIntConverter,
            ConvertersTestsClassWithConverter,
            ConvertersTestsClassWithoutConverter,
            ConvertersTestClass,
            ConvertersTestsClassWithConstructor,
            RtClassWithTwoContentAttributes,
            RtEventSource,
            RtLength,
            RtLengthUnit,
            RtAttachedOwner,
            InitPropertiesTestClass,
            RootNode,
            TypeArgument,
            ItemNode,
            GenericClassOfTypeArgument,
            SpecialHandler<Option<Uri>>,
            SpecialHandler<TimeSpan>,
            SpecialHandler<Option<TimeSpan>>,
            SpecialHandler<Option<String>>,
            DynamicSettersClass<Option<String>, Option<Uri>>,
            DynamicSettersClass<i32, TimeSpan>,
            DynamicSettersClass<Option<i32>, Option<TimeSpan>>,
            DynamicSettersClass<i32, Option<String>>,
            DynamicSettersClass<Option<i32>, Option<String>>,
            ProvidedValueType,
            DynamicProvider,
        ];
        MarkupType::register_all(types);
    });

    thread_local! {
        static REGISTERED: Cell<bool> = const { Cell::new(false) };
    }
    if REGISTERED.replace(true) {
        return;
    }
    nullable![
        Uri,
        TimeSpan,
        Rc<InitializationTestsClass>,
        Rc<ServiceProviderTestsClass>,
        Rc<ConvertersTestsClassWithConverter>,
        Rc<ConvertersTestsClassWithoutConverter>,
        Rc<dyn ITypeDescriptorContext>,
    ];
    upcast::<IntrinsicsTestsDerivedClass, IntrinsicsTestsClass>(|d| d.base.clone());
    upcast::<RtSubClassWithContentAttributeOverride, RtClassWithContentAttribute>(|d| d.base.clone());
    upcast::<InitializationTestsSupportInitializeClass, InitializationTestsClass>(|d| d.base.clone());
    upcast::<InitializationTestsTopDownClass, InitializationTestsSupportInitializeClass>(|d| d.base.clone());
    upcast::<InitializationTestsTopDownClass, InitializationTestsClass>(|d| d.base.base.clone());
    ValueTypes::register_cast::<Rc<ObjectWithAddChild>, Rc<dyn ITestAddChild>>(|o| o.clone());
    ValueTypes::register_cast::<Rc<ObjectWithGenericAddChild>, Rc<dyn ITestAddChild>>(|o| o.clone());
    ValueTypes::register_cast::<Rc<ObjectWithGenericAddChild>, Rc<dyn ITestAddChildOfString>>(|o| o.clone());
    ValueTypes::register_cast::<Rc<InitializationTestsSupportInitializeClass>, Rc<dyn ITestSupportInitialize>>(|o| {
        o.clone()
    });
    ValueTypes::register_cast::<Rc<InitializationTestsTopDownClass>, Rc<dyn ITestSupportInitialize>>(|o| {
        o.base.clone()
    });
}
