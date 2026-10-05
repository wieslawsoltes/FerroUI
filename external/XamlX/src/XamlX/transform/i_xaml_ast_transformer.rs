//! Port of `Transform/IXamlAstTransformer.cs`.

use std::rc::Rc;

use crate::ast::IXamlAstNode;
use crate::exceptions::XamlResult;

use super::AstTransformationContext;

pub trait IXamlAstTransformer {
    fn transform(
        &self,
        context: &AstTransformationContext,
        node: Rc<dyn IXamlAstNode>,
    ) -> XamlResult<Rc<dyn IXamlAstNode>>;

    /// `GetType().Name`, used in "internal compiler error" messages.
    fn transformer_name(&self) -> String {
        let full = std::any::type_name::<Self>();
        full.rsplit("::").next().unwrap_or(full).to_string()
    }
}
