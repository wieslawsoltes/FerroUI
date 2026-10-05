//! Walks the transformed AST of a document and writes the Rust statements
//! that build it: one emit per evaluation of the interpreter
//! (`runtime/interpreter/evaluators.rs`, `runtime/framework/nodes.rs`), in
//! the same order. Anything outside the supported set is reported as
//! [`UnsupportedNode`]; nothing approximate is written.

use std::any::TypeId;
use std::collections::{HashMap, HashSet};
use std::fmt;
use std::rc::Rc;

use ferroui_base::metadata::{property_accessors, rust_path_of_type, IServiceProvider, MarkupType, PropertyAccessor};
use ferroui_base::data::CompiledBindingPath;
use ferroui_base::{BoxedValue, FerroProperty, StyledElement, TypeInfo};
use xamlx::ast::XamlAstExtensions as _;
use xamlx::ast::XamlAstNodeExtensions as _;
use xamlx::ast::{
    visit_node, IXamlAstNode, IXamlAstValueNode, IXamlAstVisitor, IXamlPropertySetter, XamlAstCompilerLocalNode,
    XamlAstImperativeValueManipulation, XamlAstLocalInitializationNodeEmitter, XamlAstManipulationImperativeNode,
    XamlAstNewClrObjectNode, XamlAstTextNode, XamlConstantNode, XamlDirectCallPropertySetter, XamlManipulationGroupNode, XamlNullExtensionNode,
    XamlMarkupExtensionNode, XamlNoReturnMethodCallNode, XamlObjectInitializationNode, XamlPropertyAssignmentNode, XamlStaticExtensionNode,
    XamlStaticMember, XamlStaticOrTargetedReturnMethodCallNode, XamlTypeExtensionNode, XamlWrappedMethod, XamlDeferredContentNode,
    XamlDeferredContentInitializeIntermediateRootNode,
    XamlValueNodeWithBeginInit,
};
use xamlx::exceptions::{XamlError, XamlResult};
use xamlx::transform::transformers::AdderSetter;
use xamlx::transform::TransformerConfiguration;
use xamlx::type_system::{IXamlConstructor, IXamlField, IXamlMethod, IXamlProperty, IXamlType, XamlValue};

use crate::compiler_extensions::ast_nodes::{
    FerroXamlIlFerroListConstantAstNode, FerroXamlIlFontFamilyAstNode, FerroXamlIlGridLengthAstNode, FerroXamlIlVectorLikeConstantAstNode,
};
use crate::compiler_extensions::transformers::{
    EnsureCapacityNode, FerroNameScopeRegistrationXamlIlNode, FerroXamlIlWellKnownTypesExtensions, HandleRootObjectScopeNode,
    OptionsMarkupExtensionMethod, ResourceAdderSetter, XamlIlDirectCallPropertySetter,
};
use crate::compiler_extensions::{
    BindingSetter, BindingWithPrioritySetter, SetValueWithPrioritySetter, UnsetValueSetter, XamlIlBindingPathElementNode, XamlIlBindingPathNode, XamlIlFerroPropertyFieldNode,
    XamlIlFerroPropertyHelper, XamlIlFerroPropertyNode, XamlIlProvideValueTargetProperty,
};
use crate::runtime::interpreter::{context_definition, numeric_constant, plan_setters, RuntimeDocument};
use crate::runtime::type_system::{
    DeclaredMember, RuntimeConstructor, RuntimeField, RuntimeFieldValue, RuntimeInvoker, RuntimeMethod, RuntimeType,
};

use ferroui_markup_xaml::xaml_il::runtime::compiled::FRAMEWORK_CONTEXT;
use ferroui_markup_xaml::xaml_il::runtime::{DeferredContent, IFerroXamlIlXmlNamespaceInfoProvider};

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
    /// A `System.Type` value (`{x:Type}`): a class of the object model or a
    /// markup type. It has no Rust form of its own: it is written in the
    /// representation the destination declares ([`Emitter::coerce`]).
    SystemType { class: Option<&'static TypeInfo>, markup: Option<&'static MarkupType> },
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


/// `expr` as an owned value: a local is cloned (it may be used again), any other
/// expression is a temporary and is moved.
fn owned(expr: &str) -> String {
    match is_identifier(expr) {
        true => format!("::core::clone::Clone::clone(&{expr})"),
        false => expr.to_string(),
    }
}

/// Whether `text` is a plain identifier (the name of a local).
fn is_identifier(text: &str) -> bool {
    let mut characters = text.chars();
    match characters.next() {
        Some(first) if first.is_ascii_alphabetic() || first == '_' => {}
        _ => return false,
    }
    characters.all(|c| c.is_ascii_alphanumeric() || c == '_')
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

/// The metadata of a type of the run-time type system: the declaration it
/// was projected from, or, for a core type of the type system that projects
/// a declaration of its own (`System.TimeSpan`), the declaration registered
/// for its handle.
fn metadata_of(runtime: &RuntimeType) -> Option<&'static MarkupType> {
    runtime.markup().or_else(|| MarkupType::find_by_handle(runtime.handle()?.id()))
}

fn runtime_type(type_: &Rc<dyn IXamlType>) -> Option<&RuntimeType> {
    type_.as_any().downcast_ref::<RuntimeType>()
}

/// The nodes that need the parent stack or have a node below them that
/// does (the decision of the interpreter's `ParentStackVisitor`): an object
/// initialisation of such a node pushes its object onto the parent stack of
/// the context.
struct ParentStackNodes {
    nodes: HashSet<usize>,
    parents: Vec<usize>,
}

impl IXamlAstVisitor for ParentStackNodes {
    fn visit(&mut self, node: Rc<dyn IXamlAstNode>) -> XamlResult<Rc<dyn IXamlAstNode>> {
        if node.as_needs_parent_stack().is_some_and(|n| n.needs_parent_stack()) {
            self.nodes.insert(node_address(&node));
            self.nodes.extend(self.parents.iter().copied());
        }
        Ok(node)
    }
    fn push(&mut self, node: Rc<dyn IXamlAstNode>) {
        self.parents.push(node_address(&node));
    }
    fn pop(&mut self) {
        self.parents.pop();
    }
}

/// The Rust expression that yields the definition of a registered property:
/// a call of a public accessor recorded by the declaration macros
/// ([`property_accessors`]), by the public Rust path of the type whose
/// `impl` block declares it ([`rust_path_of_type`]). Every recorded accessor
/// returns the identical definition; the first one in the order of
/// [`property_accessors`] (the type the property was resolved on and its
/// base types, then the type that registered it and its base types) that
/// generated code can call is taken, so the choice depends only on the
/// declarations, never on what ran earlier on the thread.
fn property_definition(property: &'static FerroProperty, preferred: Option<&'static TypeInfo>) -> Result<String, String> {
    let accessors = property_accessors(property, preferred);
    let chosen = accessors.iter().find_map(|accessor: &PropertyAccessor| {
        accessor
            .public
            .then(|| rust_path_of_type(accessor.impl_type))
            .flatten()
            .map(|path| format!("{}::{}()", absolute(path), accessor.name))
    });
    chosen.ok_or_else(|| match accessors.is_empty() {
        true => format!("no accessor of the property {} is recorded", property.name()),
        false => format!("no accessor of the property {} is public and declared by a type with a public Rust path", property.name()),
    })
}

/// The Rust name of a primitive type the run-time type system defines
/// itself (`System.Int32` is `i32`), by its `TypeId`.
fn primitive_type_name(id: TypeId) -> Option<&'static str> {
    macro_rules! primitives {
        ($($type_:ty),*) => {
            [$((TypeId::of::<$type_>(), ::std::stringify!($type_))),*]
        };
    }
    let table = primitives!(bool, char, i8, u8, i16, u16, i32, u32, i64, u64, f32, f64, String);
    table.iter().find(|(known, _)| *known == id).map(|(_, name)| *name)
}

/// A `System.Type` value in the representation the Rust type `target`
/// declares (`RuntimeTypeValue::to_declared` of the run-time loader): an
/// untyped target takes the class reference of a class of the object model
/// and the canonical handle of any other type; `&'static TypeInfo` the class
/// reference; [`ValueType`](ferroui_base::data::core::ValueType) and
/// [`TypeId`] the canonical handle; each also as `Option<_>`. `None` if the
/// type has no such representation.
fn system_type_as(class: Option<&'static TypeInfo>, markup: Option<&'static MarkupType>, target: TypeId) -> Option<String> {
    use ferroui_base::data::core::ValueType;
    let class_expr = match class {
        Some(class) => Some(format!("<{} as ::ferroui_base::StaticType>::TYPE", absolute(class.rust_path()?))),
        None => None,
    };
    let handle_expr = match (&class_expr, markup) {
        (Some(class_expr), _) => Some(format!("rt::class_handle({class_expr})")),
        (None, Some(markup)) => {
            let path = markup.rust_path()?;
            let qualified = match markup.rust_path_is_trait() {
                true => format!("dyn {}", absolute(path)),
                false => absolute(path),
            };
            markup.handles.first()?;
            Some(format!("rt::markup_handle(<{qualified} as ::ferroui_base::metadata::MarkupTyped>::MARKUP)"))
        }
        (None, None) => None,
    };
    if target == TypeId::of::<Option<BoxedValue>>() {
        return class_expr.or(handle_expr).map(|expr| format!("rt::boxed({expr})"));
    }
    if target == TypeId::of::<&'static TypeInfo>() {
        return class_expr;
    }
    if target == TypeId::of::<Option<&'static TypeInfo>>() {
        return class_expr.map(|expr| format!("::core::option::Option::Some({expr})"));
    }
    if target == TypeId::of::<ValueType>() {
        return handle_expr;
    }
    if target == TypeId::of::<Option<ValueType>>() {
        return handle_expr.map(|expr| format!("::core::option::Option::Some({expr})"));
    }
    if target == TypeId::of::<TypeId>() {
        return handle_expr.map(|expr| format!("{expr}.id()"));
    }
    if target == TypeId::of::<Option<TypeId>>() {
        return handle_expr.map(|expr| format!("::core::option::Option::Some({expr}.id())"));
    }
    None
}

