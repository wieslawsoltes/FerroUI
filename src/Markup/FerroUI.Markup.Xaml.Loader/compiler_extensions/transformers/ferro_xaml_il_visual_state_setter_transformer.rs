//! The setters of visual states (`<Setter Target="Element.Property" Value="..."/>` in
//! `VisualState.Setters`, WinUI's `Setter.Target`): an addition with no upstream counterpart
//! (docs/porting/winui-calculator-gaps.md, section 5.9).
//!
//! The text of the target names an element of the name scope of the document (or of the
//! template) and a property of that element: `Element.Property`, or `Element.(Owner.Property)`
//! for an attached property. The transformer finds the element by its name, and then:
//!
//! - gives the setter the property (`Setter.Property`, the text after the element name) and a
//!   style target-type scope of the type of the element around the setter, as
//!   `x:SetterTargetType` does, so that [`FerroXamlIlSetterTransformer`] resolves the property and
//!   converts the value as for a setter of a style;
//! - replaces the text of the target with a `TargetPropertyPath` whose `TargetName` is the name
//!   and whose `Path` is the text of the property. The element is found by that name in the name
//!   scope of the template or the document when the state applies, as WinUI resolves the target
//!   of a visual state setter, so a setter may name an element declared after it.
//!
//! [`FerroXamlIlSetterTransformer`]: super::FerroXamlIlSetterTransformer

use std::rc::Rc;

use xamlx::ast::{
    IXamlAstNode, IXamlAstPropertyReference, IXamlAstTypeReference, IXamlAstValueNode, XamlAstClrProperty,
    XamlAstClrTypeReference, XamlAstExtensions, XamlAstNodeExtensions, XamlAstObjectNode,
    XamlAstPropertyReferenceExtensions,
    XamlAstTextNode, XamlAstXamlPropertyValueNode,
};
use xamlx::exceptions::{XamlError, XamlResult};
use xamlx::transform::{AstTransformationContext, IXamlAstTransformer};
use xamlx::type_system::IXamlType;

use super::ferro_xaml_il_setter_transformer::XamlStyleTransformException;
use crate::compiler_extensions::transformers::{FerroXamlIlTargetTypeMetadataNode, ScopeTypes};

pub struct FerroXamlIlVisualStateSetterTransformer;

