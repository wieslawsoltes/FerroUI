//! Port of `TypeSystem/TypeSystem.cs`.
//!
//! The C# interfaces become traits used through `Rc<dyn ...>` handles. Equality keeps the
//! upstream `IEquatable<T>` shape: every abstraction exposes `equals`, and the implementation
//! decides what identity means.

use std::any::Any;
use std::collections::HashMap;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::rc::Rc;
use std::sync::atomic::{AtomicU64, Ordering};

use crate::exceptions::{XamlError, XamlResult};

use super::XamlTypeWellKnownTypes;

/// Stand-in for a boxed `object` as it appears in custom attribute arguments,
/// literal field values and compile-time constants.
#[derive(Clone)]
pub enum XamlValue {
    Null,
    Boolean(bool),
    Char(char),
    SByte(i8),
    Byte(u8),
    Int16(i16),
    UInt16(u16),
    Int32(i32),
    UInt32(u32),
    Int64(i64),
    UInt64(u64),
    Single(f32),
    Double(f64),
    String(String),
    Type(Rc<dyn IXamlType>),
    Array(Vec<XamlValue>),
}

impl XamlValue {
    pub fn is_null(&self) -> bool {
        matches!(self, XamlValue::Null)
    }

    /// `value is string s`.
    pub fn as_str(&self) -> Option<&str> {
        match self {
            XamlValue::String(s) => Some(s),
            _ => None,
        }
    }

    /// `value as bool?`.
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            XamlValue::Boolean(b) => Some(*b),
            _ => None,
        }
    }

    /// `value as IXamlType`.
    pub fn as_type(&self) -> Option<&Rc<dyn IXamlType>> {
        match self {
            XamlValue::Type(t) => Some(t),
            _ => None,
        }
    }

    /// `GetType().IsPrimitive`.
    pub fn is_primitive(&self) -> bool {
        !matches!(
            self,
            XamlValue::Null | XamlValue::String(_) | XamlValue::Type(_) | XamlValue::Array(_)
        )
    }

    /// .NET type name of the boxed value.
    pub fn type_name(&self) -> &'static str {
        match self {
            XamlValue::Null => "null",
            XamlValue::Boolean(_) => "System.Boolean",
            XamlValue::Char(_) => "System.Char",
            XamlValue::SByte(_) => "System.SByte",
            XamlValue::Byte(_) => "System.Byte",
            XamlValue::Int16(_) => "System.Int16",
            XamlValue::UInt16(_) => "System.UInt16",
            XamlValue::Int32(_) => "System.Int32",
            XamlValue::UInt32(_) => "System.UInt32",
            XamlValue::Int64(_) => "System.Int64",
            XamlValue::UInt64(_) => "System.UInt64",
            XamlValue::Single(_) => "System.Single",
            XamlValue::Double(_) => "System.Double",
            XamlValue::String(_) => "System.String",
            XamlValue::Type(_) => "XamlX.TypeSystem.IXamlType",
            XamlValue::Array(_) => "System.Object[]",
        }
    }
}

impl PartialEq for XamlValue {
    fn eq(&self, other: &Self) -> bool {
        use XamlValue::*;
        match (self, other) {
            (Null, Null) => true,
            (Boolean(a), Boolean(b)) => a == b,
            (Char(a), Char(b)) => a == b,
            (SByte(a), SByte(b)) => a == b,
            (Byte(a), Byte(b)) => a == b,
            (Int16(a), Int16(b)) => a == b,
            (UInt16(a), UInt16(b)) => a == b,
            (Int32(a), Int32(b)) => a == b,
            (UInt32(a), UInt32(b)) => a == b,
            (Int64(a), Int64(b)) => a == b,
            (UInt64(a), UInt64(b)) => a == b,
            (Single(a), Single(b)) => a == b,
            (Double(a), Double(b)) => a == b,
            (String(a), String(b)) => a == b,
            (Type(a), Type(b)) => a.equals(&**b),
            (Array(a), Array(b)) => a == b,
            _ => false,
        }
    }
}

impl fmt::Debug for XamlValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use XamlValue::*;
        match self {
            Null => write!(f, "null"),
            Boolean(v) => write!(f, "{v}"),
            Char(v) => write!(f, "{v:?}"),
            SByte(v) => write!(f, "{v}i8"),
            Byte(v) => write!(f, "{v}u8"),
            Int16(v) => write!(f, "{v}i16"),
            UInt16(v) => write!(f, "{v}u16"),
            Int32(v) => write!(f, "{v}i32"),
            UInt32(v) => write!(f, "{v}u32"),
            Int64(v) => write!(f, "{v}i64"),
            UInt64(v) => write!(f, "{v}u64"),
            Single(v) => write!(f, "{v}f32"),
            Double(v) => write!(f, "{v}f64"),
            String(v) => write!(f, "{v:?}"),
            Type(t) => write!(f, "typeof({})", t.full_name()),
            Array(a) => f.debug_list().entries(a.iter()).finish(),
        }
    }
}

