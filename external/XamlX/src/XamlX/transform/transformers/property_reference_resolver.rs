//! Port of `Transform/Transformers/PropertyReferenceResolver.cs`.

use std::rc::Rc;

use crate::ast::{
    IXamlAstNode, XamlAstClrProperty, XamlAstClrTypeReference, XamlAstNamePropertyReference,
    XamlAstNodeExtensions,
};
use crate::exceptions::XamlResult;
use crate::transform::{AstTransformationContext, IXamlAstTransformer};
use crate::type_system::{IXamlMethod, XamlPseudoType};

pub struct PropertyReferenceResolver;

impl IXamlAstTransformer for PropertyReferenceResolver {
    fn transform(
        &self,
        context: &AstTransformationContext,
        node: Rc<dyn IXamlAstNode>,
    ) -> XamlResult<Rc<dyn IXamlAstNode>> {
        let Some(prop) = node.cast::<XamlAstNamePropertyReference>() else {
            return Ok(node);
        };
        let prop_name = prop.name();

        let fake_property = || -> Rc<dyn IXamlAstNode> {
            XamlAstClrProperty::new(&*prop, &prop_name, XamlPseudoType::unknown(), None)
        };

        let declaring_type_reference = prop.declaring_type.borrow().clone();
        let Some(declaring_ref) = declaring_type_reference.cast::<XamlAstClrTypeReference>() else {
            return context.report_transform_error(
                &format!(
                    "Unable to resolve property {} on {}",
                    prop_name,
                    declaring_type_reference.to_node_string()
                ),
                Some(&*node),
                fake_property(),
            );
        };

        let target_type_reference = prop.target_type.borrow().clone();
        let Some(target_ref) = target_type_reference.cast::<XamlAstClrTypeReference>() else {
            return context.report_transform_error(
                &format!(
                    "Unable to resolve property on {}",
                    declaring_type_reference.to_node_string()
                ),
                Some(&*node),
                fake_property(),
            );
        };

        let target_type = target_ref.type_.clone();
        let declaring_type = declaring_ref.type_.clone();

        // Can set normal properties of ancestor types and self
        if declaring_type.is_assignable_from(&*target_type) {
            let found = declaring_type.get_all_properties().into_iter().find(|p| {
                p.name() == prop_name
                    && (p
                        .getter()
                        .is_some_and(|g| !g.is_static() && g.parameters().is_empty())
                        || p.setter()
                            .is_some_and(|s| !s.is_static() && s.parameters().len() == 1))
            });
            if let Some(found) = found {
                return Ok(XamlAstClrProperty::from_property(
                    &*prop,
                    &found,
                    context.configuration(),
                )?);
            }
            let clr_event = declaring_type
                .get_all_events()
                .into_iter()
                .find(|p| p.name() == prop_name && p.add().is_some());
            if let Some(add) = clr_event.and_then(|e| e.add()) {
                return Ok(XamlAstClrProperty::with_setter_methods(
                    &*prop,
                    &prop_name,
                    add.declaring_type(),
                    None,
                    Some(vec![Some(add)]),
                    None,
                )?);
            }
        }

        // Look for attached properties on declaring type
        let mut setter: Option<Rc<dyn IXamlMethod>> = None;
        let mut getter: Option<Rc<dyn IXamlMethod>> = None;
        let mut adder: Option<Rc<dyn IXamlMethod>> = None;
        let setter_name = format!("Set{prop_name}");
        let getter_name = format!("Get{prop_name}");
        let adder_name = format!("Add{prop_name}Handler");
        for m in declaring_type.methods() {
            if m.is_public() && m.is_static() {
                let parameters = m.parameters();
                let name = m.name();
                if name == getter_name
                    && parameters.len() == 1
                    && parameters[0].is_assignable_from(&*target_type)
                {
                    getter = Some(m.clone());
                }

                if name == setter_name
                    && parameters.len() == 2
                    && parameters[0].is_assignable_from(&*target_type)
                {
                    setter = Some(m.clone());
                }

                if name == adder_name
                    && parameters.len() == 2
                    && parameters[0].is_assignable_from(&*target_type)
                {
                    adder = Some(m.clone());
                }
            }
        }

        if setter.is_some() || getter.is_some() {
            let custom_attributes = getter.as_ref().map(|g| g.custom_attributes());
            return Ok(XamlAstClrProperty::with_setter_methods(
                &*prop,
                &prop_name,
                declaring_type,
                getter,
                Some(vec![setter]),
                custom_attributes,
            )?);
        }

        if adder.is_some() {
            return Ok(XamlAstClrProperty::with_setter_methods(
                &*prop,
                &prop_name,
                declaring_type,
                None,
                Some(vec![adder]),
                None,
            )?);
        }

        context.report_transform_error(
            &format!(
                "Unable to resolve suitable regular or attached property {} on type {}",
                prop_name,
                declaring_type.get_fqn()
            ),
            Some(&*node),
            fake_property(),
        )
    }
}
