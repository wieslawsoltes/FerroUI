//! Port of `CompilerExtensions/Transformers/FerroXamlIlMetadataRemover.cs`.

use std::rc::Rc;

use xamlx::ast::{IXamlAstNode, XamlAstNodeExtensions};
use xamlx::exceptions::XamlResult;
use xamlx::transform::{AstTransformationContext, IXamlAstTransformer};

use super::FerroXamlIlTargetTypeMetadataNode;

/// Replaces every target-type metadata node with the value it wraps.
pub struct FerroXamlIlMetadataRemover;

impl IXamlAstTransformer for FerroXamlIlMetadataRemover {
    fn transform(
        &self,
        _context: &AstTransformationContext,
        node: Rc<dyn IXamlAstNode>,
    ) -> XamlResult<Rc<dyn IXamlAstNode>> {
        let mut node = node;
        while let Some(target_type) = node.cast::<FerroXamlIlTargetTypeMetadataNode>() {
            node = target_type.value();
        }
        Ok(node)
    }
}