impl IXamlAstTransformer for FerroXamlIlVisualStateSetterTransformer {
    fn transform(
        &self,
        context: &AstTransformationContext,
        node: Rc<dyn IXamlAstNode>,
    ) -> XamlResult<Rc<dyn IXamlAstNode>> {
        let Some(on) = node.cast::<XamlAstObjectNode>() else {
            return Ok(node);
        };
        let setter_type = on.type_().get_clr_type()?;
        if !setter_type.is("FerroUI.Styling", "Setter") {
            return Ok(node);
        }
        let Some(target_value) = find_property_value(&on, "Target")? else {
            return Ok(node);
        };
        let text = {
            let values = target_value.values.borrow();
            match values.as_slice() {
                [value] => value.cast::<XamlAstTextNode>(),
                _ => None,
            }
        };
        // A target written as an object (`<Setter.Target><TargetPropertyPath .../>`) is left as
        // it is.
        let Some(text) = text else {
            return Ok(node);
        };

        let parents = context.parent_nodes();
        let in_visual_state = parents.iter().any(|parent| {
            parent
                .cast::<XamlAstObjectNode>()
                .and_then(|object| object.type_().get_clr_type().ok())
                .is_some_and(|type_| type_.is("FerroUI.Controls", "VisualState"))
        });
        if !in_visual_state {
            return Err(XamlStyleTransformException::new(
                "Setter.Target is only valid in the setters of a visual state.",
                &*text,
                None,
            ));
        }
        if find_property_value(&on, "Property")?.is_some() {
            return Err(XamlStyleTransformException::new(
                "A setter of a visual state names its property in Setter.Target, not in Setter.Property.",
                &*text,
                None,
            ));
        }

        let target_text = text.text();
        let Some((element_name, path)) = split_target(&target_text) else {
            return Err(XamlStyleTransformException::new(
                format!("Setter.Target '{target_text}' is not of the form Element.Property or Element.(Owner.Property)."),
                &*text,
                None,
            ));
        };
        let Some(element_type) = find_named_element_type(&parents, &element_name)? else {
            return Err(XamlStyleTransformException::new(
                format!("Setter.Target '{target_text}': no element named '{element_name}' in the name scope of the setter."),
                &*text,
                None,
            ));
        };
        let property_text = path.strip_prefix('(').and_then(|p| p.strip_suffix(')')).unwrap_or(&path).to_string();

        // Setter.Property: the text of the property, resolved against the element's type by the
        // setter transformer.
        let property_property = clr_property(context, &*text, &setter_type, "Property")?;
        let property_value: Rc<dyn IXamlAstNode> = XamlAstXamlPropertyValueNode::new(
            &*text,
            property_property,
            string_node(context, &*text, &property_text),
            true,
        );
        // First: `Setter.Value` depends on `Setter.Property` (its value is converted to the type of
        // the property), and the assignments were already ordered by their dependencies.
        on.children.borrow_mut().insert(0, property_value);

        // Setter.Target: a TargetPropertyPath with the element name and the path.
        let path_type = context.configuration().type_system.get_type("FerroUI.Styling.TargetPropertyPath")?;
        let path_type_reference: Rc<dyn IXamlAstTypeReference> =
            XamlAstClrTypeReference::new(&*text, path_type.clone(), false);
        let path_object = XamlAstObjectNode::new(&*text, path_type_reference);
        {
            let mut children = path_object.children.borrow_mut();
            let target: Rc<dyn IXamlAstNode> = XamlAstXamlPropertyValueNode::new(
                &*text,
                clr_property(context, &*text, &path_type, "TargetName")?,
                string_node(context, &*text, &element_name),
                true,
            );
            children.push(target);
            let path_value: Rc<dyn IXamlAstNode> = XamlAstXamlPropertyValueNode::new(
                &*text,
                clr_property(context, &*text, &path_type, "Path")?,
                string_node(context, &*text, &path),
                true,
            );
            children.push(path_value);
        }
        let path_object: Rc<dyn IXamlAstValueNode> = path_object;
        *target_value.values.borrow_mut() = vec![path_object];

        let value: Rc<dyn IXamlAstValueNode> = on.clone();
        Ok(FerroXamlIlTargetTypeMetadataNode::new(
            value,
            XamlAstClrTypeReference::new(&*on, element_type, false),
            ScopeTypes::Style,
        ))
    }
}

/// `Element.Property` or `Element.(Owner.Property)`: the element name and the property path.
fn split_target(text: &str) -> Option<(String, String)> {
    let text = text.trim();
    let (element, path) = text.split_once('.')?;
    let element = element.trim();
    let path = path.trim();
    if element.is_empty() || path.is_empty() {
        return None;
    }
    let valid_path = if let Some(inner) = path.strip_prefix('(') {
        inner.strip_suffix(')').is_some_and(|inner| inner.contains('.') && !inner.contains(['(', ')']))
    } else {
        !path.contains(['.', '(', ')'])
    };
    valid_path.then(|| (element.to_string(), path.to_string()))
}

/// The resolved property `name` of `type_` (a property node as the property reference resolver
/// writes it).
fn clr_property(
    context: &AstTransformationContext,
    line: &dyn xamlx::ast::IXamlLineInfo,
    type_: &Rc<dyn IXamlType>,
    name: &str,
) -> XamlResult<Rc<dyn IXamlAstPropertyReference>> {
    let property = type_
        .get_all_properties()
        .into_iter()
        .find(|property| property.name() == name)
        .ok_or_else(|| XamlError::transform_exception(format!("{} has no property {name}", type_.get_fqn()), Some(line)))?;
    let property: Rc<XamlAstClrProperty> = XamlAstClrProperty::from_property(line, &property, context.configuration())?;
    Ok(property)
}

/// The type of the element named `name` in the name scope of the setter: the nearest enclosing
/// template (whose content is a name scope of its own) or the document. The search goes from
/// the innermost ancestor outwards, so the element of the nearest scope wins; it does not enter
/// the content of nested templates.
fn find_named_element_type(parents: &[Rc<dyn IXamlAstNode>], name: &str) -> XamlResult<Option<Rc<dyn IXamlType>>> {
    for parent in parents {
        if let Some(object) = parent.cast::<XamlAstObjectNode>() {
            if let Some(found) = search_object(&object, name)? {
                return Ok(Some(found));
            }
        }
        if let Some(property_value) = parent.cast::<XamlAstXamlPropertyValueNode>() {
            if is_template_content(&property_value)? {
                // The boundary of the name scope: the content of the template was searched from
                // the ancestors inside it.
                return Ok(None);
            }
        }
    }
    Ok(None)
}