/// `object.ToString()` of the boxed value.
impl fmt::Display for XamlValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use XamlValue::*;
        match self {
            Null => Ok(()),
            Boolean(v) => write!(f, "{}", if *v { "True" } else { "False" }),
            Char(v) => write!(f, "{v}"),
            SByte(v) => write!(f, "{v}"),
            Byte(v) => write!(f, "{v}"),
            Int16(v) => write!(f, "{v}"),
            UInt16(v) => write!(f, "{v}"),
            Int32(v) => write!(f, "{v}"),
            UInt32(v) => write!(f, "{v}"),
            Int64(v) => write!(f, "{v}"),
            UInt64(v) => write!(f, "{v}"),
            Single(v) => write!(f, "{v}"),
            Double(v) => write!(f, "{v}"),
            String(v) => write!(f, "{v}"),
            Type(t) => write!(f, "{}", t.full_name()),
            Array(_) => write!(f, "System.Object[]"),
        }
    }
}

/// `IXamlType.Id`: an opaque, hashable identity used as a cache key.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum XamlTypeId {
    /// Process-unique identity (the `Guid.NewGuid()` of pseudo types).
    Unique(u64),
    /// Identity derived from a stable name.
    Named(String),
}

impl XamlTypeId {
    pub fn new_unique() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(1);
        XamlTypeId::Unique(NEXT.fetch_add(1, Ordering::Relaxed))
    }
}

pub trait IXamlType: 'static {
    fn id(&self) -> XamlTypeId;
    fn name(&self) -> String;
    fn namespace(&self) -> Option<String>;
    fn full_name(&self) -> String;
    fn is_public(&self) -> bool;
    fn is_nested_private(&self) -> bool;
    fn assembly(&self) -> Option<Rc<dyn IXamlAssembly>>;
    fn properties(&self) -> Vec<Rc<dyn IXamlProperty>>;
    fn events(&self) -> Vec<Rc<dyn IXamlEventInfo>>;
    fn fields(&self) -> Vec<Rc<dyn IXamlField>>;
    fn methods(&self) -> Vec<Rc<dyn IXamlMethod>>;
    fn constructors(&self) -> Vec<Rc<dyn IXamlConstructor>>;
    fn custom_attributes(&self) -> Vec<Rc<dyn IXamlCustomAttribute>>;
    fn generic_arguments(&self) -> Vec<Rc<dyn IXamlType>>;
    fn is_assignable_from(&self, type_: &dyn IXamlType) -> bool;
    fn make_generic_type(
        &self,
        type_arguments: &[Rc<dyn IXamlType>],
    ) -> XamlResult<Rc<dyn IXamlType>>;
    fn generic_type_definition(&self) -> Option<Rc<dyn IXamlType>>;
    fn is_array(&self) -> bool;
    fn array_element_type(&self) -> Option<Rc<dyn IXamlType>>;
    fn make_array_type(&self, dimensions: i32) -> XamlResult<Rc<dyn IXamlType>>;
    fn base_type(&self) -> Option<Rc<dyn IXamlType>>;
    fn declaring_type(&self) -> Option<Rc<dyn IXamlType>>;
    fn is_value_type(&self) -> bool;
    fn is_enum(&self) -> bool;
    fn interfaces(&self) -> Vec<Rc<dyn IXamlType>>;
    fn is_interface(&self) -> bool;
    fn get_enum_underlying_type(&self) -> XamlResult<Rc<dyn IXamlType>>;
    fn generic_parameters(&self) -> Vec<Rc<dyn IXamlType>>;
    fn is_function_pointer(&self) -> bool;
    /// `IEquatable<IXamlType>.Equals`.
    fn equals(&self, other: &dyn IXamlType) -> bool;
    fn get_hash_code(&self) -> u64;
    fn as_any(&self) -> &dyn Any;
    /// `object.ToString()`.
    fn to_type_string(&self) -> String {
        self.full_name()
    }
}

pub trait IXamlParameterInfo: 'static {
    fn parameter_type(&self) -> Rc<dyn IXamlType>;
    fn custom_attributes(&self) -> Vec<Rc<dyn IXamlCustomAttribute>>;
}

