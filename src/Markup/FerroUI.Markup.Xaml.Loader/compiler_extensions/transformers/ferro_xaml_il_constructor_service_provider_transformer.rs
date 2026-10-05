//! Port of
//! `CompilerExtensions/Transformers/FerroXamlIlConstructorServiceProviderTransformer.cs`.

use std::rc::Rc;

use xamlx::ast::{
    IXamlAstNode, IXamlAstNodeNeedsParentStack, IXamlAstTypeReference, IXamlAstValueNode,
    IXamlLineInfo, XamlAstClrTypeReference, XamlAstExtensions, XamlAstNode,
    XamlAstNodeExtensions, XamlAstObjectNode,
};
use xamlx::exceptions::XamlResult;
use xamlx::transform::{AstTransformationContext, IXamlAstTransformer};
use xamlx::type_system::IXamlType;
use xamlx::{xaml_ast_node_members, xaml_line_info_impl};

/// Passes the service provider to objects that can only be constructed with one: an object
/// without constructor arguments whose type has no public parameterless constructor but has a
/// public constructor taking a single `IServiceProvider` gets an
/// [`InjectServiceProviderNode`] as its argument.
pub struct FerroXamlIlConstructorServiceProviderTransformer;

impl IXamlAstTransformer for FerroXamlIlConstructorServiceProviderTransformer {
    fn transform(
        &self,
        context: &AstTransformationContext,
        node: Rc<dyn IXamlAstNode>,
    ) -> XamlResult<Rc<dyn IXamlAstNode>> {
        if let Some(on) = node.cast::<XamlAstObjectNode>() {
            if on.arguments.borrow().is_empty() {
                let type_ = on.type_.borrow().clone();
                let ctors = type_.get_clr_type()?.constructors();
                if !ctors
                    .iter()
                    .any(|c| c.is_public() && !c.is_static() && c.parameters().is_empty())
                {
                    let sp = context.configuration().type_mappings.service_provider()?;
                    if ctors.iter().any(|c| {
                        let parameters = c.parameters();
                        c.is_public()
                            && !c.is_static()
                            && parameters.len() == 1
                            && parameters[0].equals(&*sp)
                    }) {
                        let argument = InjectServiceProviderNode::new(sp, &*on);
                        on.arguments.borrow_mut().push(argument);
                    }
                }
            }
        }
        Ok(node)
    }
}

/// A value node of the service provider type that stands for "the current service provider".
/// It needs the parent stack.
///
/// # What a back end has to do (upstream IL)
///
/// `Emit` (leaves one value): load the local that holds the runtime context of the method
/// being generated (the context object implements `IServiceProvider`). The result is typed as
/// the node's type; nothing is consumed.
pub struct InjectServiceProviderNode {
    base: XamlAstNode,
    type_: Rc<dyn IXamlAstTypeReference>,
}

impl InjectServiceProviderNode {
    pub fn new(type_: Rc<dyn IXamlType>, line_info: &dyn IXamlLineInfo) -> Rc<Self> {
        Rc::new(Self {
            base: XamlAstNode::new(line_info),
            type_: XamlAstClrTypeReference::new(line_info, type_, false),
        })
    }
}

xaml_line_info_impl!(InjectServiceProviderNode, base);

impl IXamlAstNode for InjectServiceProviderNode {
    xaml_ast_node_members!("InjectServiceProviderNode", value, needs_parent_stack);
}

impl IXamlAstValueNode for InjectServiceProviderNode {
    fn type_(&self) -> Rc<dyn IXamlAstTypeReference> {
        self.type_.clone()
    }
}

impl IXamlAstNodeNeedsParentStack for InjectServiceProviderNode {
    fn needs_parent_stack(&self) -> bool {
        true
    }
}