fn search_object(object: &Rc<XamlAstObjectNode>, name: &str) -> XamlResult<Option<Rc<dyn IXamlType>>> {
    let children = object.children.borrow().clone();
    for child in &children {
        if let Some(property_value) = child.cast::<XamlAstXamlPropertyValueNode>() {
            let property = property_value.property();
            if property_name(&property).as_deref() == Some("Name") {
                let values = property_value.values.borrow();
                if let [value] = values.as_slice() {
                    if value.cast::<XamlAstTextNode>().is_some_and(|text| text.text() == name) {
                        return Ok(Some(object.type_().get_clr_type()?));
                    }
                }
            }
        }
    }
    for child in &children {
        if let Some(found) = search_node(child, name)? {
            return Ok(Some(found));
        }
    }
    Ok(None)
}

fn search_node(node: &Rc<dyn IXamlAstNode>, name: &str) -> XamlResult<Option<Rc<dyn IXamlType>>> {
    if let Some(object) = node.cast::<XamlAstObjectNode>() {
        return search_object(&object, name);
    }
    if let Some(property_value) = node.cast::<XamlAstXamlPropertyValueNode>() {
        if is_template_content(&property_value)? {
            return Ok(None);
        }
        let values = property_value.values.borrow().clone();
        for value in &values {
            let value: Rc<dyn IXamlAstNode> = value.clone();
            if let Some(found) = search_node(&value, name)? {
                return Ok(Some(found));
            }
        }
        return Ok(None);
    }
    if let Some(metadata) = node.cast::<FerroXamlIlTargetTypeMetadataNode>() {
        let value: Rc<dyn IXamlAstNode> = metadata.value();
        return search_node(&value, name);
    }
    Ok(None)
}

fn property_name(property: &Rc<dyn IXamlAstPropertyReference>) -> Option<String> {
    if let Some(clr) = property.cast::<XamlAstClrProperty>() {
        return Some(clr.name());
    }
    property.cast::<xamlx::ast::XamlAstNamePropertyReference>().map(|named| named.name())
}

/// Whether the values of a property are the content of a template (a name scope of their own).
fn is_template_content(property_value: &XamlAstXamlPropertyValueNode) -> XamlResult<bool> {
    let Ok(property) = property_value.property().get_clr_property() else {
        return Ok(false);
    };
    let is_template = property
        .custom_attributes
        .borrow()
        .iter()
        .any(|attribute| attribute.type_().is("FerroUI.Metadata", "TemplateContentAttribute"));
    Ok(is_template)
}

/// `on.Children.OfType<XamlAstXamlPropertyValueNode>().FirstOrDefault(x =>
/// x.Property.GetClrProperty().Name == name)`.
fn find_property_value(on: &XamlAstObjectNode, name: &str) -> XamlResult<Option<Rc<XamlAstXamlPropertyValueNode>>> {
    let children = on.children.borrow().clone();
    for child in children {
        if let Some(property_value) = child.cast::<XamlAstXamlPropertyValueNode>() {
            if property_value.property().get_clr_property()?.name() == name {
                return Ok(Some(property_value));
            }
        }
    }
    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::split_target;

    #[test]
    fn targets_are_an_element_and_a_property() {
        let split = |text: &str| split_target(text).map(|(e, p)| format!("{e}|{p}"));
        assert_eq!(split("Button.FontSize").as_deref(), Some("Button|FontSize"));
        assert_eq!(split(" Root.(Grid.Row) ").as_deref(), Some("Root|(Grid.Row)"));
        assert_eq!(split("Root"), None);
        assert_eq!(split(".FontSize"), None);
        assert_eq!(split("Root."), None);
        assert_eq!(split("Root.A.B"), None);
        assert_eq!(split("Root.(Row)"), None);
        assert_eq!(split("Root.(Grid.Row"), None);
    }
}

/// A text value created after the type references were resolved: typed as the CLR string, which
/// the resolver would have given a text of the document.
fn string_node(context: &AstTransformationContext, line: &dyn xamlx::ast::IXamlLineInfo, text: &str) -> Rc<XamlAstTextNode> {
    XamlAstTextNode::with_type(line, text, true, Some(context.configuration().well_known_types().string.clone()))
}
