//! The run-time type system: [`IXamlTypeSystem`] over the process-wide
//! tables of classes ([`TypeInfo`]), markup metadata ([`MarkupType`]) and
//! assemblies ([`MarkupAssembly`]). The equivalent of the reflection based
//! type system of the managed original.

use std::any::TypeId;
use std::cell::{OnceCell, RefCell};
use std::collections::{HashMap, HashSet};
use std::rc::{Rc, Weak};

use ferroui_base::data::core::{ValueType, ValueTypes};
use ferroui_base::data::BindingPriority;
use ferroui_base::metadata::{
    attributes, MarkupAssembly, MarkupAttribute, MarkupAttributeValue, MarkupInvokeError, MarkupType, MarkupTypeKind,
    MarkupValue, TypeOf,
};
use ferroui_base::{BoxedValue, FerroObject, FerroProperty, FerroPropertyRegistry, Ref, TypeInfo};
use xamlx::exceptions::{XamlError, XamlResult};
use xamlx::type_system::{
    IXamlAssembly, IXamlCustomAttribute, IXamlType, IXamlTypeSystem, XamlPseudoType, XamlTypeWellKnownTypes,
    XamlValue,
};

use super::core_types;
use super::runtime_type::{
    DeclaredMember,
    substitute, to_exact, type_key, RuntimeAssembly, RuntimeConstructor, RuntimeCustomAttribute, RuntimeEvent,
    RuntimeField, RuntimeFieldValue, RuntimeInvoker, RuntimeMembers, RuntimeMethod, RuntimeProperty, RuntimeType,
    RuntimeTypeKind, RuntimeTypeOrigin, RuntimeTypeSpec,
};
use super::values::{to_untyped, RuntimeTypeValue};

/// The namespace of the attribute types the metadata attributes are
/// projected to.
pub const METADATA_NAMESPACE: &str = "FerroUI.Metadata";
/// The namespace of the attribute types of control metadata.
pub const CONTROLS_METADATA_NAMESPACE: &str = "FerroUI.Controls.Metadata";
/// The namespace of the property definition types.
pub const PROPERTY_NAMESPACE: &str = "FerroUI";

const KNOWN_ATTRIBUTES: &[&str] = &[
    "XmlnsDefinition",
    "XmlnsPrefix",
    attributes::CONTENT,
    attributes::TEMPLATE_CONTENT,
    attributes::DEPENDS_ON,
    attributes::ASSIGN_BINDING,
    attributes::USABLE_DURING_INITIALIZATION,
    attributes::WHITESPACE_SIGNIFICANT_COLLECTION,
    attributes::TRIM_SURROUNDING_WHITESPACE,
    attributes::CONTROL_TEMPLATE_SCOPE,
    attributes::DATA_TYPE,
    attributes::INHERIT_DATA_TYPE_FROM,
    attributes::INHERIT_DATA_TYPE_FROM_ITEMS,
    attributes::RESOLVE_BY_NAME,
    attributes::MARKUP_EXTENSION_OPTION,
    attributes::MARKUP_EXTENSION_DEFAULT_OPTION,
    attributes::FERRO_LIST,
    attributes::TEMPLATE_PART,
    attributes::PSEUDO_CLASSES,
    attributes::UNSTABLE,
    attributes::PRIVATE_API,
    attributes::NOT_CLIENT_IMPLEMENTABLE,
    attributes::CONSTRUCTOR_ARGUMENT,
];

/// The full name of the attribute type a metadata attribute named `name`
/// is projected to.
pub fn attribute_type_name(name: &str) -> String {
    match name {
        "Obsolete" => "System.ObsoleteAttribute".to_string(),
        "TypeConverter" => "System.ComponentModel.TypeConverterAttribute".to_string(),
        "DefaultMember" => "System.Reflection.DefaultMemberAttribute".to_string(),
        "TemplatePart" | "PseudoClasses" => format!("{CONTROLS_METADATA_NAMESPACE}.{name}Attribute"),
        // Declared next to the bindings, where the compiler looks it up.
        "AssignBinding" => format!("FerroUI.Data.{name}Attribute"),
        // Declared with the controls (`FerroUI.Controls.ResolveByNameAttribute`), where the
        // resolve-by-name replacer looks it up.
        "ResolveByName" => format!("FerroUI.Controls.{name}Attribute"),
        _ => format!("{METADATA_NAMESPACE}.{name}Attribute"),
    }
}

