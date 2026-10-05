//! Syntax tree of a MicroCom IDL file.

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Idl {
    /// `@name value` lines in file order (`cpp-preamble` holds the text
    /// between the `@@` markers).
    pub directives: Vec<Directive>,
    pub enums: Vec<Enum>,
    pub structs: Vec<Struct>,
    pub interfaces: Vec<Interface>,
}

impl Idl {
    pub fn directive(&self, name: &str) -> Option<&str> {
        self.directives.iter().find(|d| d.name == name).map(|d| d.value.as_str())
    }

    /// All values of a directive that may be repeated (e.g. `clr-map`).
    pub fn directive_values<'a>(&'a self, name: &'a str) -> impl Iterator<Item = &'a str> {
        self.directives.iter().filter(move |d| d.name == name).map(|d| d.value.as_str())
    }

    pub fn find_interface(&self, name: &str) -> Option<&Interface> {
        self.interfaces.iter().find(|i| i.name == name)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Directive {
    pub name: String,
    pub value: String,
}

/// `[name]` or `[name(value)]`.
#[derive(Debug, Clone, PartialEq)]
pub struct Attribute {
    pub name: String,
    pub value: Option<String>,
}

pub fn has_attr(attrs: &[Attribute], name: &str) -> bool {
    attrs.iter().any(|a| a.name == name)
}

pub fn attr_value<'a>(attrs: &'a [Attribute], name: &str) -> Option<&'a str> {
    attrs.iter().find(|a| a.name == name).and_then(|a| a.value.as_deref())
}

#[derive(Debug, Clone, PartialEq)]
pub struct Enum {
    pub attributes: Vec<Attribute>,
    pub name: String,
    pub members: Vec<EnumMember>,
}

impl Enum {
    pub fn is_class_enum(&self) -> bool {
        has_attr(&self.attributes, "class-enum")
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct EnumMember {
    pub name: String,
    /// Source text of the value expression, if one was given.
    pub value: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Struct {
    pub attributes: Vec<Attribute>,
    pub name: String,
    pub fields: Vec<Field>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Field {
    pub ty: TypeRef,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TypeRef {
    /// Type name as written (`int`, `FrnSize`, `IFrnWindow`, ...).
    pub name: String,
    /// Number of `*`.
    pub pointers: u32,
    /// Trailing `&`.
    pub reference: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Interface {
    pub attributes: Vec<Attribute>,
    pub name: String,
    /// `None` when no base is written (the generators then use `IUnknown`).
    pub base: Option<String>,
    pub methods: Vec<Method>,
}

impl Interface {
    pub fn uuid(&self) -> Option<&str> {
        attr_value(&self.attributes, "uuid")
    }

    pub fn cpp_virtual_inherits(&self) -> bool {
        has_attr(&self.attributes, "cpp-virtual-inherits")
    }

    pub fn base_name(&self) -> &str {
        self.base.as_deref().unwrap_or("IUnknown")
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Method {
    /// Attributes written before the return type (e.g. `[intptr]`).
    pub attributes: Vec<Attribute>,
    pub return_type: TypeRef,
    pub name: String,
    pub params: Vec<Param>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Param {
    /// `[const]`, `[intptr]`, ...
    pub attributes: Vec<Attribute>,
    pub ty: TypeRef,
    pub name: String,
}

impl Param {
    pub fn is_const(&self) -> bool {
        has_attr(&self.attributes, "const")
    }
}
