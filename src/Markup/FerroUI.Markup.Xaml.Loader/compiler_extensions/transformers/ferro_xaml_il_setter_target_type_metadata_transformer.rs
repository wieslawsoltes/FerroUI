//! Port of `CompilerExtensions/Transformers/FerroXamlIlSetterTargetTypeMetadataTransformer.cs`.

use std::rc::Rc;

use xamlx::ast::{
    IXamlAstNode, IXamlAstTypeReference, IXamlAstValueNode, XamlAstNodeExtensions,
    XamlAstObjectNode, XamlAstTextNode, XamlAstXmlDirective, XamlTypeExtensionNode,
};
use xamlx::exceptions::{XamlError, XamlResult};
use xamlx::transform::transformers::TypeReferenceResolver;
use xamlx::transform::{AstTransformationContext, IXamlAstTransformer};
use xamlx::XamlNamespaces;

use crate::compiler_extensions::transformers::{FerroXamlIlTargetTypeMetadataNode, ScopeTypes};

/// Turns the `x:SetterTargetType` directive of an object into a style target-type scope around
/// that object, so that setters outside of a style can resolve their properties.
pub struct FerroXamlIlSetterTargetTypeMetadataTransformer;

impl IXamlAstTransformer for FerroXamlIlSetterTargetTypeMetadataTransformer {
    fn transform(
        &self,
        context: &AstTransformationContext,
        node: Rc<dyn IXamlAstNode>,
    ) -> XamlResult<Rc<dyn IXamlAstNode>> {
        let Some(on) = node.cast::<XamlAstObjectNode>() else {
            return Ok(node);
        };
        let type_directive = on.children.borrow().iter().find_map(|c| {
            let directive = c.cast::<XamlAstXmlDirective>()?;
            let matches = directive.namespace.borrow().as_deref() == Some(XamlNamespaces::XAML2006)
                && *directive.name.borrow() == "SetterTargetType";
            matches.then_some(directive)
        });
        let Some(type_directive) = type_directive else {
            return Ok(node);
        };

        // `Values.Single()`
        let value = {
            let values = type_directive.values.borrow();
            match values.len() {
                1 => values[0].clone(),
                0 => {
                    return Err(XamlError::invalid_operation("Sequence contains no elements"));
                }
                _ => {
                    return Err(XamlError::invalid_operation(
                        "Sequence contains more than one element",
                    ));
                }
            }
        };
        let type_: Option<Rc<dyn IXamlAstTypeReference>> =
            if let Some(type_node) = value.cast::<XamlTypeExtensionNode>() {
                Some(type_node.value())
            } else if let Some(tn) = value.cast::<XamlAstTextNode>() {
                Some(TypeReferenceResolver::resolve_type_by_xml_name(
                    context,
                    &tn.text(),
                    false,
                    &*tn,
                    true,
                )?)
            } else {
                None
            };
        on.children
            .borrow_mut()
            .retain(|c| !c.same_node(&type_directive));

        let Some(type_) = type_ else {
            return Err(XamlError::transform_exception(
                "Unable to resolve SetterTargetType type",
                Some(&*type_directive),
            ));
        };
        let value: Rc<dyn IXamlAstValueNode> = on;
        Ok(FerroXamlIlTargetTypeMetadataNode::new(
            value,
            type_,
            ScopeTypes::Style,
        ))
    }
}
