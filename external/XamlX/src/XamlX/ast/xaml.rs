//! Port of `Ast/Xaml.cs`.

use std::cell::RefCell;
use std::rc::Rc;

use crate::exceptions::XamlResult;
use crate::type_system::IXamlType;
use crate::xaml_namespaces::XamlNamespaces;
use crate::{xaml_ast_node_members, xaml_line_info_impl};

use super::{
    visit_cell, visit_list, IXamlAstManipulationNode, IXamlAstNode, IXamlAstPropertyReference,
    IXamlAstTypeReference, IXamlAstValueNode, IXamlAstVisitor, IXamlLineInfo,
    XamlAstClrTypeReference, XamlAstNode, XamlAstXmlTypeReference,
};

pub struct XamlAstXmlDirective {
    base: XamlAstNode,
    pub namespace: RefCell<Option<String>>,
    pub name: RefCell<String>,
    pub values: RefCell<Vec<Rc<dyn IXamlAstValueNode>>>,
}

impl XamlAstXmlDirective {
    pub fn new(
        line_info: &dyn IXamlLineInfo,
        ns: Option<&str>,
        name: &str,
        values: Vec<Rc<dyn IXamlAstValueNode>>,
    ) -> Rc<Self> {
        Rc::new(Self {
            base: XamlAstNode::new(line_info),
            namespace: RefCell::new(ns.map(str::to_string)),
            name: RefCell::new(name.to_string()),
            values: RefCell::new(values),
        })
    }
}

xaml_line_info_impl!(XamlAstXmlDirective, base);

impl IXamlAstNode for XamlAstXmlDirective {
    xaml_ast_node_members!("XamlAstXmlDirective", manipulation);

    fn visit_children(&self, visitor: &mut dyn IXamlAstVisitor) -> XamlResult<()> {
        visit_list(&self.values, visitor)
    }
}

impl IXamlAstManipulationNode for XamlAstXmlDirective {}

pub struct XamlAstXamlPropertyValueNode {
    base: XamlAstNode,
    pub property: RefCell<Rc<dyn IXamlAstPropertyReference>>,
    pub values: RefCell<Vec<Rc<dyn IXamlAstValueNode>>>,
    pub is_attribute_syntax: bool,
}

impl XamlAstXamlPropertyValueNode {
    /// Constructor taking a single value.
    pub fn new(
        line_info: &dyn IXamlLineInfo,
        property: Rc<dyn IXamlAstPropertyReference>,
        value: Rc<dyn IXamlAstValueNode>,
        is_attribute_syntax: bool,
    ) -> Rc<Self> {
        Self::with_values(line_info, property, vec![value], is_attribute_syntax)
    }

    /// Constructor taking a list of values.
    pub fn with_values(
        line_info: &dyn IXamlLineInfo,
        property: Rc<dyn IXamlAstPropertyReference>,
        values: Vec<Rc<dyn IXamlAstValueNode>>,
        is_attribute_syntax: bool,
    ) -> Rc<Self> {
        Rc::new(Self {
            base: XamlAstNode::new(line_info),
            property: RefCell::new(property),
            values: RefCell::new(values),
            is_attribute_syntax,
        })
    }

    pub fn property(&self) -> Rc<dyn IXamlAstPropertyReference> {
        self.property.borrow().clone()
    }
}

xaml_line_info_impl!(XamlAstXamlPropertyValueNode, base);

impl IXamlAstNode for XamlAstXamlPropertyValueNode {
    xaml_ast_node_members!("XamlAstXamlPropertyValueNode", manipulation);

    fn visit_children(&self, visitor: &mut dyn IXamlAstVisitor) -> XamlResult<()> {
        visit_cell(&self.property, visitor)?;
        visit_list(&self.values, visitor)
    }
}

impl IXamlAstManipulationNode for XamlAstXamlPropertyValueNode {}

pub struct XamlAstObjectNode {
    base: XamlAstNode,
    pub type_: RefCell<Rc<dyn IXamlAstTypeReference>>,
    pub children: RefCell<Vec<Rc<dyn IXamlAstNode>>>,
    pub arguments: RefCell<Vec<Rc<dyn IXamlAstValueNode>>>,
}

