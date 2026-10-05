//! Port of `Ast/Common.cs`.
//!
//! Representation: every node lives behind an `Rc` handle (`Rc<dyn IXamlAstNode>`,
//! `Rc<dyn IXamlAstValueNode>`, `Rc<XamlAstObjectNode>`, ...) and keeps its mutable
//! properties in `Cell`/`RefCell` fields. That preserves the upstream semantics the
//! transformers depend on: nodes are mutated in place, visitors replace children in
//! place, and one node can be referenced from several places and compared by identity.

use std::any::Any;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

use crate::exceptions::{XamlError, XamlResult};
use crate::type_system::{IXamlType, XamlPseudoType};

use super::{
    XamlAstClrProperty, XamlAstClrTypeReference, XamlMethodCallBaseNode,
    XamlValueWithManipulationNode, XamlValueWithSideEffectNodeBase,
};

pub trait IXamlLineInfo {
    fn line(&self) -> i32;
    fn position(&self) -> i32;
    fn set_line(&self, value: i32);
    fn set_position(&self, value: i32);
}

/// A plain line/position pair.
#[derive(Debug, Clone, Default)]
pub struct XamlLineInfo {
    line: Cell<i32>,
    position: Cell<i32>,
}

impl XamlLineInfo {
    pub fn new(line: i32, position: i32) -> Self {
        Self {
            line: Cell::new(line),
            position: Cell::new(position),
        }
    }
}

impl IXamlLineInfo for XamlLineInfo {
    fn line(&self) -> i32 {
        self.line.get()
    }
    fn position(&self) -> i32 {
        self.position.get()
    }
    fn set_line(&self, value: i32) {
        self.line.set(value)
    }
    fn set_position(&self, value: i32) {
        self.position.set(value)
    }
}

pub trait IXamlAstVisitor {
    fn visit(&mut self, node: Rc<dyn IXamlAstNode>) -> XamlResult<Rc<dyn IXamlAstNode>>;
    fn push(&mut self, node: Rc<dyn IXamlAstNode>);
    fn pop(&mut self);
}

pub trait IXamlAstNode: IXamlLineInfo + 'static {
    fn is_skipped(&self) -> bool {
        false
    }

    /// Visits (and replaces in place) the children of this node.
    fn visit_children(&self, visitor: &mut dyn IXamlAstVisitor) -> XamlResult<()> {
        let _ = visitor;
        Ok(())
    }

    fn as_any(&self) -> &dyn Any;
    fn into_any_rc(self: Rc<Self>) -> Rc<dyn Any>;

    /// `GetType().Name`.
    fn type_name(&self) -> &'static str;

    /// `object.ToString()`.
    fn to_node_string(&self) -> String {
        self.type_name().to_string()
    }

    /// `this as IXamlAstValueNode`.
    fn into_value_node(self: Rc<Self>) -> Option<Rc<dyn IXamlAstValueNode>> {
        None
    }
    /// `this as IXamlAstManipulationNode`.
    fn into_manipulation_node(self: Rc<Self>) -> Option<Rc<dyn IXamlAstManipulationNode>> {
        None
    }
    /// `this as IXamlAstImperativeNode`.
    fn into_imperative_node(self: Rc<Self>) -> Option<Rc<dyn IXamlAstImperativeNode>> {
        None
    }
    /// `this as IXamlAstTypeReference`.
    fn into_type_reference(self: Rc<Self>) -> Option<Rc<dyn IXamlAstTypeReference>> {
        None
    }
    /// `this as IXamlAstPropertyReference`.
    fn into_property_reference(self: Rc<Self>) -> Option<Rc<dyn IXamlAstPropertyReference>> {
        None
    }
    /// `this as IXamlAstNodeNeedsParentStack`.
    fn as_needs_parent_stack(&self) -> Option<&dyn IXamlAstNodeNeedsParentStack> {
        None
    }

    /// `this as XamlValueWithSideEffectNodeBase` (base class of several nodes).
    fn as_value_with_side_effect_node_base(&self) -> Option<&XamlValueWithSideEffectNodeBase> {
        None
    }
    /// `this as XamlValueWithManipulationNode` (also true for its subclasses).
    fn as_value_with_manipulation_node(&self) -> Option<&XamlValueWithManipulationNode> {
        None
    }
    /// `this as XamlMethodCallBaseNode` (base class of the method call nodes).
    fn as_method_call_base_node(&self) -> Option<&XamlMethodCallBaseNode> {
        None
    }

    /// Dynamic interface query for interfaces this crate does not know about, e.g. the
    /// backend specific `IXamlAstEmitableNode<TBackendEmitter, TEmitResult>`.
    /// See [`crate::extensions::query_interface`].
    fn query_interface(self: Rc<Self>, slot: &mut dyn Any) -> bool {
        let _ = slot;
        false
    }
}

