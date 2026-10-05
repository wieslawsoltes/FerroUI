//! Tests specific to this port: the static initialisation of a class.
//!
//! A class is initialised once per thread: by its first instance, by the
//! first call of one of its property accessors, or by a lookup into it
//! (property registry, type metadata), whichever comes first. The
//! initialisation registers the handle casts of the class, its properties in
//! declaration order and runs its static constructor, base classes first.
//!
//! Every test runs on its own thread and so starts with nothing initialised.
//! The class names are unique in the crate: the table of known types is
//! process-wide.

use super::*;
use crate::data::core::{ValueType, ValueTypes};
use crate::*;
use std::any::TypeId;

thread_local! {
    /// The static constructors that ran on the thread, in order.
    static LOG: RefCell<Vec<&'static str>> = const { RefCell::new(Vec::new()) };
}

fn log(entry: &'static str) {
    LOG.with(|log| log.borrow_mut().push(entry));
}

fn logged() -> Vec<&'static str> {
    LOG.with(|log| log.borrow().clone())
}

fn names(properties: &[&'static FerroProperty]) -> Vec<String> {
    properties.iter().map(|p| p.name().to_string()).collect()
}

// A class in the new form: properties block, static constructor, class info.

test_class!(RegBase: FerroObject);
ferro_class_info!(RegBase { new: RegBase::new });

ferro_properties! {
    impl RegBase {
        /// Defines the `First` property.
        pub fn first_property() -> StyledProperty<i32> {
            FerroProperty::register::<RegBase, _>("First", 1)
        }

        pub fn second_property() -> StyledProperty<String> {
            FerroProperty::register::<RegBase, _>("Second", s("second"))
        }

        pub fn third_property() -> DirectProperty<RegBase, i32> {
            FerroProperty::register_direct::<RegBase, _>("Third", |_| 3, None, 0)
        }

        pub fn fourth_property() -> AttachedProperty<bool> {
            FerroProperty::register_attached::<RegBase, FerroObject, _>("Fourth", false)
        }
    }
}

impl RegBase {
    fn static_constructor() {
        log("RegBase");
        // The properties are registered before the static constructor runs.
        assert!(FerroPropertyRegistry::instance().find_registered(RegBase::TYPE, "Fourth").is_some());
    }
}

// A derived class whose existing `ferro_property!` invocations are wrapped in
// the block unchanged.

test_class!(RegDerived: RegBase);

ferro_properties! { impl RegDerived {
    ferro_property!(pub fn fifth_property() -> StyledProperty<f64> {
        FerroProperty::register::<RegDerived, _>("Fifth", 5.0)
    });

    ferro_property!(
        /// Defines the `First` property on this class as well.
        pub fn sixth_property() -> StyledProperty<Option<Ref<RegBase>>> {
            FerroProperty::register::<RegDerived, _>("Sixth", None)
        }
    );
} }

impl RegDerived {
    fn static_constructor() {
        log("RegDerived");
    }
}

// A class that declares nothing, deriving from one that does.
test_class!(RegLeaf: RegDerived);

#[test]
fn properties_are_registered_in_declaration_order() {
    let registry = FerroPropertyRegistry::instance();

    let declared = registry.get_declared(RegBase::TYPE);

    assert_eq!(names(&declared), ["First", "Second", "Third", "Fourth"]);
    assert!(declared.windows(2).all(|w| w[0].id() < w[1].id()));
}

#[test]
fn calling_an_accessor_first_registers_the_whole_class_in_declaration_order() {
    // The last property is asked for first: reading a static field of a
    // class runs its whole static initialisation.
    let fourth = RegBase::fourth_property();

    assert_eq!(logged(), ["RegBase"]);
    let declared = FerroPropertyRegistry::instance().get_declared(RegBase::TYPE);
    assert_eq!(names(&declared), ["First", "Second", "Third", "Fourth"]);
    assert_eq!(declared[3].id(), fourth.id());
    assert!(std::ptr::eq(RegBase::fourth_property(), fourth));
}

#[test]
fn properties_are_found_by_name_before_any_instance_exists() {
    let registry = FerroPropertyRegistry::instance();

    // No instance, no accessor call: the lookup initialises the class.
    let fifth = registry.find_registered(RegDerived::TYPE, "Fifth").expect("own property");
    let second = registry.find_registered(RegDerived::TYPE, "Second").expect("base class property");

    assert_eq!(fifth.id(), RegDerived::fifth_property().id());
    assert_eq!(second.id(), RegBase::second_property().id());
    assert!(registry.find_registered(RegBase::TYPE, "Fifth").is_none());
}

#[test]
fn a_class_without_declarations_lists_the_properties_of_its_bases() {
    let properties = RegLeaf::TYPE.properties();

    assert_eq!(names(&properties), ["Fifth", "Sixth", "First", "Second", "Third", "Fourth"]);
    assert!(RegLeaf::TYPE.find_property("Third").is_some());
    assert!(RegLeaf::TYPE.find_property("Missing").is_none());
}

#[test]
fn direct_and_attached_properties_are_listed_before_any_instance_exists() {
    let registry = FerroPropertyRegistry::instance();

    let direct = registry.get_registered_direct(RegDerived::TYPE);
    assert_eq!(names(&direct), ["Third"]);

    // The owner is initialised by the lookup above, which registers its
    // attached property on the host type.
    let attached = registry.get_registered_attached(FerroObject::TYPE);
    assert!(attached.iter().any(|p| p.name() == "Fourth"));
}

#[test]
fn static_constructors_run_once_per_thread_base_class_first() {
    assert!(logged().is_empty());

    let _a = RegLeaf::new();
    let _b = RegLeaf::new();
    let _c = RegDerived::new();
    let _d = RegBase::new();
    RegBase::first_property();
    RegDerived::fifth_property();
    RegLeaf::TYPE.ensure_class_init();
    FerroPropertyRegistry::instance().get_registered(RegLeaf::TYPE);

    assert_eq!(logged(), ["RegBase", "RegDerived"]);
}

#[test]
fn static_constructors_run_again_on_another_thread() {
    let _a = RegDerived::new();
    assert_eq!(logged(), ["RegBase", "RegDerived"]);

    let other = std::thread::spawn(|| {
        let before = logged();
        let _a = RegDerived::new();
        let _b = RegDerived::new();
        (before, logged())
    })
    .join()
    .unwrap();

    assert!(other.0.is_empty());
    assert_eq!(other.1, ["RegBase", "RegDerived"]);
    assert_eq!(logged(), ["RegBase", "RegDerived"]);
}

#[test]
fn a_lookup_runs_the_static_constructor_of_a_class_that_has_no_instance() {
    assert!(RegDerived::TYPE.find_property("Sixth").is_some());

    assert_eq!(logged(), ["RegBase", "RegDerived"]);
}

// Handle casts.

#[test]
fn derived_handles_convert_to_base_handles_without_explicit_registration() {
    let object = RegLeaf::new();
    let boxed: BoxedValue = Rc::new(object.clone());

    let base = ValueTypes::try_convert(Some(&boxed), ValueType::of::<Ref<RegBase>>()).flatten().expect("base handle");
    assert!(base.downcast_ref::<Ref<RegBase>>().unwrap().ptr_eq(&object));

    let nullable =
        ValueTypes::try_convert(Some(&boxed), ValueType::of::<Option<Ref<RegDerived>>>()).flatten().expect("nullable");
    assert!(nullable.downcast_ref::<Option<Ref<RegDerived>>>().unwrap().as_ref().unwrap().ptr_eq(&object));

    assert!(ValueTypes::is_assignable(ValueType::of::<Ref<RegLeaf>>(), ValueType::of::<Ref<RegBase>>()));
    assert!(!ValueTypes::is_assignable(ValueType::of::<Ref<RegBase>>(), ValueType::of::<Ref<RegLeaf>>()));
    let cast = ValueTypes::try_cast(&boxed, ValueType::of::<Ref<RegDerived>>()).expect("cast");
    assert!(cast.downcast_ref::<Ref<RegDerived>>().unwrap().ptr_eq(&object));

    // The handle is recognised as an object, and null converts to its
    // nullable form.
    assert!(ValueTypes::as_object(&*boxed).is_some());
    assert!(ValueTypes::try_convert(None, ValueType::of::<Option<Ref<RegLeaf>>>()).is_some());
}

#[test]
fn a_base_handle_converts_down_only_when_the_object_is_of_the_class() {
    let leaf = RegLeaf::new();
    let base = RegBase::new();
    let as_base: BoxedValue = Rc::new(leaf.clone().upcast::<RegBase>());
    let plain: BoxedValue = Rc::new(base);

    assert!(ValueTypes::try_convert(Some(&as_base), ValueType::of::<Ref<RegLeaf>>()).is_some());
    assert!(ValueTypes::try_convert(Some(&plain), ValueType::of::<Ref<RegLeaf>>()).is_none());
}

// Interface handles.

pub trait IRegNamed {
    fn reg_name(&self) -> String;
    fn identity(&self) -> *const ();
}

/// The interface handle: an `Rc<dyn Trait>` with reference equality.
#[derive(Clone)]
pub struct RegNamedHandle(Rc<dyn IRegNamed>);

impl PartialEq for RegNamedHandle {
    fn eq(&self, other: &Self) -> bool {
        self.0.identity() == other.0.identity()
    }
}

struct RegAdapter(Ref<RegShape>, &'static str);

impl IRegNamed for RegAdapter {
    fn reg_name(&self) -> String {
        format!("{} via {}", self.0.get_type().name(), self.1)
    }

    fn identity(&self) -> *const () {
        let object: &FerroObject = &self.0;
        object as *const FerroObject as *const ()
    }
}

test_class!(RegShape: FerroObject);
ferro_class_info!(RegShape { new: RegShape::new, interfaces: [RegNamedHandle] });

impl<T: ObjectType + Upcast<RegShape>> From<Ref<T>> for RegNamedHandle {
    fn from(value: Ref<T>) -> Self {
        RegNamedHandle(Rc::new(RegAdapter(value.upcast(), "RegShape")))
    }
}

test_class!(RegCircle: RegShape);

// A more derived class with an interface of its own.
test_class!(RegSquare: RegShape);
ferro_class_info!(RegSquare { interfaces: [RegSquareHandle] });

#[derive(Clone)]
pub struct RegSquareHandle(Ref<RegSquare>);

impl PartialEq for RegSquareHandle {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl From<Ref<RegSquare>> for RegSquareHandle {
    fn from(value: Ref<RegSquare>) -> Self {
        RegSquareHandle(value)
    }
}

#[test]
fn handles_convert_to_the_interfaces_their_class_declares() {
    let circle = RegCircle::new();
    let boxed: BoxedValue = Rc::new(circle.clone());
    let target = ValueType::of::<RegNamedHandle>();

    let converted = ValueTypes::try_convert(Some(&boxed), target).flatten().expect("interface handle");
    assert_eq!(converted.downcast_ref::<RegNamedHandle>().unwrap().0.reg_name(), "RegCircle via RegShape");

    let nullable = ValueTypes::try_convert(Some(&boxed), ValueType::of::<Option<RegNamedHandle>>())
        .flatten()
        .expect("nullable interface handle");
    assert!(nullable.downcast_ref::<Option<RegNamedHandle>>().unwrap().is_some());

    assert!(ValueTypes::is_assignable(ValueType::of::<Ref<RegCircle>>(), target));
    assert!(ValueTypes::is_assignable(ValueType::of::<Ref<RegCircle>>(), ValueType::of::<Option<RegNamedHandle>>()));
    assert!(ValueTypes::try_cast(&boxed, target).is_some());
    assert!(ValueTypes::try_cast(&boxed, ValueType::of::<Option<RegNamedHandle>>()).is_some());

    // The root handle and a base handle of the same object convert too.
    let object: &FerroObject = &circle;
    let root: BoxedValue = Rc::new(object.to_ref());
    assert!(ValueTypes::try_convert(Some(&root), target).is_some());
    let shape: BoxedValue = Rc::new(circle.clone().upcast::<RegShape>());
    assert!(ValueTypes::try_convert(Some(&shape), target).is_some());
}

#[test]
fn the_most_derived_declaration_of_an_interface_converts() {
    let square = RegSquare::new();
    let _circle = RegCircle::new();
    // The derived class registers the interface of its base class again,
    // with a conversion of its own.
    ValueTypes::register_interface::<RegSquare, RegNamedHandle>(|square| {
        RegNamedHandle(Rc::new(RegAdapter(square.upcast(), "RegSquare")))
    });
    let circle: BoxedValue = Rc::new(_circle.clone());
    let converted = ValueTypes::try_convert(Some(&circle), ValueType::of::<RegNamedHandle>()).flatten().unwrap();
    assert_eq!(converted.downcast_ref::<RegNamedHandle>().unwrap().0.reg_name(), "RegCircle via RegShape");
    // Held as a base class handle: the conversion still is the one of the
    // class of the object.
    let boxed: BoxedValue = Rc::new(square.clone().upcast::<RegShape>());

    let converted = ValueTypes::try_convert(Some(&boxed), ValueType::of::<RegNamedHandle>()).flatten().unwrap();
    assert_eq!(converted.downcast_ref::<RegNamedHandle>().unwrap().0.reg_name(), "RegSquare via RegSquare");

    let own = ValueTypes::try_convert(Some(&boxed), ValueType::of::<RegSquareHandle>()).flatten().unwrap();
    assert!(own.downcast_ref::<RegSquareHandle>().unwrap().0.ptr_eq(&square));
}

#[test]
fn objects_do_not_convert_to_interfaces_their_class_does_not_declare() {
    let base = RegBase::new();
    let circle = RegCircle::new();
    let _square = RegSquare::new();
    let base: BoxedValue = Rc::new(base);
    let circle: BoxedValue = Rc::new(circle);

    assert!(ValueTypes::try_convert(Some(&base), ValueType::of::<RegNamedHandle>()).is_none());
    assert!(ValueTypes::try_convert(Some(&circle), ValueType::of::<RegSquareHandle>()).is_none());
    assert!(!ValueTypes::is_assignable(ValueType::of::<Ref<RegBase>>(), ValueType::of::<RegNamedHandle>()));
    assert!(ValueTypes::try_cast(&circle, ValueType::of::<RegSquareHandle>()).is_none());
}

// Non-instantiable owners of attached properties.

pub struct RegStatics;
ferro_static_type!(RegStatics);

ferro_properties! {
    impl RegStatics {
        pub fn dock_property() -> AttachedProperty<i32> {
            FerroProperty::register_attached::<RegStatics, RegBase, _>("Dock", 0)
        }

        pub fn row_property() -> AttachedProperty<i32> {
            FerroProperty::register_attached::<RegStatics, RegBase, _>("Row", 0)
        }
    }
}

impl RegStatics {
    fn static_constructor() {
        log("RegStatics");
    }
}

#[test]
fn attached_properties_of_a_static_type_are_found_by_owner_and_name() {
    let registry = FerroPropertyRegistry::instance();

    let row = registry.find_registered(RegStatics::TYPE, "Row").expect("found by owner type");

    assert_eq!(names(&registry.get_declared(RegStatics::TYPE)), ["Dock", "Row"]);
    assert_eq!(row.id(), RegStatics::row_property().id());
    assert!(registry.get_registered_attached(RegLeaf::TYPE).iter().any(|p| p.name() == "Dock"));
    assert_eq!(logged(), ["RegStatics", "RegBase", "RegDerived"]);
}

// Properties declared in several places.

test_class!(RegParts: FerroObject);

ferro_properties! {
    impl RegParts, also [RegParts::register_more_properties] {
        pub fn one_property() -> StyledProperty<i32> {
            FerroProperty::register::<RegParts, _>("One", 1)
        }
    }
}

ferro_properties! {
    impl RegParts, fn register_more_properties {
        pub fn two_property() -> StyledProperty<i32> {
            FerroProperty::register::<RegParts, _>("Two", 2)
        }

        pub fn three_property() -> StyledProperty<i32> {
            FerroProperty::register::<RegParts, _>("Three", 3)
        }
    }
}

#[test]
fn properties_declared_in_parts_are_registered_with_the_class() {
    // An accessor of a part initialises the class like any other.
    let three = RegParts::three_property();

    let declared = FerroPropertyRegistry::instance().get_declared(RegParts::TYPE);
    assert_eq!(names(&declared), ["One", "Two", "Three"]);
    assert_eq!(declared[2].id(), three.id());
}

// The old form keeps working: lone accessors and a hand-written, flag-guarded
// `class_init` called from `constructed`.

#[repr(C)]
pub struct RegOld {
    base: FerroObject,
}

ferro_class!(RegOld: FerroObject);

impl FerroObjectImpl for RegOld {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);
        Self::class_init();
    }
}

impl RegOld {
    ferro_property!(pub fn foo_property() -> StyledProperty<i32> {
        FerroProperty::register::<RegOld, _>("Foo", 7)
    });

    ferro_property!(pub fn bar_property() -> StyledProperty<i32> {
        FerroProperty::register::<RegOld, _>("Bar", 8)
    });

    fn class_init() {
        once_per_thread!({
            log("RegOld");
            Self::bar_property();
        });
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self { base: FerroObject::construct() })
    }
}

