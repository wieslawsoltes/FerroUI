//! Port of `CompilerExtensions/Transformers/FerroXamlIlDuplicateSettersChecker.cs`.

use std::collections::HashSet;
use std::rc::Rc;

use xamlx::ast::{
    IXamlAstNode, IXamlAstValueNode, XamlAstExtensions, XamlAstNodeExtensions, XamlAstObjectNode,
    XamlAstPropertyReferenceExtensions, XamlAstTextNode, XamlAstXamlPropertyValueNode,
};
use xamlx::diagnostics::{XamlDiagnostic, XamlDiagnosticSeverity};
use xamlx::exceptions::XamlResult;
use xamlx::transform::{AstTransformationContext, IXamlAstTransformer};

use super::ferro_xaml_il_selector_transformer::value_at;
use crate::compiler_extensions::transformers::FerroXamlIlWellKnownTypesExtensions;
use crate::compiler_extensions::FerroXamlDiagnosticCodes;

/// Warns when a style or a control theme sets the same property with more than one setter.
pub struct FerroXamlIlDuplicateSettersChecker;

impl IXamlAstTransformer for FerroXamlIlDuplicateSettersChecker {
    fn transform(
        &self,
        context: &AstTransformationContext,
        node: Rc<dyn IXamlAstNode>,
    ) -> XamlResult<Rc<dyn IXamlAstNode>> {
        let Some(object_node) = node.cast::<XamlAstObjectNode>() else {
            return Ok(node);
        };

        let node_type = object_node.type_().get_clr_type()?;
        let types = context.get_ferro_types();
        if !types.style.is_assignable_from(&*node_type)
            && !types.control_theme.is_assignable_from(&*node_type)
        {
            return Ok(node);
        }

        let mut properties: Vec<String> = Vec::new();
        let children = object_node.children.borrow().clone();
        for child in children {
            let Some(setter) = child.cast::<XamlAstObjectNode>() else {
                continue;
            };
            if setter.type_().get_clr_type()?.name() != "Setter" {
                continue;
            }
            let setter_children = setter.children.borrow().clone();
            for c in setter_children {
                let Some(p) = c.cast::<XamlAstXamlPropertyValueNode>() else {
                    continue;
                };
                if p.property().get_clr_property()?.name() != "Property" {
                    continue;
                }
                let value = value_at(&p.values.borrow(), 0)?;
                if let Some(text) = value.cast::<XamlAstTextNode>() {
                    properties.push(text.text());
                }
            }
        }

        let mut index: HashSet<String> = HashSet::new();
        for property in properties {
            if !index.insert(property.clone()) {
                context.report_diagnostic(
                    XamlDiagnostic::with_line_info(
                        FerroXamlDiagnosticCodes::DUPLICATE_SETTER_ERROR,
                        XamlDiagnosticSeverity::Warning,
                        format!("Duplicate setter encountered for property '{property}'"),
                        Some(&*node),
                    ),
                    true,
                )?;
            }
        }

        Ok(node)
    }
}
