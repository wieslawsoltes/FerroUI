//! Not a port of an upstream file: the types the `{x:Bind}` fixtures of steps X2 to X4 of #525
//! bind to (docs/porting/x-bind.md), after the models of WinUI's `FunctionTests`,
//! `CastingTests`, `TwoWayTests` and `NullableTests`
//! (`src/XamlCompiler/Tests/RegressionProjects/Features/CompiledBinding/BindTestbedCS` of
//! microsoft-ui-xaml): a page with code-behind functions, a model with functions, indexers and
//! an interface, a static type with functions and a static property, and a value converter.
//! Generated code names them by the public Rust paths registered here.

use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};

use ferroui_base::data::converters::IValueConverter;
use ferroui_base::data::core::{ValueType, ValueTypes};
use ferroui_base::data::model::{Event, INotifyPropertyChanged};
use ferroui_base::data::BindingError;
use ferroui_base::utilities::CultureInfo;
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::{IRoutedEventArgs, InteractiveImpl};
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::metadata::{MarkupDelegate, MarkupType, MarkupTyped};
use ferroui_base::reactive::{Disposable, IDisposable};
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, ferro_markup_type, ferro_properties, instantiate, BoxedValue,
    FerroObjectImpl, FerroProperty, Ref, StyledElementImpl, StyledProperty, TypeInfo, VisualImpl,
};
use ferroui_controls::{ControlImpl, Panel, PanelImpl};

use super::TypeModule;

// --- The page ----------------------------------------------------------------