pub trait IXamlAstManipulationNode: IXamlAstNode {}

pub trait IXamlAstNodeNeedsParentStack {
    fn needs_parent_stack(&self) -> bool;
}

pub trait IXamlAstImperativeNode: IXamlAstNode {}

pub trait IXamlAstValueNode: IXamlAstNode {
    fn type_(&self) -> Rc<dyn IXamlAstTypeReference>;
}

pub trait IXamlAstTypeReference: IXamlAstNode {
    fn is_markup_extension(&self) -> bool;
    fn equals(&self, other: &dyn IXamlAstTypeReference) -> bool;
}

pub trait IXamlAstPropertyReference: IXamlAstNode {}

/// Implements the bookkeeping members of [`IXamlAstNode`] inside an `impl` block.
///
/// `xaml_ast_node_members!("TypeName", value, manipulation)` implements `as_any`,
/// `into_any_rc`, `type_name` and one `into_*`/`as_*` conversion per listed interface
/// (`value`, `manipulation`, `imperative`, `type_reference`, `property_reference`,
/// `needs_parent_stack`).
#[macro_export]
macro_rules! xaml_ast_node_members {
    ($name:literal $(, $kind:ident)* $(,)?) => {
        fn as_any(&self) -> &dyn ::std::any::Any {
            self
        }
        fn into_any_rc(self: ::std::rc::Rc<Self>) -> ::std::rc::Rc<dyn ::std::any::Any> {
            self
        }
        fn type_name(&self) -> &'static str {
            $name
        }
        $( $crate::xaml_ast_node_members!(@kind $kind); )*
    };
    (@kind value) => {
        fn into_value_node(self: ::std::rc::Rc<Self>) -> Option<::std::rc::Rc<dyn $crate::ast::IXamlAstValueNode>> {
            Some(self)
        }
    };
    (@kind manipulation) => {
        fn into_manipulation_node(
            self: ::std::rc::Rc<Self>,
        ) -> Option<::std::rc::Rc<dyn $crate::ast::IXamlAstManipulationNode>> {
            Some(self)
        }
    };
    (@kind imperative) => {
        fn into_imperative_node(
            self: ::std::rc::Rc<Self>,
        ) -> Option<::std::rc::Rc<dyn $crate::ast::IXamlAstImperativeNode>> {
            Some(self)
        }
    };
    (@kind type_reference) => {
        fn into_type_reference(
            self: ::std::rc::Rc<Self>,
        ) -> Option<::std::rc::Rc<dyn $crate::ast::IXamlAstTypeReference>> {
            Some(self)
        }
    };
    (@kind property_reference) => {
        fn into_property_reference(
            self: ::std::rc::Rc<Self>,
        ) -> Option<::std::rc::Rc<dyn $crate::ast::IXamlAstPropertyReference>> {
            Some(self)
        }
    };
    (@kind needs_parent_stack) => {
        fn as_needs_parent_stack(&self) -> Option<&dyn $crate::ast::IXamlAstNodeNeedsParentStack> {
            Some(self)
        }
    };
}