pub trait IXamlMember: 'static {
    fn name(&self) -> String;
    fn declaring_type(&self) -> Rc<dyn IXamlType>;
}

pub trait IXamlMethod: IXamlMember {
    fn is_public(&self) -> bool;
    fn is_private(&self) -> bool;
    fn is_family(&self) -> bool;
    fn is_static(&self) -> bool;
    fn contains_generic_parameters(&self) -> bool;
    fn is_generic_method(&self) -> bool;
    fn is_generic_method_definition(&self) -> bool;
    fn return_type(&self) -> Rc<dyn IXamlType>;
    fn parameters(&self) -> Vec<Rc<dyn IXamlType>>;
    fn make_generic_method(
        &self,
        type_arguments: &[Rc<dyn IXamlType>],
    ) -> XamlResult<Rc<dyn IXamlMethod>>;
    fn custom_attributes(&self) -> Vec<Rc<dyn IXamlCustomAttribute>>;
    fn get_parameter_info(&self, index: usize) -> XamlResult<Rc<dyn IXamlParameterInfo>>;
    fn generic_parameters(&self) -> Vec<Rc<dyn IXamlType>>;
    fn generic_arguments(&self) -> Vec<Rc<dyn IXamlType>>;
    /// `IEquatable<IXamlMethod>.Equals`.
    fn equals(&self, other: &dyn IXamlMethod) -> bool;
    fn get_hash_code(&self) -> u64;
    fn as_any(&self) -> &dyn Any;
}

pub trait IXamlConstructor: IXamlMember {
    fn is_public(&self) -> bool;
    fn is_static(&self) -> bool;
    fn parameters(&self) -> Vec<Rc<dyn IXamlType>>;
    fn get_parameter_info(&self, index: usize) -> XamlResult<Rc<dyn IXamlParameterInfo>>;
    fn equals(&self, other: &dyn IXamlConstructor) -> bool;
    fn as_any(&self) -> &dyn Any;
}

pub trait IXamlProperty: IXamlMember {
    fn property_type(&self) -> Rc<dyn IXamlType>;
    fn setter(&self) -> Option<Rc<dyn IXamlMethod>>;
    fn getter(&self) -> Option<Rc<dyn IXamlMethod>>;
    fn custom_attributes(&self) -> Vec<Rc<dyn IXamlCustomAttribute>>;
    fn indexer_parameters(&self) -> Vec<Rc<dyn IXamlType>>;
    fn equals(&self, other: &dyn IXamlProperty) -> bool;
    fn as_any(&self) -> &dyn Any;
}

pub trait IXamlEventInfo: IXamlMember {
    fn add(&self) -> Option<Rc<dyn IXamlMethod>>;
    fn equals(&self, other: &dyn IXamlEventInfo) -> bool;
    fn as_any(&self) -> &dyn Any;
}

pub trait IXamlField: IXamlMember {
    fn field_type(&self) -> Rc<dyn IXamlType>;
    fn is_public(&self) -> bool;
    fn is_static(&self) -> bool;
    fn is_literal(&self) -> bool;
    fn get_literal_value(&self) -> XamlResult<XamlValue>;
    fn custom_attributes(&self) -> Vec<Rc<dyn IXamlCustomAttribute>>;
    fn equals(&self, other: &dyn IXamlField) -> bool;
    fn as_any(&self) -> &dyn Any;
}

pub trait IXamlAssembly: 'static {
    fn name(&self) -> String;
    fn custom_attributes(&self) -> Vec<Rc<dyn IXamlCustomAttribute>>;
    fn find_type(&self, full_name: &str) -> Option<Rc<dyn IXamlType>>;
    fn equals(&self, other: &dyn IXamlAssembly) -> bool;
    fn as_any(&self) -> &dyn Any;
}

pub trait IXamlCustomAttribute: 'static {
    fn type_(&self) -> Rc<dyn IXamlType>;
    fn parameters(&self) -> Vec<XamlValue>;
    fn properties(&self) -> HashMap<String, XamlValue>;
    fn equals(&self, other: &dyn IXamlCustomAttribute) -> bool;
    fn as_any(&self) -> &dyn Any;
}

pub trait IXamlTypeSystem: 'static {
    fn assemblies(&self) -> Vec<Rc<dyn IXamlAssembly>>;
    fn well_known_types(&self) -> Rc<XamlTypeWellKnownTypes>;
    fn find_assembly(&self, substring: &str) -> Option<Rc<dyn IXamlAssembly>>;
    fn find_type(&self, name: &str) -> Option<Rc<dyn IXamlType>>;
    /// `FindType(string name, string assembly)`.
    fn find_type_in_assembly(&self, name: &str, assembly: &str) -> Option<Rc<dyn IXamlType>>;
}

