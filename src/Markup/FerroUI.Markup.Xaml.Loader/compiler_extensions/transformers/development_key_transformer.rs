//! Preserve explicit reload identity without adding a public Name or namescope entry.
use std::rc::Rc;
use xamlx::ast::{
    IXamlAstManipulationNode, IXamlAstNode, XamlAstNode, XamlAstNodeExtensions, XamlAstTextNode,
    XamlAstXmlDirective,
};
use xamlx::exceptions::{XamlError, XamlResult};
use xamlx::transform::{AstTransformationContext, IXamlAstTransformer};
use xamlx::{xaml_ast_node_members, xaml_line_info_impl};

pub const DEVELOPMENT_NAMESPACE: &str = "https://github.com/ferroui/hot-reload";
pub struct DevelopmentKeyTransformer;
impl IXamlAstTransformer for DevelopmentKeyTransformer {
    fn transform(
        &self,
        _: &AstTransformationContext,
        node: Rc<dyn IXamlAstNode>,
    ) -> XamlResult<Rc<dyn IXamlAstNode>> {
        let Some(directive) = node.cast::<XamlAstXmlDirective>() else {
            return Ok(node);
        };
        if directive.namespace.borrow().as_deref() != Some(DEVELOPMENT_NAMESPACE)
            || *directive.name.borrow() != "Key"
        {
            return Ok(node);
        }
        let values = directive.values.borrow();
        let key = (values.len() == 1)
            .then(|| values[0].cast::<XamlAstTextNode>())
            .flatten()
            .map(|text| text.text())
            .filter(|text| !text.is_empty())
            .ok_or_else(|| {
                XamlError::parse_exception(
                    "Development Key must be a nonempty literal",
                    Some(&*directive),
                )
            })?;
        Ok(Rc::new(DevelopmentKeyNode {
            base: XamlAstNode::new(&*directive),
            key,
        }))
    }
}
pub struct DevelopmentKeyNode {
    base: XamlAstNode,
    pub key: String,
}
xaml_line_info_impl!(DevelopmentKeyNode, base);
impl IXamlAstNode for DevelopmentKeyNode {
    xaml_ast_node_members!("DevelopmentKeyNode", manipulation);
}
impl IXamlAstManipulationNode for DevelopmentKeyNode {}
