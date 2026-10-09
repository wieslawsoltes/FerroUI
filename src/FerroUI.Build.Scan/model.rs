//! The build-time type model of a crate and its serialised form, the
//! `.xamlmeta` file (docs/porting/xaml.md, 9.5.1).
//!
//! The model states what the declarations of a crate say about its types:
//! the classes and their registered properties, the markup metadata of
//! classes and of other types, the enumerations, the namespaces and the
//! assembly. It is filled by the source scanner ([`crate::scanner`]) or read
//! from the `.xamlmeta` of a crate whose sources are not read. Everything in
//! it is owned text; a type is referred to by its Rust type text
//! ([`RustType`]), never by a handle of the process.
//!
//! # The file
//!
//! JSON ([`crate::json`]). Two formats exist, told apart by the member
//! `"format"`:
//!
//! - no `"format"` member: format 1, the file the emitter writes today
//!   (`rust_emitter::XamlMetadata`): `name`, `crate_name`, `documents`,
//!   `dependencies`. [`AssemblyModel::parse`] reads it as a model without
//!   types.
//! - `"format": 2`: the four members of format 1, unchanged, and the type
//!   model next to them. The reader of format 1 looks its members up by name
//!   and ignores the others, so it reads a file of format 2 as the documents
//!   of the crate.
//!
//! A member whose value is the default of its kind (an empty list, `false`,
//! nothing) is left out of a file of format 2, except the four members of
//! format 1, which are always written.

use crate::json::{text_of, Fields, Json, Members};
pub use crate::xaml_metadata::{DocumentModel, XamlMetadata};

/// The format [`AssemblyModel::to_json`] writes.
pub const FORMAT: i64 = 2;

/// A Rust type as normalised text (9.5.2, step 1): the tokens of the type as the
/// declaration writes it, spaced canonically, with every path the scanner resolved made
/// absolute (`::ferroui_base::Ref<::ferroui_controls::control::Control>`). A path of the
/// scanned crate names the module that declares the item; a path into another crate is
/// absolute as the `use` item of the file spells it. Names of the language prelude and
/// the primitive types stay as they are (`Option`, `String`, `f64`).
#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct RustType {
    /// The normalised text.
    pub text: String,
    /// The paths of the text the scanner could not resolve, as written, each once, in
    /// the order of the text. They are kept in the text as written.
    pub unresolved: Vec<String>,
}

impl RustType {
    /// A type whose every path is resolved.
    pub fn resolved(text: &str) -> Self {
        Self { text: text.to_string(), unresolved: Vec::new() }
    }

    /// Whether every path of the text is resolved.
    pub fn is_resolved(&self) -> bool {
        self.unresolved.is_empty()
    }

    fn to_json(&self) -> Json {
        if self.unresolved.is_empty() {
            Json::string(&self.text)
        } else {
            Json::object(Members::new().text("text", &self.text).list("unresolved", &self.unresolved, |path| Json::string(path)))
        }
    }

    fn from_json(value: &Json) -> Result<Self, String> {
        match value {
            Json::String(text) => Ok(Self::resolved(text)),
            _ => {
                let fields = Fields::of(value, "a type")?;
                Ok(Self { text: fields.text("text")?, unresolved: fields.list("unresolved", |path| text_of(path, "an unresolved path"))? })
            }
        }
    }
}

/// The kind of a type, as the declaration states it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TypeKind {
    /// A class of the object model (`ferro_class!`) or a plain class (`ferro_markup_type!(class ..)`).
    Class,
    Struct,
    Enum,
    Interface,
    /// A type without instances: the owner of attached properties or of static members.
    Static,
}

impl TypeKind {
    fn name(self) -> &'static str {
        match self {
            TypeKind::Class => "class",
            TypeKind::Struct => "struct",
            TypeKind::Enum => "enum",
            TypeKind::Interface => "interface",
            TypeKind::Static => "static",
        }
    }

    fn of(name: &str) -> Result<Self, String> {
        match name {
            "class" => Ok(TypeKind::Class),
            "struct" => Ok(TypeKind::Struct),
            "enum" => Ok(TypeKind::Enum),
            "interface" => Ok(TypeKind::Interface),
            "static" => Ok(TypeKind::Static),
            other => Err(format!("\"{other}\" is not a kind of type")),
        }
    }
}

/// A callable of a declaration: a path (`Setter::set_value`) or any other expression (a
/// closure).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CallableModel {
    /// The path as written, when the callable is a path; nothing for an expression.
    pub path: Option<String>,
    /// The path made absolute, when the scanner resolved its head.
    pub resolved: Option<String>,
    /// For a closure without parameters whose body dereferences the result of a function
    /// called without arguments (`|| *Button::click_event()`, the form a static field of
    /// a routed event is declared in): the path of that function, made absolute.
    pub dereferenced: Option<String>,
}

impl CallableModel {
    fn to_json(&self) -> Json {
        Json::object(
            Members::new().optional_text("path", &self.path).optional_text("resolved", &self.resolved).optional_text("dereferenced", &self.dereferenced),
        )
    }

    fn from_json(value: &Json) -> Result<Self, String> {
        let fields = Fields::of(value, "a callable")?;
        Ok(Self { path: fields.optional_text("path")?, resolved: fields.optional_text("resolved")?, dereferenced: fields.optional_text("dereferenced")? })
    }
}

/// How generated code calls a member (9.5.3). The scan chooses it for every callable of
/// the model once the models of the dependencies are attached ([`crate::call_forms`]).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CallForm {
    /// Form A: typed code derived from the declaration. The callable is a function a
    /// declaration macro writes for the type (the accessor of a routed event or of a
    /// registered property), or the closure that dereferences what such a function
    /// returns ([`CallableModel::dereferenced`]); generated code calls the function by
    /// the public path of the type.
    Structural,
    /// Form B: the public function with this absolute path (the public path of the type
    /// and the name of the function), called with the instance by reference and the
    /// arguments in the declared types.
    Path(String),
    /// Form B for code generated into the crate of the declaration only: the function is
    /// visible in its crate (`pub(crate)`). For any other crate the member is called as
    /// [`Invoker`](Self::Invoker).
    CratePath(String),
    /// Form C: the invoker of the metadata.
    Invoker,
}

impl CallForm {
    /// The form as a crate other than the declaring one calls the member.
    pub fn from_another_crate(&self) -> &CallForm {
        match self {
            CallForm::CratePath(_) => &CallForm::Invoker,
            other => other,
        }
    }

    fn to_json(&self) -> Json {
        match self {
            CallForm::Structural => Json::string("structural"),
            CallForm::Invoker => Json::string("invoker"),
            CallForm::Path(path) => Json::object(Members::new().text("path", path)),
            CallForm::CratePath(path) => Json::object(Members::new().text("path", path).flag("crate", true)),
        }
    }

    fn from_json(value: &Json) -> Result<Self, String> {
        match value {
            Json::String(text) if text == "structural" => Ok(CallForm::Structural),
            Json::String(text) if text == "invoker" => Ok(CallForm::Invoker),
            Json::String(text) => Err(format!("\"{text}\" is not a call form")),
            _ => {
                let fields = Fields::of(value, "a call form")?;
                let path = fields.text("path")?;
                Ok(if fields.flag("crate")? { CallForm::CratePath(path) } else { CallForm::Path(path) })
            }
        }
    }
}

/// An argument of an attribute.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AttributeValueModel {
    Null,
    Bool(bool),
    /// An integer or a character (its code point, as the metadata holds it).
    Int(i64),
    /// A floating point literal, as written.
    Float(String),
    Str(String),
    /// `type(T)`.
    Type(RustType),
    Array(Vec<AttributeValueModel>),
}

impl AttributeValueModel {
    fn to_json(&self) -> Json {
        match self {
            AttributeValueModel::Null => Json::Null,
            AttributeValueModel::Bool(value) => Json::Bool(*value),
            AttributeValueModel::Int(value) => Json::Integer(*value),
            AttributeValueModel::Float(text) => Json::Object(vec![("float".to_string(), Json::string(text))]),
            AttributeValueModel::Str(text) => Json::string(text),
            AttributeValueModel::Type(type_) => Json::Object(vec![("type".to_string(), type_.to_json())]),
            AttributeValueModel::Array(items) => Json::Array(items.iter().map(Self::to_json).collect()),
        }
    }

    fn from_json(value: &Json) -> Result<Self, String> {
        match value {
            Json::Null => Ok(AttributeValueModel::Null),
            Json::Bool(value) => Ok(AttributeValueModel::Bool(*value)),
            Json::Integer(value) => Ok(AttributeValueModel::Int(*value)),
            Json::Float(_) => Err("a floating point argument of an attribute is written as {\"float\": \"text\"}".to_string()),
            Json::String(text) => Ok(AttributeValueModel::Str(text.clone())),
            Json::Array(items) => Ok(AttributeValueModel::Array(items.iter().map(Self::from_json).collect::<Result<_, _>>()?)),
            Json::Object(_) => {
                let fields = Fields::of(value, "an argument of an attribute")?;
                if let Some(text) = fields.optional_text("float")? {
                    return Ok(AttributeValueModel::Float(text));
                }
                match fields.get("type") {
                    Some(type_) => Ok(AttributeValueModel::Type(RustType::from_json(type_)?)),
                    None => Err("an argument of an attribute is an object without \"float\" and \"type\"".to_string()),
                }
            }
        }
    }
}