/// Implements [`IXamlLineInfo`] by delegating to an embedded [`XamlAstNode`] (or any other
/// `IXamlLineInfo` field): `xaml_line_info_impl!(MyNode, base)`.
#[macro_export]
macro_rules! xaml_line_info_impl {
    ($ty:ty, $($field:tt)+) => {
        impl $crate::ast::IXamlLineInfo for $ty {
            fn line(&self) -> i32 {
                $crate::ast::IXamlLineInfo::line(&self.$($field)+)
            }
            fn position(&self) -> i32 {
                $crate::ast::IXamlLineInfo::position(&self.$($field)+)
            }
            fn set_line(&self, value: i32) {
                $crate::ast::IXamlLineInfo::set_line(&self.$($field)+, value)
            }
            fn set_position(&self, value: i32) {
                $crate::ast::IXamlLineInfo::set_position(&self.$($field)+, value)
            }
        }
    };
}

/// The state of the upstream abstract `XamlAstNode` base class; every node embeds one.
#[derive(Debug, Default)]
pub struct XamlAstNode {
    line: Cell<i32>,
    position: Cell<i32>,
}

impl XamlAstNode {
    pub fn new(line_info: &dyn IXamlLineInfo) -> Self {
        Self {
            line: Cell::new(line_info.line()),
            position: Cell::new(line_info.position()),
        }
    }
}

impl IXamlLineInfo for XamlAstNode {
    fn line(&self) -> i32 {
        self.line.get()
    }
    fn position(&self) -> i32 {
        self.position.get()
    }
    fn set_line(&self, value: i32) {
        self.line.set(value)
    }
    fn set_position(&self, value: i32) {
        self.position.set(value)
    }
}

/// Describes a node handle kind: one of the `dyn IXamlAst*` interfaces or a concrete node type.
/// It provides the C# casts (`node as T`, `(T) node`) between handles.
pub trait XamlAstCast: 'static {
    fn kind_name() -> &'static str;
    /// `node as Self`.
    fn cast_from(node: Rc<dyn IXamlAstNode>) -> Option<Rc<Self>>;
    fn upcast(this: Rc<Self>) -> Rc<dyn IXamlAstNode>;
}

impl XamlAstCast for dyn IXamlAstNode {
    fn kind_name() -> &'static str {
        "IXamlAstNode"
    }
    fn cast_from(node: Rc<dyn IXamlAstNode>) -> Option<Rc<Self>> {
        Some(node)
    }
    fn upcast(this: Rc<Self>) -> Rc<dyn IXamlAstNode> {
        this
    }
}

macro_rules! impl_cast_for_interface {
    ($tr:ident, $name:literal, $method:ident) => {
        impl XamlAstCast for dyn $tr {
            fn kind_name() -> &'static str {
                $name
            }
            fn cast_from(node: Rc<dyn IXamlAstNode>) -> Option<Rc<Self>> {
                node.$method()
            }
            fn upcast(this: Rc<Self>) -> Rc<dyn IXamlAstNode> {
                this
            }
        }
    };
}

impl_cast_for_interface!(IXamlAstValueNode, "IXamlAstValueNode", into_value_node);
impl_cast_for_interface!(
    IXamlAstManipulationNode,
    "IXamlAstManipulationNode",
    into_manipulation_node
);
impl_cast_for_interface!(
    IXamlAstImperativeNode,
    "IXamlAstImperativeNode",
    into_imperative_node
);
impl_cast_for_interface!(
    IXamlAstTypeReference,
    "IXamlAstTypeReference",
    into_type_reference
);
impl_cast_for_interface!(
    IXamlAstPropertyReference,
    "IXamlAstPropertyReference",
    into_property_reference
);

impl<T: IXamlAstNode> XamlAstCast for T {
    fn kind_name() -> &'static str {
        let full = std::any::type_name::<T>();
        full.rsplit("::").next().unwrap_or(full)
    }
    fn cast_from(node: Rc<dyn IXamlAstNode>) -> Option<Rc<Self>> {
        node.into_any_rc().downcast::<T>().ok()
    }
    fn upcast(this: Rc<Self>) -> Rc<dyn IXamlAstNode> {
        this
    }
}

