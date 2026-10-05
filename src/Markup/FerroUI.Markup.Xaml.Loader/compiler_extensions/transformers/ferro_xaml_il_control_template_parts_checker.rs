//! Port of `CompilerExtensions/Transformers/FerroXamlIlControlTemplatePartsChecker.cs`.

use std::rc::Rc;

use xamlx::ast::{IXamlAstNode, XamlAstExtensions, XamlAstNodeExtensions};
use xamlx::diagnostics::{XamlDiagnostic, XamlDiagnosticSeverity};
use xamlx::exceptions::XamlResult;
use xamlx::transform::{AstTransformationContext, IXamlAstTransformer};
use xamlx::type_system::{IXamlType, XamlValue};

use crate::compiler_extensions::transformers::{
    FerroXamlIlTargetTypeMetadataNode, FerroXamlIlWellKnownTypesExtensions, ScopeTypes,
};
use crate::compiler_extensions::visitors::NameScopeRegistrationVisitor;
use crate::compiler_extensions::FerroXamlDiagnosticCodes;

/// Checks the names registered inside a `ControlTemplate` against the `TemplatePart`
/// attributes of the templated control type.
pub struct FerroXamlIlControlTemplatePartsChecker;

/// A resolved `TemplatePart` attribute: `(type, isRequired)` upstream.
struct TemplatePart {
    name: String,
    type_: Option<Rc<dyn IXamlType>>,
    is_required: bool,
}

impl IXamlAstTransformer for FerroXamlIlControlTemplatePartsChecker {
    fn transform(
        &self,
        context: &AstTransformationContext,
        node: Rc<dyn IXamlAstNode>,
    ) -> XamlResult<Rc<dyn IXamlAstNode>> {
        let Some(on) = node.cast::<FerroXamlIlTargetTypeMetadataNode>() else {
            return Ok(node);
        };
        if on.scope_type != ScopeTypes::ControlTemplate {
            return Ok(node);
        }
        // Styles with template selector will also return ScopeTypes.ControlTemplate, so we need to double check.
        if !on
            .value()
            .type_()
            .get_clr_type()?
            .equals(&*context.get_ferro_types().control_template)
        {
            return Ok(node);
        }

        let target_type = on.target_type().get_clr_type()?;
        let template_parts = resolve_template_parts(&target_type);

        if template_parts.is_empty() {
            return Ok(node);
        }

        let mut visitor = NameScopeRegistrationVisitor::default();
        node.visit_children(&mut visitor)?;

        for part in &template_parts {
            let name = &part.name;

            let Some(res) = visitor.get(name) else {
                if part.is_required {
                    context.report_diagnostic(
                        XamlDiagnostic::with_line_info(
                            FerroXamlDiagnosticCodes::REQUIRED_TEMPLATE_PART_MISSING,
                            XamlDiagnosticSeverity::Error,
                            format!(
                                "Required template part with x:Name '{name}' must be defined on '{}' ControlTemplate.",
                                target_type.name()
                            ),
                            Some(&*node),
                        ),
                        true,
                    )?;
                } else {
                    context.report_diagnostic(
                        XamlDiagnostic::with_line_info(
                            FerroXamlDiagnosticCodes::OPTIONAL_TEMPLATE_PART_MISSING,
                            XamlDiagnosticSeverity::None,
                            format!(
                                "Optional template part with x:Name '{name}' can be defined on '{}' ControlTemplate.",
                                target_type.name()
                            ),
                            Some(&*node),
                        ),
                        true,
                    )?;
                }

                continue;
            };

            if let Some(expected_type) = &part.type_ {
                if !expected_type.is_assignable_from(&*res.0) {
                    context.report_diagnostic(
                        XamlDiagnostic::with_line_info(
                            FerroXamlDiagnosticCodes::TEMPLATE_PART_WRONG_TYPE,
                            XamlDiagnosticSeverity::Error,
                            format!(
                                "Template part '{name}' is expected to be assignable to '{}', but actual type is {}.",
                                expected_type.name(),
                                res.0.name()
                            ),
                            Some(&*res.1),
                        ),
                        true,
                    )?;
                }
            }
        }

        Ok(node)
    }
}

/// The template parts of `target_type`, in the order the attributes are found.
fn resolve_template_parts(target_type: &Rc<dyn IXamlType>) -> Vec<TemplatePart> {
    let mut dictionary: Vec<TemplatePart> = Vec::new();
    // Custom Attributes go in order from current type to base type. It should be possible to override parent template parts.
    for attr in target_type.get_all_custom_attributes() {
        if attr.type_().name() == "TemplatePartAttribute" {
            let properties = attr.properties();
            let parameters = attr.parameters();

            let name_obj = properties
                .get("Name")
                .cloned()
                .or_else(|| parameters.first().cloned());

            let type_obj = properties
                .get("Type")
                .cloned()
                .or_else(|| parameters.get(1).cloned());

            let is_required_obj = properties.get("IsRequired").cloned();

            if let Some(XamlValue::String(name)) = name_obj {
                if !name.is_empty() && !dictionary.iter().any(|p| p.name == name) {
                    let type_ = match type_obj {
                        Some(XamlValue::Type(type_)) => Some(type_),
                        _ => None,
                    };
                    let is_required = matches!(is_required_obj, Some(XamlValue::Boolean(true)));
                    dictionary.push(TemplatePart {
                        name,
                        type_,
                        is_required,
                    });
                }
            }
        }
    }

    dictionary
}
