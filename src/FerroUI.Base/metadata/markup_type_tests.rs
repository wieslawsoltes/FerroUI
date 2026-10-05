//! Tests specific to this port: markup metadata declarations.
//!
//! The type names are unique in the crate: the tables of known types are
//! process-wide.

use super::*;
use crate::data::core::{ValueType, ValueTypes};
use crate::tests::test_class;
use crate::*;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

fn boxed<T: PartialEq + 'static>(value: T) -> MarkupValue {
    Some(Rc::new(value))
}

fn unbox<T: Clone + 'static>(value: &MarkupValue) -> T {
    from_markup_value::<T>(value).expect("value of the expected type")
}

// A value type: handle, parse, constructors, read-only properties.

#[derive(Clone, Copy, Debug, PartialEq)]
struct MetaPoint {
    x: f64,
    y: f64,
}

impl MetaPoint {
    fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }

    fn parse(text: &str) -> Result<Self, String> {
        let mut parts = text.split(',');
        let mut next = || parts.next().and_then(|p| p.trim().parse::<f64>().ok()).ok_or("Invalid point.".to_string());
        Ok(Self { x: next()?, y: next()? })
    }
}

ferro_markup_type!(struct MetaPoint {
    handles: [MetaPoint],
    parse: MetaPoint::parse,
    constructors: [
        () => || MetaPoint::new(0.0, 0.0),
        (f64, f64) => MetaPoint::new,
    ],
    properties: [
        X: f64 { get: |p: &MetaPoint| p.x },
        Y: f64 { get: |p: &MetaPoint| p.y } [Obsolete("Use X.")],
    ],
    methods: [
        fn Scale(f64) -> MetaPoint => |p: &MetaPoint, f: f64| MetaPoint::new(p.x * f, p.y * f),
        static fn Origin() -> MetaPoint => || MetaPoint::new(0.0, 0.0),
    ],
    fields: [
        Unit: MetaPoint => || MetaPoint::new(1.0, 1.0),
    ],
    attributes: [TypeConverter(type(MetaPoint)), FerroList(",", Separators = ";", SplitOptions = 3)],
});

#[test]
fn value_type_states_its_kind_name_and_handle() {
    let markup = <MetaPoint as MarkupTyped>::MARKUP;
    assert_eq!(markup.name, "MetaPoint");
    assert_eq!(markup.kind, MarkupTypeKind::Struct);
    assert_eq!(markup.handle(), Some(ValueType::of::<MetaPoint>()));
    assert!(markup.module_path.ends_with("markup_type_tests"));
}

#[test]
fn parse_converts_text_and_reports_failure() {
    let parse = <MetaPoint as MarkupTyped>::MARKUP.parse.unwrap();
    let value = parse(&[boxed("1.5, 2".to_string())]).unwrap();
    assert_eq!(unbox::<MetaPoint>(&value), MetaPoint::new(1.5, 2.0));

    assert_eq!(parse(&[boxed("x".to_string())]), Err(MarkupInvokeError::Failed("Invalid point.".to_string())));
}

#[test]
fn constructors_are_invoked_with_converted_arguments() {
    let markup = <MetaPoint as MarkupTyped>::MARKUP;
    assert_eq!(markup.constructors.len(), 2);
    assert!(markup.constructors[0].parameters.is_empty());
    let parameters: Vec<ValueType> = markup.constructors[1].parameters.iter().map(|p| p()).collect();
    assert_eq!(parameters, [ValueType::of::<f64>(), ValueType::of::<f64>()]);

    let origin = (markup.constructors[0].invoke)(&[]).unwrap();
    assert_eq!(unbox::<MetaPoint>(&origin), MetaPoint::new(0.0, 0.0));
    let point = (markup.constructors[1].invoke)(&[boxed(3.0f64), boxed(4.0f64)]).unwrap();
    assert_eq!(unbox::<MetaPoint>(&point), MetaPoint::new(3.0, 4.0));
}

#[test]
fn invokers_check_argument_count_and_types() {
    let constructor = <MetaPoint as MarkupTyped>::MARKUP.constructors[1];
    assert_eq!(
        (constructor.invoke)(&[boxed(3.0f64)]),
        Err(MarkupInvokeError::ArgumentCount { expected: 2, actual: 1 })
    );
    let error = (constructor.invoke)(&[boxed(3.0f64), boxed("4".to_string())]).unwrap_err();
    assert!(matches!(error, MarkupInvokeError::Argument { index: 1, .. }), "{error:?}");
    let error = (constructor.invoke)(&[None, boxed(1.0f64)]).unwrap_err();
    assert!(matches!(error, MarkupInvokeError::Argument { index: 0, .. }), "{error:?}");
}

#[test]
fn properties_methods_and_fields_of_a_value_type() {
    let markup = <MetaPoint as MarkupTyped>::MARKUP;
    let point = boxed(MetaPoint::new(3.0, 4.0));

    let x = markup.find_property("X").unwrap();
    assert_eq!((x.type_)(), ValueType::of::<f64>());
    assert!(x.set.is_none());
    assert_eq!(unbox::<f64>(&(x.get.unwrap())(&[point.clone()]).unwrap()), 3.0);
    assert!(markup.find_property("Z").is_none());

    let scale = markup.find_methods("Scale").next().unwrap();
    assert!(!scale.is_static);
    assert_eq!((scale.return_type.unwrap())(), ValueType::of::<MetaPoint>());
    let scaled = (scale.invoke)(&[point, boxed(2.0f64)]).unwrap();
    assert_eq!(unbox::<MetaPoint>(&scaled), MetaPoint::new(6.0, 8.0));

    let origin = markup.find_methods("Origin").next().unwrap();
    assert!(origin.is_static);
    assert_eq!(unbox::<MetaPoint>(&(origin.invoke)(&[]).unwrap()), MetaPoint::new(0.0, 0.0));

    let unit = markup.find_field("Unit").unwrap();
    assert_eq!(unbox::<MetaPoint>(&(unit.get)()), MetaPoint::new(1.0, 1.0));
}

#[test]
fn attributes_carry_positional_and_named_arguments() {
    let markup = <MetaPoint as MarkupTyped>::MARKUP;

    let converter = markup.find_attribute(attributes::TYPE_CONVERTER).unwrap();
    assert_eq!(converter.arguments, [MarkupAttributeValue::Type(|| ValueType::of::<MetaPoint>())]);
    assert!(converter.properties.is_empty());

    let list = markup.find_attribute(attributes::FERRO_LIST).unwrap();
    assert_eq!(list.arguments, [MarkupAttributeValue::Str(",")]);
    assert_eq!(list.property("Separators"), Some(MarkupAttributeValue::Str(";")));
    assert_eq!(list.property("SplitOptions"), Some(MarkupAttributeValue::Int(3)));
    assert_eq!(list.property("Other"), None);

    let obsolete = markup.find_property("Y").unwrap().attributes;
    assert_eq!(obsolete.len(), 1);
    assert_eq!(obsolete[0].name, attributes::OBSOLETE);
    assert_eq!(obsolete[0].arguments, [MarkupAttributeValue::Str("Use X.")]);
}

// A plain shared (reference) type: settable properties, a collection, an
// event, attributes, a contract.

trait IMetaNamed {
    fn name(&self) -> String;
}

struct MetaItem {
    name: RefCell<String>,
    tag: RefCell<Option<BoxedValue>>,
    children: RefCell<Vec<Rc<MetaItem>>>,
    handlers: RefCell<Vec<MarkupDelegate>>,
}

impl PartialEq for MetaItem {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl IMetaNamed for MetaItem {
    fn name(&self) -> String {
        self.name.borrow().clone()
    }
}

impl MetaItem {
    fn new() -> Rc<Self> {
        Self::named(String::new())
    }

    fn named(name: String) -> Rc<Self> {
        Rc::new(Self {
            name: RefCell::new(name),
            tag: RefCell::new(None),
            children: RefCell::new(Vec::new()),
            handlers: RefCell::new(Vec::new()),
        })
    }

    fn item_name(&self) -> String {
        self.name.borrow().clone()
    }

    fn set_item_name(&self, name: String) {
        *self.name.borrow_mut() = name;
        for handler in self.handlers.borrow().iter() {
            handler.invoke(&[Some(Rc::new(self.item_name()))]);
        }
    }

    fn tag(&self) -> Option<BoxedValue> {
        self.tag.borrow().clone()
    }

    fn set_tag(&self, tag: Option<BoxedValue>) {
        *self.tag.borrow_mut() = tag;
    }

    fn add(&self, child: Rc<MetaItem>) {
        self.children.borrow_mut().push(child);
    }

