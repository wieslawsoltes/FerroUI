//! Port of `Ast/Xml.cs`.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use crate::exceptions::XamlResult;
use crate::{xaml_ast_node_members, xaml_line_info_impl};

use super::{
    visit_list, IXamlAstNode, IXamlAstTypeReference, IXamlAstVisitor, IXamlLineInfo, XamlAstNode,
};

pub struct XamlAstXmlTypeReference {
    base: XamlAstNode,
    pub xml_namespace: RefCell<Option<String>>,
    pub name: RefCell<String>,
    pub is_markup_extension: Cell<bool>,
    pub generic_arguments: RefCell<Vec<Rc<XamlAstXmlTypeReference>>>,
}

impl XamlAstXmlTypeReference {
    pub fn new(line_info: &dyn IXamlLineInfo, xml_namespace: Option<&str>, name: &str) -> Rc<Self> {
        Self::with_generic_arguments(line_info, xml_namespace, name, Vec::new())
    }

    pub fn with_generic_arguments(
        line_info: &dyn IXamlLineInfo,
        xml_namespace: Option<&str>,
        name: &str,
        generic_arguments: Vec<Rc<XamlAstXmlTypeReference>>,
    ) -> Rc<Self> {
        Rc::new(Self {
            base: XamlAstNode::new(line_info),
            xml_namespace: RefCell::new(xml_namespace.map(str::to_string)),
            name: RefCell::new(name.to_string()),
            is_markup_extension: Cell::new(false),
            generic_arguments: RefCell::new(generic_arguments),
        })
    }

    pub fn xml_namespace(&self) -> Option<String> {
        self.xml_namespace.borrow().clone()
    }

    pub fn name(&self) -> String {
        self.name.borrow().clone()
    }
}

xaml_line_info_impl!(XamlAstXmlTypeReference, base);

impl IXamlAstNode for XamlAstXmlTypeReference {
    xaml_ast_node_members!("XamlAstXmlTypeReference", type_reference);

    fn visit_children(&self, visitor: &mut dyn IXamlAstVisitor) -> XamlResult<()> {
        visit_list(&self.generic_arguments, visitor)
    }

    fn to_node_string(&self) -> String {
        format!(
            "xml!!{}:{}",
            self.xml_namespace.borrow().as_deref().unwrap_or(""),
            self.name.borrow()
        )
    }
}

impl IXamlAstTypeReference for XamlAstXmlTypeReference {
    fn is_markup_extension(&self) -> bool {
        self.is_markup_extension.get()
    }

    fn equals(&self, other: &dyn IXamlAstTypeReference) -> bool {
        match other.as_any().downcast_ref::<XamlAstXmlTypeReference>() {
            Some(xml) => {
                *xml.name.borrow() == *self.name.borrow()
                    && *xml.xml_namespace.borrow() == *self.xml_namespace.borrow()
                    && xml.is_markup_extension.get() == self.is_markup_extension.get()
            }
            None => false,
        }
    }
}
