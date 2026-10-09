//! The types of the build-time type system and their members.
//!
//! A [`ModelType`] is a cheap shell (name, kind, where it comes from); its
//! base type, interfaces and members are projected from the model the first
//! time one of them is asked for, as the types of the run-time type system
//! are projected from the registries. A member has no invoker: it carries
//! where the declaration it is the projection of is ([`MemberSource`]), which
//! is what generated code is written from.

use std::any::Any;
use std::cell::{Cell, RefCell};
use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::rc::{Rc, Weak};

use xamlx::exceptions::{XamlError, XamlResult};
use xamlx::type_system::{
    AnonymousParameterInfo, IXamlAssembly, IXamlConstructor, IXamlCustomAttribute, IXamlEventInfo, IXamlField, IXamlMember, IXamlMethod,
    IXamlParameterInfo, IXamlProperty, IXamlType, XamlPseudoType, XamlTypeId, XamlValue,
};

use crate::model::CallableModel;

use super::model_type_system::ModelTypeSystem;

/// What kind of type a [`ModelType`] is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ModelTypeKind {
    Class,
    Interface,
    Struct,
    Enum,
    GenericParameter,
}

/// What a [`ModelType`] is projected from.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ModelTypeOrigin {
    /// A type of the closed table of runtime library types, or a type the type system
    /// defines itself.
    Synthetic,
    /// A type of a model, by the positions of its model and of the type in it.
    Model { model: usize, type_: usize },
    /// A Rust type no model declares, by its normalised text.
    Opaque(String),
    GenericParameter,
    /// An instantiation of a synthetic generic definition; its members are the
    /// substituted members of the definition.
    Instantiation,
    Array,
}

/// The declaration a member is the projection of: what the emitter of Rust source writes
/// the call of the member from (docs/porting/xaml.md, 9.5.3).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MemberSource {
    /// A member the type system defines: of the table of runtime library types, or one of
    /// the members of the object model no declaration states.
    TypeSystem,
    /// A registered property, by the path of the type whose `ferro_properties!` block
    /// has the accessor and the name of the accessor (`Type::name_property()`).
    Registered { type_path: String, accessor: String },
    /// `new:` of a class.
    DefaultConstructor { type_path: String, callable: CallableModel },
    /// A member of markup metadata: a constructor, a method, a static field, the
    /// subscription of an event, an accessor of a plain property or `parse:`.
    Declared { type_path: String, callable: CallableModel, typed_function: Option<String>, fallible: bool },
}

/// The lazily projected part of a [`ModelType`].
#[derive(Default)]
pub struct ModelMembers {
    pub base_type: Option<Rc<dyn IXamlType>>,
    /// The interfaces the type declares (without the ones they inherit).
    pub interfaces: Vec<Rc<dyn IXamlType>>,
    pub properties: Vec<Rc<ModelProperty>>,
    pub events: Vec<Rc<ModelEvent>>,
    pub fields: Vec<Rc<ModelField>>,
    pub methods: Vec<Rc<ModelMethod>>,
    pub constructors: Vec<Rc<ModelConstructor>>,
    pub attributes: Vec<Rc<dyn IXamlCustomAttribute>>,
    pub enum_underlying_type: Option<Rc<dyn IXamlType>>,
}

pub(crate) type MembersInit = Box<dyn FnOnce(&Rc<ModelType>) -> ModelMembers>;

/// A type of the build-time type system.
pub struct ModelType {
    this: Weak<ModelType>,
    pub(crate) system: Weak<ModelTypeSystem>,
    key: String,
    hash: u64,
    namespace: String,
    name: String,
    kind: ModelTypeKind,
    origin: ModelTypeOrigin,
    assembly: RefCell<Option<Weak<ModelAssembly>>>,
    generic_parameters: Vec<Rc<ModelType>>,
    generic_definition: Option<Rc<ModelType>>,
    generic_arguments: Vec<Rc<dyn IXamlType>>,
    /// The Rust types that hold values of the type, as canonical text, the untyped form first.
    handles: RefCell<Vec<String>>,
    members: RefCell<Option<Rc<ModelMembers>>>,
    initializing: Cell<bool>,
    init: RefCell<Option<MembersInit>>,
    array_element_type: Option<Rc<dyn IXamlType>>,
    arrays: RefCell<HashMap<i32, Rc<ModelType>>>,
}