    fn name_changed(&self, handler: MarkupDelegate) {
        self.handlers.borrow_mut().push(handler);
    }
}

ferro_markup_type!(interface dyn IMetaNamed as "IMetaNamed" {
    handles: [Rc<dyn IMetaNamed>],
});

ferro_markup_type!(class MetaItem {
    handles: [Rc<MetaItem>, Option<Rc<MetaItem>>],
    namespace: "Tests.Metadata",
    this: Rc<MetaItem>,
    interfaces: [Rc<dyn IMetaNamed>],
    content: Name,
    constructors: [
        () => MetaItem::new,
        (String) => MetaItem::named,
    ],
    properties: [
        Name: String { get: MetaItem::item_name, set: MetaItem::set_item_name } [DependsOn("Tag")],
        Tag: Option<BoxedValue> { get: MetaItem::tag, set: MetaItem::set_tag } [AssignBinding],
        Count: i32 { get: |item: &Rc<MetaItem>| item.children.borrow().len() as i32 },
        WriteOnly: String { set: MetaItem::set_item_name },
    ],
    methods: [
        fn Add(Rc<MetaItem>) => MetaItem::add,
    ],
    events: [
        NameChanged(String) => MetaItem::name_changed,
    ],
    attributes: [UsableDuringInitialization, TrimSurroundingWhitespace],
});

#[test]
fn reference_type_members_work_through_its_handle() {
    let markup = <MetaItem as MarkupTyped>::MARKUP;
    assert_eq!(markup.kind, MarkupTypeKind::Class);
    assert_eq!(markup.namespace(), "Tests.Metadata");
    assert_eq!(markup.full_name(), "Tests.Metadata.MetaItem");
    assert_eq!(markup.content_property, Some("Name"));
    assert_eq!((markup.interfaces[0])(), ValueType::of::<Rc<dyn IMetaNamed>>());
    assert!(markup.find_attribute(attributes::USABLE_DURING_INITIALIZATION).is_some());
    assert!(markup.find_attribute(attributes::TRIM_SURROUNDING_WHITESPACE).is_some());
    assert!(markup.find_attribute(attributes::CONTROL_TEMPLATE_SCOPE).is_none());

    let item = (markup.constructors[1].invoke)(&[boxed("a".to_string())]).unwrap();
    let typed = unbox::<Rc<MetaItem>>(&item);
    assert_eq!(typed.item_name(), "a");

    let name = markup.find_property("Name").unwrap();
    assert_eq!(name.attributes[0].name, attributes::DEPENDS_ON);
    (name.set.unwrap())(&[item.clone(), boxed("b".to_string())]).unwrap();
    assert_eq!(unbox::<String>(&(name.get.unwrap())(&[item.clone()]).unwrap()), "b");

    let write_only = markup.find_property("WriteOnly").unwrap();
    assert!(write_only.get.is_none());
    (write_only.set.unwrap())(&[item.clone(), boxed("c".to_string())]).unwrap();
    assert_eq!(typed.item_name(), "c");

    let add = markup.find_methods("Add").next().unwrap();
    assert!(add.return_type.is_none());
    let child = (markup.constructors[0].invoke)(&[]).unwrap();
    assert_eq!((add.invoke)(&[item.clone(), child]), Ok(None));
    let count = markup.find_property("Count").unwrap();
    assert_eq!(unbox::<i32>(&(count.get.unwrap())(&[item]).unwrap()), 1);
}

#[test]
fn untyped_properties_take_any_value_and_null() {
    let markup = <MetaItem as MarkupTyped>::MARKUP;
    let item = (markup.constructors[0].invoke)(&[]).unwrap();
    let tag = markup.find_property("Tag").unwrap();
    assert_eq!((tag.type_)(), ValueType::of::<Option<BoxedValue>>());

    (tag.set.unwrap())(&[item.clone(), boxed(5i32)]).unwrap();
    let value = (tag.get.unwrap())(&[item.clone()]).unwrap();
    assert_eq!(unbox::<i32>(&value), 5);

    (tag.set.unwrap())(&[item.clone(), None]).unwrap();
    assert_eq!((tag.get.unwrap())(&[item]).unwrap(), None);
}

#[test]
fn events_subscribe_untyped_handlers() {
    let markup = <MetaItem as MarkupTyped>::MARKUP;
    let item = (markup.constructors[0].invoke)(&[]).unwrap();
    let event = markup.find_event("NameChanged").unwrap();
    assert_eq!((event.arguments[0])(), ValueType::of::<String>());

    let seen = Rc::new(RefCell::new(Vec::new()));
    let handler = MarkupDelegate::new({
        let seen = seen.clone();
        move |arguments| {
            seen.borrow_mut().push(unbox::<String>(&arguments[0]));
            None
        }
    });
    (event.add)(&[item.clone(), boxed(handler)]).unwrap();

    let name = markup.find_property("Name").unwrap();
    (name.set.unwrap())(&[item, boxed("x".to_string())]).unwrap();
    assert_eq!(*seen.borrow(), ["x"]);
}

// Enumerations.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(i32)]
enum MetaDock {
    Left = 0,
    Bottom = 1,
    Right = 2,
    Top = 5,
}

ferro_markup_enum!(MetaDock { Left, Bottom, Right, Top });

bitflags::bitflags! {
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    struct MetaRoutes: i32 {
        const DIRECT = 1;
        const TUNNEL = 2;
        const BUBBLE = 4;
    }
}

ferro_markup_enum!(flags MetaRoutes {
    Direct = MetaRoutes::DIRECT,
    Tunnel = MetaRoutes::TUNNEL,
    Bubble = MetaRoutes::BUBBLE,
}, { namespace: "Tests.Metadata" });

#[test]
fn enumerations_list_their_members() {
    let markup = <MetaDock as MarkupTyped>::MARKUP;
    assert_eq!(markup.kind, MarkupTypeKind::Enum);
    assert!(!markup.is_flags);
    let members: Vec<(&str, i64)> = markup.enum_members.iter().map(|m| (m.name, m.value)).collect();
    assert_eq!(members, [("Left", 0), ("Bottom", 1), ("Right", 2), ("Top", 5)]);

    let top = markup.find_enum_member("top", true).unwrap();
    assert_eq!((top.get)().downcast_ref::<MetaDock>(), Some(&MetaDock::Top));
    assert!(markup.find_enum_member("top", false).is_none());
    assert!(markup.find_enum_member("Middle", true).is_none());
}

#[test]
fn flags_enumerations_list_their_bits() {
    let markup = <MetaRoutes as MarkupTyped>::MARKUP;
    assert!(markup.is_flags);
    assert_eq!(markup.namespace(), "Tests.Metadata");
    let members: Vec<(&str, i64)> = markup.enum_members.iter().map(|m| (m.name, m.value)).collect();
    assert_eq!(members, [("Direct", 1), ("Tunnel", 2), ("Bubble", 4)]);
    let bubble = markup.find_enum_member("Bubble", false).unwrap();
    assert_eq!((bubble.get)().downcast_ref::<MetaRoutes>(), Some(&MetaRoutes::BUBBLE));
}

// Registration.

#[test]
fn registered_types_are_found_by_name_and_by_handle() {
    MarkupType::register_all(&[<MetaItem as MarkupTyped>::MARKUP, <MetaPoint as MarkupTyped>::MARKUP]);
    MarkupType::register(<MetaItem as MarkupTyped>::MARKUP);

    let item = MarkupType::find("Tests.Metadata", "MetaItem").unwrap();
    assert!(std::ptr::eq(item, <MetaItem as MarkupTyped>::MARKUP));
    assert!(MarkupType::find("Tests.Other", "MetaItem").is_none());
    assert_eq!(MarkupType::registered_types().iter().filter(|t| std::ptr::eq(**t, item)).count(), 1);

    for handle in [std::any::TypeId::of::<Rc<MetaItem>>(), std::any::TypeId::of::<Option<Rc<MetaItem>>>()] {
        assert!(std::ptr::eq(MarkupType::find_by_handle(handle).unwrap(), item));
    }
    let point = MarkupType::find_by_handle(std::any::TypeId::of::<MetaPoint>()).unwrap();
    assert_eq!(point.name, "MetaPoint");
    assert!(MarkupType::find_by_handle(std::any::TypeId::of::<Option<MetaPoint>>()).is_none());
    // The nullable form of a value type is a type of its own.
    let nullable = MarkupType::find_by_nullable_handle(std::any::TypeId::of::<Option<MetaPoint>>()).unwrap();
    assert!(std::ptr::eq(nullable, point));
    assert!(MarkupType::find_by_nullable_handle(std::any::TypeId::of::<Option<Rc<MetaItem>>>()).is_none());
    assert!(<MetaItem as MarkupTyped>::MARKUP.nullable.is_none());
    assert_eq!((<MetaDock as MarkupTyped>::MARKUP.nullable.unwrap())(), ValueType::of::<Option<MetaDock>>());
}

#[test]
fn assemblies_state_their_xml_namespaces() {
    static ASSEMBLY: MarkupAssembly = MarkupAssembly {
        name: "Tests.MetadataAssembly",
        crate_name: "tests_metadata_assembly",
        xmlns_definitions: &[XmlnsDefinition { xml_namespace: "https://example.org/tests", namespace: "Tests.Metadata" }],
        xmlns_prefixes: &[XmlnsPrefix { xml_namespace: "https://example.org/tests", prefix: "t" }],
        metadata: &[(MarkupAssembly::CREATE_SOURCE_INFO, "true")],
    };
    MarkupAssembly::register(&ASSEMBLY);
    MarkupAssembly::register(&ASSEMBLY);

    let found = MarkupAssembly::find("Tests.MetadataAssembly").unwrap();
    assert!(std::ptr::eq(found, &ASSEMBLY));
    assert_eq!(MarkupAssembly::registered_assemblies().iter().filter(|a| std::ptr::eq(**a, found)).count(), 1);
    assert!(std::ptr::eq(MarkupAssembly::of_module("tests_metadata_assembly::views::main").unwrap(), found));
    assert!(MarkupAssembly::of_module("tests_metadata_assembly_other").is_none());
    assert_eq!(found.metadata_value(MarkupAssembly::CREATE_SOURCE_INFO), Some("true"));
    assert_eq!(found.metadata_value("Other"), None);
}

// Classes of the object model: `ferro_class_info!` with a markup part.

#[repr(C)]
pub struct MetaPanel {
    base: FerroObject,
    children: RefCell<Vec<Ref<FerroObject>>>,
    title: RefCell<String>,
    clicks: Cell<i32>,
}

ferro_class!(MetaPanel: FerroObject);
ferro_impl_classes!(MetaPanel: FerroObjectImpl);

impl MetaPanel {
    pub fn construct() -> Self {
        Self {
            base: FerroObject::construct(),
            children: RefCell::new(Vec::new()),
            title: RefCell::new(String::new()),
            clicks: Cell::new(0),
        }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
}

impl IMetaNamed for MetaPanelNamed {
    fn name(&self) -> String {
        self.0.title.borrow().clone()
    }
}

struct MetaPanelNamed(Ref<MetaPanel>);

impl From<Ref<MetaPanel>> for Rc<dyn IMetaNamed> {
    fn from(value: Ref<MetaPanel>) -> Self {
        Rc::new(MetaPanelNamed(value))
    }
}

impl PartialEq for dyn IMetaNamed {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::addr_eq(self, other)
    }
}

impl MetaPanel {
    fn with_title(title: String) -> Ref<Self> {
        let panel = Self::new();
        *panel.title.borrow_mut() = title;
        panel
    }

