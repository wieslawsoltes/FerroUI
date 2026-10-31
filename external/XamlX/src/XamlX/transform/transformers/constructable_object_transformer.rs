//! Port of `Transform/Transformers/ConstructableObjectTransformer.cs`.

use std::rc::Rc;

use crate::ast::{
    IXamlAstNode, XamlAstConstructableObjectNode, XamlAstExtensions, XamlAstNodeExtensions,
    XamlAstObjectNode, IXamlAstValueNode, XamlAstTextNode, XamlConstantNode, XamlNullExtensionNode,
};
use crate::exceptions::{XamlError, XamlResult};
use crate::transform::{AstTransformationContext, IXamlAstTransformer, XamlTransformHelpers};
use crate::type_system::{IXamlConstructor, IXamlParameterInfo, IXamlType, XamlValue};

pub struct ConstructableObjectTransformer;

/// Backend capability: the Rust host can construct values through typed metadata
/// functions. The default retains the original CLR/IL restriction.
#[derive(Default)]
pub struct ConstructableObjectTransformerOptions {
    pub allow_value_type_construction: bool,
    /// Native callers may materialize explicit Optional/DefaultParameterValue metadata.
    /// The default keeps the original exact-arity constructor selection.
    pub allow_optional_constructor_arguments: bool,
}

fn optional_default(
    context: &AstTransformationContext,
    node: &XamlAstObjectNode,
    parameter: &dyn IXamlParameterInfo,
) -> XamlResult<Option<Rc<dyn IXamlAstValueNode>>> {
    if !parameter.is_optional() {
        return Ok(None);
    }
    let defaults = parameter
        .custom_attributes()
        .into_iter()
        .filter(|a| {
            a.type_().full_name() == "System.Runtime.InteropServices.DefaultParameterValueAttribute"
        })
        .collect::<Vec<_>>();
    if defaults.len() != 1 || defaults[0].parameters().len() != 1 {
        return Err(XamlError::load_exception(
            "Optional constructor parameters require exactly one explicit default value",
            Some(node),
        ));
    }
    let value = defaults[0].parameters().remove(0);
    let target = parameter.parameter_type();
    let literal: Rc<dyn IXamlAstValueNode> = match value {
        XamlValue::Null => XamlNullExtensionNode::new(node),
        XamlValue::String(text) => XamlAstTextNode::with_type(
            node,
            &text,
            true,
            Some(context.configuration().well_known_types().string.clone()),
        ),
        value if value.is_primitive() => {
            let primitive = if target.is_enum() {
                target.get_enum_underlying_type()?
            } else {
                target.clone()
            };
            let integer = match &value {
                XamlValue::SByte(v) => Some(i128::from(*v)),
                XamlValue::Byte(v) => Some(i128::from(*v)),
                XamlValue::Int16(v) => Some(i128::from(*v)),
                XamlValue::UInt16(v) => Some(i128::from(*v)),
                XamlValue::Int32(v) => Some(i128::from(*v)),
                XamlValue::UInt32(v) => Some(i128::from(*v)),
                XamlValue::Int64(v) => Some(i128::from(*v)),
                XamlValue::UInt64(v) => Some(i128::from(*v)),
                _ => None,
            };
            // Native attribute integers have a shared storage width. Materialize the declared
            // parameter width with a checked conversion; never truncate a default constant.
            let value = if let Some(integer) = integer {
                macro_rules! checked {
                    ($variant:ident,$type:ty) => {
                        XamlValue::$variant(<$type>::try_from(integer).map_err(|_| {
                            XamlError::load_exception(
                                "Constructor default integer is out of range",
                                Some(node),
                            )
                        })?)
                    };
                }
                match primitive.full_name().as_str() {
                    "System.SByte" => checked!(SByte, i8),
                    "System.Byte" => checked!(Byte, u8),
                    "System.Int16" => checked!(Int16, i16),
                    "System.UInt16" => checked!(UInt16, u16),
                    "System.Int32" => checked!(Int32, i32),
                    "System.UInt32" => checked!(UInt32, u32),
                    "System.Int64" => checked!(Int64, i64),
                    "System.UInt64" => checked!(UInt64, u64),
                    _ => value,
                }
            } else {
                value
            };
            let constant_type = if value.type_name() == primitive.full_name() {
                target.clone()
            } else {
                context
                    .configuration()
                    .type_system
                    .find_type(value.type_name())
                    .ok_or_else(|| {
                        XamlError::load_exception(
                            "Constructor default constant type is unavailable",
                            Some(node),
                        )
                    })?
            };
            XamlConstantNode::new(node, constant_type, value)?
        }
        _ => {
            return Err(XamlError::load_exception(
                "Unsupported constructor default constant",
                Some(node),
            ))
        }
    };
    // Defaults are metadata constants, not user text: do not run a converter to
    // turn an invalid string default into a number or an arbitrary custom object.
    if literal.as_node().cast::<XamlNullExtensionNode>().is_none()
        && !target.is_assignable_from(&*literal.type_().get_clr_type()?)
    {
        return Err(XamlError::load_exception(
            "Constructor default is not assignable to its parameter",
            Some(node),
        ));
    }
    XamlTransformHelpers::try_get_correctly_typed_value_for_parameter(context, &literal, parameter)?
        .map(Some)
        .ok_or_else(|| {
            XamlError::load_exception(
                "Constructor default is not assignable to its parameter",
                Some(node),
            )
        })
}