#[test]
fn old_form_classes_keep_registering_lazily() {
    let registry = FerroPropertyRegistry::instance();
    assert!(registry.find_registered(RegOld::TYPE, "Foo").is_none());
    assert!(logged().is_empty());

    let target = RegOld::new();
    let _again = RegOld::new();

    // The hand-written class initialisation ran once and registered what it
    // names; the other accessor registers on its first call.
    assert_eq!(logged(), ["RegOld"]);
    assert!(registry.find_registered(RegOld::TYPE, "Bar").is_some());
    assert!(registry.find_registered(RegOld::TYPE, "Foo").is_none());
    assert_eq!(target.get_value(RegOld::foo_property()), 7);
    assert!(registry.find_registered(RegOld::TYPE, "Foo").is_some());
}

#[test]
fn old_form_classes_get_their_handle_casts_and_are_known_by_name() {
    let target = RegOld::new();
    let boxed: BoxedValue = Rc::new(target.clone());

    let root = ValueTypes::try_convert(Some(&boxed), ValueType::of::<Option<Ref<FerroObject>>>()).flatten().unwrap();
    assert!(root.downcast_ref::<Option<Ref<FerroObject>>>().unwrap().is_some());
    assert!(std::ptr::eq(TypeInfo::find_by_name("RegOld").unwrap(), RegOld::TYPE));
    assert!(RegOld::TYPE.default_constructor().is_none());
}

