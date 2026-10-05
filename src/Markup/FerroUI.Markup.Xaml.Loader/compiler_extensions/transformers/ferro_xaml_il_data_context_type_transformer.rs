//! Port of `CompilerExtensions/Transformers/FerroXamlIlDataContextTypeTransformer.cs`.

use std::rc::Rc;

use xamlx::ast::{
    IXamlAstNode, IXamlAstTypeReference, IXamlAstValueNode, IXamlAstVisitor, IXamlLineInfo,
    XamlAstConstructableObjectNode, XamlAstExtensions, XamlAstNodeExtensions, XamlAstTextNode,
    XamlAstXmlDirective, XamlMarkupExtensionNode, XamlPropertyAssignmentNode,
    XamlTypeExtensionNode, XamlValueWithSideEffectNodeBase,
};
use xamlx::exceptions::{XamlError, XamlResult};
use xamlx::transform::transformers::TypeReferenceResolver;
use xamlx::transform::{AstTransformationContext, IXamlAstTransformer};
use xamlx::type_system::{IXamlType, XamlPseudoType, XamlValue};
use xamlx::{xaml_line_info_impl, XamlNamespaces};

use super::FerroXamlIlWellKnownTypesExtensions;
use crate::compiler_extensions::XamlIlBindingPathHelper;

/// `XamlDataContextException : XamlTransformException`.
///
/// Errors are values of the single [`XamlError`] type; the derived class is represented by a
/// `XamlError::Transform` tagged with the class name, which is what
/// `FerroXamlDiagnosticCodes::xaml_x_diagnostic_code_to_ferro` (code `FRN2101`) and
/// [`Self::is`] look at.
pub struct XamlDataContextException;

impl XamlDataContextException {
    pub const TYPE_NAME: &'static str = "XamlDataContextException";

    /// `new XamlDataContextException(message, lineInfo, innerException)`.
    pub fn new(
        message: impl Into<String>,
        line_info: &dyn IXamlLineInfo,
        inner_exception: Option<XamlError>,
    ) -> XamlError {
        let error = match inner_exception {
            Some(inner) => {
                XamlError::transform_exception_with_inner(message, Some(line_info), inner)
            }
            None => XamlError::transform_exception(message, Some(line_info)),
        };
        error.with_derived_type_name(Self::TYPE_NAME)
    }

    /// `e is XamlDataContextException`.
    pub fn is(error: &XamlError) -> bool {
        error.is_derived_type(Self::TYPE_NAME)
    }
}

/// Determines the data context type of every constructed object that sets one (through
/// `x:DataType`, a `DataContext` assignment, a property marked with `DataTypeAttribute` or the
/// items of the control that presents it) and wraps the object in a
/// [`FerroXamlIlDataContextTypeMetadataNode`].
pub struct FerroXamlIlDataContextTypeTransformer;