pub trait IFileSource: 'static {
    fn file_path(&self) -> String;
    fn file_contents(&self) -> Vec<u8>;
}

pub trait IXamlLocal: 'static {
    fn as_any(&self) -> &dyn Any;
}

pub trait IXamlLabel: 'static {
    fn as_any(&self) -> &dyn Any;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum XamlVisibility {
    Public,
    Assembly,
    Private,
}

/// A type under construction. `TBackendEmitter` is the backend's code generator handle.
pub trait IXamlTypeBuilder<TBackendEmitter>: IXamlType {
    fn define_field(
        &self,
        type_: &Rc<dyn IXamlType>,
        name: &str,
        visibility: XamlVisibility,
        is_static: bool,
    ) -> XamlResult<Rc<dyn IXamlField>>;
    fn add_interface_implementation(&self, type_: &Rc<dyn IXamlType>) -> XamlResult<()>;
    #[allow(clippy::too_many_arguments)]
    fn define_method(
        &self,
        return_type: &Rc<dyn IXamlType>,
        args: &[Rc<dyn IXamlType>],
        name: &str,
        visibility: XamlVisibility,
        is_static: bool,
        is_interface_impl: bool,
        override_method: Option<&Rc<dyn IXamlMethod>>,
    ) -> XamlResult<Rc<dyn IXamlMethodBuilder<TBackendEmitter>>>;
    fn define_property(
        &self,
        property_type: &Rc<dyn IXamlType>,
        name: &str,
        setter: Option<&Rc<dyn IXamlMethod>>,
        getter: Option<&Rc<dyn IXamlMethod>>,
    ) -> XamlResult<Rc<dyn IXamlProperty>>;
    fn define_constructor(
        &self,
        is_static: bool,
        args: &[Rc<dyn IXamlType>],
    ) -> XamlResult<Rc<dyn IXamlConstructorBuilder<TBackendEmitter>>>;
    fn create_type(&self) -> XamlResult<Rc<dyn IXamlType>>;
    fn define_sub_type(
        &self,
        base_type: &Rc<dyn IXamlType>,
        name: &str,
        visibility: XamlVisibility,
    ) -> XamlResult<Rc<dyn IXamlTypeBuilder<TBackendEmitter>>>;
    fn define_delegate_sub_type(
        &self,
        name: &str,
        visibility: XamlVisibility,
        return_type: &Rc<dyn IXamlType>,
        parameter_types: &[Rc<dyn IXamlType>],
    ) -> XamlResult<Rc<dyn IXamlTypeBuilder<TBackendEmitter>>>;
    fn define_generic_parameters(
        &self,
        names: &[(String, XamlGenericParameterConstraint)],
    ) -> XamlResult<()>;
}

pub trait IXamlMethodBuilder<TBackendEmitter>: IXamlMethod {
    fn generator(&self) -> TBackendEmitter;
}

pub trait IXamlConstructorBuilder<TBackendEmitter>: IXamlConstructor {
    fn generator(&self) -> TBackendEmitter;
}

pub trait IXamlDelegateTypeBuilder {
    fn define_delegate_type(
        &self,
        return_type: &Rc<dyn IXamlType>,
        argument_types: &[Rc<dyn IXamlType>],
    ) -> XamlResult<Rc<dyn IXamlType>>;
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct XamlGenericParameterConstraint {
    pub is_class: bool,
}

/// Hash-map key wrapper giving `IXamlType` the `Equals`/`GetHashCode` dictionary semantics.
#[derive(Clone)]
pub struct XamlTypeKey(pub Rc<dyn IXamlType>);

impl PartialEq for XamlTypeKey {
    fn eq(&self, other: &Self) -> bool {
        self.0.equals(&*other.0)
    }
}

impl Eq for XamlTypeKey {}

impl Hash for XamlTypeKey {
    fn hash<H: Hasher>(&self, state: &mut H) {
        state.write_u64(self.0.get_hash_code());
    }
}

/// A type that only exists inside the compiler (`{x:Null}`, unknown and unresolved types).
pub struct XamlPseudoType {
    id: XamlTypeId,
    name: String,
}

thread_local! {
    static PSEUDO_NULL: Rc<dyn IXamlType> = Rc::new(XamlPseudoType::new("{x:Null}"));
    static PSEUDO_UNKNOWN: Rc<dyn IXamlType> = Rc::new(XamlPseudoType::new("{Unknown type}"));
}

impl XamlPseudoType {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            id: XamlTypeId::new_unique(),
            name: name.into(),
        }
    }

    /// `XamlPseudoType.Null` (one instance per thread; handles are not `Send`).
    pub fn null() -> Rc<dyn IXamlType> {
        PSEUDO_NULL.with(|t| t.clone())
    }

    /// `XamlPseudoType.Unknown`.
    pub fn unknown() -> Rc<dyn IXamlType> {
        PSEUDO_UNKNOWN.with(|t| t.clone())
    }

    pub fn unresolved(message: &str) -> Rc<dyn IXamlType> {
        Rc::new(XamlPseudoType::new(format!(
            "{{Unresolved type: '{message}'}}"
        )))
    }

    /// `type == XamlPseudoType.Null`.
    pub fn is_null(type_: &dyn IXamlType) -> bool {
        PSEUDO_NULL.with(|t| t.id() == type_.id())
    }

    /// `type == XamlPseudoType.Unknown`.
    pub fn is_unknown(type_: &dyn IXamlType) -> bool {
        PSEUDO_UNKNOWN.with(|t| t.id() == type_.id())
    }
}