// Type metadata.

#[test]
fn type_metadata_is_data_of_the_runtime_type() {
    assert_eq!(RegBase::TYPE.name(), "RegBase");
    assert_eq!(RegBase::TYPE.module_path(), module_path!());
    assert!(std::ptr::eq(RegDerived::TYPE.base_type().unwrap(), RegBase::TYPE));
    assert_eq!(RegStatics::TYPE.module_path(), module_path!());
    assert!(RegStatics::TYPE.base_type().is_none());
}

#[test]
fn namespaces_are_declared_per_module_and_types_are_found_by_namespace_and_name() {
    TypeInfo::register_namespaces(&[(module_path!(), "Tests.Registration")]);
    TypeInfo::register_all(&[RegBase::TYPE, RegStatics::TYPE]);

    assert_eq!(RegBase::TYPE.namespace(), "Tests.Registration");
    assert_eq!(RegBase::TYPE.full_name(), "Tests.Registration.RegBase");
    assert!(std::ptr::eq(TypeInfo::find("Tests.Registration", "RegBase").unwrap(), RegBase::TYPE));
    assert!(std::ptr::eq(TypeInfo::find("Tests.Registration", "RegStatics").unwrap(), RegStatics::TYPE));
    assert!(std::ptr::eq(TypeInfo::find_by_name("RegBase").unwrap(), RegBase::TYPE));
    assert!(TypeInfo::find("Tests.Other", "RegBase").is_none());
    assert!(TypeInfo::find("Tests.Registration", "RegMissing").is_none());

    // Nothing was initialised by registering or finding the types.
    assert!(logged().is_empty());
}