    fn title(&self) -> String {
        self.title.borrow().clone()
    }

    fn set_title(&self, title: String) {
        *self.title.borrow_mut() = title;
    }

    fn add(&self, child: Ref<FerroObject>) {
        self.children.borrow_mut().push(child);
    }

    fn on_click(&self, amount: i32) -> i32 {
        self.clicks.set(self.clicks.get() + amount);
        self.clicks.get()
    }
}

ferro_class_info!(MetaPanel {
    markup: {
        content: Children,
        constructors: [(String) => MetaPanel::with_title],
        properties: [
            Title: String { get: MetaPanel::title, set: MetaPanel::set_title },
            ChildCount: i32 { get: |p: &Ref<MetaPanel>| p.children.borrow().len() as i32 },
        ],
        methods: [
            fn Add(Ref<FerroObject>) => MetaPanel::add,
            fn OnClick(i32) -> i32 => MetaPanel::on_click,
        ],
        indexers: [
            (i32) -> i32 {
                get: |p: &Ref<MetaPanel>, offset: i32| p.clicks.get() + offset,
                set: |p: &Ref<MetaPanel>, offset: i32, value: i32| p.clicks.set(value - offset),
            },
        ],
        events: [
            Clicked() => |p: &Ref<MetaPanel>, handler: MarkupDelegate| {
                handler.invoke(&[]);
                p.clicks.set(p.clicks.get() + 100);
            },
        ],
        attributes: [TemplatePart("PART_Header", type(Ref<MetaPanel>), IsRequired = true)],
    },
    interfaces: [Rc<dyn IMetaNamed>],
    new: MetaPanel::new,
});

ferro_properties! {
    impl MetaPanel {
        pub fn spacing_property() -> StyledProperty<f64> {
            FerroProperty::register::<MetaPanel, _>("Spacing", 0.0)
        }
    }
}

test_class!(MetaDerivedPanel: MetaPanel);
ferro_class_info!(MetaDerivedPanel { new: MetaDerivedPanel::new });

test_class!(MetaPlain: FerroObject);

#[test]
fn class_markup_is_reached_from_the_runtime_type() {
    let markup = MetaPanel::TYPE.markup().unwrap();
    assert_eq!(markup.name, "MetaPanel");
    assert_eq!(markup.kind, MarkupTypeKind::Class);
    assert!(std::ptr::eq((markup.type_info.unwrap())(), MetaPanel::TYPE));
    assert_eq!(markup.handle(), Some(ValueType::of::<Ref<MetaPanel>>()));
    assert_eq!((markup.handles[1])(), ValueType::of::<Option<Ref<MetaPanel>>>());
    assert_eq!((markup.interfaces[0])(), ValueType::of::<Rc<dyn IMetaNamed>>());
    assert_eq!(markup.content_property, Some("Children"));

    let part = markup.find_attribute(attributes::TEMPLATE_PART).unwrap();
    assert_eq!(part.arguments[0], MarkupAttributeValue::Str("PART_Header"));
    assert_eq!(part.arguments[1], MarkupAttributeValue::Type(|| ValueType::of::<Ref<MetaPanel>>()));
    assert_eq!(part.property("IsRequired"), Some(MarkupAttributeValue::Bool(true)));
}

#[test]
fn class_info_parts_combine_in_any_order() {
    assert!(MetaPanel::TYPE.default_constructor().is_some());
    assert_eq!(MetaPanel::TYPE.interfaces().len(), 1);
    assert_eq!((MetaPanel::TYPE.interfaces()[0])(), ValueType::of::<Rc<dyn IMetaNamed>>());
    // The registered properties need no markup declaration.
    assert!(MetaPanel::TYPE.find_property("Spacing").is_some());
    assert!(MetaPanel::TYPE.markup().unwrap().find_property("Spacing").is_none());
}

#[test]
fn class_without_markup_declares_none_and_inherits_the_content_property() {
    assert!(MetaPlain::TYPE.markup().is_none());
    assert!(MetaPlain::TYPE.interfaces().is_empty());
    assert_eq!(MetaPlain::TYPE.content_property(), None);

    assert!(MetaDerivedPanel::TYPE.markup().is_none());
    assert_eq!(MetaDerivedPanel::TYPE.content_property(), Some("Children"));
    assert_eq!(MetaPanel::TYPE.content_property(), Some("Children"));
}

#[test]
fn class_members_are_invoked_on_handles_of_derived_classes() {
    let markup = MetaPanel::TYPE.markup().unwrap();
    let panel = MetaDerivedPanel::new();
    let instance = into_markup_value(panel.clone());

    let title = markup.find_property("Title").unwrap();
    (title.set.unwrap())(&[instance.clone(), boxed("t".to_string())]).unwrap();
    assert_eq!(panel.title(), "t");
    assert_eq!(unbox::<String>(&(title.get.unwrap())(&[instance.clone()]).unwrap()), "t");

    // A derived handle is accepted where the parameter is a base handle.
    let add = markup.find_methods("Add").next().unwrap();
    (add.invoke)(&[instance.clone(), into_markup_value(MetaPlain::new())]).unwrap();
    let count = markup.find_property("ChildCount").unwrap();
    assert_eq!(unbox::<i32>(&(count.get.unwrap())(&[instance.clone()]).unwrap()), 1);

    let on_click = markup.find_methods("OnClick").next().unwrap();
    assert_eq!(unbox::<i32>(&(on_click.invoke)(&[instance.clone(), boxed(2i32)]).unwrap()), 2);

    // An object of an unrelated class is rejected.
    let error = (on_click.invoke)(&[into_markup_value(MetaPlain::new()), boxed(2i32)]).unwrap_err();
    assert!(matches!(error, MarkupInvokeError::Argument { index: 0, .. }), "{error:?}");
}

/// An untyped call checks the instance against the run-time class of the object, as a
/// reflected call does: a handle of a base class whose object is of the class of the
/// member is accepted (a binding has the element it found as a handle of the root class).
#[test]
fn class_members_are_invoked_on_base_handles_of_objects_of_the_class() {
    let markup = MetaPanel::TYPE.markup().unwrap();
    let panel = MetaPanel::new();
    panel.set_title("t".to_string());
    let object: Ref<FerroObject> = FerroObject::to_ref(Upcast::<FerroObject>::upcast(&*panel));
    let instance: MarkupValue = Some(Rc::new(object));

    let title = markup.find_property("Title").unwrap();
    assert_eq!(unbox::<String>(&(title.get.unwrap())(&[instance.clone()]).unwrap()), "t");
    let on_click = markup.find_methods("OnClick").next().unwrap();
    assert_eq!(unbox::<i32>(&(on_click.invoke)(&[instance.clone(), boxed(2i32)]).unwrap()), 2);
    assert!(from_markup_value::<Option<Ref<MetaPanel>>>(&instance).is_some_and(|panel| panel.is_some()));

    // The object of a derived class through the root handle, for a member of the base class.
    let derived = MetaDerivedPanel::new();
    let object: Ref<FerroObject> = FerroObject::to_ref(Upcast::<FerroObject>::upcast(&*derived));
    (title.set.unwrap())(&[Some(Rc::new(object)), boxed("d".to_string())]).unwrap();
    assert_eq!(derived.title(), "d");

    // A base handle of an object of another class is still rejected.
    let plain = MetaPlain::new();
    let object: Ref<FerroObject> = FerroObject::to_ref(Upcast::<FerroObject>::upcast(&*plain));
    let error = (title.get.unwrap())(&[Some(Rc::new(object))]).unwrap_err();
    assert!(matches!(error, MarkupInvokeError::Argument { index: 0, .. }), "{error:?}");
}

#[test]
fn class_members_are_invoked_on_an_object_held_through_a_base_handle() {
    let markup = MetaPanel::TYPE.markup().unwrap();
    let panel = MetaDerivedPanel::new();
    // The receiver as a binding holds it: the root object handle.
    let instance: MarkupValue = Some(Rc::new(FerroObject::to_ref(&panel)) as BoxedValue);

    let title = markup.find_property("Title").unwrap();
    (title.set.unwrap())(&[instance.clone(), boxed("t".to_string())]).unwrap();
    assert_eq!(panel.title(), "t");
    assert_eq!(unbox::<String>(&(title.get.unwrap())(&[instance.clone()]).unwrap()), "t");

    let on_click = markup.find_methods("OnClick").next().unwrap();
    assert_eq!(unbox::<i32>(&(on_click.invoke)(&[instance.clone(), boxed(2i32)]).unwrap()), 2);

    // Indexers and events take the receiver the same way.
    let indexer = markup.find_indexer(1).unwrap();
    (indexer.set.unwrap())(&[instance.clone(), boxed(1i32), boxed(10i32)]).unwrap();
    assert_eq!(unbox::<i32>(&(indexer.get.unwrap())(&[instance.clone(), boxed(0i32)]).unwrap()), 9);
    let clicked = markup.find_event("Clicked").unwrap();
    (clicked.add)(&[instance, boxed(MarkupDelegate::new(|_| None))]).unwrap();
    assert_eq!(unbox::<i32>(&(indexer.get.unwrap())(&[Some(Rc::new(FerroObject::to_ref(&panel)) as BoxedValue), boxed(0i32)]).unwrap()), 109);

    // An object of an unrelated class held through the root handle is rejected.
    let plain = MetaPlain::new();
    let unrelated: MarkupValue = Some(Rc::new(FerroObject::to_ref(&plain)) as BoxedValue);
    let error = (title.get.unwrap())(&[unrelated]).unwrap_err();
    assert!(matches!(error, MarkupInvokeError::Argument { index: 0, .. }), "{error:?}");
}

#[test]
fn class_constructor_with_arguments_returns_the_handle() {
    let markup = MetaPanel::TYPE.markup().unwrap();
    let panel = (markup.constructors[0].invoke)(&[boxed("title".to_string())]).unwrap();
    assert_eq!(unbox::<Ref<MetaPanel>>(&panel).title(), "title");
    assert_eq!(unbox::<Ref<FerroObject>>(&panel).get_type(), MetaPanel::TYPE);
}

#[test]
fn classes_are_found_by_the_type_of_their_handles() {
    TypeInfo::register(MetaPanel::TYPE);
    assert_eq!(MetaPanel::TYPE.handle(), Some(std::any::TypeId::of::<Ref<MetaPanel>>()));
    assert_eq!(MetaPanel::TYPE.nullable_handle(), Some(std::any::TypeId::of::<Option<Ref<MetaPanel>>>()));

    let (type_, nullable) = TypeInfo::find_by_handle(std::any::TypeId::of::<Ref<MetaPanel>>()).unwrap();
    assert!(std::ptr::eq(type_, MetaPanel::TYPE) && !nullable);
    let (type_, nullable) = TypeInfo::find_by_handle(std::any::TypeId::of::<Option<Ref<MetaPanel>>>()).unwrap();
    assert!(std::ptr::eq(type_, MetaPanel::TYPE) && nullable);
    assert!(TypeInfo::find_by_handle(std::any::TypeId::of::<MetaPanel>()).is_none());

    crate::register_types();
    let (root, _) = TypeInfo::find_by_handle(std::any::TypeId::of::<Ref<FerroObject>>()).unwrap();
    assert!(std::ptr::eq(root, FerroObject::TYPE));
}

// Untyped values.

#[test]
fn values_convert_to_and_from_their_untyped_form() {
    assert_eq!(into_markup_value(()), None);
    assert_eq!(into_markup_value(None::<BoxedValue>), None);
    assert_eq!(into_markup_value(None::<Ref<FerroObject>>), None);

    let object = MetaPlain::new();
    let untyped = into_markup_value(Some(object.clone()));
    assert!(untyped.as_ref().unwrap().is::<Ref<MetaPlain>>());
    assert_eq!(unbox::<Ref<FerroObject>>(&untyped), object);
    assert_eq!(unbox::<Option<Ref<FerroObject>>>(&untyped), Some(object.clone().upcast()));
    assert_eq!(unbox::<Option<Ref<FerroObject>>>(&None), None);
    assert!(from_markup_value::<Ref<FerroObject>>(&None).is_none());

    // The "any value" types take the value as it is.
    let any = unbox::<BoxedValue>(&boxed(1i32));
    assert_eq!(any.downcast_ref::<i32>(), Some(&1));
    assert_eq!(unbox::<Option<BoxedValue>>(&None), None);
    assert!(from_markup_value::<BoxedValue>(&None).is_none());

    // A nullable value type takes the value and null.
    ValueTypes::register_nullable::<f64>();
    assert_eq!(unbox::<Option<f64>>(&boxed(1.5f64)), Some(1.5));
    assert_eq!(unbox::<Option<f64>>(&None), None);
    assert_eq!(into_markup_value(Some(2.5f64)).unwrap().downcast_ref::<f64>(), Some(&2.5));

    // Assignability only: no numeric conversion, no parsing.
    assert!(from_markup_value::<f64>(&boxed(1i32)).is_none());
    assert!(from_markup_value::<f64>(&boxed("1".to_string())).is_none());
}

// Service providers.

struct TestServices {
    name: Rc<str>,
}

impl IServiceProvider for TestServices {
    fn get_service(&self, service_type: std::any::TypeId) -> Option<Rc<dyn std::any::Any>> {
        service(service_type, || self.name.clone()).or_else(|| service(service_type, || 42i32))
    }
}

#[test]
fn service_provider_returns_services_by_handle_type() {
    let services = TestServices { name: Rc::from("root") };
    let provider: &dyn IServiceProvider = &services;
    assert_eq!(provider.get_service_of::<Rc<str>>().as_deref(), Some("root"));
    assert_eq!(provider.get_service_of::<i32>(), Some(42));
    assert_eq!(provider.get_service_of::<String>(), None);
    let empty: &dyn IServiceProvider = &EmptyServiceProvider;
    assert_eq!(empty.get_service_of::<i32>(), None);
}

#[test]
fn base_crate_registers_its_assembly() {
    crate::register_types();
    let assembly = MarkupAssembly::find("FerroUI.Base").unwrap();
    assert_eq!(assembly.crate_name, "ferroui_base");
    assert!(std::ptr::eq(MarkupAssembly::of_module(Visual::TYPE.module_path()).unwrap(), assembly));
    assert!(assembly
        .xmlns_definitions
        .iter()
        .all(|d| d.xml_namespace == FERRO_XML_NAMESPACE && d.namespace.starts_with("FerroUI")));
    assert!(assembly.xmlns_definitions.iter().any(|d| d.namespace == "FerroUI.Media"));
}

// Contract extensions: renamed enumeration members, attributes of registered
// properties, metadata of static types.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum MetaMode {
    DataContext,
    Self_,
}

