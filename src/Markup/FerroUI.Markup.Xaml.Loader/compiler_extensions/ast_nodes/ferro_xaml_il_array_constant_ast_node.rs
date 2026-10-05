//! Port of `CompilerExtensions/AstNodes/FerroXamlIlArrayConstantAstNode.cs`.

use std::rc::Rc;

use xamlx::ast::{
    IXamlAstNode, IXamlAstTypeReference, IXamlAstValueNode, IXamlLineInfo,
    XamlAstClrTypeReference, XamlAstNode,
};
use xamlx::type_system::IXamlType;
use xamlx::{xaml_ast_node_members, xaml_line_info_impl};

/// A new array (`T[]`) filled with the given values. The node's type is the array type, or
/// `IList<T>` when the array is assigned to a list interface.
///
/// As upstream, the node does not visit its values (they are final when it is created).
///
/// # What a back end has to do (upstream IL)
///
/// Stack effect: pushes one value, the array, typed [`IXamlAstValueNode::type_`].
///
/// 1. Create an array of `element_type` with `values.len()` elements
///    (`ldc.i4 count; newarr elementType`).
/// 2. For each value, in order, with its index:
///    1. duplicate the array reference and push the index;
///    2. emit the value node converted to `element_type` (`context.Emit(value, codeGen,
///       elementType)`);
///    3. when the value node's CLR type is a value type: box it first if `element_type` is not
///       a value type (an `object[]` holding value types), then store with `stelem <value
///       type>`; otherwise store the reference (`stelem.ref`).
/// 3. Leave the array on the stack.
pub struct FerroXamlIlArrayConstantAstNode {
    base: XamlAstNode,
    element_type: Rc<dyn IXamlType>,
    values: Vec<Rc<dyn IXamlAstValueNode>>,
    type_: Rc<dyn IXamlAstTypeReference>,
}

impl FerroXamlIlArrayConstantAstNode {
    pub fn new(
        line_info: &dyn IXamlLineInfo,
        array_type: Rc<dyn IXamlType>,
        element_type: Rc<dyn IXamlType>,
        values: Vec<Rc<dyn IXamlAstValueNode>>,
    ) -> Rc<Self> {
        Rc::new(Self {
            base: XamlAstNode::new(line_info),
            element_type,
            values,
            type_: XamlAstClrTypeReference::new(line_info, array_type, false),
        })
    }

    /// `_elementType` (private upstream; exposed for the back ends).
    pub fn element_type(&self) -> &Rc<dyn IXamlType> {
        &self.element_type
    }

    /// `_values` (private upstream; exposed for the back ends).
    pub fn values(&self) -> &[Rc<dyn IXamlAstValueNode>] {
        &self.values
    }
}

xaml_line_info_impl!(FerroXamlIlArrayConstantAstNode, base);

impl IXamlAstNode for FerroXamlIlArrayConstantAstNode {
    xaml_ast_node_members!("FerroXamlIlArrayConstantAstNode", value);
}

impl IXamlAstValueNode for FerroXamlIlArrayConstantAstNode {
    fn type_(&self) -> Rc<dyn IXamlAstTypeReference> {
        self.type_.clone()
    }
}
