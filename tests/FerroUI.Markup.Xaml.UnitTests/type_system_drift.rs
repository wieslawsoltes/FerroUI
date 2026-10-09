//! The drift test of the build-time type system (docs/porting/xaml.md, 9.5.5
//! and 9.10.1): the type system over the scanned models of the base and the
//! controls crates (`ferroui_build::type_system::ModelTypeSystem`) against the
//! run-time type system over the same crates as they are linked into this
//! test (`RuntimeTypeSystem`).
//!
//! Both are read through the type-system contracts of the compiler, type by
//! type and member by member. Every difference has a kind; the test prints
//! the differences of each kind with their number, and fails when a kind that
//! must be empty is not:
//!
//! - **must be empty**: a type or a member that one side has and the other
//!   does not, and a difference of a type or a member both have (its kind,
//!   base, interfaces, the type of a member, its shape, its attributes, the
//!   value of a member of an enumeration). The scanner missed a declaration
//!   or a form of one, or a projection rule of one type system is not the
//!   rule of the other.
//! - **known**: a kind that differs for a stated reason, which is what only
//!   the build or the registration at run time decides: a declaration under a
//!   `cfg` condition, a class its crate does not list for registration, a
//!   member of an enumeration that is no constant expression.
//! - **open**: a Rust type one side maps to a type and the other holds as a
//!   type no metadata declares. It is printed and does not fail the test: a
//!   class its crate does not register is such a type until an instance of it
//!   is created, which another test of the process may do. Each entry is a
//!   handle to declare or a rule to align.
//!
//! The test was the work list of the stage that aligned the two type systems
//! (docs/porting/xaml.md, 9.5.8); with every kind that must be empty at
//! nothing it guards them against drift: a declaration form the scanner does
//! not read, or a registration the model does not state, fails it with the
//! list of what differs.
//!
//! Not from upstream: upstream has one type system per back end over the
//! same metadata and nothing to compare.
//!
//! `cargo test -p ferroui-markup-xaml-tests --lib type_system_drift -- --nocapture`

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use std::rc::Rc;

use ferroui_base::metadata::MarkupType;
use ferroui_base::TypeInfo;
use ferroui_build::scanner::{scan_crate, ScanOptions, Severity};
use ferroui_build::type_system::{ModelType, ModelTypeSystem};
use ferroui_markup_xaml_loader::runtime::type_system::{RuntimeType, RuntimeTypeOrigin, RuntimeTypeSystem};
use xamlx::type_system::{
    IXamlConstructor, IXamlCustomAttribute, IXamlEventInfo, IXamlField, IXamlMember, IXamlMethod, IXamlParameterInfo, IXamlProperty, IXamlType,
    XamlValue,
};

/// The crates the two type systems are compared for.
const CRATES: &[&str] = &["ferroui_base", "ferroui_controls"];

/// How many differences of one kind are printed.
const LISTED: usize = 400;

/// What a kind of difference means for the test.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Status {
    /// The kind must be empty: the test fails when it is not.
    MustBeEmpty,
    /// The kind differs for the stated reason.
    Known(&'static str),
    /// A difference to remove, which does not fail the test.
    Open,
}

/// The kinds of differences, in the order they are printed.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
enum Kind {
    TypeOnlyAtRunTime,
    TypeOnlyInModel,
    TypeUnderCfgOnlyInModel,
    TypeNotRegistered,
    MemberOnlyAtRunTime,
    MemberOnlyInModel,
    OpaqueSpelling,
    EnumValueNotEvaluated,
    HandleNotMappedByModel,
    HandleNotMappedAtRunTime,
    TypeKind,
    Base,
    Interfaces,
    MemberType,
    MemberShape,
    Attributes,
    EnumValue,
}