impl IXamlAstTransformer for FerroXamlIlDataContextTypeTransformer {
    fn transform(
        &self,
        context: &AstTransformationContext,
        node: Rc<dyn IXamlAstNode>,
    ) -> XamlResult<Rc<dyn IXamlAstNode>> {
        if context
            .first_parent_node()
            .is_some_and(|p| p.is::<FerroXamlIlDataContextTypeMetadataNode>())
        {
            // We've already resolved the data context type for this node.
            return Ok(node);
        }

        let Some(on) = node.cast::<XamlAstConstructableObjectNode>() else {
            return Ok(node);
        };

        let types = context.try_get_ferro_types()?;
        let mut inferred_data_context_type_node: Option<Rc<FerroXamlIlDataContextTypeMetadataNode>> =
            None;
        let mut directive_data_context_type_node: Option<Rc<FerroXamlIlDataContextTypeMetadataNode>> =
            None;

        let mut i = 0;
        loop {
            let Some(child) = on.children.borrow().get(i).cloned() else {
                break;
            };
            if let Some(directive) = child.cast::<XamlAstXmlDirective>() {
                let values = directive.values.borrow().clone();
                if directive.namespace.borrow().as_deref() == Some(XamlNamespaces::XAML2006)
                    && *directive.name.borrow() == "DataType"
                    && values.len() == 1
                {
                    on.children.borrow_mut().remove(i);
                    if let Some(type_node) = values[0].cast::<XamlTypeExtensionNode>() {
                        directive_data_context_type_node =
                            Some(FerroXamlIlDataContextTypeMetadataNode::new(
                                on.clone(),
                                type_node.value().get_clr_type()?,
                            ));
                    } else if let Some(text) = values[0].cast::<XamlAstTextNode>() {
                        directive_data_context_type_node =
                            Some(FerroXamlIlDataContextTypeMetadataNode::new(
                                on.clone(),
                                TypeReferenceResolver::resolve_type_by_xml_name(
                                    context,
                                    &text.text(),
                                    false,
                                    &*text,
                                    true,
                                )?
                                .type_
                                .clone(),
                            ));
                    } else {
                        return Err(XamlDataContextException::new(
                            "x:DataType should be set to a type name.",
                            &*values[0],
                            None,
                        ));
                    }
                    // `i--` followed by the loop's `++i`.
                    continue;
                }
            } else if let Some(pa) = child.cast::<XamlPropertyAssignmentNode>() {
                let template_data_type_attribute = &types.data_type_attribute;
                let first_value = pa.values.borrow().first().cloned();

                let data_context_object = if pa.property.name() == "DataContext"
                    && pa.property.declaring_type().equals(&*types.styled_element)
                {
                    first_value
                        .as_ref()
                        .and_then(|v| v.cast::<XamlMarkupExtensionNode>())
                        .and_then(|ext| ext.value().cast::<XamlAstConstructableObjectNode>())
                } else {
                    None
                };

                if let Some(obj) = data_context_object {
                    inferred_data_context_type_node =
                        Some(Self::parse_data_context(context, &on, &obj)?);
                } else if pa
                    .property
                    .custom_attributes()
                    .iter()
                    .any(|a| a.type_().equals(&**template_data_type_attribute))
                {
                    if let Some(data_type_node) = first_value
                        .as_ref()
                        .and_then(|v| v.cast::<XamlTypeExtensionNode>())
                    {
                        inferred_data_context_type_node =
                            Some(FerroXamlIlDataContextTypeMetadataNode::new(
                                on.clone(),
                                data_type_node.value().get_clr_type()?,
                            ));
                    }
                }
            }
            i += 1;
        }

        // If there is no x:DataType directive,
        // do more specialized inference
        if directive_data_context_type_node.is_none() && inferred_data_context_type_node.is_none()
        {
            // Infer data type from collection binding on a control that displays items.
            let parent_nodes = context.parent_nodes();
            let property = parent_nodes
                .iter()
                .find_map(|n| n.cast::<XamlPropertyAssignmentNode>());
            let attribute_type = &types.inherit_data_type_from_items_attribute;
            let attribute = property.and_then(|p| {
                p.property
                    .custom_attributes()
                    .into_iter()
                    .find(|a| a.type_().equals(&**attribute_type))
            });

            if let Some(attribute) = attribute {
                // `(string?)attribute.Parameters.First()`
                let property_name = match attribute.parameters().into_iter().next() {
                    Some(XamlValue::String(name)) => Some(name),
                    Some(XamlValue::Null) => None,
                    Some(other) => {
                        return Err(XamlError::invalid_cast(format!(
                            "Unable to cast object of type '{}' to type 'System.String'.",
                            other.type_name()
                        )))
                    }
                    None => {
                        return Err(XamlError::invalid_operation(
                            "Sequence contains no elements",
                        ))
                    }
                };

                let constructable_parents = parent_nodes
                    .iter()
                    .filter_map(|n| n.cast::<XamlAstConstructableObjectNode>());
                let parent_object = match attribute.properties().get("AncestorType") {
                    Some(XamlValue::Type(xaml_type)) => {
                        let mut found = None;
                        for n in constructable_parents {
                            if xaml_type.is_assignable_from(&*n.type_().get_clr_type()?) {
                                found = Some(n);
                                break;
                            }
                        }
                        found
                    }
                    _ => {
                        let mut constructable_parents = constructable_parents;
                        constructable_parents.next()
                    }
                };

                if let (Some(parent_object), Some(property_name)) = (parent_object, property_name)
                {
                    if !property_name.is_empty() {
                        inferred_data_context_type_node = Self::infer_data_context_of_presented_item(
                            context,
                            &on,
                            &parent_object,
                            &property_name,
                        )?;
                    }
                }
            }

            if inferred_data_context_type_node.is_none()
                // Only for IDataTemplate, as we want to notify user as early as possible,
                // and IDataTemplate cannot inherit DataType from the parent implicitly.
                && types
                    .i_data_template
                    .is_assignable_from(&*on.type_().get_clr_type()?)
            {
                // We can't infer the collection type and the currently calculated type is definitely wrong.
                // Notify the user that we were unable to infer the data context type if they use a compiled binding.
                inferred_data_context_type_node =
                    Some(FerroXamlIlUninferrableDataContextMetadataNode::new(on.clone()));
            }
        }

        Ok(
            match directive_data_context_type_node.or(inferred_data_context_type_node) {
                Some(metadata_node) => metadata_node,
                None => node,
            },
        )
    }
}

