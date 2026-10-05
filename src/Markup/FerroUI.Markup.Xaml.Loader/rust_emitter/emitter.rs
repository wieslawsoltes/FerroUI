//! Walks the transformed AST of a document and writes the Rust statements
//! that build it: one emit per evaluation of the interpreter
//! (`runtime/interpreter/evaluators.rs`, `runtime/framework/nodes.rs`), in
//! the same order. Anything outside the supported set is reported as
//! [`UnsupportedNode`]; nothing approximate is written.

use std::any::TypeId;
use std::collections::HashMap;
use std::fmt;
use std::rc::Rc;

use ferroui_base::metadata::{property_accessors, MarkupType};
use ferroui_base::{BoxedValue, FerroProperty, StyledElement, TypeInfo};
use xamlx::ast::XamlAstExtensions as _;
use xamlx::ast::XamlAstNodeExtensions as _;
use xamlx::ast::{
    visit_node, IXamlAstNode, IXamlAstValueNode, IXamlAstVisitor, XamlAstCompilerLocalNode,
    XamlAstImperativeValueManipulation, XamlAstLocalInitializationNodeEmitter, XamlAstManipulationImperativeNode,
    XamlAstNewClrObjectNode, XamlAstTextNode, XamlConstantNode, XamlDirectCallPropertySetter, XamlManipulationGroupNode, XamlNullExtensionNode,
    XamlNoReturnMethodCallNode, XamlObjectInitializationNode, XamlPropertyAssignmentNode, XamlStaticExtensionNode,
    XamlStaticMember, XamlStaticOrTargetedReturnMethodCallNode, XamlWrappedMethod,
    XamlValueNodeWithBeginInit,
};
use xamlx::exceptions::{XamlError, XamlResult};
use xamlx::transform::transformers::AdderSetter;
use xamlx::transform::TransformerConfiguration;
use xamlx::type_system::{IXamlConstructor, IXamlType, XamlValue};

use crate::compiler_extensions::ast_nodes::{
    FerroXamlIlFerroListConstantAstNode, FerroXamlIlGridLengthAstNode, FerroXamlIlVectorLikeConstantAstNode,
};
use crate::compiler_extensions::transformers::{FerroNameScopeRegistrationXamlIlNode, HandleRootObjectScopeNode};
use crate::compiler_extensions::XamlIlFerroPropertyHelper;
use crate::runtime::interpreter::{numeric_constant, single_setter};
use crate::runtime::type_system::{
    DeclaredMember, RuntimeConstructor, RuntimeField, RuntimeFieldValue, RuntimeInvoker, RuntimeMethod, RuntimeType,
};

use super::source::rust_string_literal;

/// Why a document is not eligible for emission: the first node (or member)
/// the emitter has no exact Rust for.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UnsupportedNode {
    /// The type name of the AST node.
    pub node_type_name: String,
    /// What about the node is not supported.
    pub reason: String,
    pub line: i32,
    pub position: i32,
}

impl fmt::Display for UnsupportedNode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {} (line {} position {})", self.node_type_name, self.reason, self.line, self.position)
    }
}

impl std::error::Error for UnsupportedNode {}

type EmitResult<T> = Result<T, UnsupportedNode>;

fn unsupported(node: &Rc<dyn IXamlAstNode>, reason: impl Into<String>) -> UnsupportedNode {
    UnsupportedNode {
        node_type_name: node.type_name().to_string(),
        reason: reason.into(),
        line: node.line(),
        position: node.position(),
    }
}

fn failed(node: &Rc<dyn IXamlAstNode>, error: XamlError) -> UnsupportedNode {
    unsupported(node, format!("the node could not be examined: {}", error.message()))
}

/// The Rust type of an emitted expression.
#[derive(Clone, Copy)]
enum Kind {
    /// The expression has exactly the Rust type `id`; `nullable` is the
    /// `TypeId` of `Option<that type>` when the type has a nullable form.
    Exact { id: TypeId, nullable: Option<TypeId> },
    /// The expression is a local holding `Ref<class>`.
    Class(&'static TypeInfo),
    /// `{x:Null}`.
    Null,
}

struct Typed {
    expr: String,
    kind: Kind,
}

fn exact<T: 'static>(expr: String) -> Typed {
    Typed { expr, kind: Kind::Exact { id: TypeId::of::<T>(), nullable: Some(TypeId::of::<Option<T>>()) } }
}

/// `snake_case` of a class name, for the names of locals only (the names
/// of Rust items always come from metadata): a word starts at an upper-case
/// letter that follows a lower-case letter or a digit, and at the last
/// upper-case letter of a run that is followed by a lower-case letter.
fn snake_case(name: &str) -> String {
    let characters: Vec<char> = name.chars().collect();
    let mut result = String::with_capacity(name.len() + 4);
    for (index, &current) in characters.iter().enumerate() {
        if !current.is_ascii_alphanumeric() {
            result.push('_');
            continue;
        }
        if index > 0 && current.is_ascii_uppercase() {
            let previous = characters[index - 1];
            let next_is_lower = characters.get(index + 1).is_some_and(|next| next.is_ascii_lowercase());
            if previous.is_ascii_lowercase() || previous.is_ascii_digit() || (previous.is_ascii_uppercase() && next_is_lower)
            {
                result.push('_');
            }
        }
        result.push(current.to_ascii_lowercase());
    }
    result
}

/// A public Rust path from the registries as an absolute path
/// (`::ferroui_controls::Border`), which no item of the including module can
/// shadow.
fn absolute(path: &str) -> String {
    match path.starts_with("::") {
        true => path.to_string(),
        false => format!("::{path}"),
    }
}


fn f64_literal(value: f64) -> String {
    match value {
        _ if value.is_nan() => "::core::primitive::f64::NAN".to_string(),
        f64::INFINITY => "::core::primitive::f64::INFINITY".to_string(),
        f64::NEG_INFINITY => "::core::primitive::f64::NEG_INFINITY".to_string(),
        _ => format!("{value:?}_f64"),
    }
}

fn f32_literal(value: f32) -> String {
    match value {
        _ if value.is_nan() => "::core::primitive::f32::NAN".to_string(),
        f32::INFINITY => "::core::primitive::f32::INFINITY".to_string(),
        f32::NEG_INFINITY => "::core::primitive::f32::NEG_INFINITY".to_string(),
        _ => format!("{value:?}_f32"),
    }
}

fn runtime_type(type_: &Rc<dyn IXamlType>) -> Option<&RuntimeType> {
    type_.as_any().downcast_ref::<RuntimeType>()
}

/// Whether any node of the document needs the parent stack: the interpreter
/// then pushes the objects being initialised onto the stack of the run-time
/// context, which generated code has no counterpart of yet.
struct NeedsParentStack(bool);

impl IXamlAstVisitor for NeedsParentStack {
    fn visit(&mut self, node: Rc<dyn IXamlAstNode>) -> XamlResult<Rc<dyn IXamlAstNode>> {
        if node.as_needs_parent_stack().is_some_and(|n| n.needs_parent_stack()) {
            self.0 = true;
        }
        Ok(node)
    }
    fn push(&mut self, _node: Rc<dyn IXamlAstNode>) {}
    fn pop(&mut self) {}
}

