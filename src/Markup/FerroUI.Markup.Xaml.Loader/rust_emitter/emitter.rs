//! Walks the transformed AST of a document and writes the Rust statements
//! that build it: one emit per evaluation of the interpreter
//! (`runtime/interpreter/evaluators.rs`, `runtime/framework/nodes.rs`), in
//! the same order. Anything outside the supported set is reported as
//! [`UnsupportedNode`]; nothing approximate is written.

use std::any::TypeId;
use std::fmt;
use std::rc::Rc;

use ferroui_base::metadata::MarkupType;
use ferroui_base::{BoxedValue, FerroObject, StaticType, StyledElement, TypeInfo};
use xamlx::ast::XamlAstExtensions as _;
use xamlx::ast::XamlAstNodeExtensions as _;
use xamlx::ast::{
    visit_node, IXamlAstNode, IXamlAstValueNode, IXamlAstVisitor, XamlAstNewClrObjectNode, XamlAstTextNode,
    XamlConstantNode, XamlDirectCallPropertySetter, XamlManipulationGroupNode, XamlNullExtensionNode,
    XamlObjectInitializationNode, XamlPropertyAssignmentNode, XamlStaticExtensionNode, XamlStaticMember,
};
use xamlx::exceptions::{XamlError, XamlResult};
use xamlx::transform::TransformerConfiguration;
use xamlx::type_system::{IXamlConstructor, IXamlType, XamlValue};