/// The root of the function fixture, with the code-behind WinUI's x:Bind reads: a registered
/// property (`Title`), a plain property (`Model`) and functions (`Format`, `ParseTitle`, the
/// `BindBack` of a `TwoWay` binding through `Format`); for the event fixture (step X4, after
/// WinUI's `EventTests`), handlers with the event's signature (`OnClick`), without parameters
/// (`OnClickNoArgs`) and with base parameters (`OnClickBase`), a settable plain property
/// (`Current`) and an event that is not a routed event (`Pinged`, with `OnPinged`).
#[repr(C)]
pub struct XBindPage {
    base: Panel,
    model: Rc<XBindModel>,
    parsed: RefCell<Vec<String>>,
    clicks: RefCell<Vec<String>>,
    current: RefCell<Option<Rc<XBindModel>>>,
    pinged: RefCell<Vec<(u64, MarkupDelegate)>>,
    next_ping: Cell<u64>,
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
            Current: Option<Rc<XBindModel>> {
                get: |this: &Ref<XBindPage>| this.current(),
                set: |this: &Ref<XBindPage>, value: Option<Rc<XBindModel>>| this.set_current(value)
            },
        ],
        methods: [
            fn Format(Option<String>) -> String => |this: &Ref<XBindPage>, title: Option<String>| this.format(title),
            fn ParseTitle(Option<String>) => |this: &Ref<XBindPage>, text: Option<String>| this.parse_title(text),
            fn OnClick(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<XBindPage>, sender: Option<BoxedValue>, _e: Rc<dyn IRoutedEventArgs>| {
                    this.record(if sender.is_some() { "regular" } else { "regular without a sender" })
                },
            fn OnClickNoArgs() => |this: &Ref<XBindPage>| this.record("no arguments"),
            fn OnClickBase(Option<BoxedValue>, Option<BoxedValue>) =>
                |this: &Ref<XBindPage>, _sender: Option<BoxedValue>, e: Option<BoxedValue>| {
                    this.record(if e.is_some() { "base" } else { "base without arguments" })
                },
            fn OnPinged(Option<BoxedValue>, String) =>
                |this: &Ref<XBindPage>, _sender: Option<BoxedValue>, text: String| this.record(&format!("pinged {text}")),
        ],
        events: [
            Pinged(Option<BoxedValue>, String) => |this: &Ref<XBindPage>, handler: MarkupDelegate| this.add_pinged(handler),
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
        Self {
            base: Panel::construct(),
            model: XBindModel::new("Grace"),
            parsed: RefCell::new(Vec::new()),
            clicks: RefCell::new(Vec::new()),
            current: RefCell::new(None),
            pinged: RefCell::new(Vec::new()),
            next_ping: Cell::new(1),
        }
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

    /// Sets the title to null.
    pub fn clear_title(&self) {
        self.set_value(Self::title_property(), None);
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

    fn record(&self, call: &str) {
        self.clicks.borrow_mut().push(call.to_string());
    }

    /// What the event handlers of the page were called for, in order.
    pub fn clicks(&self) -> Vec<String> {
        self.clicks.borrow().clone()
    }

    /// `XBindModel Current`: a plain property without change notification.
    pub fn current(&self) -> Option<Rc<XBindModel>> {
        self.current.borrow().clone()
    }

    pub fn set_current(&self, model: Option<Rc<XBindModel>>) {
        *self.current.borrow_mut() = model;
    }

    /// `event EventHandler<string> Pinged`: the subscription removes the handler.
    pub fn add_pinged(&self, handler: MarkupDelegate) -> Rc<dyn IDisposable> {
        let id = self.next_ping.get();
        self.next_ping.set(id + 1);
        self.pinged.borrow_mut().push((id, handler));
        let this = self.to_ref().downgrade();
        Disposable::create(move || {
            let Some(this) = this.upgrade() else {
                return;
            };
            let removed = {
                let mut handlers = this.pinged.borrow_mut();
                let index = handlers.iter().position(|(handler_id, _)| *handler_id == id);
                index.map(|index| handlers.remove(index))
            };
            drop(removed);
        })
    }

    /// The number of handlers of `Pinged`.
    pub fn pinged_handler_count(&self) -> usize {
        self.pinged.borrow().len()
    }

    /// Raises `Pinged` with `text`.
    pub fn ping(&self, text: &str) {
        let handlers: Vec<MarkupDelegate> = self.pinged.borrow().iter().map(|(_, handler)| handler.clone()).collect();
        for handler in handlers {
            handler.invoke(&[None, Some(Rc::new(text.to_string()) as BoxedValue)]);
        }
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
    pokes: Cell<u32>,
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
            pokes: Cell::new(0),
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

    /// `void Poke()`: an event handler of the model, without parameters.
    pub fn poke(&self) {
        self.pokes.set(self.pokes.get() + 1);
    }

    /// How many times `Poke` was called.
    pub fn pokes(&self) -> u32 {
        self.pokes.get()
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
        fn Poke() => |this: &Rc<XBindModel>| this.poke(),
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

    /// `static void Clicked()`: a static event handler without parameters.
    pub fn clicked() {
        CLICKED.with(|clicked| clicked.set(clicked.get() + 1));
    }

    /// How many times `Clicked` was called on this thread.
    pub fn clicked_count() -> u32 {
        CLICKED.with(Cell::get)
    }

    /// `static void Mismatched(object sender, string text)`: not the signature of a routed event.
    pub fn mismatched(_sender: Option<BoxedValue>, _text: String) {}

    /// `static void Overloaded()` and `static void Overloaded(object, RoutedEventArgs)`: an event
    /// cannot bind to an overloaded method.
    pub fn overloaded() {}

    pub fn overloaded_with(_sender: Option<BoxedValue>, _e: Rc<dyn IRoutedEventArgs>) {}
}

ferroui_base::ferro_thread_local! {
    static STORED: RefCell<Vec<String>> = RefCell::new(Vec::new());
    static CLICKED: Cell<u32> = const { Cell::new(0) };
}

ferro_markup_type!(static XBindHelpers {
    namespace: "FerroUI.Markup.Xaml.UnitTests",
    methods: [
        static fn Join(Option<String>, String, i32) -> String => XBindHelpers::join,
        static fn Describe(String, f64, bool, Option<BoxedValue>) -> String => XBindHelpers::describe,
        static fn Store(String) => XBindHelpers::store,
        static fn Clicked() => XBindHelpers::clicked,
        static fn Mismatched(Option<BoxedValue>, String) => XBindHelpers::mismatched,
        static fn Overloaded() => XBindHelpers::overloaded,
        static fn Overloaded(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) => XBindHelpers::overloaded_with,
    ],
    static_properties: [
        Prefix: String { get: XBindHelpers::prefix },
    ],
});

// --- The converter -------------------------------------------------------------

/// A value converter that shows what it is called with (step X3): `Convert` gives
/// `value|parameter|culture` (`-` for no parameter, `null` for a null value), `ConvertBack` the
/// text before the first `|`, upper-cased, and records `value|parameter|culture`.
pub struct XBindTagConverter {
    this: Weak<XBindTagConverter>,
}

crate::test_identity_eq!(XBindTagConverter);

fn converter_text(value: Option<&BoxedValue>) -> String {
    match value.and_then(|value| value.downcast_ref::<String>()) {
        Some(text) => text.clone(),
        None => "null".to_string(),
    }
}

impl XBindTagConverter {
    pub fn new() -> Rc<XBindTagConverter> {
        Rc::new_cyclic(|this| Self { this: this.clone() })
    }

    /// The converter as the value converter contract.
    pub fn as_value_converter(&self) -> Rc<dyn IValueConverter> {
        self.this.upgrade().expect("the converter is alive while it is used")
    }

    /// What `ConvertBack` was called with on this thread, in order (`value|parameter|culture`).
    pub fn converted_back() -> Vec<String> {
        CONVERTED_BACK.with(|log| log.borrow().clone())
    }
}

impl IValueConverter for XBindTagConverter {
    fn convert(
        &self,
        value: Option<&BoxedValue>,
        _target_type: ValueType,
        parameter: Option<&BoxedValue>,
        culture: &CultureInfo,
    ) -> Result<Option<BoxedValue>, BindingError> {
        let parameter = match parameter {
            Some(parameter) => converter_text(Some(parameter)),
            None => "-".to_string(),
        };
        Ok(Some(Rc::new(format!("{}|{parameter}|{}", converter_text(value), culture.name()))))
    }

    fn convert_back(
        &self,
        value: Option<&BoxedValue>,
        _target_type: ValueType,
        parameter: Option<&BoxedValue>,
        culture: &CultureInfo,
    ) -> Result<Option<BoxedValue>, BindingError> {
        let text = converter_text(value);
        let parameter = match parameter {
            Some(parameter) => converter_text(Some(parameter)),
            None => "-".to_string(),
        };
        CONVERTED_BACK.with(|log| log.borrow_mut().push(format!("{text}|{parameter}|{}", culture.name())));
        let head = text.split('|').next().unwrap_or_default().to_uppercase();
        Ok(Some(Rc::new(head)))
    }
}

ferroui_base::ferro_thread_local! {
    static CONVERTED_BACK: RefCell<Vec<String>> = RefCell::new(Vec::new());
}

ferro_markup_type!(class XBindTagConverter {
    this: Rc<XBindTagConverter>,
    handles: [XBindTagConverter, Rc<XBindTagConverter>, Option<Rc<XBindTagConverter>>],
    namespace: "FerroUI.Markup.Xaml.UnitTests",
    interfaces: [Rc<dyn IValueConverter>],
    constructors: [() => XBindTagConverter::new],
});

pub(crate) const MODULE: TypeModule = TypeModule {
    types: &[XBindPage::TYPE],
    markup_types: &[
        <dyn IXBindNamed as MarkupTyped>::MARKUP,
        <XBindModel as MarkupTyped>::MARKUP,
        <XBindItems as MarkupTyped>::MARKUP,
        <XBindMap as MarkupTyped>::MARKUP,
        <XBindHelpers as MarkupTyped>::MARKUP,
        <XBindTagConverter as MarkupTyped>::MARKUP,
    ],
    value_types: || {
        ValueTypes::register_reference::<XBindModel>();
        ValueTypes::register_reference::<XBindTagConverter>();
        ValueTypes::register_cast::<XBindTagConverter, Rc<dyn IValueConverter>>(XBindTagConverter::as_value_converter);
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
    (<XBindTagConverter as MarkupTyped>::MARKUP, "ferroui_markup_xaml_tests::support::x_bind::XBindTagConverter", false),
];
