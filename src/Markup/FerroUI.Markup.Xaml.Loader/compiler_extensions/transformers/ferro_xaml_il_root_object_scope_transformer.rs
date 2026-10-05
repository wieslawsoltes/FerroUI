//! Port of `CompilerExtensions/Transformers/FerroXamlIlRootObjectScopeTransformer.cs`.

use std::rc::Rc;

use xamlx::ast::{
    IXamlAstManipulationNode, IXamlAstNode, IXamlLineInfo, XamlAstNode, XamlManipulationGroupNode,
};
use xamlx::exceptions::XamlResult;
use xamlx::transform::{AstTransformationContext, IXamlAstTransformer};
use xamlx::{xaml_ast_node_members, xaml_line_info_impl};

use super::{FerroXamlIlWellKnownTypes, FerroXamlIlWellKnownTypesExtensions};

/// Appends a [`HandleRootObjectScopeNode`] to the manipulation of the document's root object:
/// after the root has been populated its name scope is attached and completed.
pub struct FerroXamlIlRootObjectScope;

impl IXamlAstTransformer for FerroXamlIlRootObjectScope {
    fn transform(
        &self,
        context: &AstTransformationContext,
        node: Rc<dyn IXamlAstNode>,
    ) -> XamlResult<Rc<dyn IXamlAstNode>> {
        if !context.has_parent_nodes() {
            if let Some(mnode) = node.as_value_with_manipulation_node() {
                let mut children: Vec<Rc<dyn IXamlAstManipulationNode>> = Vec::new();
                // Upstream assumes the root always has a manipulation (`mnode.Manipulation!`).
                if let Some(manipulation) = mnode.manipulation() {
                    children.push(manipulation);
                }
                children.push(HandleRootObjectScopeNode::new(
                    &*node,
                    context.try_get_ferro_types()?,
                ));
                *mnode.manipulation.borrow_mut() =
                    Some(XamlManipulationGroupNode::new(&*node, Some(children)));
            }
        }
        Ok(node)
    }
}

/// Attaches the document's name scope to the root object and completes the scope.
///
/// # What a back end has to do (upstream `FerroXamlIlRootObjectScope.Emitter`)
///
/// The emitter handles exactly this node type. Stack: the root object; it is consumed and
/// nothing is left. `scope` below is the name scope stored in the runtime context field named
/// `FerroXamlIlLanguage::CONTEXT_NAME_SCOPE_FIELD_NAME` (see
/// [`crate::compiler_extensions::FerroXamlIlContextNameScopeField`]):
///
/// 1. test whether the root object is a `StyledElement` (`isinst types.styled_element`);
/// 2. when it is, call the static method `types.name_scope_set_name_scope`:
///    `NameScope.SetNameScope(styledElement, scope)`;
/// 3. in both cases, call `types.i_name_scope_complete` on the scope: `scope.Complete()`.
///
/// That is `if (root is StyledElement s) NameScope.SetNameScope(s, context.FerroNameScope);
/// context.FerroNameScope.Complete();`.
pub struct HandleRootObjectScopeNode {
    base: XamlAstNode,
    pub types: Rc<FerroXamlIlWellKnownTypes>,
}

impl HandleRootObjectScopeNode {
    pub fn new(line_info: &dyn IXamlLineInfo, types: Rc<FerroXamlIlWellKnownTypes>) -> Rc<Self> {
        Rc::new(Self {
            base: XamlAstNode::new(line_info),
            types,
        })
    }
}

xaml_line_info_impl!(HandleRootObjectScopeNode, base);

impl IXamlAstNode for HandleRootObjectScopeNode {
    xaml_ast_node_members!("HandleRootObjectScopeNode", manipulation);
}

impl IXamlAstManipulationNode for HandleRootObjectScopeNode {}
