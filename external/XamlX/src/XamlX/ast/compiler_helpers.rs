//! Port of `Ast/CompilerHelpers.cs`.
//!
//! All nodes in this file implement `IXamlAstEmitableNode`/`IXamlAstLocalsEmitableNode` for the
//! IL backend upstream; that part is not ported (see the crate documentation for the list).

use std::cell::RefCell;
use std::rc::Rc;

use crate::exceptions::{XamlError, XamlResult};
use crate::type_system::IXamlType;
use crate::{xaml_ast_node_members, xaml_line_info_impl};

use super::{
    visit, visit_cell, IXamlAstImperativeNode, IXamlAstManipulationNode, IXamlAstNode,
    IXamlAstNodeNeedsParentStack, IXamlAstTypeReference, IXamlAstValueNode, IXamlAstVisitor,
    IXamlLineInfo, XamlAstClrTypeReference, XamlAstExtensions, XamlAstNode, XamlAstNodeExtensions,
    XamlValueWithSideEffectNodeBase,
};

pub struct XamlAstCompilerLocalNode {
    base: XamlAstNode,
    type_reference: Rc<XamlAstClrTypeReference>,
    pub type_: Rc<dyn IXamlType>,
}

impl XamlAstCompilerLocalNode {
    pub fn new(line_info: &dyn IXamlLineInfo, type_: Rc<XamlAstClrTypeReference>) -> Rc<Self> {
        Rc::new(Self {
            base: XamlAstNode::new(line_info),
            type_: type_.type_.clone(),
            type_reference: type_,
        })
    }

    /// `XamlAstCompilerLocalNode(IXamlAstValueNode value)`.
    pub fn from_value(value: &Rc<dyn IXamlAstValueNode>) -> XamlResult<Rc<Self>> {
        Ok(Self::new(&**value, value.type_().get_clr_type_reference()?))
    }
}

xaml_line_info_impl!(XamlAstCompilerLocalNode, base);

impl IXamlAstNode for XamlAstCompilerLocalNode {
    xaml_ast_node_members!("XamlAstCompilerLocalNode", value);
}

impl IXamlAstValueNode for XamlAstCompilerLocalNode {
    fn type_(&self) -> Rc<dyn IXamlAstTypeReference> {
        self.type_reference.clone()
    }
}

pub struct XamlAstLocalInitializationNodeEmitter {
    pub base: XamlValueWithSideEffectNodeBase,
    pub local: RefCell<Rc<XamlAstCompilerLocalNode>>,
}

impl XamlAstLocalInitializationNodeEmitter {
    pub fn new(
        line_info: &dyn IXamlLineInfo,
        value: Rc<dyn IXamlAstValueNode>,
        local: Rc<XamlAstCompilerLocalNode>,
    ) -> Rc<Self> {
        Rc::new(Self {
            base: XamlValueWithSideEffectNodeBase::new(line_info, value),
            local: RefCell::new(local),
        })
    }

    pub fn local(&self) -> Rc<XamlAstCompilerLocalNode> {
        self.local.borrow().clone()
    }
}

xaml_line_info_impl!(XamlAstLocalInitializationNodeEmitter, base);

impl IXamlAstNode for XamlAstLocalInitializationNodeEmitter {
    xaml_ast_node_members!("XamlAstLocalInitializationNodeEmitter", value);

    fn visit_children(&self, visitor: &mut dyn IXamlAstVisitor) -> XamlResult<()> {
        self.base.visit_children(visitor)?;
        visit_cell(&self.local, visitor)
    }

    fn as_value_with_side_effect_node_base(&self) -> Option<&XamlValueWithSideEffectNodeBase> {
        Some(&self.base)
    }
}

impl IXamlAstValueNode for XamlAstLocalInitializationNodeEmitter {
    fn type_(&self) -> Rc<dyn IXamlAstTypeReference> {
        self.base.type_()
    }
}

pub struct XamlValueNodeWithBeginInit {
    pub base: XamlValueWithSideEffectNodeBase,
}