/// An attribute of a type, a member or a parameter (`DependsOn("Property")`).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct AttributeModel {
    /// The name, without the `Attribute` suffix.
    pub name: String,
    /// The positional arguments.
    pub arguments: Vec<AttributeValueModel>,
    /// The named arguments (`IsRequired = true`).
    pub properties: Vec<(String, AttributeValueModel)>,
}

impl AttributeModel {
    fn to_json(&self) -> Json {
        Json::object(
            Members::new()
                .text("name", &self.name)
                .list("arguments", &self.arguments, AttributeValueModel::to_json)
                .list("properties", &self.properties, |(name, value)| Json::Array(vec![Json::string(name), value.to_json()])),
        )
    }

    fn from_json(value: &Json) -> Result<Self, String> {
        let fields = Fields::of(value, "an attribute")?;
        Ok(Self {
            name: fields.text("name")?,
            arguments: fields.list("arguments", AttributeValueModel::from_json)?,
            properties: fields.list("properties", |pair| match pair {
                Json::Array(pair) if pair.len() == 2 => Ok((text_of(&pair[0], "the name of a named argument")?, AttributeValueModel::from_json(&pair[1])?)),
                _ => Err("a named argument of an attribute is not a pair".to_string()),
            })?,
        })
    }
}

fn attributes_to_json(members: Members, attributes: &[AttributeModel]) -> Members {
    members.list("attributes", attributes, AttributeModel::to_json)
}

/// A parameter of a constructor, a method, an indexer or an event handler.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ParameterModel {
    /// The name, for a constructor that names its parameters.
    pub name: Option<String>,
    pub type_: RustType,
    pub attributes: Vec<AttributeModel>,
}

impl ParameterModel {
    fn to_json(&self) -> Json {
        if self.name.is_none() && self.attributes.is_empty() {
            return self.type_.to_json();
        }
        Json::object(attributes_to_json(Members::new().optional_text("name", &self.name).always("type", self.type_.to_json()), &self.attributes))
    }

    fn from_json(value: &Json) -> Result<Self, String> {
        if let Json::Object(members) = value {
            if members.iter().any(|(name, _)| name == "type") {
                let fields = Fields::of(value, "a parameter")?;
                let type_ = fields.get("type").ok_or_else(|| "\"type\" of a parameter is null".to_string())?;
                return Ok(Self {
                    name: fields.optional_text("name")?,
                    type_: RustType::from_json(type_)?,
                    attributes: fields.list("attributes", AttributeModel::from_json)?,
                });
            }
        }
        Ok(Self { name: None, type_: RustType::from_json(value)?, attributes: Vec::new() })
    }
}

/// A constructor, a method, a static field or an event of a type (`MemberModel`).
///
/// - a constructor: `name` is empty, `return_type` is nothing (the value type of the type);
/// - a method: `return_type` is nothing for a method without a result;
/// - a static field: no parameters, `return_type` is the type of the field;
/// - an event: the parameters are the ones of the handler, the callable subscribes.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct MemberModel {
    pub name: String,
    pub parameters: Vec<ParameterModel>,
    pub return_type: Option<RustType>,
    pub is_static: bool,
    /// Declared with `try`: the callable returns a `Result`.
    pub fallible: bool,
    pub attributes: Vec<AttributeModel>,
    /// The callable of the declaration.
    pub callable: CallableModel,
    /// The associated function of the declared type the declaration macro writes for the
    /// member with the feature `markup-functions` of the base crate (`__markup_new_0`,
    /// `__markup_Add_1`, `__markup_field_ClickEvent`); nothing for an event.
    pub typed_function: Option<String>,
    /// The call form (9.5.3), once it is chosen.
    pub call: Option<CallForm>,
}

impl MemberModel {
    fn to_json(&self) -> Json {
        let members = Members::new()
            .text_or_empty("name", &self.name)
            .list("parameters", &self.parameters, ParameterModel::to_json)
            .optional("return_type", self.return_type.as_ref().map(RustType::to_json))
            .flag("is_static", self.is_static)
            .flag("fallible", self.fallible);
        Json::object(
            attributes_to_json(members, &self.attributes)
                .always("callable", self.callable.to_json())
                .optional_text("typed_function", &self.typed_function)
                .optional("call", self.call.as_ref().map(CallForm::to_json)),
        )
    }

    fn from_json(value: &Json) -> Result<Self, String> {
        let fields = Fields::of(value, "a member")?;
        Ok(Self {
            name: fields.text_or_empty("name")?,
            parameters: fields.list("parameters", ParameterModel::from_json)?,
            return_type: fields.optional("return_type", RustType::from_json)?,
            is_static: fields.flag("is_static")?,
            fallible: fields.flag("fallible")?,
            attributes: fields.list("attributes", AttributeModel::from_json)?,
            callable: fields.optional("callable", CallableModel::from_json)?.unwrap_or_default(),
            typed_function: fields.optional_text("typed_function")?,
            call: fields.optional("call", CallForm::from_json)?,
        })
    }
}

/// An accessor of a plain property or of an indexer.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct AccessorModel {
    /// Declared with `try_get:` / `try_set:`.
    pub fallible: bool,
    pub callable: CallableModel,
    /// The typed function of the accessor (`__markup_get_Name`); nothing for an indexer.
    pub typed_function: Option<String>,
    /// The call form (9.5.3), once it is chosen.
    pub call: Option<CallForm>,
}

impl AccessorModel {
    fn to_json(&self) -> Json {
        Json::object(
            Members::new()
                .flag("fallible", self.fallible)
                .always("callable", self.callable.to_json())
                .optional_text("typed_function", &self.typed_function)
                .optional("call", self.call.as_ref().map(CallForm::to_json)),
        )
    }

    fn from_json(value: &Json) -> Result<Self, String> {
        let fields = Fields::of(value, "an accessor")?;
        Ok(Self {
            fallible: fields.flag("fallible")?,
            callable: fields.optional("callable", CallableModel::from_json)?.unwrap_or_default(),
            typed_function: fields.optional_text("typed_function")?,
            call: fields.optional("call", CallForm::from_json)?,
        })
    }
}

/// A plain (not registered) property of a type, instance or static, or an indexer
/// (`name` is empty and the parameters are the ones of the index).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PropertyModel {
    pub name: String,
    /// The parameters of an indexer.
    pub parameters: Vec<ParameterModel>,
    pub value_type: RustType,
    pub getter: Option<AccessorModel>,
    pub setter: Option<AccessorModel>,
    pub attributes: Vec<AttributeModel>,
}

impl PropertyModel {
    fn to_json(&self) -> Json {
        let members = Members::new()
            .text_or_empty("name", &self.name)
            .list("parameters", &self.parameters, ParameterModel::to_json)
            .always("value_type", self.value_type.to_json())
            .optional("getter", self.getter.as_ref().map(AccessorModel::to_json))
            .optional("setter", self.setter.as_ref().map(AccessorModel::to_json));
        Json::object(attributes_to_json(members, &self.attributes))
    }

    fn from_json(value: &Json) -> Result<Self, String> {
        let fields = Fields::of(value, "a property")?;
        Ok(Self {
            name: fields.text_or_empty("name")?,
            parameters: fields.list("parameters", ParameterModel::from_json)?,
            value_type: fields.optional("value_type", RustType::from_json)?.ok_or_else(|| "\"value_type\" is missing in a property".to_string())?,
            getter: fields.optional("getter", AccessorModel::from_json)?,
            setter: fields.optional("setter", AccessorModel::from_json)?,
            attributes: fields.list("attributes", AttributeModel::from_json)?,
        })
    }
}

/// The kind of a registered property, from the type of its accessor.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RegisteredKind {
    Styled,
    Direct,
    Attached,
}

impl RegisteredKind {
    fn name(self) -> &'static str {
        match self {
            RegisteredKind::Styled => "styled",
            RegisteredKind::Direct => "direct",
            RegisteredKind::Attached => "attached",
        }
    }

    fn of(name: &str) -> Result<Self, String> {
        match name {
            "styled" => Ok(RegisteredKind::Styled),
            "direct" => Ok(RegisteredKind::Direct),
            "attached" => Ok(RegisteredKind::Attached),
            other => Err(format!("\"{other}\" is not a kind of registered property")),
        }
    }
}

/// What the body of the accessor of a registered property does.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RegistrationModel {
    /// `FerroProperty::register*::<Owner, ..>("Name", ..)`: the property is declared here.
    Declared,
    /// `Other::name_property().add_owner*::<Owner>(..)`: a property another type declares,
    /// with this type as one more owner.
    AddedOwner,
    /// The body is the accessor of another property and nothing else: the same property
    /// under a second accessor.
    Alias,
    /// None of the forms: the scanner could not tell (a diagnostic says where).
    Unknown,
}