impl FerroXamlIlDataContextTypeTransformer {
    fn infer_data_context_of_presented_item(
        context: &AstTransformationContext,
        on: &Rc<XamlAstConstructableObjectNode>,
        parent_object: &Rc<XamlAstConstructableObjectNode>,
        property_name: &str,
    ) -> XamlResult<Option<Rc<FerroXamlIlDataContextTypeMetadataNode>>> {
        let types = context.try_get_ferro_types()?;
        let parent_items_assignment = parent_object
            .children
            .borrow()
            .iter()
            .filter_map(|c| c.cast::<XamlPropertyAssignmentNode>())
            .find(|pa| pa.property.name() == property_name);
        let Some(parent_items_assignment) = parent_items_assignment else {
            return Ok(None);
        };
        // `?.Values[0]`: an assignment always has a value; an empty list yields "no value" here
        // instead of the upstream `ArgumentOutOfRangeException`.
        let Some(parent_items_value) = parent_items_assignment.values.borrow().first().cloned()
        else {
            return Ok(None);
        };

        let mut items_collection_type: Option<Rc<dyn IXamlType>> = None;
        let parent_items_value_type = parent_items_value.type_().get_clr_type()?;
        if types
            .binding_base
            .is_assignable_from(&*parent_items_value_type)
        {
            if parent_items_value_type.equals(&*types.compiled_binding) {
                if let Some(parent_items_binding) = parent_items_value
                    .cast::<XamlMarkupExtensionNode>()
                    .and_then(|ext| ext.value().cast::<XamlAstConstructableObjectNode>())
                {
                    let parent_items_data_context = context
                        .parent_nodes()
                        .into_iter()
                        .skip_while(|n| !n.same_node(parent_object))
                        .find_map(|n| n.cast::<FerroXamlIlDataContextTypeMetadataNode>());
                    if let Some(parent_items_data_context) = parent_items_data_context {
                        let start_type = parent_items_data_context.data_context_type.clone();
                        items_collection_type =
                            Some(XamlIlBindingPathHelper::update_compiled_binding_extension(
                                context,
                                &parent_items_binding,
                                &|| Ok(start_type.clone()),
                                &parent_object.type_().get_clr_type()?,
                            )?);
                    }
                }
            }
        } else {
            items_collection_type = Some(parent_items_value_type);
        }

        if let Some(items_collection_type) = items_collection_type {
            let i_enumerable_of_t = context.configuration().well_known_types().i_enumerable_of_t.clone();
            for i in Self::get_all_interfaces_including_self(&items_collection_type) {
                if i.generic_type_definition()
                    .is_some_and(|d| d.equals(&*i_enumerable_of_t))
                {
                    if let Some(item_type) = i.generic_arguments().into_iter().next() {
                        return Ok(Some(FerroXamlIlDataContextTypeMetadataNode::new(
                            on.clone(),
                            item_type,
                        )));
                    }
                }
            }
        }

        Ok(None)
    }