/// The Rust expression that yields the definition of a registered property:
/// a call of an accessor recorded by the declaration macros
/// ([`property_accessors`]) on a type with a public Rust path. Every
/// recorded accessor returns the identical definition; the first one in the
/// order of [`property_accessors`] (the type the property was resolved on
/// and its base types, then the type that registered it and its base
/// types) whose type has a public Rust path is taken, so the choice depends
/// only on the declarations, never on what ran earlier on the thread.
fn property_definition(property: &'static FerroProperty, preferred: Option<&'static TypeInfo>) -> Result<String, String> {
    let accessors = property_accessors(property, preferred);
    let chosen = accessors
        .iter()
        .find_map(|accessor| accessor.owner.rust_path().map(|path| format!("{}::{}()", absolute(path), accessor.name)));
    chosen.ok_or_else(|| match accessors.is_empty() {
        true => format!("no accessor of the property {} is recorded", property.name()),
        false => format!("no accessor of the property {} is declared by a type with a public Rust path", property.name()),
    })
}

struct Emitter<'a> {
    configuration: &'a TransformerConfiguration,
    document_name: &'a str,
    lines: Vec<String>,
    /// The number of locals named after each class.
    local_names: HashMap<String, usize>,
    /// The compiler locals initialised so far, by the address of their node.
    compiler_locals: HashMap<usize, (String, Kind)>,
}

fn node_address<T: ?Sized>(node: &Rc<T>) -> usize {
    Rc::as_ptr(node) as *const () as usize
}