impl XamlAstObjectNode {
    pub fn new(line_info: &dyn IXamlLineInfo, type_: Rc<dyn IXamlAstTypeReference>) -> Rc<Self> {
        Rc::new(Self {
            base: XamlAstNode::new(line_info),
            type_: RefCell::new(type_),
            children: RefCell::new(Vec::new()),
            arguments: RefCell::new(Vec::new()),
        })
    }
}

xaml_line_info_impl!(XamlAstObjectNode, base);

impl IXamlAstNode for XamlAstObjectNode {
    xaml_ast_node_members!("XamlAstObjectNode", value);

    fn visit_children(&self, visitor: &mut dyn IXamlAstVisitor) -> XamlResult<()> {
        visit_cell(&self.type_, visitor)?;
        visit_list(&self.arguments, visitor)?;
        visit_list(&self.children, visitor)
    }
}

impl IXamlAstValueNode for XamlAstObjectNode {
    fn type_(&self) -> Rc<dyn IXamlAstTypeReference> {
        self.type_.borrow().clone()
    }
}

pub struct XamlAstTextNode {
    base: XamlAstNode,
    pub text: RefCell<String>,
    /// Indicates whether this value was created from an XML node where xml:space="preserve" was in effect.
    pub preserve_whitespace: bool,
    pub type_: RefCell<Rc<dyn IXamlAstTypeReference>>,
}

impl XamlAstTextNode {
    /// `preserve_whitespace` is true if XAML whitespace normalization should NOT be applied to this
    /// text value (i.e. xml:space="preserve" or attribute values).
    pub fn new(line_info: &dyn IXamlLineInfo, text: &str, preserve_whitespace: bool) -> Rc<Self> {
        Self::with_type(line_info, text, preserve_whitespace, None)
    }

    pub fn with_type(
        line_info: &dyn IXamlLineInfo,
        text: &str,
        preserve_whitespace: bool,
        type_: Option<Rc<dyn IXamlType>>,
    ) -> Rc<Self> {
        let type_ref: Rc<dyn IXamlAstTypeReference> = match type_ {
            Some(t) => XamlAstClrTypeReference::new(line_info, t, false),
            None => {
                XamlAstXmlTypeReference::new(line_info, Some(XamlNamespaces::XAML2006), "String")
            }
        };
        Rc::new(Self {
            base: XamlAstNode::new(line_info),
            text: RefCell::new(text.to_string()),
            preserve_whitespace,
            type_: RefCell::new(type_ref),
        })
    }

    pub fn text(&self) -> String {
        self.text.borrow().clone()
    }
}

xaml_line_info_impl!(XamlAstTextNode, base);

impl IXamlAstNode for XamlAstTextNode {
    xaml_ast_node_members!("XamlAstTextNode", value);

    fn visit_children(&self, visitor: &mut dyn IXamlAstVisitor) -> XamlResult<()> {
        visit_cell(&self.type_, visitor)
    }
}

impl IXamlAstValueNode for XamlAstTextNode {
    fn type_(&self) -> Rc<dyn IXamlAstTypeReference> {
        self.type_.borrow().clone()
    }
}

pub struct XamlAstNamePropertyReference {
    base: XamlAstNode,
    pub declaring_type: RefCell<Rc<dyn IXamlAstTypeReference>>,
    pub name: RefCell<String>,
    pub target_type: RefCell<Rc<dyn IXamlAstTypeReference>>,
}

impl XamlAstNamePropertyReference {
    pub fn new(
        line_info: &dyn IXamlLineInfo,
        declaring_type: Rc<dyn IXamlAstTypeReference>,
        name: &str,
        target_type: Rc<dyn IXamlAstTypeReference>,
    ) -> Rc<Self> {
        Rc::new(Self {
            base: XamlAstNode::new(line_info),
            declaring_type: RefCell::new(declaring_type),
            name: RefCell::new(name.to_string()),
            target_type: RefCell::new(target_type),
        })
    }

    pub fn name(&self) -> String {
        self.name.borrow().clone()
    }
}

xaml_line_info_impl!(XamlAstNamePropertyReference, base);

impl IXamlAstNode for XamlAstNamePropertyReference {
    xaml_ast_node_members!("XamlAstNamePropertyReference", property_reference);

    fn visit_children(&self, visitor: &mut dyn IXamlAstVisitor) -> XamlResult<()> {
        visit_cell(&self.declaring_type, visitor)?;
        visit_cell(&self.target_type, visitor)
    }
}

impl IXamlAstPropertyReference for XamlAstNamePropertyReference {}