impl RegistrationModel {
    fn name(self) -> &'static str {
        match self {
            RegistrationModel::Declared => "declared",
            RegistrationModel::AddedOwner => "added_owner",
            RegistrationModel::Alias => "alias",
            RegistrationModel::Unknown => "unknown",
        }
    }

    fn of(name: &str) -> Result<Self, String> {
        match name {
            "declared" => Ok(RegistrationModel::Declared),
            "added_owner" => Ok(RegistrationModel::AddedOwner),
            "alias" => Ok(RegistrationModel::Alias),
            "unknown" => Ok(RegistrationModel::Unknown),
            other => Err(format!("\"{other}\" is not a registration")),
        }
    }
}

/// The accessor of a registered property of a type (`RegisteredModel`): one function of
/// its `ferro_properties!` block.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RegisteredModel {
    /// The name of the property (`Background`): the text of the registration, or, for an
    /// added owner and an alias, the name of the property the source accessor declares
    /// when that accessor is in the scanned crate. Nothing when the source is in another
    /// crate (its model has the name) or the registration was not read.
    pub name: Option<String>,
    pub kind: RegisteredKind,
    /// The value type: the last type argument of the type of the accessor.
    pub value_type: RustType,
    /// The owner the registration names: `register::<Owner, _>` of a declared property,
    /// `add_owner::<Owner>` of an added owner. The registry lists the property under this
    /// type, whichever type has the accessor.
    pub owner: Option<RustType>,
    /// The host type of an attached property (`register_attached::<Owner, Host, _>`).
    pub host: Option<RustType>,
    /// The name of the accessor function (`background_property`); its path is the path of
    /// the type and the name, unless the accessor is a function of another type
    /// ([`function_of`](Self::function_of)).
    pub accessor: String,
    /// The Rust path of the type whose function the accessor is, when it is not the type
    /// the property is listed under: `ferro_property!(for Owner; ..)` among the members of
    /// `impl Other` declares `Other::name_property()`, a property of `Owner`
    /// (`::ferroui_base::styling::theme_variant::ThemeVariant` for the theme variant
    /// properties of `StyledElement`).
    pub function_of: Option<String>,
    /// The visibility of the accessor, as written (`pub`, `pub(crate)`; empty when private).
    pub visibility: String,
    pub registration: RegistrationModel,
    /// The accessor the body calls, for an added owner and an alias, as written
    /// (`Decorator::child_property`).
    pub source: Option<CallableModel>,
    /// The body states `.assign_binding(true)` (or `set_assign_binding(true)`).
    pub assign_binding: bool,
    /// The body states `.inherits(true)`.
    pub inherits: bool,
    /// The registration of a direct property states no setter (`None` where the setter
    /// goes): the property is read-only.
    pub read_only: bool,
    /// The types of the scanned crate that add themselves as owners of the property, by
    /// their Rust path, in the order of the scan. Owners in other crates are in the models
    /// of those crates.
    pub added_owners: Vec<String>,
}

impl RegisteredModel {
    fn to_json(&self) -> Json {
        Json::object(
            Members::new()
                .optional_text("name", &self.name)
                .text("kind", self.kind.name())
                .always("value_type", self.value_type.to_json())
                .optional("owner", self.owner.as_ref().map(RustType::to_json))
                .optional("host", self.host.as_ref().map(RustType::to_json))
                .text("accessor", &self.accessor)
                .optional_text("function_of", &self.function_of)
                .text_or_empty("visibility", &self.visibility)
                .text("registration", self.registration.name())
                .optional("source", self.source.as_ref().map(CallableModel::to_json))
                .flag("assign_binding", self.assign_binding)
                .flag("inherits", self.inherits)
                .flag("read_only", self.read_only)
                .list("added_owners", &self.added_owners, |owner| Json::string(owner)),
        )
    }

    fn from_json(value: &Json) -> Result<Self, String> {
        let fields = Fields::of(value, "a registered property")?;
        Ok(Self {
            name: fields.optional_text("name")?,
            kind: RegisteredKind::of(&fields.text("kind")?)?,
            value_type: fields.optional("value_type", RustType::from_json)?.ok_or_else(|| "\"value_type\" is missing in a registered property".to_string())?,
            owner: fields.optional("owner", RustType::from_json)?,
            host: fields.optional("host", RustType::from_json)?,
            accessor: fields.text("accessor")?,
            function_of: fields.optional_text("function_of")?,
            visibility: fields.text_or_empty("visibility")?,
            registration: RegistrationModel::of(&fields.text("registration")?)?,
            source: fields.optional("source", CallableModel::from_json)?,
            assign_binding: fields.flag("assign_binding")?,
            inherits: fields.flag("inherits")?,
            read_only: fields.flag("read_only")?,
            added_owners: fields.list("added_owners", |owner| text_of(owner, "an added owner"))?,
        })
    }
}

/// A member of an enumeration.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct EnumMemberModel {
    /// The markup name.
    pub name: String,
    /// The Rust variant of a plain enumeration (`Self_` for the member `Self`); nothing
    /// for a member of a set of flags.
    pub rust_variant: Option<String>,
    /// The constant of a member of a set of flags, as written (`RoutingStrategies::DIRECT`).
    pub rust_value: Option<String>,
    /// The numeric value, when the scanner found the declaration of the enumeration (or
    /// of the flags) in the crate and could evaluate the member.
    pub value: Option<i64>,
}

impl EnumMemberModel {
    fn to_json(&self) -> Json {
        Json::object(
            Members::new()
                .text("name", &self.name)
                .optional_text("rust_variant", &self.rust_variant)
                .optional_text("rust_value", &self.rust_value)
                .optional("value", self.value.map(Json::Integer)),
        )
    }

    fn from_json(value: &Json) -> Result<Self, String> {
        let fields = Fields::of(value, "a member of an enumeration")?;
        Ok(Self {
            name: fields.text("name")?,
            rust_variant: fields.optional_text("rust_variant")?,
            rust_value: fields.optional_text("rust_value")?,
            value: fields.optional_integer("value")?,
        })
    }
}

/// `generic: "FerroList`1" [T]`: the generic definition a type instantiates.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct GenericModel {
    pub definition: String,
    pub arguments: Vec<RustType>,
}

/// A type of a crate (`TypeModel`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TypeModel {
    /// The dotted namespace: the explicit one, else the one the namespace table of the
    /// crate gives the declaring module; empty when the table has none.
    pub namespace: String,
    /// `namespace: ".."` of the markup metadata.
    pub explicit_namespace: Option<String>,
    /// The markup name.
    pub name: String,
    pub kind: TypeKind,
    /// The type is a type of the object model: a class (`ferro_class!`) or the static
    /// owner type of attached properties (`ferro_static_type!`), with a runtime type of
    /// its own. Its handles (`Ref<X>`, `Option<Ref<X>>` for a class) are implied.
    pub object_model: bool,
    /// The class states markup metadata of its own (`markup: { .. }` of a
    /// `ferro_class_info!`): its runtime type has metadata, whatever the part lists.
    pub class_markup: bool,
    /// The crate has a list of the classes it registers (`const TYPES: &[&TypeInfo]`, which
    /// its `register_types()` hands to `TypeInfo::register_all`) and the type is not in
    /// it: the type is not known by its name or by its handle until an instance of it is
    /// created. Never set for a crate without such a list, and never for a type that is
    /// not of the object model.
    pub unregistered: bool,
    /// The Rust type: for a type the crate declares, the absolute path of the declaring
    /// module and the name; for a type the metadata is declared for, the type as the
    /// declaration writes it, normalised.
    pub rust_path: RustType,
    /// The shortest path another crate names the type by, when the scanner found one
    /// (`::ferroui_controls::Border`).
    pub public_path: Option<String>,
    /// The path the crate states for the type where no public path is found: the one of an
    /// instantiation of a generic type (`generics:` of `ferro_rust_paths!`), written with
    /// the name the crate has for the generic type and the public paths of its arguments
    /// (`::ferroui_controls::FerroListOf<::ferroui_base::Ref<::ferroui_controls::RowDefinition>>`).
    pub stated_path: Option<String>,
    /// The module of the declaration that gives the type its namespace
    /// (`ferroui_controls::border`).
    pub module: String,
    /// The `cfg` conditions the declaration is under, as written, outermost first.
    pub cfg: Vec<String>,
    /// The Rust types that hold a value of the type, the untyped form first. For a class
    /// of the object model `Ref<X>` and `Option<Ref<X>>` are implied and not listed.
    pub handles: Vec<RustType>,
    /// The type instance members receive (`this:`).
    pub this: Option<RustType>,
    /// The base class of a class of the object model, or the base type by handle (`base:`).
    pub base: Option<RustType>,
    /// The contracts, by handle (`interfaces:`).
    pub interfaces: Vec<RustType>,
    pub generic: Option<GenericModel>,
    /// `type_info: X`: the runtime type the metadata of a static type belongs to.
    pub type_info: Option<RustType>,
    /// `new:` of a class: the constructor without parameters.
    pub default_constructor: Option<CallableModel>,
    pub content_property: Option<String>,
    /// `parse:` (`has_parse` of the design is `parse.is_some()`); its typed function is
    /// `__markup_parse`.
    pub parse: Option<CallableModel>,
    /// The call form of `parse:` (9.5.3), once it is chosen.
    pub parse_call: Option<CallForm>,
    pub constructors: Vec<MemberModel>,
    pub properties: Vec<PropertyModel>,
    pub static_properties: Vec<PropertyModel>,
    pub indexers: Vec<PropertyModel>,
    pub registered: Vec<RegisteredModel>,
    pub methods: Vec<MemberModel>,
    pub fields: Vec<MemberModel>,
    pub events: Vec<MemberModel>,
    pub enum_members: Vec<EnumMemberModel>,
    pub is_flags: bool,
    pub attributes: Vec<AttributeModel>,
    /// The attributes of registered properties, by the name of the property.
    pub property_attributes: Vec<(String, Vec<AttributeModel>)>,
    /// `notify_property_changed: T`.
    pub notify_property_changed: Option<RustType>,
}