#[test]
fn a_type_becomes_known_by_name_when_it_is_initialised() {
    let _target = RegLeaf::new();

    assert!(std::ptr::eq(TypeInfo::find_by_name("RegLeaf").unwrap(), RegLeaf::TYPE));
    assert!(std::ptr::eq(TypeInfo::find_by_name("RegDerived").unwrap(), RegDerived::TYPE));
    assert!(TypeInfo::registered_types().iter().any(|t| std::ptr::eq(*t, RegLeaf::TYPE)));
}

#[test]
fn the_default_constructor_creates_an_instance_of_the_class() {
    let constructor = RegShape::TYPE.default_constructor().expect("declared constructor");

    let a = constructor();
    let b = RegShape::TYPE.create_instance().unwrap();

    assert!(std::ptr::eq(a.get_type(), RegShape::TYPE));
    assert!(a.cast::<RegShape>().is_some());
    assert!(!a.ptr_eq(&b));
    // Not inherited, and absent unless declared.
    assert!(RegCircle::TYPE.default_constructor().is_none());
    assert!(RegCircle::TYPE.create_instance().is_none());
    assert!(RegStatics::TYPE.default_constructor().is_none());
}

#[test]
fn a_type_found_by_name_can_be_inspected_and_instantiated_without_naming_the_class() {
    TypeInfo::register_namespaces(&[(module_path!(), "Tests.Registration")]);
    TypeInfo::register_all(&[RegBase::TYPE]);

    let type_ = TypeInfo::find("Tests.Registration", "RegBase").unwrap();
    let property = type_.find_property("First").expect("property by name");
    let object = type_.create_instance().expect("instance");

    assert_eq!(property.property_type(), TypeId::of::<i32>());
    assert_eq!(object.get_value_untyped(property).downcast_ref::<i32>(), Some(&1));
}