use crate::compiler_extensions::ast_nodes::{FerroXamlIlGridLengthAstNode, FerroXamlIlVectorLikeConstantAstNode};
use crate::compiler_extensions::transformers::{FerroNameScopeRegistrationXamlIlNode, HandleRootObjectScopeNode};
use crate::compiler_extensions::XamlIlFerroPropertyHelper;
use crate::runtime::interpreter::single_setter;
use crate::runtime::type_system::{
    RuntimeConstructor, RuntimeField, RuntimeFieldValue, RuntimeInvoker, RuntimeMethod, RuntimeType,
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
    /// `TypeId` of `Option<that type>`.
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

/// `snake_case` of a member name of the managed original, as the port
/// names its Rust members (the rule of `scripts/generate_markup_types.py`):
/// a word starts at an upper-case letter that follows a lower-case letter
/// or a digit, and at the last upper-case letter of a run that is followed
/// by a lower-case letter.
pub(crate) fn snake_case(name: &str) -> String {
    let characters: Vec<char> = name.chars().collect();
    let mut result = String::with_capacity(name.len() + 4);
    for (index, &current) in characters.iter().enumerate() {
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

fn is_identifier(text: &str) -> bool {
    let mut characters = text.chars();
    match characters.next() {
        Some(first) if first.is_ascii_alphabetic() || first == '_' => {}
        _ => return false,
    }
    characters.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

fn f64_literal(value: f64) -> Option<String> {
    value.is_finite().then(|| format!("{value:?}_f64"))
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

struct Emitter<'a> {
    configuration: &'a TransformerConfiguration,
    lines: Vec<String>,
    locals: usize,
}

impl Emitter<'_> {
    fn line(&mut self, text: String) {
        self.lines.push(format!("    {text}"));
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
        if let Some(n) = node.cast::<XamlAstTextNode>() {
            return Ok(exact::<String>(format!("String::from({})", rust_string_literal(&n.text()))));
        }
        if node.is::<XamlNullExtensionNode>() {
            return Ok(Typed { expr: "None".to_string(), kind: Kind::Null });
        }
        if let Some(n) = node.cast::<XamlConstantNode>() {
            let type_ = IXamlAstValueNode::type_(&*n).get_clr_type().map_err(|e| failed(node, e))?;
            return self.constant(node, &type_, &n.constant);
        }
        if let Some(n) = node.cast::<XamlStaticExtensionNode>() {
            return self.static_member(node, &n);
        }
        if let Some(n) = node.cast::<FerroXamlIlVectorLikeConstantAstNode>() {
            let mut arguments = Vec::with_capacity(n.values().len());
            for value in n.values() {
                arguments.push(f64_literal(*value).ok_or_else(|| unsupported(node, "a number that is not finite"))?);
            }
            return self.constructor_call(node, n.constructor(), &arguments);
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
            let value = f64_literal(grid_length.value).ok_or_else(|| unsupported(node, "a number that is not finite"))?;
            let unit = self.enum_member(node, &unit_type, i64::from(grid_length.grid_unit_type as i32))?;
            return self.constructor_call(node, &constructor, &[value, unit.expr]);
        }
        Err(unsupported(node, "no emitter for this value node"))
    }

    /// `new T()` of a class of the object model with its default constructor.
    fn new_object(&mut self, node: &Rc<dyn IXamlAstNode>, n: &Rc<XamlAstNewClrObjectNode>) -> EmitResult<Typed> {
        if !n.arguments.borrow().is_empty() {
            return Err(unsupported(node, "constructor arguments"));
        }
        let type_ = n.type_.borrow().get_clr_type().map_err(|e| failed(node, e))?;
        let class = runtime_type(&type_)
            .and_then(RuntimeType::type_info)
            .ok_or_else(|| unsupported(node, format!("{} is not a class of the object model", type_.full_name())))?;
        let is_default_constructor = n.constructor.as_any().downcast_ref::<RuntimeConstructor>().is_some_and(|c| {
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
        let local = format!("v{}", self.locals);
        self.locals += 1;
        self.line(format!("let {local} = {path}::new();"));
        Ok(Typed { expr: local, kind: Kind::Class(class) })
    }

    /// A compile-time constant: what `constant_value` loads for it.
    fn constant(&mut self, node: &Rc<dyn IXamlAstNode>, type_: &Rc<dyn IXamlType>, constant: &XamlValue) -> EmitResult<Typed> {
        if type_.is_enum() {
            let value = match constant {
                XamlValue::Int32(v) => i64::from(*v),
                XamlValue::Int64(v) => *v,
                _ => return Err(unsupported(node, "an enumeration constant that is not an integer")),
            };
            return self.enum_member(node, type_, value);
        }
        if type_.namespace().as_deref() != Some("System") {
            return Err(unsupported(node, format!("a constant of type {}", type_.full_name())));
        }
        let name = type_.name();
        match (name.as_str(), constant) {
            ("String", XamlValue::String(text)) => {
                Ok(exact::<String>(format!("String::from({})", rust_string_literal(text))))
            }
            ("Double", XamlValue::Double(v)) => match f64_literal(*v) {
                Some(literal) => Ok(exact::<f64>(literal)),
                None => Err(unsupported(node, "a number that is not finite")),
            },
            ("Boolean", XamlValue::Boolean(v)) => Ok(exact::<bool>(format!("{v}"))),
            ("Int32", XamlValue::Int32(v)) => Ok(exact::<i32>(format!("{v}_i32"))),
            ("Int64", XamlValue::Int64(v)) => Ok(exact::<i64>(format!("{v}_i64"))),
            _ => Err(unsupported(node, format!("a constant of type {}", type_.full_name()))),
        }
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
        Ok(Typed { expr: format!("{path}::{variant}"), kind: Kind::Exact { id, nullable } })
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

    /// A call of a declared constructor of a value type, through the source
    /// text of its declaration.
    fn constructor_call(
        &mut self,
        node: &Rc<dyn IXamlAstNode>,
        constructor: &Rc<dyn IXamlConstructor>,
        arguments: &[String],
    ) -> EmitResult<Typed> {
        let runtime = constructor
            .as_any()
            .downcast_ref::<RuntimeConstructor>()
            .ok_or_else(|| unsupported(node, "a constructor that is not one of the run-time type system"))?;
        let declaring = runtime
            .declaring_type
            .upgrade()
            .ok_or_else(|| unsupported(node, "the declaring type of the constructor is gone"))?;
        let markup = declaring
            .markup()
            .ok_or_else(|| unsupported(node, format!("{} has no metadata", declaring.full_name())))?;
        if runtime.parameter_handles.len() != arguments.len() {
            return Err(unsupported(node, "the constructor doesn't take the arguments of the node"));
        }
        // The declared constructor with the Rust parameter types of the projected one.
        let declared: Vec<_> = markup
            .constructors
            .iter()
            .filter(|declared| {
                declared.parameters.len() == runtime.parameter_handles.len()
                    && declared
                        .parameters
                        .iter()
                        .zip(&runtime.parameter_handles)
                        .all(|(declared, projected)| projected.is_some_and(|handle| handle.id() == declared().id()))
            })
            .collect();
        let [declared] = declared.as_slice() else {
            return Err(unsupported(node, format!("no single declared constructor of {} matches", markup.full_name())));
        };
        let text = declared.emit.ok_or_else(|| unsupported(node, "the constructor has no source text"))?;
        // Only `<TypeName>::<function>`: the type is known, everything else is relative to the declaring module.
        let function = match text.split_once("::") {
            Some((owner, function)) if owner.trim() == markup.name && is_identifier(function.trim()) => function.trim(),
            _ => return Err(unsupported(node, format!("the constructor is declared as `{text}`"))),
        };
        let path = markup
            .rust_path()
            .ok_or_else(|| unsupported(node, format!("no public Rust path is recorded for {}", markup.full_name())))?;
        let id = markup.handle().ok_or_else(|| unsupported(node, "the type has no value type"))?.id();
        let nullable = markup.nullable.map(|nullable| nullable().id());
        Ok(Typed {
            expr: format!("{path}::{function}({})", arguments.join(", ")),
            kind: Kind::Exact { id, nullable },
        })
    }

    /// The expression as a value of exactly the Rust type `target` (the
    /// value type of a registered property). `None`: it cannot be stated.
    fn coerce(&self, value: &Typed, target: TypeId) -> Option<String> {
        let object = TypeId::of::<Option<BoxedValue>>();
        match value.kind {
            Kind::Exact { id, nullable } => {
                if target == id {
                    Some(value.expr.clone())
                } else if nullable == Some(target) {
                    Some(format!("Some({})", value.expr))
                } else if target == object {
                    Some(format!("Some(std::rc::Rc::new({}) as ferroui_base::BoxedValue)", value.expr))
                } else {
                    None
                }
            }
            Kind::Class(class) => {
                if target == object {
                    return Some(format!("Some(std::rc::Rc::new({}.clone()) as ferroui_base::BoxedValue)", value.expr));
                }
                let (declared, nullable) = TypeInfo::find_by_handle(target)?;
                if !declared.is_assignable_from(class) {
                    return None;
                }
                let handle = if std::ptr::eq(declared, class) {
                    format!("{}.clone()", value.expr)
                } else {
                    format!("{}.clone().upcast::<{}>()", value.expr, declared.rust_path()?)
                };
                Some(if nullable { format!("Some({handle})") } else { handle })
            }
            Kind::Null => {
                let is_nullable_class = TypeInfo::find_by_handle(target).is_some_and(|(_, nullable)| nullable);
                (is_nullable_class || target == object || target == TypeId::of::<Option<String>>())
                    .then(|| "None".to_string())
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

    /// `BeginInit`, the manipulation, `EndInit` (the parent stack is never
    /// needed: a document that needs it is not eligible).
    fn object_initialization(
        &mut self,
        node: &Rc<dyn IXamlAstNode>,
        init: &Rc<XamlObjectInitializationNode>,
        target: &Typed,
    ) -> EmitResult<()> {
        let type_ = init.type_.borrow().clone();
        let supports_initialize = self
            .configuration
            .type_mappings
            .support_initialize
            .as_ref()
            .is_some_and(|support_initialize| support_initialize.is_assignable_from(&*type_));
        if supports_initialize {
            // The contract is the one of the styled element; nothing else implements it by a known path.
            self.styled_class(node, target)?;
            if !init.skip_begin_init.get() {
                self.line(format!("{}.begin_init();", target.expr));
            }
        }
        self.manipulation(&init.manipulation().as_node(), target)?;
        if supports_initialize {
            self.line(format!(
                "{}.try_end_init().map_err(|error| ferroui_markup_xaml::XamlLoadException::with_message(error.to_string()))?;",
                target.expr
            ));
        }
        Ok(())
    }

    /// An assignment of a registered (styled or attached) property through
    /// its plain accessor: `target.set_value(Owner::name_property(), value)`.
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
        let direct = setter
            .as_any()
            .downcast_ref::<XamlDirectCallPropertySetter>()
            .ok_or_else(|| unsupported(node, format!("{property_name}: not a plain property setter")))?;
        let method = direct.method().clone();
        let runtime = method
            .as_any()
            .downcast_ref::<RuntimeMethod>()
            .ok_or_else(|| unsupported(node, format!("{property_name}: the setter is not a method of the run-time type system")))?;
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
        if property.is_direct() || property.is_read_only() {
            return Err(unsupported(node, format!("{property_name}: a direct property")));
        }
        let declaring_type = field.declaring_type();
        let owner = runtime_type(&declaring_type)
            .and_then(RuntimeType::type_info)
            .and_then(TypeInfo::rust_path)
            .ok_or_else(|| {
                unsupported(node, format!("no public Rust path is recorded for {}", declaring_type.full_name()))
            })?;
        let values = assignment.values.borrow().clone();
        let [value_node] = values.as_slice() else {
            return Err(unsupported(node, format!("{property_name}: an assignment with {} values", values.len())));
        };
        let value_node = value_node.as_node();
        let value = self.value(&value_node)?;
        let typed = self.coerce(&value, property.property_type()).ok_or_else(|| {
            unsupported(
                &value_node,
                format!("{property_name}: the value cannot be stated as `{}`", property.property_type_name()),
            )
        })?;
        self.line(format!(
            "{}.set_value({owner}::{}_property(), {typed});",
            target.expr,
            snake_case(property.name())
        ));
        Ok(())
    }

    /// `if (root is StyledElement s) NameScope.SetNameScope(s, scope); scope.Complete();`.
    fn root_object_scope(&mut self, node: &Rc<dyn IXamlAstNode>, target: &Typed) -> EmitResult<()> {
        self.styled_class(node, target)?;
        self.line(format!(
            "ferroui_base::controls::NameScope::set_name_scope(&{}, name_scope.clone().map(ferroui_base::controls::NameScopeRef));",
            target.expr
        ));
        self.line("match &name_scope {".to_string());
        self.line("    Some(scope) => ferroui_base::controls::INameScope::complete(&**scope),".to_string());
        self.line(
            "    None => return Err(ferroui_markup_xaml::XamlLoadException::with_message(\"The runtime context has no name scope to complete\")),"
                .to_string(),
        );
        self.line("}".to_string());
        Ok(())
    }

    /// `context.FerroNameScope.Register(name, target)` with a name from text.
    fn name_scope_registration(
        &mut self,
        node: &Rc<dyn IXamlAstNode>,
        registration: &Rc<FerroNameScopeRegistrationXamlIlNode>,
        target: &Typed,
    ) -> EmitResult<()> {
        let class = self.styled_class(node, target)?;
        if std::ptr::eq(class, FerroObject::TYPE) {
            return Err(unsupported(node, "the target is the object base class"));
        }
        let name = registration
            .name()
            .cast::<XamlAstTextNode>()
            .ok_or_else(|| unsupported(node, "a name that is not text"))?
            .text();
        self.line("match &name_scope {".to_string());
        self.line(format!(
            "    Some(scope) => ferroui_base::controls::INameScope::try_register(&**scope, {}, {}.clone().upcast::<ferroui_base::FerroObject>()).map_err(|error| ferroui_markup_xaml::XamlLoadException::with_message(error.to_string()))?,",
            rust_string_literal(&name),
            target.expr
        ));
        self.line(
            "    None => return Err(ferroui_markup_xaml::XamlLoadException::with_message(\"The runtime context has no name scope to register a name in\")),"
                .to_string(),
        );
        self.line("}".to_string());
        Ok(())
    }
}

/// Writes the Rust function that builds the document with the transformed
/// root node `root`: the counterpart of the `Build` method
/// (`Interpreter::build` followed by `Interpreter::populate`).
///
/// ```ignore
/// pub fn <function_name>(
///     service_provider: Option<std::rc::Rc<dyn ferroui_base::metadata::IServiceProvider>>,
/// ) -> Result<ferroui_base::Ref<RootClass>, ferroui_markup_xaml::XamlLoadException>
/// ```
///
/// `service_provider` is the parent service provider of the context (the
/// caller passes `XamlIlRuntimeHelpers::create_root_service_provider_v3(..)`,
/// as the run-time loader does). `document_name` goes into the header
/// comment. `Err` means the document is not eligible; nothing is written
/// for it.
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

    let mut emitter = Emitter { configuration, lines: Vec::new(), locals: 0 };
    // `Build`: the root object, then `Populate` with a context of its own, whose name
    // scope field is filled from the parent service provider.
    let created = emitter.value(&root_value)?;
    let Kind::Class(root_class) = created.kind else {
        return Err(unsupported(&root_value, "the root object is not a class of the object model"));
    };
    let root_path = root_class
        .rust_path()
        .ok_or_else(|| unsupported(&root_value, "no public Rust path is recorded for the root class"))?;
    emitter.line("let name_scope = match &service_provider {".to_string());
    emitter.line(
        "    Some(provider) => ferroui_markup_xaml::ServiceProviderExtensions::get_name_scope(&**provider),".to_string(),
    );
    emitter.line("    None => None,".to_string());
    emitter.line("};".to_string());
    emitter.manipulation(&manipulation.as_node(), &created)?;
    emitter.line(format!("Ok({})", created.expr));

    let mut source = String::new();
    source.push_str(&format!("/// Generated from `{}`.\n", document_name.replace('`', "'").replace(['\r', '\n'], " ")));
    source.push_str(&format!("pub fn {function_name}(\n"));
    source.push_str("    service_provider: Option<std::rc::Rc<dyn ferroui_base::metadata::IServiceProvider>>,\n");
    source.push_str(&format!(
        ") -> Result<ferroui_base::Ref<{root_path}>, ferroui_markup_xaml::XamlLoadException> {{\n"
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
    fn member_names_become_snake_case() {
        assert_eq!(snake_case("Padding"), "padding");
        assert_eq!(snake_case("HorizontalAlignment"), "horizontal_alignment");
        assert_eq!(snake_case("IsVisible"), "is_visible");
        assert_eq!(snake_case("ZIndex"), "z_index");
        assert_eq!(snake_case("UIElement"), "ui_element");
        assert_eq!(snake_case("Row2Span"), "row2_span");
    }

    #[test]
    fn numbers_are_typed_literals() {
        assert_eq!(f64_literal(100.0).as_deref(), Some("100.0_f64"));
        assert_eq!(f64_literal(-0.5).as_deref(), Some("-0.5_f64"));
        assert_eq!(f64_literal(f64::NAN), None);
        assert_eq!(f64_literal(f64::INFINITY), None);
    }

    #[test]
    fn identifiers_are_recognised() {
        assert!(is_identifier("new"));
        assert!(is_identifier("from_pixels"));
        assert!(!is_identifier(""));
        assert!(!is_identifier("a::b"));
        assert!(!is_identifier("1a"));
    }
}