impl TypeModel {
    /// A type with nothing but its identity.
    pub fn new(name: &str, kind: TypeKind, rust_path: RustType, module: &str) -> Self {
        Self {
            namespace: String::new(),
            explicit_namespace: None,
            name: name.to_string(),
            kind,
            object_model: false,
            class_markup: false,
            unregistered: false,
            rust_path,
            public_path: None,
            stated_path: None,
            module: module.to_string(),
            cfg: Vec::new(),
            handles: Vec::new(),
            this: None,
            base: None,
            interfaces: Vec::new(),
            generic: None,
            type_info: None,
            default_constructor: None,
            content_property: None,
            parse: None,
            parse_call: None,
            constructors: Vec::new(),
            properties: Vec::new(),
            static_properties: Vec::new(),
            indexers: Vec::new(),
            registered: Vec::new(),
            methods: Vec::new(),
            fields: Vec::new(),
            events: Vec::new(),
            enum_members: Vec::new(),
            is_flags: false,
            attributes: Vec::new(),
            property_attributes: Vec::new(),
            notify_property_changed: None,
        }
    }

    /// The namespace-qualified name (`FerroUI.Controls.Border`).
    pub fn full_name(&self) -> String {
        if self.namespace.is_empty() {
            self.name.clone()
        } else {
            format!("{}.{}", self.namespace, self.name)
        }
    }

    /// The registered property of the type with the name `name`.
    pub fn registered(&self, name: &str) -> Option<&RegisteredModel> {
        self.registered.iter().find(|registered| registered.name.as_deref() == Some(name))
    }

    /// The path generated code and other crates name the type by: the public path when
    /// the scanner found one, else the Rust type.
    pub fn named_path(&self) -> &str {
        self.public_path.as_deref().unwrap_or(&self.rust_path.text)
    }

    /// Calls `visit` with every type text of the type: its own, its handles, base and
    /// interfaces, and the types of its members and of the arguments of its attributes.
    pub fn visit_types_mut(&mut self, visit: &mut dyn FnMut(&mut RustType)) {
        fn value(value_: &mut AttributeValueModel, visit: &mut dyn FnMut(&mut RustType)) {
            match value_ {
                AttributeValueModel::Type(type_) => visit(type_),
                AttributeValueModel::Array(items) => {
                    for item in items {
                        value(item, visit);
                    }
                }
                _ => {}
            }
        }
        fn attributes(attributes_: &mut [AttributeModel], visit: &mut dyn FnMut(&mut RustType)) {
            for attribute in attributes_ {
                for argument in &mut attribute.arguments {
                    value(argument, visit);
                }
                for (_, argument) in &mut attribute.properties {
                    value(argument, visit);
                }
            }
        }
        fn parameters(parameters_: &mut [ParameterModel], visit: &mut dyn FnMut(&mut RustType)) {
            for parameter in parameters_ {
                visit(&mut parameter.type_);
                attributes(&mut parameter.attributes, visit);
            }
        }
        fn members(members_: &mut [MemberModel], visit: &mut dyn FnMut(&mut RustType)) {
            for member in members_ {
                parameters(&mut member.parameters, visit);
                if let Some(type_) = &mut member.return_type {
                    visit(type_);
                }
                attributes(&mut member.attributes, visit);
            }
        }
        fn properties(properties_: &mut [PropertyModel], visit: &mut dyn FnMut(&mut RustType)) {
            for property in properties_ {
                parameters(&mut property.parameters, visit);
                visit(&mut property.value_type);
                attributes(&mut property.attributes, visit);
            }
        }
        visit(&mut self.rust_path);
        for type_ in self.handles.iter_mut().chain(&mut self.interfaces) {
            visit(type_);
        }
        for type_ in [&mut self.this, &mut self.base, &mut self.type_info, &mut self.notify_property_changed].into_iter().flatten() {
            visit(type_);
        }
        if let Some(generic) = &mut self.generic {
            for type_ in &mut generic.arguments {
                visit(type_);
            }
        }
        members(&mut self.constructors, visit);
        members(&mut self.methods, visit);
        members(&mut self.fields, visit);
        members(&mut self.events, visit);
        properties(&mut self.properties, visit);
        properties(&mut self.static_properties, visit);
        properties(&mut self.indexers, visit);
        for registered in &mut self.registered {
            visit(&mut registered.value_type);
            for type_ in registered.owner.iter_mut().chain(registered.host.iter_mut()) {
                visit(type_);
            }
        }
        attributes(&mut self.attributes, visit);
        for (_, property_attributes) in &mut self.property_attributes {
            attributes(property_attributes, visit);
        }
    }

    /// Calls `visit` with every callable of the type that is a path: its constructors,
    /// `parse:`, the accessors of its properties, its members, and the sources of its
    /// registered properties.
    pub fn visit_callables_mut(&mut self, visit: &mut dyn FnMut(&mut CallableModel)) {
        for callable in self.default_constructor.iter_mut().chain(self.parse.iter_mut()) {
            visit(callable);
        }
        for member in self.constructors.iter_mut().chain(&mut self.methods).chain(&mut self.fields).chain(&mut self.events) {
            visit(&mut member.callable);
        }
        for property in self.properties.iter_mut().chain(&mut self.static_properties).chain(&mut self.indexers) {
            for accessor in property.getter.iter_mut().chain(property.setter.iter_mut()) {
                visit(&mut accessor.callable);
            }
        }
        for registered in &mut self.registered {
            if let Some(callable) = &mut registered.source {
                visit(callable);
            }
        }
    }

    fn to_json(&self) -> Json {
        let optional_type = |type_: &Option<RustType>| type_.as_ref().map(RustType::to_json);
        let members = Members::new()
            .text_or_empty("namespace", &self.namespace)
            .optional_text("explicit_namespace", &self.explicit_namespace)
            .text("name", &self.name)
            .text("kind", self.kind.name())
            .flag("object_model", self.object_model)
            .flag("class_markup", self.class_markup)
            .flag("unregistered", self.unregistered)
            .always("rust_path", self.rust_path.to_json())
            .optional_text("public_path", &self.public_path)
            .optional_text("stated_path", &self.stated_path)
            .text_or_empty("module", &self.module)
            .list("cfg", &self.cfg, |condition| Json::string(condition))
            .list("handles", &self.handles, RustType::to_json)
            .optional("this", optional_type(&self.this))
            .optional("base", optional_type(&self.base))
            .list("interfaces", &self.interfaces, RustType::to_json)
            .optional(
                "generic",
                self.generic.as_ref().map(|generic| {
                    Json::object(Members::new().text("definition", &generic.definition).list("arguments", &generic.arguments, RustType::to_json))
                }),
            )
            .optional("type_info", optional_type(&self.type_info))
            .optional("default_constructor", self.default_constructor.as_ref().map(CallableModel::to_json))
            .optional_text("content_property", &self.content_property)
            .optional("parse", self.parse.as_ref().map(CallableModel::to_json))
            .optional("parse_call", self.parse_call.as_ref().map(CallForm::to_json))
            .list("constructors", &self.constructors, MemberModel::to_json)
            .list("properties", &self.properties, PropertyModel::to_json)
            .list("static_properties", &self.static_properties, PropertyModel::to_json)
            .list("indexers", &self.indexers, PropertyModel::to_json)
            .list("registered", &self.registered, RegisteredModel::to_json)
            .list("methods", &self.methods, MemberModel::to_json)
            .list("fields", &self.fields, MemberModel::to_json)
            .list("events", &self.events, MemberModel::to_json)
            .list("enum_members", &self.enum_members, EnumMemberModel::to_json)
            .flag("is_flags", self.is_flags);
        Json::object(
            attributes_to_json(members, &self.attributes)
                .list("property_attributes", &self.property_attributes, |(name, attributes)| {
                    Json::Array(vec![Json::string(name), Json::Array(attributes.iter().map(AttributeModel::to_json).collect())])
                })
                .optional("notify_property_changed", optional_type(&self.notify_property_changed)),
        )
    }