fn optional_constructor(
    context: &AstTransformationContext,
    node: &XamlAstObjectNode,
    type_: &Rc<dyn IXamlType>,
) -> XamlResult<Option<Rc<dyn IXamlConstructor>>> {
    let arguments = node.arguments.borrow().clone();
    let mut matches = Vec::new();
    for constructor in type_
        .constructors()
        .into_iter()
        .filter(|c| c.is_public() && !c.is_static() && c.parameters().len() > arguments.len())
    {
        let mut values = Vec::new();
        let mut compatible = true;
        // Verify all omitted parameters first; optional parameters cannot skip a required tail.
        for index in arguments.len()..constructor.parameters().len() {
            if !constructor.get_parameter_info(index)?.is_optional() {
                compatible = false;
                break;
            }
        }
        if !compatible {
            continue;
        }
        for (index, argument) in arguments.iter().enumerate() {
            let parameter = constructor.get_parameter_info(index)?;
            if let Some(value) = XamlTransformHelpers::try_get_correctly_typed_value_for_parameter(
                context,
                argument,
                &*parameter,
            )? {
                values.push(value)
            } else {
                compatible = false;
                break;
            }
        }
        if !compatible {
            continue;
        }
        for index in arguments.len()..constructor.parameters().len() {
            let parameter = constructor.get_parameter_info(index)?;
            let value = optional_default(context, node, &*parameter)?.ok_or_else(|| {
                XamlError::load_exception(
                    "Required constructor parameter cannot be omitted",
                    Some(node),
                )
            })?;
            values.push(value);
        }
        matches.push((constructor, values));
    }
    if matches.len() > 1 {
        return Err(XamlError::load_exception(
            "Ambiguous optional constructor arguments",
            Some(node),
        ));
    }
    if let Some((constructor, arguments)) = matches.pop() {
        *node.arguments.borrow_mut() = arguments;
        Ok(Some(constructor))
    } else {
        Ok(None)
    }
}