impl Kind {
    const ALL: &'static [Kind] = &[
        Kind::TypeOnlyAtRunTime,
        Kind::TypeOnlyInModel,
        Kind::TypeUnderCfgOnlyInModel,
        Kind::TypeNotRegistered,
        Kind::MemberOnlyAtRunTime,
        Kind::MemberOnlyInModel,
        Kind::OpaqueSpelling,
        Kind::EnumValueNotEvaluated,
        Kind::HandleNotMappedByModel,
        Kind::HandleNotMappedAtRunTime,
        Kind::TypeKind,
        Kind::Base,
        Kind::Interfaces,
        Kind::MemberType,
        Kind::MemberShape,
        Kind::Attributes,
        Kind::EnumValue,
    ];

    fn title(self) -> &'static str {
        match self {
            Kind::TypeOnlyAtRunTime => "a type the crates register and the model does not have",
            Kind::TypeOnlyInModel => "a type the model has and the crates do not register",
            Kind::TypeUnderCfgOnlyInModel => "a type declared under a `cfg` condition that the crates do not register",
            Kind::TypeNotRegistered => "a class the model marks as not in the list of registered classes of its crate",
            Kind::MemberOnlyAtRunTime => "a member only the run-time type system has",
            Kind::MemberOnlyInModel => "a member only the build-time type system has",
            Kind::OpaqueSpelling => "a Rust type no metadata declares, spelled differently",
            Kind::EnumValueNotEvaluated => "a member of an enumeration whose value the scanner did not evaluate",
            Kind::HandleNotMappedByModel => "a Rust type the run-time type system maps to a type and the build-time one does not",
            Kind::HandleNotMappedAtRunTime => "a Rust type the build-time type system maps to a type and the run-time one does not",
            Kind::TypeKind => "the kind of a type (class, value type, enumeration, interface)",
            Kind::Base => "the base type",
            Kind::Interfaces => "the interfaces",
            Kind::MemberType => "the type of a member, of its result or of a parameter",
            Kind::MemberShape => "the shape of a member (static, accessors, visibility, literal)",
            Kind::Attributes => "the custom attributes",
            Kind::EnumValue => "the value of a member of an enumeration",
        }
    }

    fn status(self) -> Status {
        match self {
            Kind::TypeOnlyAtRunTime
            | Kind::TypeOnlyInModel
            | Kind::MemberOnlyAtRunTime
            | Kind::MemberOnlyInModel
            | Kind::TypeKind
            | Kind::Base
            | Kind::Interfaces
            | Kind::MemberType
            | Kind::MemberShape
            | Kind::Attributes
            | Kind::EnumValue => Status::MustBeEmpty,
            Kind::TypeUnderCfgOnlyInModel => {
                Status::Known("the scanner reads a declaration under any `cfg` condition but `test` and records the condition; whether it holds is decided by the build")
            }
            Kind::TypeNotRegistered => Status::Known(
                "the crate declares the class and its `TYPES` list leaves it out, so `register_types()` does not make it known; the build-time type system does not find it by name or by handle either. At run time it becomes known when an instance is created, which another test of this process may have done: such a type is not compared",
            ),
            Kind::OpaqueSpelling => Status::Known(
                "the run-time type system names such a type by `std::any::type_name` (aliases and default type parameters written out), the build-time one by the normalised text of the declaration; both are opaque and equal only to themselves",
            ),
            Kind::EnumValueNotEvaluated => Status::Known(
                "the discriminant or the constant is not a constant expression over literals and the other members of its type (xaml.md 9.5.6, item 5): a function of the crate, a constant of another type",
            ),
            Kind::HandleNotMappedByModel | Kind::HandleNotMappedAtRunTime => Status::Open,
        }
    }
}

/// The differences found, by kind.
#[derive(Default)]
struct Differences {
    found: BTreeMap<Kind, Vec<String>>,
}

impl Differences {
    fn add(&mut self, kind: Kind, text: String) {
        self.found.entry(kind).or_default().push(text);
    }

    fn count(&self, kind: Kind) -> usize {
        self.found.get(&kind).map_or(0, Vec::len)
    }
}

fn is_runtime_opaque(type_: &Rc<dyn IXamlType>) -> bool {
    type_.as_any().downcast_ref::<RuntimeType>().is_some_and(|runtime| matches!(runtime.origin(), RuntimeTypeOrigin::Opaque))
}

