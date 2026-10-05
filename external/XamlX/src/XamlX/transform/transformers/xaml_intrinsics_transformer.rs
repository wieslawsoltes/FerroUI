//! Port of `Transform/Transformers/XamlIntrinsicsTransformer.cs`.

use std::rc::Rc;

use crate::ast::{
    IXamlAstNode, XamlAstNamePropertyReference, XamlAstNodeExtensions, XamlAstObjectNode,
    XamlAstTextNode, XamlAstXamlPropertyValueNode, XamlAstXmlTypeReference, XamlConstantNode,
    XamlNullExtensionNode, XamlStaticExtensionNode, XamlTypeExtensionNode,
};
use crate::exceptions::{XamlError, XamlResult};
use crate::transform::{AstTransformationContext, IXamlAstTransformer};
use crate::type_system::XamlValue;
use crate::xaml_namespaces::XamlNamespaces;

pub struct XamlIntrinsicsTransformer;

fn resolve_argument_or_value(
    node: &Rc<dyn IXamlAstNode>,
    ni: &XamlAstObjectNode,
    extension: &str,
    name: &str,
) -> XamlResult<Rc<XamlAstTextNode>> {
    let mut value: Option<Rc<dyn IXamlAstNode>> = None;

    let arguments = ni.arguments.borrow();
    let children = ni.children.borrow();
    if arguments.len() == 1 && children.is_empty() {
        value = Some(arguments[0].as_node());
    } else if arguments.is_empty() && children.len() == 1 {
        if let Some(pnode) = children[0].cast::<XamlAstXamlPropertyValueNode>() {
            if let Some(pref) = pnode.property().cast::<XamlAstNamePropertyReference>() {
                let values = pnode.values.borrow();
                if pref.name() == name && values.len() == 1 {
                    value = Some(values[0].as_node());
                }
            }
        }
    }

    let Some(value) = value else {
        return Err(XamlError::transform_exception(
            format!(
                "{extension} extension should take exactly one constructor parameter without any content OR {name} property"
            ),
            Some(&**node),
        ));
    };

    value.cast::<XamlAstTextNode>().ok_or_else(|| {
        XamlError::transform_exception("x:Type parameter should be a text node", Some(&**node))
    })
}

impl IXamlAstTransformer for XamlIntrinsicsTransformer {
    fn transform(
        &self,
        context: &AstTransformationContext,
        node: Rc<dyn IXamlAstNode>,
    ) -> XamlResult<Rc<dyn IXamlAstNode>> {
        let Some(ni) = node.cast::<XamlAstObjectNode>() else {
            return Ok(node);
        };
        let Some(xml) = ni.type_.borrow().cast::<XamlAstXmlTypeReference>() else {
            return Ok(node);
        };
        if xml.xml_namespace().as_deref() != Some(XamlNamespaces::XAML2006) {
            return Ok(node);
        }

        let xml_name = xml.name();
        let well_known_types = context.configuration().well_known_types();

        if xml_name == "Null" {
            return Ok(XamlNullExtensionNode::new(&*node));
        }
        if xml_name == "True" {
            return Ok(XamlConstantNode::new(
                &*node,
                well_known_types.boolean.clone(),
                XamlValue::Boolean(true),
            )?);
        }
        if xml_name == "False" {
            return Ok(XamlConstantNode::new(
                &*node,
                well_known_types.boolean.clone(),
                XamlValue::Boolean(false),
            )?);
        }

        if xml_name == "Type" {
            let text_node = resolve_argument_or_value(&node, &ni, "x:Type", "TypeName")?;
            let text = text_node.text();
            let type_ref_text = text.trim_matches(char::is_whitespace);
            let (prefix, type_name) = match type_ref_text.split_once(':') {
                Some((prefix, type_name)) => (prefix, type_name),
                None => ("", type_ref_text),
            };
            let Some(resolved_ns) =
                context.try_get_namespace_alias(prefix.trim_matches(char::is_whitespace))
            else {
                return context.report_transform_error(
                    &format!("Unable to resolve namespace {prefix}"),
                    Some(&*text_node),
                    node,
                );
            };

            return Ok(XamlTypeExtensionNode::new(
                &*node,
                XamlAstXmlTypeReference::with_generic_arguments(
                    &*text_node,
                    Some(&resolved_ns),
                    type_name,
                    xml.generic_arguments.borrow().clone(),
                ),
                well_known_types.type_.clone(),
            ));
        }

        if xml_name == "Static" {
            let text_node = resolve_argument_or_value(&node, &ni, "x:Static", "Member")?;
            let text = text_node.text();
            let trimmed = text.trim_matches(char::is_whitespace);
            let (ns, type_and_member) = match trimmed.split_once(':') {
                Some((ns, type_and_member)) => (ns, type_and_member),
                None => ("", trimmed),
            };

            let Some((type_name, member)) = type_and_member.split_once('.') else {
                return Err(XamlError::transform_exception(
                    format!("Unable to parse {type_and_member} as 'type.member'"),
                    Some(&*text_node),
                ));
            };

            let Some(resolved_ns) = context.try_get_namespace_alias(ns) else {
                return Err(XamlError::transform_exception(
                    format!("Unable to resolve namespace {ns}"),
                    Some(&*text_node),
                ));
            };

            return Ok(XamlStaticExtensionNode::new(
                &ni,
                Some(XamlAstXmlTypeReference::with_generic_arguments(
                    &*ni,
                    Some(&resolved_ns),
                    type_name,
                    xml.generic_arguments.borrow().clone(),
                )),
                member,
            ));
        }

        Ok(node)
    }
}