impl IXamlType for XamlPseudoType {
    fn id(&self) -> XamlTypeId {
        self.id.clone()
    }
    fn name(&self) -> String {
        self.name.clone()
    }
    fn namespace(&self) -> Option<String> {
        Some(String::new())
    }
    fn full_name(&self) -> String {
        self.name.clone()
    }
    fn is_public(&self) -> bool {
        true
    }
    fn is_nested_private(&self) -> bool {
        false
    }
    fn assembly(&self) -> Option<Rc<dyn IXamlAssembly>> {
        None
    }
    fn properties(&self) -> Vec<Rc<dyn IXamlProperty>> {
        Vec::new()
    }
    fn events(&self) -> Vec<Rc<dyn IXamlEventInfo>> {
        Vec::new()
    }
    fn fields(&self) -> Vec<Rc<dyn IXamlField>> {
        Vec::new()
    }
    fn methods(&self) -> Vec<Rc<dyn IXamlMethod>> {
        Vec::new()
    }
    fn constructors(&self) -> Vec<Rc<dyn IXamlConstructor>> {
        Vec::new()
    }
    fn custom_attributes(&self) -> Vec<Rc<dyn IXamlCustomAttribute>> {
        Vec::new()
    }
    fn generic_arguments(&self) -> Vec<Rc<dyn IXamlType>> {
        Vec::new()
    }
    fn is_assignable_from(&self, type_: &dyn IXamlType) -> bool {
        type_.id() == self.id
    }
    fn make_generic_type(
        &self,
        _type_arguments: &[Rc<dyn IXamlType>],
    ) -> XamlResult<Rc<dyn IXamlType>> {
        Err(XamlError::not_supported(
            "Specified method is not supported.",
        ))
    }
    fn generic_type_definition(&self) -> Option<Rc<dyn IXamlType>> {
        None
    }
    fn is_array(&self) -> bool {
        false
    }
    fn array_element_type(&self) -> Option<Rc<dyn IXamlType>> {
        None
    }
    fn make_array_type(&self, _dimensions: i32) -> XamlResult<Rc<dyn IXamlType>> {
        Err(XamlError::internal(
            "NullReferenceException",
            "Object reference not set to an instance of an object.",
        ))
    }
    fn base_type(&self) -> Option<Rc<dyn IXamlType>> {
        None
    }
    fn declaring_type(&self) -> Option<Rc<dyn IXamlType>> {
        None
    }
    fn is_value_type(&self) -> bool {
        false
    }
    fn is_enum(&self) -> bool {
        false
    }
    fn interfaces(&self) -> Vec<Rc<dyn IXamlType>> {
        Vec::new()
    }
    fn is_interface(&self) -> bool {
        false
    }
    fn get_enum_underlying_type(&self) -> XamlResult<Rc<dyn IXamlType>> {
        Err(XamlError::invalid_operation(
            "Operation is not valid due to the current state of the object.",
        ))
    }
    fn generic_parameters(&self) -> Vec<Rc<dyn IXamlType>> {
        Vec::new()
    }
    fn is_function_pointer(&self) -> bool {
        false
    }
    fn equals(&self, other: &dyn IXamlType) -> bool {
        other.id() == self.id
    }
    fn get_hash_code(&self) -> u64 {
        match self.id {
            XamlTypeId::Unique(v) => v,
            XamlTypeId::Named(_) => 0,
        }
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn to_type_string(&self) -> String {
        "XamlX.TypeSystem.XamlPseudoType".to_string()
    }
}

pub struct FindMethodMethodSignature {
    pub name: String,
    pub return_type: Rc<dyn IXamlType>,
    pub is_static: bool,
    pub is_exact_match: bool,
    pub declaring_only: bool,
    pub parameters: Vec<Rc<dyn IXamlType>>,
}

impl FindMethodMethodSignature {
    pub fn new(
        name: impl Into<String>,
        return_type: Rc<dyn IXamlType>,
        parameters: Vec<Rc<dyn IXamlType>>,
    ) -> Self {
        Self {
            name: name.into(),
            return_type,
            is_static: false,
            is_exact_match: true,
            declaring_only: false,
            parameters,
        }
    }
}

impl fmt::Display for FindMethodMethodSignature {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let params: Vec<String> = self.parameters.iter().map(|p| p.get_full_name()).collect();
        write!(
            f,
            "{} {} {} ({}) (exact match: {}, declaring only: {})",
            if self.is_static { "static" } else { "instance" },
            self.return_type.get_full_name(),
            self.name,
            params.join(", "),
            if self.is_exact_match { "True" } else { "False" },
            if self.declaring_only { "True" } else { "False" },
        )
    }
}

