//! Port of `CompilerExtensions/Transformers/FerroXamlIlTransitionsTypeMetadataTransformer.cs`.

use std::rc::Rc;

use xamlx::ast::{
    IXamlAstNode, XamlAstNodeExtensions, XamlAstObjectNode, XamlAstPropertyReferenceExtensions,
    XamlAstXamlPropertyValueNode,
};
use xamlx::exceptions::XamlResult;
use xamlx::transform::{AstTransformationContext, IXamlAstTransformer};

use super::{FerroXamlIlTargetTypeMetadataNode, FerroXamlIlWellKnownTypesExtensions, ScopeTypes};

/// Wraps the values of every property of type `Transitions` in a target-type metadata node of
/// the `Transitions` scope, so that registered property names inside are looked up on the type
/// of the object that owns the property.
pub struct FerroXamlIlTransitionsTypeMetadataTransformer;

impl IXamlAstTransformer for FerroXamlIlTransitionsTypeMetadataTransformer {
    fn transform(
        &self,
        context: &AstTransformationContext,
        node: Rc<dyn IXamlAstNode>,
    ) -> XamlResult<Rc<dyn IXamlAstNode>> {
        if let Some(on) = node.cast::<XamlAstObjectNode>() {
            let children = on.children.borrow().clone();
            for ch in children {
                let Some(pn) = ch.cast::<XamlAstXamlPropertyValueNode>() else {
                    continue;
                };
                let transitions = context.try_get_ferro_types()?.transitions.clone();
                let is_transitions = pn
                    .property()
                    .get_clr_property()?
                    .getter()
                    .is_some_and(|getter| getter.return_type().equals(&*transitions));
                if is_transitions {
                    let count = pn.values.borrow().len();
                    for c in 0..count {
                        let value = pn.values.borrow()[c].clone();
                        let wrapped = FerroXamlIlTargetTypeMetadataNode::new(
                            value,
                            on.type_.borrow().clone(),
                            ScopeTypes::Transitions,
                        );
                        pn.values.borrow_mut()[c] = wrapped;
                    }
                }
            }
        }
        Ok(node)
    }
}