fn is_model_opaque(type_: &Rc<dyn IXamlType>) -> bool {
    type_.as_any().downcast_ref::<ModelType>().is_some_and(ModelType::is_opaque)
}

/// The text of a Rust type without its module paths: `alloc::rc::Rc<dyn a::b::IBrush>` and
/// `::std::rc::Rc<dyn ::a::IBrush>` are both `Rc<dyn IBrush>`. What is left of the
/// difference between `std::any::type_name` and the normalised text of a declaration is a
/// difference of the type (or of how an alias is written).
fn without_paths(text: &str) -> String {
    let is_word = |character: char| character.is_alphanumeric() || character == '_';
    let characters: Vec<char> = text.chars().collect();
    let mut result = String::new();
    let mut index = 0;
    while index < characters.len() {
        if characters[index] == ':' && characters.get(index + 1) == Some(&':') {
            index += 2;
            continue;
        }
        if is_word(characters[index]) {
            let start = index;
            while index < characters.len() && is_word(characters[index]) {
                index += 1;
            }
            let followed_by_path = characters.get(index) == Some(&':') && characters.get(index + 1) == Some(&':');
            if !followed_by_path {
                result.extend(&characters[start..index]);
            }
            continue;
        }
        result.push(characters[index]);
        index += 1;
    }
    // The alias of the untyped value, which `type_name` writes out.
    result.replace("BoxedValue", "Rc<dyn AnyValue>")
}

/// The name a type is compared by: its full name, with the type arguments of an
/// instantiation and the element of an array named the same way, and a Rust type no
/// metadata declares by its text without module paths.
fn canonical_name(type_: &Rc<dyn IXamlType>) -> String {
    if is_runtime_opaque(type_) || is_model_opaque(type_) {
        return format!("rust:{}", without_paths(&type_.name()));
    }
    if let Some(element) = type_.array_element_type() {
        return format!("{}[]", canonical_name(&element));
    }
    let arguments = type_.generic_arguments();
    if arguments.is_empty() {
        return type_.full_name();
    }
    let name = type_.name();
    let qualified = match type_.namespace() {
        Some(namespace) if !namespace.is_empty() => format!("{namespace}.{name}"),
        _ => name,
    };
    format!("{qualified}[{}]", arguments.iter().map(canonical_name).collect::<Vec<_>>().join(","))
}

/// Whether a type (or a type argument of it) is opaque on this side.
fn has_opaque(type_: &Rc<dyn IXamlType>) -> bool {
    is_runtime_opaque(type_)
        || is_model_opaque(type_)
        || type_.array_element_type().is_some_and(|element| has_opaque(&element))
        || type_.generic_arguments().iter().any(has_opaque)
}

/// Compares the type a member has at run time with the one it has in the model; a
/// difference is added under the kind that says why the two differ.
fn compare_types(differences: &mut Differences, place: &str, runtime: &Rc<dyn IXamlType>, model: &Rc<dyn IXamlType>) {
    let (runtime_name, model_name) = (canonical_name(runtime), canonical_name(model));
    if runtime_name == model_name {
        return;
    }
    let kind = match (has_opaque(runtime), has_opaque(model)) {
        (true, true) => Kind::OpaqueSpelling,
        (false, true) => Kind::HandleNotMappedByModel,
        (true, false) => Kind::HandleNotMappedAtRunTime,
        (false, false) => Kind::MemberType,
    };
    differences.add(kind, format!("{place}: at run time `{}`, in the model `{}`", runtime.full_name(), model.full_name()));
}

fn value_text(value: &XamlValue) -> String {
    match value {
        XamlValue::Type(type_) => format!("typeof({})", canonical_name(type_)),
        XamlValue::Array(items) => format!("[{}]", items.iter().map(value_text).collect::<Vec<_>>().join(", ")),
        other => format!("{other:?}"),
    }
}

