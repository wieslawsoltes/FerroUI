//! Port of `Ast/Intrinsics.cs`.
//!
//! All nodes in this file implement `IXamlAstEmitableNode` for the IL backend upstream;
//! that part is not ported (see the crate documentation for the list).

use std::cell::RefCell;
use std::rc::Rc;

use crate::exceptions::{XamlError, XamlResult};
use crate::type_system::{
    IXamlField, IXamlProperty, IXamlType, XamlPseudoType, XamlTypeWellKnownTypes, XamlValue,
};
use crate::{xaml_ast_node_members, xaml_line_info_impl};

use super::{
    visit_cell, visit_optional_cell, IXamlAstNode, IXamlAstTypeReference, IXamlAstValueNode,
    IXamlAstVisitor, IXamlLineInfo, XamlAstClrTypeReference, XamlAstExtensions, XamlAstNode,
    XamlAstObjectNode, XamlValueWithSideEffectNodeBase,
};

pub struct XamlNullExtensionNode {
    base: XamlAstNode,
    type_: Rc<dyn IXamlAstTypeReference>,
}

impl XamlNullExtensionNode {
    pub fn new(line_info: &dyn IXamlLineInfo) -> Rc<Self> {
        Rc::new(Self {
            base: XamlAstNode::new(line_info),
            type_: XamlAstClrTypeReference::new(line_info, XamlPseudoType::null(), false),
        })
    }
}

xaml_line_info_impl!(XamlNullExtensionNode, base);

impl IXamlAstNode for XamlNullExtensionNode {
    xaml_ast_node_members!("XamlNullExtensionNode", value);
}

impl IXamlAstValueNode for XamlNullExtensionNode {
    fn type_(&self) -> Rc<dyn IXamlAstTypeReference> {
        self.type_.clone()
    }
}

pub struct XamlTypeExtensionNode {
    base: XamlAstNode,
    system_type: Rc<dyn IXamlType>,
    type_: Rc<dyn IXamlAstTypeReference>,
    pub value: RefCell<Rc<dyn IXamlAstTypeReference>>,
}

impl XamlTypeExtensionNode {
    pub fn new(
        line_info: &dyn IXamlLineInfo,
        value: Rc<dyn IXamlAstTypeReference>,
        system_type: Rc<dyn IXamlType>,
    ) -> Rc<Self> {
        Rc::new(Self {
            base: XamlAstNode::new(line_info),
            type_: XamlAstClrTypeReference::new(line_info, system_type.clone(), false),
            system_type,
            value: RefCell::new(value),
        })
    }

    pub fn value(&self) -> Rc<dyn IXamlAstTypeReference> {
        self.value.borrow().clone()
    }

    /// `_systemType`: `System.Type` (private upstream; exposed for emitter backends).
    pub fn system_type(&self) -> &Rc<dyn IXamlType> {
        &self.system_type
    }
}

xaml_line_info_impl!(XamlTypeExtensionNode, base);

impl IXamlAstNode for XamlTypeExtensionNode {
    xaml_ast_node_members!("XamlTypeExtensionNode", value);

    fn visit_children(&self, visitor: &mut dyn IXamlAstVisitor) -> XamlResult<()> {
        visit_cell(&self.value, visitor)
    }
}

impl IXamlAstValueNode for XamlTypeExtensionNode {
    fn type_(&self) -> Rc<dyn IXamlAstTypeReference> {
        self.type_.clone()
    }
}

/// The result of `XamlStaticExtensionNode.ResolveMember`: an `IXamlMember` that is either a
/// property or a field.
#[derive(Clone)]
pub enum XamlStaticMember {
    Property(Rc<dyn IXamlProperty>),
    Field(Rc<dyn IXamlField>),
}

impl XamlStaticMember {
    pub fn name(&self) -> String {
        match self {
            XamlStaticMember::Property(p) => p.name(),
            XamlStaticMember::Field(f) => f.name(),
        }
    }