ferro_markup_enum!(MetaMode { DataContext, Self = Self_ });

#[test]
fn enumeration_members_can_be_renamed() {
    let markup = <MetaMode as MarkupTyped>::MARKUP;
    let members: Vec<(&str, i64)> = markup.enum_members.iter().map(|m| (m.name, m.value)).collect();
    assert_eq!(members, [("DataContext", 0), ("Self", 1)]);
    let self_ = markup.find_enum_member("Self", false).unwrap();
    assert_eq!((self_.get)().downcast_ref::<MetaMode>(), Some(&MetaMode::Self_));
}

test_class!(MetaContent: FerroObject);

ferro_properties! {
    impl MetaContent {
        pub fn content_property() -> StyledProperty<i32> {
            FerroProperty::register::<MetaContent, _>("Content", 0)
        }
    }
}

ferro_class_info!(MetaContent {
    markup: {
        content: Content,
        property_attributes: [
            Content: [DependsOn("ContentTemplate"), ResolveByName],
        ],
    },
});

#[test]
fn registered_properties_take_attributes_without_being_declared() {
    let markup = MetaContent::TYPE.markup().unwrap();
    assert!(markup.properties.is_empty());
    let attributes = markup.find_property_attributes("Content");
    assert_eq!(attributes.len(), 2);
    assert_eq!(attributes[0].name, attributes::DEPENDS_ON);
    assert_eq!(attributes[0].arguments, [MarkupAttributeValue::Str("ContentTemplate")]);
    assert_eq!(attributes[1].name, attributes::RESOLVE_BY_NAME);
    assert!(markup.find_property_attributes("Other").is_empty());
}