/// The attributes as text, each `Type(arguments; Name = value, ..)`, in order of the text.
fn attributes_text(attributes: &[Rc<dyn IXamlCustomAttribute>]) -> Vec<String> {
    let mut texts: Vec<String> = attributes
        .iter()
        .map(|attribute| {
            let arguments: Vec<String> = attribute.parameters().iter().map(value_text).collect();
            let mut named: Vec<String> = attribute.properties().iter().map(|(name, value)| format!("{name} = {}", value_text(value))).collect();
            named.sort();
            format!("{}({}; {})", attribute.type_().full_name(), arguments.join(", "), named.join(", "))
        })
        .collect();
    texts.sort();
    texts
}

fn compare_attributes(differences: &mut Differences, place: &str, runtime: &[Rc<dyn IXamlCustomAttribute>], model: &[Rc<dyn IXamlCustomAttribute>]) {
    let (runtime, model) = (attributes_text(runtime), attributes_text(model));
    if runtime != model {
        differences.add(Kind::Attributes, format!("{place}: at run time {runtime:?}, in the model {model:?}"));
    }
}

/// The members of one kind of a type by what identifies them, in a stable order: a member
/// that one side lacks is reported, the pairs are returned.
fn pair<T: Clone>(
    differences: &mut Differences,
    type_name: &str,
    what: &str,
    runtime: Vec<(String, T)>,
    model: Vec<(String, T)>,
) -> Vec<(String, T, T)> {
    // Members of one identity (overloads of one name and number of parameters) are told
    // apart by their position among them.
    let numbered = |members: Vec<(String, T)>| -> BTreeMap<(String, usize), T> {
        let mut seen: BTreeMap<String, usize> = BTreeMap::new();
        let mut result = BTreeMap::new();
        for (identity, member) in members {
            let position = seen.entry(identity.clone()).or_default();
            result.insert((identity, *position), member);
            *position += 1;
        }
        result
    };
    let (runtime, mut model) = (numbered(runtime), numbered(model));
    let mut pairs = Vec::new();
    for ((identity, position), runtime_member) in runtime {
        match model.remove(&(identity.clone(), position)) {
            Some(model_member) => pairs.push((identity, runtime_member, model_member)),
            None => differences.add(Kind::MemberOnlyAtRunTime, format!("{type_name}: {what} {identity}")),
        }
    }
    for ((identity, _), _) in model {
        differences.add(Kind::MemberOnlyInModel, format!("{type_name}: {what} {identity}"));
    }
    pairs
}

fn method_identity(method: &Rc<dyn IXamlMethod>) -> String {
    format!("{}{}/{}", if method.is_static() { "static " } else { "" }, method.name(), method.parameters().len())
}

fn compare_methods(differences: &mut Differences, place: &str, runtime: &Rc<dyn IXamlMethod>, model: &Rc<dyn IXamlMethod>) {
    compare_types(differences, &format!("{place}: the result"), &runtime.return_type(), &model.return_type());
    for (index, (runtime_parameter, model_parameter)) in runtime.parameters().iter().zip(model.parameters().iter()).enumerate() {
        compare_types(differences, &format!("{place}: the parameter {index}"), runtime_parameter, model_parameter);
    }
    compare_attributes(differences, place, &runtime.custom_attributes(), &model.custom_attributes());
    if runtime.is_generic_method_definition() != model.is_generic_method_definition() {
        differences.add(Kind::MemberShape, format!("{place}: a generic method definition on one side only"));
    }
}