/// What [`ModelType::create`] needs to know about a type.
pub(crate) struct ModelTypeSpec {
    pub key: String,
    pub namespace: String,
    pub name: String,
    pub kind: ModelTypeKind,
    pub origin: ModelTypeOrigin,
    pub generic_parameter_names: Vec<String>,
    pub generic_definition: Option<Rc<ModelType>>,
    pub generic_arguments: Vec<Rc<dyn IXamlType>>,
    pub handles: Vec<String>,
    pub array_element_type: Option<Rc<dyn IXamlType>>,
    pub init: Option<MembersInit>,
}

impl ModelTypeSpec {
    pub fn new(key: String, namespace: &str, name: &str, kind: ModelTypeKind, origin: ModelTypeOrigin) -> Self {
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

fn hash_str(text: &str) -> u64 {
    let mut hasher = DefaultHasher::new();
    text.hash(&mut hasher);
    hasher.finish()
}

/// The identity of a type as a string: the key of a [`ModelType`], the debug form of the
/// identifier of any other type.
pub(crate) fn type_key(type_: &Rc<dyn IXamlType>) -> String {
    match type_.as_any().downcast_ref::<ModelType>() {
        Some(model) => model.key.clone(),
        None => format!("{:?}", type_.id()),
    }
}

impl ModelType {
    pub(crate) fn create(system: &Weak<ModelTypeSystem>, spec: ModelTypeSpec) -> Rc<ModelType> {
        Rc::new_cyclic(|this| {
            let generic_parameters = spec
                .generic_parameter_names
                .iter()
                .map(|parameter| {
                    ModelType::create(
                        system,
                        ModelTypeSpec::new(
                            format!("{}!{parameter}", spec.key),
                            "",
                            parameter,
                            ModelTypeKind::GenericParameter,
                            ModelTypeOrigin::GenericParameter,
                        ),
                    )
                })
                .collect();
            ModelType {
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

    pub fn kind(&self) -> ModelTypeKind {
        self.kind
    }

    pub fn origin(&self) -> &ModelTypeOrigin {
        &self.origin
    }

    /// Whether the type is a Rust type no model declares: assignable only to itself and
    /// to `System.Object`, without members.
    pub fn is_opaque(&self) -> bool {
        matches!(self.origin, ModelTypeOrigin::Opaque(_))
    }

    /// The Rust types that hold values of the type, as canonical text, the untyped
    /// (canonical) form first. Empty for types without values and for synthetic types
    /// whose values no declaration can name.
    pub fn handles(&self) -> Vec<String> {
        self.handles.borrow().clone()
    }

    pub(crate) fn add_handle(&self, handle: &str) {
        let mut handles = self.handles.borrow_mut();
        if !handles.iter().any(|known| known == handle) {
            handles.push(handle.to_string());
        }
    }

    pub(crate) fn set_assembly(&self, assembly: &Rc<ModelAssembly>) {
        *self.assembly.borrow_mut() = Some(Rc::downgrade(assembly));
    }

    pub(crate) fn model_assembly(&self) -> Option<Rc<ModelAssembly>> {
        self.assembly.borrow().as_ref().and_then(Weak::upgrade)
    }

    /// This type as a type system handle.
    pub fn as_type(&self) -> Rc<dyn IXamlType> {
        match self.this.upgrade() {
            Some(this) => this,
            None => XamlPseudoType::unknown(),
        }
    }

    pub(crate) fn definition(&self) -> Option<&Rc<ModelType>> {
        self.generic_definition.as_ref()
    }

    pub(crate) fn generic_parameter_types(&self) -> &[Rc<ModelType>] {
        &self.generic_parameters
    }

    pub(crate) fn generic_argument_types(&self) -> &[Rc<dyn IXamlType>] {
        &self.generic_arguments
    }

    /// The projected members, projecting them on first use.
    pub fn members(&self) -> Rc<ModelMembers> {
        if let Some(members) = self.members.borrow().as_ref() {
            return members.clone();
        }
        // A projection that (through a cycle in the model) asks for the members being
        // projected sees a type without members.
        if self.initializing.replace(true) {
            return Rc::new(ModelMembers::default());
        }
        let init = self.init.borrow_mut().take();
        let members = match (self.this.upgrade(), init, self.system.upgrade()) {
            (Some(this), Some(init), _) => init(&this),
            (Some(this), None, Some(system)) => system.project_members(&this),
            _ => ModelMembers::default(),
        };
        let members = Rc::new(members);
        *self.members.borrow_mut() = Some(members.clone());
        self.initializing.set(false);
        members
    }

    pub(crate) fn find_array(&self, dimensions: i32) -> Option<Rc<ModelType>> {
        self.arrays.borrow().get(&dimensions).cloned()
    }
}

/// Replaces the generic parameters of a definition in `type_` by the arguments of an
/// instantiation.
pub(crate) fn substitute(type_: &Rc<dyn IXamlType>, parameters: &[Rc<ModelType>], arguments: &[Rc<dyn IXamlType>]) -> Rc<dyn IXamlType> {
    let Some(model) = type_.as_any().downcast_ref::<ModelType>() else {
        return type_.clone();
    };
    if model.kind == ModelTypeKind::GenericParameter {
        return match parameters.iter().position(|parameter| parameter.key == model.key) {
            Some(index) if index < arguments.len() => arguments[index].clone(),
            _ => type_.clone(),
        };
    }
    if let Some(definition) = model.definition() {
        let new_arguments: Vec<Rc<dyn IXamlType>> = model.generic_arguments.iter().map(|argument| substitute(argument, parameters, arguments)).collect();
        let unchanged = new_arguments.iter().zip(model.generic_arguments.iter()).all(|(new, old)| new.equals(&**old));
        if !unchanged {
            if let Ok(instance) = definition.make_generic_type(&new_arguments) {
                return instance;
            }
        }
    }
    type_.clone()
}

impl IXamlType for ModelType {
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
        let mut full_name = if self.namespace.is_empty() { self.name.clone() } else { format!("{}.{}", self.namespace, self.name) };
        if !self.generic_arguments.is_empty() {
            let arguments: Vec<String> = self.generic_arguments.iter().map(|argument| argument.full_name()).collect();
            full_name = format!("{full_name}[{}]", arguments.join(","));
        }
        full_name
    }
    fn is_public(&self) -> bool {
        true
    }
    fn is_nested_private(&self) -> bool {
        false
    }
    fn assembly(&self) -> Option<Rc<dyn IXamlAssembly>> {
        self.model_assembly().map(|assembly| assembly as Rc<dyn IXamlAssembly>)
    }
    fn properties(&self) -> Vec<Rc<dyn IXamlProperty>> {
        self.members().properties.iter().map(|property| property.clone() as Rc<dyn IXamlProperty>).collect()
    }
    fn events(&self) -> Vec<Rc<dyn IXamlEventInfo>> {
        self.members().events.iter().map(|event| event.clone() as Rc<dyn IXamlEventInfo>).collect()
    }
    fn fields(&self) -> Vec<Rc<dyn IXamlField>> {
        self.members().fields.iter().map(|field| field.clone() as Rc<dyn IXamlField>).collect()
    }
    fn methods(&self) -> Vec<Rc<dyn IXamlMethod>> {
        self.members().methods.iter().map(|method| method.clone() as Rc<dyn IXamlMethod>).collect()
    }
    fn constructors(&self) -> Vec<Rc<dyn IXamlConstructor>> {
        self.members().constructors.iter().map(|constructor| constructor.clone() as Rc<dyn IXamlConstructor>).collect()
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
        while let Some(base) = base_type {
            if base.equals(self) {
                return true;
            }
            base_type = base.base_type();
        }
        self.is_interface() && type_.get_all_interfaces().iter().any(|interface| self.is_assignable_from(&**interface))
    }
    fn make_generic_type(&self, type_arguments: &[Rc<dyn IXamlType>]) -> XamlResult<Rc<dyn IXamlType>> {
        if self.generic_parameters.is_empty() {
            return Err(XamlError::invalid_operation(format!("{} is not a generic type definition", self.full_name())));
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
        self.generic_definition.clone().map(|definition| definition as Rc<dyn IXamlType>)
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
        matches!(self.kind, ModelTypeKind::Struct | ModelTypeKind::Enum)
    }
    fn is_enum(&self) -> bool {
        self.kind == ModelTypeKind::Enum
    }
    fn interfaces(&self) -> Vec<Rc<dyn IXamlType>> {
        // Declared interfaces followed by the interfaces they inherit.
        let mut interfaces: Vec<Rc<dyn IXamlType>> = Vec::new();
        for declared in self.members().interfaces.iter() {
            let inherited = declared.interfaces();
            for candidate in std::iter::once(declared.clone()).chain(inherited) {
                if !interfaces.iter().any(|existing| existing.equals(&*candidate)) {
                    interfaces.push(candidate);
                }
            }
        }
        interfaces
    }
    fn is_interface(&self) -> bool {
        self.kind == ModelTypeKind::Interface
    }
    fn get_enum_underlying_type(&self) -> XamlResult<Rc<dyn IXamlType>> {
        self.members()
            .enum_underlying_type
            .clone()
            .ok_or_else(|| XamlError::invalid_operation(format!("{} is not an enum", self.full_name())))
    }
    fn generic_parameters(&self) -> Vec<Rc<dyn IXamlType>> {
        self.generic_parameters.iter().map(|parameter| parameter.clone() as Rc<dyn IXamlType>).collect()
    }
    fn is_function_pointer(&self) -> bool {
        false
    }
    fn equals(&self, other: &dyn IXamlType) -> bool {
        match other.as_any().downcast_ref::<ModelType>() {
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

fn declaring(declaring_type: &Weak<ModelType>) -> Rc<dyn IXamlType> {
    match declaring_type.upgrade() {
        Some(type_) => type_,
        None => XamlPseudoType::unknown(),
    }
}

fn types_equal(left: &[Rc<dyn IXamlType>], right: &[Rc<dyn IXamlType>]) -> bool {
    left.len() == right.len() && left.iter().zip(right).all(|(left, right)| left.equals(&**right))
}

/// A custom attribute projected from the model.
pub struct ModelCustomAttribute {
    type_: Rc<dyn IXamlType>,
    parameters: Vec<XamlValue>,
    properties: Vec<(String, XamlValue)>,
}

impl ModelCustomAttribute {
    pub fn new(type_: Rc<dyn IXamlType>, parameters: Vec<XamlValue>, properties: Vec<(String, XamlValue)>) -> Rc<dyn IXamlCustomAttribute> {
        Rc::new(Self { type_, parameters, properties })
    }
}

impl IXamlCustomAttribute for ModelCustomAttribute {
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

/// A method projected from the model.
pub struct ModelMethod {
    pub(crate) name: String,
    pub(crate) declaring_type: Weak<ModelType>,
    pub(crate) is_static: bool,
    pub(crate) return_type: Rc<dyn IXamlType>,
    pub(crate) parameters: Vec<Rc<dyn IXamlType>>,
    pub(crate) attributes: Vec<Rc<dyn IXamlCustomAttribute>>,
    /// The type parameters of a generic method definition (the type system synthesizes the
    /// few generic members of the object model; no declaration states one).
    pub(crate) generic_parameters: Vec<Rc<ModelType>>,
    /// The type arguments of a constructed generic method.
    pub(crate) generic_arguments: Vec<Rc<dyn IXamlType>>,
    /// An abstract member of a type of the table of runtime library types.
    pub(crate) is_abstract: bool,
    pub(crate) source: MemberSource,
}

impl ModelMethod {
    /// The declaration the method is the projection of.
    pub fn source(&self) -> &MemberSource {
        &self.source
    }

    /// Whether the method is an abstract member of a runtime library type: a call is
    /// dispatched to the member of the same name of the type of the instance.
    pub fn is_abstract(&self) -> bool {
        self.is_abstract
    }

    pub(crate) fn substituted(&self, declaring_type: &Weak<ModelType>, parameters: &[Rc<ModelType>], arguments: &[Rc<dyn IXamlType>]) -> Rc<ModelMethod> {
        Rc::new(ModelMethod {
            name: self.name.clone(),
            declaring_type: declaring_type.clone(),
            is_static: self.is_static,
            return_type: substitute(&self.return_type, parameters, arguments),
            parameters: self.parameters.iter().map(|parameter| substitute(parameter, parameters, arguments)).collect(),
            attributes: self.attributes.clone(),
            generic_parameters: self.generic_parameters.clone(),
            generic_arguments: self.generic_arguments.clone(),
            is_abstract: self.is_abstract,
            source: self.source.clone(),
        })
    }
}

impl IXamlMember for ModelMethod {
    fn name(&self) -> String {
        self.name.clone()
    }
    fn declaring_type(&self) -> Rc<dyn IXamlType> {
        declaring(&self.declaring_type)
    }
}

impl IXamlMethod for ModelMethod {
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
        Ok(Rc::new(ModelMethod {
            name: self.name.clone(),
            declaring_type: self.declaring_type.clone(),
            is_static: self.is_static,
            return_type: substitute(&self.return_type, parameters, type_arguments),
            parameters: self.parameters.iter().map(|parameter| substitute(parameter, parameters, type_arguments)).collect(),
            attributes: self.attributes.clone(),
            generic_parameters: self.generic_parameters.clone(),
            generic_arguments: type_arguments.to_vec(),
            is_abstract: self.is_abstract,
            source: self.source.clone(),
        }))
    }
    fn custom_attributes(&self) -> Vec<Rc<dyn IXamlCustomAttribute>> {
        self.attributes.clone()
    }
    fn get_parameter_info(&self, index: usize) -> XamlResult<Rc<dyn IXamlParameterInfo>> {
        match self.parameters.get(index) {
            Some(parameter) => Ok(Rc::new(AnonymousParameterInfo::with_index(parameter.clone(), index))),
            None => Err(XamlError::internal("ArgumentOutOfRangeException", format!("Method {} doesn't have a parameter {index}", self.name))),
        }
    }
    fn generic_parameters(&self) -> Vec<Rc<dyn IXamlType>> {
        self.generic_parameters.iter().map(|parameter| parameter.clone() as Rc<dyn IXamlType>).collect()
    }
    fn generic_arguments(&self) -> Vec<Rc<dyn IXamlType>> {
        self.generic_arguments.clone()
    }
    fn equals(&self, other: &dyn IXamlMethod) -> bool {
        match other.as_any().downcast_ref::<ModelMethod>() {
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

/// A constructor projected from the model.
pub struct ModelConstructor {
    pub(crate) declaring_type: Weak<ModelType>,
    pub(crate) parameters: Vec<Rc<dyn IXamlType>>,
    /// `false` for the constructor of a class that markup cannot create (the protected
    /// constructor of an abstract class of the managed original).
    pub(crate) is_public: bool,
    /// The custom attributes of the parameters (by index; a missing entry is a parameter
    /// without attributes).
    pub(crate) parameter_attributes: Vec<Vec<Rc<dyn IXamlCustomAttribute>>>,
    pub(crate) source: MemberSource,
}

/// A parameter of a projected constructor.
struct ModelParameterInfo {
    parameter_type: Rc<dyn IXamlType>,
    attributes: Vec<Rc<dyn IXamlCustomAttribute>>,
}

impl IXamlParameterInfo for ModelParameterInfo {
    fn parameter_type(&self) -> Rc<dyn IXamlType> {
        self.parameter_type.clone()
    }
    fn custom_attributes(&self) -> Vec<Rc<dyn IXamlCustomAttribute>> {
        self.attributes.clone()
    }
}

impl ModelConstructor {
    /// The declaration the constructor is the projection of.
    pub fn source(&self) -> &MemberSource {
        &self.source
    }

    pub(crate) fn substituted(&self, declaring_type: &Weak<ModelType>, parameters: &[Rc<ModelType>], arguments: &[Rc<dyn IXamlType>]) -> Rc<ModelConstructor> {
        Rc::new(ModelConstructor {
            declaring_type: declaring_type.clone(),
            parameters: self.parameters.iter().map(|parameter| substitute(parameter, parameters, arguments)).collect(),
            is_public: self.is_public,
            parameter_attributes: self.parameter_attributes.clone(),
            source: self.source.clone(),
        })
    }
}

impl IXamlMember for ModelConstructor {
    fn name(&self) -> String {
        ".ctor".to_string()
    }
    fn declaring_type(&self) -> Rc<dyn IXamlType> {
        declaring(&self.declaring_type)
    }
}

impl IXamlConstructor for ModelConstructor {
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
            Some(parameter) => match self.parameter_attributes.get(index).filter(|attributes| !attributes.is_empty()) {
                Some(attributes) => Ok(Rc::new(ModelParameterInfo { parameter_type: parameter.clone(), attributes: attributes.clone() })),
                None => Ok(Rc::new(AnonymousParameterInfo::with_index(parameter.clone(), index))),
            },
            None => Err(XamlError::internal("ArgumentOutOfRangeException", format!("The constructor doesn't have a parameter {index}"))),
        }
    }
    fn equals(&self, other: &dyn IXamlConstructor) -> bool {
        match other.as_any().downcast_ref::<ModelConstructor>() {
            Some(other) => {
                std::ptr::eq(self, other)
                    || (declaring(&self.declaring_type).equals(&*declaring(&other.declaring_type)) && types_equal(&self.parameters, &other.parameters))
            }
            None => false,
        }
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// A property: a plain property of the model, or a registered property projected as a
/// property with accessors.
pub struct ModelProperty {
    pub(crate) name: String,
    pub(crate) declaring_type: Weak<ModelType>,
    pub(crate) property_type: Rc<dyn IXamlType>,
    pub(crate) getter: Option<Rc<ModelMethod>>,
    pub(crate) setter: Option<Rc<ModelMethod>>,
    pub(crate) attributes: Vec<Rc<dyn IXamlCustomAttribute>>,
    /// The types of the index parameters of an indexer; empty for a property.
    pub(crate) indexer_parameters: Vec<Rc<dyn IXamlType>>,
    /// The registered property the property is the projection of: the path of the type
    /// that has the accessor and the name of the accessor.
    pub(crate) registered: Option<(String, String)>,
}

impl ModelProperty {
    /// The registered property this property is the projection of: the path of the type
    /// whose `ferro_properties!` block has the accessor, and the accessor.
    pub fn registered(&self) -> Option<(&str, &str)> {
        self.registered.as_ref().map(|(type_path, accessor)| (type_path.as_str(), accessor.as_str()))
    }

    pub fn getter_method(&self) -> Option<&Rc<ModelMethod>> {
        self.getter.as_ref()
    }

    pub fn setter_method(&self) -> Option<&Rc<ModelMethod>> {
        self.setter.as_ref()
    }
}

impl IXamlMember for ModelProperty {
    fn name(&self) -> String {
        self.name.clone()
    }
    fn declaring_type(&self) -> Rc<dyn IXamlType> {
        declaring(&self.declaring_type)
    }
}

impl IXamlProperty for ModelProperty {
    fn property_type(&self) -> Rc<dyn IXamlType> {
        self.property_type.clone()
    }
    fn setter(&self) -> Option<Rc<dyn IXamlMethod>> {
        self.setter.clone().map(|method| method as Rc<dyn IXamlMethod>)
    }
    fn getter(&self) -> Option<Rc<dyn IXamlMethod>> {
        self.getter.clone().map(|method| method as Rc<dyn IXamlMethod>)
    }
    fn custom_attributes(&self) -> Vec<Rc<dyn IXamlCustomAttribute>> {
        self.attributes.clone()
    }
    fn indexer_parameters(&self) -> Vec<Rc<dyn IXamlType>> {
        self.indexer_parameters.clone()
    }
    fn equals(&self, other: &dyn IXamlProperty) -> bool {
        match other.as_any().downcast_ref::<ModelProperty>() {
            Some(other) => {
                std::ptr::eq(self, other) || (self.name == other.name && declaring(&self.declaring_type).equals(&*declaring(&other.declaring_type)))
            }
            None => false,
        }
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// A static field: a registered property definition, an enumeration member or a static
/// value of the model.
pub struct ModelField {
    pub(crate) name: String,
    pub(crate) declaring_type: Weak<ModelType>,
    pub(crate) field_type: Rc<dyn IXamlType>,
    pub(crate) literal: Option<XamlValue>,
    pub(crate) attributes: Vec<Rc<dyn IXamlCustomAttribute>>,
    pub(crate) source: MemberSource,
}

impl ModelField {
    /// The declaration the field is the projection of.
    pub fn source(&self) -> &MemberSource {
        &self.source
    }
}

impl IXamlMember for ModelField {
    fn name(&self) -> String {
        self.name.clone()
    }
    fn declaring_type(&self) -> Rc<dyn IXamlType> {
        declaring(&self.declaring_type)
    }
}

impl IXamlField for ModelField {
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
        self.literal.clone().ok_or_else(|| XamlError::invalid_operation(format!("Field {} is not a literal", self.name)))
    }
    fn custom_attributes(&self) -> Vec<Rc<dyn IXamlCustomAttribute>> {
        self.attributes.clone()
    }
    fn equals(&self, other: &dyn IXamlField) -> bool {
        match other.as_any().downcast_ref::<ModelField>() {
            Some(other) => {
                std::ptr::eq(self, other) || (self.name == other.name && declaring(&self.declaring_type).equals(&*declaring(&other.declaring_type)))
            }
            None => false,
        }
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// A plain (non-routed) event of the model.
pub struct ModelEvent {
    pub(crate) name: String,
    pub(crate) declaring_type: Weak<ModelType>,
    pub(crate) add: Option<Rc<ModelMethod>>,
}

impl IXamlMember for ModelEvent {
    fn name(&self) -> String {
        self.name.clone()
    }
    fn declaring_type(&self) -> Rc<dyn IXamlType> {
        declaring(&self.declaring_type)
    }
}

impl IXamlEventInfo for ModelEvent {
    fn add(&self) -> Option<Rc<dyn IXamlMethod>> {
        self.add.clone().map(|method| method as Rc<dyn IXamlMethod>)
    }
    fn equals(&self, other: &dyn IXamlEventInfo) -> bool {
        match other.as_any().downcast_ref::<ModelEvent>() {
            Some(other) => {
                std::ptr::eq(self, other) || (self.name == other.name && declaring(&self.declaring_type).equals(&*declaring(&other.declaring_type)))
            }
            None => false,
        }
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// An assembly: the crate of a model, or the core assembly of the runtime library types.
pub struct ModelAssembly {
    pub(crate) system: Weak<ModelTypeSystem>,
    pub(crate) name: String,
    /// Other names the assembly answers to (the names of the runtime library assemblies
    /// markup may state).
    pub(crate) aliases: Vec<&'static str>,
    /// The crate, as module paths spell it. `None` for the core assembly.
    pub(crate) crate_name: Option<String>,
    /// The position of the model of the crate in the set of the type system.
    pub(crate) model: Option<usize>,
    pub(crate) attributes: RefCell<Option<Vec<Rc<dyn IXamlCustomAttribute>>>>,
}

impl ModelAssembly {
    pub(crate) fn has_name(&self, name: &str) -> bool {
        self.name.eq_ignore_ascii_case(name) || self.aliases.iter().any(|alias| alias.eq_ignore_ascii_case(name))
    }

    pub fn crate_name(&self) -> Option<&str> {
        self.crate_name.as_deref()
    }
}

impl IXamlAssembly for ModelAssembly {
    fn name(&self) -> String {
        self.name.clone()
    }
    fn custom_attributes(&self) -> Vec<Rc<dyn IXamlCustomAttribute>> {
        if let Some(attributes) = self.attributes.borrow().as_ref() {
            return attributes.clone();
        }
        let attributes = match self.system.upgrade() {
            Some(system) => system.assembly_attributes(self),
            None => Vec::new(),
        };
        *self.attributes.borrow_mut() = Some(attributes.clone());
        attributes
    }
    fn find_type(&self, full_name: &str) -> Option<Rc<dyn IXamlType>> {
        let system = self.system.upgrade()?;
        let found = system.find_model_type(full_name)?;
        system.type_belongs_to(&found, self).then(|| found as Rc<dyn IXamlType>)
    }
    fn equals(&self, other: &dyn IXamlAssembly) -> bool {
        std::ptr::addr_eq(self as *const Self, other.as_any() as *const dyn Any)
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}
