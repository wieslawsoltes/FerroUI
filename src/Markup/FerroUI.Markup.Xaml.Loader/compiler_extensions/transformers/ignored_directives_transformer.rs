//! Port of `CompilerExtensions/Transformers/IgnoredDirectivesTransformer.cs`.

use std::rc::Rc;

use xamlx::ast::{IXamlAstNode, XamlAstNodeExtensions, XamlAstObjectNode, XamlAstXmlDirective};
use xamlx::exceptions::XamlResult;
use xamlx::transform::{AstTransformationContext, IXamlAstTransformer};
use xamlx::XamlNamespaces;

/// Removes the `x:Precompile`, `x:Class`, `x:FieldModifier` and `x:ClassModifier` directives.
pub struct IgnoredDirectivesTransformer;

impl IXamlAstTransformer for IgnoredDirectivesTransformer {
    fn transform(
        &self,
        _context: &AstTransformationContext,
        node: Rc<dyn IXamlAstNode>,
    ) -> XamlResult<Rc<dyn IXamlAstNode>> {
        if let Some(no) = node.cast::<XamlAstObjectNode>() {
            no.children.borrow_mut().retain(|child| {
                let Some(d) = child.cast::<XamlAstXmlDirective>() else {
                    return true;
                };
                if d.namespace.borrow().as_deref() != Some(XamlNamespaces::XAML2006) {
                    return true;
                }
                let name = d.name.borrow();
                !(*name == "Precompile"
                    || *name == "Class"
                    || *name == "FieldModifier"
                    || *name == "ClassModifier")
            });
        }
        Ok(node)
    }
}
