//! Port of `Transform/XamlTransformHelpers.cs`.

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use crate::ast::{
    IXamlAstValueNode, IXamlLineInfo, XamlAstClrProperty, XamlAstClrTypeReference,
    XamlAstContextLocalNode, XamlAstExtensions, XamlAstNeedsParentStackValueNode,
    XamlAstNewClrObjectNode, XamlAstNodeExtensions, XamlAstRuntimeCastNode, XamlAstTextNode,
    XamlLoadMethodDelegateNode, XamlMarkupExtensionNode, XamlStaticOrTargetedReturnMethodCallNode,
    XamlTypeExtensionNode,
};
use crate::exceptions::{XamlError, XamlResult};
use crate::type_system::{
    IXamlCustomAttribute, IXamlMethod, IXamlParameterInfo, IXamlProperty, IXamlType,
    TypeSystemHelpers, XamlTypeKey, XamlValue,
};

use super::transformers::TypeReferenceResolver;
use super::{AstTransformationContext, TransformerConfiguration};

#[derive(Default)]
struct AdderCache(RefCell<HashMap<XamlTypeKey, Vec<Rc<dyn IXamlMethod>>>>);

#[derive(Default)]
struct MarkupExtensionProvideValueCache {
    type_to_provide_value: RefCell<HashMap<XamlTypeKey, Option<Rc<dyn IXamlMethod>>>>,
}

pub struct XamlTransformHelpers;

impl XamlTransformHelpers {
    pub fn find_possible_adders(
        context: &AstTransformationContext,
        type_: &Rc<dyn IXamlType>,
    ) -> XamlResult<Vec<Rc<dyn IXamlMethod>>> {
        let find_possible_adders_impl = || -> XamlResult<Vec<Rc<dyn IXamlMethod>>> {
            let configuration = context.configuration();
            let known = configuration.well_known_types();

            // Attempt to cast IEnumerable and IEnumerable<T> to IList<T>
            let mut actual_type = type_.clone();
            if actual_type.equals(&*known.i_enumerable) {
                actual_type = known.i_list.clone();
            }
            if actual_type
                .generic_type_definition()
                .is_some_and(|d| d.equals(&*known.i_enumerable_of_t))
            {
                let argument = first_generic_argument(&actual_type)?;
                actual_type = known.i_list_of_t.make_generic_type(&[argument])?;
            }

            let mut inspect_types = vec![actual_type.clone()];
            inspect_types.extend(actual_type.get_all_interfaces());

            // If type supports IList<T> don't fall back to IList
            if inspect_types.iter().any(|t| {
                t.generic_type_definition()
                    .is_some_and(|d| d.equals(&*known.i_list_of_t))
            }) {
                inspect_types.retain(|t| !t.equals(&*known.i_list));
            }

            let mut rv: Vec<Rc<dyn IXamlMethod>> = Vec::new();
            for t in &inspect_types {
                for m in t.find_methods(|m| {
                    m.name() == "Add"
                        && m.is_public()
                        && !m.is_static()
                        && (m.parameters().len() == 1 || m.parameters().len() == 2)
                }) {
                    if rv.iter().any(|em| em.equals(&*m)) {
                        continue;
                    }
                    rv.push(m);
                }
            }

            // First use methods from the type itself, then from base types, then from interfaces
            let mut keyed = Vec::with_capacity(rv.len());
            for m in rv {
                let this = m.this_or_first_parameter()?;
                keyed.push((!this.equals(&*actual_type), this.is_interface(), m));
            }
            keyed.sort_by_key(|(not_actual, is_interface, _)| (*not_actual, *is_interface));
            let mut rv: Vec<Rc<dyn IXamlMethod>> = keyed.into_iter().map(|(_, _, m)| m).collect();

            if let Some(i_add_child_of_t) = &configuration.type_mappings.i_add_child_of_t {
                for t in inspect_types.iter().filter(|t| {
                    t.generic_type_definition()
                        .is_some_and(|d| d.equals(&**i_add_child_of_t))
                }) {
                    rv.push(t.get_method(|x| x.name() == "AddChild")?);
                }
            }

            if let Some(i_add_child) = &configuration.type_mappings.i_add_child {
                for t in inspect_types.iter().filter(|t| t.equals(&**i_add_child)) {
                    rv.push(t.get_method(|x| x.name() == "AddChild")?);
                }
            }

            Ok(rv)
        };

        let cache = context.get_or_create_item::<AdderCache>();
        let key = XamlTypeKey(type_.clone());
        if let Some(rvr) = cache.0.borrow().get(&key) {
            return Ok(rvr.clone());
        }
        let rv = find_possible_adders_impl()?;
        cache.0.borrow_mut().insert(key, rv.clone());
        Ok(rv)
    }

