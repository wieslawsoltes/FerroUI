//! Port of `CompilerExtensions/Transformers/FerroXamlIlDataTemplateWarningsTransformer.cs`.

use std::rc::Rc;

use xamlx::ast::{
    IXamlAstNode, IXamlAstValueNode, XamlAstExtensions, XamlAstNodeExtensions, XamlAstObjectNode,
    XamlAstPropertyReferenceExtensions, XamlAstXamlPropertyValueNode,
};
use xamlx::diagnostics::{XamlDiagnostic, XamlDiagnosticSeverity};
use xamlx::exceptions::XamlResult;
use xamlx::transform::{AstTransformationContext, IXamlAstTransformer};
use xamlx::type_system::IXamlType;

use crate::compiler_extensions::transformers::FerroXamlIlWellKnownTypesExtensions;
use crate::compiler_extensions::FerroXamlDiagnosticCodes;

/// Warns about an item container (`ListBoxItem`, ...) used as the root of an item template of
/// the matching items control.
pub struct FerroXamlIlDataTemplateWarningsTransformer;

impl IXamlAstTransformer for FerroXamlIlDataTemplateWarningsTransformer {
    fn transform(
        &self,
        context: &AstTransformationContext,
        node: Rc<dyn IXamlAstNode>,
    ) -> XamlResult<Rc<dyn IXamlAstNode>> {
        let ferro_types = context.get_ferro_types();
        let content_control = ferro_types.content_control.clone();

        // This transformers only looks for ContentControl delivered objects inside of DataTemplate
        let Some(object_node) = node.cast::<XamlAstObjectNode>() else {
            return Ok(node);
        };
        let object_type = object_node.type_().get_clr_type()?;
        if !content_control.is_assignable_from(&*object_type) {
            return Ok(node);
        }
        let parent_nodes = context.parent_nodes();
        let Some(parent_node) = parent_nodes
            .first()
            .and_then(|p| p.cast::<XamlAstObjectNode>())
        else {
            return Ok(node);
        };
        if !ferro_types
            .i_data_template
            .is_assignable_from(&*parent_node.type_().get_clr_type()?)
        {
            return Ok(node);
        }

        // And only inside of ItemTemplate or DataTemplates property value.
        let Some(value_node) = parent_nodes
            .iter()
            .find_map(|p| p.cast::<XamlAstXamlPropertyValueNode>())
        else {
            return Ok(node);
        };
        let clr_property = value_node.property().get_clr_property()?;
        let clr_property_name = clr_property.name();
        if !((clr_property_name == "ItemTemplate"
            && clr_property
                .declaring_type()
                .equals(&*ferro_types.items_control))
            || (clr_property_name == "DataTemplates"
                && clr_property.declaring_type().equals(&*ferro_types.control)))
        {
            return Ok(node);
        }

        // And only inside of ItemsControl
        let Some(items_control_node) = parent_nodes
            .iter()
            .skip_while(|p| !p.same_node(&value_node))
            .find_map(|p| p.cast::<XamlAstObjectNode>())
        else {
            return Ok(node);
        };
        let items_control_type = items_control_node.type_().get_clr_type()?;
        if !ferro_types
            .items_control
            .is_assignable_from(&*items_control_type)
        {
            return Ok(node);
        }

        // The framework doesn't have any reliable way to determine container type from the API.
        let Some(known_item_container_type_name) =
            get_known_item_container_type_full_name(&*items_control_type)
        else {
            return Ok(node);
        };
        let Some(known_item_container_type) = items_control_type
            .assembly()
            .and_then(|a| a.find_type(known_item_container_type_name))
        else {
            return Ok(node);
        };

        if known_item_container_type.is_assignable_from(&*object_type) {
            let items_control_name = items_control_type.name();
            context.report_diagnostic(
                XamlDiagnostic::with_line_info(
                    FerroXamlDiagnosticCodes::ITEM_CONTAINER_INSIDE_TEMPLATE,
                    XamlDiagnosticSeverity::Warning,
                    format!(
                        "Unexpected '{}' inside of '{items_control_name}.{clr_property_name}'. \
                         '{items_control_name}.{clr_property_name}' defines template of the container content, not the container itself.",
                        known_item_container_type.name()
                    ),
                    Some(&*node),
                ),
                true,
            )?;
        }

        Ok(node)
    }
}

fn get_known_item_container_type_full_name(items_control_type: &dyn IXamlType) -> Option<&'static str> {
    match items_control_type.full_name().as_str() {
        "FerroUI.Controls.ListBox" => Some("FerroUI.Controls.ListBoxItem"),
        "FerroUI.Controls.ComboBox" => Some("FerroUI.Controls.ComboBoxItem"),
        "FerroUI.Controls.Menu" => Some("FerroUI.Controls.MenuItem"),
        "FerroUI.Controls.MenuItem" => Some("FerroUI.Controls.MenuItem"),
        "FerroUI.Controls.Primitives.TabStrip" => Some("FerroUI.Controls.Primitives.TabStripItem"),
        "FerroUI.Controls.TabControl" => Some("FerroUI.Controls.TabItem"),
        "FerroUI.Controls.TreeView" => Some("FerroUI.Controls.TreeViewItem"),
        _ => None,
    }
}
