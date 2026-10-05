//! Port of `CompilerExtensions/Transformers/XNameTransformer.cs`.

use std::rc::Rc;

use xamlx::ast::{
    IXamlAstNode, XamlAstNamePropertyReference, XamlAstNodeExtensions, XamlAstObjectNode,
    XamlAstXamlPropertyValueNode, XamlAstXmlDirective,
};
use xamlx::exceptions::XamlResult;
use xamlx::transform::{AstTransformationContext, IXamlAstTransformer};
use xamlx::XamlNamespaces;

/// Converts x:Name directives to regular Name assignments.
pub struct XNameTransformer;

impl IXamlAstTransformer for XNameTransformer {
    fn transform(
        &self,
        _context: &AstTransformationContext,
        node: Rc<dyn IXamlAstNode>,
    ) -> XamlResult<Rc<dyn IXamlAstNode>> {
        if let Some(on) = node.cast::<XamlAstObjectNode>() {
            let count = on.children.borrow().len();
            for c in 0..count {
                let ch = on.children.borrow()[c].clone();
                if let Some(d) = ch.cast::<XamlAstXmlDirective>() {
                    if d.namespace.borrow().as_deref() == Some(XamlNamespaces::XAML2006)
                        && *d.name.borrow() == "Name"
                    {
                        let type_ = on.type_.borrow().clone();
                        let replacement = XamlAstXamlPropertyValueNode::with_values(
                            &*d,
                            XamlAstNamePropertyReference::new(&*d, type_.clone(), "Name", type_),
                            d.values.borrow().clone(),
                            true,
                        );
                        on.children.borrow_mut()[c] = replacement;
                    }
                }
            }
        }
        Ok(node)
    }
}