    pub fn get_markup_extension_provide_value_alternatives(
        context: &AstTransformationContext,
        type_: &dyn IXamlType,
    ) -> XamlResult<Vec<Rc<dyn IXamlMethod>>> {
        let sp = context.configuration().type_mappings.service_provider()?;
        Ok(type_.find_methods(|m| {
            (m.name() == "ProvideValue" || m.name() == "ProvideTypedValue")
                && m.is_public()
                && !m.is_static()
                && {
                    let parameters = m.parameters();
                    parameters.is_empty() || (parameters.len() == 1 && parameters[0].equals(&*sp))
                }
        }))
    }

    /// Returns the markup extension node wrapping `node` when its type provides a suitable
    /// `ProvideValue`/`ProvideTypedValue` method.
    pub fn try_convert_markup_extension(
        context: &AstTransformationContext,
        node: &Rc<dyn IXamlAstValueNode>,
    ) -> XamlResult<Option<Rc<XamlMarkupExtensionNode>>> {
        let cache = context.get_or_create_item::<MarkupExtensionProvideValueCache>();
        let node_type = node.type_().get_clr_type()?;
        let key = XamlTypeKey(node_type.clone());

        let cached = cache.type_to_provide_value.borrow().get(&key).cloned();
        let provide_value = match cached {
            Some(provide_value) => provide_value,
            None => {
                let candidates =
                    Self::get_markup_extension_provide_value_alternatives(context, &*node_type)?;
                let so = context.configuration().well_known_types().object.clone();
                let sp = context.configuration().type_mappings.service_provider()?;

                // Try non-object variant first and variants without IServiceProvider argument first
                let provide_value = candidates
                    .iter()
                    .find(|m| m.parameters().is_empty() && !m.return_type().equals(&*so))
                    .or_else(|| candidates.iter().find(|m| m.parameters().is_empty()))
                    .or_else(|| {
                        candidates.iter().find(|m| {
                            let p = m.parameters();
                            p.len() == 1 && p[0].equals(&*sp) && !m.return_type().equals(&*so)
                        })
                    })
                    .or_else(|| {
                        candidates.iter().find(|m| {
                            let p = m.parameters();
                            p.len() == 1 && p[0].equals(&*sp)
                        })
                    })
                    .cloned();
                cache
                    .type_to_provide_value
                    .borrow_mut()
                    .insert(key, provide_value.clone());
                provide_value
            }
        };

        let Some(provide_value) = provide_value else {
            if node.type_().is_markup_extension() {
                context.report_transform_error(
                    &format!(
                        "{} was resolved as markup extension, but doesn't have a matching ProvideValue/ProvideTypedValue method",
                        node_type.get_fqn()
                    ),
                    Some(&**node),
                    (),
                )?;
            }
            return Ok(None);
        };

        Ok(Some(XamlMarkupExtensionNode::new(
            &**node,
            provide_value,
            node.clone(),
        )))
    }

    /// `TryGetCorrectlyTypedValue(context, node, IXamlType xamlType, out rv)`.
    pub fn try_get_correctly_typed_value(
        context: &AstTransformationContext,
        node: &Rc<dyn IXamlAstValueNode>,
        xaml_type: &Rc<dyn IXamlType>,
    ) -> XamlResult<Option<Rc<dyn IXamlAstValueNode>>> {
        Self::try_get_correctly_typed_value_with_attributes(context, node, None, xaml_type)
    }

    /// `TryGetCorrectlyTypedValue(context, node, IXamlProperty property, out rv)`.
    pub fn try_get_correctly_typed_value_for_property(
        context: &AstTransformationContext,
        node: &Rc<dyn IXamlAstValueNode>,
        property: &dyn IXamlProperty,
    ) -> XamlResult<Option<Rc<dyn IXamlAstValueNode>>> {
        Self::try_get_correctly_typed_value_with_attributes(
            context,
            node,
            Some(&property.custom_attributes()),
            &property.property_type(),
        )
    }

    /// `TryGetCorrectlyTypedValue(context, node, IXamlParameterInfo parameterInfo, out rv)`.
    pub fn try_get_correctly_typed_value_for_parameter(
        context: &AstTransformationContext,
        node: &Rc<dyn IXamlAstValueNode>,
        parameter_info: &dyn IXamlParameterInfo,
    ) -> XamlResult<Option<Rc<dyn IXamlAstValueNode>>> {
        Self::try_get_correctly_typed_value_with_attributes(
            context,
            node,
            Some(&parameter_info.custom_attributes()),
            &parameter_info.parameter_type(),
        )
    }