    fn parse_data_context(
        context: &AstTransformationContext,
        on: &Rc<XamlAstConstructableObjectNode>,
        obj: &Rc<XamlAstConstructableObjectNode>,
    ) -> XamlResult<Rc<FerroXamlIlDataContextTypeMetadataNode>> {
        let types = context.try_get_ferro_types()?;
        let binding_type = &types.binding_base;
        let obj_type = obj.type_().get_clr_type()?;
        if !binding_type.is_assignable_from(&*obj_type)
            && !(obj_type.equals(&*types.reflection_binding_extension)
                || obj_type.equals(&*types.compiled_binding_extension))
        {
            return Ok(FerroXamlIlDataContextTypeMetadataNode::new(
                on.clone(),
                obj_type,
            ));
        } else if obj_type.equals(&*types.compiled_binding_extension) {
            let start_type_resolver = || -> XamlResult<Rc<dyn IXamlType>> {
                let data_type_property = obj
                    .children
                    .borrow()
                    .iter()
                    .filter_map(|c| c.cast::<XamlPropertyAssignmentNode>())
                    .find(|c| c.property.name() == "DataType");
                if let Some(data_type_property) = data_type_property {
                    let values = data_type_property.values.borrow().clone();
                    if values.len() == 1 {
                        if let Some(text) = values[0].cast::<XamlAstTextNode>() {
                            return Ok(TypeReferenceResolver::resolve_type_by_xml_name(
                                context,
                                &text.text(),
                                false,
                                &*text,
                                true,
                            )?
                            .type_
                            .clone());
                        }
                    }
                }

                let parent_data_context_node = context
                    .parent_nodes()
                    .into_iter()
                    .find_map(|n| n.cast::<FerroXamlIlDataContextTypeMetadataNode>());
                match parent_data_context_node {
                    None => Err(XamlDataContextException::new(
                        "Cannot parse a compiled binding without an explicit x:DataType directive to give a starting data type for bindings.",
                        &**obj,
                        None,
                    )),
                    Some(parent_data_context_node) => {
                        Ok(parent_data_context_node.data_context_type.clone())
                    }
                }
            };

            let binding_result_type = XamlIlBindingPathHelper::update_compiled_binding_extension(
                context,
                obj,
                &start_type_resolver,
                &on.type_().get_clr_type()?,
            )?;
            return Ok(FerroXamlIlDataContextTypeMetadataNode::new(
                on.clone(),
                binding_result_type,
            ));
        }

        Ok(FerroXamlIlUninferrableDataContextMetadataNode::new(
            on.clone(),
        ))
    }

    fn get_all_interfaces_including_self(type_: &Rc<dyn IXamlType>) -> Vec<Rc<dyn IXamlType>> {
        let mut rv = Vec::new();
        if type_.is_interface() {
            rv.push(type_.clone());
        }

        rv.extend(type_.get_all_interfaces());
        rv
    }
}

