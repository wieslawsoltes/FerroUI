//! Port of `Transform/Transformers/XArgumentsTransformer.cs`.

use std::rc::Rc;

use crate::ast::{
    IXamlAstNode, IXamlAstValueNode, XamlAstNodeExtensions, XamlAstObjectNode,
    XamlAstXmlTypeReference,
};
use crate::exceptions::XamlResult;
use crate::transform::{AstTransformationContext, IXamlAstTransformer, WhitespaceNormalization};
use crate::xaml_namespaces::XamlNamespaces;

pub struct XArgumentsTransformer;

impl IXamlAstTransformer for XArgumentsTransformer {
    fn transform(
        &self,
        context: &AstTransformationContext,
        node: Rc<dyn IXamlAstNode>,
    ) -> XamlResult<Rc<dyn IXamlAstNode>> {
        if let Some(ni) = node.cast::<XamlAstObjectNode>() {
            let arg_directives: Vec<Rc<XamlAstObjectNode>> = ni
                .children
                .borrow()
                .iter()
                .filter_map(|c| c.cast::<XamlAstObjectNode>())
                .filter(|d| {
                    d.type_
                        .borrow()
                        .cast::<XamlAstXmlTypeReference>()
                        .is_some_and(|xref| {
                            xref.xml_namespace().as_deref() == Some(XamlNamespaces::XAML2006)
                                && xref.name() == "Arguments"
                        })
                })
                .collect();
            if arg_directives.len() > 1 {
                context.report_transform_error(
                    "x:Arguments directive is specified more than once",
                    Some(&*arg_directives[1]),
                    (),
                )?;
                return Ok(node);
            }

            if arg_directives.is_empty() {
                return Ok(node);
            }

            let mut arguments: Vec<Rc<dyn IXamlAstValueNode>> = arg_directives[0]
                .children
                .borrow()
                .iter()
                .filter_map(|c| c.cast::<dyn IXamlAstValueNode>())
                .collect();
            {
                let mut children = ni.children.borrow_mut();
                if let Some(index) = children
                    .iter()
                    .position(|c| c.same_node(&arg_directives[0]))
                {
                    children.remove(index);
                }
            }

            // This is needed to remove whitespace-only nodes between actual object elements or text nodes
            WhitespaceNormalization::remove_whitespace_nodes(&mut arguments);
            *ni.arguments.borrow_mut() = arguments;
        }

        Ok(node)
    }
}