pub struct AnonymousParameterInfo {
    pub name: String,
    parameter_type: Rc<dyn IXamlType>,
}

impl AnonymousParameterInfo {
    pub fn new(type_: Rc<dyn IXamlType>, name: Option<&str>) -> Self {
        Self {
            name: name.unwrap_or("unknown").to_string(),
            parameter_type: type_,
        }
    }

    pub fn with_index(type_: Rc<dyn IXamlType>, index: usize) -> Self {
        Self::new(type_, Some(&format!("arg{index}")))
    }
}

impl IXamlParameterInfo for AnonymousParameterInfo {
    fn parameter_type(&self) -> Rc<dyn IXamlType> {
        self.parameter_type.clone()
    }
    fn custom_attributes(&self) -> Vec<Rc<dyn IXamlCustomAttribute>> {
        Vec::new()
    }
}

/// `XamlTypeSystemExtensions` for `IXamlTypeSystem`.
impl dyn IXamlTypeSystem {
    pub fn get_type(&self, type_: &str) -> XamlResult<Rc<dyn IXamlType>> {
        get_type(self, type_)
    }
}

/// `XamlTypeSystemExtensions.GetType`, callable with any (possibly sized) type system.
pub fn get_type<S: IXamlTypeSystem + ?Sized>(
    sys: &S,
    type_: &str,
) -> XamlResult<Rc<dyn IXamlType>> {
    sys.find_type(type_)
        .ok_or_else(|| XamlError::type_system_exception(format!("Unable to resolve type {type_}")))
}

fn params_match(
    actual: &[Rc<dyn IXamlType>],
    wanted: &[Rc<dyn IXamlType>],
    allow_downcast: bool,
) -> bool {
    for c in 0..wanted.len() {
        let mismatch = if allow_downcast {
            !actual[c].is_assignable_from(&*wanted[c])
        } else {
            !actual[c].equals(&*wanted[c])
        };
        if mismatch {
            return false;
        }
    }
    true
}

/// `XamlTypeSystemExtensions` for `IXamlType`.
impl dyn IXamlType {
    pub fn get_fqn(&self) -> String {
        format!(
            "{}:{}.{}",
            self.assembly().map(|a| a.name()).unwrap_or_default(),
            self.namespace().unwrap_or_default(),
            self.name()
        )
    }

    pub fn get_full_name(&self) -> String {
        let mut name = self.name();
        if let Some(ns) = self.namespace() {
            name = format!("{ns}.{name}");
        }
        if let Some(asm) = self.assembly() {
            name.push(',');
            name.push_str(&asm.name());
        }
        name
    }

    pub fn find_methods(
        &self,
        criteria: impl Fn(&dyn IXamlMethod) -> bool,
    ) -> Vec<Rc<dyn IXamlMethod>> {
        let mut rv = Vec::new();
        rv.extend(self.methods().into_iter().filter(|m| criteria(&**m)));
        let mut t = self.base_type();
        while let Some(bt) = t {
            rv.extend(bt.methods().into_iter().filter(|m| criteria(&**m)));
            t = bt.base_type();
        }
        for iface in self.interfaces() {
            rv.extend(iface.methods().into_iter().filter(|m| criteria(&**m)));
        }
        rv
    }

