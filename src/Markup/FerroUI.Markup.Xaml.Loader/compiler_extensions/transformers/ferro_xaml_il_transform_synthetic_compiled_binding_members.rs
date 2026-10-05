//! Port of
//! `CompilerExtensions/Transformers/FerroXamlIlTransformSyntheticCompiledBindingMembers.cs`.

use std::rc::Rc;

use xamlx::ast::{
    IXamlAstNode, IXamlAstPropertyReference, IXamlLineInfo, XamlAstClrTypeReference, XamlAstNode,
    XamlAstNamePropertyReference, XamlAstNodeExtensions,
};
use xamlx::exceptions::XamlResult;
use xamlx::transform::{AstTransformationContext, IXamlAstTransformer};
use xamlx::{xaml_ast_node_members, xaml_line_info_impl};

use super::FerroXamlIlWellKnownTypesExtensions;

/// Replaces the `ElementName` and `RelativeSource` property references of a compiled binding
/// extension, which the extension class does not declare, with synthetic property references.
/// The binding path parser turns their values into path nodes and removes them.
pub struct FerroXamlIlTransformSyntheticCompiledBindingMembers;

impl IXamlAstTransformer for FerroXamlIlTransformSyntheticCompiledBindingMembers {
    fn transform(
        &self,
        context: &AstTransformationContext,
        node: Rc<dyn IXamlAstNode>,
    ) -> XamlResult<Rc<dyn IXamlAstNode>> {
        if let Some(prop) = node.cast::<XamlAstNamePropertyReference>() {
            let target_type = prop.target_type.borrow().clone();
            if let Some(target_ref) = target_type.cast::<XamlAstClrTypeReference>() {
                if target_ref
                    .type_
                    .equals(&*context.try_get_ferro_types()?.compiled_binding_extension)
                {
                    let name = prop.name();
                    if name == "ElementName" {
                        return Ok(FerroSyntheticCompiledBindingProperty::new(
                            &*node,
                            SyntheticCompiledBindingPropertyName::ElementName,
                        ));
                    } else if name == "RelativeSource" {
                        return Ok(FerroSyntheticCompiledBindingProperty::new(
                            &*node,
                            SyntheticCompiledBindingPropertyName::RelativeSource,
                        ));
                    }
                }
            }
        }

        Ok(node)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SyntheticCompiledBindingPropertyName {
    ElementName,
    RelativeSource,
}

pub struct FerroSyntheticCompiledBindingProperty {
    base: XamlAstNode,
    pub name: SyntheticCompiledBindingPropertyName,
}

impl FerroSyntheticCompiledBindingProperty {
    pub fn new(
        line_info: &dyn IXamlLineInfo,
        name: SyntheticCompiledBindingPropertyName,
    ) -> Rc<Self> {
        Rc::new(Self {
            base: XamlAstNode::new(line_info),
            name,
        })
    }
}

xaml_line_info_impl!(FerroSyntheticCompiledBindingProperty, base);

impl IXamlAstNode for FerroSyntheticCompiledBindingProperty {
    xaml_ast_node_members!("FerroSyntheticCompiledBindingProperty", property_reference);
}

impl IXamlAstPropertyReference for FerroSyntheticCompiledBindingProperty {}