/// The values a property setter is performed with.
enum SetterValues<'a> {
    /// Only check that the statement can be written (no value is used).
    Checked,
    /// The setter takes no value (an unset-value setter).
    None,
    /// The evaluated value, of its static type, and its node.
    Typed(&'a Typed, &'a Rc<dyn IXamlAstNode>),
    /// The local holding the value in its untyped form (a choice at run
    /// time), and the node of the value.
    Untyped(&'a str, &'a Rc<dyn IXamlAstNode>),
}

/// The method of a direct call property setter (`XamlDirectCallPropertySetter`,
/// or the framework's `XamlIlDirectCallPropertySetter` of `Setter.Value`, a
/// method taking `object` called with the value of its static type), if it
/// is one of the run-time type system.
fn direct_setter_method(setter: &Rc<dyn IXamlPropertySetter>) -> Option<&RuntimeMethod> {
    let any = setter.as_any();
    let method = match any.downcast_ref::<XamlDirectCallPropertySetter>() {
        Some(direct) => direct.method(),
        None => &any.downcast_ref::<XamlIlDirectCallPropertySetter>()?.method,
    };
    method.as_any().downcast_ref::<RuntimeMethod>()
}

/// The value held untyped in `local` as the argument `index` of `member`
/// (`rt::exact`, its type inferred from where it goes).
fn untyped_argument(local: &str, member: &str, index: usize, node: &Rc<dyn IXamlAstNode>) -> String {
    format!(
        "rt::exact({local}.clone(), {}, {index}, {}, {})?",
        rust_string_literal(member),
        node.line(),
        node.position()
    )
}

