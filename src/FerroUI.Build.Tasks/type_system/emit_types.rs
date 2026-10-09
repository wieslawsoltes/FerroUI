//! What the emitter of Rust source asks of the build-time type system
//! ([`EmitTypes`] of the loader, docs/porting/xaml.md 9.5.10 and 9.5.12): the
//! answers of [`ModelTypeSystem`] from the models of the crates, where the
//! run-time type system answers from the registries of the process
//! (`RuntimeEmitTypes`).
//!
//! A Rust type is its one text ([`ModelSet::expanded`]): two keys are the same
//! type exactly when their texts are equal. Every text the answers hand out is
//! one of a table built once from the models (the handles of the types, the
//! types of their members, the registrations) and from the table of runtime
//! library types.
//!
//! # What each answer is read from
//!
//! | Question | Read from |
//! |---|---|
//! | the class a type is, its path, its handle, its base classes | `ferro_class!`, `ferro_static_type!`, the public path of the scan, the list of registered classes |
//! | the metadata of a type, its handles, `this`, the type of a value, the nullable form, the base and the contracts | `ferro_markup_type!`, `ferro_markup_enum!`, `markup:` of `ferro_class_info!`, by the rules of the declaration macros (the nullable form of a value type and of an enumeration is `Option<T>`; the value is `this:`, else the first handle, else `Rc<dyn Trait>` of a contract, else the type) |
//! | a method, a constructor, a static field as declared | [`MemberRust`](super::MemberRust) and [`MemberSource`] of the member |
//! | a registered property, the expression of its definition | [`RegisteredModel`]: the accessors listed under the type the property was resolved on and its base classes, then under the registering type and its base classes; the first public one of a type with a public path |
//! | whether a value of one Rust type is a value of another, the nullable forms, element references | the registrations the scanner read (`ValueTypes::register_*::<..>`, [`AssemblyModel::value_types`], [`AssemblyModel::casts`]) and what the declarations imply (a class registers its handle, a class with `interfaces:` its contracts) |
//!
//! # What the models cannot answer
//!
//! The untyped value conversions are a table of the process: generic code
//! registers types when it first meets them (`register_reference::<S>()` of a
//! binding source), accessors register element references when a class is
//! initialised, and a registration in the body of a macro is not read. A
//! conversion the models state exists; one they do not state may still be
//! registered at run time. So *yes* is answered from the models, and *no* only
//! where no registration the scanner did not read could make it a yes (the
//! shape of the two types rules out every kind of registration that is not
//! read in full: [`ModelEmitTypes::is_assignable`]). Any other question is
//! recorded ([`EmitTypes::take_unanswered`]) and the host refuses the document
//! with it, so that no document is emitted from a guess.

use std::any::Any;
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;

use ferroui_markup_xaml_loader::rust_emitter::emit_types::{
    ConstructorInfo, DeclaredKind, DeclaredMethod, EmitClass, EmitFunction, EmitMarkup, EmitProperty, EmitTypes, EnumMember, FieldInfo, FieldValue,
    FrameworkType, Handle, Known, MethodInfo, TypeKey,
};
use xamlx::type_system::{IXamlConstructor, IXamlField, IXamlMethod, IXamlType};

use crate::model::{RegisteredKind, RegisteredModel, RegistrationModel, TypeKind, TypeModel};
use crate::model_set::ModelSet;

use super::model_type_system::{ModelTypeSystem, Position};
use super::types::{DeclaredRust, MemberSource, ModelConstructor, ModelField, ModelMethod, ModelType, ModelTypeOrigin};

/// The Rust type the run-time loader holds a `System.Type` value in. No declaration names
/// it: it is the first handle of `System.Type`, equal to none of the types a member
/// declares.
const SYSTEM_TYPE_VALUE: &str = "::ferroui_markup_xaml_loader::runtime::type_system::RuntimeTypeValue";

const SHARED: &str = "::std::rc::Rc<";

fn option_of(text: &str) -> String {
    format!("Option<{text}>")
}

fn text_of(key: TypeKey<'_>) -> Option<&str> {
    match key {
        TypeKey::Text(text) => Some(text),
        TypeKey::Id(_) => None,
    }
}

/// A class of the object model, as the models declare it.
pub struct ModelClass {
    position: Position,
    name: String,
    full_name: String,
    rust_path: Option<String>,
    handle: Option<String>,
    has_default_constructor: bool,
    /// The class and its base classes, the class first.
    chain: Vec<Position>,
}