    fn from_json(value: &Json) -> Result<Self, String> {
        let fields = Fields::of(value, "a type")?;
        Ok(Self {
            namespace: fields.text_or_empty("namespace")?,
            explicit_namespace: fields.optional_text("explicit_namespace")?,
            name: fields.text("name")?,
            kind: TypeKind::of(&fields.text("kind")?)?,
            object_model: fields.flag("object_model")?,
            class_markup: fields.flag("class_markup")?,
            unregistered: fields.flag("unregistered")?,
            rust_path: fields.optional("rust_path", RustType::from_json)?.ok_or_else(|| "\"rust_path\" is missing in a type".to_string())?,
            public_path: fields.optional_text("public_path")?,
            stated_path: fields.optional_text("stated_path")?,
            module: fields.text_or_empty("module")?,
            cfg: fields.list("cfg", |condition| text_of(condition, "a cfg condition"))?,
            handles: fields.list("handles", RustType::from_json)?,
            this: fields.optional("this", RustType::from_json)?,
            base: fields.optional("base", RustType::from_json)?,
            interfaces: fields.list("interfaces", RustType::from_json)?,
            generic: fields.optional("generic", |generic| {
                let generic = Fields::of(generic, "a generic definition")?;
                Ok(GenericModel { definition: generic.text("definition")?, arguments: generic.list("arguments", RustType::from_json)? })
            })?,
            type_info: fields.optional("type_info", RustType::from_json)?,
            default_constructor: fields.optional("default_constructor", CallableModel::from_json)?,
            content_property: fields.optional_text("content_property")?,
            parse: fields.optional("parse", CallableModel::from_json)?,
            parse_call: fields.optional("parse_call", CallForm::from_json)?,
            constructors: fields.list("constructors", MemberModel::from_json)?,
            properties: fields.list("properties", PropertyModel::from_json)?,
            static_properties: fields.list("static_properties", PropertyModel::from_json)?,
            indexers: fields.list("indexers", PropertyModel::from_json)?,
            registered: fields.list("registered", RegisteredModel::from_json)?,
            methods: fields.list("methods", MemberModel::from_json)?,
            fields: fields.list("fields", MemberModel::from_json)?,
            events: fields.list("events", MemberModel::from_json)?,
            enum_members: fields.list("enum_members", EnumMemberModel::from_json)?,
            is_flags: fields.flag("is_flags")?,
            attributes: fields.list("attributes", AttributeModel::from_json)?,
            property_attributes: fields.list("property_attributes", |pair| match pair {
                Json::Array(pair) if pair.len() == 2 => {
                    let attributes = match &pair[1] {
                        Json::Array(attributes) => attributes.iter().map(AttributeModel::from_json).collect::<Result<Vec<_>, _>>()?,
                        _ => return Err("the attributes of a registered property are not an array".to_string()),
                    };
                    Ok((text_of(&pair[0], "the name of a registered property")?, attributes))
                }
                _ => Err("the attributes of a registered property are not a pair".to_string()),
            })?,
            notify_property_changed: fields.optional("notify_property_changed", RustType::from_json)?,
        })
    }
}

/// `XmlnsDefinition` of the assembly: an XML namespace and a namespace it maps to.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct XmlnsDefinitionModel {
    pub xml_namespace: String,
    pub namespace: String,
}

/// `XmlnsPrefix` of the assembly: the prefix recommended for an XML namespace.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct XmlnsPrefixModel {
    pub xml_namespace: String,
    pub prefix: String,
}

/// A path another crate can name a type of the crate by.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExportModel {
    /// The public path (`::ferroui_controls::Border`).
    pub path: String,
    /// The path of the declaring module and the name (`::ferroui_controls::border::Border`);
    /// a path into another crate for a type the crate exports again.
    pub declared: String,
}

/// A type alias without parameters of a crate (`pub type PageList = FerroList<Ref<Page>>;`):
/// the two texts are one Rust type.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AliasModel {
    /// The path of the declaring module and the name (`::ferroui_controls::page::multi_page::PageList`).
    pub path: String,
    /// The type the alias stands for.
    pub target: RustType,
}

/// One more Rust type that holds a value of a type with markup metadata, registered by a
/// crate next to the `handles:` of the declaration
/// (`MarkupType::register_handle::<Handle>(<Type as MarkupTyped>::MARKUP)`): the wrapper a
/// property of another crate holds the value in.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HandleModel {
    pub handle: RustType,
    /// The Rust type of the type with the metadata, as its declaration has it
    /// ([`TypeModel::rust_path`]).
    pub type_: RustType,
}

/// A cast a crate registers between two Rust types (`ValueTypes::register_cast::<From, To>(..)`):
/// a value of the one is a value of the other. A collection that declares an
/// instantiation of the notifying list as its `base:` is that list only when the cast from
/// the collection to the list is registered.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CastModel {
    pub from: RustType,
    pub to: RustType,
}

/// A macro the crate exports (`#[macro_export]`) that declares through a declaration
/// macro: what a crate built on this one declares by invoking it
/// (`ferroui_controls::ferro_markup_list!(pub Names: String);`). The scan of that crate
/// expands the invocation with the rules, as it expands the macros of its own crate.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MacroModel {
    pub name: String,
    /// The rules, each the text of the tokens of its matcher and of its transcriber, with
    /// `$crate` written as it is.
    pub rules: Vec<(String, String)>,
}

/// A registration of Rust types with the untyped value conversions that a function of a
/// crate makes with its types stated (`ValueTypes::register_nullable::<T>()`): what
/// decides, next to the casts and the declarations, whether a value of one Rust type is a
/// value of another, and which types are nullable forms.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ValueTypeModel {
    /// The registration function, without `register_`: `nullable` (`Option<T>` is the
    /// nullable form of `T`), `reference` (`T` is a shared object held as `Rc<T>` and
    /// `Option<Rc<T>>`), `object` (`Ref<T>` is the handle of the class `T`), `element_ref`
    /// (`ElementRef<T>` of the class `T`), `interface` (the handles of the class `T`
    /// convert to the contract handle `I`), `upcast` (the handles of the class `T` to the
    /// ones of its base).
    pub registration: String,
    /// The type arguments of the call, in order.
    pub types: Vec<RustType>,
}

/// The registration functions of the untyped value conversions the scanner reads
/// ([`ValueTypeModel`], and `cast` for [`CastModel`]), with the number of type arguments.
pub const VALUE_REGISTRATIONS: &[(&str, usize)] =
    &[("nullable", 1), ("reference", 1), ("object", 1), ("element_ref", 1), ("interface", 2), ("upcast", 2), ("cast", 2)];

/// A public function of an inherent `impl` block of a type of a crate, as far as the choice
/// of a call form reads it (9.5.3, form B).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct FunctionModel {
    pub name: String,
    /// Whether the function takes `self`.
    pub receiver: bool,
    /// The number of parameters after `self`.
    pub parameters: usize,
    /// The declaration macro that writes the function (`ferro_routed_event`), when one does.
    pub declared_by: Option<String>,
}

/// The public functions of the inherent `impl` blocks of one type of a crate
/// (`FunctionsModel`): what a callable of a crate built on this one is looked up in, so
/// that a member declared there with a function of this crate is called by its path.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct FunctionsModel {
    /// The Rust path of the type, by its declaring module.
    pub owner: String,
    /// The shortest path another crate names the type by.
    pub public_path: String,
    /// The functions, in the order of the scan.
    pub functions: Vec<FunctionModel>,
}

impl FunctionsModel {
    fn to_json(&self) -> Json {
        let function = |function: &FunctionModel| {
            let mut parts = vec![Json::string(&function.name), Json::Integer(function.parameters as i64)];
            if function.receiver || function.declared_by.is_some() {
                parts.push(Json::Bool(function.receiver));
            }
            if let Some(declared_by) = &function.declared_by {
                parts.push(Json::string(declared_by));
            }
            Json::Array(parts)
        };
        Json::Array(vec![Json::string(&self.owner), Json::string(&self.public_path), Json::Array(self.functions.iter().map(function).collect())])
    }

    fn from_json(value: &Json) -> Result<Self, String> {
        let function = |value: &Json| match value {
            Json::Array(parts) if (2..=4).contains(&parts.len()) => Ok(FunctionModel {
                name: text_of(&parts[0], "the name of a function")?,
                parameters: match &parts[1] {
                    Json::Integer(count) if *count >= 0 => *count as usize,
                    _ => return Err("the number of parameters of a function is not a number".to_string()),
                },
                receiver: match parts.get(2) {
                    None => false,
                    Some(Json::Bool(receiver)) => *receiver,
                    Some(_) => return Err("the receiver of a function is not a flag".to_string()),
                },
                declared_by: parts.get(3).map(|declared_by| text_of(declared_by, "the macro of a function")).transpose()?,
            }),
            _ => Err("a function is not a list of two to four parts".to_string()),
        };
        match value {
            Json::Array(parts) if parts.len() == 3 => Ok(Self {
                owner: text_of(&parts[0], "the owner of functions")?,
                public_path: text_of(&parts[1], "the public path of the owner of functions")?,
                functions: match &parts[2] {
                    Json::Array(functions) => functions.iter().map(function).collect::<Result<_, _>>()?,
                    _ => return Err("the functions of a type are not a list".to_string()),
                },
            }),
            _ => Err("the functions of a type are not a list of three parts".to_string()),
        }
    }
}

