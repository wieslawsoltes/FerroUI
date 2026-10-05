//! Port of `CompilerExtensions/AstNodes/FerroXamlIlGridLengthAstNode.cs`.

use std::rc::Rc;

use xamlx::ast::{
    IXamlAstNode, IXamlAstTypeReference, IXamlAstValueNode, IXamlLineInfo,
    XamlAstClrTypeReference, XamlAstNode,
};
use xamlx::{xaml_ast_node_members, xaml_line_info_impl};

use crate::compiler_extensions::transformers::FerroXamlIlWellKnownTypes;

/// The unit of a compile-time grid length, with the integer values of the framework's
/// `GridUnitType` enum.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum FerroXamlIlGridUnitType {
    /// The row or column is auto-sized to fit its content.
    Auto = 0,
    /// The row or column is sized in device independent pixels.
    Pixel = 1,
    /// The row or column is sized as a weighted proportion of available space.
    Star = 2,
}

/// The data of a `GridLength` parsed at compile time. The compiler does not link the controls
/// library, so the value is kept as its two constructor arguments.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FerroXamlIlGridLength {
    pub value: f64,
    pub grid_unit_type: FerroXamlIlGridUnitType,
}

/// A `GridLength` constant parsed at compile time.
///
/// # What a back end has to do (upstream IL)
///
/// Stack effect: pushes one value of type `types.grid_length`.
///
/// 1. Push the double `grid_length().value` (`ldc.r8`).
/// 2. Push the unit as its integer value, `grid_length().grid_unit_type as i32` (`ldc.i4`;
///    `Auto` = 0, `Pixel` = 1, `Star` = 2).
/// 3. Create the value with `types.grid_length_constructor_value_type`
///    (`new GridLength(double value, GridUnitType type)`).
pub struct FerroXamlIlGridLengthAstNode {
    base: XamlAstNode,
    types: Rc<FerroXamlIlWellKnownTypes>,
    grid_length: FerroXamlIlGridLength,
    type_: Rc<dyn IXamlAstTypeReference>,
}

impl FerroXamlIlGridLengthAstNode {
    pub fn new(
        line_info: &dyn IXamlLineInfo,
        types: &Rc<FerroXamlIlWellKnownTypes>,
        grid_length: FerroXamlIlGridLength,
    ) -> Rc<Self> {
        Rc::new(Self {
            base: XamlAstNode::new(line_info),
            types: types.clone(),
            grid_length,
            type_: XamlAstClrTypeReference::new(line_info, types.grid_length.clone(), false),
        })
    }

    /// `_types` (private upstream; exposed for the back ends).
    pub fn types(&self) -> &Rc<FerroXamlIlWellKnownTypes> {
        &self.types
    }

    /// `_gridLength`: the parsed value.
    pub fn grid_length(&self) -> FerroXamlIlGridLength {
        self.grid_length
    }
}

xaml_line_info_impl!(FerroXamlIlGridLengthAstNode, base);

impl IXamlAstNode for FerroXamlIlGridLengthAstNode {
    xaml_ast_node_members!("FerroXamlIlGridLengthAstNode", value);
}

impl IXamlAstValueNode for FerroXamlIlGridLengthAstNode {
    fn type_(&self) -> Rc<dyn IXamlAstTypeReference> {
        self.type_.clone()
    }
}