    /// `GetMethod(Func<IXamlMethod, bool>)`.
    pub fn get_method(
        &self,
        criteria: impl Fn(&dyn IXamlMethod) -> bool,
    ) -> XamlResult<Rc<dyn IXamlMethod>> {
        self.find_method(criteria).ok_or_else(|| {
            XamlError::type_system_exception(format!("Method not found on type {}", self.get_fqn()))
        })
    }

    /// `FindMethod(Func<IXamlMethod, bool>)`.
    pub fn find_method(
        &self,
        criteria: impl Fn(&dyn IXamlMethod) -> bool,
    ) -> Option<Rc<dyn IXamlMethod>> {
        if let Some(m) = self.methods().into_iter().find(|m| criteria(&**m)) {
            return Some(m);
        }
        let mut t = self.base_type();
        while let Some(bt) = t {
            if let Some(m) = bt.methods().into_iter().find(|m| criteria(&**m)) {
                return Some(m);
            }
            t = bt.base_type();
        }
        for iface in self.interfaces() {
            if let Some(m) = iface.methods().into_iter().find(|m| criteria(&**m)) {
                return Some(m);
            }
        }
        None
    }

    /// `GetMethod(string name, IXamlType returnType, bool allowDowncast, params IXamlType[] args)`.
    pub fn get_method_by_name(
        &self,
        name: &str,
        return_type: &dyn IXamlType,
        allow_downcast: bool,
        args: &[Rc<dyn IXamlType>],
    ) -> XamlResult<Rc<dyn IXamlMethod>> {
        self.find_method_by_name(name, return_type, allow_downcast, args)
            .ok_or_else(|| {
                XamlError::type_system_exception(format!(
                    "Method {name} not found with matching signature on type {}",
                    self.get_fqn()
                ))
            })
    }

    /// `FindMethod(string name, IXamlType returnType, bool allowDowncast, params IXamlType[] args)`.
    pub fn find_method_by_name(
        &self,
        name: &str,
        return_type: &dyn IXamlType,
        allow_downcast: bool,
        args: &[Rc<dyn IXamlType>],
    ) -> Option<Rc<dyn IXamlMethod>> {
        for m in self.methods() {
            let parameters = m.parameters();
            if m.name() == name
                && m.return_type().equals(return_type)
                && parameters.len() == args.len()
                && params_match(&parameters, args, allow_downcast)
            {
                return Some(m);
            }
        }
        self.base_type()
            .and_then(|bt| bt.find_method_by_name(name, return_type, allow_downcast, args))
    }

    /// `GetMethod(FindMethodMethodSignature)`.
    pub fn get_method_by_signature(
        &self,
        signature: &FindMethodMethodSignature,
    ) -> XamlResult<Rc<dyn IXamlMethod>> {
        self.find_method_by_signature(signature).ok_or_else(|| {
            XamlError::type_system_exception(format!(
                "Method with signature {signature} is not found on type {}",
                self.get_fqn()
            ))
        })
    }

    /// `FindMethod(FindMethodMethodSignature)`.
    pub fn find_method_by_signature(
        &self,
        signature: &FindMethodMethodSignature,
    ) -> Option<Rc<dyn IXamlMethod>> {
        for m in self.methods() {
            let parameters = m.parameters();
            if m.name() == signature.name
                && m.return_type().equals(&*signature.return_type)
                && parameters.len() == signature.parameters.len()
                && m.is_static() == signature.is_static
                && params_match(
                    &parameters,
                    &signature.parameters,
                    !signature.is_exact_match,
                )
            {
                return Some(m);
            }
        }
        if signature.declaring_only {
            return None;
        }
        self.base_type()
            .and_then(|bt| bt.find_method_by_signature(signature))
    }

    pub fn get_constructor(
        &self,
        args: Option<&[Rc<dyn IXamlType>]>,
    ) -> XamlResult<Rc<dyn IXamlConstructor>> {
        if let Some(found) = self.find_constructor(args) {
            return Ok(found);
        }
        match args {
            Some(args) if !args.is_empty() => {
                let args_string: Vec<String> = args.iter().map(|a| a.get_full_name()).collect();
                Err(XamlError::type_system_exception(format!(
                    "Constructor with arguments {} is not found on type {}",
                    args_string.join(", "),
                    self.get_fqn()
                )))
            }
            _ => Err(XamlError::type_system_exception(format!(
                "Constructor with no arguments is not found on type {}",
                self.get_fqn()
            ))),
        }
    }