/// Scope marker: the data context of [`FerroXamlIlDataContextTypeMetadataNode::value`] and of
/// everything below it (until the next marker) has the type
/// [`FerroXamlIlDataContextTypeMetadataNode::data_context_type`]. The node is removed again by
/// `FerroXamlIlCompiledBindingsMetadataRemover`; it never reaches a back end.
///
/// The upstream subclass `FerroXamlIlUninferrableDataContextMetadataNode` (data type
/// `XamlPseudoType.Unknown`) is this same node type created through
/// [`FerroXamlIlUninferrableDataContextMetadataNode::new`], so that
/// `node is FerroXamlIlDataContextTypeMetadataNode` (`node.cast::<..>()`) holds for both, as
/// upstream; [`FerroXamlIlDataContextTypeMetadataNode::is_uninferrable`] tells them apart.
pub struct FerroXamlIlDataContextTypeMetadataNode {
    pub base: XamlValueWithSideEffectNodeBase,
    pub data_context_type: Rc<dyn IXamlType>,
    uninferrable: bool,
}

impl FerroXamlIlDataContextTypeMetadataNode {
    pub fn new(value: Rc<dyn IXamlAstValueNode>, target_type: Rc<dyn IXamlType>) -> Rc<Self> {
        Rc::new(Self {
            base: XamlValueWithSideEffectNodeBase::new(&*value.clone(), value),
            data_context_type: target_type,
            uninferrable: false,
        })
    }

    pub fn value(&self) -> Rc<dyn IXamlAstValueNode> {
        self.base.value()
    }

    /// `this is FerroXamlIlUninferrableDataContextMetadataNode`.
    pub fn is_uninferrable(&self) -> bool {
        self.uninferrable
    }
}

xaml_line_info_impl!(FerroXamlIlDataContextTypeMetadataNode, base);

impl IXamlAstNode for FerroXamlIlDataContextTypeMetadataNode {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn into_any_rc(self: Rc<Self>) -> Rc<dyn std::any::Any> {
        self
    }
    fn type_name(&self) -> &'static str {
        if self.uninferrable {
            "FerroXamlIlUninferrableDataContextMetadataNode"
        } else {
            "FerroXamlIlDataContextTypeMetadataNode"
        }
    }
    fn into_value_node(self: Rc<Self>) -> Option<Rc<dyn IXamlAstValueNode>> {
        Some(self)
    }

    /// `[DebuggerDisplay("DataType = {DataContextType}")]` / `"DataType = Unknown"`.
    fn to_node_string(&self) -> String {
        if self.uninferrable {
            "DataType = Unknown".to_string()
        } else {
            format!("DataType = {}", self.data_context_type.to_type_string())
        }
    }

    fn visit_children(&self, visitor: &mut dyn IXamlAstVisitor) -> XamlResult<()> {
        self.base.visit_children(visitor)
    }

    fn as_value_with_side_effect_node_base(&self) -> Option<&XamlValueWithSideEffectNodeBase> {
        Some(&self.base)
    }
}

impl IXamlAstValueNode for FerroXamlIlDataContextTypeMetadataNode {
    fn type_(&self) -> Rc<dyn IXamlAstTypeReference> {
        self.base.type_()
    }
}

/// `FerroXamlIlUninferrableDataContextMetadataNode : FerroXamlIlDataContextTypeMetadataNode`:
/// the data context type of the value could not be inferred (a data template without a data
/// type); its data type is `XamlPseudoType.Unknown`, which makes compiled bindings below it
/// fail to resolve their first member.
pub struct FerroXamlIlUninferrableDataContextMetadataNode;

impl FerroXamlIlUninferrableDataContextMetadataNode {
    #[allow(clippy::new_ret_no_self)]
    pub fn new(value: Rc<dyn IXamlAstValueNode>) -> Rc<FerroXamlIlDataContextTypeMetadataNode> {
        Rc::new(FerroXamlIlDataContextTypeMetadataNode {
            base: XamlValueWithSideEffectNodeBase::new(&*value.clone(), value),
            data_context_type: XamlPseudoType::unknown(),
            uninferrable: true,
        })
    }

    /// `node is FerroXamlIlUninferrableDataContextMetadataNode`.
    pub fn is(node: &Rc<dyn IXamlAstNode>) -> bool {
        node.cast::<FerroXamlIlDataContextTypeMetadataNode>()
            .is_some_and(|n| n.is_uninferrable())
    }
}
