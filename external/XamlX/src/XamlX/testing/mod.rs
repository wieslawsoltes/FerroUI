//! An in-memory ("fake") implementation of the type system abstraction.
//!
//! It exists so that transformers (and emitter backends) can be tested without a real metadata
//! reader. Types are declared programmatically:
//!
//! ```text
//! let ts = FakeTypeSystem::new();
//! let asm = ts.define_assembly("Tests");
//! let control = asm.define_class("Tests", "Control");
//! control.add_constructor(vec![]);
//! control.add_property("Text", ts.get("System.String"));
//! ```
//!
//! Assignability follows the rules of the upstream metadata based implementation. Generic types
//! are supported (members of constructed types are substituted lazily), as are generic methods
//! ([`FakeType::add_generic_method`]) and arrays (`make_array_type`); function pointers are not.

use std::any::Any;
use std::cell::{Cell, OnceCell, RefCell};
use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::rc::{Rc, Weak};

use crate::exceptions::{XamlError, XamlResult};
use crate::type_system::{
    xaml_types_sequence_equal, IXamlAssembly, IXamlConstructor, IXamlCustomAttribute,
    IXamlEventInfo, IXamlField, IXamlMember, IXamlMethod, IXamlParameterInfo, IXamlProperty,
    IXamlType, IXamlTypeSystem, XamlPseudoType, XamlTypeId, XamlTypeWellKnownTypes, XamlValue,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FakeTypeKind {
    Class,
    Interface,
    Struct,
    Enum,
    GenericParameter,
}

fn type_key(type_: &Rc<dyn IXamlType>) -> String {
    match type_.as_any().downcast_ref::<FakeType>() {
        Some(fake) => fake.key.clone(),
        None => format!("{:?}", type_.id()),
    }
}

fn hash_str(s: &str) -> u64 {
    let mut hasher = DefaultHasher::new();
    s.hash(&mut hasher);
    hasher.finish()
}

fn substitute(
    type_: &Rc<dyn IXamlType>,
    parameters: &[Rc<FakeType>],
    arguments: &[Rc<dyn IXamlType>],
) -> Rc<dyn IXamlType> {
    if let Some(fake) = type_.as_any().downcast_ref::<FakeType>() {
        if fake.kind.get() == FakeTypeKind::GenericParameter {
            return match parameters.iter().position(|p| p.key == fake.key) {
                Some(index) => arguments[index].clone(),
                None => type_.clone(),
            };
        }
        if let Some(definition) = fake.definition() {
            let new_arguments: Vec<Rc<dyn IXamlType>> = fake
                .generic_arguments
                .iter()
                .map(|a| substitute(a, parameters, arguments))
                .collect();
            return definition.instantiate(&new_arguments);
        }
    }
    type_.clone()
}

pub struct FakeCustomAttribute {
    pub type_: Rc<dyn IXamlType>,
    pub parameters: Vec<XamlValue>,
    pub properties: HashMap<String, XamlValue>,
}

impl FakeCustomAttribute {
    pub fn new(type_: Rc<dyn IXamlType>, parameters: Vec<XamlValue>) -> Rc<Self> {
        Rc::new(Self {
            type_,
            parameters,
            properties: HashMap::new(),
        })
    }

    pub fn with_properties(
        type_: Rc<dyn IXamlType>,
        parameters: Vec<XamlValue>,
        properties: Vec<(&str, XamlValue)>,
    ) -> Rc<Self> {
        Rc::new(Self {
            type_,
            parameters,
            properties: properties
                .into_iter()
                .map(|(k, v)| (k.to_string(), v))
                .collect(),
        })
    }
}

impl IXamlCustomAttribute for FakeCustomAttribute {
    fn type_(&self) -> Rc<dyn IXamlType> {
        self.type_.clone()
    }
    fn parameters(&self) -> Vec<XamlValue> {
        self.parameters.clone()
    }
    fn properties(&self) -> HashMap<String, XamlValue> {
        self.properties.clone()
    }
    fn equals(&self, other: &dyn IXamlCustomAttribute) -> bool {
        std::ptr::addr_eq(self as *const Self, other.as_any() as *const dyn Any)
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}

pub struct FakeParameterInfo {
    parameter_type: Rc<dyn IXamlType>,
    custom_attributes: Vec<Rc<dyn IXamlCustomAttribute>>,
}

impl IXamlParameterInfo for FakeParameterInfo {
    fn parameter_type(&self) -> Rc<dyn IXamlType> {
        self.parameter_type.clone()
    }
    fn custom_attributes(&self) -> Vec<Rc<dyn IXamlCustomAttribute>> {
        self.custom_attributes.clone()
    }
}

fn parameter_info(
    parameters: &[Rc<dyn IXamlType>],
    attributes: &RefCell<HashMap<usize, Vec<Rc<dyn IXamlCustomAttribute>>>>,
    index: usize,
) -> XamlResult<Rc<dyn IXamlParameterInfo>> {
    let parameter_type = parameters.get(index).cloned().ok_or_else(|| {
        XamlError::internal(
            "ArgumentOutOfRangeException",
            format!("Parameter index {index} is out of range"),
        )
    })?;
    Ok(Rc::new(FakeParameterInfo {
        parameter_type,
        custom_attributes: attributes.borrow().get(&index).cloned().unwrap_or_default(),
    }))
}

fn upgrade_declaring_type(declaring_type: &Weak<FakeType>) -> Rc<dyn IXamlType> {
    match declaring_type.upgrade() {
        Some(t) => t,
        None => XamlPseudoType::unknown(),
    }
}

pub struct FakeMethod {
    name: String,
    declaring_type: Weak<FakeType>,
    return_type: Rc<dyn IXamlType>,
    parameters: Vec<Rc<dyn IXamlType>>,
    is_static: bool,
    pub is_public: Cell<bool>,
    pub is_private: Cell<bool>,
    pub is_family: Cell<bool>,
    custom_attributes: RefCell<Vec<Rc<dyn IXamlCustomAttribute>>>,
    parameter_attributes: RefCell<HashMap<usize, Vec<Rc<dyn IXamlCustomAttribute>>>>,
    /// Generic parameters of a generic method definition.
    generic_parameters: Vec<Rc<FakeType>>,
    /// Type arguments of a constructed generic method.
    generic_arguments: Vec<Rc<dyn IXamlType>>,
}

impl FakeMethod {
    fn new(
        declaring_type: &Weak<FakeType>,
        name: &str,
        return_type: Rc<dyn IXamlType>,
        parameters: Vec<Rc<dyn IXamlType>>,
        is_static: bool,
    ) -> Rc<Self> {
        Rc::new(Self {
            name: name.to_string(),
            declaring_type: declaring_type.clone(),
            return_type,
            parameters,
            is_static,
            is_public: Cell::new(true),
            is_private: Cell::new(false),
            is_family: Cell::new(false),
            custom_attributes: RefCell::new(Vec::new()),
            parameter_attributes: RefCell::new(HashMap::new()),
            generic_parameters: Vec::new(),
            generic_arguments: Vec::new(),
        })
    }

    pub fn add_attribute(&self, attribute: Rc<dyn IXamlCustomAttribute>) -> &Self {
        self.custom_attributes.borrow_mut().push(attribute);
        self
    }

    pub fn add_parameter_attribute(
        &self,
        index: usize,
        attribute: Rc<dyn IXamlCustomAttribute>,
    ) -> &Self {
        self.parameter_attributes
            .borrow_mut()
            .entry(index)
            .or_default()
            .push(attribute);
        self
    }

    /// Marks the method as private.
    pub fn make_private(&self) -> &Self {
        self.is_public.set(false);
        self.is_private.set(true);
        self
    }

    fn substituted(
        &self,
        declaring_type: &Weak<FakeType>,
        parameters: &[Rc<FakeType>],
        arguments: &[Rc<dyn IXamlType>],
    ) -> Rc<FakeMethod> {
        Rc::new(Self {
            name: self.name.clone(),
            declaring_type: declaring_type.clone(),
            return_type: substitute(&self.return_type, parameters, arguments),
            parameters: self
                .parameters
                .iter()
                .map(|p| substitute(p, parameters, arguments))
                .collect(),
            is_static: self.is_static,
            is_public: Cell::new(self.is_public.get()),
            is_private: Cell::new(self.is_private.get()),
            is_family: Cell::new(self.is_family.get()),
            custom_attributes: RefCell::new(self.custom_attributes.borrow().clone()),
            parameter_attributes: RefCell::new(self.parameter_attributes.borrow().clone()),
            generic_parameters: self.generic_parameters.clone(),
            generic_arguments: self
                .generic_arguments
                .iter()
                .map(|a| substitute(a, parameters, arguments))
                .collect(),
        })
    }
}

impl IXamlMember for FakeMethod {
    fn name(&self) -> String {
        self.name.clone()
    }
    fn declaring_type(&self) -> Rc<dyn IXamlType> {
        upgrade_declaring_type(&self.declaring_type)
    }
}

impl IXamlMethod for FakeMethod {
    fn is_public(&self) -> bool {
        self.is_public.get()
    }
    fn is_private(&self) -> bool {
        self.is_private.get()
    }
    fn is_family(&self) -> bool {
        self.is_family.get()
    }
    fn is_static(&self) -> bool {
        self.is_static
    }
    fn contains_generic_parameters(&self) -> bool {
        !self.generic_parameters.is_empty()
    }
    fn is_generic_method(&self) -> bool {
        !self.generic_parameters.is_empty() || !self.generic_arguments.is_empty()
    }
    fn is_generic_method_definition(&self) -> bool {
        !self.generic_parameters.is_empty()
    }
    fn return_type(&self) -> Rc<dyn IXamlType> {
        self.return_type.clone()
    }
    fn parameters(&self) -> Vec<Rc<dyn IXamlType>> {
        self.parameters.clone()
    }
    fn make_generic_method(
        &self,
        type_arguments: &[Rc<dyn IXamlType>],
    ) -> XamlResult<Rc<dyn IXamlMethod>> {
        if self.generic_parameters.is_empty() {
            return Err(XamlError::invalid_operation(format!(
                "{} is not a generic method definition",
                self.name
            )));
        }
        if self.generic_parameters.len() != type_arguments.len() {
            return Err(XamlError::argument(format!(
                "{} expects {} type arguments, got {}",
                self.name,
                self.generic_parameters.len(),
                type_arguments.len()
            )));
        }
        let parameters = &self.generic_parameters;
        Ok(Rc::new(Self {
            name: self.name.clone(),
            declaring_type: self.declaring_type.clone(),
            return_type: substitute(&self.return_type, parameters, type_arguments),
            parameters: self
                .parameters
                .iter()
                .map(|p| substitute(p, parameters, type_arguments))
                .collect(),
            is_static: self.is_static,
            is_public: Cell::new(self.is_public.get()),
            is_private: Cell::new(self.is_private.get()),
            is_family: Cell::new(self.is_family.get()),
            custom_attributes: RefCell::new(self.custom_attributes.borrow().clone()),
            parameter_attributes: RefCell::new(self.parameter_attributes.borrow().clone()),
            generic_parameters: Vec::new(),
            generic_arguments: type_arguments.to_vec(),
        }))
    }
    fn custom_attributes(&self) -> Vec<Rc<dyn IXamlCustomAttribute>> {
        self.custom_attributes.borrow().clone()
    }
    fn get_parameter_info(&self, index: usize) -> XamlResult<Rc<dyn IXamlParameterInfo>> {
        parameter_info(&self.parameters, &self.parameter_attributes, index)
    }
    fn generic_parameters(&self) -> Vec<Rc<dyn IXamlType>> {
        self.generic_parameters
            .iter()
            .map(|p| p.clone() as Rc<dyn IXamlType>)
            .collect()
    }
    fn generic_arguments(&self) -> Vec<Rc<dyn IXamlType>> {
        self.generic_arguments.clone()
    }
    fn equals(&self, other: &dyn IXamlMethod) -> bool {
        match other.as_any().downcast_ref::<FakeMethod>() {
            Some(other) => {
                self.name == other.name
                    && self.is_static == other.is_static
                    && self.declaring_type().equals(&*other.declaring_type())
                    && self.return_type.equals(&*other.return_type)
                    && xaml_types_sequence_equal(&self.parameters, &other.parameters)
                    && self.generic_parameters.len() == other.generic_parameters.len()
                    && xaml_types_sequence_equal(&self.generic_arguments, &other.generic_arguments)
            }
            None => false,
        }
    }
    fn get_hash_code(&self) -> u64 {
        hash_str(&self.name) ^ self.declaring_type().get_hash_code()
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}

pub struct FakeConstructor {
    declaring_type: Weak<FakeType>,
    parameters: Vec<Rc<dyn IXamlType>>,
    is_public: bool,
    is_static: bool,
    parameter_attributes: RefCell<HashMap<usize, Vec<Rc<dyn IXamlCustomAttribute>>>>,
}

impl IXamlMember for FakeConstructor {
    fn name(&self) -> String {
        if self.is_static { ".cctor" } else { ".ctor" }.to_string()
    }
    fn declaring_type(&self) -> Rc<dyn IXamlType> {
        upgrade_declaring_type(&self.declaring_type)
    }
}

impl IXamlConstructor for FakeConstructor {
    fn is_public(&self) -> bool {
        self.is_public
    }
    fn is_static(&self) -> bool {
        self.is_static
    }
    fn parameters(&self) -> Vec<Rc<dyn IXamlType>> {
        self.parameters.clone()
    }
    fn get_parameter_info(&self, index: usize) -> XamlResult<Rc<dyn IXamlParameterInfo>> {
        parameter_info(&self.parameters, &self.parameter_attributes, index)
    }
    fn equals(&self, other: &dyn IXamlConstructor) -> bool {
        match other.as_any().downcast_ref::<FakeConstructor>() {
            Some(other) => {
                self.is_static == other.is_static
                    && self.declaring_type().equals(&*other.declaring_type())
                    && xaml_types_sequence_equal(&self.parameters, &other.parameters)
            }
            None => false,
        }
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}

pub struct FakeProperty {
    name: String,
    declaring_type: Weak<FakeType>,
    property_type: Rc<dyn IXamlType>,
    getter: Option<Rc<FakeMethod>>,
    setter: Option<Rc<FakeMethod>>,
    custom_attributes: RefCell<Vec<Rc<dyn IXamlCustomAttribute>>>,
    /// The parameter types of an indexer property (empty for a plain property).
    indexer_parameters: Vec<Rc<dyn IXamlType>>,
}

impl FakeProperty {
    pub fn add_attribute(&self, attribute: Rc<dyn IXamlCustomAttribute>) -> &Self {
        self.custom_attributes.borrow_mut().push(attribute);
        self
    }

    pub fn getter_method(&self) -> Option<&Rc<FakeMethod>> {
        self.getter.as_ref()
    }

    pub fn setter_method(&self) -> Option<&Rc<FakeMethod>> {
        self.setter.as_ref()
    }
}

impl IXamlMember for FakeProperty {
    fn name(&self) -> String {
        self.name.clone()
    }
    fn declaring_type(&self) -> Rc<dyn IXamlType> {
        upgrade_declaring_type(&self.declaring_type)
    }
}

impl IXamlProperty for FakeProperty {
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
        self.custom_attributes.borrow().clone()
    }
    fn indexer_parameters(&self) -> Vec<Rc<dyn IXamlType>> {
        self.indexer_parameters.clone()
    }
    fn equals(&self, other: &dyn IXamlProperty) -> bool {
        match other.as_any().downcast_ref::<FakeProperty>() {
            Some(other) => {
                self.name == other.name && self.declaring_type().equals(&*other.declaring_type())
            }
            None => false,
        }
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}

pub struct FakeField {
    name: String,
    declaring_type: Weak<FakeType>,
    field_type: Rc<dyn IXamlType>,
    is_public: bool,
    is_static: bool,
    literal: Option<XamlValue>,
    custom_attributes: RefCell<Vec<Rc<dyn IXamlCustomAttribute>>>,
}

impl FakeField {
    pub fn add_attribute(&self, attribute: Rc<dyn IXamlCustomAttribute>) -> &Self {
        self.custom_attributes.borrow_mut().push(attribute);
        self
    }
}

impl IXamlMember for FakeField {
    fn name(&self) -> String {
        self.name.clone()
    }
    fn declaring_type(&self) -> Rc<dyn IXamlType> {
        upgrade_declaring_type(&self.declaring_type)
    }
}

impl IXamlField for FakeField {
    fn field_type(&self) -> Rc<dyn IXamlType> {
        self.field_type.clone()
    }
    fn is_public(&self) -> bool {
        self.is_public
    }
    fn is_static(&self) -> bool {
        self.is_static
    }
    fn is_literal(&self) -> bool {
        self.literal.is_some()
    }
    fn get_literal_value(&self) -> XamlResult<XamlValue> {
        self.literal.clone().ok_or_else(|| {
            XamlError::invalid_operation(format!("Field {} is not a literal", self.name))
        })
    }
    fn custom_attributes(&self) -> Vec<Rc<dyn IXamlCustomAttribute>> {
        self.custom_attributes.borrow().clone()
    }
    fn equals(&self, other: &dyn IXamlField) -> bool {
        match other.as_any().downcast_ref::<FakeField>() {
            Some(other) => {
                self.name == other.name && self.declaring_type().equals(&*other.declaring_type())
            }
            None => false,
        }
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}

pub struct FakeEvent {
    name: String,
    declaring_type: Weak<FakeType>,
    add: Option<Rc<FakeMethod>>,
}

impl IXamlMember for FakeEvent {
    fn name(&self) -> String {
        self.name.clone()
    }
    fn declaring_type(&self) -> Rc<dyn IXamlType> {
        upgrade_declaring_type(&self.declaring_type)
    }
}

impl IXamlEventInfo for FakeEvent {
    fn add(&self) -> Option<Rc<dyn IXamlMethod>> {
        self.add.clone().map(|m| m as Rc<dyn IXamlMethod>)
    }
    fn equals(&self, other: &dyn IXamlEventInfo) -> bool {
        match other.as_any().downcast_ref::<FakeEvent>() {
            Some(other) => {
                self.name == other.name && self.declaring_type().equals(&*other.declaring_type())
            }
            None => false,
        }
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}

pub struct FakeType {
    this: Weak<FakeType>,
    key: String,
    assembly: Weak<FakeAssembly>,
    namespace: String,
    name: String,
    kind: Cell<FakeTypeKind>,
    pub is_public: Cell<bool>,
    base_type: RefCell<Option<Rc<dyn IXamlType>>>,
    interfaces: RefCell<Vec<Rc<dyn IXamlType>>>,
    properties: RefCell<Vec<Rc<FakeProperty>>>,
    events: RefCell<Vec<Rc<FakeEvent>>>,
    fields: RefCell<Vec<Rc<FakeField>>>,
    methods: RefCell<Vec<Rc<FakeMethod>>>,
    constructors: RefCell<Vec<Rc<FakeConstructor>>>,
    custom_attributes: RefCell<Vec<Rc<dyn IXamlCustomAttribute>>>,
    enum_underlying_type: RefCell<Option<Rc<dyn IXamlType>>>,
    /// Generic parameters of a generic type definition.
    generic_parameters: Vec<Rc<FakeType>>,
    /// The definition and the arguments of a constructed generic type.
    generic_definition: Weak<FakeType>,
    generic_arguments: Vec<Rc<dyn IXamlType>>,
    /// Constructed types created from this definition.
    instances: RefCell<HashMap<String, Rc<FakeType>>>,
    /// The element type of an array type.
    array_element_type: RefCell<Option<Rc<dyn IXamlType>>>,
    /// Array types created from this element type, by number of dimensions.
    arrays: RefCell<HashMap<i32, Rc<FakeType>>>,
}

impl FakeType {
    #[allow(clippy::too_many_arguments)]
    fn create(
        key: String,
        assembly: Weak<FakeAssembly>,
        namespace: &str,
        name: &str,
        kind: FakeTypeKind,
        generic_parameter_names: &[&str],
        generic_definition: Weak<FakeType>,
        generic_arguments: Vec<Rc<dyn IXamlType>>,
    ) -> Rc<FakeType> {
        Rc::new_cyclic(|this| {
            let generic_parameters = generic_parameter_names
                .iter()
                .map(|parameter_name| {
                    FakeType::create(
                        format!("{key}!{parameter_name}"),
                        assembly.clone(),
                        "",
                        parameter_name,
                        FakeTypeKind::GenericParameter,
                        &[],
                        Weak::new(),
                        Vec::new(),
                    )
                })
                .collect();
            FakeType {
                this: this.clone(),
                key,
                assembly,
                namespace: namespace.to_string(),
                name: name.to_string(),
                kind: Cell::new(kind),
                is_public: Cell::new(true),
                base_type: RefCell::new(None),
                interfaces: RefCell::new(Vec::new()),
                properties: RefCell::new(Vec::new()),
                events: RefCell::new(Vec::new()),
                fields: RefCell::new(Vec::new()),
                methods: RefCell::new(Vec::new()),
                constructors: RefCell::new(Vec::new()),
                custom_attributes: RefCell::new(Vec::new()),
                enum_underlying_type: RefCell::new(None),
                generic_parameters,
                generic_definition,
                generic_arguments,
                instances: RefCell::new(HashMap::new()),
                array_element_type: RefCell::new(None),
                arrays: RefCell::new(HashMap::new()),
            }
        })
    }

    fn definition(&self) -> Option<Rc<FakeType>> {
        self.generic_definition.upgrade()
    }

    fn instantiate(&self, arguments: &[Rc<dyn IXamlType>]) -> Rc<dyn IXamlType> {
        let argument_keys: Vec<String> = arguments.iter().map(type_key).collect();
        let key = format!("{}<{}>", self.key, argument_keys.join(","));
        if let Some(existing) = self.instances.borrow().get(&key) {
            return existing.clone();
        }
        let instance = FakeType::create(
            key.clone(),
            self.assembly.clone(),
            &self.namespace,
            &self.name,
            self.kind.get(),
            &[],
            self.this.clone(),
            arguments.to_vec(),
        );
        self.instances.borrow_mut().insert(key, instance.clone());
        instance
    }

    /// This type as a type system handle.
    pub fn as_type(&self) -> Rc<dyn IXamlType> {
        match self.this.upgrade() {
            Some(t) => t,
            None => XamlPseudoType::unknown(),
        }
    }

    /// The generic parameter with the given index (for generic type definitions).
    pub fn generic_parameter(&self, index: usize) -> Rc<dyn IXamlType> {
        self.generic_parameters[index].clone()
    }

    pub fn set_base_type(&self, base_type: Rc<dyn IXamlType>) -> &Self {
        *self.base_type.borrow_mut() = Some(base_type);
        self
    }

    pub fn add_interface(&self, interface: Rc<dyn IXamlType>) -> &Self {
        self.interfaces.borrow_mut().push(interface);
        self
    }

    pub fn add_attribute(&self, attribute: Rc<dyn IXamlCustomAttribute>) -> &Self {
        self.custom_attributes.borrow_mut().push(attribute);
        self
    }

    /// Adds a public constructor.
    pub fn add_constructor(&self, parameters: Vec<Rc<dyn IXamlType>>) -> Rc<FakeConstructor> {
        let ctor = Rc::new(FakeConstructor {
            declaring_type: self.this.clone(),
            parameters,
            is_public: true,
            is_static: false,
            parameter_attributes: RefCell::new(HashMap::new()),
        });
        self.constructors.borrow_mut().push(ctor.clone());
        ctor
    }

    /// Adds a public method.
    pub fn add_method(
        &self,
        name: &str,
        return_type: Rc<dyn IXamlType>,
        parameters: Vec<Rc<dyn IXamlType>>,
        is_static: bool,
    ) -> Rc<FakeMethod> {
        let method = FakeMethod::new(&self.this, name, return_type, parameters, is_static);
        self.methods.borrow_mut().push(method.clone());
        method
    }

    /// Adds a public generic method definition. `signature` receives the method's generic
    /// parameters (one per name) and returns the return type and the parameter types, which may
    /// use them: `t.add_generic_method("Get", &["T"], false, |g| (g[0].clone(), vec![]))`.
    pub fn add_generic_method(
        &self,
        name: &str,
        generic_parameter_names: &[&str],
        is_static: bool,
        signature: impl FnOnce(&[Rc<dyn IXamlType>]) -> (Rc<dyn IXamlType>, Vec<Rc<dyn IXamlType>>),
    ) -> Rc<FakeMethod> {
        let generic_parameters: Vec<Rc<FakeType>> = generic_parameter_names
            .iter()
            .map(|parameter_name| {
                FakeType::create(
                    format!("{}::{name}!!{parameter_name}", self.key),
                    self.assembly.clone(),
                    "",
                    parameter_name,
                    FakeTypeKind::GenericParameter,
                    &[],
                    Weak::new(),
                    Vec::new(),
                )
            })
            .collect();
        let handles: Vec<Rc<dyn IXamlType>> = generic_parameters
            .iter()
            .map(|p| p.clone() as Rc<dyn IXamlType>)
            .collect();
        let (return_type, parameters) = signature(&handles);
        let method = Rc::new(FakeMethod {
            name: name.to_string(),
            declaring_type: self.this.clone(),
            return_type,
            parameters,
            is_static,
            is_public: Cell::new(true),
            is_private: Cell::new(false),
            is_family: Cell::new(false),
            custom_attributes: RefCell::new(Vec::new()),
            parameter_attributes: RefCell::new(HashMap::new()),
            generic_parameters,
            generic_arguments: Vec::new(),
        });
        self.methods.borrow_mut().push(method.clone());
        method
    }

    /// Adds a public instance property with a getter and a setter.
    pub fn add_property(&self, name: &str, property_type: Rc<dyn IXamlType>) -> Rc<FakeProperty> {
        self.add_property_with(name, property_type, true, true, false)
    }

    /// Adds a public property; the accessor methods (`get_X`/`set_X`) are added to the type's methods.
    pub fn add_property_with(
        &self,
        name: &str,
        property_type: Rc<dyn IXamlType>,
        has_getter: bool,
        has_setter: bool,
        is_static: bool,
    ) -> Rc<FakeProperty> {
        let void = self.void_type();
        let getter = has_getter.then(|| {
            self.add_method(
                &format!("get_{name}"),
                property_type.clone(),
                Vec::new(),
                is_static,
            )
        });
        let setter = has_setter.then(|| {
            self.add_method(
                &format!("set_{name}"),
                void,
                vec![property_type.clone()],
                is_static,
            )
        });
        let property = Rc::new(FakeProperty {
            name: name.to_string(),
            declaring_type: self.this.clone(),
            property_type,
            getter,
            setter,
            custom_attributes: RefCell::new(Vec::new()),
            indexer_parameters: Vec::new(),
        });
        self.properties.borrow_mut().push(property.clone());
        property
    }

    /// Adds a public instance indexer property (`this[...]`): the accessor methods are
    /// `get_<name>(parameters...)` and, with `has_setter`, `set_<name>(parameters..., value)`.
    /// Mark the type with `DefaultMemberAttribute(name)` to make it the type's indexer.
    pub fn add_indexer(
        &self,
        name: &str,
        property_type: Rc<dyn IXamlType>,
        indexer_parameters: Vec<Rc<dyn IXamlType>>,
        has_setter: bool,
    ) -> Rc<FakeProperty> {
        let void = self.void_type();
        let getter = self.add_method(
            &format!("get_{name}"),
            property_type.clone(),
            indexer_parameters.clone(),
            false,
        );
        let setter = has_setter.then(|| {
            let mut parameters = indexer_parameters.clone();
            parameters.push(property_type.clone());
            self.add_method(&format!("set_{name}"), void, parameters, false)
        });
        let property = Rc::new(FakeProperty {
            name: name.to_string(),
            declaring_type: self.this.clone(),
            property_type,
            getter: Some(getter),
            setter,
            custom_attributes: RefCell::new(Vec::new()),
            indexer_parameters,
        });
        self.properties.borrow_mut().push(property.clone());
        property
    }

    /// Adds a public field; pass a literal value to make it a constant.
    pub fn add_field(
        &self,
        name: &str,
        field_type: Rc<dyn IXamlType>,
        is_static: bool,
        literal: Option<XamlValue>,
    ) -> Rc<FakeField> {
        let field = Rc::new(FakeField {
            name: name.to_string(),
            declaring_type: self.this.clone(),
            field_type,
            is_public: true,
            is_static,
            literal,
            custom_attributes: RefCell::new(Vec::new()),
        });
        self.fields.borrow_mut().push(field.clone());
        field
    }

    /// Adds a public instance event with an `add_X(handler)` method.
    pub fn add_event(&self, name: &str, handler_type: Rc<dyn IXamlType>) -> Rc<FakeEvent> {
        let void = self.void_type();
        let add = self.add_method(&format!("add_{name}"), void, vec![handler_type], false);
        let event = Rc::new(FakeEvent {
            name: name.to_string(),
            declaring_type: self.this.clone(),
            add: Some(add),
        });
        self.events.borrow_mut().push(event.clone());
        event
    }

    fn void_type(&self) -> Rc<dyn IXamlType> {
        self.assembly
            .upgrade()
            .and_then(|a| a.type_system.upgrade())
            .and_then(|ts| ts.find_type("System.Void"))
            .unwrap_or_else(XamlPseudoType::unknown)
    }

    fn declared_interfaces(&self) -> Vec<Rc<dyn IXamlType>> {
        match self.definition() {
            Some(definition) => definition
                .interfaces
                .borrow()
                .iter()
                .map(|i| substitute(i, &definition.generic_parameters, &self.generic_arguments))
                .collect(),
            None => self.interfaces.borrow().clone(),
        }
    }
}

impl IXamlType for FakeType {
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
        let mut rv = if self.namespace.is_empty() {
            self.name.clone()
        } else {
            format!("{}.{}", self.namespace, self.name)
        };
        if !self.generic_arguments.is_empty() {
            let arguments: Vec<String> = self
                .generic_arguments
                .iter()
                .map(|a| a.full_name())
                .collect();
            rv = format!("{rv}[{}]", arguments.join(","));
        }
        rv
    }
    fn is_public(&self) -> bool {
        self.is_public.get()
    }
    fn is_nested_private(&self) -> bool {
        false
    }
    fn assembly(&self) -> Option<Rc<dyn IXamlAssembly>> {
        self.assembly.upgrade().map(|a| a as Rc<dyn IXamlAssembly>)
    }
    fn properties(&self) -> Vec<Rc<dyn IXamlProperty>> {
        match self.definition() {
            Some(definition) => definition
                .properties
                .borrow()
                .iter()
                .map(|p| {
                    let parameters = &definition.generic_parameters;
                    let arguments = &self.generic_arguments;
                    Rc::new(FakeProperty {
                        name: p.name.clone(),
                        declaring_type: self.this.clone(),
                        property_type: substitute(&p.property_type, parameters, arguments),
                        getter: p
                            .getter
                            .as_ref()
                            .map(|m| m.substituted(&self.this, parameters, arguments)),
                        setter: p
                            .setter
                            .as_ref()
                            .map(|m| m.substituted(&self.this, parameters, arguments)),
                        custom_attributes: RefCell::new(p.custom_attributes.borrow().clone()),
                        indexer_parameters: p
                            .indexer_parameters
                            .iter()
                            .map(|t| substitute(t, parameters, arguments))
                            .collect(),
                    }) as Rc<dyn IXamlProperty>
                })
                .collect(),
            None => self
                .properties
                .borrow()
                .iter()
                .map(|p| p.clone() as Rc<dyn IXamlProperty>)
                .collect(),
        }
    }
    fn events(&self) -> Vec<Rc<dyn IXamlEventInfo>> {
        match self.definition() {
            Some(definition) => definition
                .events
                .borrow()
                .iter()
                .map(|e| {
                    Rc::new(FakeEvent {
                        name: e.name.clone(),
                        declaring_type: self.this.clone(),
                        add: e.add.as_ref().map(|m| {
                            m.substituted(
                                &self.this,
                                &definition.generic_parameters,
                                &self.generic_arguments,
                            )
                        }),
                    }) as Rc<dyn IXamlEventInfo>
                })
                .collect(),
            None => self
                .events
                .borrow()
                .iter()
                .map(|e| e.clone() as Rc<dyn IXamlEventInfo>)
                .collect(),
        }
    }
    fn fields(&self) -> Vec<Rc<dyn IXamlField>> {
        match self.definition() {
            Some(definition) => definition
                .fields
                .borrow()
                .iter()
                .map(|f| {
                    Rc::new(FakeField {
                        name: f.name.clone(),
                        declaring_type: self.this.clone(),
                        field_type: substitute(
                            &f.field_type,
                            &definition.generic_parameters,
                            &self.generic_arguments,
                        ),
                        is_public: f.is_public,
                        is_static: f.is_static,
                        literal: f.literal.clone(),
                        custom_attributes: RefCell::new(f.custom_attributes.borrow().clone()),
                    }) as Rc<dyn IXamlField>
                })
                .collect(),
            None => self
                .fields
                .borrow()
                .iter()
                .map(|f| f.clone() as Rc<dyn IXamlField>)
                .collect(),
        }
    }
    fn methods(&self) -> Vec<Rc<dyn IXamlMethod>> {
        match self.definition() {
            Some(definition) => definition
                .methods
                .borrow()
                .iter()
                .map(|m| {
                    m.substituted(
                        &self.this,
                        &definition.generic_parameters,
                        &self.generic_arguments,
                    ) as Rc<dyn IXamlMethod>
                })
                .collect(),
            None => self
                .methods
                .borrow()
                .iter()
                .map(|m| m.clone() as Rc<dyn IXamlMethod>)
                .collect(),
        }
    }
    fn constructors(&self) -> Vec<Rc<dyn IXamlConstructor>> {
        match self.definition() {
            Some(definition) => definition
                .constructors
                .borrow()
                .iter()
                .map(|c| {
                    Rc::new(FakeConstructor {
                        declaring_type: self.this.clone(),
                        parameters: c
                            .parameters
                            .iter()
                            .map(|p| {
                                substitute(
                                    p,
                                    &definition.generic_parameters,
                                    &self.generic_arguments,
                                )
                            })
                            .collect(),
                        is_public: c.is_public,
                        is_static: c.is_static,
                        parameter_attributes: RefCell::new(c.parameter_attributes.borrow().clone()),
                    }) as Rc<dyn IXamlConstructor>
                })
                .collect(),
            None => self
                .constructors
                .borrow()
                .iter()
                .map(|c| c.clone() as Rc<dyn IXamlConstructor>)
                .collect(),
        }
    }
    fn custom_attributes(&self) -> Vec<Rc<dyn IXamlCustomAttribute>> {
        match self.definition() {
            Some(definition) => definition.custom_attributes.borrow().clone(),
            None => self.custom_attributes.borrow().clone(),
        }
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

        self.is_interface()
            && type_
                .get_all_interfaces()
                .iter()
                .any(|i| self.is_assignable_from(&**i))
    }
    fn make_generic_type(
        &self,
        type_arguments: &[Rc<dyn IXamlType>],
    ) -> XamlResult<Rc<dyn IXamlType>> {
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
        Ok(self.instantiate(type_arguments))
    }
    fn generic_type_definition(&self) -> Option<Rc<dyn IXamlType>> {
        self.definition().map(|d| d as Rc<dyn IXamlType>)
    }
    fn is_array(&self) -> bool {
        self.array_element_type.borrow().is_some()
    }
    fn array_element_type(&self) -> Option<Rc<dyn IXamlType>> {
        self.array_element_type.borrow().clone()
    }
    fn make_array_type(&self, dimensions: i32) -> XamlResult<Rc<dyn IXamlType>> {
        if dimensions < 1 {
            return Err(XamlError::argument(
                "An array type needs at least one dimension",
            ));
        }
        if let Some(existing) = self.arrays.borrow().get(&dimensions) {
            return Ok(existing.clone());
        }
        let suffix = format!("[{}]", ",".repeat((dimensions - 1) as usize));
        let array = FakeType::create(
            format!("{}{suffix}", self.key),
            self.assembly.clone(),
            &self.namespace,
            &format!("{}{suffix}", self.name),
            FakeTypeKind::Class,
            &[],
            Weak::new(),
            Vec::new(),
        );
        let element = self.as_type();
        *array.array_element_type.borrow_mut() = Some(element.clone());
        let core_type = |name: &str| {
            self.assembly
                .upgrade()
                .and_then(|a| a.type_system.upgrade())
                .and_then(|ts| ts.find_type(name))
        };
        if let Some(array_base) = core_type("System.Array") {
            array.set_base_type(array_base);
        }
        if dimensions == 1 {
            // A vector implements the generic list interfaces of its element type.
            for name in [
                "System.Collections.Generic.IList`1",
                "System.Collections.Generic.IReadOnlyList`1",
            ] {
                if let Some(interface) = core_type(name) {
                    array.add_interface(interface.make_generic_type(&[element.clone()])?);
                }
            }
        }
        if let Some(list) = core_type("System.Collections.IList") {
            array.add_interface(list);
        }
        self.arrays.borrow_mut().insert(dimensions, array.clone());
        Ok(array)
    }
    fn base_type(&self) -> Option<Rc<dyn IXamlType>> {
        match self.definition() {
            Some(definition) => definition
                .base_type
                .borrow()
                .as_ref()
                .map(|b| substitute(b, &definition.generic_parameters, &self.generic_arguments)),
            None => self.base_type.borrow().clone(),
        }
    }
    fn declaring_type(&self) -> Option<Rc<dyn IXamlType>> {
        None
    }
    fn is_value_type(&self) -> bool {
        matches!(self.kind.get(), FakeTypeKind::Struct | FakeTypeKind::Enum)
    }
    fn is_enum(&self) -> bool {
        self.kind.get() == FakeTypeKind::Enum
    }
    fn interfaces(&self) -> Vec<Rc<dyn IXamlType>> {
        // Declared interfaces followed by the interfaces they inherit.
        let mut rv: Vec<Rc<dyn IXamlType>> = Vec::new();
        for declared in self.declared_interfaces() {
            let inherited = declared.interfaces();
            for candidate in std::iter::once(declared).chain(inherited) {
                if !rv.iter().any(|existing| existing.equals(&*candidate)) {
                    rv.push(candidate);
                }
            }
        }
        rv
    }
    fn is_interface(&self) -> bool {
        self.kind.get() == FakeTypeKind::Interface
    }
    fn get_enum_underlying_type(&self) -> XamlResult<Rc<dyn IXamlType>> {
        self.enum_underlying_type.borrow().clone().ok_or_else(|| {
            XamlError::invalid_operation(format!("{} is not an enum", self.full_name()))
        })
    }
    fn generic_parameters(&self) -> Vec<Rc<dyn IXamlType>> {
        self.generic_parameters
            .iter()
            .map(|p| p.clone() as Rc<dyn IXamlType>)
            .collect()
    }
    fn is_function_pointer(&self) -> bool {
        false
    }
    fn equals(&self, other: &dyn IXamlType) -> bool {
        match other.as_any().downcast_ref::<FakeType>() {
            Some(other) => self.key == other.key,
            None => false,
        }
    }
    fn get_hash_code(&self) -> u64 {
        hash_str(&self.key)
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}

pub struct FakeAssembly {
    this: Weak<FakeAssembly>,
    type_system: Weak<FakeTypeSystem>,
    name: String,
    types: RefCell<Vec<Rc<FakeType>>>,
    custom_attributes: RefCell<Vec<Rc<dyn IXamlCustomAttribute>>>,
}

impl FakeAssembly {
    fn define(
        &self,
        namespace: &str,
        name: &str,
        kind: FakeTypeKind,
        generic_parameters: &[&str],
    ) -> Rc<FakeType> {
        let full_name = if namespace.is_empty() {
            name.to_string()
        } else {
            format!("{namespace}.{name}")
        };
        let type_ = FakeType::create(
            format!("{}|{}", self.name, full_name),
            self.this.clone(),
            namespace,
            name,
            kind,
            generic_parameters,
            Weak::new(),
            Vec::new(),
        );
        self.types.borrow_mut().push(type_.clone());
        type_
    }

    fn core_type(&self, name: &str) -> Option<Rc<dyn IXamlType>> {
        self.type_system.upgrade().and_then(|ts| ts.find_type(name))
    }

    /// Defines a public class deriving from `System.Object`. No constructor is added.
    pub fn define_class(&self, namespace: &str, name: &str) -> Rc<FakeType> {
        self.define_generic_class(namespace, name, &[])
    }

    /// Defines a generic class definition; `name` must carry the arity suffix (e.g. ``List`1``).
    pub fn define_generic_class(
        &self,
        namespace: &str,
        name: &str,
        generic_parameters: &[&str],
    ) -> Rc<FakeType> {
        let type_ = self.define(namespace, name, FakeTypeKind::Class, generic_parameters);
        if let Some(object) = self.core_type("System.Object") {
            if !object.equals(&*type_) {
                type_.set_base_type(object);
            }
        }
        type_
    }

    pub fn define_interface(&self, namespace: &str, name: &str) -> Rc<FakeType> {
        self.define_generic_interface(namespace, name, &[])
    }

    pub fn define_generic_interface(
        &self,
        namespace: &str,
        name: &str,
        generic_parameters: &[&str],
    ) -> Rc<FakeType> {
        self.define(namespace, name, FakeTypeKind::Interface, generic_parameters)
    }

    /// Defines a value type deriving from `System.ValueType`.
    pub fn define_struct(&self, namespace: &str, name: &str) -> Rc<FakeType> {
        self.define_generic_struct(namespace, name, &[])
    }

    pub fn define_generic_struct(
        &self,
        namespace: &str,
        name: &str,
        generic_parameters: &[&str],
    ) -> Rc<FakeType> {
        let type_ = self.define(namespace, name, FakeTypeKind::Struct, generic_parameters);
        if let Some(value_type) = self.core_type("System.ValueType") {
            type_.set_base_type(value_type);
        }
        type_
    }

    /// Defines an `Int32` based enum with the given members.
    pub fn define_enum(
        &self,
        namespace: &str,
        name: &str,
        members: &[(&str, i32)],
    ) -> Rc<FakeType> {
        let type_ = self.define(namespace, name, FakeTypeKind::Enum, &[]);
        if let Some(enum_type) = self.core_type("System.Enum") {
            type_.set_base_type(enum_type);
        }
        *type_.enum_underlying_type.borrow_mut() = self.core_type("System.Int32");
        for (member, value) in members {
            type_.add_field(member, type_.clone(), true, Some(XamlValue::Int32(*value)));
        }
        type_
    }

    pub fn add_attribute(&self, attribute: Rc<dyn IXamlCustomAttribute>) -> &Self {
        self.custom_attributes.borrow_mut().push(attribute);
        self
    }
}

impl IXamlAssembly for FakeAssembly {
    fn name(&self) -> String {
        self.name.clone()
    }
    fn custom_attributes(&self) -> Vec<Rc<dyn IXamlCustomAttribute>> {
        self.custom_attributes.borrow().clone()
    }
    fn find_type(&self, full_name: &str) -> Option<Rc<dyn IXamlType>> {
        self.types
            .borrow()
            .iter()
            .find(|t| t.full_name() == full_name)
            .map(|t| t.clone() as Rc<dyn IXamlType>)
    }
    fn equals(&self, other: &dyn IXamlAssembly) -> bool {
        match other.as_any().downcast_ref::<FakeAssembly>() {
            Some(other) => self.name == other.name,
            None => false,
        }
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}

pub struct FakeTypeSystem {
    this: Weak<FakeTypeSystem>,
    assemblies: RefCell<Vec<Rc<FakeAssembly>>>,
    well_known_types: OnceCell<Rc<XamlTypeWellKnownTypes>>,
}

impl FakeTypeSystem {
    /// Creates a type system containing a core assembly (`System.Runtime`) with the well known
    /// types the compiler requires.
    pub fn new() -> Rc<Self> {
        let ts = Rc::new_cyclic(|this| FakeTypeSystem {
            this: this.clone(),
            assemblies: RefCell::new(Vec::new()),
            well_known_types: OnceCell::new(),
        });
        ts.define_core_assembly();
        ts
    }

    pub fn define_assembly(&self, name: &str) -> Rc<FakeAssembly> {
        let assembly = Rc::new_cyclic(|this| FakeAssembly {
            this: this.clone(),
            type_system: self.this.clone(),
            name: name.to_string(),
            types: RefCell::new(Vec::new()),
            custom_attributes: RefCell::new(Vec::new()),
        });
        self.assemblies.borrow_mut().push(assembly.clone());
        assembly
    }

    /// The core assembly.
    pub fn core(&self) -> Rc<FakeAssembly> {
        self.assemblies.borrow()[0].clone()
    }

    /// Looks a type up by its full name.
    ///
    /// # Panics
    /// Panics when the type is not defined; this is a test helper.
    pub fn get(&self, name: &str) -> Rc<dyn IXamlType> {
        match self.find_type(name) {
            Some(t) => t,
            None => panic!("Type {name} is not defined in the fake type system"),
        }
    }

    /// Looks the declaration of a (non-constructed) type up by its full name, to add members
    /// or attributes to it.
    ///
    /// # Panics
    /// Panics when the type is not defined; this is a test helper.
    pub fn get_fake_type(&self, name: &str) -> Rc<FakeType> {
        let found = self
            .assemblies
            .borrow()
            .iter()
            .find_map(|a| a.types.borrow().iter().find(|t| t.full_name() == name).cloned());
        match found {
            Some(t) => t,
            None => panic!("Type {name} is not defined in the fake type system"),
        }
    }

    /// This type system as an `IXamlTypeSystem` handle.
    pub fn as_type_system(&self) -> Rc<dyn IXamlTypeSystem> {
        match self.this.upgrade() {
            Some(ts) => ts,
            None => panic!("The fake type system has been dropped"),
        }
    }

    fn define_core_assembly(&self) {
        let core = self.define_assembly("System.Runtime");
        let object = core.define_class("System", "Object");
        object.add_constructor(vec![]);
        let object: Rc<dyn IXamlType> = object;
        let value_type = core.define_class("System", "ValueType");
        let void: Rc<dyn IXamlType> = core.define_struct("System", "Void");
        core.define_class("System", "Enum")
            .set_base_type(value_type.clone());
        core.define_class("System", "Attribute");

        for name in [
            "Boolean", "Byte", "SByte", "Char", "Int16", "UInt16", "Int32", "UInt32", "Int64",
            "UInt64", "Single", "Double", "IntPtr",
        ] {
            core.define_struct("System", name);
        }
        let int32 = self.get("System.Int32");
        let int_ptr = self.get("System.IntPtr");

        let string: Rc<dyn IXamlType> = core.define_class("System", "String");
        let type_: Rc<dyn IXamlType> = core.define_class("System", "Type");
        core.define_class("System", "Array");
        core.define_class("System", "Uri")
            .add_constructor(vec![string.clone()]);
        let delegate: Rc<dyn IXamlType> = core.define_class("System", "Delegate");
        let multicast_delegate = core.define_class("System", "MulticastDelegate");
        multicast_delegate.set_base_type(delegate);
        let multicast_delegate: Rc<dyn IXamlType> = multicast_delegate;

        let exception: Rc<dyn IXamlType> = core.define_class("System", "Exception");
        for name in [
            "InvalidCastException",
            "NotSupportedException",
            "NullReferenceException",
        ] {
            let e = core.define_class("System", name);
            e.set_base_type(exception.clone());
            e.add_constructor(vec![]);
        }

        let attribute = self.get("System.Attribute");
        for (namespace, name) in [
            ("System", "ObsoleteAttribute"),
            ("System", "FlagsAttribute"),
            ("System", "AttributeUsageAttribute"),
            ("System.Diagnostics.CodeAnalysis", "ExperimentalAttribute"),
            ("System.ComponentModel", "TypeConverterAttribute"),
        ] {
            core.define_class(namespace, name)
                .set_base_type(attribute.clone());
        }

        core.define_interface("System", "IDisposable").add_method(
            "Dispose",
            void.clone(),
            vec![],
            false,
        );
        let format_provider: Rc<dyn IXamlType> = core.define_interface("System", "IFormatProvider");
        let culture_info = core.define_class("System.Globalization", "CultureInfo");
        culture_info.add_interface(format_provider);
        culture_info.add_property_with("InvariantCulture", culture_info.clone(), true, false, true);
        let culture_info: Rc<dyn IXamlType> = culture_info;
        core.define_class("System.Reflection", "MethodInfo");

        let service_provider = core.define_interface("System", "IServiceProvider");
        service_provider.add_method("GetService", object.clone(), vec![type_.clone()], false);
        let service_provider: Rc<dyn IXamlType> = service_provider;
        let type_descriptor_context: Rc<dyn IXamlType> = {
            let t = core.define_interface("System.ComponentModel", "ITypeDescriptorContext");
            t.add_interface(service_provider);
            t
        };
        let support_initialize =
            core.define_interface("System.ComponentModel", "ISupportInitialize");
        support_initialize.add_method("BeginInit", void.clone(), vec![], false);
        support_initialize.add_method("EndInit", void.clone(), vec![], false);
        let type_converter = core.define_class("System.ComponentModel", "TypeConverter");
        type_converter.add_constructor(vec![]);
        type_converter.add_method(
            "ConvertFrom",
            object.clone(),
            vec![type_descriptor_context, culture_info, object.clone()],
            false,
        );

        // Collections
        let enumerator = core.define_interface("System.Collections", "IEnumerator");
        let enumerable = core.define_interface("System.Collections", "IEnumerable");
        enumerable.add_method("GetEnumerator", enumerator.clone(), vec![], false);
        let enumerable: Rc<dyn IXamlType> = enumerable;
        let list = core.define_interface("System.Collections", "IList");
        list.add_interface(enumerable.clone());
        list.add_method("Add", int32, vec![object.clone()], false);
        let list: Rc<dyn IXamlType> = list;

        core.define_generic_interface("System.Collections.Generic", "IEnumerator`1", &["T"])
            .add_interface(enumerator);
        let enumerable_of_t =
            core.define_generic_interface("System.Collections.Generic", "IEnumerable`1", &["T"]);
        enumerable_of_t.add_interface(enumerable);
        let collection_of_t =
            core.define_generic_interface("System.Collections.Generic", "ICollection`1", &["T"]);
        collection_of_t
            .add_interface(enumerable_of_t.instantiate(&[collection_of_t.generic_parameter(0)]));
        collection_of_t.add_method(
            "Add",
            void.clone(),
            vec![collection_of_t.generic_parameter(0)],
            false,
        );
        let list_of_t =
            core.define_generic_interface("System.Collections.Generic", "IList`1", &["T"]);
        list_of_t.add_interface(collection_of_t.instantiate(&[list_of_t.generic_parameter(0)]));
        let read_only_list_of_t =
            core.define_generic_interface("System.Collections.Generic", "IReadOnlyList`1", &["T"]);
        read_only_list_of_t.add_interface(
            enumerable_of_t.instantiate(&[read_only_list_of_t.generic_parameter(0)]),
        );

        let list_class = core.define_generic_class("System.Collections.Generic", "List`1", &["T"]);
        list_class.add_constructor(vec![]);
        list_class.add_interface(list_of_t.instantiate(&[list_class.generic_parameter(0)]));
        list_class
            .add_interface(read_only_list_of_t.instantiate(&[list_class.generic_parameter(0)]));
        list_class.add_interface(list);
        list_class.add_method(
            "Add",
            void.clone(),
            vec![list_class.generic_parameter(0)],
            false,
        );

        let dictionary = core.define_generic_class(
            "System.Collections.Generic",
            "Dictionary`2",
            &["TKey", "TValue"],
        );
        dictionary.add_constructor(vec![]);
        dictionary.add_method(
            "Add",
            void.clone(),
            vec![
                dictionary.generic_parameter(0),
                dictionary.generic_parameter(1),
            ],
            false,
        );

        core.define_generic_struct("System", "Nullable`1", &["T"]);

        // Delegates
        let define_delegate = |name: &str, parameter_names: &[&str], has_result: bool| {
            let delegate = core.define_generic_class("System", name, parameter_names);
            delegate.set_base_type(multicast_delegate.clone());
            delegate.add_constructor(vec![object.clone(), int_ptr.clone()]);
            let parameter_count = parameter_names.len();
            let (arguments, result) = if has_result {
                let arguments = (0..parameter_count - 1)
                    .map(|i| delegate.generic_parameter(i))
                    .collect();
                (arguments, delegate.generic_parameter(parameter_count - 1))
            } else {
                (
                    (0..parameter_count)
                        .map(|i| delegate.generic_parameter(i))
                        .collect(),
                    void.clone(),
                )
            };
            delegate.add_method("Invoke", result, arguments, false);
        };
        define_delegate("Action", &[], false);
        for count in 1..=16usize {
            let names: Vec<String> = (1..=count).map(|i| format!("T{i}")).collect();
            let names: Vec<&str> = names.iter().map(String::as_str).collect();
            define_delegate(&format!("Action`{count}"), &names, false);
        }
        for count in 1..=17usize {
            let mut names: Vec<String> = (1..count).map(|i| format!("T{i}")).collect();
            names.push("TResult".to_string());
            let names: Vec<&str> = names.iter().map(String::as_str).collect();
            define_delegate(&format!("Func`{count}"), &names, true);
        }
    }
}

impl IXamlTypeSystem for FakeTypeSystem {
    fn assemblies(&self) -> Vec<Rc<dyn IXamlAssembly>> {
        self.assemblies
            .borrow()
            .iter()
            .map(|a| a.clone() as Rc<dyn IXamlAssembly>)
            .collect()
    }
    fn well_known_types(&self) -> Rc<XamlTypeWellKnownTypes> {
        self.well_known_types
            .get_or_init(|| match XamlTypeWellKnownTypes::new(self) {
                Ok(types) => Rc::new(types),
                Err(e) => panic!("The fake core assembly is incomplete: {e}"),
            })
            .clone()
    }
    fn find_assembly(&self, substring: &str) -> Option<Rc<dyn IXamlAssembly>> {
        self.assemblies
            .borrow()
            .iter()
            .find(|a| a.name.contains(substring))
            .map(|a| a.clone() as Rc<dyn IXamlAssembly>)
    }
    fn find_type(&self, name: &str) -> Option<Rc<dyn IXamlType>> {
        self.assemblies
            .borrow()
            .iter()
            .find_map(|a| a.find_type(name))
    }
    fn find_type_in_assembly(&self, name: &str, assembly: &str) -> Option<Rc<dyn IXamlType>> {
        self.assemblies
            .borrow()
            .iter()
            .filter(|a| a.name == assembly)
            .find_map(|a| a.find_type(name))
    }
}