    /// `TryGetCorrectlyTypedValue(context, node, customAttributes, IXamlType type, out rv)`.
    pub fn try_get_correctly_typed_value_with_attributes(
        context: &AstTransformationContext,
        node: &Rc<dyn IXamlAstValueNode>,
        custom_attributes: Option<&[Rc<dyn IXamlCustomAttribute>]>,
        type_: &Rc<dyn IXamlType>,
    ) -> XamlResult<Option<Rc<dyn IXamlAstValueNode>>> {
        if type_.is_assignable_from(&*node.type_().get_clr_type()?) {
            return Ok(Some(node.clone()));
        }

        Self::try_convert_value(context, node, custom_attributes, type_, None)
    }

    pub fn try_get_type_converter_from_custom_attribute(
        cfg: &TransformerConfiguration,
        attribute: Option<&Rc<dyn IXamlCustomAttribute>>,
    ) -> Option<Rc<dyn IXamlType>> {
        let attribute = attribute?;
        match attribute.parameters().into_iter().next() {
            Some(XamlValue::Type(t)) => Some(t),
            Some(XamlValue::String(sarg)) => cfg.type_system.find_type(&sarg),
            _ => None,
        }
    }

    pub fn get_common_base_class(types: &[Rc<dyn IXamlType>]) -> XamlResult<Rc<dyn IXamlType>> {
        let Some(first) = types.first() else {
            return Err(XamlError::argument("Input types array must not be empty"));
        };

        let mut ret = first.clone();

        for t in &types[1..] {
            if t.is_assignable_from(&*ret) {
                ret = t.clone();
            } else {
                // This will always terminate when ret == typeof(object)
                while !ret.is_assignable_from(&**t) {
                    ret = ret.base_type().ok_or_else(|| {
                        XamlError::internal(
                            "NullReferenceException",
                            "Unable to find a common base class",
                        )
                    })?;
                }
            }
        }

        Ok(ret)
    }

    fn create_invariant_culture(
        cfg: &TransformerConfiguration,
        line_info: &dyn IXamlLineInfo,
    ) -> XamlResult<Rc<dyn IXamlAstValueNode>> {
        let method = cfg
            .well_known_types()
            .culture_info
            .methods()
            .into_iter()
            .find(|x| x.is_public() && x.is_static() && x.name() == "get_InvariantCulture")
            .ok_or_else(|| XamlError::invalid_operation("Sequence contains no matching element"))?;
        Ok(XamlStaticOrTargetedReturnMethodCallNode::new(
            line_info, method, None,
        ))
    }

