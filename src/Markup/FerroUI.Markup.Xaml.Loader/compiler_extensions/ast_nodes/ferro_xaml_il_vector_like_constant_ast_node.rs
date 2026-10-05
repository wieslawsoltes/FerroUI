//! Port of `CompilerExtensions/AstNodes/FerroXamlIlVectorLikeConstantAstNode.cs`.

use std::rc::Rc;

use xamlx::ast::{
    IXamlAstNode, IXamlAstTypeReference, IXamlAstValueNode, IXamlLineInfo,
    XamlAstClrTypeReference, XamlAstNode,
};
use xamlx::exceptions::{XamlError, XamlResult};
use xamlx::type_system::{IXamlConstructor, IXamlType};
use xamlx::{xaml_ast_node_members, xaml_line_info_impl};

use crate::compiler_extensions::transformers::FerroXamlIlWellKnownTypes;

/// A value type built from a fixed number of doubles (`Thickness`, `Point`, `Vector`, `Size`,
/// `Matrix`, `CornerRadius`) that was parsed at compile time.
///
/// # What a back end has to do (upstream IL)
///
/// Stack effect: pushes one value typed [`IXamlAstValueNode::type_`].
///
/// 1. Push every element of `values`, in order, as a double (`ldc.r8`).
/// 2. Create the value with `constructor`, which takes exactly `values.len()` doubles
///    (`newobj constructor`).
pub struct FerroXamlIlVectorLikeConstantAstNode {
    base: XamlAstNode,
    constructor: Rc<dyn IXamlConstructor>,
    values: Vec<f64>,
    type_: Rc<dyn IXamlAstTypeReference>,
}

impl FerroXamlIlVectorLikeConstantAstNode {
    pub fn new(
        line_info: &dyn IXamlLineInfo,
        types: &FerroXamlIlWellKnownTypes,
        type_: Rc<dyn IXamlType>,
        constructor: Rc<dyn IXamlConstructor>,
        values: Vec<f64>,
    ) -> XamlResult<Rc<Self>> {
        let parameters = constructor.parameters();

        if parameters.len() != values.len() {
            return Err(XamlError::type_system_exception(format!(
                "Constructor that takes {} parameters is expected, got {} instead.",
                values.len(),
                parameters.len()
            )));
        }

        let element_type = &types.xaml_il_types.double;

        for parameter in &parameters {
            if !parameter.equals(&**element_type) {
                return Err(XamlError::type_system_exception(format!(
                    "Expected parameter of type {}, got {} instead.",
                    element_type.to_type_string(),
                    parameter.to_type_string()
                )));
            }
        }

        Ok(Rc::new(Self {
            base: XamlAstNode::new(line_info),
            constructor,
            values,
            type_: XamlAstClrTypeReference::new(line_info, type_, false),
        }))
    }

    /// `_constructor` (private upstream; exposed for the back ends).
    pub fn constructor(&self) -> &Rc<dyn IXamlConstructor> {
        &self.constructor
    }

    /// `_values`: the constructor arguments, in parameter order.
    pub fn values(&self) -> &[f64] {
        &self.values
    }
}

xaml_line_info_impl!(FerroXamlIlVectorLikeConstantAstNode, base);

impl IXamlAstNode for FerroXamlIlVectorLikeConstantAstNode {
    xaml_ast_node_members!("FerroXamlIlVectorLikeConstantAstNode", value);
}

impl IXamlAstValueNode for FerroXamlIlVectorLikeConstantAstNode {
    fn type_(&self) -> Rc<dyn IXamlAstTypeReference> {
        self.type_.clone()
    }
}