impl EmitClass for ModelClass {
    fn name(&self) -> &str {
        &self.name
    }
    fn full_name(&self) -> String {
        self.full_name.clone()
    }
    fn rust_path(&self) -> Option<&str> {
        self.rust_path.as_deref()
    }
    fn handle(&self) -> Option<TypeKey<'_>> {
        self.handle.as_deref().map(TypeKey::Text)
    }
    fn has_default_constructor(&self) -> bool {
        self.has_default_constructor
    }
    fn is_assignable_from(&self, other: &dyn EmitClass) -> bool {
        other.as_any().downcast_ref::<ModelClass>().is_some_and(|other| other.chain.contains(&self.position))
    }
    fn same(&self, other: &dyn EmitClass) -> bool {
        other.as_any().downcast_ref::<ModelClass>().is_some_and(|other| other.position == self.position)
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// The markup metadata of a type, as the models declare it.
pub struct ModelMarkup {
    position: Position,
    full_name: String,
    rust_path: Option<String>,
    is_trait: bool,
    handles: Vec<String>,
    nullable: Option<String>,
    interfaces: Vec<String>,
    base: Option<Rc<ModelMarkup>>,
    this: Option<String>,
    value: Option<String>,
    is_flags: bool,
    /// The members of an enumeration: name, value and Rust variant.
    enum_members: Vec<(String, Option<i64>, Option<String>)>,
    /// The questions the metadata could not answer, for the host.
    unanswered: Rc<RefCell<Vec<String>>>,
}

impl EmitMarkup for ModelMarkup {
    fn full_name(&self) -> String {
        self.full_name.clone()
    }
    fn rust_path(&self) -> Option<&str> {
        self.rust_path.as_deref()
    }
    fn rust_path_is_trait(&self) -> bool {
        self.rust_path.is_some() && self.is_trait
    }
    fn handles(&self) -> Vec<TypeKey<'_>> {
        self.handles.iter().map(|handle| TypeKey::Text(handle)).collect()
    }
    fn handle_is_shared(&self) -> bool {
        self.handles.first().is_some_and(|handle| handle.starts_with(SHARED))
    }
    fn nullable(&self) -> Option<TypeKey<'_>> {
        self.nullable.as_deref().map(TypeKey::Text)
    }
    fn interfaces(&self) -> Vec<TypeKey<'_>> {
        self.interfaces.iter().map(|interface| TypeKey::Text(interface)).collect()
    }
    fn base_type(&self) -> Option<&dyn EmitMarkup> {
        self.base.as_deref().map(|base| base as &dyn EmitMarkup)
    }
    fn this(&self) -> Option<TypeKey<'_>> {
        self.this.as_deref().map(TypeKey::Text)
    }
    fn value(&self) -> Option<TypeKey<'_>> {
        self.value.as_deref().map(TypeKey::Text)
    }
    fn is_flags(&self) -> bool {
        self.is_flags
    }
    fn enum_members(&self) -> Vec<EnumMember<'_>> {
        self.enum_members
            .iter()
            .filter_map(|(name, value, rust_variant)| match value {
                Some(value) => Some(EnumMember { name, value: *value, rust_variant: rust_variant.as_deref() }),
                None => {
                    self.unanswered.borrow_mut().push(format!("the value of the member {name} of {} is not a constant expression the scanner evaluates", self.full_name));
                    None
                }
            })
            .collect()
    }
    fn flags_compose(&self, value: i64) -> bool {
        let mut rest = value;
        for (name, bits, _) in &self.enum_members {
            let Some(bits) = bits else {
                self.unanswered.borrow_mut().push(format!("the value of the member {name} of {} is not a constant expression the scanner evaluates", self.full_name));
                return false;
            };
            if !self.is_flags {
                if *bits == value {
                    return true;
                }
            } else if value & bits == *bits {
                rest &= !bits;
            }
        }
        self.is_flags && rest == 0
    }
    fn same(&self, other: &dyn EmitMarkup) -> bool {
        other.as_any().downcast_ref::<ModelMarkup>().is_some_and(|other| other.position == self.position)
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// A registered property, as the models declare it.
pub struct ModelEmitProperty {
    /// The registration whose accessors return this definition: the declaration, or the
    /// added owner of a direct property (which is a definition of its own).
    identity: (Position, usize),
    name: String,
    owner_name: String,
    /// The class that registered the property.
    owner: Option<Position>,
    is_direct: bool,
    is_read_only: bool,
    property_type: String,
}

impl EmitProperty for ModelEmitProperty {
    fn name(&self) -> &str {
        &self.name
    }
    fn owner_name(&self) -> String {
        self.owner_name.clone()
    }
    fn is_direct(&self) -> bool {
        self.is_direct
    }
    fn is_read_only(&self) -> bool {
        self.is_read_only
    }
    fn property_type(&self) -> TypeKey<'_> {
        TypeKey::Text(&self.property_type)
    }
    fn property_type_name(&self) -> String {
        self.property_type.clone()
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// What the crates register with the untyped value conversions, as far as the models
/// state it.
#[derive(Default)]
struct Conversions {
    /// The nullable forms, each with the type it holds.
    nullable: HashMap<String, String>,
    /// The casts, by the two types.
    casts: HashSet<(String, String)>,
    /// The handles of the classes (`Ref<T>`), each with its class.
    objects: HashMap<String, Position>,
    /// The classes that state a contract handle among their interfaces, by the handle.
    interfaces: HashMap<String, Vec<Position>>,
    /// The forms of the shared types (`register_reference::<T>`): `T` by `Rc<T>` and the reverse.
    reference_handles: HashMap<String, String>,
    reference_objects: HashMap<String, String>,
    /// The element references (`ElementRef<T>` and its nullable form), with the class.
    element_refs: HashMap<String, (Position, bool)>,
    /// The registration functions with calls the scanner did not read, over all models.
    unread: HashSet<String>,
}

/// [`EmitTypes`] of the build-time type system; see the module.
pub struct ModelEmitTypes {
    system: Rc<ModelTypeSystem>,
    /// Every Rust type text the answers hand out.
    texts: HashSet<String>,
    known: HashMap<Known, (String, String)>,
    classes: HashMap<Position, ModelClass>,
    /// The metadata of the types with markup metadata, and of the classes that state some.
    markups: HashMap<Position, Rc<ModelMarkup>>,
    /// The registered properties, by the registration the static field holds.
    properties: HashMap<(Position, usize), ModelEmitProperty>,
    class_handles: HashMap<String, (Position, bool)>,
    markup_handles: HashMap<String, Position>,
    conversions: Conversions,
    styled_element: Option<Position>,
    /// The classes with markup of their own in the group that is compiled, by full name.
    class_documents: HashSet<String>,
    unanswered: Rc<RefCell<Vec<String>>>,
}

/// The known Rust types, with the paths the base crate and the XAML runtime library
/// export them by.
const KNOWN: &[(Known, &str)] = &[
    (Known::String, "String"),
    (Known::OptionString, "Option<String>"),
    (Known::Bool, "bool"),
    (Known::Char, "char"),
    (Known::I8, "i8"),
    (Known::U8, "u8"),
    (Known::I16, "i16"),
    (Known::U16, "u16"),
    (Known::I32, "i32"),
    (Known::U32, "u32"),
    (Known::I64, "i64"),
    (Known::U64, "u64"),
    (Known::F32, "f32"),
    (Known::F64, "f64"),
    (Known::Object, "Option<::ferroui_base::BoxedValue>"),
    (Known::ServiceProvider, "::std::rc::Rc<dyn ::ferroui_base::metadata::IServiceProvider>"),
    (Known::OptionServiceProvider, "Option<::std::rc::Rc<dyn ::ferroui_base::metadata::IServiceProvider>>"),
    (Known::Selector, "::ferroui_base::styling::Selector"),
    (Known::OptionSelector, "Option<::ferroui_base::styling::Selector>"),
    (Known::OptionUri, "Option<::ferroui_base::utilities::Uri>"),
    (Known::Class, "&'static ::ferroui_base::TypeInfo"),
    (Known::OptionClass, "Option<&'static ::ferroui_base::TypeInfo>"),
    (Known::ValueType, "::ferroui_base::data::core::ValueType"),
    (Known::OptionValueType, "Option<::ferroui_base::data::core::ValueType>"),
    (Known::TypeId, "::std::any::TypeId"),
    (Known::OptionTypeId, "Option<::std::any::TypeId>"),
    (Known::BindingPriority, "::ferroui_base::data::BindingPriority"),
    (Known::DeferredContent, "::std::rc::Rc<::ferroui_markup_xaml::xaml_il::runtime::DeferredContent>"),
    (Known::CompiledBindingPath, "::ferroui_base::data::CompiledBindingPath"),
    (Known::Property, "&'static ::ferroui_base::FerroProperty"),
    (Known::OptionProperty, "Option<&'static ::ferroui_base::FerroProperty>"),
    (Known::Delegate, "::ferroui_base::metadata::MarkupDelegate"),
];

/// The primitive types generated code names by their name, and `String` by its path.
const PRIMITIVES: &[(&str, &str)] = &[
    ("bool", "bool"),
    ("char", "char"),
    ("i8", "i8"),
    ("u8", "u8"),
    ("i16", "i16"),
    ("u16", "u16"),
    ("i32", "i32"),
    ("u32", "u32"),
    ("i64", "i64"),
    ("u64", "u64"),
    ("f32", "f32"),
    ("f64", "f64"),
    ("String", "::std::string::String"),
];

/// The type whose function the accessor `registered` of the type `listed` is, when a
/// model declares it.
fn accessor_type<'a>(set: &'a ModelSet, listed: &'a TypeModel, registered: &RegisteredModel) -> Option<&'a TypeModel> {
    match &registered.function_of {
        Some(function_of) => set.find_rust_type(function_of).map(|(_, type_)| type_),
        None => Some(listed),
    }
}

/// The public path generated code names a type by: the one the scan found, or the one
/// the crate states for an instantiation of a generic type, for a type its crate registers.
fn public_path(declared: &TypeModel) -> Option<String> {
    declared.public_path.clone().or_else(|| declared.stated_path.clone()).filter(|_| !declared.unregistered)
}

impl ModelEmitTypes {
    /// What `system` states for the emitter. `class_documents` are the full names of the
    /// classes with markup of their own (`x:Class`) among the documents that are compiled
    /// and the compiled documents of the crates they are built on.
    pub fn new(system: Rc<ModelTypeSystem>, class_documents: impl IntoIterator<Item = String>) -> Self {
        let unanswered: Rc<RefCell<Vec<String>>> = Rc::default();
        let set = system.models();
        let ref_path = system.ref_path().to_string();
        let element_ref_path = set.canonical_path("::ferroui_base::ElementRef");
        let mut texts: HashSet<String> = HashSet::new();
        let mut add_text = |text: &str| {
            if !texts.contains(text) {
                texts.insert(option_of(text));
                texts.insert(text.to_string());
            }
        };

        let known: HashMap<Known, (String, String)> = KNOWN
            .iter()
            .map(|(known, text)| {
                let text = set.expanded(text);
                (*known, (text.clone(), option_of(&text)))
            })
            .collect();
        for (text, optional) in known.values() {
            add_text(text);
            add_text(optional);
        }
        add_text(SYSTEM_TYPE_VALUE);
        for (text, _) in ferroui_markup_xaml_loader::core_table::core_handles() {
            add_text(&set.expanded(&text));
        }
        for handle in ferroui_markup_xaml_loader::core_table::PROPERTY_HANDLES {
            add_text(&set.expanded(handle));
        }

        // The classes, with their chains of base classes.
        let class_position = |text: &str| set.position_of_rust_type(text).filter(|position| set.type_at(*position).object_model);
        let mut classes: HashMap<Position, ModelClass> = HashMap::new();
        let mut class_handles: HashMap<String, (Position, bool)> = HashMap::new();
        let mut conversions = Conversions::default();
        for (model_index, model) in set.models().iter().enumerate() {
            for (registration, count) in &model.unread_value_types {
                if *count > 0 {
                    conversions.unread.insert(registration.clone());
                }
            }
            for (type_index, declared) in model.types.iter().enumerate() {
                let position = (model_index, type_index);
                let mut scratch = declared.clone();
                scratch.visit_types_mut(&mut |type_| add_text(&set.expanded(&type_.text)));
                if !declared.object_model {
                    continue;
                }
                let path = set.expanded(&declared.rust_path.text);
                let handle = (declared.kind == TypeKind::Class).then(|| format!("{ref_path}<{path}>"));
                let mut chain = vec![position];
                let mut current = declared;
                while let Some(base) = current.base.as_ref().and_then(|base| class_position(&base.text)) {
                    if chain.contains(&base) {
                        break;
                    }
                    chain.push(base);
                    current = set.type_at(base);
                }
                if let Some(handle) = &handle {
                    add_text(handle);
                    add_text(&format!("{element_ref_path}<{path}>"));
                    // A class the crate does not register is not found by its handle.
                    if !declared.unregistered {
                        class_handles.entry(handle.clone()).or_insert((position, false));
                        class_handles.entry(option_of(handle)).or_insert((position, true));
                    }
                    // Every class registers its handle with the untyped value conversions
                    // when it is initialised: the handle is an object, and its nullable form.
                    conversions.objects.entry(handle.clone()).or_insert(position);
                    conversions.nullable.entry(option_of(handle)).or_insert_with(|| handle.clone());
                    conversions.casts.insert((handle.clone(), option_of(handle)));
                    // And the contracts it states (`interfaces:` of `ferro_class_info!`).
                    for interface in &declared.interfaces {
                        let interface = set.expanded(&interface.text);
                        conversions.nullable.entry(option_of(&interface)).or_insert_with(|| interface.clone());
                        conversions.casts.insert((interface.clone(), option_of(&interface)));
                        conversions.interfaces.entry(interface).or_default().push(position);
                    }
                }
                classes.insert(
                    position,
                    ModelClass {
                        position,
                        name: declared.name.clone(),
                        full_name: declared.full_name(),
                        rust_path: public_path(declared),
                        handle,
                        has_default_constructor: declared.default_constructor.is_some(),
                        chain,
                    },
                );
            }
        }

        // What the functions of the crates register next to the declarations.
        for model in set.models() {
            let mut scratch = model.clone();
            scratch.visit_types_mut(&mut |type_| add_text(&set.expanded(&type_.text)));
            for cast in model.casts.iter().filter(|cast| cast.from.is_resolved() && cast.to.is_resolved()) {
                conversions.casts.insert((set.expanded(&cast.from.text), set.expanded(&cast.to.text)));
            }
            for registration in model.value_types.iter().filter(|registration| registration.types.iter().all(|type_| type_.is_resolved())) {
                let types: Vec<String> = registration.types.iter().map(|type_| set.expanded(&type_.text)).collect();
                let nullable = |conversions: &mut Conversions, inner: &str| {
                    conversions.nullable.entry(option_of(inner)).or_insert_with(|| inner.to_string());
                    conversions.casts.insert((inner.to_string(), option_of(inner)));
                };
                match (registration.registration.as_str(), types.as_slice()) {
                    ("nullable", [inner]) => nullable(&mut conversions, inner),
                    ("reference", [object]) => {
                        let shared = format!("{SHARED}{object}>");
                        let optional = option_of(&shared);
                        add_text(&shared);
                        conversions.nullable.entry(optional.clone()).or_insert_with(|| object.clone());
                        conversions.reference_handles.insert(shared.clone(), object.clone());
                        conversions.reference_objects.insert(object.clone(), shared.clone());
                        for (from, to) in [(object, &optional), (object, &shared), (&shared, object), (&shared, &optional)] {
                            conversions.casts.insert((from.clone(), to.clone()));
                        }
                    }
                    ("object", [class]) => {
                        if let Some(position) = class_position(class) {
                            let handle = format!("{ref_path}<{class}>");
                            conversions.objects.entry(handle.clone()).or_insert(position);
                            nullable(&mut conversions, &handle);
                        }
                    }
                    ("element_ref", [class]) => {
                        if let Some(position) = class_position(class) {
                            let handle = format!("{ref_path}<{class}>");
                            let element = format!("{element_ref_path}<{class}>");
                            let (optional_handle, optional_element) = (option_of(&handle), option_of(&element));
                            nullable(&mut conversions, &element);
                            conversions.element_refs.insert(element.clone(), (position, false));
                            conversions.element_refs.insert(optional_element.clone(), (position, true));
                            for (from, to) in [
                                (&handle, &element),
                                (&handle, &optional_element),
                                (&optional_handle, &optional_element),
                                (&element, &optional_handle),
                                (&optional_element, &optional_handle),
                            ] {
                                conversions.casts.insert((from.clone(), to.clone()));
                            }
                        }
                    }
                    ("interface", [class, interface]) => {
                        if let Some(position) = class_position(class) {
                            conversions.interfaces.entry(interface.clone()).or_default().push(position);
                            nullable(&mut conversions, interface);
                        }
                    }
                    ("upcast", [class, base]) => {
                        let (from, to) = (format!("{ref_path}<{class}>"), format!("{ref_path}<{base}>"));
                        conversions.casts.insert((from.clone(), option_of(&to)));
                        conversions.casts.insert((from, to));
                    }
                    _ => {}
                }
            }
        }
        for text in conversions.nullable.keys().chain(conversions.nullable.values()) {
            add_text(text);
        }

        let mut types = Self {
            system: system.clone(),
            texts,
            known,
            classes,
            markups: HashMap::new(),
            properties: HashMap::new(),
            class_handles,
            markup_handles: HashMap::new(),
            conversions,
            styled_element: system.named("FerroUI.StyledElement").filter(|position| set.type_at(*position).object_model),
            class_documents: class_documents.into_iter().collect(),
            unanswered,
        };
        types.index_markup_handles();
        for model_index in 0..set.models().len() {
            for type_index in 0..set.models()[model_index].types.len() {
                let _ = types.markup_at((model_index, type_index), &mut Vec::new());
            }
        }
        types.index_properties();
        types
    }

    fn set(&self) -> &ModelSet {
        self.system.models()
    }

    /// The handles of the types with markup metadata, as the registry of the metadata has
    /// them: the handles each declaration lists, the first declaration of a handle first,
    /// then the handles the crates register for a type next to its declaration.
    fn index_markup_handles(&mut self) {
        let set = self.system.models();
        let mut handles: HashMap<String, Position> = HashMap::new();
        for (model_index, model) in set.models().iter().enumerate() {
            for (type_index, declared) in model.types.iter().enumerate() {
                if !Self::is_registered_metadata(declared) {
                    continue;
                }
                for handle in Self::declared_handles(set, self.system.ref_path(), declared) {
                    handles.entry(handle).or_insert((model_index, type_index));
                }
            }
        }
        for model in set.models() {
            for handle in &model.handles {
                if let Some(position) = set.position_of_rust_type(&handle.type_.text) {
                    handles.entry(set.expanded(&handle.handle.text)).or_insert(position);
                }
            }
        }
        self.markup_handles = handles;
    }

    /// Whether the metadata `declared` states is in the registry of the types with markup
    /// metadata: every type that is not a class of the object model, and a static type
    /// whose metadata names it (`type_info:`). The metadata of a class is reached through
    /// the class only.
    fn is_registered_metadata(declared: &TypeModel) -> bool {
        !declared.object_model || declared.type_info.is_some()
    }

    /// The handles the metadata of `declared` lists, in the order of the declaration.
    fn declared_handles(set: &ModelSet, ref_path: &str, declared: &TypeModel) -> Vec<String> {
        let path = set.expanded(&declared.rust_path.text);
        let mut handles: Vec<String> = Vec::new();
        if declared.object_model && declared.kind == TypeKind::Class {
            handles.push(format!("{ref_path}<{path}>"));
            handles.push(format!("Option<{ref_path}<{path}>>"));
        }
        if declared.kind == TypeKind::Enum {
            handles.push(path);
        }
        for handle in &declared.handles {
            let handle = set.expanded(&handle.text);
            if !handles.contains(&handle) {
                handles.push(handle);
            }
        }
        handles
    }

    /// The metadata the declaration at `position` states, when it states some: of a type
    /// with markup metadata, or of a class with `markup:` (or a static type with
    /// `type_info:`).
    fn markup_at(&mut self, position: Position, visiting: &mut Vec<Position>) -> Option<Rc<ModelMarkup>> {
        if let Some(found) = self.markups.get(&position) {
            return Some(found.clone());
        }
        let system = self.system.clone();
        let set = system.models();
        let declared = set.type_at(position);
        if declared.object_model && !declared.class_markup && declared.type_info.is_none() {
            return None;
        }
        if visiting.contains(&position) {
            return None;
        }
        visiting.push(position);
        let path = set.expanded(&declared.rust_path.text);
        let is_trait = path.starts_with("dyn ");
        let handles = Self::declared_handles(set, system.ref_path(), declared);
        let is_class = declared.object_model && declared.kind == TypeKind::Class;
        // The base: of the metadata of a class, the metadata of the nearest base class
        // that states some; else the metadata the base handle is a handle of.
        let class_base = self.classes.get(&position).map(|class| class.chain[1..].to_vec()).and_then(|chain| {
            chain.into_iter().find_map(|base| {
                let stated = set.type_at(base).class_markup;
                stated.then(|| self.markup_at(base, visiting)).flatten()
            })
        });
        let base = match class_base {
            Some(base) => Some(base),
            None if is_class => None,
            None => declared
                .base
                .as_ref()
                .and_then(|base| self.markup_handles.get(&set.expanded(&base.text)).copied())
                .and_then(|base| self.markup_at(base, visiting)),
        };
        visiting.pop();
        let explicit_this = declared.this.as_ref().map(|this| set.expanded(&this.text));
        let (this, value) = if is_class {
            (handles.first().cloned(), handles.first().cloned())
        } else {
            // `this:`, else the declared type; a contract without `this:` has no instance type.
            let this = explicit_this.clone().or_else(|| (!is_trait).then(|| path.clone()));
            // `this:`, else the first handle, else the handle of a contract, else the type.
            let value = explicit_this.or_else(|| handles.first().cloned()).unwrap_or_else(|| match is_trait {
                true => format!("{SHARED}{path}>"),
                false => path.clone(),
            });
            (this, Some(value))
        };
        // The nullable form of a value type and of an enumeration is `Option<T>`.
        let nullable = matches!(declared.kind, TypeKind::Struct | TypeKind::Enum).then(|| option_of(&path));
        for text in handles.iter().chain(&this).chain(&value).chain(&nullable) {
            if !self.texts.contains(text) {
                self.texts.insert(text.clone());
            }
        }
        let markup = Rc::new(ModelMarkup {
            position,
            full_name: declared.full_name(),
            // The metadata of a class has no path of its own: the class has it.
            rust_path: if is_class { None } else { public_path(declared) },
            is_trait,
            handles,
            nullable,
            interfaces: declared.interfaces.iter().map(|interface| set.expanded(&interface.text)).collect(),
            base,
            this,
            value,
            is_flags: declared.is_flags,
            enum_members: declared.enum_members.iter().map(|member| (member.name.clone(), member.value, member.rust_variant.clone())).collect(),
            unanswered: self.unanswered.clone(),
        });
        self.markups.insert(position, markup.clone());
        Some(markup)
    }

    /// The registered properties, one description per registration a static field holds.
    fn index_properties(&mut self) {
        let system = self.system.clone();
        let set = system.models();
        // The registrations by their address, to name the declaration of an added owner.
        let mut positions: HashMap<usize, (Position, usize)> = HashMap::new();
        for (model_index, model) in set.models().iter().enumerate() {
            for (type_index, declared) in model.types.iter().enumerate() {
                for (index, registered) in declared.registered.iter().enumerate() {
                    positions.insert(registered as *const RegisteredModel as usize, ((model_index, type_index), index));
                }
            }
        }
        let class_position = |text: &str| set.position_of_rust_type(text).filter(|position| set.type_at(*position).object_model);
        for (model_index, model) in set.models().iter().enumerate() {
            for (type_index, declared) in model.types.iter().enumerate() {
                for (index, registered) in declared.registered.iter().enumerate() {
                    let listed = (model_index, type_index);
                    let Some(name) = set.name_of(registered) else { continue };
                    // The added owner of a direct property is a definition of its own.
                    let own = registered.registration == RegistrationModel::Declared
                        || (registered.kind == RegisteredKind::Direct && registered.registration == RegistrationModel::AddedOwner);
                    let identity = match own {
                        true => Some((listed, index)),
                        false => set.declaration_of(registered).and_then(|declaration| positions.get(&(declaration as *const RegisteredModel as usize)).copied()),
                    };
                    let Some(identity) = identity else { continue };
                    let definition = &set.type_at(identity.0).registered[identity.1];
                    let owner = definition.owner.as_ref().and_then(|owner| class_position(&owner.text)).unwrap_or(identity.0);
                    self.properties.insert(
                        (listed, index),
                        ModelEmitProperty {
                            identity,
                            name: name.to_string(),
                            owner_name: set.type_at(owner).name.clone(),
                            owner: Some(owner),
                            is_direct: definition.kind == RegisteredKind::Direct,
                            is_read_only: definition.kind == RegisteredKind::Direct && definition.read_only,
                            property_type: set.expanded(&definition.value_type.text),
                        },
                    );
                }
            }
        }
    }

    fn model_type<'t>(&self, type_: &'t dyn IXamlType) -> Option<&'t ModelType> {
        type_.as_any().downcast_ref::<ModelType>()
    }

    /// The position of the declaration a type of the models is projected from.
    fn position_of(&self, type_: &dyn IXamlType) -> Option<Position> {
        match self.model_type(type_)?.origin() {
            ModelTypeOrigin::Model { model, type_ } => Some((*model, *type_)),
            _ => None,
        }
    }

    /// The key of the Rust type `text`, when a model or the table states the type.
    fn key(&self, text: &str) -> Option<TypeKey<'_>> {
        self.texts.get(text).map(|text| TypeKey::Text(text.as_str()))
    }

    /// The key of a Rust type a member declares; a text no table has is a question the
    /// models cannot answer.
    fn key_of(&self, text: &str, what: &str) -> Option<TypeKey<'_>> {
        let found = self.key(text);
        if found.is_none() {
            self.cannot_answer(format!("{what}: the Rust type `{text}` is in no model"));
        }
        found
    }

    fn handle(&self, text: &str, what: &str) -> Option<Handle<'_>> {
        let TypeKey::Text(text) = self.key_of(text, what)? else { return None };
        let object = self.known.get(&Known::Object).is_some_and(|(object, _)| object == text) || text == self.boxed_value();
        Some(Handle { key: TypeKey::Text(text), object, name: text })
    }

    fn boxed_value(&self) -> &str {
        self.known.get(&Known::Object).and_then(|(object, _)| object.strip_prefix("Option<")).and_then(|inner| inner.strip_suffix('>')).unwrap_or("")
    }

    fn is_object(&self, text: &str) -> bool {
        self.known.get(&Known::Object).is_some_and(|(object, _)| object == text) || (!text.is_empty() && text == self.boxed_value())
    }

    fn cannot_answer(&self, question: String) {
        let mut unanswered = self.unanswered.borrow_mut();
        if !unanswered.contains(&question) {
            unanswered.push(question);
        }
    }

    fn emit_function(source: &MemberSource) -> Option<EmitFunction> {
        match source {
            MemberSource::Declared { typed_function: Some(function), fallible, .. } => Some(EmitFunction { function: function.clone(), fallible: *fallible }),
            _ => None,
        }
    }

    /// Whether a registration function of the untyped value conversions has calls the
    /// scanner did not read.
    fn unread(&self, registrations: &[&str]) -> bool {
        registrations.iter().any(|registration| self.conversions.unread.contains(*registration))
    }

    /// The other form of a shared type: `Rc<T>` of `T`, `T` of `Rc<T>`.
    fn other_reference_form(&self, text: &str) -> Option<&String> {
        self.conversions.reference_handles.get(text).or_else(|| self.conversions.reference_objects.get(text))
    }

    /// A cast between two forms of shared types that a registered cast between other forms
    /// of them implies.
    fn reference_cast(&self, from: &str, to: &str) -> bool {
        let (from_other, to_other) = (self.other_reference_form(from), self.other_reference_form(to));
        if from_other.is_none() && to_other.is_none() {
            return false;
        }
        let casts = &self.conversions.casts;
        let has = |from: &str, to: &str| casts.contains(&(from.to_string(), to.to_string()));
        for source in [Some(from), from_other.map(String::as_str)].into_iter().flatten() {
            for target in [Some(to), to_other.map(String::as_str)].into_iter().flatten() {
                if (source, target) == (from, to) || source == target || !has(source, target) {
                    continue;
                }
                // The first cast between two forms decides, as at run time.
                return (source == from || has(from, source)) && (target == to || has(target, to));
            }
        }
        false
    }

    /// What the models state about the assignability of `from` to `to`
    /// (`ValueTypes::is_assignable`, rule by rule).
    fn stated_assignable(&self, from: &str, to: &str) -> bool {
        if from == to || self.is_object(to) {
            return true;
        }
        let conversions = &self.conversions;
        if conversions.casts.contains(&(from.to_string(), to.to_string())) || self.reference_cast(from, to) {
            return true;
        }
        let (from_class, to_class) = (conversions.objects.get(from), conversions.objects.get(to));
        if let (Some(from_class), Some(to_class)) = (from_class, to_class) {
            return self.classes.get(from_class).is_some_and(|class| class.chain.contains(to_class));
        }
        if let Some(from_class) = from_class {
            let chain = self.classes.get(from_class).map(|class| class.chain.as_slice()).unwrap_or(&[]);
            if conversions.interfaces.get(to).is_some_and(|classes| classes.iter().any(|class| chain.contains(class))) {
                return true;
            }
        }
        match (conversions.nullable.get(from), conversions.nullable.get(to)) {
            (Some(inner), Some(_)) => self.stated_assignable(inner, to),
            (None, Some(inner)) => self.stated_assignable(from, inner),
            _ => false,
        }
    }

    /// Whether no registration the scanner did not read can make `from` assignable to
    /// `to`. Each registration function makes casts of one shape:
    ///
    /// | Registration | Casts |
    /// |---|---|
    /// | `register_cast::<A, B>` | `A` to `B`: any two types |
    /// | `register_nullable::<T>` | `T` to `Option<T>` |
    /// | `register_reference::<T>` | between `T`, `Rc<T>` and `Option<Rc<T>>` |
    /// | `register_object::<T>` | `Ref<T>` to `Option<Ref<T>>` |
    /// | `register_element_ref::<T>` | between `Ref<T>`, `ElementRef<T>` and their nullable forms |
    /// | `register_upcast::<T, B>` | `Ref<T>` to `Ref<B>` and `Option<Ref<B>>` |
    /// | `register_interface::<T, I>` | `Ref<T>` to `I`, `I` to `Option<I>` |
    ///
    /// A nullable form also assigns as the type it holds, so a type that may be a nullable
    /// form a registration that is not read makes is decided for the type it would hold too.
    ///
    /// `register_object` and `register_upcast` take classes, whose handles the models have
    /// in full, so a call of them that is not read changes nothing.
    fn decided_not_assignable(&self, from: &str, to: &str, depth: usize) -> Result<(), String> {
        const MAX_DEPTH: usize = 8;
        if depth > MAX_DEPTH {
            return Err("the nullable forms of the two types nest too deep".to_string());
        }
        if self.unread(&["cast"]) {
            return Err("a crate registers casts whose types the scanner did not read".to_string());
        }
        let casts = &self.conversions.casts;
        let ref_path = self.system.ref_path();
        let is_handle = |text: &str| text.starts_with(&format!("{ref_path}<")) || text.starts_with(&format!("Option<{ref_path}<"));
        if self.unread(&["reference"]) {
            // The other form each type would have as a shared type: `T` of `Rc<T>`, `Rc<T>` of `T`.
            let other = |text: &str| match text.strip_prefix(SHARED).and_then(|inner| inner.strip_suffix('>')) {
                Some(object) => object.to_string(),
                None => format!("{SHARED}{text}>"),
            };
            let (from_other, to_other) = (other(from), other(to));
            // The forms of one shared type are one reference.
            if to == from_other || to == option_of(&from_other) {
                return Err(format!("`{from}` and `{to}` may be forms of one shared type a registration the scanner did not read makes"));
            }
            // A cast registered for one form of a shared type applies to its other forms.
            for source in [from, from_other.as_str()] {
                for target in [to, to_other.as_str()] {
                    if (source, target) != (from, to) && casts.contains(&(source.to_string(), target.to_string())) {
                        return Err(format!("the cast from `{source}` to `{target}` applies if `{from}` or `{to}` is a shared type a registration the scanner did not read makes"));
                    }
                }
            }
        }
        if is_handle(from) && self.unread(&["interface"]) {
            return Err(format!("`{from}` is the handle of a class and a crate registers contracts of classes the scanner did not read"));
        }
        if (from.contains("ElementRef<") || to.contains("ElementRef<")) && self.unread(&["element_ref"]) {
            return Err("a crate registers element references the scanner did not read".to_string());
        }
        // The nullable forms: null stays null, a value is assigned as the type the form
        // holds. A form the models state was followed; one they do not state may be made
        // by a registration that is not read, and then decides as the type it would hold.
        let nullable_forms = ["nullable", "reference", "interface", "element_ref"];
        let held = |text: &str| -> Vec<String> {
            let Some(inner) = text.strip_prefix("Option<").and_then(|inner| inner.strip_suffix('>')) else { return Vec::new() };
            let mut held = vec![inner.to_string()];
            // `Option<Rc<T>>` of a shared type holds `T`.
            held.extend(inner.strip_prefix(SHARED).and_then(|object| object.strip_suffix('>')).map(str::to_string));
            held
        };
        let unread_forms = self.unread(&nullable_forms);
        let to_stated = self.conversions.nullable.get(to);
        let from_stated = self.conversions.nullable.get(from);
        let to_held: Vec<String> = match to_stated {
            Some(inner) => vec![inner.clone()],
            None if unread_forms => held(to),
            None => Vec::new(),
        };
        let from_held: Vec<String> = match from_stated {
            Some(inner) => vec![inner.clone()],
            None if unread_forms => held(from),
            None => Vec::new(),
        };
        // `(None, Some(inner))`: assigned as the type the target holds.
        for inner in &to_held {
            if to_stated.is_none() && self.stated_assignable(from, inner) {
                return Err(format!("`{to}` may be a nullable form a registration the scanner did not read makes"));
            }
            self.decided_not_assignable(from, inner, depth + 1)?;
        }
        // `(Some(inner), Some(_))`: the value the source holds, assigned to the target.
        if !to_held.is_empty() {
            for inner in &from_held {
                if from_stated.is_none() && self.stated_assignable(inner, to) {
                    return Err(format!("`{from}` may be a nullable form a registration the scanner did not read makes"));
                }
                self.decided_not_assignable(inner, to, depth + 1)?;
            }
        }
        Ok(())
    }

    /// The description of the class at `position`.
    fn class_at(&self, position: Position) -> Option<&dyn EmitClass> {
        self.classes.get(&position).map(|class| class as &dyn EmitClass)
    }
}