    pub fn declaring_type(&self) -> Rc<dyn IXamlType> {
        match self {
            XamlStaticMember::Property(p) => p.declaring_type(),
            XamlStaticMember::Field(f) => f.declaring_type(),
        }
    }
}

pub struct XamlStaticExtensionNode {
    base: XamlAstNode,
    pub member: RefCell<String>,
    pub target_type: RefCell<Option<Rc<dyn IXamlAstTypeReference>>>,
}

impl XamlStaticExtensionNode {
    pub fn new(
        line_info: &XamlAstObjectNode,
        target_type: Option<Rc<dyn IXamlAstTypeReference>>,
        member: &str,
    ) -> Rc<Self> {
        Rc::new(Self {
            base: XamlAstNode::new(line_info),
            member: RefCell::new(member.to_string()),
            target_type: RefCell::new(target_type),
        })
    }

    pub fn resolve_member(&self, throw_on_unknown: bool) -> XamlResult<Option<XamlStaticMember>> {
        let target_type = self.target_type.borrow().clone();
        let type_ = match &target_type {
            Some(t) => Some(t.get_clr_type()?),
            None => None,
        };
        let member_name = self.member.borrow().clone();
        let mut member = None;
        if let Some(type_) = &type_ {
            member = type_
                .get_all_fields()
                .into_iter()
                .find(|f| f.is_public() && f.is_static() && f.name() == member_name)
                .map(XamlStaticMember::Field);
            if member.is_none() {
                member = type_
                    .get_all_properties()
                    .into_iter()
                    .find(|p| {
                        p.name() == member_name
                            && p.getter().is_some_and(|g| g.is_public() && g.is_static())
                    })
                    .map(XamlStaticMember::Property);
            }
        }

        if member.is_some() {
            Ok(member)
        } else if throw_on_unknown {
            Err(XamlError::transform_exception(
                format!(
                    "Unable to resolve \"{}.{}\" as static field, property, constant or enum value",
                    type_.map(|t| t.name()).unwrap_or_default(),
                    member_name
                ),
                Some(self),
            ))
        } else {
            Ok(None)
        }
    }
}

xaml_line_info_impl!(XamlStaticExtensionNode, base);

impl IXamlAstNode for XamlStaticExtensionNode {
    xaml_ast_node_members!("XamlStaticExtensionNode", value);

    fn visit_children(&self, visitor: &mut dyn IXamlAstVisitor) -> XamlResult<()> {
        visit_optional_cell(&self.target_type, visitor)
    }
}

impl IXamlAstValueNode for XamlStaticExtensionNode {
    /// Upstream throws from this getter while the target type is still unresolved; the port
    /// reports the unknown pseudo type in that case.
    fn type_(&self) -> Rc<dyn IXamlAstTypeReference> {
        let type_ = match self.resolve_member(false) {
            Ok(Some(XamlStaticMember::Field(field))) => field.field_type(),
            Ok(Some(XamlStaticMember::Property(prop))) => match prop.getter() {
                Some(getter) => getter.return_type(),
                None => XamlPseudoType::unknown(),
            },
            _ => XamlPseudoType::unknown(),
        };
        XamlAstClrTypeReference::new(self, type_, false)
    }
}

pub struct XamlConstantNode {
    base: XamlAstNode,
    pub constant: XamlValue,
    type_: Rc<dyn IXamlAstTypeReference>,
}

impl XamlConstantNode {
    pub fn new(
        line_info: &dyn IXamlLineInfo,
        type_: Rc<dyn IXamlType>,
        constant: XamlValue,
    ) -> XamlResult<Rc<Self>> {
        if !constant.is_primitive() && !matches!(constant, XamlValue::String(_)) {
            return Err(XamlError::argument(format!(
                "Don't know how to emit {} constant",
                constant.type_name()
            )));
        }
        Ok(Rc::new(Self {
            base: XamlAstNode::new(line_info),
            constant,
            type_: XamlAstClrTypeReference::new(line_info, type_, false),
        }))
    }
}

xaml_line_info_impl!(XamlConstantNode, base);

