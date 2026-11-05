//! Not a port of an upstream file: the types the `{x:Bind}` fixtures of step X2 of #525 bind
//! to (docs/porting/x-bind.md), after the models of WinUI's `FunctionTests` and
//! `CastingTests` (`src/XamlCompiler/Tests/RegressionProjects/Features/CompiledBinding/BindTestbedCS`
//! of microsoft-ui-xaml): a page with code-behind functions, a model with functions, indexers
//! and an interface, and a static type with functions and a static property. Generated code
//! names them by the public Rust paths registered here.

use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};

use ferroui_base::data::core::ValueTypes;
use ferroui_base::data::model::{Event, INotifyPropertyChanged};
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::metadata::{MarkupType, MarkupTyped};
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, ferro_markup_type, ferro_properties, instantiate, BoxedValue,
    FerroObjectImpl, FerroProperty, Ref, StyledElementImpl, StyledProperty, TypeInfo, VisualImpl,
};
use ferroui_controls::{ControlImpl, Panel, PanelImpl};

use super::TypeModule;

// --- The page ----------------------------------------------------------------

/// The root of the function fixture, with the code-behind WinUI's x:Bind reads: a registered
/// property (`Title`), a plain property (`Model`) and functions (`Format`, `ParseTitle`, the
/// `BindBack` of a `TwoWay` binding through `Format`).
#[repr(C)]
pub struct XBindPage {
    base: Panel,
    model: Rc<XBindModel>,
    parsed: RefCell<Vec<String>>,
}

ferro_class!(XBindPage: Panel);
ferro_impl_classes!(
    XBindPage: FerroObjectImpl,
    StyledElementImpl,
    VisualImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl,
    PanelImpl
);
ferro_class_info!(XBindPage {
    new: XBindPage::new,
    markup: {
        namespace: "FerroUI.Markup.Xaml.UnitTests",
        properties: [
            Model: Rc<XBindModel> { get: |this: &Ref<XBindPage>| this.model() },
        ],
        methods: [
            fn Format(Option<String>) -> String => |this: &Ref<XBindPage>, title: Option<String>| this.format(title),
            fn ParseTitle(Option<String>) => |this: &Ref<XBindPage>, text: Option<String>| this.parse_title(text),
        ],
    },
});

ferro_properties! {
    impl XBindPage {
        pub fn title_property() -> StyledProperty<Option<String>> {
            FerroProperty::register::<XBindPage, _>("Title", None)
        }
    }
}

impl XBindPage {
    pub fn construct() -> Self {
        Self { base: Panel::construct(), model: XBindModel::new("Grace"), parsed: RefCell::new(Vec::new()) }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    pub fn title(&self) -> Option<String> {
        self.get_value(Self::title_property())
    }

    pub fn set_title(&self, title: &str) {
        self.set_value(Self::title_property(), Some(title.to_string()));
    }

    pub fn model(&self) -> Rc<XBindModel> {
        self.model.clone()
    }

    /// `string Format(string title)`: a function on the root.
    pub fn format(&self, title: Option<String>) -> String {
        format!("Title: {}", title.unwrap_or_default())
    }

    /// The `BindBack` of `Format`: the title of a formatted text.
    pub fn parse_title(&self, text: Option<String>) {
        let text = text.unwrap_or_default();
        self.parsed.borrow_mut().push(text.clone());
        let title = text.strip_prefix("Title: ").unwrap_or(&text).to_string();
        self.set_title(&title);
    }

    /// The texts `ParseTitle` was called with, in order.
    pub fn parsed(&self) -> Vec<String> {
        self.parsed.borrow().clone()
    }
}

// --- The model ---------------------------------------------------------------

/// A contract the model implements: what `path.(local:IXBindNamed.DisplayName)` reads.
pub trait IXBindNamed {
    fn display_name(&self) -> String;
}

crate::test_identity_eq!(dyn IXBindNamed);

ferro_markup_type!(interface dyn IXBindNamed as "IXBindNamed" {
    this: Rc<dyn IXBindNamed>,
    handles: [Rc<dyn IXBindNamed>, Option<Rc<dyn IXBindNamed>>],
    namespace: "FerroUI.Markup.Xaml.UnitTests",
    properties: [
        DisplayName: String { get: |this: &Rc<dyn IXBindNamed>| this.display_name() },
    ],
});

/// The model of the fixtures (WinUI's `DataModel`): a notifying `Name`, functions with path
/// and literal arguments (`Greet`, `Shout`), a function re-read on a `PropertyChanged` with
/// its name (`Stamp`), the `BindBack` of a path (`Rename`), and an integer and a string indexer
/// (`Items`, `Map`).
pub struct XBindModel {
    this: Weak<XBindModel>,
    name: RefCell<String>,
    stamp: Cell<u32>,
    items: Rc<XBindItems>,
    map: Rc<XBindMap>,
    property_changed: Event<str>,
}

crate::test_identity_eq!(XBindModel);

impl INotifyPropertyChanged for XBindModel {
    fn property_changed(&self) -> &Event<str> {
        &self.property_changed
    }
}

impl IXBindNamed for XBindModel {
    fn display_name(&self) -> String {
        format!("named {}", self.name())
    }
}

impl XBindModel {
    pub fn new(name: &str) -> Rc<Self> {
        Rc::new_cyclic(|this| Self {
            this: this.clone(),
            name: RefCell::new(name.to_string()),
            stamp: Cell::new(1),
            items: Rc::new(XBindItems),
            map: Rc::new(XBindMap),
            property_changed: Event::new(),
        })
    }