/// `XamlAstNode.Visit`: lets the visitor replace the node, then visits the children of the result.
pub fn visit_node(
    node: &Rc<dyn IXamlAstNode>,
    visitor: &mut dyn IXamlAstVisitor,
) -> XamlResult<Rc<dyn IXamlAstNode>> {
    if node.is_skipped() {
        return Ok(node.clone());
    }

    let node = visitor.visit(node.clone())?;
    visitor.push(node.clone());
    let result = node.visit_children(visitor);
    visitor.pop();
    result?;
    Ok(node)
}

fn invalid_cast<K: ?Sized + XamlAstCast>(from: &'static str) -> XamlError {
    XamlError::invalid_cast(format!(
        "Unable to cast object of type '{from}' to type '{}'.",
        K::kind_name()
    ))
}

/// `(K) node.Visit(visitor)`.
pub fn visit<K: ?Sized + XamlAstCast>(
    node: &Rc<K>,
    visitor: &mut dyn IXamlAstVisitor,
) -> XamlResult<Rc<K>> {
    let result = visit_node(&K::upcast(node.clone()), visitor)?;
    let name = result.type_name();
    K::cast_from(result).ok_or_else(|| invalid_cast::<K>(name))
}

/// `Property = (K) Property.Visit(visitor)`.
pub fn visit_cell<K: ?Sized + XamlAstCast>(
    cell: &RefCell<Rc<K>>,
    visitor: &mut dyn IXamlAstVisitor,
) -> XamlResult<()> {
    let current = cell.borrow().clone();
    let visited = visit(&current, visitor)?;
    *cell.borrow_mut() = visited;
    Ok(())
}

/// `Property = (K?) Property?.Visit(visitor)`.
pub fn visit_optional_cell<K: ?Sized + XamlAstCast>(
    cell: &RefCell<Option<Rc<K>>>,
    visitor: &mut dyn IXamlAstVisitor,
) -> XamlResult<()> {
    let current = cell.borrow().clone();
    if let Some(current) = current {
        let visited = visit(&current, visitor)?;
        *cell.borrow_mut() = Some(visited);
    }
    Ok(())
}

/// `XamlAstNode.VisitList`. The list is not borrowed while a child is being visited,
/// so transformers may inspect it through the parent stack.
pub fn visit_list<K: ?Sized + XamlAstCast>(
    list: &RefCell<Vec<Rc<K>>>,
    visitor: &mut dyn IXamlAstVisitor,
) -> XamlResult<()> {
    let count = list.borrow().len();
    for c in 0..count {
        let Some(current) = list.borrow().get(c).cloned() else {
            break;
        };
        let visited = visit(&current, visitor)?;
        if let Some(slot) = list.borrow_mut().get_mut(c) {
            *slot = visited;
        }
    }
    Ok(())
}

/// Casts and identity checks on node handles.
pub trait XamlAstNodeExtensions {
    /// Upcast to the base node handle.
    fn as_node(&self) -> Rc<dyn IXamlAstNode>;
    /// `node as K`.
    fn cast<K: ?Sized + XamlAstCast>(&self) -> Option<Rc<K>>;
    /// `node is K`.
    fn is<K: ?Sized + XamlAstCast>(&self) -> bool;
    /// `ReferenceEquals(node, other)`.
    fn same_node<O: ?Sized + XamlAstCast>(&self, other: &Rc<O>) -> bool;
}

impl<N: ?Sized + XamlAstCast> XamlAstNodeExtensions for Rc<N> {
    fn as_node(&self) -> Rc<dyn IXamlAstNode> {
        N::upcast(self.clone())
    }
    fn cast<K: ?Sized + XamlAstCast>(&self) -> Option<Rc<K>> {
        K::cast_from(self.as_node())
    }
    fn is<K: ?Sized + XamlAstCast>(&self) -> bool {
        self.cast::<K>().is_some()
    }
    fn same_node<O: ?Sized + XamlAstCast>(&self, other: &Rc<O>) -> bool {
        std::ptr::addr_eq(Rc::as_ptr(self), Rc::as_ptr(other))
    }
}

