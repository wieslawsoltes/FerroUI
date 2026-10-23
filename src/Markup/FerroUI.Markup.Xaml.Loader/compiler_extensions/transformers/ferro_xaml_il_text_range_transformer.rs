//! Construction of the WinUI `TextRange` value through its FerroUI markup carrier.
//!
//! `TextHighlightRange` retains value semantics in Rust and in both type systems.
//! Its declared constructors return one shared carrier while the normal property
//! assignments initialize it; the collection adder then copies the completed value.
//! Lowering this explicit contract before XamlX's general value-type guard keeps
//! runtime loading and Rust emission on the same constructor and assignment nodes.
//! No other value type acquires mutable object initialization through this adapter.

use std::rc::Rc;

use xamlx::ast::{
    IXamlAstNode, XamlAstConstructableObjectNode, XamlAstExtensions, XamlAstNodeExtensions,
    XamlAstObjectNode,
};
use xamlx::exceptions::{XamlError, XamlResult};
use xamlx::transform::{AstTransformationContext, IXamlAstTransformer, XamlTransformHelpers};

pub struct FerroXamlIlTextRangeTransformer;

impl IXamlAstTransformer for FerroXamlIlTextRangeTransformer {
    fn transform(
        &self,
        context: &AstTransformationContext,
        node: Rc<dyn IXamlAstNode>,
    ) -> XamlResult<Rc<dyn IXamlAstNode>> {
        let Some(object) = node.cast::<XamlAstObjectNode>() else {
            return Ok(node);
        };
        let type_ = object.type_.borrow().get_clr_type()?;
        if !type_.is_value_type() || !type_.is("FerroUI.Controls.Documents", "TextRange") {
            return Ok(node);
        }

        let arguments = object.arguments.borrow().clone();
        let argument_types = arguments
            .iter()
            .map(|argument| argument.type_().get_clr_type())
            .collect::<XamlResult<Vec<_>>>()?;
        let constructor = type_
            .find_constructor(Some(&argument_types))
            .or_else(|| {
                type_.constructors().into_iter().find(|constructor| {
                    constructor.is_public()
                        && !constructor.is_static()
                        && constructor.parameters().len() == arguments.len()
                })
            })
            .ok_or_else(|| {
                XamlError::load_exception(
                    format!(
                        "Unable to find a public constructor for TextRange with {} arguments",
                        arguments.len()
                    ),
                    Some(&*node),
                )
            })?;

        let mut converted = Vec::with_capacity(arguments.len());
        for (index, argument) in arguments.iter().enumerate() {
            let parameter = constructor.get_parameter_info(index)?;
            let value = XamlTransformHelpers::try_get_correctly_typed_value_for_parameter(
                context, argument, &*parameter,
            )?
            .ok_or_else(|| {
                XamlError::load_exception(
                    format!(
                        "Unable to convert constructor argument {} to {} for TextRange",
                        index,
                        constructor.parameters()[index].get_fqn()
                    ),
                    Some(&**argument),
                )
            })?;
            converted.push(value);
        }

        let type_reference = object.type_.borrow().get_clr_type_reference()?;
        let children = object.children.borrow().clone();
        Ok(XamlAstConstructableObjectNode::new(
            &*object,
            type_reference,
            constructor,
            converted,
            children,
        ))
    }
}