    fn as_named(&self) -> Rc<dyn IXBindNamed> {
        self.this.upgrade().expect("the model is alive while it is used")
    }

    pub fn name(&self) -> String {
        self.name.borrow().clone()
    }

    pub fn set_name(&self, name: &str) {
        let changed = *self.name.borrow() != name;
        if changed {
            *self.name.borrow_mut() = name.to_string();
            self.property_changed.raise("Name");
        }
    }

    /// `string Greet(string name, string greeting, int times)`.
    pub fn greet(&self, name: Option<String>, greeting: String, times: i32) -> String {
        format!("{greeting} {} x{times}", name.unwrap_or_default())
    }

    /// `string Shout(string text)`.
    pub fn shout(&self, text: String) -> String {
        text.to_uppercase()
    }

    /// The `BindBack` of `Shout`: the name of a shouted text.
    pub fn rename(&self, text: String) {
        self.set_name(&text.to_lowercase());
    }

    /// `string Stamp()`: changes without an argument; the model raises `PropertyChanged`
    /// with the name of the function when it does.
    pub fn stamp(&self) -> String {
        format!("stamp {}", self.stamp.get())
    }

    pub fn bump(&self) {
        self.stamp.set(self.stamp.get() + 1);
        self.property_changed.raise("Stamp");
    }

    pub fn items(&self) -> Rc<XBindItems> {
        self.items.clone()
    }

    pub fn map(&self) -> Rc<XBindMap> {
        self.map.clone()
    }
}

ferro_markup_type!(class XBindModel {
    this: Rc<XBindModel>,
    handles: [XBindModel, Rc<XBindModel>, Option<Rc<XBindModel>>],
    namespace: "FerroUI.Markup.Xaml.UnitTests",
    interfaces: [Rc<dyn IXBindNamed>],
    properties: [
        Name: String {
            get: |this: &Rc<XBindModel>| this.name(),
            set: |this: &Rc<XBindModel>, value: String| this.set_name(&value)
        },
        Items: Rc<XBindItems> { get: |this: &Rc<XBindModel>| this.items() },
        Map: Rc<XBindMap> { get: |this: &Rc<XBindModel>| this.map() },
    ],
    methods: [
        fn Greet(Option<String>, String, i32) -> String =>
            |this: &Rc<XBindModel>, name: Option<String>, greeting: String, times: i32| this.greet(name, greeting, times),
        fn Shout(String) -> String => |this: &Rc<XBindModel>, text: String| this.shout(text),
        fn Rename(String) => |this: &Rc<XBindModel>, text: String| this.rename(text),
        fn Stamp() -> String => |this: &Rc<XBindModel>| this.stamp(),
    ],
    notify_property_changed: XBindModel,
});

/// A type with an integer indexer: `Items[1]`.
pub struct XBindItems;

crate::test_identity_eq!(XBindItems);

ferro_markup_type!(class XBindItems {
    this: Rc<XBindItems>,
    handles: [Rc<XBindItems>, Option<Rc<XBindItems>>],
    namespace: "FerroUI.Markup.Xaml.UnitTests",
    indexers: [
        (i32) -> String { get: |_this: &Rc<XBindItems>, index: i32| format!("item {index}") },
    ],
});

/// A type with a string indexer: `Map['key']`.
pub struct XBindMap;

crate::test_identity_eq!(XBindMap);

ferro_markup_type!(class XBindMap {
    this: Rc<XBindMap>,
    handles: [Rc<XBindMap>, Option<Rc<XBindMap>>],
    namespace: "FerroUI.Markup.Xaml.UnitTests",
    indexers: [
        (String) -> String { get: |_this: &Rc<XBindMap>, key: String| format!("value of {key}") },
    ],
});

// --- The static functions ----------------------------------------------------

/// Static functions and a static property: `local:XBindHelpers.Join(Title, '-', 2)`,
/// `local:XBindHelpers.Prefix`.
pub struct XBindHelpers;

impl XBindHelpers {
    /// `static string Join(string text, string separator, int count)`.
    pub fn join(text: Option<String>, separator: String, count: i32) -> String {
        let text = text.unwrap_or_default();
        vec![text; usize::try_from(count).unwrap_or(0)].join(separator.as_str())
    }