impl Emitter<'_> {
    fn line(&mut self, text: String) {
        self.lines.push(format!("    {text}"));
    }

    /// The position marker of 9.3.6: `// <document>(line,position) <what>`.
    fn marker(&mut self, node: &Rc<dyn IXamlAstNode>, what: &str) {
        let document = self.document_name.replace(['\r', '\n'], " ");
        self.line(format!("// {document}({},{}) {what}", node.line(), node.position()));
    }

    fn local_for(&mut self, class: &'static TypeInfo) -> String {
        self.local_named(&snake_case(class.name()))
    }

    fn local_named(&mut self, base: &str) -> String {
        let counter = self.local_names.entry(base.to_string()).or_insert(0);
        let name = format!("{base}_{counter}");
        *counter += 1;
        name
    }

    // --- values -------------------------------------------------------------

    fn value(&mut self, node: &Rc<dyn IXamlAstNode>) -> EmitResult<Typed> {
        if let Some(group) = node.as_value_with_manipulation_node() {
            // `value_with_manipulation`: the value, then its manipulation unless it is an empty group.
            let created = self.value(&group.value().as_node())?;
            if let Some(manipulation) = group.manipulation() {
                let empty = manipulation
                    .cast::<XamlManipulationGroupNode>()
                    .is_some_and(|inner| inner.children.borrow().is_empty());
                if !empty {
                    self.manipulation(&manipulation.as_node(), &created)?;
                }
            }
            return Ok(created);
        }
        if let Some(n) = node.cast::<XamlAstNewClrObjectNode>() {
            return self.new_object(node, &n);
        }
        if let Some(n) = node.cast::<XamlValueNodeWithBeginInit>() {
            return self.value_with_begin_init(node, &n);
        }
        if let Some(n) = node.cast::<XamlAstLocalInitializationNodeEmitter>() {
            return self.local_initialization(node, &n);
        }
        if node.is::<XamlAstCompilerLocalNode>() {
            let (expr, kind) = self
                .compiler_locals
                .get(&node_address(node))
                .cloned()
                .ok_or_else(|| unsupported(node, "a compiler local that is read before it is initialised"))?;
            return Ok(Typed { expr, kind });
        }
        if let Some(n) = node.cast::<XamlAstTextNode>() {
            return Ok(exact::<String>(format!("::std::string::String::from({})", rust_string_literal(&n.text()))));
        }
        if node.is::<XamlStaticOrTargetedReturnMethodCallNode>() {
            return self.method_call_value(node);
        }
        if node.is::<XamlNullExtensionNode>() {
            return Ok(Typed { expr: "::core::option::Option::None".to_string(), kind: Kind::Null });
        }
        if let Some(n) = node.cast::<XamlConstantNode>() {
            let type_ = IXamlAstValueNode::type_(&*n).get_clr_type().map_err(|e| failed(node, e))?;
            return self.constant(node, &type_, &n.constant);
        }
        if let Some(n) = node.cast::<XamlStaticExtensionNode>() {
            return self.static_member(node, &n);
        }
        if let Some(n) = node.cast::<FerroXamlIlVectorLikeConstantAstNode>() {
            let arguments: Vec<Typed> = n.values().iter().map(|value| exact::<f64>(f64_literal(*value))).collect();
            return self.constructor_call(node, n.constructor(), &arguments);
        }
        if let Some(n) = node.cast::<FerroXamlIlFerroListConstantAstNode>() {
            return self.list_constant(node, &n);
        }
        if let Some(n) = node.cast::<FerroXamlIlGridLengthAstNode>() {
            let types = n.types().clone();
            let constructor = types.grid_length_constructor_value_type.clone();
            let unit_type = constructor
                .parameters()
                .get(1)
                .cloned()
                .ok_or_else(|| unsupported(node, "the grid length constructor doesn't take a unit"))?;
            let grid_length = n.grid_length();
            let value = exact::<f64>(f64_literal(grid_length.value));
            let unit = self.enum_member(node, &unit_type, i64::from(grid_length.grid_unit_type as i32))?;
            return self.constructor_call(node, &constructor, &[value, unit]);
        }
        Err(unsupported(node, "no emitter for this value node"))
    }

    /// `new T()` of a class of the object model with its default constructor.
    fn new_object(&mut self, node: &Rc<dyn IXamlAstNode>, n: &Rc<XamlAstNewClrObjectNode>) -> EmitResult<Typed> {
        let type_ = n.type_.borrow().get_clr_type().map_err(|e| failed(node, e))?;
        let is_declared = n.constructor.as_any().downcast_ref::<RuntimeConstructor>().is_some_and(|c| c.declared.is_some());
        if is_declared {
            // A constructor of metadata (with or without arguments): the arguments in order,
            // then the typed function.
            let values = n.arguments.borrow().clone();
            let mut arguments = Vec::with_capacity(values.len());
            for value in &values {
                arguments.push(self.value(&value.as_node())?);
            }
            return self.constructor_call(node, &n.constructor, &arguments);
        }
        if !n.arguments.borrow().is_empty() {
            return Err(unsupported(node, "constructor arguments"));
        }
        self.default_object(node, &type_, &n.constructor)
    }

    /// `new T()` of a class of the object model through its default
    /// constructor (`T::new()`).
    fn default_object(
        &mut self,
        node: &Rc<dyn IXamlAstNode>,
        type_: &Rc<dyn IXamlType>,
        constructor: &Rc<dyn IXamlConstructor>,
    ) -> EmitResult<Typed> {
        let class = runtime_type(type_)
            .and_then(RuntimeType::type_info)
            .ok_or_else(|| unsupported(node, format!("{} is not a class of the object model", type_.full_name())))?;
        let is_default_constructor = constructor.as_any().downcast_ref::<RuntimeConstructor>().is_some_and(|c| {
            c.parameters.is_empty() && matches!(c.invoker, RuntimeInvoker::Dynamic(_))
        });
        if !is_default_constructor || class.default_constructor().is_none() {
            return Err(unsupported(node, format!("{} has no default constructor", type_.full_name())));
        }
        if crate::FerroRuntimeXamlLoader::has_class_document(class) {
            return Err(unsupported(node, format!("{} is a class with markup of its own", type_.full_name())));
        }
        let path = class
            .rust_path()
            .ok_or_else(|| unsupported(node, format!("no public Rust path is recorded for {}", type_.full_name())))?;
        let local = self.local_for(class);
        self.marker(node, class.name());
        self.line(format!("let {local} = {}::new();", absolute(path)));
        Ok(Typed { expr: local, kind: Kind::Class(class) })
    }

    /// A value whose `BeginInit` runs as soon as it is created (an object
    /// that is usable during its initialisation: the consumer attaches it
    /// before it is populated).
    fn value_with_begin_init(&mut self, node: &Rc<dyn IXamlAstNode>, n: &Rc<XamlValueNodeWithBeginInit>) -> EmitResult<Typed> {
        let value = n.base.value();
        let type_ = value.type_().get_clr_type().map_err(|e| failed(node, e))?;
        let created = self.value(&value.as_node())?;
        if self.supports_initialize(&type_) {
            self.styled_class(node, &created)?;
            self.line(format!("{}.begin_init();", created.expr));
        }
        Ok(created)
    }

    /// The initialisation of a compiler local: the value, kept in a Rust
    /// local the reads of the compiler local name. The interpreter converts
    /// the value to the type of the local, which for an object of a class is
    /// the object itself.
    fn local_initialization(
        &mut self,
        node: &Rc<dyn IXamlAstNode>,
        n: &Rc<XamlAstLocalInitializationNodeEmitter>,
    ) -> EmitResult<Typed> {
        let local = n.local();
        let value = self.value(&n.base.value().as_node())?;
        let Kind::Class(class) = value.kind else {
            return Err(unsupported(node, "a compiler local that does not hold an object of the object model"));
        };
        let local_class = runtime_type(&local.type_)
            .and_then(RuntimeType::type_info)
            .ok_or_else(|| unsupported(node, format!("a compiler local of type {}", local.type_.full_name())))?;
        if !local_class.is_assignable_from(class) {
            return Err(unsupported(node, format!("a compiler local of type {}", local.type_.full_name())));
        }
        self.compiler_locals.insert(node_address(&local), (value.expr.clone(), value.kind));
        Ok(value)
    }

    /// A list from text (`"*,Auto"` for the column definitions of a grid):
    /// the list, its capacity, then each element added in order, as the
    /// interpreter's `list_constant` builds it.
    fn list_constant(&mut self, node: &Rc<dyn IXamlAstNode>, n: &Rc<FerroXamlIlFerroListConstantAstNode>) -> EmitResult<Typed> {
        let list_type = IXamlAstValueNode::type_(&**n).get_clr_type().map_err(|e| failed(node, e))?;
        let constructor = n.constructor().clone();
        let is_declared = constructor.as_any().downcast_ref::<RuntimeConstructor>().is_some_and(|c| c.declared.is_some());
        let list = match is_declared {
            true => self.constructor_call(node, &constructor, &[])?,
            false => self.default_object(node, &list_type, &constructor)?,
        };
        let set_capacity = n
            .list_set_capacity_method()
            .as_any()
            .downcast_ref::<RuntimeMethod>()
            .ok_or_else(|| unsupported(node, "the capacity setter is not a method of the run-time type system"))?;
        let capacity = i32::try_from(n.values().len()).unwrap_or(i32::MAX);
        let call = self.declared_call(node, set_capacity, &[Typed { expr: list.expr.clone(), kind: list.kind }, exact::<i32>(format!("{capacity}_i32"))])?;
        self.line(format!("{call};"));
        let add = n
            .list_add_method()
            .as_any()
            .downcast_ref::<RuntimeMethod>()
            .ok_or_else(|| unsupported(node, "the adder is not a method of the run-time type system"))?;
        for value in n.values() {
            let element = self.value(&value.as_node())?;
            let call = self.declared_call(node, add, &[Typed { expr: list.expr.clone(), kind: list.kind }, element])?;
            self.line(format!("{call};"));
        }
        Ok(list)
    }

    /// A compile-time constant: what `constant_value` loads for it.
    fn constant(&mut self, node: &Rc<dyn IXamlAstNode>, type_: &Rc<dyn IXamlType>, constant: &XamlValue) -> EmitResult<Typed> {
        if let XamlValue::String(text) = constant {
            return Ok(exact::<String>(format!("::std::string::String::from({})", rust_string_literal(text))));
        }
        if matches!(constant, XamlValue::Null) {
            return Ok(Typed { expr: "::core::option::Option::None".to_string(), kind: Kind::Null });
        }
        let (integer, float) =
            numeric_constant(constant).ok_or_else(|| unsupported(node, format!("the constant {constant:?}")))?;
        if type_.is_enum() {
            let value = i64::try_from(integer)
                .map_err(|_| unsupported(node, format!("{integer} is not a value of {}", type_.full_name())))?;
            return self.enum_member(node, type_, value);
        }
        let out_of_range = || unsupported(node, format!("{integer} is out of the range of {}", type_.full_name()));
        macro_rules! integer {
            ($type_:ty, $suffix:literal) => {
                Ok(exact::<$type_>(format!("{}{}", <$type_>::try_from(integer).map_err(|_| out_of_range())?, $suffix)))
            };
        }
        if type_.namespace().as_deref() == Some("System") {
            match type_.name().as_str() {
                "Boolean" => return Ok(exact::<bool>(format!("{}", integer != 0))),
                "Char" => {
                    let character = u32::try_from(integer).ok().and_then(char::from_u32).ok_or_else(out_of_range)?;
                    return Ok(exact::<char>(format!("{character:?}")));
                }
                "SByte" => return integer!(i8, "_i8"),
                "Byte" => return integer!(u8, "_u8"),
                "Int16" => return integer!(i16, "_i16"),
                "UInt16" => return integer!(u16, "_u16"),
                "Int32" => return integer!(i32, "_i32"),
                "UInt32" => return integer!(u32, "_u32"),
                "Int64" => return integer!(i64, "_i64"),
                "UInt64" => return integer!(u64, "_u64"),
                "Single" => return Ok(exact::<f32>(f32_literal(float as f32))),
                "Double" => return Ok(exact::<f64>(f64_literal(float))),
                _ => {}
            }
        }
        // The constant is typed as something else (`System.Object`): its own kind decides.
        Ok(match constant {
            XamlValue::Boolean(v) => exact::<bool>(format!("{v}")),
            XamlValue::Char(v) => exact::<char>(format!("{v:?}")),
            XamlValue::SByte(v) => exact::<i8>(format!("{v}_i8")),
            XamlValue::Byte(v) => exact::<u8>(format!("{v}_u8")),
            XamlValue::Int16(v) => exact::<i16>(format!("{v}_i16")),
            XamlValue::UInt16(v) => exact::<u16>(format!("{v}_u16")),
            XamlValue::Int32(v) => exact::<i32>(format!("{v}_i32")),
            XamlValue::UInt32(v) => exact::<u32>(format!("{v}_u32")),
            XamlValue::Int64(v) => exact::<i64>(format!("{v}_i64")),
            XamlValue::UInt64(v) => exact::<u64>(format!("{v}_u64")),
            XamlValue::Single(v) => exact::<f32>(f32_literal(*v)),
            XamlValue::Double(v) => exact::<f64>(f64_literal(*v)),
            _ => return Err(unsupported(node, format!("the constant {constant:?} as a value of {}", type_.full_name()))),
        })
    }

    /// The member of a plain enumeration with the numeric value `value`, as
    /// its Rust variant.
    fn enum_member(&mut self, node: &Rc<dyn IXamlAstNode>, type_: &Rc<dyn IXamlType>, value: i64) -> EmitResult<Typed> {
        let markup = runtime_type(type_)
            .and_then(RuntimeType::markup)
            .ok_or_else(|| unsupported(node, format!("{} has no metadata", type_.full_name())))?;
        if markup.is_flags {
            return Err(unsupported(node, format!("{} is a set of flags", type_.full_name())));
        }
        let member = markup.enum_members.iter().find(|member| member.value == value);
        let variant = member
            .and_then(|member| member.rust_variant)
            .ok_or_else(|| unsupported(node, format!("{value} is not a member of {}", type_.full_name())))?;
        self.enum_variant(node, markup, variant)
    }

    fn enum_variant(&mut self, node: &Rc<dyn IXamlAstNode>, markup: &'static MarkupType, variant: &str) -> EmitResult<Typed> {
        let path = markup
            .rust_path()
            .ok_or_else(|| unsupported(node, format!("no public Rust path is recorded for {}", markup.full_name())))?;
        let id = markup.handle().ok_or_else(|| unsupported(node, "the enumeration has no value type"))?.id();
        let nullable = markup.nullable.map(|nullable| nullable().id());
        Ok(Typed { expr: format!("{}::{variant}", absolute(path)), kind: Kind::Exact { id, nullable } })
    }

    /// `{x:Static Type.Member}` naming a member of a plain enumeration.
    fn static_member(&mut self, node: &Rc<dyn IXamlAstNode>, n: &Rc<XamlStaticExtensionNode>) -> EmitResult<Typed> {
        let member = n.resolve_member(true).map_err(|e| failed(node, e))?;
        let Some(XamlStaticMember::Field(field)) = member else {
            return Err(unsupported(node, "a static member that is not a field"));
        };
        let Some(runtime) = field.as_any().downcast_ref::<RuntimeField>() else {
            return Err(unsupported(node, "a field that is not a field of the run-time type system"));
        };
        if !matches!(runtime.value, RuntimeFieldValue::EnumMember(_)) {
            return Err(unsupported(node, "a static field that is not an enumeration member"));
        }
        let declaring_type = field.declaring_type();
        let markup = runtime_type(&declaring_type)
            .and_then(RuntimeType::markup)
            .ok_or_else(|| unsupported(node, format!("{} has no metadata", declaring_type.full_name())))?;
        if markup.is_flags {
            return Err(unsupported(node, format!("{} is a set of flags", declaring_type.full_name())));
        }
        let name = field.name();
        let variant = markup
            .enum_members
            .iter()
            .find(|member| member.name == name)
            .and_then(|member| member.rust_variant)
            .ok_or_else(|| unsupported(node, format!("{name} is not a member of {}", declaring_type.full_name())))?;
        self.enum_variant(node, markup, variant)
    }

    /// A call of a declared constructor through the typed function the
    /// declaration generated for it, with the arguments stated as the
    /// declared parameter types. An object of a class is kept in a local.
    fn constructor_call(
        &mut self,
        node: &Rc<dyn IXamlAstNode>,
        constructor: &Rc<dyn IXamlConstructor>,
        arguments: &[Typed],
    ) -> EmitResult<Typed> {
        let runtime = constructor
            .as_any()
            .downcast_ref::<RuntimeConstructor>()
            .ok_or_else(|| unsupported(node, "a constructor that is not one of the run-time type system"))?;
        let declaring = runtime
            .declaring_type
            .upgrade()
            .ok_or_else(|| unsupported(node, "the declaring type of the constructor is gone"))?;
        let declared = runtime
            .declared
            .ok_or_else(|| unsupported(node, format!("a constructor of {} that metadata does not declare", declaring.full_name())))?;
        let emit = declared
            .emit
            .ok_or_else(|| unsupported(node, format!("the constructor of {} has no typed function", declaring.full_name())))?;
        let owner = self.type_path(node, &declaring)?;
        if runtime.parameter_handles.len() != arguments.len() {
            return Err(unsupported(node, "the constructor doesn't take the arguments of the node"));
        }
        let mut texts = Vec::with_capacity(arguments.len());
        for (index, (argument, handle)) in arguments.iter().zip(&runtime.parameter_handles).enumerate() {
            let handle = handle.ok_or_else(|| unsupported(node, format!("argument {index} of the constructor has no Rust type")))?;
            texts.push(self.coerce(argument, handle.id()).ok_or_else(|| {
                unsupported(node, format!("argument {index} of the constructor cannot be stated as `{}`", handle.name()))
            })?);
        }
        let value = declaring
            .markup()
            .and_then(|markup| markup.value)
            .ok_or_else(|| unsupported(node, format!("the value type of {} is not known", declaring.full_name())))?;
        let mut call = format!("{owner}::{}({})", emit.function, texts.join(", "));
        if emit.fallible {
            call.push_str(&format!(".map_err(|error| rt::at(rt::TARGET_INVOCATION_EXCEPTION, error, {}, {}))?", node.line(), node.position()));
        }
        let kind = self.kind_of(value().id());
        if let Kind::Class(class) = kind {
            let local = self.local_for(class);
            self.marker(node, class.name());
            self.line(format!("let {local} = {call};"));
            return Ok(Typed { expr: local, kind });
        }
        Ok(Typed { expr: call, kind })
    }

    /// The expression as a value of exactly the Rust type `target` (the
    /// value type of a registered property), with the conversion the
    /// run-time loader applies to the argument of the property's setter
    /// (`to_exact`). `None`: it cannot be stated.
    fn coerce(&self, value: &Typed, target: TypeId) -> Option<String> {
        self.coerce_static(value, target).or_else(|| self.coerce_registered(value, target))
    }

    /// A conversion through the assignability casts of the untyped value
    /// conversions (an interface handle, a registered cast), where
    /// `ValueTypes::is_assignable` proves it exists: `rt::cast(value)`, which
    /// performs the very cast of the run-time loader.
    fn coerce_registered(&self, value: &Typed, target: TypeId) -> Option<String> {
        use ferroui_base::data::core::{ValueType, ValueTypes};
        let (from, expr) = match value.kind {
            Kind::Exact { id, .. } => (id, value.expr.clone()),
            Kind::Class(class) => (class.handle()?, format!("{}.clone()", value.expr)),
            Kind::Null => return None,
        };
        ValueTypes::is_assignable(ValueType::new(from, ""), ValueType::new(target, "")).then(|| format!("rt::cast({expr})"))
    }

    fn coerce_static(&self, value: &Typed, target: TypeId) -> Option<String> {
        let object = TypeId::of::<Option<BoxedValue>>();
        match value.kind {
            Kind::Exact { id, nullable } => {
                if target == id {
                    Some(value.expr.clone())
                } else if nullable == Some(target) {
                    Some(format!("::core::option::Option::Some({})", value.expr))
                } else if target == object {
                    Some(format!("rt::to_object({})", value.expr))
                } else {
                    None
                }
            }
            Kind::Class(class) => {
                if target == object {
                    return Some(format!("rt::to_object(::core::clone::Clone::clone(&{}))", value.expr));
                }
                let (declared, nullable) = TypeInfo::find_by_handle(target)?;
                if !declared.is_assignable_from(class) {
                    return None;
                }
                let handle = if std::ptr::eq(declared, class) {
                    format!("::core::clone::Clone::clone(&{})", value.expr)
                } else {
                    format!("::core::clone::Clone::clone(&{}).upcast::<{}>()", value.expr, absolute(declared.rust_path()?))
                };
                Some(if nullable { format!("::core::option::Option::Some({handle})") } else { handle })
            }
            Kind::Null => {
                let is_nullable_class = TypeInfo::find_by_handle(target).is_some_and(|(_, nullable)| nullable);
                (is_nullable_class || target == object || target == TypeId::of::<Option<String>>())
                    .then(|| "::core::option::Option::None".to_string())
            }
        }
    }

    // --- manipulations ------------------------------------------------------

    fn manipulation(&mut self, node: &Rc<dyn IXamlAstNode>, target: &Typed) -> EmitResult<()> {
        if let Some(n) = node.cast::<XamlObjectInitializationNode>() {
            return self.object_initialization(node, &n, target);
        }
        if let Some(n) = node.cast::<XamlManipulationGroupNode>() {
            let children = n.children.borrow().clone();
            for child in &children {
                self.manipulation(&child.as_node(), target)?;
            }
            return Ok(());
        }
        if let Some(n) = node.cast::<XamlPropertyAssignmentNode>() {
            return self.property_assignment(node, &n, target);
        }
        if node.is::<HandleRootObjectScopeNode>() {
            return self.root_object_scope(node, target);
        }
        if node.is::<XamlNoReturnMethodCallNode>() {
            let (call, _) = self.method_call(node, Some(target))?;
            self.line(format!("{call};"));
            return Ok(());
        }
        if let Some(n) = node.cast::<XamlAstManipulationImperativeNode>() {
            // The value this node is "supposed" to manipulate is discarded.
            let imperative = n.imperative().as_node();
            let Some(manipulation) = imperative.cast::<XamlAstImperativeValueManipulation>() else {
                return Err(unsupported(&imperative, "no emitter for this imperative node"));
            };
            let value = self.value(&manipulation.value().as_node())?;
            return self.manipulation(&manipulation.manipulation().as_node(), &value);
        }
        if let Some(n) = node.cast::<FerroNameScopeRegistrationXamlIlNode>() {
            return self.name_scope_registration(node, &n, target);
        }
        Err(unsupported(node, "no emitter for this manipulation node"))
    }

    fn styled_class(&self, node: &Rc<dyn IXamlAstNode>, target: &Typed) -> EmitResult<&'static TypeInfo> {
        match target.kind {
            Kind::Class(class) if StyledElement::TYPE.is_assignable_from(class) => Ok(class),
            _ => Err(unsupported(node, "the target is not a styled element")),
        }
    }

    /// Whether `BeginInit` / `EndInit` are called on a value of the type:
    /// the type is assignable to the initialisation contract of the
    /// language.
    fn supports_initialize(&self, type_: &Rc<dyn IXamlType>) -> bool {
        self.configuration
            .type_mappings
            .support_initialize
            .as_ref()
            .is_some_and(|support_initialize| support_initialize.is_assignable_from(&**type_))
    }

    /// `BeginInit`, the manipulation, `EndInit` (the parent stack is never
    /// needed: a document that needs it is not eligible).
    fn object_initialization(
        &mut self,
        node: &Rc<dyn IXamlAstNode>,
        init: &Rc<XamlObjectInitializationNode>,
        target: &Typed,
    ) -> EmitResult<()> {
        let type_ = init.type_.borrow().clone();
        let supports_initialize = self.supports_initialize(&type_);
        if supports_initialize {
            // The contract is the one of the styled element; nothing else implements it by a known path.
            self.styled_class(node, target)?;
            if !init.skip_begin_init.get() {
                self.line(format!("{}.begin_init();", target.expr));
            }
        }
        self.manipulation(&init.manipulation().as_node(), target)?;
        if supports_initialize {
            // The interpreter calls `EndInit` as a member of the contract: its failure is the
            // exception that wraps the exception of the member.
            self.line(format!(
                "{}.try_end_init().map_err(|error| rt::at(rt::TARGET_INVOCATION_EXCEPTION, error, {}, {}))?;",
                target.expr,
                node.line(),
                node.position()
            ));
        }
        Ok(())
    }

    /// An assignment of a registered (styled, attached or direct) property
    /// through the accessor the type system projects for it:
    /// `target.set_value(Owner::name_property(), value)`, or
    /// `target.set_direct_value(..)` for a direct property.
    fn property_assignment(
        &mut self,
        node: &Rc<dyn IXamlAstNode>,
        assignment: &Rc<XamlPropertyAssignmentNode>,
        target: &Typed,
    ) -> EmitResult<()> {
        let property_name = assignment.property.name();
        let setter = single_setter(node, assignment)
            .map_err(|e| failed(node, e))?
            .ok_or_else(|| unsupported(node, format!("{property_name}: the setter is chosen at run time")))?;
        if let Some(adder) = setter.as_any().downcast_ref::<AdderSetter>() {
            return self.adder_assignment(node, assignment, adder, target);
        }
        let direct = setter
            .as_any()
            .downcast_ref::<XamlDirectCallPropertySetter>()
            .ok_or_else(|| unsupported(node, format!("{property_name}: not a plain property setter")))?;
        let method = direct.method().clone();
        let runtime = method
            .as_any()
            .downcast_ref::<RuntimeMethod>()
            .ok_or_else(|| unsupported(node, format!("{property_name}: the setter is not a method of the run-time type system")))?;
        if runtime.declared().is_some() {
            return self.declared_setter_assignment(node, assignment, runtime, target);
        }
        let field = XamlIlFerroPropertyHelper::try_get_ferro_property_field(&assignment.property)
            .ok_or_else(|| unsupported(node, format!("{property_name}: not a registered property")))?;
        let property = field
            .as_any()
            .downcast_ref::<RuntimeField>()
            .and_then(RuntimeField::ferro_property)
            .ok_or_else(|| unsupported(node, format!("{property_name}: not a registered property")))?;
        // The setter must be the accessor the type system builds for the property
        // (`set_value_untyped(property, value, LocalValue)`), not a declared one.
        let expected_name = match runtime.is_static {
            true => format!("Set{}", property.name()),
            false => format!("set_{}", property.name()),
        };
        let expected_parameters = if runtime.is_static { 2 } else { 1 };
        let is_plain_accessor = matches!(runtime.invoker, RuntimeInvoker::Dynamic(_))
            && runtime.name == expected_name
            && runtime.parameters.len() == expected_parameters
            && method.declaring_type().equals(&*field.declaring_type());
        if !is_plain_accessor {
            return Err(unsupported(node, format!("{property_name}: the setter is a declared member")));
        }
        if property.is_read_only() {
            return Err(unsupported(node, format!("{property_name}: a read-only property")));
        }
        // Every class of the object model is a `FerroObject`, the owner of the property store.
        if !matches!(target.kind, Kind::Class(_)) {
            return Err(unsupported(node, format!("{property_name}: the target is not an object of the object model")));
        }
        let declaring_type = field.declaring_type();
        let definition = property_definition(property, runtime_type(&declaring_type).and_then(RuntimeType::type_info))
            .map_err(|reason| unsupported(node, format!("{property_name}: {reason}")))?;
        let values = assignment.values.borrow().clone();
        let [value_node] = values.as_slice() else {
            return Err(unsupported(node, format!("{property_name}: an assignment with {} values", values.len())));
        };
        let value_node = value_node.as_node();
        self.marker(node, &property_name);
        let value = self.value(&value_node)?;
        let typed = self.coerce(&value, property.property_type()).ok_or_else(|| {
            unsupported(
                &value_node,
                format!("{property_name}: the value cannot be stated as `{}`", property.property_type_name()),
            )
        })?;
        let call = if property.is_direct() { "set_direct_value" } else { "set_value" };
        self.line(format!("{}.{call}({definition}, {typed});", target.expr));
        Ok(())
    }

    /// The kind of a value of the Rust type `id` held in a local: an object
    /// of a class, or a value of exactly that type (with the nullable form
    /// of a value type with metadata).
    fn kind_of(&self, id: TypeId) -> Kind {
        if let Some((class, false)) = TypeInfo::find_by_handle(id) {
            return Kind::Class(class);
        }
        let nullable = MarkupType::find_by_handle(id).and_then(|markup| markup.nullable).map(|nullable| nullable().id());
        Kind::Exact { id, nullable }
    }

    /// A method call node (`XamlStaticOrTargetedReturnMethodCallNode`, or
    /// `XamlNoReturnMethodCallNode` on `target`): the arguments in order,
    /// then the call of the declared member. Returns the call and the Rust
    /// type it yields.
    fn method_call(&mut self, node: &Rc<dyn IXamlAstNode>, target: Option<&Typed>) -> EmitResult<(String, Option<TypeId>)> {
        let call = node.as_method_call_base_node().ok_or_else(|| unsupported(node, "not a method call"))?;
        let wrapped = call.method.borrow().clone();
        let method = wrapped
            .as_any()
            .downcast_ref::<XamlWrappedMethod>()
            .map(|wrapped| wrapped.method().clone())
            .ok_or_else(|| unsupported(node, "a method call with argument casts"))?;
        let runtime = method
            .as_any()
            .downcast_ref::<RuntimeMethod>()
            .ok_or_else(|| unsupported(node, "a method that is not a method of the run-time type system"))?;
        let mut arguments = Vec::new();
        if let Some(target) = target {
            arguments.push(Typed { expr: target.expr.clone(), kind: target.kind });
        }
        let values = call.arguments.borrow().clone();
        for value in &values {
            arguments.push(self.value(&value.as_node())?);
        }
        let text = self.declared_call(node, runtime, &arguments)?;
        let returned = match target {
            Some(_) => None,
            None => Some(self.declared_return(node, runtime)?),
        };
        Ok((text, returned))
    }

    /// A method call that yields a value, kept in a local.
    fn method_call_value(&mut self, node: &Rc<dyn IXamlAstNode>) -> EmitResult<Typed> {
        let (call, returned) = self.method_call(node, None)?;
        let returned = returned.ok_or_else(|| unsupported(node, "a method call that yields nothing"))?;
        let local = self.local_named("value");
        self.line(format!("let {local} = {call};"));
        Ok(Typed { expr: local, kind: self.kind_of(returned) })
    }

    /// The values of a property assignment, evaluated in order.
    fn assignment_values(&mut self, node: &Rc<dyn IXamlAstNode>, assignment: &XamlPropertyAssignmentNode) -> EmitResult<Vec<Typed>> {
        let values = assignment.values.borrow().clone();
        if values.is_empty() {
            return Err(unsupported(node, "an assignment without values"));
        }
        let mut typed = Vec::with_capacity(values.len());
        for value in &values {
            typed.push(self.value(&value.as_node())?);
        }
        Ok(typed)
    }

    /// An assignment through a declared accessor: the setter of a plain
    /// property (`this.Name = value`) or a static accessor of an attached
    /// property (`Owner.SetName(target, value)`), called with the target
    /// followed by the values, as the interpreter calls the method of the
    /// setter.
    fn declared_setter_assignment(
        &mut self,
        node: &Rc<dyn IXamlAstNode>,
        assignment: &Rc<XamlPropertyAssignmentNode>,
        method: &RuntimeMethod,
        target: &Typed,
    ) -> EmitResult<()> {
        let property_name = assignment.property.name();
        self.marker(node, &property_name);
        let mut arguments = vec![Typed { expr: target.expr.clone(), kind: target.kind }];
        arguments.extend(self.assignment_values(node, assignment)?);
        let call = self.declared_call(node, method, &arguments)?;
        self.line(format!("{call};"));
        Ok(())
    }

    /// An assignment through the adder of a collection property: the
    /// collection is read with the getter first, then the values are
    /// evaluated and added (`AdderSetter`).
    fn adder_assignment(
        &mut self,
        node: &Rc<dyn IXamlAstNode>,
        assignment: &Rc<XamlPropertyAssignmentNode>,
        adder: &AdderSetter,
        target: &Typed,
    ) -> EmitResult<()> {
        let property_name = assignment.property.name();
        let getter = adder
            .getter()
            .as_any()
            .downcast_ref::<RuntimeMethod>()
            .ok_or_else(|| unsupported(node, format!("{property_name}: the getter is not a method of the run-time type system")))?;
        let add = adder
            .adder()
            .as_any()
            .downcast_ref::<RuntimeMethod>()
            .ok_or_else(|| unsupported(node, format!("{property_name}: the adder is not a method of the run-time type system")))?;
        self.marker(node, &property_name);
        let (call, collection_type) = match getter.declared() {
            Some(_) => {
                let collection_type = self.declared_return(node, getter)?;
                (self.declared_call(node, getter, std::slice::from_ref(target))?, collection_type)
            }
            None => self.registered_getter(node, assignment, getter, target)?,
        };
        let local = self.local_named(&format!("{}_collection", snake_case(&property_name)));
        self.line(format!("let {local} = {call};"));
        let collection = Typed { expr: local, kind: self.kind_of(collection_type) };
        let mut arguments = vec![collection];
        arguments.extend(self.assignment_values(node, assignment)?);
        let call = self.declared_call(node, add, &arguments)?;
        self.line(format!("{call};"));
        Ok(())
    }

    /// The read of a registered property through the getter the type system
    /// projects for it (`get_value_untyped(property)` in the interpreter):
    /// `target.get_value(..)`, or `target.get_direct_value(..)` for a direct
    /// property. Returns the call and the value type of the property.
    fn registered_getter(
        &self,
        node: &Rc<dyn IXamlAstNode>,
        assignment: &XamlPropertyAssignmentNode,
        getter: &RuntimeMethod,
        target: &Typed,
    ) -> EmitResult<(String, TypeId)> {
        let property_name = assignment.property.name();
        let field = XamlIlFerroPropertyHelper::try_get_ferro_property_field(&assignment.property)
            .ok_or_else(|| unsupported(node, format!("{property_name}: the getter is neither declared nor registered")))?;
        let property = field
            .as_any()
            .downcast_ref::<RuntimeField>()
            .and_then(RuntimeField::ferro_property)
            .ok_or_else(|| unsupported(node, format!("{property_name}: not a registered property")))?;
        let is_plain_getter = matches!(getter.invoker, RuntimeInvoker::Dynamic(_))
            && !getter.is_static
            && getter.name == format!("get_{}", property.name())
            && getter.parameters.is_empty();
        if !is_plain_getter || !matches!(target.kind, Kind::Class(_)) {
            return Err(unsupported(node, format!("{property_name}: the getter is not the accessor of the registered property")));
        }
        let declaring_type = field.declaring_type();
        let definition = property_definition(property, runtime_type(&declaring_type).and_then(RuntimeType::type_info))
            .map_err(|reason| unsupported(node, format!("{property_name}: {reason}")))?;
        let call = if property.is_direct() { "get_direct_value" } else { "get_value" };
        Ok((format!("{}.{call}({definition})", target.expr), property.property_type()))
    }

    /// The Rust type a declared getter or method returns.
    fn declared_return(&self, node: &Rc<dyn IXamlAstNode>, method: &RuntimeMethod) -> EmitResult<TypeId> {
        let returned = match method.declared() {
            Some(DeclaredMember::Getter(property)) | Some(DeclaredMember::StaticGetter(property)) => Some((property.type_)()),
            Some(DeclaredMember::Method(declared)) => declared.return_type.map(|return_type| return_type()),
            Some(DeclaredMember::Parse(markup)) => markup.value.map(|value| value()),
            _ => None,
        };
        returned
            .map(|handle| handle.id())
            .ok_or_else(|| unsupported(node, format!("{}: the member returns no declared value", method.name)))
    }

    /// The instance of an instance member, from a value of another Rust
    /// type (a nullable collection read from a property): converted at run
    /// time as the run-time loader converts it (`rt::argument`), with the
    /// loader's error for a value that does not convert. `None` for an
    /// argument that is not the instance.
    fn instance_argument(&self, node: &Rc<dyn IXamlAstNode>, method: &RuntimeMethod, index: usize, argument: &Typed) -> Option<String> {
        if index != 0 || method.is_static || !matches!(argument.kind, Kind::Exact { .. }) {
            return None;
        }
        let declaring = method.declaring_type.upgrade()?;
        let member = format!("{}.{}", declaring.full_name(), method.name);
        Some(format!(
            "rt::argument({}.clone(), {}, 0, {}, {})?",
            argument.expr,
            rust_string_literal(&member),
            node.line(),
            node.position()
        ))
    }

    /// The path the associated functions of the declaring type of `method`
    /// are called by.
    fn owner_path(&self, node: &Rc<dyn IXamlAstNode>, method: &RuntimeMethod) -> EmitResult<String> {
        let declaring = method
            .declaring_type
            .upgrade()
            .ok_or_else(|| unsupported(node, format!("{}: the declaring type is gone", method.name)))?;
        self.type_path(node, &declaring)
    }

    /// The path the associated functions of a type are called by: the
    /// public path of the class or the markup type (`<dyn ::path::Trait>` for
    /// a contract).
    fn type_path(&self, node: &Rc<dyn IXamlAstNode>, runtime: &Rc<RuntimeType>) -> EmitResult<String> {
        if let Some(class) = runtime.type_info() {
            return class
                .rust_path()
                .map(absolute)
                .ok_or_else(|| unsupported(node, format!("no public Rust path is recorded for {}", runtime.full_name())));
        }
        let markup = runtime
            .markup()
            .ok_or_else(|| unsupported(node, format!("{} has no metadata", runtime.full_name())))?;
        let path = markup
            .rust_path()
            .ok_or_else(|| unsupported(node, format!("no public Rust path is recorded for {}", runtime.full_name())))?;
        Ok(match markup.rust_path_is_trait() {
            true => format!("<dyn {}>", absolute(path)),
            false => absolute(path),
        })
    }

    /// A call of the typed function of the declared member `method`
    /// ([`DeclaredMember`]) with `arguments` (the instance first for an
    /// instance member), each stated as the Rust type the member declares,
    /// with the failure of a fallible member as a load error at `node`.
    fn declared_call(&self, node: &Rc<dyn IXamlAstNode>, method: &RuntimeMethod, arguments: &[Typed]) -> EmitResult<String> {
        let name = &method.name;
        let declared = method.declared().ok_or_else(|| unsupported(node, format!("{name}: not a declared member")))?;
        let emit = declared.emit().ok_or_else(|| unsupported(node, format!("{name}: the declaration has no typed function")))?;
        let owner = self.owner_path(node, method)?;
        let mut parameters: Vec<TypeId> = Vec::with_capacity(arguments.len());
        if !method.is_static {
            let this = method
                .declaring_type
                .upgrade()
                .and_then(|declaring| declaring.markup())
                .and_then(|markup| markup.this)
                .ok_or_else(|| unsupported(node, format!("{name}: the instance type of the declaration is not known")))?;
            parameters.push(this().id());
        }
        for handle in &method.parameter_handles {
            let handle = handle.ok_or_else(|| unsupported(node, format!("{name}: a parameter without a Rust type")))?;
            parameters.push(handle.id());
        }
        if parameters.len() != arguments.len() {
            return Err(unsupported(node, format!("{name}: {} arguments for {} parameters", arguments.len(), parameters.len())));
        }
        let mut texts = Vec::with_capacity(arguments.len());
        for (index, (argument, parameter)) in arguments.iter().zip(&parameters).enumerate() {
            let text = match self.coerce(argument, *parameter) {
                Some(text) => text,
                None => self.instance_argument(node, method, index, argument).ok_or_else(|| {
                    unsupported(node, format!("{name}: argument {index} cannot be stated as the declared type"))
                })?,
            };
            // The instance is passed by reference: a local of exactly the declared type as it is.
            let is_instance = index == 0 && !method.is_static;
            texts.push(match (is_instance, text.strip_suffix(".clone()")) {
                (true, Some(local)) if local == argument.expr => format!("&{local}"),
                (true, _) if text == argument.expr => format!("&{text}"),
                (true, _) => format!("&{text}"),
                (false, _) => text,
            });
        }
        let mut call = format!("{owner}::{}({})", emit.function, texts.join(", "));
        if emit.fallible {
            call.push_str(&format!(".map_err(|error| rt::at(rt::TARGET_INVOCATION_EXCEPTION, error, {}, {}))?", node.line(), node.position()));
        }
        Ok(call)
    }

    /// `if (root is StyledElement s) NameScope.SetNameScope(s, scope); scope.Complete();`.
    fn root_object_scope(&mut self, node: &Rc<dyn IXamlAstNode>, target: &Typed) -> EmitResult<()> {
        let Kind::Class(class) = target.kind else {
            return Err(unsupported(node, "the root object is not an object of the object model"));
        };
        // The exact class of the root is known, so is whether it is a styled element.
        let root = match StyledElement::TYPE.is_assignable_from(class) {
            true => format!("::core::option::Option::Some(&{})", target.expr),
            false => "::core::option::Option::None".to_string(),
        };
        self.line(format!(
            "rt::complete_root_name_scope({root}, name_scope.as_ref(), {}, {})?;",
            node.line(),
            node.position()
        ));
        Ok(())
    }

    /// `context.FerroNameScope.Register(name, target)` with a name from text.
    fn name_scope_registration(
        &mut self,
        node: &Rc<dyn IXamlAstNode>,
        registration: &Rc<FerroNameScopeRegistrationXamlIlNode>,
        target: &Typed,
    ) -> EmitResult<()> {
        if !matches!(target.kind, Kind::Class(_)) {
            return Err(unsupported(node, "the target is not an object of the object model"));
        }
        let name = registration
            .name()
            .cast::<XamlAstTextNode>()
            .ok_or_else(|| unsupported(node, "a name that is not text"))?
            .text();
        self.line(format!(
            "rt::register_name(name_scope.as_ref(), {}, ::core::clone::Clone::clone(&{}).upcast::<::ferroui_base::FerroObject>(), {}, {})?;",
            rust_string_literal(&name),
            target.expr,
            node.line(),
            node.position()
        ));
        Ok(())
    }
}