fn transform_arguments_and_get_constructor(
    context: &AstTransformationContext,
    n: &XamlAstObjectNode,
) -> XamlResult<Option<Rc<dyn IXamlConstructor>>> {
    let type_ = n.type_.borrow().get_clr_type()?;

    let arguments = n.arguments.borrow().clone();
    let mut arg_types: Vec<Rc<dyn IXamlType>> = Vec::with_capacity(arguments.len());
    for a in &arguments {
        arg_types.push(a.type_().get_clr_type()?);
    }
    let mut ctor = type_.find_constructor(Some(&arg_types));
    if ctor.is_none() {
        if !arg_types.is_empty() {
            ctor = type_.constructors().into_iter().find(|x| {
                !x.is_static() && x.is_public() && x.parameters().len() == arg_types.len()
            });
        }

        if ctor.is_none() {
            if context
                .configuration()
                .get_or_create_extra::<ConstructableObjectTransformerOptions>()
                .allow_optional_constructor_arguments
            {
                return optional_constructor(context, n, &type_);
            }
            return Ok(None);
        }
    }
    let Some(ctor) = ctor else { return Ok(None) };

    for (c, argument) in arguments.iter().enumerate() {
        let parameter_info = ctor.get_parameter_info(c)?;
        match XamlTransformHelpers::try_get_correctly_typed_value_for_parameter(
            context,
            argument,
            &*parameter_info,
        )? {
            Some(arg) => n.arguments.borrow_mut()[c] = arg,
            None => {
                return Err(XamlError::load_exception(
                    format!(
                        "Unable to convert {} to {} for constructor of {}",
                        argument.type_().get_clr_type()?.get_fqn(),
                        ctor.parameters()[c].get_fqn(),
                        n.type_.borrow().get_clr_type()?.get_fqn()
                    ),
                    Some(&**argument),
                ))
            }
        }
    }

    Ok(Some(ctor))
}

impl IXamlAstTransformer for ConstructableObjectTransformer {
    fn transform(
        &self,
        context: &AstTransformationContext,
        node: Rc<dyn IXamlAstNode>,
    ) -> XamlResult<Rc<dyn IXamlAstNode>> {
        if let Some(ni) = node.cast::<XamlAstObjectNode>() {
            let t = ni.type_.borrow().get_clr_type()?;
            if t.is_value_type()
                && !context
                    .configuration()
                    .get_or_create_extra::<ConstructableObjectTransformerOptions>()
                    .allow_value_type_construction
            {
                return Err(XamlError::load_exception(
                    "Value types can only be loaded via converters. We don't want to mess with indirect loads and other weird stuff",
                    Some(&*node),
                ));
            }

            let matching_ctor_is_required = context.has_parent_nodes();
            let ctor = transform_arguments_and_get_constructor(context, &ni)?;
            let type_reference = ni.type_.borrow().get_clr_type_reference()?;
            if let Some(ctor) = ctor {
                return Ok(XamlAstConstructableObjectNode::new(
                    &*ni,
                    type_reference,
                    ctor,
                    ni.arguments.borrow().clone(),
                    ni.children.borrow().clone(),
                ));
            } else if !matching_ctor_is_required {
                // If matching ctor isn't required and it wasn't found, pass the first possible ctor.
                // But don't pass any arguments, as compiler doesn't know what to pass at this point.
                let first_ctor = t.constructors().into_iter().next().ok_or_else(|| {
                    XamlError::internal(
                        "ArgumentOutOfRangeException",
                        format!("Type {} doesn't have any constructors", t.get_fqn()),
                    )
                })?;
                return Ok(XamlAstConstructableObjectNode::new(
                    &*ni,
                    type_reference,
                    first_ctor,
                    Vec::new(),
                    ni.children.borrow().clone(),
                ));
            } else {
                let mut argument_names = Vec::new();
                for at in ni.arguments.borrow().iter() {
                    argument_names.push(at.type_().get_clr_type()?.get_fqn());
                }
                return Err(XamlError::load_exception(
                    format!(
                        "Unable to find public constructor for type {}({})",
                        t.get_fqn(),
                        argument_names.join(", ")
                    ),
                    Some(&*ni),
                ));
            }
        }

        Ok(node)
    }
}