type GenericIndex = (usize, HashMap<String, Vec<&'static MarkupType>>);

/// The run-time type system.
///
/// Types are looked up in the registries on demand and projected lazily:
/// creating the type system costs the closed table of runtime library types,
/// a type costs its name until one of its members is asked for.
pub struct RuntimeTypeSystem {
    this: Weak<RuntimeTypeSystem>,
    types: RefCell<HashMap<String, Rc<RuntimeType>>>,
    by_class: RefCell<HashMap<usize, Rc<RuntimeType>>>,
    by_markup: RefCell<HashMap<usize, Rc<RuntimeType>>>,
    by_handle: RefCell<HashMap<TypeId, Rc<dyn IXamlType>>>,
    assemblies: RefCell<Vec<Rc<RuntimeAssembly>>>,
    /// The definitions whose instantiations need not be registered.
    instantiable: RefCell<HashSet<String>>,
    generic_index: RefCell<GenericIndex>,
    well_known_types: OnceCell<Rc<XamlTypeWellKnownTypes>>,
    /// Guards the lazy definition of the property definition types against re-entry.
    defining_property_types: std::cell::Cell<bool>,
}

/// The generic definition of the dictionary of the framework.
const DICTIONARY_DEFINITION: &str = "FerroUI.Collections.FerroDictionary`2";

impl RuntimeTypeSystem {
    /// Creates a type system over the registries. Types registered later
    /// are found too.
    pub fn new() -> Rc<Self> {
        // Class references are passed to members as nullable values.
        ValueTypes::register_nullable::<&'static TypeInfo>();
        ValueTypes::register_nullable::<RuntimeTypeValue>();
        let system = Rc::new_cyclic(|this| RuntimeTypeSystem {
            this: this.clone(),
            types: RefCell::new(HashMap::new()),
            by_class: RefCell::new(HashMap::new()),
            by_markup: RefCell::new(HashMap::new()),
            by_handle: RefCell::new(HashMap::new()),
            assemblies: RefCell::new(Vec::new()),
            instantiable: RefCell::new(HashSet::new()),
            generic_index: RefCell::new((usize::MAX, HashMap::new())),
            well_known_types: OnceCell::new(),
            defining_property_types: std::cell::Cell::new(false),
        });
        core_types::define_core_types(&system);
        system
    }

    /// This type system as a type system handle.
    pub fn as_type_system(&self) -> Rc<dyn IXamlTypeSystem> {
        match self.this.upgrade() {
            Some(this) => this,
            None => unreachable!("a type system is only used through its handle"),
        }
    }

    pub(crate) fn weak(&self) -> Weak<RuntimeTypeSystem> {
        self.this.clone()
    }

    /// The type named `full_name`, or the unknown pseudo type.
    pub fn get(&self, full_name: &str) -> Rc<dyn IXamlType> {
        match self.find_runtime_type(full_name) {
            Some(found) => found,
            None => XamlPseudoType::unknown(),
        }
    }

    // --- assemblies ---------------------------------------------------------

    pub(crate) fn core_assembly(&self) -> Rc<RuntimeAssembly> {
        if let Some(core) = self.assemblies.borrow().first() {
            return core.clone();
        }
        let core = Rc::new(RuntimeAssembly {
            system: self.this.clone(),
            name: "System.Runtime".to_string(),
            aliases: vec!["mscorlib", "netstandard", "System.Private.CoreLib"],
            crate_name: None,
            attributes: RefCell::new(Vec::new()),
        });
        self.assemblies.borrow_mut().push(core.clone());
        core
    }

    /// Creates the assemblies of the crates registered since the last call.
    fn sync_assemblies(&self) {
        let _ = self.core_assembly();
        for registered in MarkupAssembly::registered_assemblies() {
            let known = self
                .assemblies
                .borrow()
                .iter()
                .any(|a| a.name == registered.name && a.crate_name.as_deref() == Some(registered.crate_name));
            if known {
                continue;
            }
            let assembly = Rc::new(RuntimeAssembly {
                system: self.this.clone(),
                name: registered.name.to_string(),
                aliases: Vec::new(),
                crate_name: Some(registered.crate_name.to_string()),
                attributes: RefCell::new(Vec::new()),
            });
            self.assemblies.borrow_mut().push(assembly.clone());
            let xmlns_definition = self.attribute_type("XmlnsDefinition");
            let attributes: Vec<Rc<dyn IXamlCustomAttribute>> = registered
                .xmlns_definitions
                .iter()
                .map(|definition| {
                    RuntimeCustomAttribute::new(
                        xmlns_definition.clone(),
                        vec![
                            XamlValue::String(definition.xml_namespace.to_string()),
                            XamlValue::String(definition.namespace.to_string()),
                        ],
                        Vec::new(),
                    )
                })
                .collect();
            *assembly.attributes.borrow_mut() = attributes;
        }
    }

    /// The assembly of the crate that declares the module `module_path`:
    /// the first registered assembly of the crate, or an assembly named
    /// after the crate when the crate registered none.
    fn assembly_of_module(&self, module_path: &str) -> Rc<RuntimeAssembly> {
        self.sync_assemblies();
        let crate_name = module_path.split("::").next().unwrap_or(module_path);
        if crate_name.is_empty() {
            return self.core_assembly();
        }
        let found = self.assemblies.borrow().iter().find(|a| a.crate_name.as_deref() == Some(crate_name)).cloned();
        if let Some(found) = found {
            return found;
        }
        let assembly = Rc::new(RuntimeAssembly {
            system: self.this.clone(),
            name: crate_name.to_string(),
            aliases: Vec::new(),
            crate_name: Some(crate_name.to_string()),
            attributes: RefCell::new(Vec::new()),
        });
        self.assemblies.borrow_mut().push(assembly.clone());
        assembly
    }

    /// The assembly the synthetic framework types (attribute and property
    /// definition types) belong to.
    fn framework_assembly(&self) -> Rc<RuntimeAssembly> {
        self.sync_assemblies();
        let found = self.assemblies.borrow().iter().find(|a| a.name == "FerroUI.Base").cloned();
        found.unwrap_or_else(|| self.core_assembly())
    }

    pub(crate) fn type_belongs_to(&self, type_: &RuntimeType, assembly: &RuntimeAssembly) -> bool {
        match type_.runtime_assembly() {
            Some(own) => {
                std::ptr::eq(&*own, assembly)
                    || (own.crate_name.is_some() && own.crate_name == assembly.crate_name)
            }
            None => false,
        }
    }

    // --- type shells --------------------------------------------------------

    fn insert(&self, type_: &Rc<RuntimeType>, assembly: &Rc<RuntimeAssembly>) {
        type_.set_assembly(assembly);
        // The first type of a name (or the first registered instantiation) is the type.
        self.types.borrow_mut().entry(type_.key().to_string()).or_insert_with(|| type_.clone());
    }

    fn unique_key(&self, full_name: String, namespace: &str, module_path: &str, name: &str) -> String {
        if namespace.is_empty() || self.types.borrow().contains_key(&full_name) {
            format!("rust:{module_path}::{name}")
        } else {
            full_name
        }
    }

    /// Defines a type of the type system itself.
    pub(crate) fn define_synthetic(
        &self,
        assembly: &Rc<RuntimeAssembly>,
        namespace: &str,
        name: &str,
        kind: RuntimeTypeKind,
        generic_parameters: &[&str],
        handles: Vec<ValueType>,
        init: impl FnOnce(&mut MemberBuilder) + 'static,
    ) -> Rc<RuntimeType> {
        let key = if namespace.is_empty() { name.to_string() } else { format!("{namespace}.{name}") };
        let mut spec = RuntimeTypeSpec::new(key, namespace, name, kind, RuntimeTypeOrigin::Synthetic);
        spec.generic_parameter_names = generic_parameters.iter().map(|p| p.to_string()).collect();
        spec.handles = handles.clone();
        let system = self.this.clone();
        spec.init = Some(Box::new(move |type_: &Rc<RuntimeType>| {
            let Some(system) = system.upgrade() else { return RuntimeMembers::default() };
            let mut builder = MemberBuilder { system, type_: type_.clone(), members: RuntimeMembers::default(), projected: false };
            if kind == RuntimeTypeKind::Class && !(namespace_is(type_, "System") && type_.name() == "Object") {
                builder.members.base_type = builder.system.find_type("System.Object");
            }
            // One source: a runtime library type the framework declares metadata for
            // (`ISupportInitialize`, `IServiceProvider`, `Uri`, ...) has the members of
            // that metadata; the hand-written members only fill in what it lacks.
            let type_namespace = type_.namespace().unwrap_or_default();
            if type_namespace.starts_with("System") && type_.generic_parameter_types().is_empty() {
                builder.project_metadata(&type_namespace, &type_.name());
            }
            init(&mut builder);
            builder.members
        }));
        let type_ = RuntimeType::create(&self.this, spec);
        self.insert(&type_, assembly);
        if !generic_parameters.is_empty() {
            self.instantiable.borrow_mut().insert(type_.key().to_string());
        }
        let as_type: Rc<dyn IXamlType> = type_.clone();
        for handle in handles {
            self.by_handle.borrow_mut().insert(handle.id(), as_type.clone());
        }
        type_
    }

    /// States that values of the Rust type `handle` are values of `type_`.
    pub(crate) fn map_handle(&self, handle: ValueType, type_: &Rc<dyn IXamlType>) {
        if let Some(runtime) = type_.as_any().downcast_ref::<RuntimeType>() {
            runtime.add_handle(handle);
        }
        self.by_handle.borrow_mut().insert(handle.id(), type_.clone());
    }

    /// The type of a class of the object model.
    pub fn type_of_class(&self, type_info: &'static TypeInfo) -> Rc<dyn IXamlType> {
        self.runtime_type_of_class(type_info)
    }

    fn runtime_type_of_class(&self, type_info: &'static TypeInfo) -> Rc<RuntimeType> {
        let address = type_info as *const TypeInfo as usize;
        if let Some(found) = self.by_class.borrow().get(&address) {
            return found.clone();
        }
        let namespace = type_info.namespace();
        let key = self.unique_key(type_info.full_name(), namespace, type_info.module_path(), type_info.name());
        let mut spec = RuntimeTypeSpec::new(
            key,
            namespace,
            type_info.name(),
            RuntimeTypeKind::Class,
            RuntimeTypeOrigin::Class(type_info),
        );
        if let (Some(handle), Some(nullable)) = (type_info.handle(), type_info.nullable_handle()) {
            spec.handles = vec![ValueType::new(handle, type_info.name()), ValueType::new(nullable, type_info.name())];
        }
        let type_ = RuntimeType::create(&self.this, spec);
        self.by_class.borrow_mut().insert(address, type_.clone());
        self.insert(&type_, &self.assembly_of_module(type_info.module_path()));
        type_
    }

    /// The type of a type with markup metadata.
    pub fn type_of_markup(&self, markup: &'static MarkupType) -> Rc<dyn IXamlType> {
        self.runtime_type_of_markup(markup)
    }

    fn runtime_type_of_markup(&self, markup: &'static MarkupType) -> Rc<RuntimeType> {
        if let Some(type_info) = markup.type_info {
            return self.runtime_type_of_class(type_info());
        }
        let address = markup as *const MarkupType as usize;
        if let Some(found) = self.by_markup.borrow().get(&address) {
            return found.clone();
        }
        let namespace = markup.namespace();
        // Metadata of a runtime library type the type system defines itself is that type.
        if markup.generic.is_none() && namespace.starts_with("System") {
            let existing = self.types.borrow().get(&markup.full_name()).cloned();
            if let Some(type_) = existing.filter(|t| matches!(t.origin(), RuntimeTypeOrigin::Synthetic)) {
                for handle in markup.handles {
                    let handle = handle();
                    type_.add_handle(handle);
                    self.by_handle.borrow_mut().insert(handle.id(), type_.clone() as Rc<dyn IXamlType>);
                }
                self.by_markup.borrow_mut().insert(address, type_.clone());
                return type_;
            }
        }
        // A static type of the object model (the owner of attached properties) that
        // also is a type with values of its own (`XamlSourceInfo`) is one type: its
        // class with the members and handles of the metadata of the same name.
        if markup.generic.is_none() {
            if let Some(class) = TypeInfo::find(namespace, markup.name).filter(|c| c.module_path() == markup.module_path) {
                let type_ = self.runtime_type_of_class(class);
                for handle in markup.handles {
                    let handle = handle();
                    type_.add_handle(handle);
                    self.by_handle.borrow_mut().insert(handle.id(), type_.clone() as Rc<dyn IXamlType>);
                }
                self.by_markup.borrow_mut().insert(address, type_.clone());
                return type_;
            }
        }
        let kind = match markup.kind {
            MarkupTypeKind::Class | MarkupTypeKind::Static => RuntimeTypeKind::Class,
            MarkupTypeKind::Struct => RuntimeTypeKind::Struct,
            MarkupTypeKind::Enum => RuntimeTypeKind::Enum,
            MarkupTypeKind::Interface => RuntimeTypeKind::Interface,
        };
        let mut spec = match markup.generic {
            Some(generic) => {
                let definition = self.definition_for(namespace, generic.definition, kind, markup.module_path);
                let arguments: Vec<Rc<dyn IXamlType>> = generic.arguments.iter().map(|a| self.resolve(a())).collect();
                let key = instantiation_key(&definition, &arguments);
                let mut spec =
                    RuntimeTypeSpec::new(key, namespace, generic.definition, kind, RuntimeTypeOrigin::Markup(markup));
                spec.generic_definition = Some(definition);
                spec.generic_arguments = arguments;
                spec
            }
            None => {
                let key = self.unique_key(markup.full_name(), namespace, markup.module_path, markup.name);
                RuntimeTypeSpec::new(key, namespace, markup.name, kind, RuntimeTypeOrigin::Markup(markup))
            }
        };
        spec.handles = markup.handles.iter().map(|h| h()).collect();
        let type_ = RuntimeType::create(&self.this, spec);
        self.by_markup.borrow_mut().insert(address, type_.clone());
        self.insert(&type_, &self.assembly_of_module(markup.module_path));
        type_
    }

    /// The type of a Rust type without metadata: assignable only to itself
    /// and to `System.Object`.
    fn opaque(&self, handle: ValueType) -> Rc<dyn IXamlType> {
        let key = format!("rust:{}", handle.name());
        if let Some(found) = self.types.borrow().get(&key) {
            return found.clone();
        }
        let mut spec =
            RuntimeTypeSpec::new(key, "", handle.name(), RuntimeTypeKind::Class, RuntimeTypeOrigin::Opaque);
        spec.handles = vec![handle];
        let type_ = RuntimeType::create(&self.this, spec);
        self.insert(&type_, &self.core_assembly());
        type_
    }

    /// The generic definition named `name` in `namespace`: a definition of
    /// the type system, or one synthesised for registered instantiations.
    fn definition_for(&self, namespace: &str, name: &str, kind: RuntimeTypeKind, module_path: &str) -> Rc<RuntimeType> {
        let key = if namespace.is_empty() { name.to_string() } else { format!("{namespace}.{name}") };
        if let Some(found) = self.types.borrow().get(&key) {
            return found.clone();
        }
        let arity = name.rsplit_once('`').and_then(|(_, arity)| arity.parse::<usize>().ok()).unwrap_or(1);
        let mut spec = RuntimeTypeSpec::new(key, namespace, name, kind, RuntimeTypeOrigin::Synthetic);
        spec.generic_parameter_names = (1..=arity).map(|i| format!("T{i}")).collect();
        let system = self.this.clone();
        spec.init = Some(Box::new(move |_| RuntimeMembers {
            base_type: match kind {
                RuntimeTypeKind::Class => system.upgrade().and_then(|s| s.find_type("System.Object")),
                _ => None,
            },
            ..RuntimeMembers::default()
        }));
        let type_ = RuntimeType::create(&self.this, spec);
        self.insert(&type_, &self.assembly_of_module(module_path));
        type_
    }

    fn with_generic_index<R>(&self, f: impl FnOnce(&HashMap<String, Vec<&'static MarkupType>>) -> R) -> R {
        let registered = MarkupType::registered_types();
        if self.generic_index.borrow().0 != registered.len() {
            let mut index: HashMap<String, Vec<&'static MarkupType>> = HashMap::new();
            for markup in &registered {
                if let Some(generic) = markup.generic {
                    let namespace = markup.namespace();
                    let key = if namespace.is_empty() {
                        generic.definition.to_string()
                    } else {
                        format!("{namespace}.{}", generic.definition)
                    };
                    index.entry(key).or_default().push(markup);
                }
            }
            *self.generic_index.borrow_mut() = (registered.len(), index);
        }
        f(&self.generic_index.borrow().1)
    }

    /// `definition.MakeGenericType(arguments)`: the registered instantiation,
    /// or a synthetic one for a definition of the type system.
    pub(crate) fn instantiate(
        &self,
        definition: &Rc<RuntimeType>,
        arguments: &[Rc<dyn IXamlType>],
    ) -> XamlResult<Rc<dyn IXamlType>> {
        let key = instantiation_key(definition, arguments);
        if let Some(found) = self.types.borrow().get(&key) {
            return Ok(found.clone());
        }
        // The list converter is synthesized for every element type.
        if definition.key() == super::list_converter::DEFINITION && arguments.len() == 1 {
            let mut spec = RuntimeTypeSpec::new(
                key,
                &definition.namespace().unwrap_or_default(),
                &definition.name(),
                RuntimeTypeKind::Class,
                RuntimeTypeOrigin::Synthetic,
            );
            spec.generic_definition = Some(definition.clone());
            spec.generic_arguments = arguments.to_vec();
            let system = self.this.clone();
            let element_type = arguments[0].clone();
            spec.init = Some(Box::new(move |type_: &Rc<RuntimeType>| {
                let Some(system) = system.upgrade() else { return RuntimeMembers::default() };
                let mut builder = MemberBuilder { system, type_: type_.clone(), members: RuntimeMembers::default(), projected: false };
                super::list_converter::members(&mut builder, element_type)
            }));
            let type_ = RuntimeType::create(&self.this, spec);
            let assembly = definition.runtime_assembly().unwrap_or_else(|| self.core_assembly());
            self.insert(&type_, &assembly);
            return Ok(type_);
        }
        let candidates = self.with_generic_index(|index| index.get(definition.key()).cloned().unwrap_or_default());
        for candidate in candidates {
            let _ = self.runtime_type_of_markup(candidate);
        }
        if let Some(found) = self.types.borrow().get(&key) {
            return Ok(found.clone());
        }
        if !self.instantiable.borrow().contains(definition.key()) {
            return Err(XamlError::type_system_exception(format!(
                "The instantiation {}[{}] is not registered: the run-time type system only knows the instantiations of a generic type that declare markup metadata",
                definition.full_name(),
                arguments.iter().map(|a| a.full_name()).collect::<Vec<_>>().join(",")
            )));
        }
        let mut spec = RuntimeTypeSpec::new(
            key,
            &definition.namespace().unwrap_or_default(),
            &definition.name(),
            definition.kind(),
            RuntimeTypeOrigin::Instantiation,
        );
        spec.generic_definition = Some(definition.clone());
        spec.generic_arguments = arguments.to_vec();
        let type_ = RuntimeType::create(&self.this, spec);
        let assembly = definition.runtime_assembly().unwrap_or_else(|| self.core_assembly());
        self.insert(&type_, &assembly);
        Ok(type_)
    }

    /// The open instantiation of a generic definition whose instantiations
    /// are otherwise the registered ones only: the definition applied to
    /// type parameters (of a generic method), deriving from `base`.
    pub(crate) fn instantiate_open(
        &self,
        definition: &Rc<RuntimeType>,
        arguments: &[Rc<dyn IXamlType>],
        base: Rc<dyn IXamlType>,
    ) -> Rc<dyn IXamlType> {
        let key = instantiation_key(definition, arguments);
        if let Some(found) = self.types.borrow().get(&key) {
            return found.clone();
        }
        let mut spec = RuntimeTypeSpec::new(
            key,
            &definition.namespace().unwrap_or_default(),
            &definition.name(),
            definition.kind(),
            RuntimeTypeOrigin::Synthetic,
        );
        spec.generic_definition = Some(definition.clone());
        spec.generic_arguments = arguments.to_vec();
        spec.init = Some(Box::new(move |_| RuntimeMembers { base_type: Some(base), ..RuntimeMembers::default() }));
        let type_ = RuntimeType::create(&self.this, spec);
        let assembly = definition.runtime_assembly().unwrap_or_else(|| self.core_assembly());
        self.insert(&type_, &assembly);
        type_
    }

    pub(crate) fn create_array_type(&self, element: &Rc<RuntimeType>, dimensions: i32) -> Rc<RuntimeType> {
        let suffix = format!("[{}]", ",".repeat((dimensions - 1) as usize));
        let mut spec = RuntimeTypeSpec::new(
            format!("{}{suffix}", element.key()),
            &element.namespace().unwrap_or_default(),
            &format!("{}{suffix}", element.name()),
            RuntimeTypeKind::Class,
            RuntimeTypeOrigin::Array,
        );
        let element_type: Rc<dyn IXamlType> = element.clone();
        spec.array_element_type = Some(element_type.clone());
        let system = self.this.clone();
        spec.init = Some(Box::new(move |_| {
            let mut members = RuntimeMembers::default();
            let Some(system) = system.upgrade() else { return members };
            members.base_type = system.find_type("System.Array");
            if dimensions == 1 {
                // A vector implements the generic list interfaces of its element type.
                for name in ["System.Collections.Generic.IList`1", "System.Collections.Generic.IReadOnlyList`1"] {
                    if let Some(Ok(interface)) =
                        system.find_type(name).map(|i| i.make_generic_type(std::slice::from_ref(&element_type)))
                    {
                        members.interfaces.push(interface);
                    }
                }
            }
            members.interfaces.extend(system.find_type("System.Collections.IList"));
            members
        }));
        let type_ = RuntimeType::create(&self.this, spec);
        let assembly = element.runtime_assembly().unwrap_or_else(|| self.core_assembly());
        self.insert(&type_, &assembly);
        type_
    }

    // --- lookups ------------------------------------------------------------

    /// The type that values of the Rust type `handle` are values of: a
    /// runtime library type, a class (for its handle and nullable handle),
    /// a type with markup metadata (the nullable form of a value type is
    /// `System.Nullable<T>`), or else an opaque type named after the Rust
    /// type.
    pub fn resolve(&self, handle: ValueType) -> Rc<dyn IXamlType> {
        if let Some(found) = self.by_handle.borrow().get(&handle.id()) {
            return found.clone();
        }
        let found: Option<Rc<dyn IXamlType>> = if let Some((type_info, _)) = TypeInfo::find_by_handle(handle.id()) {
            Some(self.runtime_type_of_class(type_info))
        } else if let Some(markup) = MarkupType::find_by_handle(handle.id()) {
            Some(self.runtime_type_of_markup(markup))
        } else if let Some(markup) = MarkupType::find_by_nullable_handle(handle.id()) {
            let inner: Rc<dyn IXamlType> = self.runtime_type_of_markup(markup);
            self.nullable_of(&inner).inspect(|nullable| {
                if let Some(runtime) = nullable.as_any().downcast_ref::<RuntimeType>() {
                    runtime.add_handle(handle);
                }
            })
        } else if let Some(element) = super::values::bindable_array_element(handle.id()) {
            // An array bindings index into: `T[]` of its element type.
            self.resolve(element).make_array_type(1).ok()
        } else {
            self.resolve_optional_reference(handle)
        };
        match found {
            Some(found) => {
                self.by_handle.borrow_mut().insert(handle.id(), found.clone());
                found
            }
            None => self.opaque(handle),
        }
    }

    /// `Option<T>` of a reference type whose metadata lists only `T` among
    /// its handles: a nullable reference is the type itself.
    fn resolve_optional_reference(&self, handle: ValueType) -> Option<Rc<dyn IXamlType>> {
        let inner = handle.name().strip_prefix("core::option::Option<")?.strip_suffix('>')?;
        let markup = MarkupType::registered_types().into_iter().find(|markup| {
            matches!(markup.kind, MarkupTypeKind::Class | MarkupTypeKind::Interface)
                && markup.handles.iter().any(|h| h().name() == inner)
        })?;
        let type_ = self.runtime_type_of_markup(markup);
        type_.add_handle(handle);
        Some(type_)
    }

    /// `System.Nullable<T>` of a value type.
    pub fn nullable_of(&self, type_: &Rc<dyn IXamlType>) -> Option<Rc<dyn IXamlType>> {
        self.find_type("System.Nullable`1")?.make_generic_type(std::slice::from_ref(type_)).ok()
    }

    pub(crate) fn find_runtime_type(&self, full_name: &str) -> Option<Rc<RuntimeType>> {
        if let Some(found) = self.types.borrow().get(full_name) {
            return Some(found.clone());
        }
        let (namespace, name) = match full_name.rsplit_once('.') {
            Some((namespace, name)) => (namespace, name),
            None => ("", full_name),
        };
        if let Some(type_info) = TypeInfo::find(namespace, name) {
            return Some(self.runtime_type_of_class(type_info));
        }
        if let Some(markup) = MarkupType::find(namespace, name).filter(|m| m.generic.is_none()) {
            return Some(self.runtime_type_of_markup(markup));
        }
        if name.contains('`') {
            let first = self.with_generic_index(|index| index.get(full_name).and_then(|i| i.first().copied()));
            if let Some(first) = first {
                // The instantiation creates the definition it belongs to.
                return self.runtime_type_of_markup(first).definition().cloned();
            }
        }
        let attribute = name.strip_suffix("Attribute").filter(|attribute| {
            KNOWN_ATTRIBUTES.contains(attribute) && attribute_type_name(attribute) == full_name
        });
        if let Some(attribute) = attribute {
            return self.attribute_type(attribute).as_any().downcast_ref::<RuntimeType>().and_then(RuntimeType::rc);
        }
        // The property definition types are defined on first use; a lookup
        // by name is a use.
        if namespace == PROPERTY_NAMESPACE
            && matches!(
                name,
                "FerroProperty" | "FerroProperty`1" | "StyledProperty`1" | "AttachedProperty`1" | "DirectPropertyBase`1"
                    | "DirectProperty`2"
            )
            && !self.defining_property_types.replace(true)
        {
            core_types::define_property_types(self);
            self.defining_property_types.set(false);
            return self.types.borrow().get(full_name).cloned();
        }
        None
    }

    /// The type of the attribute a metadata attribute named `name` is
    /// projected to; defined by the type system if no such type exists.
    pub fn attribute_type(&self, name: &str) -> Rc<dyn IXamlType> {
        let full_name = attribute_type_name(name);
        if let Some(found) = self.types.borrow().get(&full_name) {
            return found.clone();
        }
        let (namespace, type_name) = full_name.rsplit_once('.').unwrap_or(("", &full_name));
        if let Some(type_info) = TypeInfo::find(namespace, type_name) {
            return self.runtime_type_of_class(type_info);
        }
        if let Some(markup) = MarkupType::find(namespace, type_name) {
            return self.runtime_type_of_markup(markup);
        }
        let assembly = if namespace.starts_with("System") { self.core_assembly() } else { self.framework_assembly() };
        self.define_synthetic(&assembly, namespace, type_name, RuntimeTypeKind::Class, &[], Vec::new(), |b| {
            b.base("System.Attribute");
        })
    }

    /// Projects a metadata attribute.
    pub fn project_attribute(&self, attribute: &MarkupAttribute) -> Rc<dyn IXamlCustomAttribute> {
        let value = |value: &MarkupAttributeValue| self.project_attribute_value(value);
        RuntimeCustomAttribute::new(
            self.attribute_type(attribute.name),
            attribute.arguments.iter().map(value).collect(),
            attribute.properties.iter().map(|(name, v)| (name.to_string(), value(v))).collect(),
        )
    }

    /// Projects an argument of a metadata attribute: an array is an array-valued
    /// argument (`XamlValue::Array`), as reflection reports `new[] { .. }`.
    fn project_attribute_value(&self, value: &MarkupAttributeValue) -> XamlValue {
        match value {
            MarkupAttributeValue::Null => XamlValue::Null,
            MarkupAttributeValue::Bool(value) => XamlValue::Boolean(*value),
            MarkupAttributeValue::Int(value) => match i32::try_from(*value) {
                Ok(value) => XamlValue::Int32(value),
                Err(_) => XamlValue::Int64(*value),
            },
            MarkupAttributeValue::Float(value) => XamlValue::Double(*value),
            MarkupAttributeValue::Str(value) => XamlValue::String(value.to_string()),
            MarkupAttributeValue::Type(type_) => XamlValue::Type(self.resolve(type_())),
            MarkupAttributeValue::Array(items) => {
                XamlValue::Array(items.iter().map(|item| self.project_attribute_value(item)).collect())
            }
        }
    }

    fn project_attributes(&self, attributes: &[MarkupAttribute]) -> Vec<Rc<dyn IXamlCustomAttribute>> {
        attributes.iter().map(|a| self.project_attribute(a)).collect()
    }

    fn marker_attribute(&self, name: &str) -> Rc<dyn IXamlCustomAttribute> {
        RuntimeCustomAttribute::new(self.attribute_type(name), Vec::new(), Vec::new())
    }

    // --- run-time values ----------------------------------------------------

    /// The run-time type of a value: the class of an object, the type the
    /// Rust type of any other value is a handle of.
    pub fn runtime_type_of(&self, value: &BoxedValue) -> Rc<dyn IXamlType> {
        if let Some(converter) = value.downcast_ref::<super::list_converter::RuntimeListConverter>() {
            let definition = self.find_type(super::list_converter::DEFINITION);
            if let Some(Ok(type_)) =
                definition.map(|d| d.make_generic_type(std::slice::from_ref(converter.element_type())))
            {
                return type_;
            }
        }
        if let Some(array) = value.downcast_ref::<super::values::RuntimeArray>() {
            if let Ok(array_type) = array.element_type().make_array_type(1) {
                return array_type;
            }
        }
        if let Some(found) = self.by_handle.borrow().get(&value.value_type_id()) {
            if found.as_any().downcast_ref::<RuntimeType>().is_some_and(|t| t.type_info().is_none()) {
                return found.clone();
            }
        }
        match ValueTypes::as_object(&**value) {
            Some(object) => self.runtime_type_of_class(object.get_type()),
            None => self.resolve(ValueType::of_value(&**value)),
        }
    }

    /// `value is T` (the `isinst` check of the managed runtime): whether the
    /// value is a non-null instance of `type_`. For `System.Nullable<T>` the
    /// value is checked against `T`.
    pub fn is_instance(&self, value: &MarkupValue, type_: &dyn IXamlType) -> bool {
        let Some(value) = value else { return false };
        let runtime_type = self.runtime_type_of(value);
        if type_.is_nullable() {
            return type_.generic_arguments().first().is_some_and(|inner| inner.is_assignable_from(&*runtime_type));
        }
        type_.is_assignable_from(&*runtime_type)
    }

    /// A `System.Type` value for `type_`.
    pub fn type_value(&self, type_: &Rc<dyn IXamlType>) -> BoxedValue {
        Rc::new(RuntimeTypeValue::new(type_.clone()))
    }

    // --- projection ---------------------------------------------------------

    pub(crate) fn project_members(&self, type_: &Rc<RuntimeType>) -> RuntimeMembers {
        let object = || self.find_type("System.Object");
        match type_.origin() {
            RuntimeTypeOrigin::Class(type_info) => self.project_class(type_, type_info),
            RuntimeTypeOrigin::Markup(markup) => self.project_markup_type(type_, markup),
            RuntimeTypeOrigin::Instantiation => self.project_instantiation(type_),
            RuntimeTypeOrigin::Opaque => RuntimeMembers { base_type: object(), ..RuntimeMembers::default() },
            RuntimeTypeOrigin::Synthetic if type_.kind() == RuntimeTypeKind::Class => {
                RuntimeMembers { base_type: object(), ..RuntimeMembers::default() }
            }
            _ => RuntimeMembers::default(),
        }
    }

    fn project_instantiation(&self, type_: &Rc<RuntimeType>) -> RuntimeMembers {
        let Some(definition) = type_.definition().cloned() else { return RuntimeMembers::default() };
        let source = definition.members();
        let parameters = definition.generic_parameter_types();
        let arguments = type_.generic_argument_types();
        let weak = Rc::downgrade(type_);
        let sub = |t: &Rc<dyn IXamlType>| substitute(t, parameters, arguments);
        RuntimeMembers {
            base_type: source.base_type.as_ref().map(sub),
            interfaces: source.interfaces.iter().map(sub).collect(),
            properties: source
                .properties
                .iter()
                .map(|p| {
                    Rc::new(RuntimeProperty {
                        name: p.name.clone(),
                        declaring_type: weak.clone(),
                        property_type: sub(&p.property_type),
                        getter: p.getter.as_ref().map(|m| m.substituted(&weak, parameters, arguments)),
                        setter: p.setter.as_ref().map(|m| m.substituted(&weak, parameters, arguments)),
                        attributes: p.attributes.clone(),
                        ferro_property: p.ferro_property,
                        indexer_parameters: p.indexer_parameters.iter().map(sub).collect(),
                    })
                })
                .collect(),
            events: Vec::new(),
            fields: source
                .fields
                .iter()
                .map(|f| {
                    Rc::new(RuntimeField {
                        system: f.system.clone(),
                        name: f.name.clone(),
                        declaring_type: weak.clone(),
                        field_type: sub(&f.field_type),
                        literal: f.literal.clone(),
                        value: f.value.clone(),
                        attributes: f.attributes.clone(),
                    })
                })
                .collect(),
            methods: source.methods.iter().map(|m| m.substituted(&weak, parameters, arguments)).collect(),
            constructors: source.constructors.iter().map(|c| c.substituted(&weak, parameters, arguments)).collect(),
            attributes: source.attributes.clone(),
            enum_underlying_type: None,
        }
    }

    fn resolve_all(&self, types: &[TypeOf]) -> (Vec<Rc<dyn IXamlType>>, Vec<Option<ValueType>>) {
        let handles: Vec<ValueType> = types.iter().map(|t| t()).collect();
        (handles.iter().map(|h| self.resolve(*h)).collect(), handles.into_iter().map(Some).collect())
    }

    pub(crate) fn method(
        &self,
        type_: &Rc<RuntimeType>,
        name: String,
        is_static: bool,
        return_type: Rc<dyn IXamlType>,
        parameters: (Vec<Rc<dyn IXamlType>>, Vec<Option<ValueType>>),
        invoker: RuntimeInvoker,
        attributes: Vec<Rc<dyn IXamlCustomAttribute>>,
    ) -> Rc<RuntimeMethod> {
        Rc::new(RuntimeMethod {
            system: self.this.clone(),
            name,
            declaring_type: Rc::downgrade(type_),
            is_static,
            return_type,
            parameters: parameters.0,
            parameter_handles: parameters.1,
            invoker,
            attributes,
            generic_parameters: Vec::new(),
            generic_arguments: Vec::new(),
            declared: None,
        })
    }

    pub(crate) fn void(&self) -> Rc<dyn IXamlType> {
        self.get("System.Void")
    }

    /// Projects what markup metadata declares into `members`.
    fn project_markup(&self, type_: &Rc<RuntimeType>, markup: &'static MarkupType, members: &mut RuntimeMembers) {
        let weak = Rc::downgrade(type_);
        let self_type: Rc<dyn IXamlType> = type_.clone();
        for interface in markup.interfaces {
            let interface = self.resolve(interface());
            if !members.interfaces.iter().any(|i| i.equals(&*interface)) {
                members.interfaces.push(interface);
            }
        }
        // The attributes of a constructor parameter are the ones its declaration states
        // (`(property: T [InheritDataTypeFrom(2)]) => ..`); a constructor declared in the
        // positional form states none.
        for constructor in markup.constructors {
            let (parameters, parameter_handles) = self.resolve_all(constructor.parameters);
            let declared = !constructor.parameter_info.is_empty();
            let parameter_attributes: Vec<Vec<Rc<dyn IXamlCustomAttribute>>> = (0..parameters.len())
                .map(|parameter_index| match declared {
                    true => self.project_attributes(constructor.parameter_attributes(parameter_index)),
                    false => Vec::new(),
                })
                .collect();
            members.constructors.push(Rc::new(RuntimeConstructor {
                system: self.this.clone(),
                declaring_type: weak.clone(),
                parameters,
                parameter_handles,
                invoker: RuntimeInvoker::Static(constructor.invoke),
                is_public: true,
                parameter_attributes,
            }));
        }
        for property in markup.properties {
            let handle = (property.type_)();
            let property_type = self.resolve(handle);
            let getter = property.get.map(|get| {
                self.method(
                    type_,
                    format!("get_{}", property.name),
                    false,
                    property_type.clone(),
                    (Vec::new(), Vec::new()),
                    RuntimeInvoker::Static(get),
                    Vec::new(),
                )
                .with_declared(DeclaredMember::Getter(property))
            });
            let setter = property.set.map(|set| {
                self.method(
                    type_,
                    format!("set_{}", property.name),
                    false,
                    self.void(),
                    (vec![property_type.clone()], vec![Some(handle)]),
                    RuntimeInvoker::Static(set),
                    Vec::new(),
                )
                .with_declared(DeclaredMember::Setter(property))
            });
            let mut attributes = self.project_attributes(property.attributes);
            if markup.content_property == Some(property.name) {
                attributes.push(self.marker_attribute(attributes::CONTENT));
            }
            members.methods.extend(getter.iter().cloned());
            members.methods.extend(setter.iter().cloned());
            members.properties.push(Rc::new(RuntimeProperty {
                name: property.name.to_string(),
                declaring_type: weak.clone(),
                property_type,
                getter,
                setter,
                attributes,
                ferro_property: None,
                indexer_parameters: Vec::new(),
            }));
        }
        // An indexer is the default member of its type (`this[..]` compiles to the
        // property `Item` and `[DefaultMember("Item")]` on the type).
        for indexer in markup.indexers {
            let value_handle = (indexer.type_)();
            let value_type = self.resolve(value_handle);
            let (parameters, parameter_handles) = self.resolve_all(indexer.parameters);
            let getter = indexer.get.map(|get| {
                self.method(
                    type_,
                    "get_Item".to_string(),
                    false,
                    value_type.clone(),
                    (parameters.clone(), parameter_handles.clone()),
                    RuntimeInvoker::Static(get),
                    Vec::new(),
                )
            });
            let setter = indexer.set.map(|set| {
                let mut setter_parameters = parameters.clone();
                setter_parameters.push(value_type.clone());
                let mut setter_handles = parameter_handles.clone();
                setter_handles.push(Some(value_handle));
                self.method(
                    type_,
                    "set_Item".to_string(),
                    false,
                    self.void(),
                    (setter_parameters, setter_handles),
                    RuntimeInvoker::Static(set),
                    Vec::new(),
                )
            });
            members.methods.extend(getter.iter().cloned());
            members.methods.extend(setter.iter().cloned());
            members.properties.push(Rc::new(RuntimeProperty {
                name: "Item".to_string(),
                declaring_type: weak.clone(),
                property_type: value_type,
                getter,
                setter,
                attributes: self.project_attributes(indexer.attributes),
                ferro_property: None,
                indexer_parameters: parameters,
            }));
        }
        if !markup.indexers.is_empty() {
            members.attributes.push(RuntimeCustomAttribute::new(
                self.attribute_type("DefaultMember"),
                vec![XamlValue::String("Item".to_string())],
                Vec::new(),
            ));
        }
        for method in markup.methods {
            let return_type = match method.return_type {
                Some(return_type) => self.resolve(return_type()),
                None => self.void(),
            };
            members.methods.push(
                self.method(
                    type_,
                    method.name.to_string(),
                    method.is_static,
                    return_type,
                    self.resolve_all(method.parameters),
                    RuntimeInvoker::Static(method.invoke),
                    self.project_attributes(method.attributes),
                )
                .with_declared(DeclaredMember::Method(method)),
            );
        }
        if let Some(parse) = markup.parse {
            members.methods.push(
                self.method(
                    type_,
                    "Parse".to_string(),
                    true,
                    self_type.clone(),
                    (vec![self.get("System.String")], vec![Some(ValueType::of::<String>())]),
                    RuntimeInvoker::Static(parse),
                    Vec::new(),
                )
                .with_declared(DeclaredMember::Parse(markup)),
            );
        }
        for field in markup.fields {
            members.fields.push(Rc::new(RuntimeField {
                system: self.this.clone(),
                name: field.name.to_string(),
                declaring_type: weak.clone(),
                field_type: self.resolve((field.type_)()),
                literal: None,
                value: RuntimeFieldValue::Getter(field.get),
                attributes: self.project_attributes(field.attributes),
            }));
        }
        // Static properties: `static T Name { get; set; }` (static accessors, no field).
        for property in markup.static_properties {
            let handle = (property.type_)();
            let property_type = self.resolve(handle);
            let getter = property.get.map(|get| {
                self.method(
                    type_,
                    format!("get_{}", property.name),
                    true,
                    property_type.clone(),
                    (Vec::new(), Vec::new()),
                    RuntimeInvoker::Static(get),
                    Vec::new(),
                )
                .with_declared(DeclaredMember::StaticGetter(property))
            });
            let setter = property.set.map(|set| {
                self.method(
                    type_,
                    format!("set_{}", property.name),
                    true,
                    self.void(),
                    (vec![property_type.clone()], vec![Some(handle)]),
                    RuntimeInvoker::Static(set),
                    Vec::new(),
                )
                .with_declared(DeclaredMember::StaticSetter(property))
            });
            members.methods.extend(getter.iter().cloned());
            members.methods.extend(setter.iter().cloned());
            members.properties.push(Rc::new(RuntimeProperty {
                name: property.name.to_string(),
                declaring_type: weak.clone(),
                property_type,
                getter,
                setter,
                attributes: self.project_attributes(property.attributes),
                ferro_property: None,
                indexer_parameters: Vec::new(),
            }));
        }
        for event in markup.events {
            let (arguments, _) = self.resolve_all(event.arguments);
            let handler_type = match arguments.len() {
                0 => self.get("System.Action"),
                // `EventHandler<TArgs>`: the sender and the arguments.
                2 if arguments[0].is("System", "Object") => self
                    .find_type("System.EventHandler`1")
                    .and_then(|handler| handler.make_generic_type(&arguments[1..]).ok())
                    .unwrap_or_else(|| self.get("System.Delegate")),
                count => self
                    .find_type(&format!("System.Action`{count}"))
                    .and_then(|action| action.make_generic_type(&arguments).ok())
                    .unwrap_or_else(|| self.get("System.Delegate")),
            };
            let add = self.method(
                type_,
                format!("add_{}", event.name),
                false,
                self.void(),
                (vec![handler_type], vec![None]),
                RuntimeInvoker::Static(event.add),
                Vec::new(),
            );
            members.methods.push(add.clone());
            members.events.push(Rc::new(RuntimeEvent {
                name: event.name.to_string(),
                declaring_type: weak.clone(),
                add: Some(add),
            }));
        }
        if markup.kind == MarkupTypeKind::Enum {
            let wide = markup.enum_members.iter().any(|m| i32::try_from(m.value).is_err());
            members.enum_underlying_type =
                Some(self.get(if wide { "System.Int64" } else { "System.Int32" }));
            for member in markup.enum_members {
                members.fields.push(Rc::new(RuntimeField {
                    system: self.this.clone(),
                    name: member.name.to_string(),
                    declaring_type: weak.clone(),
                    field_type: self_type.clone(),
                    literal: Some(if wide { XamlValue::Int64(member.value) } else { XamlValue::Int32(member.value as i32) }),
                    value: RuntimeFieldValue::EnumMember(member.get),
                    attributes: Vec::new(),
                }));
            }
            if markup.is_flags {
                members.attributes.push(RuntimeCustomAttribute::new(
                    self.get("System.FlagsAttribute"),
                    Vec::new(),
                    Vec::new(),
                ));
            }
        }
        members.attributes.extend(self.project_attributes(markup.attributes));
        // A content property that an own property doesn't carry is stated on the type.
        if let Some(content) = markup.content_property {
            if !members.properties.iter().any(|p| p.name == content) {
                members.attributes.push(RuntimeCustomAttribute::new(
                    self.attribute_type(attributes::CONTENT),
                    Vec::new(),
                    vec![("Name".to_string(), XamlValue::String(content.to_string()))],
                ));
            }
        }
    }

    fn project_markup_type(&self, type_: &Rc<RuntimeType>, markup: &'static MarkupType) -> RuntimeMembers {
        let mut members = RuntimeMembers::default();
        // A collection that derives from an instantiation of the notifying list is that list
        // (and implements its list contracts) only when its values can be cast to it: the
        // members of the list (`Capacity`, `Add`) are called with the collection as the
        // instance. Without the registered cast the collection is a type of its own.
        let declared_base = markup.base.filter(|base| {
            let base = base();
            let is_list = self
                .resolve(base)
                .as_any()
                .downcast_ref::<RuntimeType>()
                .and_then(|base| base.definition().map(|d| d.key() == super::list_converter::LIST_DEFINITION))
                .unwrap_or(false);
            !is_list || markup.handles.first().is_some_and(|handle| ValueTypes::is_assignable(handle(), base))
        });
        members.base_type = match declared_base {
            Some(base) => Some(self.resolve(base())),
            None => match markup.kind {
                MarkupTypeKind::Class | MarkupTypeKind::Static => self.find_type("System.Object"),
                MarkupTypeKind::Struct => self.find_type("System.ValueType"),
                MarkupTypeKind::Enum => self.find_type("System.Enum"),
                MarkupTypeKind::Interface => None,
            },
        };
        // An instantiation of a definition of the type system has the base
        // type and the interfaces of the definition.
        if let Some(definition) = type_.definition().filter(|d| self.instantiable.borrow().contains(d.key())) {
            let source = definition.members();
            let parameters = definition.generic_parameter_types();
            let arguments = type_.generic_argument_types();
            if declared_base.is_none() {
                if let Some(base) = &source.base_type {
                    members.base_type = Some(substitute(base, parameters, arguments));
                }
            }
            members.interfaces = source.interfaces.iter().map(|i| substitute(i, parameters, arguments)).collect();
        }
        // The dictionary of the framework implements the dictionary contract of its key and
        // value types (`FerroDictionary<TKey, TValue> : IDictionary<TKey, TValue>`): a member
        // of the managed original that is declared as `IDictionary<TKey, TValue>` is declared
        // as the dictionary handle here (`ResourceDictionary.ThemeDictionaries`), and the
        // transformers ask for the contract.
        if type_.definition().is_some_and(|d| d.key() == DICTIONARY_DEFINITION) {
            let arguments = type_.generic_argument_types();
            let interface = self.find_type("System.Collections.Generic.IDictionary`2");
            if let Some(Ok(interface)) = interface.map(|i| i.make_generic_type(arguments)) {
                if !members.interfaces.iter().any(|i| i.equals(&*interface)) {
                    members.interfaces.push(interface);
                }
            }
        }
        // The notifying list of the framework implements the list contracts of its element
        // type (`FerroList<T> : IList<T>, IList, IReadOnlyList<T>, INotifyCollectionChanged`),
        // and with it every collection that derives from one of its instantiations.
        if type_.definition().is_some_and(|d| d.key() == super::list_converter::LIST_DEFINITION) {
            let arguments = type_.generic_argument_types();
            for name in ["System.Collections.Generic.IList`1", "System.Collections.Generic.IReadOnlyList`1"] {
                if let Some(Ok(interface)) = self.find_type(name).map(|i| i.make_generic_type(arguments)) {
                    if !members.interfaces.iter().any(|i| i.equals(&*interface)) {
                        members.interfaces.push(interface);
                    }
                }
            }
            for name in ["System.Collections.IList", "System.Collections.Specialized.INotifyCollectionChanged"] {
                if let Some(interface) = self.find_type(name) {
                    if !members.interfaces.iter().any(|i| i.equals(&*interface)) {
                        members.interfaces.push(interface);
                    }
                }
            }
        }
        self.project_markup(type_, markup, &mut members);
        // A registered instantiation of a definition of the type system also has the
        // members of the definition its metadata does not declare (the indexer and `Count`
        // of a list): abstract members, dispatched to the run-time type of the instance.
        if let Some(definition) = type_.definition().filter(|d| self.instantiable.borrow().contains(d.key())) {
            let source = definition.members();
            let parameters = definition.generic_parameter_types();
            let arguments = type_.generic_argument_types();
            let weak = Rc::downgrade(type_);
            let mut inherited: Vec<(Rc<RuntimeMethod>, Rc<RuntimeMethod>)> = Vec::new();
            for method in source.methods.iter().filter(|m| matches!(m.invoker, RuntimeInvoker::Virtual)) {
                let declared = members
                    .methods
                    .iter()
                    .any(|m| m.name == method.name && m.is_static == method.is_static && m.parameters.len() == method.parameters.len());
                if !declared {
                    inherited.push((method.clone(), method.substituted(&weak, parameters, arguments)));
                }
            }
            let accessor = |accessor: &Option<Rc<RuntimeMethod>>| {
                accessor.as_ref().and_then(|accessor| {
                    inherited.iter().find(|(source, _)| Rc::ptr_eq(source, accessor)).map(|(_, method)| method.clone())
                })
            };
            for property in &source.properties {
                if members.properties.iter().any(|p| p.name == property.name) {
                    continue;
                }
                let (getter, setter) = (accessor(&property.getter), accessor(&property.setter));
                if getter.is_none() && setter.is_none() {
                    continue;
                }
                members.properties.push(Rc::new(RuntimeProperty {
                    name: property.name.clone(),
                    declaring_type: weak.clone(),
                    property_type: substitute(&property.property_type, parameters, arguments),
                    getter,
                    setter,
                    attributes: property.attributes.clone(),
                    ferro_property: None,
                    indexer_parameters: property
                        .indexer_parameters
                        .iter()
                        .map(|p| substitute(p, parameters, arguments))
                        .collect(),
                }));
            }
            members.methods.extend(inherited.into_iter().map(|(_, method)| method));
            for attribute in &source.attributes {
                if !members.attributes.iter().any(|a| a.type_().equals(&*attribute.type_())) {
                    members.attributes.push(attribute.clone());
                }
            }
        }
        members
    }

    fn project_class(&self, type_: &Rc<RuntimeType>, type_info: &'static TypeInfo) -> RuntimeMembers {
        let mut members = RuntimeMembers::default();
        let weak = Rc::downgrade(type_);
        members.base_type = match type_info.base_type() {
            Some(base) => Some(self.runtime_type_of_class(base) as Rc<dyn IXamlType>),
            None => self.find_type("System.Object"),
        };
        for interface in type_info.interfaces() {
            let interface = self.resolve(interface());
            if !members.interfaces.iter().any(|i| i.equals(&*interface)) {
                members.interfaces.push(interface);
            }
        }
        if let Some(constructor) = type_info.default_constructor() {
            members.constructors.push(Rc::new(RuntimeConstructor {
                system: self.this.clone(),
                declaring_type: weak.clone(),
                parameters: Vec::new(),
                parameter_handles: Vec::new(),
                invoker: RuntimeInvoker::Dynamic(Rc::new(move |arguments| {
                    if !arguments.is_empty() {
                        return Err(MarkupInvokeError::ArgumentCount { expected: 0, actual: arguments.len() });
                    }
                    let object: BoxedValue = Rc::new(constructor());
                    Ok(Some(object))
                })),
                is_public: true,
                parameter_attributes: Vec::new(),
            }));
        }
        let markup = MarkupType::find_by_type_info(type_info).or_else(|| {
            // Metadata of the same name declared next to the class (see `runtime_type_of_markup`).
            MarkupType::find(type_info.namespace(), type_info.name())
                .filter(|m| m.generic.is_none() && m.module_path == type_info.module_path())
        });
        if let Some(markup) = markup.filter(|m| m.type_info.is_none()) {
            let _ = self.runtime_type_of_markup(markup);
        }
        let object_type: Rc<dyn IXamlType> = self.runtime_type_of_class(FerroObject::TYPE);
        let object_handle = Some(ValueType::of::<Ref<FerroObject>>());
        // The attributes of declared accessors of attached properties, by accessor name.
        let mut accessor_attributes: Vec<(String, Vec<Rc<dyn IXamlCustomAttribute>>)> = Vec::new();
        for property in FerroPropertyRegistry::instance().get_declared(type_info) {
            let handle = ValueType::new(property.property_type(), property.property_type_name());
            let property_type = self.resolve(handle);
            let name = property.name().to_string();
            // The attributes metadata states for the registered property sit where the
            // attribute of the managed property sits: on the property, on the field
            // with its definition and on the accessors of an attached property.
            let declared_attributes =
                self.project_attributes(markup.map_or(&[][..], |m| m.find_property_attributes(property.name())));
            members.fields.push(Rc::new(RuntimeField {
                system: self.this.clone(),
                name: format!("{name}Property"),
                declaring_type: weak.clone(),
                field_type: self.property_definition_type(property, &property_type, type_),
                literal: None,
                value: RuntimeFieldValue::Property(property),
                attributes: declared_attributes.clone(),
            }));
            let get = RuntimeInvoker::Dynamic(Rc::new(move |arguments: &[MarkupValue]| {
                check_count(arguments, 1)?;
                let object = object_argument(arguments)?;
                Ok(to_untyped(object.get_value_untyped(property)))
            }));
            let set = RuntimeInvoker::Dynamic(Rc::new(move |arguments: &[MarkupValue]| {
                check_count(arguments, 2)?;
                let object = object_argument(arguments)?;
                let value = to_exact(&arguments[1], handle, 1)?;
                let _ = object.set_value_untyped(property, value.as_any(), BindingPriority::LocalValue);
                Ok(None)
            }));
            // An attached property a class adds itself as an owner of (`TextBlock.FontSize`
            // of `TextElement.FontSize`) is an ordinary property of that class; the static
            // accessors belong to the type that registered it.
            if property.is_attached() && std::ptr::eq(property.owner_type(), type_info) {
                // The accessors take the host type of the attached property.
                let object_type: Rc<dyn IXamlType> = match property.host_type() {
                    Some(host) => self.runtime_type_of_class(host),
                    None => object_type.clone(),
                };
                // `[AssignBinding]` of the accessors is part of the definition of the property.
                let mut declared_attributes = declared_attributes;
                if property.assign_binding()
                    && !declared_attributes.iter().any(|a| a.type_().name() == "AssignBindingAttribute")
                {
                    declared_attributes.push(self.marker_attribute(attributes::ASSIGN_BINDING));
                }
                // Accessors the metadata declares itself are the accessors of the managed
                // original (which may do more than set the value, or be overloaded:
                // `Design.SetPreviewWith`); they carry what the definition of the property
                // states about them. An accessor that is not declared is the plain one.
                let declares_any = |accessor: &str| {
                    markup.is_some_and(|m| m.methods.iter().any(|method| method.is_static && method.name == accessor))
                };
                // A declared accessor stands for the plain one only if it is the accessor
                // for the host type of the property: a declaration may list only the
                // additional overloads (`Design.SetDataContext(IDataTemplate, object)`
                // next to the accessor of the property, which takes a control).
                let declares = |accessor: &str| {
                    markup.is_some_and(|m| {
                        m.methods.iter().any(|method| {
                            method.is_static
                                && method.name == accessor
                                && method.parameters.first().is_some_and(|host| self.resolve(host()).equals(&*object_type))
                        })
                    })
                };
                let getter_name = format!("Get{name}");
                let setter_name = format!("Set{name}");
                if declares_any(&getter_name) || declares_any(&setter_name) {
                    if !declared_attributes.is_empty() {
                        accessor_attributes.push((getter_name.clone(), declared_attributes.clone()));
                        accessor_attributes.push((setter_name.clone(), declared_attributes.clone()));
                    }
                }
                if !declares(&getter_name) {
                    members.methods.push(self.method(
                        type_,
                        getter_name,
                        true,
                        property_type.clone(),
                        (vec![object_type.clone()], vec![object_handle]),
                        get,
                        declared_attributes.clone(),
                    ));
                }
                if !declares(&setter_name) {
                    members.methods.push(self.method(
                        type_,
                        setter_name,
                        true,
                        self.void(),
                        (vec![object_type.clone(), property_type], vec![object_handle, Some(handle)]),
                        set,
                        declared_attributes,
                    ));
                }
                continue;
            }
            let getter = self.method(
                type_,
                format!("get_{name}"),
                false,
                property_type.clone(),
                (Vec::new(), Vec::new()),
                get,
                Vec::new(),
            );
            let setter = (!property.is_read_only()).then(|| {
                self.method(
                    type_,
                    format!("set_{name}"),
                    false,
                    self.void(),
                    (vec![property_type.clone()], vec![Some(handle)]),
                    set,
                    Vec::new(),
                )
            });
            let mut attributes = declared_attributes;
            if property.assign_binding() && !attributes.iter().any(|a| a.type_().name() == "AssignBindingAttribute") {
                attributes.push(self.marker_attribute(attributes::ASSIGN_BINDING));
            }
            if markup.and_then(|m| m.content_property) == Some(property.name()) {
                attributes.push(self.marker_attribute(attributes::CONTENT));
            }
            members.methods.push(getter.clone());
            members.methods.extend(setter.iter().cloned());
            members.properties.push(Rc::new(RuntimeProperty {
                name,
                declaring_type: weak.clone(),
                property_type,
                getter: Some(getter),
                setter,
                attributes,
                ferro_property: Some(property),
                indexer_parameters: Vec::new(),
            }));
        }
        if let Some(markup) = markup {
            let first_declared = members.methods.len();
            self.project_markup(type_, markup, &mut members);
            for method in &mut members.methods[first_declared..] {
                if !method.is_static {
                    continue;
                }
                let Some((_, attributes)) = accessor_attributes.iter().find(|(name, _)| *name == method.name) else {
                    continue;
                };
                let mut merged = method.attributes.clone();
                for attribute in attributes {
                    if !merged.iter().any(|a| a.type_().equals(&*attribute.type_())) {
                        merged.push(attribute.clone());
                    }
                }
                *method = Rc::new(RuntimeMethod {
                    system: method.system.clone(),
                    name: method.name.clone(),
                    declaring_type: method.declaring_type.clone(),
                    is_static: method.is_static,
                    return_type: method.return_type.clone(),
                    parameters: method.parameters.clone(),
                    parameter_handles: method.parameter_handles.clone(),
                    invoker: method.invoker.clone(),
                    attributes: merged,
                    generic_parameters: method.generic_parameters.clone(),
                    generic_arguments: method.generic_arguments.clone(),
                    declared: method.declared,
                });
            }
        }
        if members.constructors.is_empty() {
            // Every class of the managed original has a constructor; the one of a class
            // markup cannot create (an abstract class, a class without metadata for its
            // constructors) is not public. A document that populates an existing root
            // instance of such a class only needs it to exist.
            members.constructors.push(Rc::new(RuntimeConstructor {
                system: self.this.clone(),
                declaring_type: weak.clone(),
                parameters: Vec::new(),
                parameter_handles: Vec::new(),
                invoker: RuntimeInvoker::None,
                is_public: false,
                parameter_attributes: Vec::new(),
            }));
        }
        super::object_model::project_object_model_members(self, type_, type_info, &mut members);
        members
    }

    /// The type of the definition of a registered property:
    /// `FerroUI.StyledProperty<T>`, `FerroUI.AttachedProperty<T>` or
    /// `FerroUI.DirectProperty<TOwner, T>`.
    fn property_definition_type(
        &self,
        property: &FerroProperty,
        property_type: &Rc<dyn IXamlType>,
        owner: &Rc<RuntimeType>,
    ) -> Rc<dyn IXamlType> {
        core_types::define_property_types(self);
        let (definition, arguments): (&str, Vec<Rc<dyn IXamlType>>) = if property.is_direct() {
            ("DirectProperty`2", vec![owner.clone() as Rc<dyn IXamlType>, property_type.clone()])
        } else if property.is_attached()
            && owner.type_info().is_some_and(|class| std::ptr::eq(property.owner_type(), class))
        {
            // Only the class that registered an attached property declares it as one. A
            // class that adds itself as an owner declares the field as a styled property
            // (`StyledProperty<T> XProperty = Owner.XProperty.AddOwner<TOwner>()`), so that
            // the property is an instance property of that class.
            ("AttachedProperty`1", vec![property_type.clone()])
        } else {
            ("StyledProperty`1", vec![property_type.clone()])
        };
        self.find_type(&format!("{PROPERTY_NAMESPACE}.{definition}"))
            .and_then(|definition| definition.make_generic_type(&arguments).ok())
            .unwrap_or_else(|| self.get(&format!("{PROPERTY_NAMESPACE}.FerroProperty")))
    }

    pub(crate) fn framework_assembly_for_types(&self) -> Rc<RuntimeAssembly> {
        self.framework_assembly()
    }
}

/// Whether a static value of metadata is the definition of a registered property or of a
/// routed event (`WidthProperty`, `ClickEvent`): a static field of the managed original.
/// Decided by the declared Rust type of the value.
fn is_definition_field(type_: ValueType) -> bool {
    let name = type_.name();
    // The type without its path and type arguments (`RoutedEvent` of
    // `ferroui_base::interactivity::RoutedEvent<RoutedEventArgs>`, `FerroProperty` of
    // `&'static ferroui_base::FerroProperty`).
    let name = name.split('<').next().unwrap_or(name);
    let name = name.rsplit("::").next().unwrap_or(name);
    matches!(name, "FerroProperty" | "StyledProperty" | "AttachedProperty" | "DirectProperty" | "RoutedEvent")
}

fn namespace_is(type_: &Rc<RuntimeType>, namespace: &str) -> bool {
    type_.namespace().as_deref() == Some(namespace)
}

fn instantiation_key(definition: &Rc<RuntimeType>, arguments: &[Rc<dyn IXamlType>]) -> String {
    let arguments: Vec<String> = arguments.iter().map(type_key).collect();
    format!("{}<{}>", definition.key(), arguments.join(","))
}

fn check_count(arguments: &[MarkupValue], expected: usize) -> Result<(), MarkupInvokeError> {
    if arguments.len() != expected {
        return Err(MarkupInvokeError::ArgumentCount { expected, actual: arguments.len() });
    }
    Ok(())
}

/// The instance (or host) argument of an accessor of a registered property.
fn object_argument(arguments: &[MarkupValue]) -> Result<Ref<FerroObject>, MarkupInvokeError> {
    let object = arguments.first().and_then(|a| a.as_ref()).and_then(|a| ValueTypes::as_object(&**a));
    object.ok_or_else(|| MarkupInvokeError::Argument {
        index: 0,
        expected: "ferroui_base::Ref<ferroui_base::FerroObject>",
        actual: match arguments.first() {
            Some(Some(value)) => value.type_name().to_string(),
            _ => "null".to_string(),
        },
    })
}

impl IXamlTypeSystem for RuntimeTypeSystem {
    fn assemblies(&self) -> Vec<Rc<dyn IXamlAssembly>> {
        self.sync_assemblies();
        self.assemblies.borrow().iter().map(|a| a.clone() as Rc<dyn IXamlAssembly>).collect()
    }
    fn well_known_types(&self) -> Rc<XamlTypeWellKnownTypes> {
        self.well_known_types
            .get_or_init(|| match XamlTypeWellKnownTypes::new(self) {
                Ok(types) => Rc::new(types),
                Err(e) => panic!("The table of runtime library types is incomplete: {e}"),
            })
            .clone()
    }
    fn find_assembly(&self, substring: &str) -> Option<Rc<dyn IXamlAssembly>> {
        self.sync_assemblies();
        self.assemblies
            .borrow()
            .iter()
            .find(|a| a.name.to_lowercase().contains(&substring.to_lowercase()))
            .map(|a| a.clone() as Rc<dyn IXamlAssembly>)
    }
    fn find_type(&self, name: &str) -> Option<Rc<dyn IXamlType>> {
        self.find_runtime_type(name).map(|t| t as Rc<dyn IXamlType>)
    }
    fn find_type_in_assembly(&self, name: &str, assembly: &str) -> Option<Rc<dyn IXamlType>> {
        self.sync_assemblies();
        let found = self.find_runtime_type(name)?;
        let assemblies = self.assemblies.borrow();
        assemblies
            .iter()
            .filter(|a| a.has_name(assembly))
            .any(|a| self.type_belongs_to(&found, a))
            .then(|| found as Rc<dyn IXamlType>)
    }
}

/// Builds the members of a type the type system defines itself.
pub(crate) struct MemberBuilder {
    pub system: Rc<RuntimeTypeSystem>,
    pub type_: Rc<RuntimeType>,
    pub members: RuntimeMembers,
    /// The registered metadata of the type has been projected onto it.
    pub projected: bool,
}

impl MemberBuilder {
    /// The type named `full_name`.
    pub fn t(&self, full_name: &str) -> Rc<dyn IXamlType> {
        self.system.get(full_name)
    }

    /// The generic parameter `index` of the type being defined.
    pub fn parameter(&self, index: usize) -> Rc<dyn IXamlType> {
        match self.type_.generic_parameter_types().get(index) {
            Some(parameter) => parameter.clone(),
            None => XamlPseudoType::unknown(),
        }
    }

    /// `definition<arguments>`.
    pub fn generic(&self, definition: &str, arguments: &[Rc<dyn IXamlType>]) -> Rc<dyn IXamlType> {
        self.t(definition).make_generic_type(arguments).unwrap_or_else(|_| XamlPseudoType::unknown())
    }

    /// Projects the members of the registered markup metadata of the type
    /// `namespace.name` onto this type, which then is that metadata under
    /// its runtime library name. `false` (and nothing projected) if no such
    /// metadata is registered.
    pub fn project_metadata(&mut self, namespace: &str, name: &str) -> bool {
        if self.projected {
            return true;
        }
        let Some(markup) = MarkupType::find(namespace, name).filter(|m| m.generic.is_none()) else {
            return false;
        };
        self.projected = true;
        self.system.by_markup.borrow_mut().insert(markup as *const MarkupType as usize, self.type_.clone());
        self.system.project_markup(&self.type_, markup, &mut self.members);
        true
    }

    pub fn base(&mut self, full_name: &str) {
        self.members.base_type = self.system.find_type(full_name);
    }

    pub fn base_type(&mut self, type_: Rc<dyn IXamlType>) {
        self.members.base_type = Some(type_);
    }

    pub fn interface(&mut self, type_: Rc<dyn IXamlType>) {
        self.members.interfaces.push(type_);
    }

    pub fn method(
        &mut self,
        name: &str,
        is_static: bool,
        return_type: Rc<dyn IXamlType>,
        parameters: Vec<Rc<dyn IXamlType>>,
        invoker: RuntimeInvoker,
    ) -> Rc<RuntimeMethod> {
        // A member the registered metadata of the type declares is not declared a second time.
        if let Some(existing) = self.members.methods.iter().find(|m| {
            m.name == name && m.is_static == is_static && m.parameters.len() == parameters.len()
        }) {
            return existing.clone();
        }
        let handles = vec![None; parameters.len()];
        let method = self.system.method(
            &self.type_,
            name.to_string(),
            is_static,
            return_type,
            (parameters, handles),
            invoker,
            Vec::new(),
        );
        self.members.methods.push(method.clone());
        method
    }

    pub fn constructor(&mut self, parameters: Vec<Rc<dyn IXamlType>>, invoker: RuntimeInvoker) {
        if self.members.constructors.iter().any(|c| c.parameters.len() == parameters.len()) {
            return;
        }
        let parameter_handles = vec![None; parameters.len()];
        self.members.constructors.push(Rc::new(RuntimeConstructor {
            system: self.system.weak(),
            declaring_type: Rc::downgrade(&self.type_),
            parameters,
            parameter_handles,
            invoker,
            is_public: true,
                parameter_attributes: Vec::new(),
        }));
    }

    /// The indexer of the type: the property `Item` with the accessors `get_Item` and (when
    /// `writable`) `set_Item`, and `[DefaultMember("Item")]` on the type. The accessors are
    /// abstract: a call is dispatched to the indexer the run-time type of the instance
    /// declares in its metadata.
    pub fn indexer(&mut self, parameters: Vec<Rc<dyn IXamlType>>, type_: Rc<dyn IXamlType>, writable: bool) {
        let getter = self.method("get_Item", false, type_.clone(), parameters.clone(), RuntimeInvoker::Virtual);
        let setter = writable.then(|| {
            let mut setter_parameters = parameters.clone();
            setter_parameters.push(type_.clone());
            let void = self.system.void();
            self.method("set_Item", false, void, setter_parameters, RuntimeInvoker::Virtual)
        });
        self.members.properties.push(Rc::new(RuntimeProperty {
            name: "Item".to_string(),
            declaring_type: Rc::downgrade(&self.type_),
            property_type: type_,
            getter: Some(getter),
            setter,
            attributes: Vec::new(),
            ferro_property: None,
            indexer_parameters: parameters,
        }));
        self.members.attributes.push(RuntimeCustomAttribute::new(
            self.system.attribute_type("DefaultMember"),
            vec![XamlValue::String("Item".to_string())],
            Vec::new(),
        ));
    }

    /// A read-only property with the getter `get_<name>`.
    pub fn property(&mut self, name: &str, type_: Rc<dyn IXamlType>, is_static: bool, getter: RuntimeInvoker) {
        let getter = self.method(&format!("get_{name}"), is_static, type_.clone(), Vec::new(), getter);
        self.members.properties.push(Rc::new(RuntimeProperty {
            name: name.to_string(),
            declaring_type: Rc::downgrade(&self.type_),
            property_type: type_,
            getter: Some(getter),
            setter: None,
            attributes: Vec::new(),
            ferro_property: None,
            indexer_parameters: Vec::new(),
        }));
    }
}
