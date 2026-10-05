//! Port of `Transform/Transformers/ConstructableObjectTransformer.cs`.

use std::rc::Rc;

use crate::ast::{
    IXamlAstNode, XamlAstConstructableObjectNode, XamlAstExtensions, XamlAstNodeExtensions,
    XamlAstObjectNode,
};
use crate::exceptions::{XamlError, XamlResult};
use crate::transform::{AstTransformationContext, IXamlAstTransformer, XamlTransformHelpers};
use crate::type_system::{IXamlConstructor, IXamlType};

pub struct ConstructableObjectTransformer;

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
            if t.is_value_type() {
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
