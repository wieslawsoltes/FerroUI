//! Port of `Transform/Transformers/ObsoleteWarningsTransformer.cs`.

use std::rc::Rc;

use crate::ast::{
    IXamlAstNode, XamlAstExtensions, XamlAstNodeExtensions, XamlAstObjectNode,
    XamlAstPropertyReferenceExtensions, XamlAstXamlPropertyValueNode, XamlStaticExtensionNode,
    XamlStaticMember,
};
use crate::diagnostics::{XamlDiagnosticSeverity, XamlXWellKnownDiagnosticCodes};
use crate::exceptions::XamlResult;
use crate::transform::{AstTransformationContext, IXamlAstTransformer, XamlDiagnosticCodeSource};
use crate::type_system::{IXamlCustomAttribute, IXamlType, XamlValue};

pub struct ObsoleteWarningsTransformer;

#[derive(Clone, Copy)]
enum DiagnosticType {
    Obsolete,
    Experimental,
}

impl IXamlAstTransformer for ObsoleteWarningsTransformer {
    fn transform(
        &self,
        context: &AstTransformationContext,
        node: Rc<dyn IXamlAstNode>,
    ) -> XamlResult<Rc<dyn IXamlAstNode>> {
        let well_known_types = context.configuration().well_known_types();
        let obsolete_attribute_type: Rc<dyn IXamlType> =
            well_known_types.obsolete_attribute.clone();
        let experimental_attribute_type = well_known_types.experimental_attribute.clone();

        let find_attr =
            |attributes: Vec<Rc<dyn IXamlCustomAttribute>>| -> Option<(Rc<dyn IXamlCustomAttribute>, DiagnosticType)> {
                for attribute in attributes {
                    let attribute_type = attribute.type_();
                    if attribute_type.equals(&*obsolete_attribute_type) {
                        return Some((attribute, DiagnosticType::Obsolete));
                    }
                    if experimental_attribute_type.as_ref().is_some_and(|t| attribute_type.equals(&**t)) {
                        return Some((attribute, DiagnosticType::Experimental));
                    }
                }
                None
            };

        let report_obsolete =
            |member: &str, attribute: &dyn IXamlCustomAttribute| -> XamlResult<()> {
                let mut title = format!("'{member}' is obsolete");
                let parameters = attribute.parameters();
                if let Some(description) = parameters.first().filter(|d| !d.is_null()) {
                    title.push_str(": ");
                    title.push_str(&description.to_string());
                }

                let is_error = parameters.get(1).and_then(|v| v.as_bool()).unwrap_or(false);

                let code = (context.configuration().diagnostics_handler.code_mappings)(
                    &XamlDiagnosticCodeSource::WellKnown(XamlXWellKnownDiagnosticCodes::Obsolete),
                );
                context.report_diagnostic_at(
                    &code,
                    if is_error {
                        XamlDiagnosticSeverity::Error
                    } else {
                        XamlDiagnosticSeverity::Warning
                    },
                    &title,
                    Some(&*node),
                    XamlDiagnosticSeverity::None,
                )
            };

        let report_experimental = |member: &str,
                                   attribute: &dyn IXamlCustomAttribute|
         -> XamlResult<()> {
            let parameters = attribute.parameters();
            let Some(XamlValue::String(diagnostic_id)) = parameters.first() else {
                return Ok(());
            };

            let properties = attribute.properties();
            let message = properties
                .get("Message")
                .and_then(|m| m.as_str())
                .unwrap_or("");

            let title = if message.is_empty() {
                format!("'{member}' is for evaluation purposes only and is subject to change or removal in future updates.")
            } else {
                format!("'{member}' is for evaluation purposes only and is subject to change or removal in future updates: '{message}'.")
            };

            let code = (context.configuration().diagnostics_handler.code_mappings)(
                &XamlDiagnosticCodeSource::Id(diagnostic_id),
            );
            context.report_diagnostic_at(
                &code,
                XamlDiagnosticSeverity::Warning,
                &title,
                Some(&*node),
                XamlDiagnosticSeverity::None,
            )
        };

        let report = |member: &str,
                      attribute: &dyn IXamlCustomAttribute,
                      diagnostic_type: DiagnosticType| {
            match diagnostic_type {
                DiagnosticType::Obsolete => report_obsolete(member, attribute),
                DiagnosticType::Experimental => report_experimental(member, attribute),
            }
        };

        if let Some(ctor_node) = node.cast::<XamlAstObjectNode>() {
            let type_ = ctor_node.type_.borrow().get_clr_type()?;
            if let Some((type_attr, type_diagnostic)) = find_attr(type_.custom_attributes()) {
                report(&type_.name(), &*type_attr, type_diagnostic)?;
            }
        } else if let Some(prop_node) = node.cast::<XamlAstXamlPropertyValueNode>() {
            let prop = prop_node.property().get_clr_property()?;
            if let Some((prop_attr, prop_diagnostic)) = find_attr(prop.custom_attributes()) {
                report(
                    &format!("{}.{}", prop.declaring_type().name(), prop.name()),
                    &*prop_attr,
                    prop_diagnostic,
                )?;
            }
        } else if let Some(static_ext) = node.cast::<XamlStaticExtensionNode>() {
            let member = static_ext.resolve_member(false)?;
            let result = match &member {
                Some(XamlStaticMember::Field(field)) => find_attr(field.custom_attributes()),
                Some(XamlStaticMember::Property(prop)) => match prop.getter() {
                    Some(getter) => {
                        let mut attributes = prop.custom_attributes();
                        attributes.extend(getter.custom_attributes());
                        find_attr(attributes)
                    }
                    None => None,
                },
                None => None,
            };

            if let (Some((static_attr, static_diagnostic)), Some(member)) = (result, member) {
                report(
                    &format!("{}.{}", member.declaring_type().name(), member.name()),
                    &*static_attr,
                    static_diagnostic,
                )?;
            }
        }

        Ok(node)
    }
}
