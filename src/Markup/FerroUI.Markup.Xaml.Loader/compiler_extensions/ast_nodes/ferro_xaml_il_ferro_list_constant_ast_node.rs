//! Port of `CompilerExtensions/AstNodes/FerroXamlIlFerroListConstantAstNode.cs`.

use std::rc::Rc;

use xamlx::ast::{
    IXamlAstNode, IXamlAstTypeReference, IXamlAstValueNode, IXamlLineInfo,
    XamlAstClrTypeReference, XamlAstNode,
};
use xamlx::exceptions::XamlResult;
use xamlx::type_system::{FindMethodMethodSignature, IXamlConstructor, IXamlMethod, IXamlType};
use xamlx::{xaml_ast_node_members, xaml_line_info_impl};

use crate::compiler_extensions::transformers::FerroXamlIlWellKnownTypes;

/// A new instance of a `FerroList<T>` (or a type deriving from it) filled with the given values.
///
/// As upstream, the node does not visit its values (they are final when it is created).
///
/// # What a back end has to do (upstream IL)
///
/// Stack effect: pushes one value, the list, typed [`IXamlAstValueNode::type_`].
///
/// 1. Create the list with its parameterless constructor (`newobj constructor`).
/// 2. Duplicate the list reference, push `values.len()` and call `list_set_capacity_method`
///    (`set_Capacity(int)`).
/// 3. For each value, in order: duplicate the list reference, emit the value node converted to
///    `element_type` (`context.Emit(value, codeGen, elementType)`) and call `list_add_method`
///    (`Add(T)`, returns nothing).
/// 4. Leave the list on the stack.
pub struct FerroXamlIlFerroListConstantAstNode {
    base: XamlAstNode,
    element_type: Rc<dyn IXamlType>,
    values: Vec<Rc<dyn IXamlAstValueNode>>,
    constructor: Rc<dyn IXamlConstructor>,
    list_add_method: Rc<dyn IXamlMethod>,
    list_set_capacity_method: Rc<dyn IXamlMethod>,
    type_: Rc<dyn IXamlAstTypeReference>,
}

impl FerroXamlIlFerroListConstantAstNode {
    pub fn new(
        line_info: &dyn IXamlLineInfo,
        types: &FerroXamlIlWellKnownTypes,
        list_type: Rc<dyn IXamlType>,
        element_type: Rc<dyn IXamlType>,
        values: Vec<Rc<dyn IXamlAstValueNode>>,
    ) -> XamlResult<Rc<Self>> {
        let constructor = list_type.get_constructor(None)?;
        let list_add_method = list_type.get_method_by_signature(&FindMethodMethodSignature::new(
            "Add",
            types.xaml_il_types.void.clone(),
            vec![element_type.clone()],
        ))?;
        let list_set_capacity_method =
            list_type.get_method_by_signature(&FindMethodMethodSignature::new(
                "set_Capacity",
                types.xaml_il_types.void.clone(),
                vec![types.int.clone()],
            ))?;

        Ok(Rc::new(Self {
            base: XamlAstNode::new(line_info),
            constructor,
            list_add_method,
            list_set_capacity_method,
            element_type,
            values,
            type_: XamlAstClrTypeReference::new(line_info, list_type, false),
        }))
    }

    /// `_elementType` (private upstream; exposed for the back ends).
    pub fn element_type(&self) -> &Rc<dyn IXamlType> {
        &self.element_type
    }

    /// `_values` (private upstream; exposed for the back ends).
    pub fn values(&self) -> &[Rc<dyn IXamlAstValueNode>] {
        &self.values
    }

    /// `_constructor`: the parameterless constructor of the list type.
    pub fn constructor(&self) -> &Rc<dyn IXamlConstructor> {
        &self.constructor
    }

    /// `_listAddMethod`: `void Add(T)`.
    pub fn list_add_method(&self) -> &Rc<dyn IXamlMethod> {
        &self.list_add_method
    }

    /// `_listSetCapacityMethod`: `void set_Capacity(int)`.
    pub fn list_set_capacity_method(&self) -> &Rc<dyn IXamlMethod> {
        &self.list_set_capacity_method
    }
}

xaml_line_info_impl!(FerroXamlIlFerroListConstantAstNode, base);

impl IXamlAstNode for FerroXamlIlFerroListConstantAstNode {
    xaml_ast_node_members!("FerroXamlIlFerroListConstantAstNode", value);
}

impl IXamlAstValueNode for FerroXamlIlFerroListConstantAstNode {
    fn type_(&self) -> Rc<dyn IXamlAstTypeReference> {
        self.type_.clone()
    }
}
