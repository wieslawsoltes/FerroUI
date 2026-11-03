//! Port of `Transform/Transformers/KnownDirectivesTransformer.cs`.

use std::rc::Rc;

use crate::ast::{
    IXamlAstNode, IXamlAstValueNode, XamlAstNodeExtensions, XamlAstObjectNode, XamlAstXmlDirective,
    XamlAstXmlTypeReference,
};
use crate::exceptions::XamlResult;
use crate::transform::{AstTransformationContext, IXamlAstTransformer};

pub struct KnownDirectivesTransformer;

impl IXamlAstTransformer for KnownDirectivesTransformer {
    fn transform(
        &self,
        context: &AstTransformationContext,
        node: Rc<dyn IXamlAstNode>,
    ) -> XamlResult<Rc<dyn IXamlAstNode>> {
        let Some(object) = node.as_any().downcast_ref::<XamlAstObjectNode>() else {
            return Ok(node);
        };
        let known = &context.configuration().known_directives;
        if known.borrow().is_empty() {
            return Ok(node);
        }

        let first_child = {
            let directives = known.borrow();
            let Some((ns, name)) = match_directive(object, &directives) else {
                return Ok(node);
            };
            let children = object.children.borrow();
            match children.first() {
                None => return Ok(XamlAstXmlDirective::new(object, Some(ns), name, Vec::new())),
                Some(child) => child.clone(),
            }
        };
        // The original transformer queries the first child's value role before
        // reporting it. Release the configuration and AST borrows first so a
        // diagnostic handler can modify them.
        let _value = first_child.cast::<dyn IXamlAstValueNode>();
        context.report_transform_error(
            "Only value nodes are allowed as directive children elements",
            Some(&*first_child),
            (),
        )?;
        Ok(node)
    }
}

fn match_directive<'a>(
    object: &XamlAstObjectNode,
    directives: &'a [(String, String)],
) -> Option<&'a (String, String)> {
    let type_ref = object.type_.borrow();
    let xml = type_ref.as_any().downcast_ref::<XamlAstXmlTypeReference>()?;
    let ns = xml.xml_namespace.borrow();
    let name = xml.name.borrow();
    directives.iter().find(|(n, k)| ns.as_deref() == Some(n.as_str()) && k == &*name)
}