    pub fn try_convert_value(
        context: &AstTransformationContext,
        node: &Rc<dyn IXamlAstValueNode>,
        custom_attributes: Option<&[Rc<dyn IXamlCustomAttribute>]>,
        type_: &Rc<dyn IXamlType>,
        property_context: Option<&Rc<XamlAstClrProperty>>,
    ) -> XamlResult<Option<Rc<dyn IXamlAstValueNode>>> {
        let cfg = context.configuration().clone();
        let known = cfg.well_known_types();
        let mut type_ = type_.clone();
        // Since we are doing a conversion anyway, it makes sense to check for the underlying nullable type
        if type_
            .generic_type_definition()
            .is_some_and(|d| d.equals(&*known.nullable_t))
        {
            type_ = first_generic_argument(&type_)?;
        }

        let node_type = node.type_().get_clr_type()?;

        // Try with property-defined converter first
        if let Some(property_context) = property_context {
            let property_converter_type = property_context
                .type_converters
                .borrow()
                .get(&XamlTypeKey(type_.clone()))
                .cloned();
            if let Some(property_converter_type) = property_converter_type {
                return Ok(Some(Self::convert_with_converter(
                    node,
                    &property_converter_type,
                    &cfg,
                    &type_,
                )?));
            }
        }

        // Ask the hosting platform to apply its custom conversions
        if let Some(converter) = &cfg.custom_value_converter {
            let property_attributes;
            let attrs: Option<&[Rc<dyn IXamlCustomAttribute>]> = match custom_attributes {
                Some(a) if !a.is_empty() => Some(a),
                _ => match property_context {
                    Some(p) => {
                        property_attributes = p.custom_attributes();
                        Some(&property_attributes)
                    }
                    None => None,
                },
            };
            if let Some(rv) = converter(context, node, attrs, &type_)? {
                return Ok(Some(rv));
            }
        }

        // Implicit type converters
        if !node_type.equals(&*known.string) {
            return Ok(None);
        }

        if let Some(tn) = node.cast::<XamlAstTextNode>() {
            let text = tn.text();
            if type_.is_enum() {
                if let Some(enum_constant_node) =
                    TypeSystemHelpers::try_get_enum_value_node(&type_, &text, &*tn, false)?
                {
                    return Ok(Some(enum_constant_node));
                }
            }

            // Well known types
            if let Some(constant_node) =
                TypeSystemHelpers::parse_constant_if_type_allows(&text, &type_, &*tn)?
            {
                return Ok(Some(constant_node));
            }

            if type_.is("System", "Type") {
                let resolved_type = TypeReferenceResolver::resolve_type_by_xml_name(
                    context, &text, false, &*tn, true,
                )?;
                return Ok(Some(XamlTypeExtensionNode::new(
                    &*tn,
                    resolved_type,
                    type_.clone(),
                )));
            }

            if known.delegate.is_assignable_from(&*type_) {
                let invoke = type_.get_method(|m| m.name() == "Invoke")?;
                let root_object = context.root_object()?;
                let root_type = root_object.type_().get_clr_type()?;
                let handler = root_type.find_method_by_name(
                    &text,
                    &*invoke.return_type(),
                    false,
                    &invoke.parameters(),
                );
                if let Some(handler) = handler {
                    return Ok(Some(XamlLoadMethodDelegateNode::new(
                        &*tn,
                        root_object,
                        type_.clone(),
                        handler,
                    )));
                }
            }
        }

        let candidates: Vec<Rc<dyn IXamlMethod>> = type_
            .methods()
            .into_iter()
            .filter(|m| {
                m.name() == "Parse" && m.return_type().equals(&*type_) && {
                    let p = m.parameters();
                    !p.is_empty() && p[0].equals(&*known.string)
                }
            })
            .collect();

        // Types with parse method
        let parser = candidates
            .iter()
            .find(|m| {
                let p = m.parameters();
                p.len() == 2
                    && (p[1].equals(&*known.culture_info) || p[1].equals(&*known.i_format_provider))
            })
            .or_else(|| candidates.iter().find(|m| m.parameters().len() == 1))
            .cloned();

        if let Some(parser) = parser {
            let mut args: Vec<Rc<dyn IXamlAstValueNode>> = vec![node.clone()];
            if parser.parameters().len() == 2 {
                args.push(Self::create_invariant_culture(&cfg, &**node)?);
            }

            return Ok(Some(XamlStaticOrTargetedReturnMethodCallNode::new(
                &**node,
                parser,
                Some(args),
            )));
        }

        if cfg.type_mappings.type_descriptor_context.is_some() {
            let type_converter_attribute = cfg
                .get_custom_attributes_for_type(
                    &*type_,
                    &cfg.type_mappings.type_converter_attributes,
                )
                .into_iter()
                .next();
            if let Some(type_converter_attribute) = type_converter_attribute {
                let converter_type = Self::try_get_type_converter_from_custom_attribute(
                    &cfg,
                    Some(&type_converter_attribute),
                );
                if let Some(converter_type) = converter_type {
                    return Ok(Some(Self::convert_with_converter(
                        node,
                        &converter_type,
                        &cfg,
                        &type_,
                    )?));
                }
            }
        }

        Ok(None)
    }

    fn convert_with_converter(
        node: &Rc<dyn IXamlAstValueNode>,
        converter_type: &Rc<dyn IXamlType>,
        cfg: &TransformerConfiguration,
        type_: &Rc<dyn IXamlType>,
    ) -> XamlResult<Rc<dyn IXamlAstValueNode>> {
        let known = cfg.well_known_types();
        let type_descriptor_context = cfg
            .type_mappings
            .type_descriptor_context
            .clone()
            .ok_or_else(|| {
                XamlError::internal(
                    "NullReferenceException",
                    "XamlLanguageTypeMappings.TypeDescriptorContext is not set",
                )
            })?;
        let li: &dyn IXamlLineInfo = &**node;
        let converter_method = converter_type.get_method_by_name(
            "ConvertFrom",
            &*known.object,
            false,
            &[
                type_descriptor_context.clone(),
                known.culture_info.clone(),
                known.object.clone(),
            ],
        )?;
        let call_arguments: Vec<Rc<dyn IXamlAstValueNode>> = vec![
            XamlAstNewClrObjectNode::new(
                li,
                XamlAstClrTypeReference::new(li, converter_type.clone(), false),
                converter_type.get_constructor(None)?,
                Vec::new(),
            ),
            XamlAstContextLocalNode::new(li, type_descriptor_context),
            Self::create_invariant_culture(cfg, li)?,
            node.clone(),
        ];
        Ok(XamlAstNeedsParentStackValueNode::new(
            li,
            XamlAstRuntimeCastNode::new(
                li,
                XamlStaticOrTargetedReturnMethodCallNode::new(
                    li,
                    converter_method,
                    Some(call_arguments),
                ),
                XamlAstClrTypeReference::new(li, type_.clone(), false),
            ),
        ))
    }
}

fn first_generic_argument(type_: &Rc<dyn IXamlType>) -> XamlResult<Rc<dyn IXamlType>> {
    type_.generic_arguments().into_iter().next().ok_or_else(|| {
        XamlError::internal(
            "ArgumentOutOfRangeException",
            format!("{} doesn't have generic arguments", type_.get_fqn()),
        )
    })
}