struct Emitter<'a> {
    configuration: &'a TransformerConfiguration,
    document_name: &'a str,
    lines: Vec<String>,
    /// The number of locals named after each class.
    local_names: HashMap<String, usize>,
    /// The compiler locals initialised so far, by the address of their node.
    compiler_locals: HashMap<usize, (String, Kind)>,
    /// The position of the node being emitted: where a failed conversion is reported.
    position: std::cell::Cell<(i32, i32)>,
    /// The nodes that need the parent stack, by address ([`ParentStackNodes`]).
    parent_stack_nodes: HashSet<usize>,
    /// The base URI of the document (a Rust expression) and the constant of
    /// its namespace information: what every context of the document gets.
    base_uri: String,
    namespaces: String,
    /// Whether the statements written so far in the current scope (the
    /// build function, or the body of deferred content) use the context and
    /// the name scope.
    uses_context: bool,
    uses_name_scope: bool,
    /// The property assignments being emitted, the innermost last: the
    /// property a markup extension provides its value for is the one of the
    /// innermost (the interpreter's nearest parent assignment node).
    assignments: Vec<Rc<XamlPropertyAssignmentNode>>,
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

    /// `value` held in a local: the expression is evaluated exactly once, here, and every use
    /// of the result names the local. A value that already is a local is returned as it is.
    fn bind(&mut self, value: &Typed, base: &str) -> Typed {
        if is_identifier(&value.expr) {
            return Typed { expr: value.expr.clone(), kind: value.kind };
        }
        let local = self.local_named(base);
        self.line(format!("let {local} = {};", value.expr));
        Typed { expr: local, kind: value.kind }
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
                    // The value is evaluated once; the manipulation and the consumer use it.
                    let created = self.bind(&created, "value");
                    self.manipulation(&manipulation.as_node(), &created)?;
                    return Ok(created);
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
        if let Some(n) = node.cast::<XamlMarkupExtensionNode>() {
            return self.markup_extension(node, &n);
        }
        if let Some(n) = node.cast::<XamlDeferredContentNode>() {
            return self.deferred_content(node, &n);
        }
        if let Some(n) = node.cast::<XamlDeferredContentInitializeIntermediateRootNode>() {
            // The root of the deferred content: created, then the intermediate root of the context.
            let value = self.value(&n.value().as_node())?;
            let untyped = match value.kind {
                Kind::Class(_) => format!("rt::to_value({}.clone())", value.expr),
                Kind::Exact { .. } => format!("rt::to_value({})", value.expr),
                Kind::Null | Kind::SystemType { .. } => {
                    return Err(unsupported(node, "an intermediate root that is not an object"));
                }
            };
            self.uses_context = true;
            self.line(format!("context.set_intermediate_root_object({untyped});"));
            return Ok(value);
        }
        if let Some(n) = node.cast::<XamlIlBindingPathNode>() {
            return self.binding_path(node, &n);
        }
        if let Some(n) = node.cast::<FerroXamlIlFontFamilyAstNode>() {
            // `new FontFamily(context.BaseUri, text)`.
            self.uses_context = true;
            let base_uri = Typed {
                expr: "context.base_uri()".to_string(),
                kind: Kind::Exact { id: TypeId::of::<Option<ferroui_base::utilities::Uri>>(), nullable: None },
            };
            let text = exact::<String>(format!("::std::string::String::from({})", rust_string_literal(n.text())));
            let constructor = n.types().font_family_constructor_uri_name.clone();
            return self.constructor_call(node, &constructor, &[base_uri, text]);
        }
        if let Some(n) = node.cast::<XamlIlFerroPropertyNode>() {
            let field = n.resolve_ferro_property_field().map_err(|e| failed(node, e))?;
            return self.property_field(node, &field);
        }
        if let Some(n) = node.cast::<XamlIlFerroPropertyFieldNode>() {
            return self.property_field(node, n.field());
        }
        if let Some(n) = node.cast::<XamlTypeExtensionNode>() {
            // `typeof(T)`: the type, written where it goes.
            let type_ = n.value().get_clr_type().map_err(|e| failed(node, e))?;
            let runtime = runtime_type(&type_).ok_or_else(|| unsupported(node, "a type that is not one of the run-time type system"))?;
            let class = runtime.type_info();
            let markup = match class {
                Some(_) => None,
                None => metadata_of(runtime),
            };
            if class.is_none() && markup.is_none() {
                return Err(unsupported(node, format!("{} has no metadata", runtime.full_name())));
            }
            return Ok(Typed { expr: String::new(), kind: Kind::SystemType { class, markup } });
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
        // The list is created once (`newobj`, then `dup` for each call): it is held in a local
        // that the capacity, every `Add` and the consumer use.
        let list = self.bind(&list, &snake_case(&list_type.name()));
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
            return self.flags_value(node, markup, value);
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

    /// A value of a set of flags from its integer value: the typed function
    /// the declaration generates (`__markup_flags`), for a value the members
    /// of the flags make up (as the metadata of the flags converts it).
    fn flags_value(&mut self, node: &Rc<dyn IXamlAstNode>, markup: &'static MarkupType, value: i64) -> EmitResult<Typed> {
        let composes = markup.enum_from_value.is_some_and(|from_value| from_value(value).is_some());
        if !composes {
            return Err(unsupported(node, format!("{value} is not made of members of {}", markup.full_name())));
        }
        let path = markup
            .rust_path()
            .ok_or_else(|| unsupported(node, format!("no public Rust path is recorded for {}", markup.full_name())))?;
        let id = markup.handle().ok_or_else(|| unsupported(node, "the flags have no value type"))?.id();
        let nullable = markup.nullable.map(|nullable| nullable().id());
        Ok(Typed { expr: format!("{}::__markup_flags({value}_i64)", absolute(path)), kind: Kind::Exact { id, nullable } })
    }

    /// `{x:Static Type.Member}` naming a member of an enumeration, a static
    /// property or field.
    fn static_member(&mut self, node: &Rc<dyn IXamlAstNode>, n: &Rc<XamlStaticExtensionNode>) -> EmitResult<Typed> {
        let member = n.resolve_member(true).map_err(|e| failed(node, e))?;
        let field = match member {
            Some(XamlStaticMember::Field(field)) => field,
            Some(XamlStaticMember::Property(property)) => {
                // The static getter, called through its typed function.
                let getter = property
                    .getter()
                    .ok_or_else(|| unsupported(node, format!("{}: a static property without a getter", property.name())))?;
                let getter = getter
                    .as_any()
                    .downcast_ref::<RuntimeMethod>()
                    .ok_or_else(|| unsupported(node, "a getter that is not a method of the run-time type system"))?;
                let call = self.declared_call(node, getter, &[])?;
                let returned = self.declared_return(node, getter)?;
                let local = self.local_named("value");
                self.line(format!("let {local} = {call};"));
                return Ok(Typed { expr: local, kind: self.kind_of(returned) });
            }
            None => return Err(unsupported(node, "a static member that does not resolve")),
        };
        let Some(runtime) = field.as_any().downcast_ref::<RuntimeField>() else {
            return Err(unsupported(node, "a field that is not a field of the run-time type system"));
        };
        if runtime.ferro_property().is_some() {
            return self.property_field(node, &field);
        }
        if let RuntimeFieldValue::Declared(declared) = runtime.value {
            // A field of metadata, read through its typed function.
            let emit = declared
                .emit
                .ok_or_else(|| unsupported(node, format!("{}: the declaration has no typed function", declared.name)))?;
            let declaring = runtime
                .declaring_type
                .upgrade()
                .ok_or_else(|| unsupported(node, format!("{}: the declaring type is gone", declared.name)))?;
            let owner = self.type_path(node, &declaring)?;
            let local = self.local_named("value");
            self.line(format!("let {local} = {owner}::{}();", emit.function));
            return Ok(Typed { expr: local, kind: self.kind_of((declared.type_)().id()) });
        }
        if !matches!(runtime.value, RuntimeFieldValue::EnumMember(_)) {
            return Err(unsupported(node, "a static field that is not an enumeration member"));
        }
        let declaring_type = field.declaring_type();
        let markup = runtime_type(&declaring_type)
            .and_then(RuntimeType::markup)
            .ok_or_else(|| unsupported(node, format!("{} has no metadata", declaring_type.full_name())))?;
        let name = field.name();
        if markup.is_flags {
            let member = markup
                .enum_members
                .iter()
                .find(|member| member.name == name)
                .ok_or_else(|| unsupported(node, format!("{name} is not a member of {}", declaring_type.full_name())))?;
            return self.flags_value(node, markup, member.value);
        }
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
            self.position.set((node.line(), node.position()));
            texts.push(self.coerce(argument, handle.id()).ok_or_else(|| {
                unsupported(node, format!("argument {index} of the constructor cannot be stated as `{}`", handle.name()))
            })?);
        }
        let value = metadata_of(&declaring)
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
            Kind::Exact { id, .. } => (id, owned(&value.expr)),
            Kind::Class(class) => (class.handle()?, format!("{}.clone()", value.expr)),
            Kind::Null | Kind::SystemType { .. } => return None,
        };
        ValueTypes::is_assignable(ValueType::new(from, ""), ValueType::new(target, ""))
            .then(|| format!("rt::cast({expr}, {}, {})?", self.position.get().0, self.position.get().1))
    }

    /// A value of the type with markup metadata `from` as the handle of a contract
    /// (`Rc<dyn Trait>`, or its nullable form) that the declaration of `from` (or of one of
    /// its base types) lists among its interfaces: an unsizing coercion rustc checks.
    fn coerce_to_contract(&self, from: TypeId, owned: &str, target: TypeId) -> Option<String> {
        let source = MarkupType::find_by_handle(from)?;
        let contract = MarkupType::find_by_handle(target)?;
        let handle = contract.handle()?.id();
        let nullable = match target {
            _ if target == handle => false,
            _ if contract.handles.get(1).is_some_and(|second| second().id() == target) => true,
            _ => return None,
        };
        if !contract.rust_path_is_trait() {
            return None;
        }
        let path = absolute(contract.rust_path()?);
        let mut current = Some(source);
        let mut declared = false;
        while let Some(type_) = current {
            declared |= type_.interfaces.iter().any(|interface| interface().id() == handle);
            current = type_.base_type();
        }
        if !declared {
            return None;
        }
        // The value is bound first: the expected type of a binding would otherwise choose
        // the `Clone` implementation of the contract handle for a cloned local.
        let coerced = format!("{{ let value = {owned}; let value: ::std::rc::Rc<dyn {path}> = value; value }}");
        Some(if nullable { format!("::core::option::Option::Some({coerced})") } else { coerced })
    }

    fn coerce_static(&self, value: &Typed, target: TypeId) -> Option<String> {
        let object = TypeId::of::<Option<BoxedValue>>();
        match value.kind {
            Kind::Exact { id, nullable } => {
                let owned = owned(&value.expr);
                if target == id {
                    Some(owned)
                } else if nullable == Some(target) {
                    Some(format!("::core::option::Option::Some({owned})"))
                } else if target == object {
                    Some(format!("rt::to_object({owned})"))
                } else {
                    self.coerce_to_contract(id, &owned, target)
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
            Kind::SystemType { class, markup } => system_type_as(class, markup, target),
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
            self.assignments.push(n.clone());
            let result = self.property_assignment(node, &n, target);
            self.assignments.pop();
            return result;
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
        if let Some(n) = node.cast::<EnsureCapacityNode>() {
            return self.ensure_capacity(node, &n, target);
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

    /// `BeginInit`, the manipulation (with the object on the parent stack of
    /// the context while it runs, if the language keeps a parent stack, the
    /// object is not a value and a node below needs the stack), `EndInit`.
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
        let push_parent = FRAMEWORK_CONTEXT.parent_stack_provider
            && !type_.is_value_type()
            && self.parent_stack_nodes.contains(&node_address(node));
        if push_parent {
            self.uses_context = true;
            self.line(format!("context.push_parent(rt::to_value({}.clone()));", target.expr));
        }
        self.manipulation(&init.manipulation().as_node(), target)?;
        if push_parent {
            self.line("context.pop_parent();".to_string());
        }
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

    /// A property assignment, with the setter the interpreter's plan
    /// decides: one setter is always used; of several, the first that takes
    /// the run-time value of the last value is ([`Self::dynamic_assignment`]).
    fn property_assignment(
        &mut self,
        node: &Rc<dyn IXamlAstNode>,
        assignment: &Rc<XamlPropertyAssignmentNode>,
        target: &Typed,
    ) -> EmitResult<()> {
        let property_name = assignment.property.name();
        let (setters, value_types) = plan_setters(node, assignment).map_err(|e| failed(node, e))?;
        let setter = match setters.as_slice() {
            [setter] => setter.clone(),
            _ => return self.dynamic_assignment(node, assignment, &setters, &value_types, target),
        };
        if let Some(adder) = setter.as_any().downcast_ref::<AdderSetter>() {
            return self.adder_assignment(node, assignment, adder, target);
        }
        if let Some(adder) = setter.as_any().downcast_ref::<ResourceAdderSetter>() {
            return self.resource_assignment(node, assignment, adder, target);
        }
        // The value of an unset-value assignment is not evaluated at all.
        if setter.as_any().is::<UnsetValueSetter>() {
            self.marker(node, &format!("{property_name} (unset)"));
            let statement = self.setter_statement(node, assignment, &setter, target, &SetterValues::None, None)?;
            self.line(statement);
            return Ok(());
        }
        if let Some(runtime) = direct_setter_method(&setter) {
            if runtime.declared().is_some() {
                return self.declared_setter_assignment(node, assignment, runtime, target);
            }
        }
        let values = assignment.values.borrow().clone();
        // `(priority, value)`: a binding with a priority does not evaluate the priority at all;
        // a value with a priority evaluates the value, then the priority.
        if let [priority_node, value_node] = values.as_slice() {
            let value_node = value_node.as_node();
            let priority_node = priority_node.as_node();
            let with_priority = setter.as_any().is::<SetValueWithPrioritySetter>();
            if !with_priority && !setter.as_any().is::<BindingWithPrioritySetter>() {
                return Err(unsupported(node, format!("{property_name}: an assignment with 2 values")));
            }
            self.setter_statement(node, assignment, &setter, target, &SetterValues::Checked, Some(""))?;
            self.marker(node, &property_name);
            let value = self.value(&value_node)?;
            let priority = match with_priority {
                true => Some(self.priority_value(&priority_node)?),
                false => None,
            };
            let statement = self.setter_statement(
                node,
                assignment,
                &setter,
                target,
                &SetterValues::Typed(&value, &value_node),
                priority.as_deref(),
            )?;
            self.line(statement);
            return Ok(());
        }
        let [value_node] = values.as_slice() else {
            return Err(unsupported(node, format!("{property_name}: an assignment with {} values", values.len())));
        };
        let value_node = value_node.as_node();
        // What is not supported is known before anything is written.
        self.setter_statement(node, assignment, &setter, target, &SetterValues::Checked, None)?;
        self.marker(node, &property_name);
        let value = self.value(&value_node)?;
        let statement = self.setter_statement(node, assignment, &setter, target, &SetterValues::Typed(&value, &value_node), None)?;
        self.line(statement);
        Ok(())
    }

    /// The priority of an assignment with a priority, held in a local.
    fn priority_value(&mut self, node: &Rc<dyn IXamlAstNode>) -> EmitResult<String> {
        let priority = self.value(node)?;
        let text = self
            .coerce(&priority, TypeId::of::<ferroui_base::data::BindingPriority>())
            .ok_or_else(|| unsupported(node, "a priority that is not a binding priority"))?;
        let local = self.local_named("priority");
        self.line(format!("let {local} = {text};"));
        Ok(local)
    }

    /// The statement a property setter performs with the value (`values`):
    /// `target.set_value(definition, value)` for the accessor the type system
    /// builds for a registered property, a call of the typed function of a
    /// declared setter, `rt::bind` for a binding setter, `rt::unset_value`
    /// for an unset-value setter.
    fn setter_statement(
        &self,
        node: &Rc<dyn IXamlAstNode>,
        assignment: &XamlPropertyAssignmentNode,
        setter: &Rc<dyn IXamlPropertySetter>,
        target: &Typed,
        values: &SetterValues<'_>,
        priority: Option<&str>,
    ) -> EmitResult<String> {
        let property_name = assignment.property.name();
        // The setters of registered properties act on the property store of an object.
        let object_target = || match target.kind {
            Kind::Class(_) => Ok(()),
            _ => Err(unsupported(node, format!("{property_name}: the target is not an object of the object model"))),
        };
        let any = setter.as_any();
        if let Some(unset) = any.downcast_ref::<UnsetValueSetter>() {
            object_target()?;
            let definition = self.registered_definition(node, &property_name, &unset.ferro_property())?;
            return Ok(format!("rt::unset_value(&{}.clone().upcast::<::ferroui_base::FerroObject>(), {definition});", target.expr));
        }
        let binding_field = match (any.downcast_ref::<BindingSetter>(), any.downcast_ref::<BindingWithPrioritySetter>()) {
            (Some(binding), _) => Some(binding.ferro_property()),
            // The priority is discarded: a binding decides its own priority.
            (None, Some(binding)) => Some(binding.ferro_property()),
            (None, None) => None,
        };
        if let Some(set) = any.downcast_ref::<SetValueWithPrioritySetter>() {
            object_target()?;
            let field = set.ferro_property();
            let definition = self.registered_definition(node, &property_name, &field)?;
            let property = field
                .as_any()
                .downcast_ref::<RuntimeField>()
                .and_then(RuntimeField::ferro_property)
                .ok_or_else(|| unsupported(node, format!("{property_name}: not a registered property")))?;
            if property.is_direct() {
                return Err(unsupported(node, format!("{property_name}: a direct property set with a priority")));
            }
            let value = match values {
                SetterValues::Typed(value, value_node) => self.coerce(value, property.property_type()).ok_or_else(|| {
                    unsupported(
                        value_node,
                        format!("{property_name}: the value cannot be stated as `{}`", property.property_type_name()),
                    )
                })?,
                SetterValues::Untyped(local, value_node) => {
                    untyped_argument(local, &format!("{}.{}", property.owner_type().name(), property.name()), 1, value_node)
                }
                SetterValues::Checked => return Ok(String::new()),
                SetterValues::None => return Err(unsupported(node, format!("{property_name}: a setter without its value"))),
            };
            let priority = priority.ok_or_else(|| unsupported(node, format!("{property_name}: a priority setter without a priority")))?;
            return Ok(format!("{}.set_value_with_priority({definition}, {value}, {priority});", target.expr));
        }
        if let Some(binding_field) = binding_field {
            object_target()?;
            let definition = self.registered_definition(node, &property_name, &binding_field)?;
            let (value, line, position) = match values {
                SetterValues::Typed(value, value_node) => {
                    let expr = match value.kind {
                        Kind::Class(_) => format!("{}.clone()", value.expr),
                        Kind::SystemType { .. } => self
                            .coerce(value, TypeId::of::<Option<BoxedValue>>())
                            .ok_or_else(|| unsupported(value_node, "a type without a run-time representation"))?,
                        _ => value.expr.clone(),
                    };
                    (expr, value_node.line(), value_node.position())
                }
                SetterValues::Untyped(local, value_node) => (format!("{local}.clone()"), value_node.line(), value_node.position()),
                SetterValues::Checked | SetterValues::None => (String::new(), 0, 0),
            };
            return Ok(format!(
                "rt::bind(&{}.clone().upcast::<::ferroui_base::FerroObject>(), {definition}, {value}, {line}, {position})?;",
                target.expr
            ));
        }
        let runtime = direct_setter_method(setter)
            .ok_or_else(|| unsupported(node, format!("{property_name}: not a plain property setter")))?;
        if runtime.declared().is_some() {
            // A declared setter in a choice at run time: the value converted to the declared type.
            let mut arguments = vec![Typed { expr: target.expr.clone(), kind: target.kind }];
            match values {
                SetterValues::Untyped(local, value_node) => {
                    let handle = runtime
                        .parameter_handles
                        .last()
                        .copied()
                        .flatten()
                        .ok_or_else(|| unsupported(node, format!("{property_name}: a parameter without a Rust type")))?;
                    let member = self.member_name(runtime);
                    arguments.push(Typed {
                        expr: untyped_argument(local, &member, runtime.parameters.len() - 1, value_node),
                        kind: Kind::Exact { id: handle.id(), nullable: None },
                    });
                }
                SetterValues::Typed(value, _) => arguments.push(Typed { expr: value.expr.clone(), kind: value.kind }),
                SetterValues::Checked => return Ok(String::new()),
                SetterValues::None => return Err(unsupported(node, format!("{property_name}: a setter without its value"))),
            }
            return Ok(format!("{};", self.declared_call(node, runtime, &arguments)?));
        }
        object_target()?;
        let (property, definition) = self.registered_accessor(node, assignment, runtime)?;
        let value = match values {
            SetterValues::Typed(value, value_node) => self.coerce(value, property.property_type()).ok_or_else(|| {
                unsupported(
                    value_node,
                    format!("{property_name}: the value cannot be stated as `{}`", property.property_type_name()),
                )
            })?,
            SetterValues::Untyped(local, value_node) => untyped_argument(local, &self.member_name(runtime), 0, value_node),
            SetterValues::Checked => return Ok(String::new()),
            SetterValues::None => return Err(unsupported(node, format!("{property_name}: a setter without its value"))),
        };
        let call = if property.is_direct() { "set_direct_value" } else { "set_value" };
        Ok(format!("{}.{call}({definition}, {value});", target.expr))
    }

    /// `Type.Member`, as the loader names a member in an argument error.
    fn member_name(&self, method: &RuntimeMethod) -> String {
        match method.declaring_type.upgrade() {
            Some(declaring) => format!("{}.{}", declaring.full_name(), method.name),
            None => method.name.clone(),
        }
    }

    /// The definition of the registered property in the static field
    /// `field` of a custom setter.
    fn registered_definition(&self, node: &Rc<dyn IXamlAstNode>, property_name: &str, field: &Rc<dyn IXamlField>) -> EmitResult<String> {
        let property = field
            .as_any()
            .downcast_ref::<RuntimeField>()
            .and_then(RuntimeField::ferro_property)
            .ok_or_else(|| unsupported(node, format!("{property_name}: not a registered property")))?;
        let declaring_type = field.declaring_type();
        property_definition(property, runtime_type(&declaring_type).and_then(RuntimeType::type_info))
            .map_err(|reason| unsupported(node, format!("{property_name}: {reason}")))
    }

    /// The registered property `runtime` is the accessor of (the setter the
    /// type system builds for it, `set_value_untyped(property, value,
    /// LocalValue)`, not a declared one), with the expression of its
    /// definition.
    fn registered_accessor(
        &self,
        node: &Rc<dyn IXamlAstNode>,
        assignment: &XamlPropertyAssignmentNode,
        runtime: &RuntimeMethod,
    ) -> EmitResult<(&'static FerroProperty, String)> {
        let property_name = assignment.property.name();
        let field = XamlIlFerroPropertyHelper::try_get_ferro_property_field(&assignment.property)
            .ok_or_else(|| unsupported(node, format!("{property_name}: not a registered property")))?;
        let property = field
            .as_any()
            .downcast_ref::<RuntimeField>()
            .and_then(RuntimeField::ferro_property)
            .ok_or_else(|| unsupported(node, format!("{property_name}: not a registered property")))?;
        let expected_name = match runtime.is_static {
            true => format!("Set{}", property.name()),
            false => format!("set_{}", property.name()),
        };
        let expected_parameters = if runtime.is_static { 2 } else { 1 };
        let declares = runtime.declaring_type.upgrade().is_some_and(|declaring| declaring.equals(&*field.declaring_type()));
        let is_plain_accessor = matches!(runtime.invoker, RuntimeInvoker::Dynamic(_))
            && runtime.name == expected_name
            && runtime.parameters.len() == expected_parameters
            && declares;
        if !is_plain_accessor {
            return Err(unsupported(node, format!("{property_name}: the setter is a declared member")));
        }
        if property.is_read_only() {
            return Err(unsupported(node, format!("{property_name}: a read-only property")));
        }
        let declaring_type = field.declaring_type();
        let definition = property_definition(property, runtime_type(&declaring_type).and_then(RuntimeType::type_info))
            .map_err(|reason| unsupported(node, format!("{property_name}: {reason}")))?;
        Ok((property, definition))
    }

    /// An assignment whose setter is chosen at run time (the dynamic setter
    /// of the interpreter's `property_assignment`): the value in its untyped
    /// form, then each setter in order of the plan, taken if the value is an
    /// instance of its parameter type (not checked where the static type of
    /// the value proves it; a non-null value where the setter refuses null),
    /// then the first setter that allows null for a null value, else the
    /// loader's error.
    fn dynamic_assignment(
        &mut self,
        node: &Rc<dyn IXamlAstNode>,
        assignment: &Rc<XamlPropertyAssignmentNode>,
        setters: &[Rc<dyn IXamlPropertySetter>],
        value_types: &[Rc<dyn IXamlType>],
        target: &Typed,
    ) -> EmitResult<()> {
        let property_name = assignment.property.name();
        let values = assignment.values.borrow().clone();
        // One value, or a priority and a value (the setters with a priority).
        let (priority_node, value_node, dynamic_type) = match (values.as_slice(), value_types) {
            ([value_node], [dynamic_type]) => (None, value_node.as_node(), dynamic_type.clone()),
            ([priority_node, value_node], [_, dynamic_type]) => {
                (Some(priority_node.as_node()), value_node.as_node(), dynamic_type.clone())
            }
            _ => {
                return Err(unsupported(
                    node,
                    format!("{property_name}: a choice at run time among setters of {} values", values.len()),
                ));
            }
        };
        let dynamic_type = &dynamic_type;
        // The branches, decided before anything is written.
        let mut branches: Vec<(Option<String>, Rc<dyn IXamlPropertySetter>)> = Vec::with_capacity(setters.len());
        let mut first_allowing_null: Option<Rc<dyn IXamlPropertySetter>> = None;
        for setter in setters {
            let allows_null = setter.binder_parameters().allow_runtime_null.get();
            if allows_null && first_allowing_null.is_none() {
                first_allowing_null = Some(setter.clone());
            }
            let parameter = setter
                .parameters()
                .last()
                .cloned()
                .ok_or_else(|| unsupported(node, format!("{property_name}: a setter without a value parameter")))?;
            let condition = if !parameter.is_assignable_from(&**dynamic_type) {
                let checked = match parameter.is_nullable() {
                    true => parameter.generic_arguments().into_iter().next().unwrap_or_else(|| parameter.clone()),
                    false => parameter.clone(),
                };
                Some(format!("rt::is_instance(&{{value}}, {})", self.handle_expr(node, &checked)?))
            } else if !allows_null {
                Some("{value}.is_some()".to_string())
            } else {
                None
            };
            let checked_priority = priority_node.as_ref().map(|_| "");
            self.setter_statement(node, assignment, setter, target, &SetterValues::Checked, checked_priority)?;
            branches.push((condition, setter.clone()));
        }
        self.marker(node, &format!("{property_name} (setter chosen at run time)"));
        // Every value is evaluated, in order, before the choice.
        let priority = match &priority_node {
            Some(priority_node) => Some(self.priority_value(priority_node)?),
            None => None,
        };
        let value = self.value(&value_node)?;
        self.position.set((node.line(), node.position()));
        let local = self.local_named("value");
        let untyped = match value.kind {
            Kind::SystemType { .. } => {
                return Err(unsupported(&value_node, format!("{property_name}: a type value in a choice of the setter at run time")));
            }
            Kind::Null => "::core::option::Option::None".to_string(),
            Kind::Class(_) => format!("rt::to_value({}.clone())", value.expr),
            Kind::Exact { .. } => format!("rt::to_value({})", value.expr),
        };
        self.line(format!("let {local}: ::ferroui_base::metadata::MarkupValue = {untyped};"));
        let mut lines: Vec<String> = Vec::new();
        let mut closed = false;
        for (index, (condition, setter)) in branches.iter().enumerate() {
            let statement = self.setter_statement(node, assignment, setter, target, &SetterValues::Untyped(&local, &value_node), priority.as_deref())?;
            match (condition, index) {
                (Some(condition), 0) => lines.push(format!("if {} {{", condition.replace("{value}", &local))),
                (Some(condition), _) => lines.push(format!("}} else if {} {{", condition.replace("{value}", &local))),
                (None, 0) => lines.push("{".to_string()),
                (None, _) => lines.push("} else {".to_string()),
            }
            lines.push(format!("    {statement}"));
            if condition.is_none() {
                closed = true;
                break;
            }
        }
        if !closed {
            if let Some(setter) = &first_allowing_null {
                let statement = self.setter_statement(node, assignment, setter, target, &SetterValues::Untyped(&local, &value_node), priority.as_deref())?;
                lines.push(format!("}} else if {local}.is_none() {{"));
                lines.push(format!("    {statement}"));
            }
            lines.push("} else {".to_string());
            lines.push(format!(
                "    return ::core::result::Result::Err(rt::no_setter({}, &{local}, {}, {}));",
                rust_string_literal(&property_name),
                node.line(),
                node.position()
            ));
        }
        lines.push("}".to_string());
        for line in lines {
            self.line(line);
        }
        Ok(())
    }

    /// The expression of the canonical handle of a type (`rt::class_handle`
    /// of a class, `rt::markup_handle` of a markup type), for a run-time
    /// type check.
    fn handle_expr(&self, node: &Rc<dyn IXamlAstNode>, type_: &Rc<dyn IXamlType>) -> EmitResult<String> {
        let runtime = runtime_type(type_).ok_or_else(|| unsupported(node, format!("{} is not a type of the run-time type system", type_.get_fqn())))?;
        let handle = runtime.handle().ok_or_else(|| unsupported(node, format!("{} has no handle", runtime.full_name())))?;
        if let Some(primitive) = primitive_type_name(handle.id()) {
            return Ok(format!("::ferroui_base::data::core::ValueType::of::<{primitive}>()"));
        }
        if let Some(class) = runtime.type_info() {
            let path = class
                .rust_path()
                .ok_or_else(|| unsupported(node, format!("no public Rust path is recorded for {}", runtime.full_name())))?;
            if class.handle() != Some(handle.id()) {
                return Err(unsupported(node, format!("the handle of {} is not the handle of its class", runtime.full_name())));
            }
            return Ok(format!("rt::class_handle(<{} as ::ferroui_base::StaticType>::TYPE)", absolute(path)));
        }
        let markup = metadata_of(runtime).ok_or_else(|| unsupported(node, format!("{} has no metadata", runtime.full_name())))?;
        if markup.handles.first().map(|first| first().id()) != Some(handle.id()) {
            return Err(unsupported(node, format!("the handle of {} is not the first handle of its metadata", runtime.full_name())));
        }
        let path = markup
            .rust_path()
            .ok_or_else(|| unsupported(node, format!("no public Rust path is recorded for {}", runtime.full_name())))?;
        let qualified = match markup.rust_path_is_trait() {
            true => format!("dyn {}", absolute(path)),
            false => absolute(path),
        };
        Ok(format!("rt::markup_handle(<{qualified} as ::ferroui_base::metadata::MarkupTyped>::MARKUP)"))
    }

    /// `dictionary.Add(key, value)` of a resource (`ResourceAdderSetter`):
    /// the dictionary (the getter called on the target, or the target
    /// itself), then the key and the value, then the call of the adder.
    fn resource_assignment(
        &mut self,
        node: &Rc<dyn IXamlAstNode>,
        assignment: &Rc<XamlPropertyAssignmentNode>,
        setter: &ResourceAdderSetter,
        target: &Typed,
    ) -> EmitResult<()> {
        let property_name = assignment.property.name();
        if setter.emit_source_info {
            return Err(unsupported(node, format!("{property_name}: a resource with source information")));
        }
        let adder = setter
            .adder
            .as_any()
            .downcast_ref::<RuntimeMethod>()
            .ok_or_else(|| unsupported(node, format!("{property_name}: the adder is not a method of the run-time type system")))?;
        let values = assignment.values.borrow().clone();
        let [key_node, value_node] = values.as_slice() else {
            return Err(unsupported(node, format!("{property_name}: a resource with {} values", values.len())));
        };
        self.marker(node, &format!("{property_name} (resource)"));
        let dictionary = match &setter.getter {
            Some(getter) => {
                let getter = getter
                    .as_any()
                    .downcast_ref::<RuntimeMethod>()
                    .ok_or_else(|| unsupported(node, format!("{property_name}: the getter is not a method of the run-time type system")))?;
                let (call, dictionary_type) = match getter.declared() {
                    Some(_) => (self.declared_call(node, getter, std::slice::from_ref(target))?, self.declared_return(node, getter)?),
                    None => self.registered_getter(node, assignment, getter, target)?,
                };
                let local = self.local_named("dictionary");
                self.line(format!("let {local} = {call};"));
                Typed { expr: local, kind: self.kind_of(dictionary_type) }
            }
            None => Typed { expr: target.expr.clone(), kind: target.kind },
        };
        let key = self.value(&key_node.as_node())?;
        let value = self.value(&value_node.as_node())?;
        let call = self.declared_call(node, adder, &[dictionary, key, value])?;
        self.line(format!("{call};"));
        Ok(())
    }

    /// Deferred content (`XamlDeferredContentNode`): the build function as a
    /// closure that creates its own context (chained to the service provider
    /// it is called with, the root object of that provider as its root) and
    /// builds the value anew, given with the context of the declaration to
    /// the deferred content customisation of the language
    /// (`DeferredTransformationFactoryV3<T>`, `rt::deferred_content`).
    fn deferred_content(&mut self, node: &Rc<dyn IXamlAstNode>, deferred: &Rc<XamlDeferredContentNode>) -> EmitResult<Typed> {
        let customization = deferred
            .deferred_content_customization()
            .ok_or_else(|| unsupported(node, "deferred content without the customisation of the language"))?;
        if customization.name() != "DeferredTransformationFactoryV3" {
            return Err(unsupported(node, format!("the deferred content customisation {}", customization.name())));
        }
        // The handle of the type argument; the "any value" type without one (or for `object`).
        let object = "::ferroui_base::data::core::ValueType::object()".to_string();
        let result_type = match deferred.deferred_content_customization_type_parameter() {
            Some(type_) => match runtime_type(type_).and_then(RuntimeType::handle) {
                Some(handle) if handle.is_object() => object,
                Some(_) => self.handle_expr(node, type_)?,
                None => object,
            },
            None => object,
        };
        // The body: a scope of its own (the interpreter evaluates it with a new evaluation).
        let outer_lines = std::mem::take(&mut self.lines);
        let outer_assignments = std::mem::take(&mut self.assignments);
        let outer_uses = (std::mem::take(&mut self.uses_context), std::mem::take(&mut self.uses_name_scope));
        let body = self.deferred_body(deferred);
        let body_lines = std::mem::replace(&mut self.lines, outer_lines);
        self.assignments = outer_assignments;
        let (uses_context, uses_name_scope) = (self.uses_context || self.uses_name_scope, self.uses_name_scope);
        (self.uses_context, self.uses_name_scope) = outer_uses;
        let returned = body?;

        // The context of the declaration is given to the customisation.
        self.uses_context = true;
        let context_name = if uses_context { "context" } else { "_context" };
        let local = self.local_named("deferred");
        self.line(format!("let {local} = rt::deferred_content("));
        self.line(format!("    {result_type},"));
        self.line("    &context,".to_string());
        self.line("    rt::deferred_builder(move |service_provider| {".to_string());
        self.line(format!(
            "        let {context_name} = rt::deferred_context(service_provider, {}, {});",
            self.base_uri, self.namespaces
        ));
        if uses_name_scope {
            self.line("        let name_scope = context.name_scope_field();".to_string());
        }
        for line in body_lines {
            self.lines.push(format!("        {line}"));
        }
        self.line(format!("        ::core::result::Result::Ok({returned})"));
        self.line("    }),".to_string());
        self.line(format!("    {},", node.line()));
        self.line(format!("    {},", node.position()));
        self.line(")?;".to_string());
        Ok(Typed { expr: local, kind: Kind::Exact { id: TypeId::of::<Rc<DeferredContent>>(), nullable: None } })
    }

    /// The statements of the body of deferred content and the value it
    /// returns (the value as an object, as the interpreter evaluates it).
    fn deferred_body(&mut self, deferred: &XamlDeferredContentNode) -> EmitResult<String> {
        let value_node = deferred.value().as_node();
        let value = self.value(&value_node)?;
        Ok(match value.kind {
            Kind::Null => "::core::option::Option::None".to_string(),
            Kind::Class(_) => format!("rt::to_value({}.clone())", value.expr),
            Kind::Exact { .. } => format!("rt::to_value({})", value.expr),
            Kind::SystemType { .. } => {
                return Err(unsupported(&value_node, "deferred content that is a type"));
            }
        })
    }

    /// The value of the static field holding the definition of a registered
    /// property: the definition, as `&'static FerroProperty` (`rt::property`).
    fn property_field(&mut self, node: &Rc<dyn IXamlAstNode>, field: &Rc<dyn IXamlField>) -> EmitResult<Typed> {
        let definition = self.registered_definition(node, &field.name(), field)?;
        Ok(Typed {
            expr: format!("rt::property({definition})"),
            kind: Kind::Exact { id: TypeId::of::<&'static FerroProperty>(), nullable: None },
        })
    }

    /// `EnsureCapacityNode`: the resources (the getter called on the target,
    /// or the target), and if they are a resource dictionary, its capacity
    /// raised to its count plus the number of resources the document adds.
    fn ensure_capacity(&mut self, node: &Rc<dyn IXamlAstNode>, ensure: &EnsureCapacityNode, target: &Typed) -> EmitResult<()> {
        let types = self.configuration.try_get_ferro_types().map_err(|e| failed(node, e))?;
        let get_count = types
            .resource_dictionary_get_count
            .as_any()
            .downcast_ref::<RuntimeMethod>()
            .ok_or_else(|| unsupported(node, "ResourceDictionary.get_Count is not a method of the run-time type system"))?;
        let ensure_method = types
            .resource_dictionary_ensure_capacity
            .as_any()
            .downcast_ref::<RuntimeMethod>()
            .ok_or_else(|| unsupported(node, "ResourceDictionary.EnsureCapacity is not a method of the run-time type system"))?;
        let resources = match &ensure.resources_getter {
            Some(getter) => {
                let getter = getter
                    .as_any()
                    .downcast_ref::<RuntimeMethod>()
                    .ok_or_else(|| unsupported(node, "the getter of the resources is not a method of the run-time type system"))?;
                if getter.declared().is_none() {
                    return Err(unsupported(node, "the getter of the resources is not a declared member"));
                }
                let call = self.declared_call(node, getter, std::slice::from_ref(target))?;
                Typed { expr: call, kind: self.kind_of(self.declared_return(node, getter)?) }
            }
            None => Typed { expr: target.expr.clone(), kind: target.kind },
        };
        let dictionary_type = &types.resource_dictionary;
        let handle = self.handle_expr(node, dictionary_type)?;
        let this = get_count
            .declaring_type
            .upgrade()
            .and_then(|declaring| metadata_of(&declaring))
            .and_then(|markup| markup.this)
            .ok_or_else(|| unsupported(node, "the instance type of ResourceDictionary is not known"))?;
        let local = self.local_named("resources");
        let untyped = match resources.kind {
            Kind::Class(_) => format!("rt::to_value({}.clone())", resources.expr),
            Kind::Exact { .. } => format!("rt::to_value({})", resources.expr),
            Kind::Null | Kind::SystemType { .. } => return Err(unsupported(node, "resources that are not an object")),
        };
        // The dictionary as the instance of each call, with the loader's error naming the method.
        let instance = |member: String| Typed {
            expr: format!("rt::exact({local}.clone(), {}, 0, {}, {})?", rust_string_literal(&member), node.line(), node.position()),
            kind: Kind::Exact { id: this().id(), nullable: None },
        };
        let (count_member, ensure_member) = (self.member_name(get_count), self.member_name(ensure_method));
        let count = self.declared_call(node, get_count, &[instance(count_member)])?;
        let count_local = self.local_named("count");
        let capacity = Typed {
            expr: format!("{count_local}.wrapping_add({}_i32)", ensure.capacity),
            kind: Kind::Exact { id: TypeId::of::<i32>(), nullable: None },
        };
        let ensure_call = self.declared_call(node, ensure_method, &[instance(ensure_member), capacity])?;
        self.line(format!("let {local}: ::ferroui_base::metadata::MarkupValue = {untyped};"));
        self.line(format!("if rt::is_instance(&{local}, {handle}) {{"));
        self.line(format!("    let {count_local}: i32 = {count};"));
        self.line(format!("    {ensure_call};"));
        self.line("}".to_string());
        Ok(())
    }

    /// The description of a plain property (`XamlIlClrPropertyInfoEmitter`)
    /// as a value: `rt::clr_property_info` over the declaration of its
    /// accessors.
    fn clr_property_info(&self, node: &Rc<dyn IXamlAstNode>, property: &Rc<dyn IXamlProperty>) -> EmitResult<String> {
        Ok(format!("rt::boxed(rt::clr_property_info({}))", self.property_info_arguments(node, property)?))
    }

    /// The arguments that describe a plain property to the run-time helpers:
    /// the metadata that declares its accessors, its name, whether it is
    /// static, the handle of its type, and whether its getter returns the
    /// shared boolean boxes.
    fn property_info_arguments(&self, node: &Rc<dyn IXamlAstNode>, property: &Rc<dyn IXamlProperty>) -> EmitResult<String> {
        let name = property.name();
        let accessor = property
            .getter()
            .or_else(|| property.setter())
            .ok_or_else(|| unsupported(node, format!("{name}: a property without accessors")))?;
        let runtime = accessor
            .as_any()
            .downcast_ref::<RuntimeMethod>()
            .ok_or_else(|| unsupported(node, format!("{name}: an accessor that is not a method of the run-time type system")))?;
        let is_declared = matches!(
            runtime.declared(),
            Some(DeclaredMember::Getter(_) | DeclaredMember::Setter(_) | DeclaredMember::StaticGetter(_) | DeclaredMember::StaticSetter(_))
        );
        if !is_declared {
            return Err(unsupported(node, format!("{name}: the accessors of the property are not declared")));
        }
        let declaring = runtime
            .declaring_type
            .upgrade()
            .ok_or_else(|| unsupported(node, format!("{name}: the declaring type is gone")))?;
        let markup = self.markup_expr(node, &declaring)?;
        let property_type = property.property_type();
        let handle = match runtime_type(&property_type).and_then(RuntimeType::handle) {
            Some(handle) if !handle.is_object() => self.handle_expr(node, &property_type)?,
            _ => "::ferroui_base::data::core::ValueType::object()".to_string(),
        };
        let cached_boxed_boolean = property.getter().is_some_and(|getter| getter.return_type().is("System", "Boolean"));
        Ok(format!(
            "{markup}, {}, {}, {handle}, {cached_boxed_boolean}",
            rust_string_literal(&name),
            runtime.is_static
        ))
    }

    /// A compiled binding path (`XamlIlBindingPathNode`): a builder, the
    /// builder call of each transform element and then of each element, as
    /// the interpreter's `binding_path::evaluate` makes them, then `build()`.
    fn binding_path(&mut self, node: &Rc<dyn IXamlAstNode>, path: &Rc<XamlIlBindingPathNode>) -> EmitResult<Typed> {
        path.try_enable_typed_emission();
        let transform_elements = path.transform_elements.borrow().clone();
        let elements = path.elements.borrow().clone();
        let mut calls = Vec::with_capacity(transform_elements.len() + elements.len());
        for element in transform_elements.iter().chain(elements.iter()) {
            calls.push(self.path_element(node, element)?);
        }
        let local = self.local_named("path");
        self.line(format!("let {local} = {{"));
        self.line("    let builder = ::ferroui_base::data::CompiledBindingPathBuilder::new();".to_string());
        for call in calls {
            self.line(format!("    let builder = {call};"));
        }
        self.line("    builder.build()".to_string());
        self.line("};".to_string());
        Ok(Typed { expr: format!("{local}.clone()"), kind: self.kind_of(TypeId::of::<CompiledBindingPath>()) })
    }

    /// The builder call of one element of a binding path (`builder` is the
    /// builder so far).
    fn path_element(&mut self, node: &Rc<dyn IXamlAstNode>, element: &XamlIlBindingPathElementNode) -> EmitResult<String> {
        let class_of = |type_: &Rc<dyn IXamlType>| -> EmitResult<String> {
            let class = runtime_type(type_)
                .and_then(RuntimeType::type_info)
                .ok_or_else(|| unsupported(node, format!("{} is not a class of the object model: it cannot be an ancestor type", type_.get_full_name())))?;
            let path = class
                .rust_path()
                .ok_or_else(|| unsupported(node, format!("no public Rust path is recorded for {}", class.full_name())))?;
            Ok(format!("<{} as ::ferroui_base::StaticType>::TYPE", absolute(path)))
        };
        let level_of = |level: i32| -> EmitResult<usize> {
            usize::try_from(level).map_err(|_| unsupported(node, "a negative ancestor level"))
        };
        Ok(match element {
            XamlIlBindingPathElementNode::Not(_) => "builder.not()".to_string(),
            XamlIlBindingPathElementNode::StreamObservable(_) => "builder.stream_observable()".to_string(),
            XamlIlBindingPathElementNode::StreamTask(_) => "builder.stream_task()".to_string(),
            XamlIlBindingPathElementNode::SelfElement(_) => "builder.self_()".to_string(),
            XamlIlBindingPathElementNode::FindAncestor(e) => {
                format!("builder.ancestor(::core::option::Option::Some({}), {})", class_of(&e.type_)?, level_of(e.level)?)
            }
            XamlIlBindingPathElementNode::FindVisualAncestor(e) => {
                format!("builder.visual_ancestor(::core::option::Option::Some({}), {})", class_of(&e.type_)?, level_of(e.level)?)
            }
            XamlIlBindingPathElementNode::ElementName(e) => {
                self.uses_name_scope = true;
                format!(
                    "builder.element_name(rt::path_name_scope(name_scope.as_ref(), {}, {})?, {})",
                    node.line(),
                    node.position(),
                    rust_string_literal(&e.name)
                )
            }
            XamlIlBindingPathElementNode::TemplatedParent(_) => "builder.templated_parent()".to_string(),
            XamlIlBindingPathElementNode::FerroProperty(e) => {
                let definition = self.registered_definition(node, &e.field.name(), &e.field)?;
                match e.accepts_null {
                    true => format!("builder.ferro_property_with(rt::property({definition}), true)"),
                    false => format!("builder.ferro_property(rt::property({definition}))"),
                }
            }
            XamlIlBindingPathElementNode::ClrProperty(e) => format!(
                "rt::path_property(&builder, {}, {}, {})",
                self.property_info_arguments(node, &e.property)?,
                e.accepts_null,
                e.emit_typed.get()
            ),
            XamlIlBindingPathElementNode::ArrayIndexer(e) => {
                let indices: Vec<String> = e.values.iter().map(|value| format!("{value}_i32")).collect();
                format!("builder.array_element(&[{}])", indices.join(", "))
            }
            XamlIlBindingPathElementNode::TypeCast(e) => {
                let runtime = runtime_type(&e.type_)
                    .ok_or_else(|| unsupported(node, format!("{} is not a type of the run-time type system", e.type_.get_full_name())))?;
                match runtime.type_info() {
                    Some(_) => format!(
                        "builder.type_cast_value(::ferroui_base::data::core::expression_nodes::CastTarget::Class({}))",
                        class_of(&e.type_)?
                    ),
                    None => format!(
                        "builder.type_cast_value(::ferroui_base::data::core::expression_nodes::CastTarget::Value({}))",
                        self.handle_expr(node, &e.type_)?
                    ),
                }
            }
            XamlIlBindingPathElementNode::ClrIndexer(_) => return Err(unsupported(node, "a binding path with an indexer")),
            XamlIlBindingPathElementNode::ClrMethod(_) => return Err(unsupported(node, "a binding path with a method")),
            XamlIlBindingPathElementNode::ClrMethodAsCommand(_) => {
                return Err(unsupported(node, "a binding path with a method as a command"));
            }
        })
    }

    /// The expression of the metadata of a type: `<T as MarkupTyped>::MARKUP`
    /// of a markup type, `rt::class_markup` of a class.
    fn markup_expr(&self, node: &Rc<dyn IXamlAstNode>, runtime: &Rc<RuntimeType>) -> EmitResult<String> {
        let markup = metadata_of(runtime).ok_or_else(|| unsupported(node, format!("{} has no metadata", runtime.full_name())))?;
        if let Some(class) = runtime.type_info() {
            let path = class
                .rust_path()
                .ok_or_else(|| unsupported(node, format!("no public Rust path is recorded for {}", runtime.full_name())))?;
            return Ok(format!("rt::class_markup(<{} as ::ferroui_base::StaticType>::TYPE)", absolute(path)));
        }
        let path = markup
            .rust_path()
            .ok_or_else(|| unsupported(node, format!("no public Rust path is recorded for {}", runtime.full_name())))?;
        let qualified = match markup.rust_path_is_trait() {
            true => format!("dyn {}", absolute(path)),
            false => absolute(path),
        };
        Ok(format!("<{qualified} as ::ferroui_base::metadata::MarkupTyped>::MARKUP"))
    }

    /// A markup extension (`XamlMarkupExtensionNode`): the extension object,
    /// then its `ProvideValue` through the typed function of the declared
    /// method, given the context as the service provider if it takes one.
    /// While it runs, the provide-value target property of the context is
    /// the property of the innermost assignment (a registered property's
    /// definition, else its name), as the interpreter sets it.
    fn markup_extension(&mut self, node: &Rc<dyn IXamlAstNode>, extension: &Rc<XamlMarkupExtensionNode>) -> EmitResult<Typed> {
        if let Some(options) = extension.provide_value.as_any().downcast_ref::<OptionsMarkupExtensionMethod>() {
            return self.options_markup_extension(node, extension, options);
        }
        let method = extension
            .provide_value
            .as_any()
            .downcast_ref::<RuntimeMethod>()
            .ok_or_else(|| unsupported(node, "ProvideValue is not a method of the run-time type system"))?;
        let parameters = method.parameter_handles.clone();
        let needs_context = !parameters.is_empty();
        let property = self.assignments.last().map(|assignment| assignment.property.clone());
        let provide_value_target = needs_context && FRAMEWORK_CONTEXT.provide_value_target && property.is_some();

        let value_node = extension.value().as_node();
        let value = self.value(&value_node)?;
        // The extension is held in a local: the context is set between creating and calling it.
        let value = match value.kind {
            Kind::Class(_) => value,
            _ => {
                let local = self.local_named("extension");
                self.line(format!("let {local} = {};", value.expr));
                Typed { expr: local, kind: value.kind }
            }
        };
        let mut arguments = vec![value];
        if let Some(parameter) = parameters.first() {
            let parameter = parameter.ok_or_else(|| unsupported(node, "ProvideValue takes a parameter without a Rust type"))?;
            let provider = TypeId::of::<Rc<dyn IServiceProvider>>();
            let nullable = TypeId::of::<Option<Rc<dyn IServiceProvider>>>();
            if parameter.id() != provider && parameter.id() != nullable {
                return Err(unsupported(node, "ProvideValue takes a parameter that is not the service provider"));
            }
            self.uses_context = true;
            arguments.push(Typed {
                expr: "rt::service_provider(&context)".to_string(),
                kind: Kind::Exact { id: provider, nullable: Some(nullable) },
            });
        }
        if let (true, Some(property)) = (provide_value_target, &property) {
            self.set_target_property(node, property)?;
        }
        let call = self.declared_call(node, method, &arguments)?;
        let returned = self.declared_return(node, method)?;
        let local = self.local_named("provided");
        self.line(format!("let {local} = {call};"));
        if provide_value_target {
            self.line("context.set_target_property(::core::option::Option::None);".to_string());
        }
        Ok(Typed { expr: local, kind: self.kind_of(returned) })
    }

    /// `context.ProvideTargetProperty = <descriptor>` for the property of
    /// the innermost assignment: a registered property's definition, a plain
    /// property's description, else its name.
    fn set_target_property(&mut self, node: &Rc<dyn IXamlAstNode>, property: &Rc<xamlx::ast::XamlAstClrProperty>) -> EmitResult<()> {
        let descriptor = match XamlIlFerroPropertyHelper::try_get_provide_value_target(property) {
            Some(XamlIlProvideValueTargetProperty::FerroProperty(field)) => {
                let registered = field
                    .as_any()
                    .downcast_ref::<RuntimeField>()
                    .and_then(RuntimeField::ferro_property)
                    .ok_or_else(|| unsupported(node, format!("{}: not a registered property", property.name())))?;
                let declaring_type = field.declaring_type();
                let definition = property_definition(registered, runtime_type(&declaring_type).and_then(RuntimeType::type_info))
                    .map_err(|reason| unsupported(node, format!("{}: {reason}", property.name())))?;
                format!("rt::property_value({definition})")
            }
            Some(XamlIlProvideValueTargetProperty::ClrProperty(clr)) => self.clr_property_info(node, &clr)?,
            None => format!("rt::boxed(::std::string::String::from({}))", rust_string_literal(&property.name())),
        };
        self.uses_context = true;
        self.line(format!("context.set_target_property({descriptor});"));
        Ok(())
    }

    /// A markup extension with options (`OnPlatform`, `OnFormFactor`: the
    /// `OptionsMarkupExtensionMethod` of the transform): the extension
    /// object, then the branches in order, each option evaluated and its
    /// condition called only when the branches before it did not hold, the
    /// value of the first that holds converted to the return type, else the
    /// default node (or the null of a reference type), as the interpreter's
    /// `options_markup_extension` does.
    fn options_markup_extension(
        &mut self,
        node: &Rc<dyn IXamlAstNode>,
        extension: &Rc<XamlMarkupExtensionNode>,
        options: &OptionsMarkupExtensionMethod,
    ) -> EmitResult<Typed> {
        let needs_context = !options.parameters().is_empty();
        let property = self.assignments.last().map(|assignment| assignment.property.clone());
        let provide_value_target = needs_context && FRAMEWORK_CONTEXT.provide_value_target && property.is_some();
        let value_node = extension.value().as_node();
        let instance = self.value(&value_node)?;
        let instance = match instance.kind {
            Kind::Class(_) => instance,
            _ => {
                // Created as the run-time loader creates it; the branches may not use it.
                let local = self.local_named("_extension");
                self.line(format!("let {local} = {};", instance.expr));
                Typed { expr: format!("{local}.clone()"), kind: instance.kind }
            }
        };
        if let (true, Some(property)) = (provide_value_target, &property) {
            self.set_target_property(node, property)?;
        }
        let return_type = options.return_type();
        let return_handle = runtime_type(&return_type)
            .and_then(RuntimeType::handle)
            .ok_or_else(|| unsupported(node, format!("the return type {} has no handle", return_type.get_fqn())))?;
        let local = self.local_named("provided");
        let label = format!("'{local}");
        self.line(format!("let {local} = {label}: {{"));
        let container = &options.extension_node_container;
        for branch in container.branches() {
            let condition = branch
                .condition_method
                .as_any()
                .downcast_ref::<RuntimeMethod>()
                .ok_or_else(|| unsupported(node, "the condition of an option is not a method of the run-time type system"))?;
            let branch_node: Rc<dyn IXamlAstNode> = branch.clone();
            let mut arguments = Vec::with_capacity(3);
            if !condition.is_static {
                arguments.push(Typed { expr: instance.expr.clone(), kind: instance.kind });
            }
            if branch.has_context() {
                self.uses_context = true;
                arguments.push(Typed {
                    expr: "rt::service_provider(&context)".to_string(),
                    kind: Kind::Exact {
                        id: TypeId::of::<Rc<dyn IServiceProvider>>(),
                        nullable: Some(TypeId::of::<Option<Rc<dyn IServiceProvider>>>()),
                    },
                });
            }
            let option = self.indented_by(1, |emitter| emitter.value(&branch.option().as_node()))?;
            arguments.push(option);
            let call = self.declared_call(&branch_node, condition, &arguments)?;
            self.line(format!("    if {call} {{"));
            let value_node = branch.value().as_node();
            let value = self.indented_by(2, |emitter| emitter.value(&value_node))?;
            let converted = self.coerce(&value, return_handle.id()).ok_or_else(|| {
                unsupported(&value_node, format!("an option value that cannot be stated as {}", return_type.get_fqn()))
            })?;
            self.line(format!("        break {label} {converted};"));
            self.line("    }".to_string());
        }
        let default = match container.default_node() {
            Some(default_node) => {
                let default_node = default_node.as_node();
                let value = self.indented_by(1, |emitter| emitter.value(&default_node))?;
                self.coerce(&value, return_handle.id()).ok_or_else(|| {
                    unsupported(&default_node, format!("a default value that cannot be stated as {}", return_type.get_fqn()))
                })?
            }
            None if !return_type.is_value_type() || return_type.is_nullable() => self
                .coerce(&Typed { expr: "::core::option::Option::None".to_string(), kind: Kind::Null }, return_handle.id())
                .ok_or_else(|| unsupported(node, format!("the null of {}", return_type.get_fqn())))?,
            // `default(T)` of a value type, as the interpreter states it (`default_value`): the
            // constant 0 of an enumeration or of a primitive type.
            None if return_type.is_enum() || return_type.namespace().as_deref() == Some("System") => {
                let zero = self.constant(node, &return_type, &XamlValue::Int32(0))?;
                self.coerce(&zero, return_handle.id())
                    .ok_or_else(|| unsupported(node, format!("the default value of {}", return_type.get_fqn())))?
            }
            None => return Err(unsupported(node, format!("the default value of {}", return_type.get_fqn()))),
        };
        self.line(format!("    {default}"));
        self.line("};".to_string());
        if provide_value_target {
            self.line("context.set_target_property(::core::option::Option::None);".to_string());
        }
        Ok(Typed { expr: format!("{local}.clone()"), kind: self.kind_of(return_handle.id()) })
    }

    /// Runs `emit`, indenting the statements it writes by `levels`.
    fn indented_by<T>(&mut self, levels: usize, emit: impl FnOnce(&mut Self) -> EmitResult<T>) -> EmitResult<T> {
        let start = self.lines.len();
        let value = emit(self)?;
        let pad = "    ".repeat(levels);
        for line in &mut self.lines[start..] {
            line.insert_str(0, &pad);
        }
        Ok(value)
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
        // The last value is converted to the parameter type of the setter, as the
        // interpreter evaluates it: a value of type `object` with the checked cast.
        let values = assignment.values.borrow().clone();
        if let (Some(last), Some(value_node), Some(parameter), Some(Some(handle))) = (
            arguments.last_mut(),
            values.last(),
            method.parameters.last(),
            method.parameter_handles.last(),
        ) {
            if let Some(checked) = self.checked_cast(&value_node.as_node(), last, parameter, *handle, method)? {
                *last = checked;
            }
        }
        let call = self.declared_call(node, method, &arguments)?;
        self.line(format!("{call};"));
        Ok(())
    }

    /// A value of type `object` where a reference type `parameter` (held in
    /// `handle`) is expected: the checked cast of the interpreter's
    /// conversion (`rt::cast_checked`: the value if it is an instance of the
    /// type, else the loader's error), then the value as the declared type
    /// of the argument of `method`. `None` if no cast is needed.
    fn checked_cast(
        &self,
        node: &Rc<dyn IXamlAstNode>,
        value: &Typed,
        parameter: &Rc<dyn IXamlType>,
        handle: ferroui_base::data::core::ValueType,
        method: &RuntimeMethod,
    ) -> EmitResult<Option<Typed>> {
        let object = TypeId::of::<Option<BoxedValue>>();
        let Kind::Exact { id, .. } = value.kind else { return Ok(None) };
        if id != object || handle.is_object() || parameter.is_value_type() {
            return Ok(None);
        }
        let type_handle = self.handle_expr(node, parameter)?;
        let member = self.member_name(method);
        let index = method.parameters.len() - 1;
        Ok(Some(Typed {
            expr: format!(
                "rt::exact(rt::cast_checked({}, {type_handle}, {}, {}, {})?, {}, {index}, {}, {})?",
                value.expr,
                rust_string_literal(&parameter.full_name()),
                node.line(),
                node.position(),
                rust_string_literal(&member),
                node.line(),
                node.position()
            ),
            kind: Kind::Exact { id: handle.id(), nullable: None },
        }))
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

    /// The instance of a member that takes `&This` (`target`), by reference and without a
    /// copy where the Rust types allow it: a local of the declared type itself; an object
    /// as a handle of the declaring base class (`Ref::upcast_ref`); a value whose
    /// declaration names the declared type as its base, by deref coercion
    /// (`&RowDefinitions` as `&FerroList<Ref<RowDefinition>>`).
    fn receiver(&self, argument: &Typed, target: TypeId) -> Option<String> {
        if !is_identifier(&argument.expr) {
            return None;
        }
        match argument.kind {
            Kind::Class(class) => {
                let (declared, false) = TypeInfo::find_by_handle(target)? else { return None };
                if std::ptr::eq(declared, class) {
                    return Some(format!("&{}", argument.expr));
                }
                declared.is_assignable_from(class).then(|| {
                    declared.rust_path().map(|path| format!("{}.upcast_ref::<{}>()", argument.expr, absolute(path)))
                })?
            }
            Kind::Exact { id, .. } => {
                if id == target {
                    return Some(format!("&{}", argument.expr));
                }
                let mut current = MarkupType::find_by_handle(id)?.base_type();
                while let Some(type_) = current {
                    if let Some(handle) = type_.handle().filter(|handle| handle.id() == target) {
                        // A shared handle of the base (`Rc<ReflectionBinding>`) is not reached by
                        // deref coercion from the handle of the derived value
                        // (`Rc<ReflectionBindingExtension>` dereferences to the base itself): the
                        // caller converts it at run time.
                        if handle.name().starts_with("alloc::rc::Rc<") {
                            return None;
                        }
                        return Some(format!("&{}", argument.expr));
                    }
                    current = type_.base_type();
                }
                None
            }
            Kind::Null | Kind::SystemType { .. } => None,
        }
    }

    /// The instance of an instance member, from a value of another Rust
    /// type: borrowed from the nullable form of the declared type (a
    /// nullable collection read from a property, `rt::instance`), else
    /// converted at run time as the run-time loader converts it
    /// (`rt::argument`); either with the loader's error for a value that
    /// does not convert. `None` for an argument that is not the instance.
    fn instance_argument(
        &self,
        node: &Rc<dyn IXamlAstNode>,
        method: &RuntimeMethod,
        index: usize,
        argument: &Typed,
        parameter: TypeId,
    ) -> Option<String> {
        let Kind::Exact { id, .. } = argument.kind else { return None };
        if index != 0 || method.is_static {
            return None;
        }
        use ferroui_base::data::core::{ValueType, ValueTypes};
        let declaring = method.declaring_type.upgrade()?;
        let member = format!("{}.{}", declaring.full_name(), method.name);
        if ValueTypes::nullable_inner(ValueType::new(id, "")).is_some_and(|inner| inner.id() == parameter) {
            return Some(format!(
                "rt::instance(&{}, {}, {}, {})?",
                argument.expr,
                rust_string_literal(&member),
                node.line(),
                node.position()
            ));
        }
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
        let markup = metadata_of(runtime)
            .ok_or_else(|| unsupported(node, format!("{} has no metadata", runtime.full_name())))?;
        let path = markup
            .rust_path()
            .ok_or_else(|| unsupported(node, format!("no public Rust path is recorded for {}", runtime.full_name())))?;
        // A generic instantiation is qualified (`<::path::List<T>>::f`).
        Ok(match (markup.rust_path_is_trait(), path.contains('<')) {
            (true, _) => format!("<dyn {}>", absolute(path)),
            (false, true) => format!("<{}>", absolute(path)),
            (false, false) => absolute(path),
        })
    }

    /// A call of the typed function of the declared member `method`
    /// ([`DeclaredMember`]) with `arguments` (the instance first for an
    /// instance member), each stated as the Rust type the member declares,
    /// with the failure of a fallible member as a load error at `node`.
    fn declared_call(&self, node: &Rc<dyn IXamlAstNode>, method: &RuntimeMethod, arguments: &[Typed]) -> EmitResult<String> {
        self.position.set((node.line(), node.position()));
        let name = &method.name;
        let declared = method.declared().ok_or_else(|| unsupported(node, format!("{name}: not a declared member")))?;
        let emit = declared.emit().ok_or_else(|| unsupported(node, format!("{name}: the declaration has no typed function")))?;
        let owner = self.owner_path(node, method)?;
        let mut parameters: Vec<TypeId> = Vec::with_capacity(arguments.len());
        if !method.is_static {
            let this = method
                .declaring_type
                .upgrade()
                .and_then(|declaring| metadata_of(&declaring))
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
            if index == 0 && !method.is_static {
                if let Some(receiver) = self.receiver(argument, *parameter) {
                    texts.push(receiver);
                    continue;
                }
            }
            let text = match self.coerce(argument, *parameter) {
                Some(text) => text,
                None => self.instance_argument(node, method, index, argument, *parameter).ok_or_else(|| {
                    unsupported(node, format!("{name}: argument {index} cannot be stated as the declared type"))
                })?,
            };
            // The instance is passed by reference: a local of exactly the declared type as it is.
            let is_instance = index == 0 && !method.is_static;
            texts.push(match (is_instance, text.strip_suffix(".clone()")) {
                // Already a borrow of the instance.
                (true, _) if text.starts_with("rt::instance(") => text,
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
        self.uses_name_scope = true;
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
        self.uses_name_scope = true;
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

/// The namespace information of a document as the value of an
/// `rt::XmlNamespaceTable` (the argument of `rt::create_context`): what the
/// static provider of the document gives the run-time loader's contexts,
/// prefixes in order, one per line. `None` if the document has none.
pub fn namespace_table(document: &RuntimeDocument) -> Option<String> {
    let service_type = TypeId::of::<Rc<dyn IFerroXamlIlXmlNamespaceInfoProvider>>();
    let provider = document.static_providers.iter().find_map(|provider| provider.get_static_service(service_type))?;
    let provider = provider.downcast_ref::<Rc<dyn IFerroXamlIlXmlNamespaceInfoProvider>>()?;
    let namespaces = provider.xml_namespaces();
    let mut prefixes: Vec<&String> = namespaces.keys().collect();
    prefixes.sort();
    let entries: Vec<String> = prefixes
        .into_iter()
        .map(|prefix| {
            let infos: Vec<String> = namespaces[prefix]
                .iter()
                .map(|info| {
                    format!(
                        "({}, {})",
                        rust_string_literal(&info.clr_namespace()),
                        rust_string_literal(&info.clr_assembly_name())
                    )
                })
                .collect();
            format!("    ({}, &[{}]),\n", rust_string_literal(prefix), infos.join(", "))
        })
        .collect();
    Some(format!("&[\n{}]", entries.concat()))
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
/// `namespaces` names the constant (an `rt::XmlNamespaceTable`) that holds
/// the [`namespace_table`] of the document; the caller writes it.
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
    document: &RuntimeDocument,
    namespaces: &str,
    function_name: &str,
    document_name: &str,
) -> Result<String, UnsupportedNode> {
    if context_definition(configuration) != FRAMEWORK_CONTEXT {
        return Err(unsupported(root, "the language does not define the context of the framework language"));
    }
    let base_uri = match &document.base_uri {
        Some(uri) => format!("::core::option::Option::Some({})", rust_string_literal(uri.original_string())),
        None => "::core::option::Option::None".to_string(),
    };
    let mut parent_stack = ParentStackNodes { nodes: HashSet::new(), parents: Vec::new() };
    visit_node(root, &mut parent_stack).map_err(|e| failed(root, e))?;
    let group = root
        .as_value_with_manipulation_node()
        .ok_or_else(|| unsupported(root, "the root is not a value with a manipulation"))?;
    let manipulation = group.manipulation().ok_or_else(|| unsupported(root, "the root has no manipulation"))?;
    let root_value = group.value().as_node();
    if !root_value.is::<XamlAstNewClrObjectNode>() {
        return Err(unsupported(&root_value, "the root object is not created with a constructor"));
    }

    let mut emitter = Emitter {
        configuration,
        document_name,
        lines: Vec::new(),
        local_names: HashMap::new(),
        compiler_locals: HashMap::new(),
        parent_stack_nodes: parent_stack.nodes,
        base_uri: base_uri.clone(),
        namespaces: namespaces.to_string(),
        uses_context: false,
        uses_name_scope: false,
        assignments: Vec::new(),
        position: std::cell::Cell::new((0, 0)),
    };
    // `Build`: the root object, then `Populate` with a context of its own (its name
    // scope field is filled from the parent service provider) whose root object is
    // the root.
    let created = emitter.value(&root_value)?;
    let Kind::Class(root_class) = created.kind else {
        return Err(unsupported(&root_value, "the root object is not a class of the object model"));
    };
    let root_path = root_class
        .rust_path()
        .ok_or_else(|| unsupported(&root_value, "no public Rust path is recorded for the root class"))?;
    emitter.line(format!(
        "let context = rt::populate_context(service_provider, {base_uri}, {namespaces}, rt::to_value({}.clone()));",
        created.expr
    ));
    emitter.line("let name_scope = context.name_scope_field();".to_string());
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
