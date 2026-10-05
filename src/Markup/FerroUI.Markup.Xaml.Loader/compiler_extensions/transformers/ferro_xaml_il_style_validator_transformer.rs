//! Port of `CompilerExtensions/Transformers/FerroXamlIlStyleValidatorTransformer.cs`.

use std::rc::Rc;

use xamlx::ast::{
    IXamlAstNode, IXamlAstValueNode, XamlAstExtensions, XamlAstNodeExtensions, XamlAstObjectNode,
    XamlAstPropertyReferenceExtensions, XamlAstXamlPropertyValueNode,
};
use xamlx::diagnostics::{XamlDiagnostic, XamlDiagnosticSeverity};
use xamlx::exceptions::XamlResult;
use xamlx::transform::{AstTransformationContext, IXamlAstTransformer};

use crate::compiler_extensions::transformers::FerroXamlIlWellKnownTypesExtensions;
use crate::compiler_extensions::FerroXamlDiagnosticCodes;

/// Warns about a style placed directly in `ResourceDictionary.MergedDictionaries`, where its
/// nested styles are ignored.
pub struct FerroXamlIlStyleValidatorTransformer;

impl IXamlAstTransformer for FerroXamlIlStyleValidatorTransformer {
    fn transform(
        &self,
        context: &AstTransformationContext,
        node: Rc<dyn IXamlAstNode>,
    ) -> XamlResult<Rc<dyn IXamlAstNode>> {
        let Some(on) = node.cast::<XamlAstObjectNode>() else {
            return Ok(node);
        };
        let types = context.get_ferro_types();
        let node_type = on.type_().get_clr_type()?;
        if !types.i_style.is_assignable_from(&*node_type) {
            return Ok(node);
        }

        if let Some(property_value_node) = context
            .first_parent_node()
            .and_then(|p| p.cast::<XamlAstXamlPropertyValueNode>())
        {
            let clr_property = property_value_node.property().get_clr_property()?;
            if clr_property.name() == "MergedDictionaries"
                && clr_property
                    .declaring_type()
                    .equals(&*types.resource_dictionary)
            {
                let node_name = node_type.name();
                context.report_diagnostic(
                    XamlDiagnostic::with_line_info(
                        FerroXamlDiagnosticCodes::STYLE_IN_MERGED_DICTIONARIES,
                        XamlDiagnosticSeverity::Warning,
                        // Keep it single line, as MSBuild splits multiline warnings into two warnings.
                        format!(
                            "Including {node_name} as part of MergedDictionaries will ignore any nested styles.\
                             Instead, you can add {node_name} to the Styles collection on the same control or application."
                        ),
                        Some(&*node),
                    ),
                    true,
                )?;
            }
        }

        Ok(node)
    }
}
