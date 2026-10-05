//! The types of the run-time type system and their members.
//!
//! A [`RuntimeType`] is a cheap shell (name, kind, where it comes from); its
//! base type, interfaces and members are projected from the metadata the
//! first time one of them is asked for. Every invocable member carries the
//! invoker of the metadata it was projected from, so that the interpreter can
//! call it.

use std::any::{Any, TypeId};
use std::cell::{Cell, RefCell};
use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::rc::{Rc, Weak};

use ferroui_base::data::core::{ValueType, ValueTypes};
use ferroui_base::metadata::{MarkupInvoke, MarkupInvokeError, MarkupType, MarkupValue};
use ferroui_base::{BoxedValue, FerroProperty, TypeInfo};
use xamlx::exceptions::{XamlError, XamlResult};
use xamlx::type_system::{
    AnonymousParameterInfo, IXamlAssembly, IXamlConstructor, IXamlCustomAttribute, IXamlEventInfo,
    IXamlField, IXamlMember, IXamlMethod, IXamlParameterInfo, IXamlProperty, IXamlType,
    XamlPseudoType, XamlTypeId, XamlValue,
};

use super::runtime_type_system::RuntimeTypeSystem;
use super::values::{normalize_object, RuntimeTypeValue, RuntimeArray};

/// What kind of type a [`RuntimeType`] is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RuntimeTypeKind {
    Class,
    Interface,
    Struct,
    Enum,
    GenericParameter,
}

/// The metadata a [`RuntimeType`] is projected from.
#[derive(Clone, Copy)]
pub enum RuntimeTypeOrigin {
    /// A type of the closed table of runtime library types, or a type the
    /// type system defines itself.
    Synthetic,
    /// A class of the object model (and its markup metadata, if any).
    Class(&'static TypeInfo),
    /// A type with markup metadata that is not a class of the object model.
    Markup(&'static MarkupType),
    /// A Rust type without metadata.
    Opaque,
    GenericParameter,
    /// An instantiation of a synthetic generic definition; its members are
    /// the substituted members of the definition.
    Instantiation,
    Array,
}

/// How a projected member is invoked.
#[derive(Clone)]
pub enum RuntimeInvoker {
    /// The invoker declared in metadata.
    Static(MarkupInvoke),
    /// An invoker built by the type system (accessors of registered
    /// properties, members of runtime library types).
    Dynamic(Rc<dyn Fn(&[MarkupValue]) -> Result<MarkupValue, MarkupInvokeError>>),
    /// An abstract member (of a runtime library interface or of a synthetic
    /// instantiation): the call is dispatched by name to the member of the
    /// run-time type of the instance.
    Virtual,
    /// The member cannot be invoked.
    None,
}

/// The lazily projected part of a [`RuntimeType`].
#[derive(Default)]
pub struct RuntimeMembers {
    pub base_type: Option<Rc<dyn IXamlType>>,
    /// The interfaces the type declares (without the ones they inherit).
    pub interfaces: Vec<Rc<dyn IXamlType>>,
    pub properties: Vec<Rc<RuntimeProperty>>,
    pub events: Vec<Rc<RuntimeEvent>>,
    pub fields: Vec<Rc<RuntimeField>>,
    pub methods: Vec<Rc<RuntimeMethod>>,
    pub constructors: Vec<Rc<RuntimeConstructor>>,
    pub attributes: Vec<Rc<dyn IXamlCustomAttribute>>,
    pub enum_underlying_type: Option<Rc<dyn IXamlType>>,
}

pub(crate) type MembersInit = Box<dyn FnOnce(&Rc<RuntimeType>) -> RuntimeMembers>;

/// A type of the run-time type system.
pub struct RuntimeType {
    this: Weak<RuntimeType>,
    pub(crate) system: Weak<RuntimeTypeSystem>,
    key: String,
    hash: u64,
    namespace: String,
    name: String,
    kind: RuntimeTypeKind,
    origin: RuntimeTypeOrigin,
    assembly: RefCell<Option<Weak<RuntimeAssembly>>>,
    generic_parameters: Vec<Rc<RuntimeType>>,
    generic_definition: Option<Rc<RuntimeType>>,
    generic_arguments: Vec<Rc<dyn IXamlType>>,
    /// The Rust types that hold values of the type, the untyped form first.
    handles: RefCell<Vec<ValueType>>,
    members: RefCell<Option<Rc<RuntimeMembers>>>,
    initializing: Cell<bool>,
    init: RefCell<Option<MembersInit>>,
    array_element_type: Option<Rc<dyn IXamlType>>,
    arrays: RefCell<HashMap<i32, Rc<RuntimeType>>>,
}

/// What [`RuntimeType::create`] needs to know about a type.
pub(crate) struct RuntimeTypeSpec {
    pub key: String,
    pub namespace: String,
    pub name: String,
    pub kind: RuntimeTypeKind,
    pub origin: RuntimeTypeOrigin,
    pub generic_parameter_names: Vec<String>,
    pub generic_definition: Option<Rc<RuntimeType>>,
    pub generic_arguments: Vec<Rc<dyn IXamlType>>,
    pub handles: Vec<ValueType>,
    pub array_element_type: Option<Rc<dyn IXamlType>>,
    pub init: Option<MembersInit>,
}

impl RuntimeTypeSpec {
    pub fn new(key: String, namespace: &str, name: &str, kind: RuntimeTypeKind, origin: RuntimeTypeOrigin) -> Self {
        Self {
            key,
            namespace: namespace.to_string(),
            name: name.to_string(),
            kind,
            origin,
            generic_parameter_names: Vec::new(),
            generic_definition: None,
            generic_arguments: Vec::new(),
            handles: Vec::new(),
            array_element_type: None,
            init: None,
        }
    }
}

fn hash_str(s: &str) -> u64 {
    let mut hasher = DefaultHasher::new();
    s.hash(&mut hasher);
    hasher.finish()
}

/// The identity of a type as a string: the key of a [`RuntimeType`], the
/// debug form of the identifier of any other type.
pub(crate) fn type_key(type_: &Rc<dyn IXamlType>) -> String {
    match type_.as_any().downcast_ref::<RuntimeType>() {
        Some(runtime) => runtime.key.clone(),
        None => format!("{:?}", type_.id()),
    }
}

impl RuntimeType {
    pub(crate) fn create(system: &Weak<RuntimeTypeSystem>, spec: RuntimeTypeSpec) -> Rc<RuntimeType> {
        Rc::new_cyclic(|this| {
            let generic_parameters = spec
                .generic_parameter_names
                .iter()
                .map(|parameter| {
                    RuntimeType::create(
                        system,
                        RuntimeTypeSpec::new(
                            format!("{}!{parameter}", spec.key),
                            "",
                            parameter,
                            RuntimeTypeKind::GenericParameter,
                            RuntimeTypeOrigin::GenericParameter,
                        ),
                    )
                })
                .collect();
            RuntimeType {
                this: this.clone(),
                system: system.clone(),
                hash: hash_str(&spec.key),
                key: spec.key,
                namespace: spec.namespace,
                name: spec.name,
                kind: spec.kind,
                origin: spec.origin,
                assembly: RefCell::new(None),
                generic_parameters,
                generic_definition: spec.generic_definition,
                generic_arguments: spec.generic_arguments,
                handles: RefCell::new(spec.handles),
                members: RefCell::new(None),
                initializing: Cell::new(false),
                init: RefCell::new(spec.init),
                array_element_type: spec.array_element_type,
                arrays: RefCell::new(HashMap::new()),
            }
        })
    }

    /// The identity of the type: its full name with its type arguments.
    pub fn key(&self) -> &str {
        &self.key
    }

    pub fn kind(&self) -> RuntimeTypeKind {
        self.kind
    }

    pub fn origin(&self) -> RuntimeTypeOrigin {
        self.origin
    }

    /// The class of the object model the type is projected from.
    pub fn type_info(&self) -> Option<&'static TypeInfo> {
        match self.origin {
            RuntimeTypeOrigin::Class(type_info) => Some(type_info),
            _ => None,
        }
    }

    /// The markup metadata the type is projected from: of the type itself,
    /// or of the class.
    pub fn markup(&self) -> Option<&'static MarkupType> {
        match self.origin {
            RuntimeTypeOrigin::Markup(markup) => Some(markup),
            RuntimeTypeOrigin::Class(type_info) => MarkupType::find_by_type_info(type_info),
            _ => None,
        }
    }