fn pair_to_json(first: &str, second: &str) -> Json {
    Json::Array(vec![Json::string(first), Json::string(second)])
}

fn pair_from_json(value: &Json, what: &str) -> Result<(String, String), String> {
    match value {
        Json::Array(pair) if pair.len() == 2 => Ok((text_of(&pair[0], what)?, text_of(&pair[1], what)?)),
        _ => Err(format!("{what} is not a pair")),
    }
}

/// What a crate states about itself for markup (`AssemblyModel`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AssemblyModel {
    /// The assembly name.
    pub name: String,
    /// The name of the crate, as Rust paths spell it.
    pub crate_name: String,
    pub xmlns_definitions: Vec<XmlnsDefinitionModel>,
    pub xmlns_prefixes: Vec<XmlnsPrefixModel>,
    /// The metadata of the assembly (`FerroXamlCreateSourceInfo`), name and value.
    pub metadata: Vec<(String, String)>,
    /// The namespace table: a module path and the dotted namespace of the types declared
    /// below it. A type belongs to the namespace of the longest module path that is a
    /// prefix of the path of its module.
    pub namespaces: Vec<(String, String)>,
    pub types: Vec<TypeModel>,
    /// The export table of the crate: every path another crate can name a type of the
    /// crate by, with the path of the module that declares it (9.5.2, step 1). What a glob
    /// import of a module of the crate brings into another crate, and what makes a path
    /// into the crate canonical, is read from it.
    pub exports: Vec<ExportModel>,
    /// The type aliases of the crate whose type the scanner resolved.
    pub aliases: Vec<AliasModel>,
    /// The handles the crate registers for types with markup metadata, of this crate or
    /// of the crates it is built on.
    pub handles: Vec<HandleModel>,
    /// The casts the crate registers between Rust types, in the order of the scan.
    pub casts: Vec<CastModel>,
    /// The other registrations with the untyped value conversions the functions of the
    /// crate make with their types stated.
    pub value_types: Vec<ValueTypeModel>,
    /// The registrations the text of the crate has and the model does not: for each
    /// registration function ([`VALUE_REGISTRATIONS`]) with such calls, their number (a call
    /// that leaves a type to inference, a call in the body of a macro, in a macro
    /// invocation or in test code). A crate with such calls registers more than its model
    /// states, so nothing follows from a registration that is not in the model.
    pub unread_value_types: Vec<(String, i64)>,
    /// The macros the crate exports that declare through a declaration macro.
    pub macros: Vec<MacroModel>,
    /// The public functions of the inherent `impl` blocks of the types of the crate that
    /// another crate can name, by type, in the order of the scan.
    pub functions: Vec<FunctionsModel>,
    /// The compiled documents, in the order of the compilation.
    pub documents: Vec<DocumentModel>,
    /// The `.xamlmeta` files of the crates this crate is built on, relative to the
    /// directory of this file or absolute.
    pub dependencies: Vec<String>,
}

impl AssemblyModel {
    /// A model without types and documents.
    pub fn new(name: &str, crate_name: &str) -> Self {
        Self {
            name: name.to_string(),
            crate_name: crate_name.to_string(),
            xmlns_definitions: Vec::new(),
            xmlns_prefixes: Vec::new(),
            metadata: Vec::new(),
            namespaces: Vec::new(),
            types: Vec::new(),
            exports: Vec::new(),
            aliases: Vec::new(),
            handles: Vec::new(),
            casts: Vec::new(),
            value_types: Vec::new(),
            unread_value_types: Vec::new(),
            macros: Vec::new(),
            functions: Vec::new(),
            documents: Vec::new(),
            dependencies: Vec::new(),
        }
    }

    /// The model of a file of format 1: the documents of a crate, no types.
    pub fn from_metadata(metadata: XamlMetadata) -> Self {
        let mut model = Self::new(&metadata.name, &metadata.crate_name);
        model.documents = metadata.documents;
        model.dependencies = metadata.dependencies;
        model
    }

    /// The part of the model a file of format 1 holds: what the compiler reads today.
    pub fn metadata(&self) -> XamlMetadata {
        XamlMetadata {
            name: self.name.clone(),
            crate_name: self.crate_name.clone(),
            documents: self.documents.clone(),
            dependencies: self.dependencies.clone(),
        }
    }

    /// The namespace the table gives the module `module`; empty when it has none.
    pub fn namespace_of_module(&self, module: &str) -> &str {
        let mut best: Option<(&str, &str)> = None;
        for (prefix, namespace) in &self.namespaces {
            let matches = match module.strip_prefix(prefix.as_str()) {
                Some(rest) => rest.is_empty() || rest.starts_with("::"),
                None => false,
            };
            if matches && best.is_none_or(|(known, _)| prefix.len() > known.len()) {
                best = Some((prefix.as_str(), namespace.as_str()));
            }
        }
        best.map_or("", |(_, namespace)| namespace)
    }

    /// The type with the namespace-qualified name `full_name`.
    pub fn find_type(&self, full_name: &str) -> Option<&TypeModel> {
        self.types.iter().find(|type_| type_.full_name() == full_name)
    }

    /// The type with the Rust type text `rust_path` (its path, for a type the crate declares).
    pub fn find_rust_type(&self, rust_path: &str) -> Option<&TypeModel> {
        self.types.iter().find(|type_| type_.rust_path.text == rust_path)
    }

    /// Calls `visit` with every type text of the model that is not of one of its types: the
    /// types of its aliases, its registered handles and its registered casts.
    pub fn visit_types_mut(&mut self, visit: &mut dyn FnMut(&mut RustType)) {
        for alias in &mut self.aliases {
            visit(&mut alias.target);
        }
        for handle in &mut self.handles {
            visit(&mut handle.handle);
            visit(&mut handle.type_);
        }
        for cast in &mut self.casts {
            visit(&mut cast.from);
            visit(&mut cast.to);
        }
        for registration in &mut self.value_types {
            for type_ in &mut registration.types {
                visit(type_);
            }
        }
    }

    /// The model as the text of a `.xamlmeta` file of format 2.
    pub fn to_json(&self) -> String {
        let document = |document: &DocumentModel| {
            let optional = |text: &Option<String>| text.as_deref().map_or(Json::Null, Json::string);
            Json::Object(vec![
                ("uri".to_string(), Json::string(&document.uri)),
                ("root_type".to_string(), Json::string(&document.root_type)),
                ("class_rust_path".to_string(), optional(&document.class_rust_path)),
                ("build_path".to_string(), optional(&document.build_path)),
                ("populate_path".to_string(), optional(&document.populate_path)),
                ("public".to_string(), Json::Bool(document.public)),
            ])
        };
        let members = Members::new()
            .always("format", Json::Integer(FORMAT))
            .text("name", &self.name)
            .text("crate_name", &self.crate_name)
            .always("documents", Json::Array(self.documents.iter().map(document).collect()))
            .always("dependencies", Json::Array(self.dependencies.iter().map(|path| Json::string(path)).collect()))
            .list("xmlns_definitions", &self.xmlns_definitions, |definition| pair_to_json(&definition.xml_namespace, &definition.namespace))
            .list("xmlns_prefixes", &self.xmlns_prefixes, |prefix| pair_to_json(&prefix.xml_namespace, &prefix.prefix))
            .list("metadata", &self.metadata, |(name, value)| pair_to_json(name, value))
            .list("namespaces", &self.namespaces, |(module, namespace)| pair_to_json(module, namespace))
            .list("types", &self.types, TypeModel::to_json)
            .list("exports", &self.exports, |export| pair_to_json(&export.path, &export.declared))
            .list("aliases", &self.aliases, |alias| Json::Array(vec![Json::string(&alias.path), alias.target.to_json()]))
            .list("handles", &self.handles, |handle| Json::Array(vec![handle.handle.to_json(), handle.type_.to_json()]))
            .list("casts", &self.casts, |cast| Json::Array(vec![cast.from.to_json(), cast.to.to_json()]))
            .list("value_types", &self.value_types, |registration| {
                let mut parts = vec![Json::string(&registration.registration)];
                parts.extend(registration.types.iter().map(RustType::to_json));
                Json::Array(parts)
            })
            .list("unread_value_types", &self.unread_value_types, |(registration, count)| Json::Array(vec![Json::string(registration), Json::Integer(*count)]))
            .list("macros", &self.macros, |exported| {
                let rules = exported.rules.iter().map(|(matcher, transcriber)| pair_to_json(matcher, transcriber)).collect();
                Json::Array(vec![Json::string(&exported.name), Json::Array(rules)])
            })
            .list("functions", &self.functions, FunctionsModel::to_json);
        Json::object(members).to_text()
    }