impl XamlValueNodeWithBeginInit {
    pub fn new(value: Rc<dyn IXamlAstValueNode>) -> Rc<Self> {
        Rc::new(Self {
            base: XamlValueWithSideEffectNodeBase::new(&*value.clone(), value),
        })
    }
}

xaml_line_info_impl!(XamlValueNodeWithBeginInit, base);

impl IXamlAstNode for XamlValueNodeWithBeginInit {
    xaml_ast_node_members!("XamlValueNodeWithBeginInit", value);

    fn visit_children(&self, visitor: &mut dyn IXamlAstVisitor) -> XamlResult<()> {
        self.base.visit_children(visitor)
    }

    fn as_value_with_side_effect_node_base(&self) -> Option<&XamlValueWithSideEffectNodeBase> {
        Some(&self.base)
    }
}

impl IXamlAstValueNode for XamlValueNodeWithBeginInit {
    fn type_(&self) -> Rc<dyn IXamlAstTypeReference> {
        self.base.type_()
    }
}

pub struct XamlAstManipulationImperativeNode {
    base: XamlAstNode,
    pub imperative: RefCell<Rc<dyn IXamlAstImperativeNode>>,
}

impl XamlAstManipulationImperativeNode {
    pub fn new(
        line_info: &dyn IXamlLineInfo,
        imperative: Rc<dyn IXamlAstImperativeNode>,
    ) -> Rc<Self> {
        Rc::new(Self {
            base: XamlAstNode::new(line_info),
            imperative: RefCell::new(imperative),
        })
    }

    pub fn imperative(&self) -> Rc<dyn IXamlAstImperativeNode> {
        self.imperative.borrow().clone()
    }
}

xaml_line_info_impl!(XamlAstManipulationImperativeNode, base);

impl IXamlAstNode for XamlAstManipulationImperativeNode {
    xaml_ast_node_members!("XamlAstManipulationImperativeNode", manipulation);

    fn visit_children(&self, visitor: &mut dyn IXamlAstVisitor) -> XamlResult<()> {
        visit_cell(&self.imperative, visitor)
    }
}

impl IXamlAstManipulationNode for XamlAstManipulationImperativeNode {}

pub struct XamlAstImperativeValueManipulation {
    base: XamlAstNode,
    pub value: RefCell<Rc<dyn IXamlAstValueNode>>,
    pub manipulation: RefCell<Rc<dyn IXamlAstManipulationNode>>,
}

impl XamlAstImperativeValueManipulation {
    pub fn new(
        line_info: &dyn IXamlLineInfo,
        value: Rc<dyn IXamlAstValueNode>,
        manipulation: Rc<dyn IXamlAstManipulationNode>,
    ) -> Rc<Self> {
        Rc::new(Self {
            base: XamlAstNode::new(line_info),
            value: RefCell::new(value),
            manipulation: RefCell::new(manipulation),
        })
    }

    pub fn value(&self) -> Rc<dyn IXamlAstValueNode> {
        self.value.borrow().clone()
    }

    pub fn manipulation(&self) -> Rc<dyn IXamlAstManipulationNode> {
        self.manipulation.borrow().clone()
    }
}

xaml_line_info_impl!(XamlAstImperativeValueManipulation, base);

impl IXamlAstNode for XamlAstImperativeValueManipulation {
    xaml_ast_node_members!("XamlAstImperativeValueManipulation", imperative);

    fn visit_children(&self, visitor: &mut dyn IXamlAstVisitor) -> XamlResult<()> {
        // Upstream: Value = (XamlAstCompilerLocalNode) Value.Visit(visitor);
        let current = self.value.borrow().clone();
        let visited = visit(&current, visitor)?;
        if !visited.is::<XamlAstCompilerLocalNode>() {
            return Err(XamlError::invalid_cast(format!(
                "Unable to cast object of type '{}' to type 'XamlAstCompilerLocalNode'.",
                visited.type_name()
            )));
        }
        *self.value.borrow_mut() = visited;
        visit_cell(&self.manipulation, visitor)
    }
}