// The crate's own type table.

#[test]
fn the_types_of_this_crate_are_found_by_namespace_and_name() {
    crate::register_types();

    let find = |namespace, name| TypeInfo::find(namespace, name).unwrap_or_else(|| panic!("{namespace}.{name}"));
    assert!(std::ptr::eq(find("FerroUI", "Visual"), Visual::TYPE));
    assert!(std::ptr::eq(find("FerroUI", "FerroObject"), FerroObject::TYPE));
    assert!(std::ptr::eq(find("FerroUI.Media", "SolidColorBrush"), media::SolidColorBrush::TYPE));
    assert!(std::ptr::eq(find("FerroUI.Media", "CombinedGeometry"), media::CombinedGeometry::TYPE));
    assert!(std::ptr::eq(find("FerroUI.Media", "DropShadowEffect"), media::effects::DropShadowEffect::TYPE));
    assert!(std::ptr::eq(find("FerroUI.Layout", "Layoutable"), layout::Layoutable::TYPE));
    assert!(std::ptr::eq(find("FerroUI.Animation", "DoubleTransition"), animation::DoubleTransition::TYPE));
    assert_eq!(Visual::TYPE.full_name(), "FerroUI.Visual");
    assert_eq!(input::InputElement::TYPE.namespace(), "FerroUI.Input");
}