pub struct MetaStatic;
ferro_static_type!(MetaStatic);

ferro_markup_type!(static MetaStatic {
    type_info: MetaStatic,
    methods: [static fn Twice(i32) -> i32 => |x: i32| x * 2],
    property_attributes: [Target: [ResolveByName]],
});

#[test]
fn static_types_link_their_metadata_to_the_runtime_type() {
    let markup = <MetaStatic as MarkupTyped>::MARKUP;
    assert_eq!(markup.kind, MarkupTypeKind::Static);
    assert!(std::ptr::eq((markup.type_info.unwrap())(), MetaStatic::TYPE));
    assert!(MarkupType::find_by_type_info(MetaStatic::TYPE).is_none());

    MarkupType::register(markup);
    assert!(std::ptr::eq(MarkupType::find_by_type_info(MetaStatic::TYPE).unwrap(), markup));
    // A class answers with its own metadata, registered or not.
    assert!(std::ptr::eq(MarkupType::find_by_type_info(MetaContent::TYPE).unwrap(), MetaContent::TYPE.markup().unwrap()));
    assert!(MarkupType::find_by_type_info(MetaPlain::TYPE).is_none());

    let twice = markup.find_methods("Twice").next().unwrap();
    assert_eq!(unbox::<i32>(&(twice.invoke)(&[boxed(4i32)]).unwrap()), 8);
    assert_eq!(markup.find_property_attributes("Target")[0].name, attributes::RESOLVE_BY_NAME);
}

#[test]
fn events_without_arguments_invoke_the_handler_with_none() {
    struct Ping {
        handlers: RefCell<Vec<MarkupDelegate>>,
    }
    impl PartialEq for Ping {
        fn eq(&self, other: &Self) -> bool {
            std::ptr::eq(self, other)
        }
    }
    ferro_markup_type!(class Ping {
        handles: [Rc<Ping>],
        this: Rc<Ping>,
        events: [
            Pinged() => |p: &Rc<Ping>, handler: MarkupDelegate| p.handlers.borrow_mut().push(handler),
            try Closed() => |_p: &Rc<Ping>, _handler: MarkupDelegate| Err::<(), _>("Already closed."),
        ],
    });

    let event = <Ping as MarkupTyped>::MARKUP.find_event("Pinged").unwrap();
    assert!(event.arguments.is_empty());
    let ping = Rc::new(Ping { handlers: RefCell::new(Vec::new()) });
    let count = Rc::new(Cell::new(0));
    let handler = MarkupDelegate::new({
        let count = count.clone();
        move |arguments| {
            assert!(arguments.is_empty());
            count.set(count.get() + 1);
            None
        }
    });
    (event.add)(&[boxed(ping.clone()), boxed(handler)]).unwrap();
    for handler in ping.handlers.borrow().iter() {
        handler.invoke(&[]);
    }
    assert_eq!(count.get(), 1);

    // A fallible subscription reports failure.
    let closed = <Ping as MarkupTyped>::MARKUP.find_event("Closed").unwrap();
    let handler = MarkupDelegate::new(|_| None);
    assert_eq!(
        (closed.add)(&[boxed(ping.clone()), boxed(handler)]),
        Err(MarkupInvokeError::Failed("Already closed.".to_string()))
    );
}

#[test]
fn enumerations_convert_numeric_values() {
    let from_value = <MetaDock as MarkupTyped>::MARKUP.enum_from_value.unwrap();
    assert_eq!(from_value(5).unwrap().downcast_ref::<MetaDock>(), Some(&MetaDock::Top));
    assert!(from_value(3).is_none());

    let from_value = <MetaMode as MarkupTyped>::MARKUP.enum_from_value.unwrap();
    assert_eq!(from_value(1).unwrap().downcast_ref::<MetaMode>(), Some(&MetaMode::Self_));

    let from_value = <MetaRoutes as MarkupTyped>::MARKUP.enum_from_value.unwrap();
    assert_eq!(from_value(0).unwrap().downcast_ref::<MetaRoutes>(), Some(&MetaRoutes::empty()));
    assert_eq!(
        from_value(5).unwrap().downcast_ref::<MetaRoutes>(),
        Some(&(MetaRoutes::DIRECT | MetaRoutes::BUBBLE))
    );
    assert!(from_value(8).is_none());
    assert!(<MetaPoint as MarkupTyped>::MARKUP.enum_from_value.is_none());
}

test_class!(MetaHost: FerroObject);
test_class!(MetaOwner: FerroObject);

ferro_properties! {
    impl MetaOwner {
        pub fn row_property() -> AttachedProperty<i32> {
            FerroProperty::register_attached::<MetaOwner, MetaHost, _>("Row", 0)
        }

        pub fn plain_property() -> StyledProperty<i32> {
            FerroProperty::register::<MetaOwner, _>("Plain", 0)
        }
    }
}

#[test]
fn attached_properties_state_their_host_type() {
    assert_eq!(MetaOwner::row_property().host_type(), Some(MetaHost::TYPE));
    assert_eq!(MetaOwner::plain_property().host_type(), None);
    assert_eq!(MetaOwner::TYPE.find_property("Row").unwrap().host_type(), Some(MetaHost::TYPE));
}

#[test]
fn view_models_expose_property_change_notifications() {
    use crate::data::model::{Event, INotifyPropertyChanged};

    struct MetaViewModel {
        changed: Event<str>,
    }
    impl PartialEq for MetaViewModel {
        fn eq(&self, other: &Self) -> bool {
            std::ptr::eq(self, other)
        }
    }
    impl INotifyPropertyChanged for MetaViewModel {
        fn property_changed(&self) -> &Event<str> {
            &self.changed
        }
    }
    ferro_markup_type!(class MetaViewModel {
        handles: [MetaViewModel],
        notify_property_changed: MetaViewModel,
    });

    let markup = <MetaViewModel as MarkupTyped>::MARKUP;
    let view = markup.notify_property_changed.unwrap();
    let model: BoxedValue = Rc::new(MetaViewModel { changed: Event::new() });
    let notifier = view(&*model).expect("the value is a view model");
    assert!(std::ptr::eq(notifier.property_changed(), &model.downcast_ref::<MetaViewModel>().unwrap().changed));
    let other: BoxedValue = Rc::new(1i32);
    assert!(view(&*other).is_none());
    assert!(<MetaPoint as MarkupTyped>::MARKUP.notify_property_changed.is_none());
}

// Fallible members.

#[derive(Clone, Copy, Debug, PartialEq)]
struct MetaRatio(f64);

impl MetaRatio {
    fn checked(value: f64) -> Result<Self, String> {
        if (0.0..=1.0).contains(&value) {
            Ok(Self(value))
        } else {
            Err(format!("{value} is not a ratio."))
        }
    }

    fn divided_by(&self, divisor: f64) -> Result<f64, &'static str> {
        if divisor == 0.0 {
            Err("Division by zero.")
        } else {
            Ok(self.0 / divisor)
        }
    }
}

ferro_markup_type!(struct MetaRatio {
    handles: [MetaRatio],
    constructors: [
        (f64) => MetaRatio,
        try (f64, bool) => |value: f64, _checked: bool| MetaRatio::checked(value),
    ],
    properties: [
        Inverse: f64 { try_get: |r: &MetaRatio| r.divided_by(r.0 * r.0) },
        Checked: f64 { get: |r: &MetaRatio| r.0, try_set: |_r: &MetaRatio, v: f64| MetaRatio::checked(v) },
    ],
    methods: [
        try fn DividedBy(f64) -> f64 => MetaRatio::divided_by,
        try fn Check(f64) => |_r: &MetaRatio, v: f64| MetaRatio::checked(v),
        static try fn Create(f64) -> MetaRatio => MetaRatio::checked,
        static try fn Validate(f64) => MetaRatio::checked,
        fn Value() -> f64 => |r: &MetaRatio| r.0,
    ],
});