impl EmitTypes for ModelEmitTypes {
    fn known(&self, known: Known) -> TypeKey<'_> {
        match self.known.get(&known) {
            Some((text, _)) => TypeKey::Text(text),
            None => TypeKey::Text(""),
        }
    }

    fn option_of_known(&self, known: Known) -> TypeKey<'_> {
        match self.known.get(&known) {
            Some((_, optional)) => TypeKey::Text(optional),
            None => TypeKey::Text(""),
        }
    }

    fn is_own(&self, type_: &dyn IXamlType) -> bool {
        self.model_type(type_).is_some()
    }

    fn class_of(&self, type_: &dyn IXamlType) -> Option<&dyn EmitClass> {
        self.class_at(self.position_of(type_)?)
    }

    fn markup_of(&self, type_: &dyn IXamlType) -> Option<&dyn EmitMarkup> {
        self.markups.get(&self.position_of(type_)?).map(|markup| &**markup as &dyn EmitMarkup)
    }

    fn metadata_of(&self, type_: &dyn IXamlType) -> Option<&dyn EmitMarkup> {
        if let Some(found) = self.markup_of(type_) {
            return Some(found);
        }
        let handle = self.handle_of(type_)?;
        self.markup_by_handle(handle.key)
    }

    fn handle_of(&self, type_: &dyn IXamlType) -> Option<Handle<'_>> {
        let model = self.model_type(type_)?;
        // The run-time loader holds a type value in a type of its own, which is the first
        // handle of `System.Type`.
        if model.key() == "System.Type" {
            return self.handle(SYSTEM_TYPE_VALUE, "the handle of System.Type");
        }
        let first = model.handles().into_iter().next()?;
        self.handle(&first, &format!("the handle of {}", type_.full_name()))
    }

    fn class_by_handle(&self, key: TypeKey<'_>) -> Option<(&dyn EmitClass, bool)> {
        let (position, nullable) = self.class_handles.get(text_of(key)?)?;
        Some((self.class_at(*position)?, *nullable))
    }

    fn markup_by_handle(&self, key: TypeKey<'_>) -> Option<&dyn EmitMarkup> {
        let position = self.markup_handles.get(text_of(key)?)?;
        self.markups.get(position).map(|markup| &**markup as &dyn EmitMarkup)
    }

    fn element_ref_class(&self, key: TypeKey<'_>) -> Option<(&dyn EmitClass, bool)> {
        let text = text_of(key)?;
        match self.conversions.element_refs.get(text) {
            Some((position, nullable)) => Some((self.class_at(*position)?, *nullable)),
            None => {
                if text.contains("ElementRef<") && self.unread(&["element_ref"]) {
                    self.cannot_answer(format!("whether `{text}` is a registered element reference: a crate registers element references the scanner did not read"));
                }
                None
            }
        }
    }

    fn primitive_type_name(&self, key: TypeKey<'_>) -> Option<&'static str> {
        let text = text_of(key)?;
        PRIMITIVES.iter().find(|(primitive, _)| *primitive == text).map(|(_, name)| *name)
    }

    fn is_assignable(&self, from: TypeKey<'_>, to: TypeKey<'_>) -> bool {
        let (Some(from), Some(to)) = (text_of(from), text_of(to)) else { return false };
        if self.stated_assignable(from, to) {
            return true;
        }
        if let Err(reason) = self.decided_not_assignable(from, to, 0) {
            self.cannot_answer(format!("whether a value of `{from}` is a value of `{to}`: {reason}"));
        }
        false
    }

    fn null_converts_to(&self, target: TypeKey<'_>) -> bool {
        let Some(target) = text_of(target) else { return false };
        if self.known.get(&Known::Object).is_some_and(|(object, _)| object == target) || self.conversions.nullable.contains_key(target) {
            return true;
        }
        // Only an `Option<_>` is a nullable form.
        if target.starts_with("Option<") && self.unread(&["nullable", "reference", "object", "interface", "element_ref"]) {
            self.cannot_answer(format!("whether `{target}` is a registered nullable form: a crate registers nullable forms the scanner did not read"));
        }
        false
    }

    fn nullable_inner(&self, key: TypeKey<'_>) -> Option<TypeKey<'_>> {
        let text = text_of(key)?;
        match self.conversions.nullable.get(text) {
            Some(inner) => self.key_of(inner, "the type a nullable form holds"),
            None => {
                if text.starts_with("Option<") && self.unread(&["nullable", "reference", "object", "interface", "element_ref"]) {
                    self.cannot_answer(format!("whether `{text}` is a registered nullable form: a crate registers nullable forms the scanner did not read"));
                }
                None
            }
        }
    }

    fn is_styled_element(&self, class: &dyn EmitClass) -> bool {
        match (class.as_any().downcast_ref::<ModelClass>(), self.styled_element) {
            (Some(class), Some(styled_element)) => class.chain.contains(&styled_element),
            _ => false,
        }
    }

    fn has_class_document(&self, class: &dyn EmitClass) -> bool {
        self.class_documents.contains(&class.full_name())
    }

    fn framework_path(&self, type_: FrameworkType) -> Option<String> {
        let full_name = match type_ {
            FrameworkType::Setter => "FerroUI.Styling.Setter",
            FrameworkType::SetterBase => "FerroUI.Styling.SetterBase",
            FrameworkType::StyleBase => "FerroUI.Styling.StyleBase",
        };
        public_path(self.set().type_at(self.system.named(full_name)?))
    }

    fn method(&self, method: &dyn IXamlMethod) -> Option<MethodInfo<'_>> {
        let model = method.as_any().downcast_ref::<ModelMethod>()?;
        let declaring = model.declaring_type.upgrade();
        let member = || format!("{}.{}", declaring.as_ref().map_or_else(String::new, |declaring| declaring.full_name()), model.name);
        let rust = model.rust();
        let returns = |text: &Option<String>| text.as_ref().and_then(|text| self.key_of(text, &format!("the type {} returns", member())));
        let declared = rust.declared.as_ref().and_then(|declared| {
            let (kind, returns) = match declared {
                DeclaredRust::Getter(type_) => (DeclaredKind::Getter, returns(&Some(type_.clone()))),
                DeclaredRust::StaticGetter(type_) => (DeclaredKind::StaticGetter, returns(&Some(type_.clone()))),
                DeclaredRust::Setter => (DeclaredKind::Setter, None),
                DeclaredRust::StaticSetter => (DeclaredKind::StaticSetter, None),
                DeclaredRust::Method(type_) => (DeclaredKind::Method, returns(type_)),
                // `Parse` returns a value of the type.
                DeclaredRust::Parse => {
                    let value = declaring.as_ref().and_then(|declaring| self.metadata_of(&**declaring)).and_then(|markup| markup.value());
                    (DeclaredKind::Parse, value)
                }
                DeclaredRust::Constructor | DeclaredRust::Field(_) | DeclaredRust::EnumMember => return None,
            };
            Some(DeclaredMethod { kind, emit: Self::emit_function(model.source()), returns })
        });
        // The accessors of a registered property and the members of the runtime library
        // that have an implementation are built by the type system.
        let built = match model.source() {
            MemberSource::Registered { .. } => true,
            MemberSource::TypeSystem => !model.is_abstract(),
            _ => false,
        };
        Some(MethodInfo::new(
            model.name.clone(),
            model.is_static,
            declaring.clone().map(|declaring| declaring as Rc<dyn IXamlType>),
            model.parameters.clone(),
            rust.parameters.iter().map(|parameter| parameter.as_ref().and_then(|text| self.handle(text, &format!("a parameter of {}", member())))).collect(),
            declared,
            built,
        ))
    }

    fn constructor(&self, constructor: &dyn IXamlConstructor) -> Option<ConstructorInfo<'_>> {
        let model = constructor.as_any().downcast_ref::<ModelConstructor>()?;
        let declaring = model.declaring_type.upgrade();
        let what = format!("a parameter of the constructor of {}", declaring.as_ref().map_or_else(String::new, |declaring| declaring.full_name()));
        let rust = model.rust();
        Some(ConstructorInfo {
            declaring_type: declaring.map(|declaring| declaring as Rc<dyn IXamlType>),
            parameters: model.parameters.clone(),
            parameter_handles: rust.parameters.iter().map(|parameter| parameter.as_ref().and_then(|text| self.handle(text, &what))).collect(),
            declared: matches!(rust.declared, Some(DeclaredRust::Constructor)).then(|| Self::emit_function(model.source())),
            built: matches!(model.source(), MemberSource::DefaultConstructor { .. }),
        })
    }

    fn field(&self, field: &dyn IXamlField) -> Option<FieldInfo<'_>> {
        let model = field.as_any().downcast_ref::<ModelField>()?;
        let rust = model.rust();
        let value = match &rust.declared {
            Some(DeclaredRust::Field(type_)) => match self.key_of(type_, &format!("the type of the field {}", model.name)) {
                Some(type_) => FieldValue::Declared { name: model.name.clone(), emit: Self::emit_function(model.source()), type_ },
                None => FieldValue::Other,
            },
            Some(DeclaredRust::EnumMember) => FieldValue::EnumMember,
            _ => FieldValue::Other,
        };
        Some(FieldInfo {
            declaring_type: model.declaring_type.upgrade().map(|declaring| declaring as Rc<dyn IXamlType>),
            property: rust.registered.and_then(|registered| self.properties.get(&registered)).map(|property| property as &dyn EmitProperty),
            value,
        })
    }

    fn property_definition(&self, property: &dyn EmitProperty, preferred: Option<&dyn EmitClass>) -> Result<String, String> {
        let property = property.as_any().downcast_ref::<ModelEmitProperty>().ok_or_else(|| "the property is not one of the build-time type system".to_string())?;
        let set = self.set();
        // The type the property was resolved on and its base classes, then the type that
        // registered it and its base classes.
        let mut types: Vec<Position> = Vec::new();
        let preferred = preferred.and_then(|class| class.as_any().downcast_ref::<ModelClass>()).map(|class| class.position);
        for start in preferred.into_iter().chain(property.owner) {
            let chain = self.classes.get(&start).map_or_else(|| vec![start], |class| class.chain.clone());
            for position in chain {
                if !types.contains(&position) {
                    types.push(position);
                }
            }
        }
        let mut any = false;
        for position in types {
            let listed = set.type_at(position);
            for (index, registered) in listed.registered.iter().enumerate() {
                if self.properties.get(&(position, index)).map(|known| known.identity) != Some(property.identity) {
                    continue;
                }
                any = true;
                if registered.visibility != "pub" {
                    continue;
                }
                if let Some(path) = accessor_type(set, listed, registered).and_then(public_path) {
                    return Ok(format!("{path}::{}()", registered.accessor));
                }
            }
        }
        Err(match any {
            false => format!("no accessor of the property {} is recorded", property.name),
            true => format!("no accessor of the property {} is public and declared by a type with a public Rust path", property.name),
        })
    }

    fn take_unanswered(&self) -> Vec<String> {
        std::mem::take(&mut *self.unanswered.borrow_mut())
    }
}