pub struct SkipXamlAstNode {
    base: XamlAstNode,
}

impl SkipXamlAstNode {
    pub fn new(line_info: &dyn IXamlLineInfo) -> Rc<Self> {
        Rc::new(Self {
            base: XamlAstNode::new(line_info),
        })
    }
}

xaml_line_info_impl!(SkipXamlAstNode, base);

impl IXamlAstNode for SkipXamlAstNode {
    xaml_ast_node_members!("SkipXamlAstNode", value, manipulation);

    fn is_skipped(&self) -> bool {
        true
    }
}

impl IXamlAstValueNode for SkipXamlAstNode {
    fn type_(&self) -> Rc<dyn IXamlAstTypeReference> {
        XamlAstClrTypeReference::new(self, XamlPseudoType::unknown(), false)
    }
}

impl IXamlAstManipulationNode for SkipXamlAstNode {}

/// Subclass of [`XamlValueWithManipulationNode`] used as the placeholder for nodes whose
/// transformation failed with a non-fatal error.
pub struct SkipXamlValueWithManipulationNode {
    base: XamlValueWithManipulationNode,
}

impl SkipXamlValueWithManipulationNode {
    pub fn new(line_info: &dyn IXamlLineInfo) -> Rc<Self> {
        Rc::new(Self {
            base: XamlValueWithManipulationNode::new_inline(
                line_info,
                SkipXamlAstNode::new(line_info),
                None,
            ),
        })
    }
}

xaml_line_info_impl!(SkipXamlValueWithManipulationNode, base);

impl IXamlAstNode for SkipXamlValueWithManipulationNode {
    xaml_ast_node_members!("SkipXamlValueWithManipulationNode", value, manipulation);

    fn is_skipped(&self) -> bool {
        true
    }

    fn as_value_with_side_effect_node_base(&self) -> Option<&XamlValueWithSideEffectNodeBase> {
        Some(&self.base.base)
    }

    fn as_value_with_manipulation_node(&self) -> Option<&XamlValueWithManipulationNode> {
        Some(&self.base)
    }
}

impl IXamlAstValueNode for SkipXamlValueWithManipulationNode {
    fn type_(&self) -> Rc<dyn IXamlAstTypeReference> {
        self.base.type_()
    }
}

impl IXamlAstManipulationNode for SkipXamlValueWithManipulationNode {}

/// `XamlAstExtensions`.
pub trait XamlAstExtensions {
    fn get_clr_type(&self) -> XamlResult<Rc<dyn IXamlType>>;
    fn get_clr_type_reference(&self) -> XamlResult<Rc<XamlAstClrTypeReference>>;
}

impl XamlAstExtensions for Rc<dyn IXamlAstTypeReference> {
    fn get_clr_type(&self) -> XamlResult<Rc<dyn IXamlType>> {
        Ok(self.get_clr_type_reference()?.type_.clone())
    }

    fn get_clr_type_reference(&self) -> XamlResult<Rc<XamlAstClrTypeReference>> {
        self.cast::<XamlAstClrTypeReference>().ok_or_else(|| {
            XamlError::transform_exception(
                format!("Unable to convert {} to CLR type", self.to_node_string()),
                Some(&**self),
            )
        })
    }
}

/// `XamlAstExtensions.GetClrProperty`.
pub trait XamlAstPropertyReferenceExtensions {
    fn get_clr_property(&self) -> XamlResult<Rc<XamlAstClrProperty>>;
}

impl XamlAstPropertyReferenceExtensions for Rc<dyn IXamlAstPropertyReference> {
    fn get_clr_property(&self) -> XamlResult<Rc<XamlAstClrProperty>> {
        self.cast::<XamlAstClrProperty>().ok_or_else(|| {
            XamlError::transform_exception(
                format!(
                    "Unable to convert {} to CLR property",
                    self.to_node_string()
                ),
                Some(&**self),
            )
        })
    }
}