#[test]
fn fallible_members_report_failure_instead_of_panicking() {
    let markup = <MetaRatio as MarkupTyped>::MARKUP;
    let failed = |message: &str| Err(MarkupInvokeError::Failed(message.to_string()));
    let half = boxed(MetaRatio(0.5));

    let constructor = markup.constructors[1].invoke;
    assert_eq!(unbox::<MetaRatio>(&constructor(&[boxed(0.25f64), boxed(true)]).unwrap()), MetaRatio(0.25));
    assert_eq!(constructor(&[boxed(2.0f64), boxed(true)]), failed("2 is not a ratio."));

    let method = |name: &str| markup.find_methods(name).next().unwrap().invoke;
    assert_eq!(unbox::<f64>(&method("DividedBy")(&[half.clone(), boxed(2.0f64)]).unwrap()), 0.25);
    assert_eq!(method("DividedBy")(&[half.clone(), boxed(0.0f64)]), failed("Division by zero."));
    assert_eq!(method("Check")(&[half.clone(), boxed(0.5f64)]), Ok(None));
    assert_eq!(method("Check")(&[half.clone(), boxed(5.0f64)]), failed("5 is not a ratio."));
    assert_eq!(unbox::<MetaRatio>(&method("Create")(&[boxed(1.0f64)]).unwrap()), MetaRatio(1.0));
    assert_eq!(method("Create")(&[boxed(-1.0f64)]), failed("-1 is not a ratio."));
    assert_eq!(method("Validate")(&[boxed(-1.0f64)]), failed("-1 is not a ratio."));
    assert_eq!(unbox::<f64>(&method("Value")(&[half.clone()]).unwrap()), 0.5);

    let inverse = markup.find_property("Inverse").unwrap();
    assert!(inverse.set.is_none());
    assert_eq!(unbox::<f64>(&(inverse.get.unwrap())(&[half.clone()]).unwrap()), 2.0);
    assert_eq!((inverse.get.unwrap())(&[boxed(MetaRatio(0.0))]), failed("Division by zero."));

    let checked = markup.find_property("Checked").unwrap();
    assert_eq!(unbox::<f64>(&(checked.get.unwrap())(&[half.clone()]).unwrap()), 0.5);
    assert_eq!((checked.set.unwrap())(&[half.clone(), boxed(0.1f64)]), Ok(None));
    assert_eq!((checked.set.unwrap())(&[half, boxed(7.0f64)]), failed("7 is not a ratio."));
}

#[test]
fn global_value_registrations_reach_every_thread() {
    #[derive(Clone, Copy, Debug, PartialEq)]
    struct MetaGlobal(i32);

    fn register() {
        ValueTypes::register_nullable::<MetaGlobal>();
    }

    let converts = || from_markup_value::<Option<MetaGlobal>>(&boxed(MetaGlobal(1))) == Some(Some(MetaGlobal(1)));
    // A thread that used the table before the registration...
    assert!(!converts());
    ValueTypes::register_global(register);
    ValueTypes::register_global(register);
    // ...sees it on its next use, and so does a thread that starts later.
    assert!(converts());
    assert!(std::thread::spawn(converts).join().unwrap());
}

#[test]
fn add_child_handles_made_from_one_object_are_equal() {
    use crate::styling::Style;

    let style = Style::new();
    let a: Rc<dyn IAddChild<BoxedValue>> = style.clone().upcast::<crate::styling::StyleBase>().into();
    let b: Rc<dyn IAddChild<BoxedValue>> = style.clone().upcast::<crate::styling::StyleBase>().into();
    let other: Rc<dyn IAddChild<BoxedValue>> = Style::new().upcast::<crate::styling::StyleBase>().into();
    assert!(*a == *b);
    assert!(*a != *other);
}

#[test]
fn binding_accessors_find_the_notifier_of_a_metadata_view_model() {
    use crate::data::core::plugins::InpcPropertyAccessor;
    use crate::data::model::{Event, INotifyPropertyChanged};

    struct MetaNotifying {
        changed: Event<str>,
    }
    impl PartialEq for MetaNotifying {
        fn eq(&self, other: &Self) -> bool {
            std::ptr::eq(self, other)
        }
    }
    impl INotifyPropertyChanged for MetaNotifying {
        fn property_changed(&self) -> &Event<str> {
            &self.changed
        }
    }
    ferro_markup_type!(class MetaNotifying {
        handles: [MetaNotifying],
        notify_property_changed: MetaNotifying,
    });

    // A plain `Rc` view model, known only through its markup metadata.
    let model: Rc<MetaNotifying> = Rc::new(MetaNotifying { changed: Event::new() });
    let boxed: BoxedValue = model.clone();
    assert!(InpcPropertyAccessor::find_notifier(&*boxed).is_none());

    MarkupType::register(<MetaNotifying as MarkupTyped>::MARKUP);
    let notifier = InpcPropertyAccessor::find_notifier(&*boxed).expect("the metadata declares the notifier");
    assert!(std::ptr::eq(notifier.property_changed(), &model.changed));
}

#[test]
fn reference_objects_cast_to_contract_handles_through_the_box() {
    trait IMetaContract {
        fn id(&self) -> i32;
    }
    impl PartialEq for dyn IMetaContract {
        fn eq(&self, other: &Self) -> bool {
            std::ptr::addr_eq(self, other)
        }
    }
    #[derive(PartialEq)]
    struct MetaReference(i32);
    impl IMetaContract for MetaReference {
        fn id(&self) -> i32 {
            self.0
        }
    }

    ValueTypes::register_reference::<MetaReference>();
    ValueTypes::register_boxed_cast::<MetaReference, Rc<dyn IMetaContract>>(|object| {
        let any: Rc<dyn std::any::Any> = object.clone();
        any.downcast::<MetaReference>().ok().map(|object| object as Rc<dyn IMetaContract>)
    });

    // The object form: the box is the object.
    let object: BoxedValue = Rc::new(MetaReference(7));
    let contract = from_markup_value::<Rc<dyn IMetaContract>>(&Some(object.clone())).expect("an assignability cast");
    assert_eq!(contract.id(), 7);
    assert!(ValueTypes::is_assignable(ValueType::of::<MetaReference>(), ValueType::of::<Rc<dyn IMetaContract>>()));
    // The typed handle still converts to and from the object form.
    assert_eq!(unbox::<Rc<MetaReference>>(&Some(object)).0, 7);
    assert!(into_markup_value(Rc::new(MetaReference(1))).unwrap().is::<MetaReference>());
}

#[test]
fn the_root_class_carries_its_markup_metadata() {
    crate::register_types();
    let markup = FerroObject::TYPE.markup().expect("declared in the annotations of this crate");
    assert_eq!(markup.name, "FerroObject");
    assert!(std::ptr::eq(MarkupType::find_by_type_info(FerroObject::TYPE).unwrap(), markup));
}

// Indexers, attributes of methods, floating point attribute arguments.

pub struct MetaTable {
    rows: RefCell<Vec<String>>,
    named: RefCell<Vec<(String, i32)>>,
}

impl PartialEq for MetaTable {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl MetaTable {
    fn row(&self, index: i32) -> Result<String, String> {
        usize::try_from(index)
            .ok()
            .and_then(|index| self.rows.borrow().get(index).cloned())
            .ok_or_else(|| format!("no row {index}"))
    }

    fn set_row(&self, index: i32, value: String) -> Result<(), String> {
        let index = usize::try_from(index).map_err(|_| format!("no row {index}"))?;
        match self.rows.borrow_mut().get_mut(index) {
            Some(row) => {
                *row = value;
                Ok(())
            }
            None => Err(format!("no row {index}")),
        }
    }

    fn named(&self, key: String) -> Option<i32> {
        self.named.borrow().iter().find(|(name, _)| *name == key).map(|(_, value)| *value)
    }

    fn cell(&self, row: i32, column: i32) -> i32 {
        row * 10 + column
    }

    fn set_cell(&self, row: i32, column: i32, value: i32) {
        self.named.borrow_mut().push((format!("{row},{column}"), value));
    }

    fn total(&self) -> i32 {
        self.rows.borrow().len() as i32
    }

    fn scale(&self, factor: f64) -> Result<f64, String> {
        if factor < 0.0 {
            return Err("negative".to_string());
        }
        Ok(factor * 2.0)
    }

    fn create(count: i32) -> Rc<MetaTable> {
        Rc::new(MetaTable {
            rows: RefCell::new((0..count).map(|i| format!("row {i}")).collect()),
            named: RefCell::new(vec![("a".to_string(), 1)]),
        })
    }