    pub fn find_constructor(
        &self,
        args: Option<&[Rc<dyn IXamlType>]>,
    ) -> Option<Rc<dyn IXamlConstructor>> {
        let args = args.unwrap_or(&[]);
        self.constructors().into_iter().find(|ctor| {
            if !ctor.is_public() || ctor.is_static() {
                return false;
            }
            let parameters = ctor.parameters();
            parameters.len() == args.len() && params_match(&parameters, args, true)
        })
    }

    pub fn accepts_null(&self) -> bool {
        !self.is_value_type() || self.is_nullable()
    }

    pub fn is_nullable(&self) -> bool {
        match self.generic_type_definition() {
            Some(def) => def.is("System", "Nullable`1"),
            None => false,
        }
    }

    pub fn is_nullable_of(&self, vtype: &dyn IXamlType) -> bool {
        self.is_nullable()
            && self
                .generic_arguments()
                .first()
                .is_some_and(|a| a.equals(vtype))
    }

    pub fn get_all_interfaces(&self) -> Vec<Rc<dyn IXamlType>> {
        let mut rv = self.interfaces();
        let mut t = self.base_type();
        while let Some(bt) = t {
            rv.extend(bt.interfaces());
            t = bt.base_type();
        }
        rv
    }

    pub fn get_all_custom_attributes(&self) -> Vec<Rc<dyn IXamlCustomAttribute>> {
        let mut rv = self.custom_attributes();
        let mut t = self.base_type();
        while let Some(bt) = t {
            for attribute in bt.custom_attributes() {
                let usage_attribute = attribute.type_().custom_attributes().into_iter().find(|a| {
                    let ty = a.type_();
                    ty.name() == "AttributeUsageAttribute"
                        && ty.namespace().as_deref() == Some("System")
                });
                let inherited = match usage_attribute {
                    None => true,
                    Some(usage) => {
                        usage
                            .properties()
                            .get("Inherited")
                            .and_then(|v| v.as_bool())
                            == Some(true)
                    }
                };
                if inherited {
                    rv.push(attribute);
                }
            }
            t = bt.base_type();
        }
        rv
    }

    pub fn get_all_properties(&self) -> Vec<Rc<dyn IXamlProperty>> {
        let mut rv = self.properties();
        let mut t = self.base_type();
        while let Some(bt) = t {
            rv.extend(bt.properties());
            t = bt.base_type();
        }
        rv
    }

    pub fn get_all_fields(&self) -> Vec<Rc<dyn IXamlField>> {
        let mut rv = self.fields();
        let mut t = self.base_type();
        while let Some(bt) = t {
            rv.extend(bt.fields());
            t = bt.base_type();
        }
        rv
    }

    pub fn get_all_events(&self) -> Vec<Rc<dyn IXamlEventInfo>> {
        let mut rv = self.events();
        let mut t = self.base_type();
        while let Some(bt) = t {
            rv.extend(bt.events());
            t = bt.base_type();
        }
        rv
    }

    pub fn is_directly_assignable_from(&self, other: &dyn IXamlType) -> bool {
        if self.is_value_type() || other.is_value_type() {
            return self.equals(other);
        }
        self.is_assignable_from(other)
    }

    /// `Is(string ns, string name)`.
    pub fn is(&self, ns: &str, name: &str) -> bool {
        self.name() == name && self.namespace().as_deref() == Some(ns)
    }
}

/// `XamlTypeSystemExtensions` for `IXamlMethod`.
impl dyn IXamlMethod {
    /// Fails (instead of throwing `ArgumentOutOfRangeException`) for a static method without parameters.
    pub fn this_or_first_parameter(&self) -> XamlResult<Rc<dyn IXamlType>> {
        if self.is_static() {
            self.parameters().into_iter().next().ok_or_else(|| {
                XamlError::internal(
                    "ArgumentOutOfRangeException",
                    format!("Static method {} doesn't have any parameters", self.name()),
                )
            })
        } else {
            Ok(self.declaring_type())
        }
    }

    pub fn parameters_with_this(&self) -> Vec<Rc<dyn IXamlType>> {
        if self.is_static() {
            return self.parameters();
        }
        let mut lst = self.parameters();
        lst.insert(0, self.declaring_type());
        lst
    }
}

/// `IReadOnlyList<IXamlType>.SequenceEqual`.
pub fn xaml_types_sequence_equal(a: &[Rc<dyn IXamlType>], b: &[Rc<dyn IXamlType>]) -> bool {
    a.len() == b.len() && a.iter().zip(b.iter()).all(|(x, y)| x.equals(&**y))
}