/// Compares a type both sides have, member by member.
fn compare(differences: &mut Differences, name: &str, runtime: &Rc<dyn IXamlType>, model: &Rc<dyn IXamlType>) {
    let kinds = |type_: &Rc<dyn IXamlType>| (type_.is_value_type(), type_.is_enum(), type_.is_interface(), type_.generic_parameters().len());
    if kinds(runtime) != kinds(model) {
        differences.add(
            Kind::TypeKind,
            format!("{name}: (value type, enumeration, interface, type parameters) is {:?} at run time and {:?} in the model", kinds(runtime), kinds(model)),
        );
    }

    let base_name = |type_: &Rc<dyn IXamlType>| type_.base_type().map(|base| canonical_name(&base));
    if base_name(runtime) != base_name(model) {
        let text = format!("{name}: at run time {:?}, in the model {:?}", base_name(runtime), base_name(model));
        match (runtime.base_type(), model.base_type()) {
            // A base no metadata declares is a difference of its spelling or of a handle.
            (Some(runtime_base), Some(model_base)) if has_opaque(&runtime_base) || has_opaque(&model_base) => {
                compare_types(differences, &format!("{name}: the base type"), &runtime_base, &model_base)
            }
            _ => differences.add(Kind::Base, text),
        }
    }

    let interfaces = |type_: &Rc<dyn IXamlType>| type_.interfaces().iter().map(canonical_name).collect::<BTreeSet<String>>();
    let (runtime_interfaces, model_interfaces) = (interfaces(runtime), interfaces(model));
    if runtime_interfaces != model_interfaces {
        let only_runtime: Vec<&String> = runtime_interfaces.difference(&model_interfaces).collect();
        let only_model: Vec<&String> = model_interfaces.difference(&runtime_interfaces).collect();
        differences.add(Kind::Interfaces, format!("{name}: only at run time {only_runtime:?}, only in the model {only_model:?}"));
    }

    compare_attributes(differences, name, &runtime.custom_attributes(), &model.custom_attributes());

    // Properties, by name and number of index parameters.
    let properties = |type_: &Rc<dyn IXamlType>| -> Vec<(String, Rc<dyn IXamlProperty>)> {
        type_.properties().into_iter().map(|property| (format!("{}/{}", property.name(), property.indexer_parameters().len()), property)).collect()
    };
    for (identity, runtime_property, model_property) in pair(differences, name, "the property", properties(runtime), properties(model)) {
        let place = format!("{name}.{identity}");
        compare_types(differences, &place, &runtime_property.property_type(), &model_property.property_type());
        let shape = |property: &Rc<dyn IXamlProperty>| {
            let is_static = property.getter().or(property.setter()).is_some_and(|accessor| accessor.is_static());
            (property.getter().is_some(), property.setter().is_some(), is_static)
        };
        if shape(&runtime_property) != shape(&model_property) {
            differences.add(
                Kind::MemberShape,
                format!("{place}: (getter, setter, static) is {:?} at run time and {:?} in the model", shape(&runtime_property), shape(&model_property)),
            );
        }
        for (index, (runtime_parameter, model_parameter)) in
            runtime_property.indexer_parameters().iter().zip(model_property.indexer_parameters().iter()).enumerate()
        {
            compare_types(differences, &format!("{place}: the index parameter {index}"), runtime_parameter, model_parameter);
        }
        compare_attributes(differences, &place, &runtime_property.custom_attributes(), &model_property.custom_attributes());
    }

    // Fields, by name.
    let fields = |type_: &Rc<dyn IXamlType>| -> Vec<(String, Rc<dyn IXamlField>)> { type_.fields().into_iter().map(|field| (field.name(), field)).collect() };
    for (identity, runtime_field, model_field) in pair(differences, name, "the field", fields(runtime), fields(model)) {
        let place = format!("{name}.{identity}");
        compare_types(differences, &place, &runtime_field.field_type(), &model_field.field_type());
        match (runtime_field.get_literal_value().ok(), model_field.get_literal_value().ok()) {
            (Some(runtime_value), Some(model_value)) if runtime_value != model_value => {
                differences.add(Kind::EnumValue, format!("{place}: at run time {runtime_value:?}, in the model {model_value:?}"));
            }
            (Some(runtime_value), None) if runtime.is_enum() => differences.add(Kind::EnumValueNotEvaluated, format!("{place}: at run time {runtime_value:?}")),
            (Some(_), None) | (None, Some(_)) => differences.add(Kind::MemberShape, format!("{place}: a literal on one side only")),
            _ => {}
        }
        compare_attributes(differences, &place, &runtime_field.custom_attributes(), &model_field.custom_attributes());
    }

    // Methods, by name, whether they are static, and number of parameters.
    let methods =
        |type_: &Rc<dyn IXamlType>| -> Vec<(String, Rc<dyn IXamlMethod>)> { type_.methods().into_iter().map(|method| (method_identity(&method), method)).collect() };
    for (identity, runtime_method, model_method) in pair(differences, name, "the method", methods(runtime), methods(model)) {
        compare_methods(differences, &format!("{name}.{identity}"), &runtime_method, &model_method);
    }

    // Constructors, by number of parameters.
    let constructors = |type_: &Rc<dyn IXamlType>| -> Vec<(String, Rc<dyn IXamlConstructor>)> {
        type_.constructors().into_iter().map(|constructor| (format!(".ctor/{}", constructor.parameters().len()), constructor)).collect()
    };
    for (identity, runtime_constructor, model_constructor) in pair(differences, name, "the constructor", constructors(runtime), constructors(model)) {
        let place = format!("{name}{identity}");
        if runtime_constructor.is_public() != model_constructor.is_public() {
            differences.add(
                Kind::MemberShape,
                format!("{place}: public is {} at run time and {} in the model", runtime_constructor.is_public(), model_constructor.is_public()),
            );
        }
        let parameters = runtime_constructor.parameters();
        for (index, (runtime_parameter, model_parameter)) in parameters.iter().zip(model_constructor.parameters().iter()).enumerate() {
            compare_types(differences, &format!("{place}: the parameter {index}"), runtime_parameter, model_parameter);
            if let (Ok(runtime_info), Ok(model_info)) = (runtime_constructor.get_parameter_info(index), model_constructor.get_parameter_info(index)) {
                compare_attributes(differences, &format!("{place}: the parameter {index}"), &runtime_info.custom_attributes(), &model_info.custom_attributes());
            }
        }
    }

    // Events, by name.
    let events = |type_: &Rc<dyn IXamlType>| -> Vec<(String, Rc<dyn IXamlEventInfo>)> { type_.events().into_iter().map(|event| (event.name(), event)).collect() };
    for (identity, runtime_event, model_event) in pair(differences, name, "the event", events(runtime), events(model)) {
        if let (Some(runtime_add), Some(model_add)) = (runtime_event.add(), model_event.add()) {
            compare_methods(differences, &format!("{name}.{identity}: the subscription"), &runtime_add, &model_add);
        }
    }
}

