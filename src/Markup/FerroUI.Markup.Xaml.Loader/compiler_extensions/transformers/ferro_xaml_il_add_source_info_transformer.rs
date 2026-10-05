//! Port of `CompilerExtensions/Transformers/FerroXamlIlAddSourceInfoTransformer.cs`.

use std::cell::Cell;
use std::rc::Rc;

use xamlx::ast::{
    IXamlAstManipulationNode, IXamlAstNode, XamlAstExtensions, XamlAstNewClrObjectNode,
    XamlAstNode, XamlAstNodeExtensions, XamlValueWithManipulationNode,
};
use xamlx::exceptions::XamlResult;
use xamlx::transform::{AstTransformationContext, IXamlAstTransformer};
use xamlx::{xaml_ast_node_members, xaml_line_info_impl};

use super::{FerroXamlIlWellKnownTypes, FerroXamlIlWellKnownTypesExtensions};

/// An AST transformer that injects `XamlSourceInfo` metadata into the generated XAML code.
///
/// This transformer wraps object creation nodes with a manipulation node that adds source
/// information. This source information includes line number, position, and document name,
/// which can be useful for debugging and diagnostics.
/// Note: ResourceDictionary source info is handled separately in
/// [`super::FerroXamlResourceTransformer`].
#[derive(Default)]
pub struct FerroXamlIlAddSourceInfoTransformer {
    create_source_info: Cell<bool>,
}

impl FerroXamlIlAddSourceInfoTransformer {
    pub fn new() -> Self {
        Self::default()
    }

    /// Gets a value indicating whether source information should be generated and injected
    /// into the compiled XAML output.
    pub fn create_source_info(&self) -> bool {
        self.create_source_info.get()
    }

    pub fn set_create_source_info(&self, value: bool) {
        self.create_source_info.set(value);
    }
}

impl IXamlAstTransformer for FerroXamlIlAddSourceInfoTransformer {
    fn transform(
        &self,
        context: &AstTransformationContext,
        node: Rc<dyn IXamlAstNode>,
    ) -> XamlResult<Rc<dyn IXamlAstNode>> {
        if !self.create_source_info.get() {
            return Ok(node);
        }
        let Some(obj_node) = node.cast::<XamlAstNewClrObjectNode>() else {
            return Ok(node);
        };
        let already_wrapped = context.first_parent_node().is_some_and(|parent| {
            parent
                .as_value_with_manipulation_node()
                .and_then(|parent| parent.manipulation())
                .is_some_and(|manipulation| manipulation.is::<XamlSourceInfoValueManipulation>())
        });
        if already_wrapped {
            return Ok(node);
        }
        let obj_type = obj_node.type_.borrow().clone();
        if obj_type.get_clr_type()?.is_value_type() {
            return Ok(node);
        }

        let ferro_types = context.try_get_ferro_types()?;
        Ok(XamlValueWithManipulationNode::new(
            &*obj_node,
            obj_node.clone(),
            Some(XamlSourceInfoValueManipulation::new(
                ferro_types,
                &obj_node,
                context.current_document(),
            )),
        ))
    }
}

/// Attaches the source location of an object creation to the created object.
///
/// # What a back end has to do (upstream IL)
///
/// `Emit` (stack: the target object; consumes it, leaves nothing):
///
/// 1. load the `int` constants `line()` and `position()` of this node (the location of the
///    object creation node);
/// 2. load the string constant [`XamlSourceInfoValueManipulation::document`], or `null` when
///    there is none;
/// 3. construct the source info with `types.xaml_source_info_constructor`:
///    `new XamlSourceInfo(line, position, document)`;
/// 4. call the static method `types.xaml_source_info_setter`:
///    `XamlSourceInfo.SetXamlSourceInfo(target, info)`.
pub struct XamlSourceInfoValueManipulation {
    base: XamlAstNode,
    pub types: Rc<FerroXamlIlWellKnownTypes>,
    pub document: Option<String>,
}

impl XamlSourceInfoValueManipulation {
    pub fn new(
        ferro_types: Rc<FerroXamlIlWellKnownTypes>,
        obj_node: &Rc<XamlAstNewClrObjectNode>,
        document: Option<String>,
    ) -> Rc<Self> {
        Rc::new(Self {
            base: XamlAstNode::new(&**obj_node),
            types: ferro_types,
            document,
        })
    }
}

xaml_line_info_impl!(XamlSourceInfoValueManipulation, base);

impl IXamlAstNode for XamlSourceInfoValueManipulation {
    xaml_ast_node_members!("XamlSourceInfoValueManipulation", manipulation);
}

impl IXamlAstManipulationNode for XamlSourceInfoValueManipulation {}