#[test]
fn every_type_of_this_crate_initialises_and_has_a_namespace() {
    crate::register_types();

    for type_ in TypeInfo::registered_types() {
        if !type_.module_path().starts_with("ferroui_base::") || type_.module_path().contains("test") {
            continue;
        }
        assert!(type_.namespace().starts_with("FerroUI"), "{type_} has no namespace");
        // Runs the static initialisation of every class on one thread.
        let properties = type_.properties();
        for property in properties.iter() {
            assert!(type_.find_property(property.name()).is_some(), "{type_}.{property}");
        }
    }
}

/// Guards the list in `register_types.rs` against the sources: every class
/// and static type declared outside of test code is in it.
#[test]
fn register_types_lists_every_type_declared_in_the_sources() {
    fn visit(dir: &std::path::Path, out: &mut Vec<(String, String)>) {
        for entry in std::fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            let name = path.file_name().unwrap().to_string_lossy().to_string();
            if path.is_dir() {
                if !matches!(name.as_str(), "tests" | "testing" | "target") {
                    visit(&path, out);
                }
                continue;
            }
            if !name.ends_with(".rs") || name.ends_with("_tests.rs") || name.contains("test_support") {
                continue;
            }
            if matches!(name.as_str(), "type_system.rs" | "ferro_property.rs") {
                continue;
            }
            let source = std::fs::read_to_string(&path).unwrap();
            // Test modules at the end of a file declare test classes.
            let source = source.split("#[cfg(test)]").next().unwrap();
            for marker in ["ferro_class!", "ferro_static_type!(", "StaticType for ", "ferro_transition_class!("] {
                for (at, _) in source.match_indices(marker) {
                    let line_start = source[..at].rfind('\n').map_or(0, |i| i + 1);
                    let before = source[line_start..at].trim();
                    if before.starts_with("//") || before.contains('$') {
                        continue;
                    }
                    let rest = &source[at + marker.len()..];
                    // The class name: the first identifier that is followed
                    // by `:` (or ends the declaration), after any doc lines.
                    let mut found = None;
                    for line in rest.lines().take(8) {
                        let line = line.trim().trim_start_matches(['(', '{']).trim();
                        if line.is_empty() || line.starts_with("//") {
                            continue;
                        }
                        let ident: String = line.chars().take_while(|c| c.is_alphanumeric() || *c == '_').collect();
                        if !ident.is_empty() && !ident.starts_with('$') {
                            found = Some(ident);
                        }
                        break;
                    }
                    if let Some(found) = found {
                        out.push((found, path.display().to_string()));
                    }
                }
            }
        }
    }

    crate::register_types();
    let mut declared = Vec::new();
    visit(std::path::Path::new(env!("CARGO_MANIFEST_DIR")), &mut declared);
    assert!(declared.len() > 50, "the scan found only {} declarations", declared.len());

    let known = TypeInfo::registered_types();
    let missing: Vec<_> = declared
        .iter()
        .filter(|(name, _)| !known.iter().any(|t| t.name() == name))
        .collect();
    assert!(missing.is_empty(), "types missing from `register_types.rs`: {missing:?}");
}