/// Writes the Rust function that builds the document with the transformed
/// root node `root`: the counterpart of the `Build` method
/// (`Interpreter::build` followed by `Interpreter::populate`).
///
/// ```ignore
/// pub fn <function_name>(
///     service_provider: Option<::std::rc::Rc<dyn ::ferroui_base::metadata::IServiceProvider>>,
/// ) -> Result<::ferroui_base::Ref<RootClass>, ::ferroui_markup_xaml::XamlLoadException>
/// ```
///
/// `service_provider` is the parent service provider of the context (the
/// caller passes `XamlIlRuntimeHelpers::create_root_service_provider_v3(..)`,
/// as the run-time loader does). The function refers to the helpers of
/// compiled markup as `rt` (`use ::ferroui_markup_xaml::xaml_il::runtime::compiled as rt;`
/// in the including module). `document_name` goes into the header comment
/// and the position markers. `Err` means the document is not eligible;
/// nothing is written for it.
pub fn emit_document(
    root: &Rc<dyn IXamlAstNode>,
    configuration: &TransformerConfiguration,
    function_name: &str,
    document_name: &str,
) -> Result<String, UnsupportedNode> {
    let mut needs_parent_stack = NeedsParentStack(false);
    visit_node(root, &mut needs_parent_stack).map_err(|e| failed(root, e))?;
    if needs_parent_stack.0 {
        return Err(unsupported(root, "a node of the document needs the parent stack"));
    }
    let group = root
        .as_value_with_manipulation_node()
        .ok_or_else(|| unsupported(root, "the root is not a value with a manipulation"))?;
    let manipulation = group.manipulation().ok_or_else(|| unsupported(root, "the root has no manipulation"))?;
    let root_value = group.value().as_node();
    if !root_value.is::<XamlAstNewClrObjectNode>() {
        return Err(unsupported(&root_value, "the root object is not created with a constructor"));
    }

    let mut emitter = Emitter { configuration, document_name, lines: Vec::new(), local_names: HashMap::new(), compiler_locals: HashMap::new() };
    // `Build`: the root object, then `Populate` with a context of its own, whose name
    // scope field is filled from the parent service provider.
    let created = emitter.value(&root_value)?;
    let Kind::Class(root_class) = created.kind else {
        return Err(unsupported(&root_value, "the root object is not a class of the object model"));
    };
    let root_path = root_class
        .rust_path()
        .ok_or_else(|| unsupported(&root_value, "no public Rust path is recorded for the root class"))?;
    emitter.line("let name_scope = rt::name_scope_of(service_provider.as_ref());".to_string());
    emitter.manipulation(&manipulation.as_node(), &created)?;
    emitter.line(format!("::core::result::Result::Ok({})", created.expr));

    let mut source = String::new();
    source.push_str(&format!("/// Generated from `{}`.\n", document_name.replace('`', "'").replace(['\r', '\n'], " ")));
    source.push_str(&format!("pub fn {function_name}(\n"));
    source.push_str("    service_provider: ::core::option::Option<::std::rc::Rc<dyn ::ferroui_base::metadata::IServiceProvider>>,\n");
    source.push_str(&format!(
        ") -> ::core::result::Result<::ferroui_base::Ref<{}>, ::ferroui_markup_xaml::XamlLoadException> {{\n",
        absolute(root_path)
    ));
    for line in &emitter.lines {
        source.push_str(line);
        source.push('\n');
    }
    source.push_str("}\n");
    Ok(source)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn class_names_become_local_names() {
        assert_eq!(snake_case("Border"), "border");
        assert_eq!(snake_case("StackPanel"), "stack_panel");
        assert_eq!(snake_case("UIElement"), "ui_element");
        assert_eq!(snake_case("FerroList`1"), "ferro_list_1");
    }

    #[test]
    fn numbers_are_typed_literals() {
        assert_eq!(f64_literal(100.0), "100.0_f64");
        assert_eq!(f64_literal(-0.5), "-0.5_f64");
        assert_eq!(f64_literal(1e300), "1e300_f64");
        assert_eq!(f64_literal(f64::NAN), "::core::primitive::f64::NAN");
        assert_eq!(f64_literal(f64::INFINITY), "::core::primitive::f64::INFINITY");
        assert_eq!(f64_literal(f64::NEG_INFINITY), "::core::primitive::f64::NEG_INFINITY");
        assert_eq!(f32_literal(0.25), "0.25_f32");
        assert_eq!(f32_literal(f32::NEG_INFINITY), "::core::primitive::f32::NEG_INFINITY");
    }
}