    fn parse_count(text: String) -> Result<i32, std::num::ParseIntError> {
        text.parse()
    }
}

ferro_markup_type!(class MetaTable {
    this: Rc<MetaTable>,
    handles: [MetaTable, Rc<MetaTable>, Option<Rc<MetaTable>>],
    indexers: [
        (i32) -> String { try_get: MetaTable::row, try_set: MetaTable::set_row } [Obsolete("rows")],
        (String) -> Option<i32> { get: MetaTable::named },
        (i32, i32) -> i32 { set: MetaTable::set_cell, get: MetaTable::cell },
    ],
    methods: [
        fn Total() -> i32 => MetaTable::total [DependsOn("Rows"), Unstable],
        fn Clear() => (|table: &Rc<MetaTable>| table.rows.borrow_mut().clear()) [Obsolete("use Reset", true)],
        try fn Scale(f64) -> f64 => MetaTable::scale [Range(0.5, -2.5, Step = 0.25)],
        try fn SetRow(i32, String) => MetaTable::set_row [DependsOn("Rows")],
        static fn Create(i32) -> Rc<MetaTable> => MetaTable::create [Factory],
        static fn Touch() => (|| ()) [Factory],
        static try fn ParseCount(String) -> i32 => MetaTable::parse_count [Factory("count")],
        static try fn Check(String) => (|text: String| MetaTable::parse_count(text).map(|_| ())) [Factory],
        fn Plain() -> i32 => MetaTable::total,
        fn PlainClosure(i32) -> i32 => |table: &Rc<MetaTable>, by: i32| table.total() * by,
    ],
    attributes: [Ratio(1.5, Minimum = -0.5)],
});

#[test]
fn indexers_are_declared_and_invoked_with_their_arguments() {
    ValueTypes::register_reference::<MetaTable>();
    let markup = <MetaTable as MarkupTyped>::MARKUP;
    let table = MetaTable::create(2);
    let this: MarkupValue = Some(table.clone());

    assert_eq!(markup.indexers.len(), 3);
    let rows = &markup.indexers[0];
    assert_eq!(rows.parameters.len(), 1);
    assert_eq!(rows.parameters[0](), ValueType::of::<i32>());
    assert_eq!((rows.type_)(), ValueType::of::<String>());
    assert_eq!(rows.attributes.len(), 1);
    assert_eq!(rows.attributes[0].name, attributes::OBSOLETE);
    assert_eq!(rows.attributes[0].arguments, [MarkupAttributeValue::Str("rows")]);

    let get = rows.get.expect("a getter");
    assert_eq!(unbox::<String>(&get(&[this.clone(), boxed(1)]).unwrap()), "row 1");
    // A failing accessor is reported, not raised.
    assert_eq!(get(&[this.clone(), boxed(5)]), Err(MarkupInvokeError::Failed("no row 5".to_string())));
    assert_eq!(get(&[this.clone()]), Err(MarkupInvokeError::ArgumentCount { expected: 2, actual: 1 }));
    assert!(matches!(
        get(&[this.clone(), boxed("x".to_string())]),
        Err(MarkupInvokeError::Argument { index: 1, .. })
    ));

    let set = rows.set.expect("a setter");
    assert_eq!(set(&[this.clone(), boxed(0), boxed("first".to_string())]), Ok(None));
    assert_eq!(table.rows.borrow()[0], "first");
    assert_eq!(
        set(&[this.clone(), boxed(7), boxed("x".to_string())]),
        Err(MarkupInvokeError::Failed("no row 7".to_string()))
    );
    assert_eq!(set(&[this.clone(), boxed(0)]), Err(MarkupInvokeError::ArgumentCount { expected: 3, actual: 2 }));

    // A read-only indexer keyed by text, with a nullable value.
    let named = &markup.indexers[1];
    assert!(named.set.is_none());
    assert!(named.attributes.is_empty());
    let get = named.get.expect("a getter");
    assert_eq!(unbox::<i32>(&get(&[this.clone(), boxed("a".to_string())]).unwrap()), 1);
    assert_eq!(get(&[this.clone(), boxed("missing".to_string())]), Ok(None));

    // Two index parameters; the accessors in either order.
    let cells = markup.find_indexer(2).expect("the indexer with two parameters");
    assert_eq!(unbox::<i32>(&(cells.get.unwrap())(&[this.clone(), boxed(3), boxed(4)]).unwrap()), 34);
    (cells.set.unwrap())(&[this.clone(), boxed(3), boxed(4), boxed(9)]).unwrap();
    assert_eq!(table.named("3,4".to_string()), Some(9));

    assert!(std::ptr::eq(markup.find_indexer(1).unwrap(), rows));
    assert!(markup.find_indexer(3).is_none());
    assert!(<MetaPoint as MarkupTyped>::MARKUP.indexers.is_empty());
}

#[test]
fn methods_carry_attributes_in_every_form() {
    ValueTypes::register_reference::<MetaTable>();
    let markup = <MetaTable as MarkupTyped>::MARKUP;
    let table = MetaTable::create(3);
    let this: MarkupValue = Some(table.clone());
    let method = |name: &str| markup.find_methods(name).next().unwrap_or_else(|| panic!("method {name}"));

    let total = method("Total");
    assert_eq!(total.attributes.len(), 2);
    assert_eq!(total.attributes[0].name, attributes::DEPENDS_ON);
    assert_eq!(total.attributes[0].arguments, [MarkupAttributeValue::Str("Rows")]);
    assert_eq!(total.attributes[1].name, attributes::UNSTABLE);
    assert!(!total.is_static);
    assert_eq!(total.return_type.map(|t| t()), Some(ValueType::of::<i32>()));
    assert_eq!(unbox::<i32>(&(total.invoke)(&[this.clone()]).unwrap()), 3);

    let scale = method("Scale");
    assert_eq!(scale.attributes[0].name, "Range");
    assert_eq!(scale.attributes[0].arguments, [MarkupAttributeValue::Float(0.5), MarkupAttributeValue::Float(-2.5)]);
    assert_eq!(scale.attributes[0].property("Step"), Some(MarkupAttributeValue::Float(0.25)));
    assert_eq!(unbox::<f64>(&(scale.invoke)(&[this.clone(), boxed(1.5f64)]).unwrap()), 3.0);
    assert_eq!((scale.invoke)(&[this.clone(), boxed(-1.0f64)]), Err(MarkupInvokeError::Failed("negative".to_string())));

    let set_row = method("SetRow");
    assert_eq!(set_row.attributes[0].arguments, [MarkupAttributeValue::Str("Rows")]);
    assert!(set_row.return_type.is_none());
    assert_eq!((set_row.invoke)(&[this.clone(), boxed(0), boxed("x".to_string())]), Ok(None));
    assert!((set_row.invoke)(&[this.clone(), boxed(9), boxed("x".to_string())]).is_err());

    let create = method("Create");
    assert!(create.is_static);
    assert_eq!(create.attributes[0].name, "Factory");
    let created = (create.invoke)(&[boxed(4)]).unwrap();
    assert_eq!(from_markup_value::<Rc<MetaTable>>(&created).unwrap().total(), 4);

    let touch = method("Touch");
    assert!(touch.is_static && touch.return_type.is_none());
    assert_eq!(touch.attributes.len(), 1);
    assert_eq!((touch.invoke)(&[]), Ok(None));

    let parse_count = method("ParseCount");
    assert_eq!(parse_count.attributes[0].arguments, [MarkupAttributeValue::Str("count")]);
    assert_eq!(unbox::<i32>(&(parse_count.invoke)(&[boxed("12".to_string())]).unwrap()), 12);
    assert!(matches!((parse_count.invoke)(&[boxed("x".to_string())]), Err(MarkupInvokeError::Failed(_))));

    let check = method("Check");
    assert!(check.is_static && check.return_type.is_none());
    assert_eq!(check.attributes.len(), 1);
    assert_eq!((check.invoke)(&[boxed("1".to_string())]), Ok(None));
    assert!((check.invoke)(&[boxed("x".to_string())]).is_err());

    // A parenthesised callable with attributes.
    let clear = method("Clear");
    assert_eq!(
        clear.attributes[0].arguments,
        [MarkupAttributeValue::Str("use Reset"), MarkupAttributeValue::Bool(true)]
    );

    // Methods without an attribute list are declared as before.
    assert!(method("Plain").attributes.is_empty());
    let plain_closure = method("PlainClosure");
    assert!(plain_closure.attributes.is_empty());
    assert_eq!(unbox::<i32>(&(plain_closure.invoke)(&[this.clone(), boxed(2)]).unwrap()), 6);

    (clear.invoke)(&[this.clone()]).unwrap();
    assert_eq!(table.total(), 0);
}

#[test]
fn attribute_arguments_can_be_floating_point_numbers() {
    let ratio = <MetaTable as MarkupTyped>::MARKUP.find_attribute("Ratio").expect("the attribute");
    assert_eq!(ratio.arguments, [MarkupAttributeValue::Float(1.5)]);
    assert_eq!(ratio.property("Minimum"), Some(MarkupAttributeValue::Float(-0.5)));
    assert_ne!(MarkupAttributeValue::Float(1.0), MarkupAttributeValue::Int(1));
    assert_eq!(format!("{:?}", MarkupAttributeValue::Float(1.5)), "1.5");
    assert_eq!(MarkupLiteral(2.5f64).value(), MarkupAttributeValue::Float(2.5));
    // Integer literals stay integers.
    assert_eq!(MarkupLiteral(2).value(), MarkupAttributeValue::Int(2));
}

#[test]
fn delegates_of_declared_methods_know_their_method_and_target() {
    ValueTypes::register_reference::<MetaTable>();
    let markup = <MetaTable as MarkupTyped>::MARKUP;
    let table = MetaTable::create(2);
    let total = markup.find_methods("Total").next().unwrap();
    let delegate = MarkupDelegate::for_method(Some(table.clone()), markup, total);

    assert!(std::ptr::eq(delegate.method().unwrap(), total));
    assert!(std::ptr::eq(delegate.declaring_type().unwrap(), markup));
    assert!(delegate.target().is_some());
    assert_eq!(unbox::<i32>(&delegate.invoke(&[])), 2);
    assert_eq!(delegate.to_string(), "System.Func`1[System.Int32]");
    assert_eq!(delegate.try_invoke(&[boxed(1)]), Err(MarkupInvokeError::ArgumentCount { expected: 1, actual: 2 }));
    assert_eq!(delegate.invoke(&[boxed(1)]), None);

    // A static method has no target.
    let create = markup.find_methods("Create").next().unwrap();
    let delegate = MarkupDelegate::for_method(Some(table.clone()), markup, create);
    assert!(delegate.target().is_none());
    assert!(delegate.invoke(&[boxed(1)]).is_some());

    // A callback knows neither.
    let callback = MarkupDelegate::new(|_| None);
    assert!(callback.method().is_none() && callback.target().is_none() && callback.declaring_type().is_none());
    assert_eq!(callback.try_invoke(&[]), Ok(None));
    assert_eq!(callback.to_string(), "System.Delegate");
}

pub struct MetaTableBase;

impl PartialEq for MetaTableBase {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

pub struct MetaTableDerived;

impl PartialEq for MetaTableDerived {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

ferro_markup_type!(class MetaTableBase { handles: [MetaTableBase, Rc<MetaTableBase>] });
ferro_markup_type!(class MetaTableDerived {
    handles: [MetaTableDerived, Rc<MetaTableDerived>],
    base: Rc<MetaTableBase>,
});

#[test]
fn base_types_are_walked_through_their_metadata() {
    MarkupType::register_all(&[<MetaTableBase as MarkupTyped>::MARKUP, <MetaTableDerived as MarkupTyped>::MARKUP]);
    let base = <MetaTableDerived as MarkupTyped>::MARKUP.base_type().expect("the base type");
    assert!(std::ptr::eq(base, <MetaTableBase as MarkupTyped>::MARKUP));
    assert!(base.base_type().is_none());

    // A class of the object model: the nearest base class with metadata.
    let derived = MetaDerivedPanel::TYPE.markup().or(MetaPanel::TYPE.markup()).expect("class metadata");
    let _ = derived.base_type();
    let panel = MetaPanel::TYPE.markup().expect("the metadata of the panel");
    let base = panel.base_type().expect("a base class with metadata");
    assert!(base.type_info.is_some_and(|type_info| type_info().is_assignable_from(MetaPanel::TYPE)));
}

// --- array attribute arguments, constructor parameters, static properties ------

thread_local! {
    static META_STATIC_LEVEL: std::cell::Cell<i32> = const { std::cell::Cell::new(3) };
}

#[derive(Clone, Debug, PartialEq)]
pub struct MetaAnnotated {
    path: String,
    count: i32,
}

impl MetaAnnotated {
    fn with_path(path: String, count: i32) -> Self {
        Self { path, count }
    }