// The "assign binding" flag of a property definition.

test_class!(RegBindings: FerroObject);

ferro_properties! {
    impl RegBindings {
        pub fn plain_property() -> StyledProperty<i32> {
            FerroProperty::register::<RegBindings, _>("Plain", 0)
        }

        pub fn member_binding_property() -> StyledProperty<Option<i32>> {
            FerroProperty::register_with::<RegBindings, _>(
                "MemberBinding",
                StyledPropertyOptions::new(None).assign_binding(true),
            )
        }

        pub fn attached_binding_property() -> AttachedProperty<Option<i32>> {
            FerroProperty::register_attached_with::<RegBindings, FerroObject, _>(
                "AttachedBinding",
                StyledPropertyOptions::new(None).assign_binding(true),
            )
        }

        pub fn direct_binding_property() -> DirectProperty<RegBindings, i32> {
            let property = FerroProperty::register_direct::<RegBindings, _>("DirectBinding", |_| 0, None, 0);
            property.set_assign_binding(true);
            property
        }
    }
}

test_class!(RegBindingsOwner: FerroObject);

#[test]
fn assign_binding_is_a_flag_of_the_property_definition() {
    assert!(!RegBindings::plain_property().assign_binding());
    assert!(RegBindings::member_binding_property().assign_binding());
    assert!(RegBindings::attached_binding_property().assign_binding());
    assert!(RegBindings::direct_binding_property().assign_binding());

    // Found by name, and kept by an added owner.
    assert!(RegBindings::TYPE.find_property("MemberBinding").unwrap().assign_binding());
    assert!(RegBindings::member_binding_property().add_owner::<RegBindingsOwner>().assign_binding());
}