fn crate_of(module_path: &str) -> &str {
    module_path.split("::").next().unwrap_or(module_path)
}

/// The build-time type system over the scans of the base and the controls crates agrees
/// with the run-time type system over the same crates as this test links them.
#[test]
fn model_type_system_agrees_with_the_runtime_type_system() {
    crate::register_types();
    ferroui_controls::register_types();

    // The model: the two crates read as files, the controls with the model of the base
    // crate, as their build scripts would export them.
    let source = Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("..").join("src");
    let base = scan_crate(&ScanOptions::new("ferroui_base", source.join("FerroUI.Base").join("lib.rs")));
    let controls =
        scan_crate(&ScanOptions::new("ferroui_controls", source.join("FerroUI.Controls").join("lib.rs")).with_dependencies(vec![base.model.clone()]));
    for scan in [&base, &controls] {
        println!("==== the scan of {} ====\n{}\n", scan.model.crate_name, scan.summary());
        // What the scanner did not read explains a type or a member only the run-time side has.
        let not_read: Vec<String> = scan.diagnostics_of(Severity::Error).map(|diagnostic| diagnostic.to_string()).collect();
        println!("---- {}: {} declarations the scanner did not read ----", scan.model.crate_name, not_read.len());
        for line in &not_read {
            println!("{line}");
        }
        println!();
    }
    let model_system = ModelTypeSystem::new(vec![base.model, controls.model]);
    let runtime_system = RuntimeTypeSystem::new();

    // The types of the two crates, on each side, by the name they are compared by.
    let mut runtime_types: BTreeMap<String, Rc<dyn IXamlType>> = BTreeMap::new();
    for type_info in TypeInfo::registered_types() {
        if CRATES.contains(&crate_of(type_info.module_path())) {
            let type_ = runtime_system.type_of_class(type_info);
            runtime_types.entry(canonical_name(&type_)).or_insert(type_);
        }
    }
    for markup in MarkupType::registered_types() {
        if CRATES.contains(&crate_of(markup.module_path)) {
            let type_ = runtime_system.type_of_markup(markup);
            runtime_types.entry(canonical_name(&type_)).or_insert(type_);
        }
    }
    // In the model: every declaration, with the conditions it is under and whether its
    // crate leaves it out of its list of registered classes.
    let mut model_types: BTreeMap<String, (Rc<dyn IXamlType>, Vec<String>, bool)> = BTreeMap::new();
    for model in 0..model_system.models().models().len() {
        for (declared, type_) in model_system.types_of_model(model) {
            let type_: Rc<dyn IXamlType> = type_;
            model_types.entry(canonical_name(&type_)).or_insert((type_, declared.cfg.clone(), declared.unregistered));
        }
    }
    println!("types of {CRATES:?}: {} at run time, {} in the model\n", runtime_types.len(), model_types.len());
    assert!(runtime_types.len() > 500, "only {} types of the two crates are registered", runtime_types.len());

    let mut differences = Differences::default();
    let mut compared = 0;
    for (name, runtime) in &runtime_types {
        match model_types.get(name) {
            // Known at run time because a test of this process created an instance of it.
            Some((_, _, true)) => {}
            Some((model, _, false)) => {
                compared += 1;
                compare(&mut differences, name, runtime, model);
            }
            None => differences.add(Kind::TypeOnlyAtRunTime, name.clone()),
        }
    }
    for (name, (_, cfg, unregistered)) in &model_types {
        let at_run_time = runtime_types.contains_key(name);
        match (cfg.is_empty(), *unregistered) {
            (true, true) => differences.add(Kind::TypeNotRegistered, format!("{name}{}", if at_run_time { " (known at run time: an instance was created in this process)" } else { "" })),
            _ if at_run_time => {}
            (true, false) => differences.add(Kind::TypeOnlyInModel, name.clone()),
            (false, _) => differences.add(Kind::TypeUnderCfgOnlyInModel, format!("{name} (cfg: {})", cfg.join(", "))),
        }
    }

    // The report: every kind with its number, and its differences.
    println!("{compared} types are on both sides and compared member by member.\n");
    assert!(compared > 500, "only {compared} types are compared");
    let mut failed: Vec<String> = Vec::new();
    for kind in Kind::ALL {
        let count = differences.count(*kind);
        let status = match kind.status() {
            Status::MustBeEmpty => "MUST BE EMPTY".to_string(),
            Status::Known(reason) => format!("known: {reason}"),
            Status::Open => "open: to align".to_string(),
        };
        println!("---- {count} x {kind:?}: {} ({status}) ----", kind.title());
        for line in differences.found.get(kind).into_iter().flatten().take(LISTED) {
            println!("{line}");
        }
        if count > LISTED {
            println!("... and {} more", count - LISTED);
        }
        println!();
        if kind.status() == Status::MustBeEmpty && count > 0 {
            failed.push(format!("{count} x {kind:?} ({})", kind.title()));
        }
    }
    let summary: Vec<String> = Kind::ALL.iter().map(|kind| format!("{kind:?}: {}", differences.count(*kind))).collect();
    println!("summary: {}", summary.join(", "));
    assert!(
        failed.is_empty(),
        "the build-time type system differs from the run-time one in kinds that must be empty (the lists are printed above; run with --nocapture): {}",
        failed.join("; ")
    );
}

/// The comparison names a Rust type no metadata declares without its module paths.
#[test]
fn opaque_types_are_compared_without_their_paths() {
    assert_eq!(without_paths("alloc::rc::Rc<dyn ferroui_base::media::IBrush>"), "Rc<dyn IBrush>");
    assert_eq!(without_paths("::std::rc::Rc<dyn ::ferroui_base::media::brush::IBrush>"), "Rc<dyn IBrush>");
    assert_eq!(without_paths("core::option::Option<alloc::vec::Vec<(f64, alloc::string::String)>>"), "Option<Vec<(f64, String)>>");
    assert_eq!(without_paths("&'static ferroui_base::FerroProperty"), "&'static FerroProperty");
    assert_eq!(without_paths("Vec<::ferroui_base::ferro_property::BoxedValue>"), "Vec<Rc<dyn AnyValue>>");
}