    fn try_with_path(path: String) -> Result<Self, String> {
        if path.is_empty() {
            return Err("The path is empty.".to_string());
        }
        Ok(Self { path, count: 0 })
    }

    fn set_level(value: i32) -> Result<(), String> {
        if value < 0 {
            return Err("The level is negative.".to_string());
        }
        META_STATIC_LEVEL.with(|level| level.set(value));
        Ok(())
    }
}

ferro_markup_type!(struct MetaAnnotated {
    handles: [MetaAnnotated],
    constructors: [
        () => || MetaAnnotated::with_path(String::new(), 0),
        (i32) => |count: i32| MetaAnnotated::with_path(String::new(), count),
        (path: String [ConstructorArgument("Path"), InheritDataTypeFrom(2)], count: i32) => MetaAnnotated::with_path,
        try (path: String [ConstructorArgument("Path")]) => MetaAnnotated::try_with_path,
    ],
    fields: [Empty: MetaAnnotated => || MetaAnnotated::with_path(String::new(), 0)],
    static_properties: [
        Default: MetaAnnotated { get: || MetaAnnotated::with_path("default".to_string(), 1) } [Unstable],
        Level: i32 {
            get: || META_STATIC_LEVEL.with(|level| level.get()),
            try_set: MetaAnnotated::set_level
        },
    ],
    attributes: [
        FerroList(Separators = [",", " "]),
        Sample(["a", "b"], [1, [2.5, null], type(MetaAnnotated)], [], true, Names = ["x"], Empty = []),
    ],
});

#[test]
fn attribute_arguments_may_be_arrays() {
    use MarkupAttributeValue::{Array, Bool, Float, Int, Null, Str, Type};
    let markup = <MetaAnnotated as MarkupTyped>::MARKUP;

    let list = markup.find_attribute(attributes::FERRO_LIST).unwrap();
    assert!(list.arguments.is_empty());
    assert_eq!(list.property("Separators"), Some(Array(&[Str(","), Str(" ")])));

    let sample = markup.find_attribute("Sample").unwrap();
    assert_eq!(
        sample.arguments,
        [
            Array(&[Str("a"), Str("b")]),
            Array(&[Int(1), Array(&[Float(2.5), Null]), Type(|| ValueType::of::<MetaAnnotated>())]),
            Array(&[]),
            Bool(true),
        ]
    );
    assert_eq!(sample.properties.len(), 2);
    assert_eq!(sample.property("Names"), Some(Array(&[Str("x")])));
    assert_eq!(sample.property("Empty"), Some(Array(&[])));
    assert_ne!(Array(&[Str("a")]), Array(&[Str("b")]));
    assert_ne!(Array(&[Str("a")]), Str("a"));
    assert_eq!(format!("{:?}", Array(&[Str("a"), Int(1)])), r#"["a", 1]"#);
}

#[test]
fn constructor_parameters_carry_names_and_attributes() {
    let markup = <MetaAnnotated as MarkupTyped>::MARKUP;
    assert_eq!(markup.constructors.len(), 4);

    // The positional form states nothing about its parameters.
    assert!(markup.constructors[0].parameter_info.is_empty());
    let positional = &markup.constructors[1];
    assert_eq!(positional.parameters.len(), 1);
    assert!(positional.parameter_info.is_empty());
    assert_eq!(positional.parameter_name(0), None);
    assert!(positional.parameter_attributes(0).is_empty());

    let named = &markup.constructors[2];
    assert_eq!(named.parameters.len(), 2);
    assert_eq!((named.parameters[0])(), ValueType::of::<String>());
    assert_eq!((named.parameters[1])(), ValueType::of::<i32>());
    assert_eq!(named.parameter_info.len(), 2);
    assert_eq!(named.parameter_name(0), Some("path"));
    assert_eq!(named.parameter_name(1), Some("count"));
    assert_eq!(named.parameter_name(2), None);
    let path = named.parameter_attributes(0);
    assert_eq!(path.len(), 2);
    assert_eq!(path[0].name, attributes::CONSTRUCTOR_ARGUMENT);
    assert_eq!(path[0].arguments, [MarkupAttributeValue::Str("Path")]);
    assert_eq!(path[1].name, attributes::INHERIT_DATA_TYPE_FROM);
    assert_eq!(path[1].arguments, [MarkupAttributeValue::Int(2)]);
    assert!(named.parameter_attributes(1).is_empty());
    let built = (named.invoke)(&[into_markup_value("a".to_string()), into_markup_value(2i32)]).unwrap();
    assert_eq!(from_markup_value::<MetaAnnotated>(&built), Some(MetaAnnotated::with_path("a".to_string(), 2)));
    assert_eq!(
        (named.invoke)(&[into_markup_value("a".to_string())]).unwrap_err(),
        MarkupInvokeError::ArgumentCount { expected: 2, actual: 1 }
    );

    // The fallible form.
    let fallible = &markup.constructors[3];
    assert_eq!(fallible.parameter_name(0), Some("path"));
    assert_eq!(fallible.parameter_attributes(0).len(), 1);
    assert!((fallible.invoke)(&[into_markup_value("a".to_string())]).unwrap().is_some());
    assert_eq!(
        (fallible.invoke)(&[into_markup_value(String::new())]).unwrap_err(),
        MarkupInvokeError::Failed("The path is empty.".to_string())
    );
}

#[test]
fn static_properties_are_not_static_fields() {
    let markup = <MetaAnnotated as MarkupTyped>::MARKUP;
    assert_eq!(markup.fields.len(), 1);
    assert!(markup.find_field("Empty").is_some());
    assert!(markup.find_field("Default").is_none());
    assert!(markup.find_static_property("Empty").is_none());
    assert!(markup.find_property("Default").is_none());

    let default = markup.find_static_property("Default").expect("the static property");
    assert_eq!((default.type_)(), ValueType::of::<MetaAnnotated>());
    assert!(default.set.is_none());
    assert_eq!(default.attributes[0].name, attributes::UNSTABLE);
    let value = (default.get.unwrap())(&[]).unwrap();
    assert_eq!(from_markup_value::<MetaAnnotated>(&value), Some(MetaAnnotated::with_path("default".to_string(), 1)));
    // A static accessor takes no instance.
    assert_eq!(
        (default.get.unwrap())(&[None]).unwrap_err(),
        MarkupInvokeError::ArgumentCount { expected: 0, actual: 1 }
    );

    let level = markup.find_static_property("Level").expect("the static property");
    let (get, set) = (level.get.unwrap(), level.set.unwrap());
    assert_eq!(from_markup_value::<i32>(&get(&[]).unwrap()), Some(3));
    assert_eq!(set(&[into_markup_value(7i32)]), Ok(None));
    assert_eq!(from_markup_value::<i32>(&get(&[]).unwrap()), Some(7));
    assert_eq!(set(&[into_markup_value(-1i32)]), Err(MarkupInvokeError::Failed("The level is negative.".to_string())));
    assert!(matches!(set(&[into_markup_value("x".to_string())]), Err(MarkupInvokeError::Argument { index: 0, .. })));
    assert_eq!(from_markup_value::<i32>(&get(&[]).unwrap()), Some(7));
}