    /// The Rust types that hold values of the type, the untyped (canonical)
    /// form first. Empty for types without values and for synthetic types.
    pub fn handles(&self) -> Vec<ValueType> {
        self.handles.borrow().clone()
    }

    /// The untyped (canonical) handle type of the type.
    pub fn handle(&self) -> Option<ValueType> {
        self.handles.borrow().first().copied()
    }

    pub(crate) fn add_handle(&self, handle: ValueType) {
        let mut handles = self.handles.borrow_mut();
        if !handles.contains(&handle) {
            handles.push(handle);
        }
    }

    pub(crate) fn set_assembly(&self, assembly: &Rc<RuntimeAssembly>) {
        *self.assembly.borrow_mut() = Some(Rc::downgrade(assembly));
    }

    pub(crate) fn runtime_assembly(&self) -> Option<Rc<RuntimeAssembly>> {
        self.assembly.borrow().as_ref().and_then(Weak::upgrade)
    }

    /// This type as a type system handle.
    pub fn as_type(&self) -> Rc<dyn IXamlType> {
        match self.this.upgrade() {
            Some(this) => this,
            None => XamlPseudoType::unknown(),
        }
    }

    pub(crate) fn rc(&self) -> Option<Rc<RuntimeType>> {
        self.this.upgrade()
    }

    pub(crate) fn definition(&self) -> Option<&Rc<RuntimeType>> {
        self.generic_definition.as_ref()
    }

    pub(crate) fn generic_parameter_types(&self) -> &[Rc<RuntimeType>] {
        &self.generic_parameters
    }

    pub(crate) fn generic_argument_types(&self) -> &[Rc<dyn IXamlType>] {
        &self.generic_arguments
    }

    /// The projected members, projecting them on first use.
    pub fn members(&self) -> Rc<RuntimeMembers> {
        if let Some(members) = self.members.borrow().as_ref() {
            return members.clone();
        }
        // A projection that (through a cycle in the metadata) asks for the
        // members being projected sees a type without members.
        if self.initializing.replace(true) {
            return Rc::new(RuntimeMembers::default());
        }
        let init = self.init.borrow_mut().take();
        let members = match (self.this.upgrade(), init, self.system.upgrade()) {
            (Some(this), Some(init), _) => init(&this),
            (Some(this), None, Some(system)) => system.project_members(&this),
            _ => RuntimeMembers::default(),
        };
        let members = Rc::new(members);
        *self.members.borrow_mut() = Some(members.clone());
        self.initializing.set(false);
        members
    }

    /// The projected methods, with their invokers.
    pub fn runtime_methods(&self) -> Vec<Rc<RuntimeMethod>> {
        self.members().methods.clone()
    }

    /// The projected constructors, with their invokers.
    pub fn runtime_constructors(&self) -> Vec<Rc<RuntimeConstructor>> {
        self.members().constructors.clone()
    }

    /// The projected fields, with their values.
    pub fn runtime_fields(&self) -> Vec<Rc<RuntimeField>> {
        self.members().fields.clone()
    }

    /// The projected properties.
    pub fn runtime_properties(&self) -> Vec<Rc<RuntimeProperty>> {
        self.members().properties.clone()
    }

    pub(crate) fn find_array(&self, dimensions: i32) -> Option<Rc<RuntimeType>> {
        self.arrays.borrow().get(&dimensions).cloned()
    }
}

/// Replaces the generic parameters of a definition in `type_` by the
/// arguments of an instantiation.
pub(crate) fn substitute(
    type_: &Rc<dyn IXamlType>,
    parameters: &[Rc<RuntimeType>],
    arguments: &[Rc<dyn IXamlType>],
) -> Rc<dyn IXamlType> {
    let Some(runtime) = type_.as_any().downcast_ref::<RuntimeType>() else {
        return type_.clone();
    };
    if runtime.kind == RuntimeTypeKind::GenericParameter {
        return match parameters.iter().position(|p| p.key == runtime.key) {
            Some(index) if index < arguments.len() => arguments[index].clone(),
            _ => type_.clone(),
        };
    }
    if let Some(definition) = runtime.definition() {
        let new_arguments: Vec<Rc<dyn IXamlType>> =
            runtime.generic_arguments.iter().map(|a| substitute(a, parameters, arguments)).collect();
        let unchanged =
            new_arguments.iter().zip(runtime.generic_arguments.iter()).all(|(new, old)| new.equals(&**old));
        if !unchanged {
            if let Ok(instance) = definition.make_generic_type(&new_arguments) {
                return instance;
            }
        }
    }
    type_.clone()
}

