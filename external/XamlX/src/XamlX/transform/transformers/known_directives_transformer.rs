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
        let Some(ni) = node.cast::<XamlAstObjectNode>() else {
            return Ok(node);
        };
        let Some(type_) = ni.type_.borrow().cast::<XamlAstXmlTypeReference>() else {
            return Ok(node);
        };

        let known_directives = context.configuration().known_directives.borrow().clone();
        for (ns, name) in known_directives {
            if type_.xml_namespace().as_deref() == Some(ns.as_str()) && type_.name() == name {
                let mut vnodes: Vec<Rc<dyn IXamlAstValueNode>> = Vec::new();
                // As upstream: the first child (of any kind) is reported and ends the transformation.
                let first_child = ni.children.borrow().first().cloned();
                if let Some(ch) = first_child {
                    if let Some(vn) = ch.cast::<dyn IXamlAstValueNode>() {
                        vnodes.push(vn);
                    }
                    context.report_transform_error(
                        "Only value nodes are allowed as directive children elements",
                        Some(&*ch),
                        (),
                    )?;
                    return Ok(ni);
                }

                return Ok(XamlAstXmlDirective::new(
                    &*ni,
                    type_.xml_namespace().as_deref(),
                    &type_.name(),
                    vnodes,
                ));
            }
        }

        Ok(node)
    }
}