impl IXamlAstNode for XamlConstantNode {
    xaml_ast_node_members!("XamlConstantNode", value);
}

impl IXamlAstValueNode for XamlConstantNode {
    fn type_(&self) -> Rc<dyn IXamlAstTypeReference> {
        self.type_.clone()
    }
}

pub struct XamlRootObjectNode {
    base: XamlAstNode,
    pub type_: RefCell<Rc<dyn IXamlAstTypeReference>>,
}

impl XamlRootObjectNode {
    pub fn new(root: &XamlAstObjectNode) -> Rc<Self> {
        Rc::new(Self {
            base: XamlAstNode::new(root),
            type_: RefCell::new(root.type_()),
        })
    }
}

xaml_line_info_impl!(XamlRootObjectNode, base);

impl IXamlAstNode for XamlRootObjectNode {
    xaml_ast_node_members!("XamlRootObjectNode", value);

    fn visit_children(&self, visitor: &mut dyn IXamlAstVisitor) -> XamlResult<()> {
        visit_cell(&self.type_, visitor)
    }
}

impl IXamlAstValueNode for XamlRootObjectNode {
    fn type_(&self) -> Rc<dyn IXamlAstTypeReference> {
        self.type_.borrow().clone()
    }
}

pub struct XamlIntermediateRootObjectNode {
    base: XamlAstNode,
    pub type_: RefCell<Rc<dyn IXamlAstTypeReference>>,
}

impl XamlIntermediateRootObjectNode {
    pub fn new(line_info: &dyn IXamlLineInfo, types: &XamlTypeWellKnownTypes) -> Rc<Self> {
        Rc::new(Self {
            base: XamlAstNode::new(line_info),
            type_: RefCell::new(XamlAstClrTypeReference::new(
                line_info,
                types.object.clone(),
                false,
            )),
        })
    }
}

xaml_line_info_impl!(XamlIntermediateRootObjectNode, base);

impl IXamlAstNode for XamlIntermediateRootObjectNode {
    xaml_ast_node_members!("XamlIntermediateRootObjectNode", value);

    fn visit_children(&self, visitor: &mut dyn IXamlAstVisitor) -> XamlResult<()> {
        visit_cell(&self.type_, visitor)
    }
}

impl IXamlAstValueNode for XamlIntermediateRootObjectNode {
    fn type_(&self) -> Rc<dyn IXamlAstTypeReference> {
        self.type_.borrow().clone()
    }
}

pub struct XamlLoadMethodDelegateNode {
    pub base: XamlValueWithSideEffectNodeBase,
    pub delegate_type: Rc<dyn IXamlType>,
    pub method: Rc<dyn crate::type_system::IXamlMethod>,
    type_: Rc<dyn IXamlAstTypeReference>,
}

impl XamlLoadMethodDelegateNode {
    pub fn new(
        line_info: &dyn IXamlLineInfo,
        value: Rc<dyn IXamlAstValueNode>,
        delegate_type: Rc<dyn IXamlType>,
        method: Rc<dyn crate::type_system::IXamlMethod>,
    ) -> Rc<Self> {
        let type_ = XamlAstClrTypeReference::new(&*value, delegate_type.clone(), false);
        Rc::new(Self {
            base: XamlValueWithSideEffectNodeBase::new(line_info, value),
            delegate_type,
            method,
            type_,
        })
    }
}

xaml_line_info_impl!(XamlLoadMethodDelegateNode, base);

impl IXamlAstNode for XamlLoadMethodDelegateNode {
    xaml_ast_node_members!("XamlLoadMethodDelegateNode", value);

    fn visit_children(&self, visitor: &mut dyn IXamlAstVisitor) -> XamlResult<()> {
        self.base.visit_children(visitor)
    }

    fn as_value_with_side_effect_node_base(&self) -> Option<&XamlValueWithSideEffectNodeBase> {
        Some(&self.base)
    }
}

impl IXamlAstValueNode for XamlLoadMethodDelegateNode {
    fn type_(&self) -> Rc<dyn IXamlAstTypeReference> {
        self.type_.clone()
    }
}