    /// `static string Describe(string text, double number, bool flag, object value)`: what a
    /// call with one literal of each kind receives.
    pub fn describe(text: String, number: f64, flag: bool, value: Option<BoxedValue>) -> String {
        let value = match value {
            Some(value) => ValueTypes::to_display_string(Some(&value)),
            None => "null".to_string(),
        };
        format!("{text}|{number}|{flag}|{value}")
    }

    /// `static void Store(string text)`: the `BindBack` of a static function.
    pub fn store(text: String) {
        STORED.with(|stored| stored.borrow_mut().push(text));
    }

    /// The texts `Store` was called with on this thread, in order.
    pub fn stored() -> Vec<String> {
        STORED.with(|stored| stored.borrow().clone())
    }

    pub fn prefix() -> String {
        "prefix".to_string()
    }
}

ferroui_base::ferro_thread_local! {
    static STORED: RefCell<Vec<String>> = RefCell::new(Vec::new());
}

ferro_markup_type!(static XBindHelpers {
    namespace: "FerroUI.Markup.Xaml.UnitTests",
    methods: [
        static fn Join(Option<String>, String, i32) -> String => XBindHelpers::join,
        static fn Describe(String, f64, bool, Option<BoxedValue>) -> String => XBindHelpers::describe,
        static fn Store(String) => XBindHelpers::store,
    ],
    static_properties: [
        Prefix: String { get: XBindHelpers::prefix },
    ],
});

pub(crate) const MODULE: TypeModule = TypeModule {
    types: &[XBindPage::TYPE],
    markup_types: &[
        <dyn IXBindNamed as MarkupTyped>::MARKUP,
        <XBindModel as MarkupTyped>::MARKUP,
        <XBindItems as MarkupTyped>::MARKUP,
        <XBindMap as MarkupTyped>::MARKUP,
        <XBindHelpers as MarkupTyped>::MARKUP,
    ],
    value_types: || {
        ValueTypes::register_reference::<XBindModel>();
        ValueTypes::register_nullable::<Rc<XBindItems>>();
        ValueTypes::register_nullable::<Rc<XBindMap>>();
        ValueTypes::register_nullable::<Rc<dyn IXBindNamed>>();
        ValueTypes::register_cast::<XBindModel, Rc<dyn IXBindNamed>>(XBindModel::as_named);
    },
};

/// The public Rust paths of the classes of this module, for generated code.
pub(crate) const RUST_PATHS: &[(&TypeInfo, &str)] =
    &[(XBindPage::TYPE, "ferroui_markup_xaml_tests::support::x_bind::XBindPage")];

/// The public Rust path of the type whose accessor of a registered property generated code
/// calls (`XBindPage::title_property()`), by the type itself.
pub(crate) const TYPE_RUST_PATHS: &[(fn() -> std::any::TypeId, &str)] =
    &[(|| std::any::TypeId::of::<XBindPage>(), "ferroui_markup_xaml_tests::support::x_bind::XBindPage")];

/// The public Rust paths of the types with markup metadata of this module, for generated code.
pub(crate) const MARKUP_RUST_PATHS: &[(&MarkupType, &str, bool)] = &[
    (<dyn IXBindNamed as MarkupTyped>::MARKUP, "ferroui_markup_xaml_tests::support::x_bind::IXBindNamed", true),
    (<XBindModel as MarkupTyped>::MARKUP, "ferroui_markup_xaml_tests::support::x_bind::XBindModel", false),
    (<XBindItems as MarkupTyped>::MARKUP, "ferroui_markup_xaml_tests::support::x_bind::XBindItems", false),
    (<XBindMap as MarkupTyped>::MARKUP, "ferroui_markup_xaml_tests::support::x_bind::XBindMap", false),
    (<XBindHelpers as MarkupTyped>::MARKUP, "ferroui_markup_xaml_tests::support::x_bind::XBindHelpers", false),
];
