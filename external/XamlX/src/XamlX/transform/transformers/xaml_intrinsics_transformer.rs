//! Port of `Transform/Transformers/XamlIntrinsicsTransformer.cs`.

use std::rc::Rc;

use crate::ast::{
    IXamlAstNode, XamlAstNamePropertyReference, XamlAstNodeExtensions, XamlAstObjectNode,
    XamlAstTextNode, XamlAstXamlPropertyValueNode, XamlAstXmlTypeReference, XamlConstantNode,
    XamlNullExtensionNode, XamlStaticExtensionNode, XamlTypeExtensionNode,
};
use crate::exceptions::{XamlError, XamlResult};
use crate::parsers::{CommaSeparatedParenthesesTreeParser, Node};
use crate::transform::transformers::TypeReferenceResolver;
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

/// The type of an `x:Type` or `x:Static` expression: a name, optionally with inline generic
/// arguments (`scg:Dictionary(x:String, scg:List(x:Int32))`) and a nullable indicator
/// (`x:Int32?`, a `Nullable<T>` for a value type). Inline arguments and `x:TypeArguments`
/// cannot be mixed.
fn resolve_type_expression(
    context: &AstTransformationContext,
    xml: &XamlAstXmlTypeReference,
    extension_name: &str,
    type_expression: &str,
    text_node: &Rc<XamlAstTextNode>,
) -> XamlResult<Rc<XamlAstXmlTypeReference>> {
    let has_inline_generic_syntax = type_expression.contains(['(', ')']);
    let generic_arguments = xml.generic_arguments.borrow().clone();

    if !generic_arguments.is_empty() && has_inline_generic_syntax {
        return Err(XamlError::transform_exception(
            format!("{extension_name} generic arguments cannot be specified both inline and in x:TypeArguments"),
            Some(&**text_node),
        ));
    }

    if has_inline_generic_syntax {
        let tree = CommaSeparatedParenthesesTreeParser::parse(type_expression).map_err(|e| {
            XamlError::transform_exception(format!("Unable to parse {extension_name}: {e}"), Some(&**text_node))
        })?;
        if tree.len() != 1 {
            return Err(XamlError::transform_exception(
                format!("{extension_name} should contain exactly one type expression"),
                Some(&**text_node),
            ));
        }
        return convert_node(context, extension_name, &tree[0], text_node);
    }

    let type_reference = resolve_type_name(context, type_expression, text_node, None)?;
    if !generic_arguments.is_empty() {
        *type_reference.generic_arguments.borrow_mut() = generic_arguments;
    }
    Ok(type_reference)
}

fn convert_node(
    context: &AstTransformationContext,
    extension_name: &str,
    parsed_node: &Node,
    text_node: &Rc<XamlAstTextNode>,
) -> XamlResult<Rc<XamlAstXmlTypeReference>> {
    let value = parsed_node.value.as_deref().unwrap_or("");
    if value.trim().is_empty() {
        return Err(XamlError::transform_exception(
            format!("{extension_name} contains an empty type name"),
            Some(&**text_node),
        ));
    }
    let generic_arguments = parsed_node
        .children
        .iter()
        .map(|child| convert_node(context, extension_name, child, text_node))
        .collect::<XamlResult<Vec<_>>>()?;
    resolve_type_name(context, value.trim(), text_node, Some(generic_arguments))
}

fn resolve_type_name(
    context: &AstTransformationContext,
    type_name: &str,
    text_node: &Rc<XamlAstTextNode>,
    generic_arguments: Option<Vec<Rc<XamlAstXmlTypeReference>>>,
) -> XamlResult<Rc<XamlAstXmlTypeReference>> {
    let (type_name, is_nullable) = match type_name.strip_suffix('?') {
        Some(type_name) => (type_name, true),
        None => (type_name, false),
    };
    if type_name.contains('?') {
        return Err(XamlError::transform_exception(
            format!("Type {type_name}? is cannot contain multiple nullable indicators ('?')"),
            Some(&**text_node),
        ));
    }

    let (prefix, name) = type_name.split_once(':').unwrap_or(("", type_name));
    let Some(resolved_ns) = context.try_get_namespace_alias_at(prefix.trim(), &**text_node) else {
        return Err(XamlError::transform_exception(
            format!("Unable to resolve namespace {prefix}"),
            Some(&**text_node),
        ));
    };

    let type_reference = XamlAstXmlTypeReference::with_generic_arguments(
        &**text_node,
        Some(&resolved_ns),
        name.trim(),
        generic_arguments.unwrap_or_default(),
    );

    if is_nullable {
        let resolved = TypeReferenceResolver::resolve_type_reference(context, &type_reference)?;
        if resolved.type_.is_value_type() {
            return Ok(XamlAstXmlTypeReference::with_generic_arguments(
                &**text_node,
                Some(XamlNamespaces::XAML2006),
                "Nullable`1",
                vec![type_reference],
            ));
        }
    }

    Ok(type_reference)
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
            let type_reference = resolve_type_expression(context, &xml, "x:Type", type_ref_text, &text_node)?;

            return Ok(XamlTypeExtensionNode::new(&*node, type_reference, well_known_types.type_.clone()));
        }

        if xml_name == "Static" {
            let text_node = resolve_argument_or_value(&node, &ni, "x:Static", "Member")?;
            let text = text_node.text();
            let type_ref_text = text.trim_matches(char::is_whitespace);

            let Some((type_expression, member)) = type_ref_text.split_once('.') else {
                return Err(XamlError::transform_exception(
                    format!("Unable to parse {type_ref_text} as 'type.member'"),
                    Some(&*text_node),
                ));
            };

            let type_reference = resolve_type_expression(context, &xml, "x:Static", type_expression, &text_node)?;
            return Ok(XamlStaticExtensionNode::new(&ni, Some(type_reference), member));
        }

        Ok(node)
    }
}
