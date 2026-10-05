//! Port of `CompilerExtensions/Transformers/XDataTypeTransformer.cs`.

use std::rc::Rc;

use xamlx::ast::{
    IXamlAstNode, XamlAstClrTypeReference, XamlAstNamePropertyReference, XamlAstNodeExtensions,
    XamlAstObjectNode, XamlAstXamlPropertyValueNode, XamlAstXmlDirective,
};
use xamlx::exceptions::{XamlError, XamlResult};
use xamlx::transform::{AstTransformationContext, IXamlAstTransformer};
use xamlx::type_system::{IXamlProperty, IXamlType};
use xamlx::XamlNamespaces;

use super::FerroXamlIlWellKnownTypesExtensions;

const DATA_TYPE_PROPERTY_NAME: &str = "DataType";

pub struct XDataTypeTransformer;

impl XDataTypeTransformer {
    /// `GetAllProperties(t)`: the properties of the type and of its base types, each with the
    /// type that declares it, most derived first.
    fn get_all_properties(t: &Rc<dyn IXamlType>) -> Vec<(Rc<dyn IXamlType>, Rc<dyn IXamlProperty>)> {
        let mut rv = Vec::new();
        let mut current = Some(t.clone());
        while let Some(type_) = current {
            for p in type_.properties() {
                rv.push((type_.clone(), p));
            }
            current = type_.base_type();
        }
        rv
    }
}

impl IXamlAstTransformer for XDataTypeTransformer {
    /// Converts x:DataType directives to regular DataType assignments if property with
    /// `FerroUI.Metadata.DataTypeAttribute` exists.
    fn transform(
        &self,
        context: &AstTransformationContext,
        node: Rc<dyn IXamlAstNode>,
    ) -> XamlResult<Rc<dyn IXamlAstNode>> {
        if let Some(on) = node.cast::<XamlAstObjectNode>() {
            let mut c = 0;
            loop {
                let Some(ch) = on.children.borrow().get(c).cloned() else {
                    break;
                };
                if let Some(d) = ch.cast::<XamlAstXmlDirective>() {
                    if d.namespace.borrow().as_deref() == Some(XamlNamespaces::XAML2006)
                        && *d.name.borrow() == DATA_TYPE_PROPERTY_NAME
                    {
                        let property_values: Vec<Rc<XamlAstXamlPropertyValueNode>> = on
                            .children
                            .borrow()
                            .iter()
                            .filter_map(|n| n.cast::<XamlAstXamlPropertyValueNode>())
                            .collect();
                        let mut any_data_type = false;
                        for p in property_values {
                            // `((XamlAstNamePropertyReference)p.Property)?.Name`
                            let property = p.property();
                            let Some(name_reference) =
                                property.cast::<XamlAstNamePropertyReference>()
                            else {
                                return Err(XamlError::invalid_cast(format!(
                                    "Unable to cast object of type '{}' to type 'XamlAstNamePropertyReference'.",
                                    property.type_name()
                                )));
                            };
                            if name_reference.name() == DATA_TYPE_PROPERTY_NAME {
                                any_data_type = true;
                                break;
                            }
                        }
                        if any_data_type {
                            // Break iteration if any DataType property was already set by user code.
                            break;
                        }

                        let template_data_type_attribute =
                            context.try_get_ferro_types()?.data_type_attribute.clone();

                        let Some(clr_type) = on
                            .type_
                            .borrow()
                            .cast::<XamlAstClrTypeReference>()
                            .map(|t| t.type_.clone())
                        else {
                            break;
                        };

                        // Technically it's possible to map "x:DataType" to a property with [DataType] attribute regardless of its name,
                        // but we go explicitly strict here and check the name as well.
                        let found = Self::get_all_properties(&clr_type).into_iter().find(
                            |(_, property)| {
                                property.name() == DATA_TYPE_PROPERTY_NAME
                                    && property
                                        .custom_attributes()
                                        .iter()
                                        .any(|a| a.type_().equals(&*template_data_type_attribute))
                            },
                        );

                        if let Some((declaring_type, data_type_property)) = found {
                            let values = d.values.borrow().clone();
                            let replacement = XamlAstXamlPropertyValueNode::with_values(
                                &*d,
                                XamlAstNamePropertyReference::new(
                                    &*d,
                                    XamlAstClrTypeReference::new(&*ch, declaring_type, false),
                                    &data_type_property.name(),
                                    on.type_.borrow().clone(),
                                ),
                                values,
                                true,
                            );
                            if let Some(slot) = on.children.borrow_mut().get_mut(c) {
                                *slot = replacement;
                            }
                        }
                    }
                }
                c += 1;
            }
        }

        Ok(node)
    }
}
