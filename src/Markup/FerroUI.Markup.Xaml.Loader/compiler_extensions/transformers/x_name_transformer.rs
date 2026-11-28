//! Port of `CompilerExtensions/Transformers/XNameTransformer.cs`.

use std::{
    cell::RefCell,
    collections::HashMap,
    rc::{Rc, Weak},
};

use super::FerroNameScopeRegistrationXamlIlNode;
use xamlx::ast::{
    IXamlAstNode, XamlAstExtensions, XamlAstNamePropertyReference, XamlAstNodeExtensions,
    XamlAstObjectNode, XamlAstXamlPropertyValueNode, XamlAstXmlDirective,
};
use xamlx::exceptions::XamlResult;
use xamlx::transform::{AstTransformationContext, IXamlAstTransformer};
use xamlx::XamlNamespaces;

// Per-transformation provenance: regular Name syntax must never acquire the
// semantics of x:Name. Weak entries cannot retain an AST or match a reused pointer.
#[derive(Default)]
struct XNameAssignments {
    nodes: RefCell<HashMap<usize, Weak<XamlAstXamlPropertyValueNode>>>,
}

/// Converts x:Name directives to regular Name assignments.
pub struct XNameTransformer;

impl IXamlAstTransformer for XNameTransformer {
    fn transform(
        &self,
        context: &AstTransformationContext,
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
                        context
                            .get_or_create_item::<XNameAssignments>()
                            .nodes
                            .borrow_mut()
                            .insert(
                                Rc::as_ptr(&replacement) as usize,
                                Rc::downgrade(&replacement),
                            );
                        on.children.borrow_mut()[c] = replacement;
                    }
                }
            }
        }
        Ok(node)
    }
}

/// After type resolution, register an x:Name directly when the target is a
/// native FerroObject with no Name member. Both back ends already implement
/// the registration node. Controls keep their existing Name assignment path.
pub struct FerroXamlIlRegisterNamedObjectTransformer;
impl IXamlAstTransformer for FerroXamlIlRegisterNamedObjectTransformer {
    fn transform(
        &self,
        context: &AstTransformationContext,
        node: Rc<dyn IXamlAstNode>,
    ) -> XamlResult<Rc<dyn IXamlAstNode>> {
        let Some(value) = node.cast::<XamlAstXamlPropertyValueNode>() else {
            return Ok(node);
        };
        let marked = context
            .get_or_create_item::<XNameAssignments>()
            .nodes
            .borrow_mut()
            .remove(&(Rc::as_ptr(&value) as usize))
            .and_then(|weak| weak.upgrade())
            .is_some_and(|original| Rc::ptr_eq(&original, &value));
        if !marked {
            return Ok(node);
        }
        let property = value.property();
        let Some(reference) = property.cast::<XamlAstNamePropertyReference>() else {
            return Ok(node);
        };
        let target = reference.target_type.borrow().get_clr_type()?;
        // Preserve existing property/event/attached-member resolution, including
        // its diagnostics and setter side effects, whenever Name is declared.
        if target
            .get_all_properties()
            .iter()
            .any(|p| p.name() == "Name")
            || target.get_all_events().iter().any(|p| p.name() == "Name")
            || target
                .methods()
                .iter()
                .any(|m| m.is_static() && matches!(m.name().as_str(), "GetName" | "SetName"))
        {
            return Ok(node);
        }
        let mut ancestor = Some(target.clone());
        let mut native = false;
        while let Some(type_) = ancestor {
            if type_.is("FerroUI", "FerroObject") {
                native = true;
                break;
            }
            ancestor = type_.base_type();
        }
        if !native || value.values.borrow().len() != 1 {
            return Ok(node);
        }
        let name = value.values.borrow()[0].clone();
        Ok(FerroNameScopeRegistrationXamlIlNode::new(
            name,
            Some(target),
        ))
    }
}