    /// Reads the model from the text of a `.xamlmeta` file of format 1 or 2.
    pub fn parse(text: &str) -> Result<Self, String> {
        let value = Json::parse(text)?;
        let fields = Fields::of(&value, "the file")?;
        let format = fields.optional_integer("format")?.unwrap_or(1);
        if !(1..=FORMAT).contains(&format) {
            return Err(format!("the file has the format {format}; this reader reads the formats 1 to {FORMAT}"));
        }
        let documents = fields.list("documents", |document| {
            let document = Fields::of(document, "a document")?;
            Ok(DocumentModel {
                uri: document.text("uri")?,
                root_type: document.text("root_type")?,
                class_rust_path: document.optional_text("class_rust_path")?,
                build_path: document.optional_text("build_path")?,
                populate_path: document.optional_text("populate_path")?,
                public: document.flag("public")?,
            })
        })?;
        Ok(Self {
            name: fields.text("name")?,
            crate_name: fields.text("crate_name")?,
            xmlns_definitions: fields.list("xmlns_definitions", |pair| {
                pair_from_json(pair, "an xmlns definition").map(|(xml_namespace, namespace)| XmlnsDefinitionModel { xml_namespace, namespace })
            })?,
            xmlns_prefixes: fields.list("xmlns_prefixes", |pair| {
                pair_from_json(pair, "an xmlns prefix").map(|(xml_namespace, prefix)| XmlnsPrefixModel { xml_namespace, prefix })
            })?,
            metadata: fields.list("metadata", |pair| pair_from_json(pair, "an entry of the metadata"))?,
            namespaces: fields.list("namespaces", |pair| pair_from_json(pair, "an entry of the namespace table"))?,
            types: fields.list("types", TypeModel::from_json)?,
            exports: fields.list("exports", |pair| pair_from_json(pair, "an entry of the export table").map(|(path, declared)| ExportModel { path, declared }))?,
            aliases: fields.list("aliases", |pair| match pair {
                Json::Array(pair) if pair.len() == 2 => Ok(AliasModel { path: text_of(&pair[0], "the path of an alias")?, target: RustType::from_json(&pair[1])? }),
                _ => Err("a type alias is not a pair".to_string()),
            })?,
            handles: fields.list("handles", |pair| match pair {
                Json::Array(pair) if pair.len() == 2 => Ok(HandleModel { handle: RustType::from_json(&pair[0])?, type_: RustType::from_json(&pair[1])? }),
                _ => Err("a registered handle is not a pair".to_string()),
            })?,
            casts: fields.list("casts", |pair| match pair {
                Json::Array(pair) if pair.len() == 2 => Ok(CastModel { from: RustType::from_json(&pair[0])?, to: RustType::from_json(&pair[1])? }),
                _ => Err("a registered cast is not a pair".to_string()),
            })?,
            value_types: fields.list("value_types", |parts| match parts {
                Json::Array(parts) if !parts.is_empty() => Ok(ValueTypeModel {
                    registration: text_of(&parts[0], "the name of a registration")?,
                    types: parts[1..].iter().map(RustType::from_json).collect::<Result<_, _>>()?,
                }),
                _ => Err("a registration of value types is not a list".to_string()),
            })?,
            unread_value_types: fields.list("unread_value_types", |pair| match pair {
                Json::Array(pair) if pair.len() == 2 => match &pair[1] {
                    Json::Integer(count) => Ok((text_of(&pair[0], "the name of a registration")?, *count)),
                    _ => Err("the number of registrations that are not read is not an integer".to_string()),
                },
                _ => Err("an entry of the registrations that are not read is not a pair".to_string()),
            })?,
            macros: fields.list("macros", |exported| match exported {
                Json::Array(parts) if parts.len() == 2 => match &parts[1] {
                    Json::Array(rules) => Ok(MacroModel {
                        name: text_of(&parts[0], "the name of a macro")?,
                        rules: rules.iter().map(|rule| pair_from_json(rule, "a rule of a macro")).collect::<Result<_, _>>()?,
                    }),
                    _ => Err("the rules of a macro are not a list".to_string()),
                },
                _ => Err("an exported macro is not a pair".to_string()),
            })?,
            functions: fields.list("functions", FunctionsModel::from_json)?,
            documents,
            dependencies: fields.list("dependencies", |path| text_of(path, "a dependency"))?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    fn callable(path: &str, resolved: Option<&str>) -> CallableModel {
        CallableModel { path: Some(path.to_string()), resolved: resolved.map(str::to_string), dereferenced: None }
    }

    /// A model with every member of every part set to something that is not its default.
    fn full_model() -> AssemblyModel {
        let attribute = AttributeModel {
            name: "TemplatePart".to_string(),
            arguments: vec![
                AttributeValueModel::Str("PART_\"Bar\"".to_string()),
                AttributeValueModel::Type(RustType::resolved("::ferroui_base::Ref<::fixture::Border>")),
                AttributeValueModel::Array(vec![AttributeValueModel::Int(-1), AttributeValueModel::Null, AttributeValueModel::Float("1.5".to_string())]),
            ],
            properties: vec![("IsRequired".to_string(), AttributeValueModel::Bool(true))],
        };
        let unresolved = RustType { text: "Option<Mystery>".to_string(), unresolved: vec!["Mystery".to_string()] };
        let mut type_ = TypeModel::new("Panel", TypeKind::Class, RustType::resolved("::fixture::panel::Panel"), "fixture::panel");
        type_.namespace = "Fixture.Controls".to_string();
        type_.explicit_namespace = Some("Fixture.Controls".to_string());
        type_.object_model = true;
        type_.unregistered = true;
        type_.public_path = Some("::fixture::Panel".to_string());
        type_.cfg = vec!["feature = \"panels\"".to_string()];
        type_.handles = vec![RustType::resolved("::fixture::panel::Panel"), unresolved.clone()];
        type_.this = Some(RustType::resolved("::std::rc::Rc<::fixture::panel::Panel>"));
        type_.base = Some(RustType::resolved("::fixture::control::Control"));
        type_.interfaces = vec![RustType::resolved("::std::rc::Rc<dyn ::fixture::IPanel>")];
        type_.generic = Some(GenericModel { definition: "FerroList`1".to_string(), arguments: vec![RustType::resolved("f64")] });
        type_.type_info = Some(RustType::resolved("::fixture::panel::Panel"));
        type_.default_constructor = Some(callable("Panel::new", Some("::fixture::panel::Panel::new")));
        type_.content_property = Some("Children".to_string());
        type_.parse = Some(CallableModel::default());
        type_.parse_call = Some(CallForm::CratePath("::fixture::Panel::parse".to_string()));
        type_.constructors = vec![MemberModel {
            parameters: vec![
                ParameterModel { name: Some("name".to_string()), type_: RustType::resolved("String"), attributes: vec![attribute.clone()] },
                ParameterModel { name: None, type_: unresolved.clone(), attributes: Vec::new() },
            ],
            is_static: true,
            fallible: true,
            callable: callable("Panel::with_name", None),
            typed_function: Some("__markup_new_0".to_string()),
            call: Some(CallForm::Path("::fixture::Panel::with_name".to_string())),
            ..MemberModel::default()
        }];
        let accessor = AccessorModel {
            fallible: true,
            callable: CallableModel::default(),
            typed_function: Some("__markup_get_Children".to_string()),
            call: Some(CallForm::Invoker),
        };
        type_.properties = vec![PropertyModel {
            name: "Children".to_string(),
            parameters: Vec::new(),
            value_type: RustType::resolved("::fixture::Controls"),
            getter: Some(accessor.clone()),
            setter: None,
            attributes: vec![attribute.clone()],
        }];
        type_.static_properties = vec![PropertyModel {
            name: "Default".to_string(),
            parameters: Vec::new(),
            value_type: RustType::resolved("i32"),
            getter: None,
            setter: Some(AccessorModel { call: Some(CallForm::Structural), ..AccessorModel::default() }),
            attributes: Vec::new(),
        }];
        type_.indexers = vec![PropertyModel {
            name: String::new(),
            parameters: vec![ParameterModel { name: None, type_: RustType::resolved("i32"), attributes: Vec::new() }],
            value_type: RustType::resolved("String"),
            getter: Some(accessor),
            setter: None,
            attributes: Vec::new(),
        }];
        type_.registered = vec![
            RegisteredModel {
                name: Some("Row".to_string()),
                kind: RegisteredKind::Attached,
                value_type: RustType::resolved("i32"),
                owner: Some(RustType::resolved("::fixture::panel::Panel")),
                host: Some(RustType::resolved("::fixture::control::Control")),
                accessor: "row_property".to_string(),
                function_of: Some("::fixture::grid::Grid".to_string()),
                visibility: "pub".to_string(),
                registration: RegistrationModel::Declared,
                source: None,
                assign_binding: true,
                inherits: true,
                read_only: false,
                added_owners: vec!["::fixture::grid::Grid".to_string()],
            },
            RegisteredModel {
                name: None,
                kind: RegisteredKind::Direct,
                value_type: unresolved.clone(),
                owner: None,
                host: None,
                accessor: "text_property".to_string(),
                function_of: None,
                visibility: String::new(),
                registration: RegistrationModel::AddedOwner,
                source: Some(callable("TextBlock::text_property", None)),
                assign_binding: false,
                inherits: false,
                read_only: true,
                added_owners: Vec::new(),
            },
        ];
        type_.methods = vec![MemberModel {
            name: "Add".to_string(),
            parameters: vec![ParameterModel { name: None, type_: RustType::resolved("::fixture::Control"), attributes: Vec::new() }],
            return_type: Some(RustType::resolved("bool")),
            attributes: vec![attribute.clone()],
            callable: CallableModel::default(),
            typed_function: Some("__markup_Add_0".to_string()),
            ..MemberModel::default()
        }];
        type_.fields = vec![MemberModel {
            name: "ClickEvent".to_string(),
            return_type: Some(unresolved.clone()),
            is_static: true,
            callable: CallableModel { path: None, resolved: None, dereferenced: Some("::fixture::panel::Panel::click_event".to_string()) },
            ..MemberModel::default()
        }];
        type_.events = vec![MemberModel { name: "Closed".to_string(), fallible: true, ..MemberModel::default() }];
        type_.enum_members = vec![
            EnumMemberModel { name: "Self".to_string(), rust_variant: Some("Self_".to_string()), rust_value: None, value: Some(2) },
            EnumMemberModel { name: "Direct".to_string(), rust_variant: None, rust_value: Some("Routing::DIRECT".to_string()), value: None },
        ];
        type_.is_flags = true;
        type_.attributes = vec![attribute.clone()];
        type_.property_attributes = vec![("Row".to_string(), vec![attribute])];
        type_.notify_property_changed = Some(unresolved);

        let mut model = AssemblyModel::new("\u{30a2}\u{30bb}\u{30f3}\u{30d6}\u{30ea}", "fixture");
        model.xmlns_definitions = vec![XmlnsDefinitionModel { xml_namespace: "https://example.org/fixture".to_string(), namespace: "Fixture".to_string() }];
        model.xmlns_prefixes = vec![XmlnsPrefixModel { xml_namespace: "https://example.org/fixture".to_string(), prefix: "f".to_string() }];
        model.metadata = vec![("FerroXamlCreateSourceInfo".to_string(), "true".to_string())];
        model.namespaces = vec![("fixture".to_string(), "Fixture".to_string()), ("fixture::panel".to_string(), "Fixture.Controls".to_string())];
        model.types = vec![type_, TypeModel::new("Dock", TypeKind::Enum, RustType::resolved("::fixture::Dock"), "fixture")];
        model.exports = vec![ExportModel { path: "::fixture::Panel".to_string(), declared: "::fixture::panel::Panel".to_string() }];
        model.aliases = vec![AliasModel { path: "::fixture::panel::Panels".to_string(), target: RustType::resolved("Vec<::fixture::panel::Panel>") }];
        model.handles = vec![HandleModel {
            handle: RustType { text: "Option<Wrapper>".to_string(), unresolved: vec!["Wrapper".to_string()] },
            type_: RustType::resolved("dyn ::fixture::IPanel"),
        }];
        model.casts = vec![CastModel { from: RustType::resolved("::fixture::panel::Panels"), to: RustType::resolved("Vec<::fixture::panel::Panel>") }];
        model.functions = vec![FunctionsModel {
            owner: "::fixture::panel::Panel".to_string(),
            public_path: "::fixture::Panel".to_string(),
            functions: vec![
                FunctionModel { name: "new".to_string(), ..FunctionModel::default() },
                FunctionModel { name: "add".to_string(), receiver: true, parameters: 1, declared_by: None },
                FunctionModel { name: "click_event".to_string(), receiver: false, parameters: 0, declared_by: Some("ferro_routed_event".to_string()) },
            ],
        }];
        model.documents = vec![DocumentModel {
            uri: "ferres://Fixture/Main.xaml".to_string(),
            root_type: "Fixture.Controls.Panel".to_string(),
            class_rust_path: Some("::fixture::Main".to_string()),
            build_path: None,
            populate_path: Some("::fixture::compiled_xaml::populate".to_string()),
            public: true,
        }];
        model.dependencies = vec!["../Other/compiled_xaml.xamlmeta".to_string()];
        model
    }

    /// Not from upstream: the model is read back as it was written, whatever is set in it,
    /// and the text is stable.
    #[test]
    fn model_round_trips_through_its_text() {
        let model = full_model();
        let text = model.to_json();
        assert_eq!(AssemblyModel::parse(&text), Ok(model), "{text}");
        assert_eq!(AssemblyModel::parse(&text).map(|read| read.to_json()), Ok(text.clone()));
        let empty = AssemblyModel::new("A", "a");
        assert_eq!(AssemblyModel::parse(&empty.to_json()), Ok(empty.clone()));
        assert_eq!(empty.to_json(), "{\n  \"format\": 2,\n  \"name\": \"A\",\n  \"crate_name\": \"a\",\n  \"documents\": [],\n  \"dependencies\": []\n}\n");
    }

    /// Not from upstream: the reader of format 1 (the compiler's) reads a file of format 2
    /// as the documents of the crate, and a file of format 1 is a model without types.
    #[test]
    fn both_formats_are_read_by_both_readers() {
        let model = full_model();
        assert_eq!(XamlMetadata::parse(&model.to_json()), Ok(model.metadata()));
        let first = model.metadata().to_json();
        assert!(!first.contains("\"format\""), "{first}");
        let read = AssemblyModel::parse(&first).expect("a file of format 1");
        assert_eq!(read, AssemblyModel::from_metadata(model.metadata()));
        assert!(read.types.is_empty() && read.namespaces.is_empty());
        assert_eq!(read.documents, model.documents);
        let newer = model.to_json().replacen("\"format\": 2", "\"format\": 3", 1);
        assert_eq!(AssemblyModel::parse(&newer), Err("the file has the format 3; this reader reads the formats 1 to 2".to_string()));
    }

    /// Not from upstream: the checked-in `.xamlmeta` of the themes (format 1, written by the
    /// emitter) is read, and written back in format 2 it gives the compiler the same
    /// documents.
    #[test]
    fn checked_in_metadata_of_the_themes_is_read() {
        let source = Path::new(env!("CARGO_MANIFEST_DIR")).parent().expect("the directory of the crates");
        for (directory, name, crate_name, class) in [
            ("FerroUI.Themes.Simple", "FerroUI.Themes.Simple", "ferroui_themes_simple", "::ferroui_themes_simple::SimpleTheme"),
            ("FerroUI.Themes.Fluent", "FerroUI.Themes.Fluent", "ferroui_themes_fluent", "::ferroui_themes_fluent::FluentTheme"),
        ] {
            let path = source.join(directory).join("compiled_xaml.xamlmeta");
            let text = std::fs::read_to_string(&path).unwrap_or_else(|error| panic!("{}: {error}", path.display()));
            let model = AssemblyModel::parse(&text).unwrap_or_else(|error| panic!("{}: {error}", path.display()));
            let metadata = XamlMetadata::parse(&text).unwrap_or_else(|error| panic!("{}: {error}", path.display()));
            assert_eq!(model.name, name, "{}", path.display());
            assert_eq!(model.crate_name, crate_name, "{}", path.display());
            assert!(model.types.is_empty(), "{}: a file of format 1 has no types", path.display());
            assert!(!model.documents.is_empty(), "{}: no documents", path.display());
            assert_eq!(model.metadata(), metadata, "{}: the two readers disagree", path.display());
            assert!(
                model.documents.iter().any(|document| document.class_rust_path.as_deref() == Some(class)),
                "{}: no document of the class {class}: {:?}",
                path.display(),
                model.documents
            );
            assert_eq!(XamlMetadata::parse(&model.to_json()), Ok(metadata), "{}", path.display());
        }
    }

    /// Not from upstream: the namespace of a module is the one of the longest prefix of its
    /// path, as `TypeInfo::namespace` finds it.
    #[test]
    fn namespace_is_the_one_of_the_longest_module_prefix() {
        let mut model = AssemblyModel::new("A", "a");
        model.namespaces = vec![
            ("a".to_string(), "A".to_string()),
            ("a::media".to_string(), "A.Media".to_string()),
            ("a::media::imaging".to_string(), "A.Media.Imaging".to_string()),
        ];
        assert_eq!(model.namespace_of_module("a"), "A");
        assert_eq!(model.namespace_of_module("a::border"), "A");
        assert_eq!(model.namespace_of_module("a::media::brush"), "A.Media");
        assert_eq!(model.namespace_of_module("a::media::imaging"), "A.Media.Imaging");
        assert_eq!(model.namespace_of_module("a::mediator"), "A");
        assert_eq!(model.namespace_of_module("b::media"), "");
    }
}