impl IXamlType for RuntimeType {
    fn id(&self) -> XamlTypeId {
        XamlTypeId::Named(self.key.clone())
    }
    fn name(&self) -> String {
        self.name.clone()
    }
    fn namespace(&self) -> Option<String> {
        Some(self.namespace.clone())
    }
    fn full_name(&self) -> String {
        let mut rv =
            if self.namespace.is_empty() { self.name.clone() } else { format!("{}.{}", self.namespace, self.name) };
        if !self.generic_arguments.is_empty() {
            let arguments: Vec<String> = self.generic_arguments.iter().map(|a| a.full_name()).collect();
            rv = format!("{rv}[{}]", arguments.join(","));
        }
        rv
    }
    fn is_public(&self) -> bool {
        true
    }
    fn is_nested_private(&self) -> bool {
        false
    }
    fn assembly(&self) -> Option<Rc<dyn IXamlAssembly>> {
        self.runtime_assembly().map(|a| a as Rc<dyn IXamlAssembly>)
    }
    fn properties(&self) -> Vec<Rc<dyn IXamlProperty>> {
        self.members().properties.iter().map(|p| p.clone() as Rc<dyn IXamlProperty>).collect()
    }
    fn events(&self) -> Vec<Rc<dyn IXamlEventInfo>> {
        self.members().events.iter().map(|e| e.clone() as Rc<dyn IXamlEventInfo>).collect()
    }
    fn fields(&self) -> Vec<Rc<dyn IXamlField>> {
        self.members().fields.iter().map(|f| f.clone() as Rc<dyn IXamlField>).collect()
    }
    fn methods(&self) -> Vec<Rc<dyn IXamlMethod>> {
        self.members().methods.iter().map(|m| m.clone() as Rc<dyn IXamlMethod>).collect()
    }
    fn constructors(&self) -> Vec<Rc<dyn IXamlConstructor>> {
        self.members().constructors.iter().map(|c| c.clone() as Rc<dyn IXamlConstructor>).collect()
    }
    fn custom_attributes(&self) -> Vec<Rc<dyn IXamlCustomAttribute>> {
        self.members().attributes.clone()
    }
    fn generic_arguments(&self) -> Vec<Rc<dyn IXamlType>> {
        self.generic_arguments.clone()
    }
    fn is_assignable_from(&self, type_: &dyn IXamlType) -> bool {
        let this: &dyn IXamlType = self;
        if XamlPseudoType::is_null(type_) {
            return !self.is_value_type() || this.is_nullable();
        }
        if type_.is_value_type() && this.is_nullable_of(type_) {
            return true;
        }
        if this.is("System", "Object") && type_.is_interface() {
            return true;
        }
        if type_.equals(self) {
            return true;
        }
        let mut base_type = type_.base_type();
        while let Some(bt) = base_type {
            if bt.equals(self) {
                return true;
            }
            base_type = bt.base_type();
        }
        self.is_interface() && type_.get_all_interfaces().iter().any(|i| self.is_assignable_from(&**i))
    }
    fn make_generic_type(&self, type_arguments: &[Rc<dyn IXamlType>]) -> XamlResult<Rc<dyn IXamlType>> {
        if self.generic_parameters.is_empty() {
            return Err(XamlError::invalid_operation(format!(
                "{} is not a generic type definition",
                self.full_name()
            )));
        }
        if self.generic_parameters.len() != type_arguments.len() {
            return Err(XamlError::argument(format!(
                "{} expects {} type arguments, got {}",
                self.full_name(),
                self.generic_parameters.len(),
                type_arguments.len()
            )));
        }
        match (self.system.upgrade(), self.this.upgrade()) {
            (Some(system), Some(this)) => system.instantiate(&this, type_arguments),
            _ => Err(XamlError::invalid_operation("The type system has been dropped")),
        }
    }
    fn generic_type_definition(&self) -> Option<Rc<dyn IXamlType>> {
        self.generic_definition.clone().map(|d| d as Rc<dyn IXamlType>)
    }
    fn is_array(&self) -> bool {
        self.array_element_type.is_some()
    }
    fn array_element_type(&self) -> Option<Rc<dyn IXamlType>> {
        self.array_element_type.clone()
    }
    fn make_array_type(&self, dimensions: i32) -> XamlResult<Rc<dyn IXamlType>> {
        if dimensions < 1 {
            return Err(XamlError::argument("An array type needs at least one dimension"));
        }
        if let Some(existing) = self.find_array(dimensions) {
            return Ok(existing);
        }
        let (Some(system), Some(this)) = (self.system.upgrade(), self.this.upgrade()) else {
            return Err(XamlError::invalid_operation("The type system has been dropped"));
        };
        let array = system.create_array_type(&this, dimensions);
        self.arrays.borrow_mut().insert(dimensions, array.clone());
        Ok(array)
    }
    fn base_type(&self) -> Option<Rc<dyn IXamlType>> {
        self.members().base_type.clone()
    }
    fn declaring_type(&self) -> Option<Rc<dyn IXamlType>> {
        None
    }
    fn is_value_type(&self) -> bool {
        matches!(self.kind, RuntimeTypeKind::Struct | RuntimeTypeKind::Enum)
    }
    fn is_enum(&self) -> bool {
        self.kind == RuntimeTypeKind::Enum
    }
    fn interfaces(&self) -> Vec<Rc<dyn IXamlType>> {
        // Declared interfaces followed by the interfaces they inherit.
        let mut rv: Vec<Rc<dyn IXamlType>> = Vec::new();
        for declared in self.members().interfaces.iter() {
            let inherited = declared.interfaces();
            for candidate in std::iter::once(declared.clone()).chain(inherited) {
                if !rv.iter().any(|existing| existing.equals(&*candidate)) {
                    rv.push(candidate);
                }
            }
        }
        rv
    }
    fn is_interface(&self) -> bool {
        self.kind == RuntimeTypeKind::Interface
    }
    fn get_enum_underlying_type(&self) -> XamlResult<Rc<dyn IXamlType>> {
        self.members()
            .enum_underlying_type
            .clone()
            .ok_or_else(|| XamlError::invalid_operation(format!("{} is not an enum", self.full_name())))
    }
    fn generic_parameters(&self) -> Vec<Rc<dyn IXamlType>> {
        self.generic_parameters.iter().map(|p| p.clone() as Rc<dyn IXamlType>).collect()
    }
    fn is_function_pointer(&self) -> bool {
        false
    }
    fn equals(&self, other: &dyn IXamlType) -> bool {
        match other.as_any().downcast_ref::<RuntimeType>() {
            Some(other) => self.key == other.key,
            None => false,
        }
    }
    fn get_hash_code(&self) -> u64 {
        self.hash
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}

fn declaring(declaring_type: &Weak<RuntimeType>) -> Rc<dyn IXamlType> {
    match declaring_type.upgrade() {
        Some(type_) => type_,
        None => XamlPseudoType::unknown(),
    }
}

fn types_equal(a: &[Rc<dyn IXamlType>], b: &[Rc<dyn IXamlType>]) -> bool {
    a.len() == b.len() && a.iter().zip(b).all(|(x, y)| x.equals(&**y))
}

/// A custom attribute projected from metadata.
pub struct RuntimeCustomAttribute {
    type_: Rc<dyn IXamlType>,
    parameters: Vec<XamlValue>,
    properties: Vec<(String, XamlValue)>,
}

impl RuntimeCustomAttribute {
    pub fn new(
        type_: Rc<dyn IXamlType>,
        parameters: Vec<XamlValue>,
        properties: Vec<(String, XamlValue)>,
    ) -> Rc<dyn IXamlCustomAttribute> {
        Rc::new(Self { type_, parameters, properties })
    }
}

impl IXamlCustomAttribute for RuntimeCustomAttribute {
    fn type_(&self) -> Rc<dyn IXamlType> {
        self.type_.clone()
    }
    fn parameters(&self) -> Vec<XamlValue> {
        self.parameters.clone()
    }
    fn properties(&self) -> HashMap<String, XamlValue> {
        self.properties.iter().cloned().collect()
    }
    fn equals(&self, other: &dyn IXamlCustomAttribute) -> bool {
        std::ptr::addr_eq(self as *const Self, other.as_any() as *const dyn Any)
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// A method projected from metadata, with its invoker.
pub struct RuntimeMethod {
    pub(crate) system: Weak<RuntimeTypeSystem>,
    pub(crate) name: String,
    pub(crate) declaring_type: Weak<RuntimeType>,
    pub(crate) is_static: bool,
    pub(crate) return_type: Rc<dyn IXamlType>,
    pub(crate) parameters: Vec<Rc<dyn IXamlType>>,
    /// The declared Rust types of the parameters (without the instance).
    pub(crate) parameter_handles: Vec<Option<ValueType>>,
    pub(crate) invoker: RuntimeInvoker,
    pub(crate) attributes: Vec<Rc<dyn IXamlCustomAttribute>>,
    /// The type parameters of a generic method definition (the type system
    /// synthesizes the few generic members of the object model; metadata
    /// declares none).
    pub(crate) generic_parameters: Vec<Rc<RuntimeType>>,
    /// The type arguments of a constructed generic method.
    pub(crate) generic_arguments: Vec<Rc<dyn IXamlType>>,
    /// The member of markup metadata the method is the projection of.
    pub(crate) declared: Option<DeclaredMember>,
}

/// The member of markup metadata a [`RuntimeMethod`] is the projection of:
/// what the emitter of Rust source calls through the typed function the
/// declaration generated ([`MarkupEmit`](ferroui_base::metadata::MarkupEmit)).
#[derive(Clone, Copy)]
pub enum DeclaredMember {
    /// The getter of an instance property.
    Getter(&'static ferroui_base::metadata::MarkupProperty),
    /// The setter of an instance property.
    Setter(&'static ferroui_base::metadata::MarkupProperty),
    /// The getter of a static property.
    StaticGetter(&'static ferroui_base::metadata::MarkupProperty),
    /// The setter of a static property.
    StaticSetter(&'static ferroui_base::metadata::MarkupProperty),
    /// A method.
    Method(&'static ferroui_base::metadata::MarkupMethod),
    /// The `Parse(string)` of the type (`parse:`).
    Parse(&'static MarkupType),
}

impl DeclaredMember {
    /// The typed function of the member, if the declaration generated one.
    pub fn emit(&self) -> Option<ferroui_base::metadata::MarkupEmit> {
        match self {
            Self::Getter(property) | Self::StaticGetter(property) => property.emit_get,
            Self::Setter(property) | Self::StaticSetter(property) => property.emit_set,
            Self::Method(method) => method.emit,
            Self::Parse(markup) => markup.parse_type.map(|_| ferroui_base::metadata::MarkupEmit {
                function: "__markup_parse",
                fallible: true,
            }),
        }
    }
}

impl RuntimeMethod {
    /// The invoker of the method.
    pub fn invoker(&self) -> &RuntimeInvoker {
        &self.invoker
    }

    /// The member of markup metadata the method is the projection of.
    pub fn declared(&self) -> Option<DeclaredMember> {
        self.declared
    }

    /// The method as the projection of the declared member `declared`.
    pub(crate) fn with_declared(self: Rc<Self>, declared: DeclaredMember) -> Rc<Self> {
        let mut method = Rc::try_unwrap(self).unwrap_or_else(|shared| shared.copy());
        method.declared = Some(declared);
        Rc::new(method)
    }

    fn copy(&self) -> Self {
        Self {
            system: self.system.clone(),
            name: self.name.clone(),
            declaring_type: self.declaring_type.clone(),
            is_static: self.is_static,
            return_type: self.return_type.clone(),
            parameters: self.parameters.clone(),
            parameter_handles: self.parameter_handles.clone(),
            invoker: self.invoker.clone(),
            attributes: self.attributes.clone(),
            generic_parameters: self.generic_parameters.clone(),
            generic_arguments: self.generic_arguments.clone(),
            declared: self.declared,
        }
    }

    /// Whether the method has an invoker of its own (it is not abstract).
    pub fn is_invocable(&self) -> bool {
        matches!(self.invoker, RuntimeInvoker::Static(_) | RuntimeInvoker::Dynamic(_))
    }

    /// Calls the method: `arguments` holds the instance (for an instance
    /// method) followed by the arguments, in untyped form. An abstract
    /// method is dispatched to the method of the same name and arity of the
    /// run-time type of the instance.
    pub fn invoke(&self, arguments: &[MarkupValue]) -> Result<MarkupValue, MarkupInvokeError> {
        let offset = usize::from(!self.is_static);
        let invoke: &dyn Fn(&[MarkupValue]) -> Result<MarkupValue, MarkupInvokeError> = match &self.invoker {
            RuntimeInvoker::Static(invoke) => invoke,
            RuntimeInvoker::Dynamic(invoke) => &**invoke,
            RuntimeInvoker::Virtual => return self.dispatch(arguments),
            RuntimeInvoker::None => {
                return Err(MarkupInvokeError::Failed(format!(
                    "Method {}.{} has no invoker",
                    declaring(&self.declaring_type).full_name(),
                    self.name
                )))
            }
        };
        let adapted = adapt_arguments(arguments, &self.parameter_handles, offset).map_err(|e| {
            named(e, &format!("{}.{}", declaring(&self.declaring_type).full_name(), self.name))
        })?;
        let result = match adapted {
            Some(adapted) => invoke(&adapted),
            None => invoke(arguments),
        }?;
        Ok(normalize_result(&self.system, result))
    }

    fn dispatch(&self, arguments: &[MarkupValue]) -> Result<MarkupValue, MarkupInvokeError> {
        let failed = |message: String| MarkupInvokeError::Failed(message);
        let Some(Some(instance)) = arguments.first() else {
            return Err(failed(format!("Method {} was called on a null reference", self.name)));
        };
        let system = self.system.upgrade().ok_or_else(|| failed("The type system has been dropped".into()))?;
        let runtime_type = system.runtime_type_of(instance);
        let mut current: Option<Rc<dyn IXamlType>> = Some(runtime_type.clone());
        let mut candidates: Vec<Rc<dyn IXamlType>> = Vec::new();
        while let Some(type_) = current {
            current = type_.base_type();
            candidates.push(type_);
        }
        candidates.extend(runtime_type.get_all_interfaces());
        let mut fallback: Option<Rc<RuntimeMethod>> = None;
        for candidate in &candidates {
            let Some(candidate) = candidate.as_any().downcast_ref::<RuntimeType>() else { continue };
            for method in candidate.members().methods.iter() {
                if method.name != self.name
                    || method.is_static
                    || !method.is_invocable()
                    || method.parameters.len() != self.parameters.len()
                {
                    continue;
                }
                if types_equal(&method.parameters, &self.parameters) {
                    return method.invoke(arguments);
                }
                let accepts = method.parameters.iter().zip(&arguments[1..]).all(|(p, a)| system.is_instance(a, &**p));
                if accepts && fallback.is_none() {
                    fallback = Some(method.clone());
                }
            }
        }
        match fallback {
            Some(method) => method.invoke(arguments),
            None => Err(failed(format!(
                "Type {} doesn't implement {}.{} with {} argument(s)",
                runtime_type.full_name(),
                declaring(&self.declaring_type).full_name(),
                self.name,
                self.parameters.len()
            ))),
        }
    }

    pub(crate) fn substituted(
        &self,
        declaring_type: &Weak<RuntimeType>,
        parameters: &[Rc<RuntimeType>],
        arguments: &[Rc<dyn IXamlType>],
    ) -> Rc<RuntimeMethod> {
        Rc::new(RuntimeMethod {
            system: self.system.clone(),
            name: self.name.clone(),
            declaring_type: declaring_type.clone(),
            is_static: self.is_static,
            return_type: substitute(&self.return_type, parameters, arguments),
            parameters: self.parameters.iter().map(|p| substitute(p, parameters, arguments)).collect(),
            parameter_handles: self.parameter_handles.clone(),
            invoker: self.invoker.clone(),
            attributes: self.attributes.clone(),
            generic_parameters: self.generic_parameters.clone(),
            generic_arguments: self.generic_arguments.clone(),
            declared: self.declared,
        })
    }

    /// The definition as a generic method definition with the type
    /// parameters `generic_parameters` (which its signature refers to).
    pub(crate) fn into_generic_definition(mut self, generic_parameters: Vec<Rc<RuntimeType>>) -> Self {
        self.generic_parameters = generic_parameters;
        self
    }
}

/// Names the member a failed argument conversion belongs to.
fn named(error: MarkupInvokeError, member: &str) -> MarkupInvokeError {
    match error {
        MarkupInvokeError::Failed(message) => MarkupInvokeError::Failed(format!("{member}: {message}")),
        other => other,
    }
}

/// Converts the arguments that are not held in the Rust type the invoker
/// declares: `System.Type` values. `None` if nothing needs converting.
fn adapt_arguments(
    arguments: &[MarkupValue],
    handles: &[Option<ValueType>],
    offset: usize,
) -> Result<Option<Vec<MarkupValue>>, MarkupInvokeError> {
    let type_value = TypeId::of::<RuntimeTypeValue>();
    let array = TypeId::of::<RuntimeArray>();
    let needs = |index: usize, value: &MarkupValue| -> bool {
        let Some(value) = value else { return false };
        let id = value.value_type_id();
        if index < offset {
            return false;
        }
        if id != type_value && id != array {
            // An object passed to an untyped parameter is handed over in the
            // untyped form of the framework.
            return matches!(handles.get(index - offset), Some(Some(handle)) if handle.is_object())
                && super::values::untyped_object_form(value).is_some();
        }
        match handles.get(index - offset) {
            Some(Some(handle)) => handle.id() != id && !(id == array && handle.is_object()),
            _ => false,
        }
    };
    if !arguments.iter().enumerate().any(|(index, value)| needs(index, value)) {
        return Ok(None);
    }
    let mut adapted = arguments.to_vec();
    for (index, value) in arguments.iter().enumerate() {
        if !needs(index, value) {
            continue;
        }
        let (Some(value), Some(Some(handle))) = (value, handles.get(index - offset)) else { continue };
        if let Some(array) = value.downcast_ref::<RuntimeArray>() {
            adapted[index] = Some(array.to_declared(*handle).map_err(MarkupInvokeError::Failed)?);
            continue;
        }
        if let Some(object) = super::values::untyped_object_form(value) {
            adapted[index] = Some(object);
            continue;
        }
        let Some(type_value) = value.downcast_ref::<RuntimeTypeValue>() else { continue };
        adapted[index] = Some(type_value.to_declared(*handle).map_err(MarkupInvokeError::Failed)?);
    }
    Ok(Some(adapted))
}

/// Brings a value returned by an invoker to the form the interpreter works
/// with: an object is held in the handle of its run-time class, a class
/// reference is a `System.Type` value.
pub(crate) fn normalize_result(system: &Weak<RuntimeTypeSystem>, value: MarkupValue) -> MarkupValue {
    let value = value?;
    if let Some(type_info) = value.downcast_ref::<&'static TypeInfo>() {
        if let Some(system) = system.upgrade() {
            return Some(Rc::new(RuntimeTypeValue::new(system.type_of_class(type_info))));
        }
    }
    Some(normalize_object(value))
}

impl IXamlMember for RuntimeMethod {
    fn name(&self) -> String {
        self.name.clone()
    }
    fn declaring_type(&self) -> Rc<dyn IXamlType> {
        declaring(&self.declaring_type)
    }
}

impl IXamlMethod for RuntimeMethod {
    fn is_public(&self) -> bool {
        true
    }
    fn is_private(&self) -> bool {
        false
    }
    fn is_family(&self) -> bool {
        false
    }
    fn is_static(&self) -> bool {
        self.is_static
    }
    fn contains_generic_parameters(&self) -> bool {
        !self.generic_parameters.is_empty() && self.generic_arguments.is_empty()
    }
    fn is_generic_method(&self) -> bool {
        !self.generic_parameters.is_empty()
    }
    fn is_generic_method_definition(&self) -> bool {
        !self.generic_parameters.is_empty() && self.generic_arguments.is_empty()
    }
    fn return_type(&self) -> Rc<dyn IXamlType> {
        self.return_type.clone()
    }
    fn parameters(&self) -> Vec<Rc<dyn IXamlType>> {
        self.parameters.clone()
    }
    fn make_generic_method(&self, type_arguments: &[Rc<dyn IXamlType>]) -> XamlResult<Rc<dyn IXamlMethod>> {
        if !self.is_generic_method_definition() {
            return Err(XamlError::invalid_operation(format!(
                "{}.{} is not a generic method definition",
                declaring(&self.declaring_type).full_name(),
                self.name
            )));
        }
        if type_arguments.len() != self.generic_parameters.len() {
            return Err(XamlError::argument(format!(
                "{}.{} takes {} type argument(s), {} were given",
                declaring(&self.declaring_type).full_name(),
                self.name,
                self.generic_parameters.len(),
                type_arguments.len()
            )));
        }
        let parameters = &self.generic_parameters;
        Ok(Rc::new(RuntimeMethod {
            system: self.system.clone(),
            name: self.name.clone(),
            declaring_type: self.declaring_type.clone(),
            is_static: self.is_static,
            return_type: substitute(&self.return_type, parameters, type_arguments),
            parameters: self.parameters.iter().map(|p| substitute(p, parameters, type_arguments)).collect(),
            parameter_handles: self.parameter_handles.clone(),
            invoker: self.invoker.clone(),
            attributes: self.attributes.clone(),
            generic_parameters: self.generic_parameters.clone(),
            generic_arguments: type_arguments.to_vec(),
            declared: self.declared,
        }))
    }
    fn custom_attributes(&self) -> Vec<Rc<dyn IXamlCustomAttribute>> {
        self.attributes.clone()
    }
    fn get_parameter_info(&self, index: usize) -> XamlResult<Rc<dyn IXamlParameterInfo>> {
        match self.parameters.get(index) {
            Some(parameter) => Ok(Rc::new(AnonymousParameterInfo::with_index(parameter.clone(), index))),
            None => Err(XamlError::internal(
                "ArgumentOutOfRangeException",
                format!("Method {} doesn't have a parameter {index}", self.name),
            )),
        }
    }
    fn generic_parameters(&self) -> Vec<Rc<dyn IXamlType>> {
        self.generic_parameters.iter().map(|p| p.clone() as Rc<dyn IXamlType>).collect()
    }
    fn generic_arguments(&self) -> Vec<Rc<dyn IXamlType>> {
        self.generic_arguments.clone()
    }
    fn equals(&self, other: &dyn IXamlMethod) -> bool {
        match other.as_any().downcast_ref::<RuntimeMethod>() {
            Some(other) => {
                std::ptr::eq(self, other)
                    || (self.name == other.name
                        && self.is_static == other.is_static
                        && declaring(&self.declaring_type).equals(&*declaring(&other.declaring_type))
                        && self.return_type.equals(&*other.return_type)
                        && types_equal(&self.parameters, &other.parameters))
            }
            None => false,
        }
    }
    fn get_hash_code(&self) -> u64 {
        hash_str(&self.name)
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// A constructor projected from metadata, with its invoker.
pub struct RuntimeConstructor {
    pub(crate) system: Weak<RuntimeTypeSystem>,
    pub(crate) declaring_type: Weak<RuntimeType>,
    pub(crate) parameters: Vec<Rc<dyn IXamlType>>,
    pub(crate) parameter_handles: Vec<Option<ValueType>>,
    pub(crate) invoker: RuntimeInvoker,
    /// `false` for the constructor of a class that markup cannot create (the
    /// protected constructor of an abstract class of the managed original).
    pub(crate) is_public: bool,
    /// The custom attributes of the parameters (by index; a missing entry is a parameter
    /// without attributes).
    pub(crate) parameter_attributes: Vec<Vec<Rc<dyn IXamlCustomAttribute>>>,
}

/// A parameter of a projected constructor.
struct RuntimeParameterInfo {
    parameter_type: Rc<dyn IXamlType>,
    attributes: Vec<Rc<dyn IXamlCustomAttribute>>,
}

impl IXamlParameterInfo for RuntimeParameterInfo {
    fn parameter_type(&self) -> Rc<dyn IXamlType> {
        self.parameter_type.clone()
    }
    fn custom_attributes(&self) -> Vec<Rc<dyn IXamlCustomAttribute>> {
        self.attributes.clone()
    }
}

impl RuntimeConstructor {
    /// Creates an instance from the arguments, in untyped form.
    pub fn invoke(&self, arguments: &[MarkupValue]) -> Result<MarkupValue, MarkupInvokeError> {
        let invoke: &dyn Fn(&[MarkupValue]) -> Result<MarkupValue, MarkupInvokeError> = match &self.invoker {
            RuntimeInvoker::Static(invoke) => invoke,
            RuntimeInvoker::Dynamic(invoke) => &**invoke,
            _ => {
                return Err(MarkupInvokeError::Failed(format!(
                    "Type {} cannot be instantiated at run time: no instantiation with a constructor is registered",
                    declaring(&self.declaring_type).full_name()
                )))
            }
        };
        let adapted = adapt_arguments(arguments, &self.parameter_handles, 0)
            .map_err(|e| named(e, &format!("{}..ctor", declaring(&self.declaring_type).full_name())))?;
        let result = match adapted {
            Some(adapted) => invoke(&adapted),
            None => invoke(arguments),
        }?;
        Ok(normalize_result(&self.system, result))
    }

    pub(crate) fn substituted(
        &self,
        declaring_type: &Weak<RuntimeType>,
        parameters: &[Rc<RuntimeType>],
        arguments: &[Rc<dyn IXamlType>],
    ) -> Rc<RuntimeConstructor> {
        Rc::new(RuntimeConstructor {
            system: self.system.clone(),
            declaring_type: declaring_type.clone(),
            parameters: self.parameters.iter().map(|p| substitute(p, parameters, arguments)).collect(),
            parameter_handles: self.parameter_handles.clone(),
            invoker: self.invoker.clone(),
            is_public: self.is_public,
            parameter_attributes: self.parameter_attributes.clone(),
        })
    }
}

impl IXamlMember for RuntimeConstructor {
    fn name(&self) -> String {
        ".ctor".to_string()
    }
    fn declaring_type(&self) -> Rc<dyn IXamlType> {
        declaring(&self.declaring_type)
    }
}

impl IXamlConstructor for RuntimeConstructor {
    fn is_public(&self) -> bool {
        self.is_public
    }
    fn is_static(&self) -> bool {
        false
    }
    fn parameters(&self) -> Vec<Rc<dyn IXamlType>> {
        self.parameters.clone()
    }
    fn get_parameter_info(&self, index: usize) -> XamlResult<Rc<dyn IXamlParameterInfo>> {
        match self.parameters.get(index) {
            Some(parameter) => match self.parameter_attributes.get(index).filter(|a| !a.is_empty()) {
                Some(attributes) => Ok(Rc::new(RuntimeParameterInfo {
                    parameter_type: parameter.clone(),
                    attributes: attributes.clone(),
                })),
                None => Ok(Rc::new(AnonymousParameterInfo::with_index(parameter.clone(), index))),
            },
            None => Err(XamlError::internal(
                "ArgumentOutOfRangeException",
                format!("The constructor doesn't have a parameter {index}"),
            )),
        }
    }
    fn equals(&self, other: &dyn IXamlConstructor) -> bool {
        match other.as_any().downcast_ref::<RuntimeConstructor>() {
            Some(other) => {
                std::ptr::eq(self, other)
                    || (declaring(&self.declaring_type).equals(&*declaring(&other.declaring_type))
                        && types_equal(&self.parameters, &other.parameters))
            }
            None => false,
        }
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// A property: a plain property of the metadata, or a registered property
/// projected as a property with accessors.
pub struct RuntimeProperty {
    pub(crate) name: String,
    pub(crate) declaring_type: Weak<RuntimeType>,
    pub(crate) property_type: Rc<dyn IXamlType>,
    pub(crate) getter: Option<Rc<RuntimeMethod>>,
    pub(crate) setter: Option<Rc<RuntimeMethod>>,
    pub(crate) attributes: Vec<Rc<dyn IXamlCustomAttribute>>,
    /// The registered property this property is the projection of.
    pub(crate) ferro_property: Option<&'static FerroProperty>,
    /// The types of the index parameters of an indexer; empty for a property.
    pub(crate) indexer_parameters: Vec<Rc<dyn IXamlType>>,
}

impl RuntimeProperty {
    /// The registered property this property is the projection of.
    pub fn ferro_property(&self) -> Option<&'static FerroProperty> {
        self.ferro_property
    }

    /// The plain instance property of markup metadata (`properties:`) this property is
    /// the projection of.
    pub fn markup_property(&self) -> Option<&'static ferroui_base::metadata::MarkupProperty> {
        if self.ferro_property.is_some() || !self.indexer_parameters.is_empty() {
            return None;
        }
        let accessor = self.getter.as_ref().or(self.setter.as_ref())?;
        if accessor.is_static {
            return None;
        }
        self.declaring_type.upgrade()?.markup()?.find_property(&self.name)
    }

    pub fn getter_method(&self) -> Option<&Rc<RuntimeMethod>> {
        self.getter.as_ref()
    }

    pub fn setter_method(&self) -> Option<&Rc<RuntimeMethod>> {
        self.setter.as_ref()
    }
}

impl IXamlMember for RuntimeProperty {
    fn name(&self) -> String {
        self.name.clone()
    }
    fn declaring_type(&self) -> Rc<dyn IXamlType> {
        declaring(&self.declaring_type)
    }
}

impl IXamlProperty for RuntimeProperty {
    fn property_type(&self) -> Rc<dyn IXamlType> {
        self.property_type.clone()
    }
    fn setter(&self) -> Option<Rc<dyn IXamlMethod>> {
        self.setter.clone().map(|m| m as Rc<dyn IXamlMethod>)
    }
    fn getter(&self) -> Option<Rc<dyn IXamlMethod>> {
        self.getter.clone().map(|m| m as Rc<dyn IXamlMethod>)
    }
    fn custom_attributes(&self) -> Vec<Rc<dyn IXamlCustomAttribute>> {
        self.attributes.clone()
    }
    fn indexer_parameters(&self) -> Vec<Rc<dyn IXamlType>> {
        self.indexer_parameters.clone()
    }
    fn equals(&self, other: &dyn IXamlProperty) -> bool {
        match other.as_any().downcast_ref::<RuntimeProperty>() {
            Some(other) => {
                std::ptr::eq(self, other)
                    || (self.name == other.name
                        && declaring(&self.declaring_type).equals(&*declaring(&other.declaring_type)))
            }
            None => false,
        }
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// How the value of a [`RuntimeField`] is obtained.
#[derive(Clone)]
pub enum RuntimeFieldValue {
    /// The getter declared in metadata.
    Getter(fn() -> MarkupValue),
    /// The member of an enumeration, as a value of the enumeration.
    EnumMember(fn() -> BoxedValue),
    /// The definition of a registered property.
    Property(&'static FerroProperty),
    /// The field has no value at run time.
    None,
}

/// A static field: a registered property definition, an enumeration member
/// or a static value of the metadata.
pub struct RuntimeField {
    pub(crate) system: Weak<RuntimeTypeSystem>,
    pub(crate) name: String,
    pub(crate) declaring_type: Weak<RuntimeType>,
    pub(crate) field_type: Rc<dyn IXamlType>,
    pub(crate) literal: Option<XamlValue>,
    pub(crate) value: RuntimeFieldValue,
    pub(crate) attributes: Vec<Rc<dyn IXamlCustomAttribute>>,
}

impl RuntimeField {
    /// The value of the field, in untyped form.
    pub fn get(&self) -> Result<MarkupValue, MarkupInvokeError> {
        match &self.value {
            RuntimeFieldValue::Getter(get) => Ok(normalize_result(&self.system, get())),
            RuntimeFieldValue::EnumMember(get) => Ok(Some(get())),
            RuntimeFieldValue::Property(property) => Ok(Some(Rc::new(*property))),
            RuntimeFieldValue::None => Err(MarkupInvokeError::Failed(format!(
                "Field {}.{} has no value at run time",
                declaring(&self.declaring_type).full_name(),
                self.name
            ))),
        }
    }

    /// The registered property the field holds the definition of.
    pub fn ferro_property(&self) -> Option<&'static FerroProperty> {
        match self.value {
            RuntimeFieldValue::Property(property) => Some(property),
            _ => None,
        }
    }
}

impl IXamlMember for RuntimeField {
    fn name(&self) -> String {
        self.name.clone()
    }
    fn declaring_type(&self) -> Rc<dyn IXamlType> {
        declaring(&self.declaring_type)
    }
}

impl IXamlField for RuntimeField {
    fn field_type(&self) -> Rc<dyn IXamlType> {
        self.field_type.clone()
    }
    fn is_public(&self) -> bool {
        true
    }
    fn is_static(&self) -> bool {
        true
    }
    fn is_literal(&self) -> bool {
        self.literal.is_some()
    }
    fn get_literal_value(&self) -> XamlResult<XamlValue> {
        self.literal
            .clone()
            .ok_or_else(|| XamlError::invalid_operation(format!("Field {} is not a literal", self.name)))
    }
    fn custom_attributes(&self) -> Vec<Rc<dyn IXamlCustomAttribute>> {
        self.attributes.clone()
    }
    fn equals(&self, other: &dyn IXamlField) -> bool {
        match other.as_any().downcast_ref::<RuntimeField>() {
            Some(other) => {
                std::ptr::eq(self, other)
                    || (self.name == other.name
                        && declaring(&self.declaring_type).equals(&*declaring(&other.declaring_type)))
            }
            None => false,
        }
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// A plain (non-routed) event of the metadata.
pub struct RuntimeEvent {
    pub(crate) name: String,
    pub(crate) declaring_type: Weak<RuntimeType>,
    pub(crate) add: Option<Rc<RuntimeMethod>>,
}

impl IXamlMember for RuntimeEvent {
    fn name(&self) -> String {
        self.name.clone()
    }
    fn declaring_type(&self) -> Rc<dyn IXamlType> {
        declaring(&self.declaring_type)
    }
}

impl IXamlEventInfo for RuntimeEvent {
    fn add(&self) -> Option<Rc<dyn IXamlMethod>> {
        self.add.clone().map(|m| m as Rc<dyn IXamlMethod>)
    }
    fn equals(&self, other: &dyn IXamlEventInfo) -> bool {
        match other.as_any().downcast_ref::<RuntimeEvent>() {
            Some(other) => {
                std::ptr::eq(self, other)
                    || (self.name == other.name
                        && declaring(&self.declaring_type).equals(&*declaring(&other.declaring_type)))
            }
            None => false,
        }
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// An assembly: a registered crate, or the core assembly of the runtime
/// library types.
pub struct RuntimeAssembly {
    pub(crate) system: Weak<RuntimeTypeSystem>,
    pub(crate) name: String,
    /// Other names the assembly answers to (the names of the runtime
    /// library assemblies markup may state).
    pub(crate) aliases: Vec<&'static str>,
    /// The crate, as module paths spell it. `None` for the core assembly.
    pub(crate) crate_name: Option<String>,
    pub(crate) attributes: RefCell<Vec<Rc<dyn IXamlCustomAttribute>>>,
}

impl RuntimeAssembly {
    pub(crate) fn has_name(&self, name: &str) -> bool {
        self.name.eq_ignore_ascii_case(name) || self.aliases.iter().any(|alias| alias.eq_ignore_ascii_case(name))
    }

    pub fn crate_name(&self) -> Option<&str> {
        self.crate_name.as_deref()
    }
}

impl IXamlAssembly for RuntimeAssembly {
    fn name(&self) -> String {
        self.name.clone()
    }
    fn custom_attributes(&self) -> Vec<Rc<dyn IXamlCustomAttribute>> {
        self.attributes.borrow().clone()
    }
    fn find_type(&self, full_name: &str) -> Option<Rc<dyn IXamlType>> {
        let system = self.system.upgrade()?;
        let found = system.find_runtime_type(full_name)?;
        system.type_belongs_to(&found, self).then(|| found as Rc<dyn IXamlType>)
    }
    fn equals(&self, other: &dyn IXamlAssembly) -> bool {
        std::ptr::addr_eq(self as *const Self, other.as_any() as *const dyn Any)
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// Converts an untyped value to exactly the Rust type `target` with the
/// assignability casts of the untyped value conversions.
pub(crate) fn to_exact(value: &MarkupValue, target: ValueType, index: usize) -> Result<BoxedValue, MarkupInvokeError> {
    // A `System.Type` value is passed in the representation the target declares.
    if let Some(type_value) = value.as_ref().and_then(|v| v.downcast_ref::<RuntimeTypeValue>()) {
        return type_value.to_declared(target).map_err(MarkupInvokeError::Failed);
    }
    if let Some(array) = value.as_ref().and_then(|v| v.downcast_ref::<RuntimeArray>()) {
        return array.to_declared(target).map_err(MarkupInvokeError::Failed);
    }
    if target.is_object() {
        if let Some(object) = value.as_ref().and_then(super::values::untyped_object_form) {
            return to_exact(&Some(object), target, index);
        }
    }
    let converted = match value {
        Some(boxed) if boxed.value_type_id() == target.id() => Some(boxed.clone()),
        Some(boxed) => ValueTypes::try_cast(boxed, target),
        None => ValueTypes::try_convert(None, target).flatten(),
    };
    converted.ok_or_else(|| MarkupInvokeError::Argument {
        index,
        expected: target.name(),
        actual: match value {
            Some(value) => value.type_name().to_string(),
            None => "null".to_string(),
        },
    })
}