impl IXamlAstImperativeNode for XamlAstImperativeValueManipulation {}

pub struct XamlAstContextLocalNode {
    base: XamlAstNode,
    type_: Rc<dyn IXamlAstTypeReference>,
}

impl XamlAstContextLocalNode {
    pub fn new(line_info: &dyn IXamlLineInfo, type_: Rc<dyn IXamlType>) -> Rc<Self> {
        Rc::new(Self {
            base: XamlAstNode::new(line_info),
            type_: XamlAstClrTypeReference::new(line_info, type_, false),
        })
    }
}

xaml_line_info_impl!(XamlAstContextLocalNode, base);

impl IXamlAstNode for XamlAstContextLocalNode {
    xaml_ast_node_members!("XamlAstContextLocalNode", value);
}

impl IXamlAstValueNode for XamlAstContextLocalNode {
    fn type_(&self) -> Rc<dyn IXamlAstTypeReference> {
        self.type_.clone()
    }
}

pub struct XamlAstRuntimeCastNode {
    base: XamlAstNode,
    pub value: RefCell<Rc<dyn IXamlAstValueNode>>,
    pub type_: RefCell<Rc<dyn IXamlAstTypeReference>>,
}

impl XamlAstRuntimeCastNode {
    pub fn new(
        line_info: &dyn IXamlLineInfo,
        value: Rc<dyn IXamlAstValueNode>,
        cast_to: Rc<dyn IXamlAstTypeReference>,
    ) -> Rc<Self> {
        Rc::new(Self {
            base: XamlAstNode::new(line_info),
            value: RefCell::new(value),
            type_: RefCell::new(cast_to),
        })
    }

    pub fn value(&self) -> Rc<dyn IXamlAstValueNode> {
        self.value.borrow().clone()
    }
}

xaml_line_info_impl!(XamlAstRuntimeCastNode, base);

impl IXamlAstNode for XamlAstRuntimeCastNode {
    xaml_ast_node_members!("XamlAstRuntimeCastNode", value);

    fn visit_children(&self, visitor: &mut dyn IXamlAstVisitor) -> XamlResult<()> {
        visit_cell(&self.value, visitor)?;
        visit_cell(&self.type_, visitor)
    }
}

impl IXamlAstValueNode for XamlAstRuntimeCastNode {
    fn type_(&self) -> Rc<dyn IXamlAstTypeReference> {
        self.type_.borrow().clone()
    }
}

pub struct XamlAstNeedsParentStackValueNode {
    pub base: XamlValueWithSideEffectNodeBase,
}

impl XamlAstNeedsParentStackValueNode {
    pub fn new(line_info: &dyn IXamlLineInfo, value: Rc<dyn IXamlAstValueNode>) -> Rc<Self> {
        Rc::new(Self {
            base: XamlValueWithSideEffectNodeBase::new(line_info, value),
        })
    }
}

xaml_line_info_impl!(XamlAstNeedsParentStackValueNode, base);

impl IXamlAstNode for XamlAstNeedsParentStackValueNode {
    xaml_ast_node_members!(
        "XamlAstNeedsParentStackValueNode",
        value,
        needs_parent_stack
    );

    fn visit_children(&self, visitor: &mut dyn IXamlAstVisitor) -> XamlResult<()> {
        self.base.visit_children(visitor)
    }

    fn as_value_with_side_effect_node_base(&self) -> Option<&XamlValueWithSideEffectNodeBase> {
        Some(&self.base)
    }
}

impl IXamlAstValueNode for XamlAstNeedsParentStackValueNode {
    fn type_(&self) -> Rc<dyn IXamlAstTypeReference> {
        self.base.type_()
    }
}

impl IXamlAstNodeNeedsParentStack for XamlAstNeedsParentStackValueNode {
    fn needs_parent_stack(&self) -> bool {
        true
    }
}
