//! Port of `Ast/Clr.cs`.
//!
//! The IL-specific members (`IXamlILOptimizedEmitablePropertySetter`, the `Emit*` methods of
//! the setters, wrapped methods and deferred content nodes) are not ported; the data they
//! operate on is exposed through accessors so that a backend can implement them.

use std::any::Any;
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;

use crate::exceptions::{XamlError, XamlResult};
use crate::transform::{TransformerConfiguration, XamlTransformHelpers};
use crate::type_system::{
    xaml_types_sequence_equal, IXamlConstructor, IXamlCustomAttribute, IXamlMember, IXamlMethod,
    IXamlParameterInfo, IXamlProperty, IXamlType, XamlTypeKey,
};
use crate::{xaml_ast_node_members, xaml_line_info_impl};

use super::{
    visit_cell, visit_list, visit_optional_cell, IXamlAstManipulationNode, IXamlAstNode,
    IXamlAstNodeNeedsParentStack, IXamlAstPropertyReference, IXamlAstTypeReference,
    IXamlAstValueNode, IXamlAstVisitor, IXamlLineInfo, XamlAstNode,
};

pub struct XamlAstClrTypeReference {
    base: XamlAstNode,
    pub type_: Rc<dyn IXamlType>,
    is_markup_extension: bool,
}

impl XamlAstClrTypeReference {
    pub fn new(
        line_info: &dyn IXamlLineInfo,
        type_: Rc<dyn IXamlType>,
        is_markup_extension: bool,
    ) -> Rc<Self> {
        Rc::new(Self {
            base: XamlAstNode::new(line_info),
            type_,
            is_markup_extension,
        })
    }
}

xaml_line_info_impl!(XamlAstClrTypeReference, base);

impl IXamlAstNode for XamlAstClrTypeReference {
    xaml_ast_node_members!("XamlAstClrTypeReference", type_reference);

    fn to_node_string(&self) -> String {
        self.type_.get_fqn()
    }
}

impl IXamlAstTypeReference for XamlAstClrTypeReference {
    fn is_markup_extension(&self) -> bool {
        self.is_markup_extension
    }

    fn equals(&self, other: &dyn IXamlAstTypeReference) -> bool {
        match other.as_any().downcast_ref::<XamlAstClrTypeReference>() {
            Some(clr) => {
                clr.type_.equals(&*self.type_)
                    && clr.is_markup_extension == self.is_markup_extension
            }
            None => false,
        }
    }
}

pub struct XamlAstClrProperty {
    base: XamlAstNode,
    pub name: RefCell<String>,
    pub is_public: Cell<bool>,
    pub is_private: Cell<bool>,
    pub is_family: Cell<bool>,
    pub getter: RefCell<Option<Rc<dyn IXamlMethod>>>,
    pub setters: RefCell<Vec<Rc<dyn IXamlPropertySetter>>>,
    pub custom_attributes: RefCell<Vec<Rc<dyn IXamlCustomAttribute>>>,
    pub declaring_type: RefCell<Rc<dyn IXamlType>>,
    pub type_converters: RefCell<HashMap<XamlTypeKey, Rc<dyn IXamlType>>>,
    /// The state of a host class derived from `XamlAstClrProperty` (upstream hosts subclass the
    /// property node to attach data to it). `None` for a plain property.
    pub extension: RefCell<Option<Rc<dyn IXamlAstClrPropertyExtension>>>,
}

/// The state (and interfaces) a host "subclass" of [`XamlAstClrProperty`] adds to the node.
///
/// `property is TDerived` becomes [`XamlAstClrProperty::extension_as`], and
/// `property is IInterface` becomes
/// [`crate::extensions::query_clr_property_extension_interface`].
pub trait IXamlAstClrPropertyExtension: 'static {
    fn as_any(&self) -> &dyn Any;
    fn into_any_rc(self: Rc<Self>) -> Rc<dyn Any>;
    /// `GetType().Name` of the derived class.
    fn type_name(&self) -> &'static str;
    /// Dynamic interface query for the interfaces the derived class implements.
    fn query_interface(self: Rc<Self>, slot: &mut dyn Any) -> bool {
        let _ = slot;
        false
    }
}

impl XamlAstClrProperty {
    /// The state of the derived class, if this node is an instance of one.
    pub fn extension(&self) -> Option<Rc<dyn IXamlAstClrPropertyExtension>> {
        self.extension.borrow().clone()
    }

    /// `this as TDerived`: the derived class state when it is a `T`.
    pub fn extension_as<T: IXamlAstClrPropertyExtension>(&self) -> Option<Rc<T>> {
        self.extension()
            .and_then(|e| e.into_any_rc().downcast::<T>().ok())
    }

    /// Turns this node into an instance of a derived class.
    pub fn set_extension(&self, extension: Rc<dyn IXamlAstClrPropertyExtension>) {
        *self.extension.borrow_mut() = Some(extension);
    }
}

impl XamlAstClrProperty {
    /// `XamlAstClrProperty(IXamlLineInfo, IXamlProperty, TransformerConfiguration)`.
    pub fn from_property(
        line_info: &dyn IXamlLineInfo,
        property: &Rc<dyn IXamlProperty>,
        cfg: &TransformerConfiguration,
    ) -> XamlResult<Rc<Self>> {
        let getter = property.getter();
        let mut setters: Vec<Rc<dyn IXamlPropertySetter>> = Vec::new();
        if let Some(setter) = property.setter() {
            setters.push(XamlDirectCallPropertySetter::new(setter)?);
        }
        let accessor = property.getter().or_else(|| property.setter());
        let mut type_converters = HashMap::new();
        let type_converter_attributes = cfg.get_custom_attributes_for_property(
            &**property,
            &cfg.type_mappings.type_converter_attributes,
        );
        for attr in type_converter_attributes {
            if let Some(type_converter) =
                XamlTransformHelpers::try_get_type_converter_from_custom_attribute(cfg, Some(&attr))
            {
                type_converters.insert(XamlTypeKey(property.property_type()), type_converter);
                break;
            }
        }
        Ok(Rc::new(Self {
            base: XamlAstNode::new(line_info),
            name: RefCell::new(property.name()),
            is_public: Cell::new(accessor.as_ref().is_some_and(|a| a.is_public())),
            is_private: Cell::new(accessor.as_ref().is_some_and(|a| a.is_private())),
            is_family: Cell::new(accessor.as_ref().is_some_and(|a| a.is_family())),
            getter: RefCell::new(getter),
            setters: RefCell::new(setters),
            custom_attributes: RefCell::new(property.custom_attributes()),
            declaring_type: RefCell::new(property.declaring_type()),
            type_converters: RefCell::new(type_converters),
            extension: RefCell::new(None),
        }))
    }

    /// `XamlAstClrProperty(lineInfo, name, declaringType, getter, IEnumerable<IXamlPropertySetter>?, customAttributes)`.
    pub fn with_setters(
        line_info: &dyn IXamlLineInfo,
        name: &str,
        declaring_type: Rc<dyn IXamlType>,
        getter: Option<Rc<dyn IXamlMethod>>,
        setters: Option<Vec<Rc<dyn IXamlPropertySetter>>>,
        custom_attributes: Option<Vec<Rc<dyn IXamlCustomAttribute>>>,
    ) -> Rc<Self> {
        Rc::new(Self {
            base: XamlAstNode::new(line_info),
            name: RefCell::new(name.to_string()),
            is_public: Cell::new(getter.as_ref().is_some_and(|g| g.is_public())),
            is_private: Cell::new(getter.as_ref().is_some_and(|g| g.is_private())),
            is_family: Cell::new(getter.as_ref().is_some_and(|g| g.is_family())),
            getter: RefCell::new(getter),
            setters: RefCell::new(setters.unwrap_or_default()),
            custom_attributes: RefCell::new(custom_attributes.unwrap_or_default()),
            declaring_type: RefCell::new(declaring_type),
            type_converters: RefCell::new(HashMap::new()),
            extension: RefCell::new(None),
        })
    }

    /// `XamlAstClrProperty(lineInfo, name, declaringType, getter, IEnumerable<IXamlMethod?>?, customAttributes)`.
    pub fn with_setter_methods(
        line_info: &dyn IXamlLineInfo,
        name: &str,
        declaring_type: Rc<dyn IXamlType>,
        getter: Option<Rc<dyn IXamlMethod>>,
        setters: Option<Vec<Option<Rc<dyn IXamlMethod>>>>,
        custom_attributes: Option<Vec<Rc<dyn IXamlCustomAttribute>>>,
    ) -> XamlResult<Rc<Self>> {
        let setters = match setters {
            Some(methods) => {
                let mut list: Vec<Rc<dyn IXamlPropertySetter>> = Vec::new();
                for method in methods.into_iter().flatten() {
                    list.push(XamlDirectCallPropertySetter::new(method)?);
                }
                Some(list)
            }
            None => None,
        };
        Ok(Self::with_setters(
            line_info,
            name,
            declaring_type,
            getter,
            setters,
            custom_attributes,
        ))
    }

    /// `XamlAstClrProperty(lineInfo, name, declaringType, getter)`.
    pub fn new(
        line_info: &dyn IXamlLineInfo,
        name: &str,
        declaring_type: Rc<dyn IXamlType>,
        getter: Option<Rc<dyn IXamlMethod>>,
    ) -> Rc<Self> {
        Self::with_setters(line_info, name, declaring_type, getter, None, None)
    }

    pub fn name(&self) -> String {
        self.name.borrow().clone()
    }

    pub fn getter(&self) -> Option<Rc<dyn IXamlMethod>> {
        self.getter.borrow().clone()
    }

    pub fn declaring_type(&self) -> Rc<dyn IXamlType> {
        self.declaring_type.borrow().clone()
    }

    pub fn setters(&self) -> Vec<Rc<dyn IXamlPropertySetter>> {
        self.setters.borrow().clone()
    }

    pub fn custom_attributes(&self) -> Vec<Rc<dyn IXamlCustomAttribute>> {
        self.custom_attributes.borrow().clone()
    }
}

xaml_line_info_impl!(XamlAstClrProperty, base);

impl IXamlAstNode for XamlAstClrProperty {
    xaml_ast_node_members!("XamlAstClrProperty", property_reference);

    fn to_node_string(&self) -> String {
        format!(
            "{}.{}",
            self.declaring_type.borrow().get_fqn(),
            self.name.borrow()
        )
    }
}

impl IXamlAstPropertyReference for XamlAstClrProperty {}

pub struct PropertySetterBinderParameters {
    pub allow_multiple: Cell<bool>,
    pub allow_x_null: Cell<bool>,
    pub allow_runtime_null: Cell<bool>,
    pub allow_attribute_syntax: Cell<bool>,
}

impl Default for PropertySetterBinderParameters {
    fn default() -> Self {
        Self {
            allow_multiple: Cell::new(false),
            allow_x_null: Cell::new(true),
            allow_runtime_null: Cell::new(true),
            allow_attribute_syntax: Cell::new(true),
        }
    }
}

impl PropertySetterBinderParameters {
    /// `IEquatable<PropertySetterBinderParameters>.Equals` (as upstream, `AllowAttributeSyntax`
    /// does not take part in the comparison).
    pub fn equals(&self, other: &PropertySetterBinderParameters) -> bool {
        self.allow_multiple.get() == other.allow_multiple.get()
            && self.allow_x_null.get() == other.allow_x_null.get()
            && self.allow_runtime_null.get() == other.allow_runtime_null.get()
    }
}

pub trait IXamlPropertySetter: 'static {
    fn target_type(&self) -> Rc<dyn IXamlType>;
    fn binder_parameters(&self) -> &PropertySetterBinderParameters;
    fn parameters(&self) -> Vec<Rc<dyn IXamlType>>;
    fn custom_attributes(&self) -> Vec<Rc<dyn IXamlCustomAttribute>>;
    fn as_any(&self) -> &dyn Any;
    /// `GetType()` name, used in diagnostics.
    fn type_name(&self) -> &'static str;
    /// `object.Equals`; reference equality unless overridden.
    fn equals(&self, other: &dyn IXamlPropertySetter) -> bool {
        std::ptr::addr_eq(
            self.as_any() as *const dyn Any,
            other.as_any() as *const dyn Any,
        )
    }
    /// Dynamic interface query, e.g. for a backend's `IXamlEmitablePropertySetter<TBackendEmitter>`.
    fn query_interface(self: Rc<Self>, slot: &mut dyn Any) -> bool {
        let _ = slot;
        false
    }
}

/// A setter that calls a method directly: `target.Method(args...)`.
pub struct XamlDirectCallPropertySetter {
    method: Rc<dyn IXamlMethod>,
    target_type: Rc<dyn IXamlType>,
    binder_parameters: PropertySetterBinderParameters,
    parameters: Vec<Rc<dyn IXamlType>>,
}

impl XamlDirectCallPropertySetter {
    pub fn new(method: Rc<dyn IXamlMethod>) -> XamlResult<Rc<Self>> {
        let parameters: Vec<Rc<dyn IXamlType>> =
            method.parameters_with_this().into_iter().skip(1).collect();
        let target_type = method.this_or_first_parameter()?;
        let last = parameters.last().ok_or_else(|| {
            XamlError::internal(
                "ArgumentOutOfRangeException",
                "Property setter method doesn't have a value parameter",
            )
        })?;
        let allow_null = last.accepts_null();
        Ok(Rc::new(Self {
            method,
            target_type,
            binder_parameters: PropertySetterBinderParameters {
                allow_multiple: Cell::new(false),
                allow_x_null: Cell::new(allow_null),
                allow_runtime_null: Cell::new(allow_null),
                allow_attribute_syntax: Cell::new(true),
            },
            parameters,
        }))
    }

    /// The method called by this setter (private upstream; exposed for emitter backends).
    pub fn method(&self) -> &Rc<dyn IXamlMethod> {
        &self.method
    }
}

impl IXamlPropertySetter for XamlDirectCallPropertySetter {
    fn target_type(&self) -> Rc<dyn IXamlType> {
        self.target_type.clone()
    }
    fn binder_parameters(&self) -> &PropertySetterBinderParameters {
        &self.binder_parameters
    }
    fn parameters(&self) -> Vec<Rc<dyn IXamlType>> {
        self.parameters.clone()
    }
    fn custom_attributes(&self) -> Vec<Rc<dyn IXamlCustomAttribute>> {
        self.method.custom_attributes()
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn type_name(&self) -> &'static str {
        "XamlDirectCallPropertySetter"
    }
    fn equals(&self, other: &dyn IXamlPropertySetter) -> bool {
        match other
            .as_any()
            .downcast_ref::<XamlDirectCallPropertySetter>()
        {
            Some(other) => {
                self.method.equals(&*other.method)
                    && self.binder_parameters.equals(&other.binder_parameters)
            }
            None => false,
        }
    }
}

pub struct XamlPropertyAssignmentNode {
    base: XamlAstNode,
    pub property: Rc<XamlAstClrProperty>,
    pub possible_setters: RefCell<Vec<Rc<dyn IXamlPropertySetter>>>,
    pub values: RefCell<Vec<Rc<dyn IXamlAstValueNode>>>,
}

impl XamlPropertyAssignmentNode {
    pub fn new(
        line_info: &dyn IXamlLineInfo,
        property: Rc<XamlAstClrProperty>,
        setters: Vec<Rc<dyn IXamlPropertySetter>>,
        values: Vec<Rc<dyn IXamlAstValueNode>>,
    ) -> Rc<Self> {
        Rc::new(Self {
            base: XamlAstNode::new(line_info),
            property,
            possible_setters: RefCell::new(setters),
            values: RefCell::new(values),
        })
    }
}

xaml_line_info_impl!(XamlPropertyAssignmentNode, base);

impl IXamlAstNode for XamlPropertyAssignmentNode {
    xaml_ast_node_members!("XamlPropertyAssignmentNode", manipulation);

    fn visit_children(&self, visitor: &mut dyn IXamlAstVisitor) -> XamlResult<()> {
        visit_list(&self.values, visitor)
    }
}

impl IXamlAstManipulationNode for XamlPropertyAssignmentNode {}

pub struct XamlPropertyValueManipulationNode {
    base: XamlAstNode,
    pub property: RefCell<Rc<XamlAstClrProperty>>,
    pub manipulation: RefCell<Rc<dyn IXamlAstManipulationNode>>,
}

impl XamlPropertyValueManipulationNode {
    pub fn new(
        line_info: &dyn IXamlLineInfo,
        property: Rc<XamlAstClrProperty>,
        manipulation: Rc<dyn IXamlAstManipulationNode>,
    ) -> Rc<Self> {
        Rc::new(Self {
            base: XamlAstNode::new(line_info),
            property: RefCell::new(property),
            manipulation: RefCell::new(manipulation),
        })
    }
}

xaml_line_info_impl!(XamlPropertyValueManipulationNode, base);

impl IXamlAstNode for XamlPropertyValueManipulationNode {
    xaml_ast_node_members!("XamlPropertyValueManipulationNode", manipulation);

    fn visit_children(&self, visitor: &mut dyn IXamlAstVisitor) -> XamlResult<()> {
        visit_cell(&self.manipulation, visitor)
    }
}

impl IXamlAstManipulationNode for XamlPropertyValueManipulationNode {}

/// The state of the upstream abstract `XamlMethodCallBaseNode`.
pub struct XamlMethodCallBaseNode {
    base: XamlAstNode,
    pub method: RefCell<Rc<dyn IXamlWrappedMethod>>,
    pub arguments: RefCell<Vec<Rc<dyn IXamlAstValueNode>>>,
}

impl XamlMethodCallBaseNode {
    pub fn new(
        line_info: &dyn IXamlLineInfo,
        method: Rc<dyn IXamlWrappedMethod>,
        args: Option<Vec<Rc<dyn IXamlAstValueNode>>>,
    ) -> Self {
        Self {
            base: XamlAstNode::new(line_info),
            method: RefCell::new(method),
            arguments: RefCell::new(args.unwrap_or_default()),
        }
    }

    pub fn method(&self) -> Rc<dyn IXamlWrappedMethod> {
        self.method.borrow().clone()
    }

    fn visit_children(&self, visitor: &mut dyn IXamlAstVisitor) -> XamlResult<()> {
        visit_list(&self.arguments, visitor)
    }
}

xaml_line_info_impl!(XamlMethodCallBaseNode, base);

pub struct XamlNoReturnMethodCallNode {
    pub base: XamlMethodCallBaseNode,
}

impl XamlNoReturnMethodCallNode {
    pub fn new(
        line_info: &dyn IXamlLineInfo,
        method: Rc<dyn IXamlMethod>,
        args: Vec<Rc<dyn IXamlAstValueNode>>,
    ) -> Rc<Self> {
        Self::with_wrapped_method(line_info, XamlWrappedMethod::new(method), args)
    }

    pub fn with_wrapped_method(
        line_info: &dyn IXamlLineInfo,
        method: Rc<dyn IXamlWrappedMethod>,
        args: Vec<Rc<dyn IXamlAstValueNode>>,
    ) -> Rc<Self> {
        Rc::new(Self {
            base: XamlMethodCallBaseNode::new(line_info, method, Some(args)),
        })
    }
}

xaml_line_info_impl!(XamlNoReturnMethodCallNode, base);

impl IXamlAstNode for XamlNoReturnMethodCallNode {
    xaml_ast_node_members!("XamlNoReturnMethodCallNode", manipulation);

    fn visit_children(&self, visitor: &mut dyn IXamlAstVisitor) -> XamlResult<()> {
        self.base.visit_children(visitor)
    }

    fn as_method_call_base_node(&self) -> Option<&XamlMethodCallBaseNode> {
        Some(&self.base)
    }
}

impl IXamlAstManipulationNode for XamlNoReturnMethodCallNode {}

pub struct XamlStaticOrTargetedReturnMethodCallNode {
    pub base: XamlMethodCallBaseNode,
    type_: Rc<dyn IXamlAstTypeReference>,
}

impl XamlStaticOrTargetedReturnMethodCallNode {
    pub fn with_wrapped_method(
        line_info: &dyn IXamlLineInfo,
        method: Rc<dyn IXamlWrappedMethod>,
        args: Option<Vec<Rc<dyn IXamlAstValueNode>>>,
    ) -> Rc<Self> {
        let type_ = XamlAstClrTypeReference::new(line_info, method.return_type(), false);
        Rc::new(Self {
            base: XamlMethodCallBaseNode::new(line_info, method, args),
            type_,
        })
    }

    pub fn new(
        line_info: &dyn IXamlLineInfo,
        method: Rc<dyn IXamlMethod>,
        args: Option<Vec<Rc<dyn IXamlAstValueNode>>>,
    ) -> Rc<Self> {
        Self::with_wrapped_method(line_info, XamlWrappedMethod::new(method), args)
    }
}

xaml_line_info_impl!(XamlStaticOrTargetedReturnMethodCallNode, base);

impl IXamlAstNode for XamlStaticOrTargetedReturnMethodCallNode {
    xaml_ast_node_members!("XamlStaticOrTargetedReturnMethodCallNode", value);

    fn visit_children(&self, visitor: &mut dyn IXamlAstVisitor) -> XamlResult<()> {
        self.base.visit_children(visitor)
    }

    fn as_method_call_base_node(&self) -> Option<&XamlMethodCallBaseNode> {
        Some(&self.base)
    }
}

impl IXamlAstValueNode for XamlStaticOrTargetedReturnMethodCallNode {
    fn type_(&self) -> Rc<dyn IXamlAstTypeReference> {
        self.type_.clone()
    }
}

pub struct XamlManipulationGroupNode {
    base: XamlAstNode,
    pub children: RefCell<Vec<Rc<dyn IXamlAstManipulationNode>>>,
}

impl XamlManipulationGroupNode {
    pub fn new(
        line_info: &dyn IXamlLineInfo,
        children: Option<Vec<Rc<dyn IXamlAstManipulationNode>>>,
    ) -> Rc<Self> {
        Rc::new(Self {
            base: XamlAstNode::new(line_info),
            children: RefCell::new(children.unwrap_or_default()),
        })
    }
}

xaml_line_info_impl!(XamlManipulationGroupNode, base);

impl IXamlAstNode for XamlManipulationGroupNode {
    xaml_ast_node_members!("XamlManipulationGroupNode", manipulation);

    fn visit_children(&self, visitor: &mut dyn IXamlAstVisitor) -> XamlResult<()> {
        visit_list(&self.children, visitor)
    }
}

impl IXamlAstManipulationNode for XamlManipulationGroupNode {}

/// The state of the upstream abstract `XamlValueWithSideEffectNodeBase`.
pub struct XamlValueWithSideEffectNodeBase {
    base: XamlAstNode,
    pub value: RefCell<Rc<dyn IXamlAstValueNode>>,
}

impl XamlValueWithSideEffectNodeBase {
    pub fn new(line_info: &dyn IXamlLineInfo, value: Rc<dyn IXamlAstValueNode>) -> Self {
        Self {
            base: XamlAstNode::new(line_info),
            value: RefCell::new(value),
        }
    }

    pub fn value(&self) -> Rc<dyn IXamlAstValueNode> {
        self.value.borrow().clone()
    }

    /// The non-overridden `Type` property: `Value.Type`.
    pub fn type_(&self) -> Rc<dyn IXamlAstTypeReference> {
        self.value.borrow().type_()
    }

    /// The base `VisitChildren`.
    pub fn visit_children(&self, visitor: &mut dyn IXamlAstVisitor) -> XamlResult<()> {
        visit_cell(&self.value, visitor)
    }
}

xaml_line_info_impl!(XamlValueWithSideEffectNodeBase, base);

pub struct XamlValueWithManipulationNode {
    pub base: XamlValueWithSideEffectNodeBase,
    pub manipulation: RefCell<Option<Rc<dyn IXamlAstManipulationNode>>>,
}

impl XamlValueWithManipulationNode {
    pub fn new(
        line_info: &dyn IXamlLineInfo,
        value: Rc<dyn IXamlAstValueNode>,
        manipulation: Option<Rc<dyn IXamlAstManipulationNode>>,
    ) -> Rc<Self> {
        Rc::new(Self::new_inline(line_info, value, manipulation))
    }

    /// Builds the node state without boxing it; used by "subclasses" that embed it.
    pub fn new_inline(
        line_info: &dyn IXamlLineInfo,
        value: Rc<dyn IXamlAstValueNode>,
        manipulation: Option<Rc<dyn IXamlAstManipulationNode>>,
    ) -> Self {
        Self {
            base: XamlValueWithSideEffectNodeBase::new(line_info, value),
            manipulation: RefCell::new(manipulation),
        }
    }

    pub fn value(&self) -> Rc<dyn IXamlAstValueNode> {
        self.base.value()
    }

    pub fn manipulation(&self) -> Option<Rc<dyn IXamlAstManipulationNode>> {
        self.manipulation.borrow().clone()
    }
}

xaml_line_info_impl!(XamlValueWithManipulationNode, base);

impl IXamlAstNode for XamlValueWithManipulationNode {
    xaml_ast_node_members!("XamlValueWithManipulationNode", value);

    fn visit_children(&self, visitor: &mut dyn IXamlAstVisitor) -> XamlResult<()> {
        self.base.visit_children(visitor)?;
        visit_optional_cell(&self.manipulation, visitor)
    }

    fn as_value_with_side_effect_node_base(&self) -> Option<&XamlValueWithSideEffectNodeBase> {
        Some(&self.base)
    }

    fn as_value_with_manipulation_node(&self) -> Option<&XamlValueWithManipulationNode> {
        Some(self)
    }
}

impl IXamlAstValueNode for XamlValueWithManipulationNode {
    fn type_(&self) -> Rc<dyn IXamlAstTypeReference> {
        self.base.type_()
    }
}

pub struct XamlAstNewClrObjectNode {
    base: XamlAstNode,
    pub type_: RefCell<Rc<dyn IXamlAstTypeReference>>,
    pub constructor: Rc<dyn IXamlConstructor>,
    pub arguments: RefCell<Vec<Rc<dyn IXamlAstValueNode>>>,
}

impl XamlAstNewClrObjectNode {
    pub fn new(
        line_info: &dyn IXamlLineInfo,
        type_: Rc<XamlAstClrTypeReference>,
        ctor: Rc<dyn IXamlConstructor>,
        arguments: Vec<Rc<dyn IXamlAstValueNode>>,
    ) -> Rc<Self> {
        Rc::new(Self {
            base: XamlAstNode::new(line_info),
            type_: RefCell::new(type_),
            constructor: ctor,
            arguments: RefCell::new(arguments),
        })
    }
}

xaml_line_info_impl!(XamlAstNewClrObjectNode, base);

impl IXamlAstNode for XamlAstNewClrObjectNode {
    xaml_ast_node_members!("XamlAstNewClrObjectNode", value);

    fn visit_children(&self, visitor: &mut dyn IXamlAstVisitor) -> XamlResult<()> {
        visit_cell(&self.type_, visitor)?;
        visit_list(&self.arguments, visitor)
    }
}

impl IXamlAstValueNode for XamlAstNewClrObjectNode {
    fn type_(&self) -> Rc<dyn IXamlAstTypeReference> {
        self.type_.borrow().clone()
    }
}

pub struct XamlAstConstructableObjectNode {
    base: XamlAstNode,
    pub type_: RefCell<Rc<dyn IXamlAstTypeReference>>,
    pub constructor: Rc<dyn IXamlConstructor>,
    pub arguments: RefCell<Vec<Rc<dyn IXamlAstValueNode>>>,
    pub children: RefCell<Vec<Rc<dyn IXamlAstNode>>>,
}

impl XamlAstConstructableObjectNode {
    pub fn new(
        line_info: &dyn IXamlLineInfo,
        type_: Rc<XamlAstClrTypeReference>,
        ctor: Rc<dyn IXamlConstructor>,
        arguments: Vec<Rc<dyn IXamlAstValueNode>>,
        children: Vec<Rc<dyn IXamlAstNode>>,
    ) -> Rc<Self> {
        Rc::new(Self {
            base: XamlAstNode::new(line_info),
            type_: RefCell::new(type_),
            constructor: ctor,
            arguments: RefCell::new(arguments),
            children: RefCell::new(children),
        })
    }
}

xaml_line_info_impl!(XamlAstConstructableObjectNode, base);

impl IXamlAstNode for XamlAstConstructableObjectNode {
    xaml_ast_node_members!("XamlAstConstructableObjectNode", value);

    fn visit_children(&self, visitor: &mut dyn IXamlAstVisitor) -> XamlResult<()> {
        visit_cell(&self.type_, visitor)?;
        visit_list(&self.arguments, visitor)?;
        visit_list(&self.children, visitor)
    }
}

impl IXamlAstValueNode for XamlAstConstructableObjectNode {
    fn type_(&self) -> Rc<dyn IXamlAstTypeReference> {
        self.type_.borrow().clone()
    }
}

pub struct XamlMarkupExtensionNode {
    base: XamlAstNode,
    pub value: RefCell<Rc<dyn IXamlAstValueNode>>,
    pub provide_value: Rc<dyn IXamlMethod>,
    type_: Rc<dyn IXamlAstTypeReference>,
    /// The state of a host class derived from `XamlMarkupExtensionNode` (upstream hosts subclass
    /// the node to override `Type` and `VisitChildren`). `None` for a plain node.
    pub extension: RefCell<Option<Rc<dyn IXamlMarkupExtensionNodeExtension>>>,
}

/// The state (and overrides) a host "subclass" of [`XamlMarkupExtensionNode`] adds to the node.
///
/// `node is TDerived` becomes [`XamlMarkupExtensionNode::extension_as`], and
/// `node is IInterface` is answered by the node's `query_interface`, which forwards to
/// [`IXamlMarkupExtensionNodeExtension::query_interface`].
pub trait IXamlMarkupExtensionNodeExtension: 'static {
    fn as_any(&self) -> &dyn Any;
    fn into_any_rc(self: Rc<Self>) -> Rc<dyn Any>;
    /// `GetType().Name` of the derived class.
    fn type_name(&self) -> &'static str;
    /// The derived class's override of `IXamlAstValueNode.Type`; `None` keeps the base value.
    fn type_(&self, node: &XamlMarkupExtensionNode) -> Option<Rc<dyn IXamlAstTypeReference>> {
        let _ = node;
        None
    }
    /// The part of the derived class's `VisitChildren` override that runs before the base
    /// implementation.
    fn visit_children(&self, visitor: &mut dyn IXamlAstVisitor) -> XamlResult<()> {
        let _ = visitor;
        Ok(())
    }
    /// Dynamic interface query for the interfaces the derived class implements.
    fn query_interface(self: Rc<Self>, slot: &mut dyn Any) -> bool {
        let _ = slot;
        false
    }
}

impl XamlMarkupExtensionNode {
    pub fn new(
        line_info: &dyn IXamlLineInfo,
        provide_value: Rc<dyn IXamlMethod>,
        value: Rc<dyn IXamlAstValueNode>,
    ) -> Rc<Self> {
        let type_ = XamlAstClrTypeReference::new(line_info, provide_value.return_type(), false);
        Rc::new(Self {
            base: XamlAstNode::new(line_info),
            value: RefCell::new(value),
            provide_value,
            type_,
            extension: RefCell::new(None),
        })
    }

    pub fn value(&self) -> Rc<dyn IXamlAstValueNode> {
        self.value.borrow().clone()
    }

    /// The state of the derived class, if this node is an instance of one.
    pub fn extension(&self) -> Option<Rc<dyn IXamlMarkupExtensionNodeExtension>> {
        self.extension.borrow().clone()
    }

    /// `this as TDerived`: the derived class state when it is a `T`.
    pub fn extension_as<T: IXamlMarkupExtensionNodeExtension>(&self) -> Option<Rc<T>> {
        self.extension()
            .and_then(|e| e.into_any_rc().downcast::<T>().ok())
    }

    /// Turns this node into an instance of a derived class.
    pub fn set_extension(&self, extension: Rc<dyn IXamlMarkupExtensionNodeExtension>) {
        *self.extension.borrow_mut() = Some(extension);
    }
}

xaml_line_info_impl!(XamlMarkupExtensionNode, base);

impl IXamlAstNode for XamlMarkupExtensionNode {
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn into_any_rc(self: Rc<Self>) -> Rc<dyn Any> {
        self
    }
    fn type_name(&self) -> &'static str {
        match &*self.extension.borrow() {
            Some(extension) => extension.type_name(),
            None => "XamlMarkupExtensionNode",
        }
    }
    fn into_value_node(self: Rc<Self>) -> Option<Rc<dyn IXamlAstValueNode>> {
        Some(self)
    }
    fn as_needs_parent_stack(&self) -> Option<&dyn IXamlAstNodeNeedsParentStack> {
        Some(self)
    }

    fn visit_children(&self, visitor: &mut dyn IXamlAstVisitor) -> XamlResult<()> {
        let extension = self.extension();
        if let Some(extension) = extension {
            extension.visit_children(visitor)?;
        }
        visit_cell(&self.value, visitor)
    }

    fn query_interface(self: Rc<Self>, slot: &mut dyn Any) -> bool {
        match self.extension() {
            Some(extension) => extension.query_interface(slot),
            None => false,
        }
    }
}

impl IXamlAstValueNode for XamlMarkupExtensionNode {
    fn type_(&self) -> Rc<dyn IXamlAstTypeReference> {
        let extension = self.extension();
        match extension.and_then(|e| e.type_(self)) {
            Some(type_) => type_,
            None => self.type_.clone(),
        }
    }
}

impl IXamlAstNodeNeedsParentStack for XamlMarkupExtensionNode {
    fn needs_parent_stack(&self) -> bool {
        !self.provide_value.parameters().is_empty()
    }
}

pub struct XamlObjectInitializationNode {
    base: XamlAstNode,
    pub manipulation: RefCell<Rc<dyn IXamlAstManipulationNode>>,
    pub type_: RefCell<Rc<dyn IXamlType>>,
    pub skip_begin_init: Cell<bool>,
}

impl XamlObjectInitializationNode {
    pub fn new(
        line_info: &dyn IXamlLineInfo,
        manipulation: Rc<dyn IXamlAstManipulationNode>,
        type_: Rc<dyn IXamlType>,
    ) -> Rc<Self> {
        Rc::new(Self {
            base: XamlAstNode::new(line_info),
            manipulation: RefCell::new(manipulation),
            type_: RefCell::new(type_),
            skip_begin_init: Cell::new(false),
        })
    }

    pub fn manipulation(&self) -> Rc<dyn IXamlAstManipulationNode> {
        self.manipulation.borrow().clone()
    }
}

xaml_line_info_impl!(XamlObjectInitializationNode, base);

impl IXamlAstNode for XamlObjectInitializationNode {
    xaml_ast_node_members!("XamlObjectInitializationNode", manipulation);

    fn visit_children(&self, visitor: &mut dyn IXamlAstVisitor) -> XamlResult<()> {
        visit_cell(&self.manipulation, visitor)
    }
}

impl IXamlAstManipulationNode for XamlObjectInitializationNode {}

pub struct XamlToArrayNode {
    base: XamlAstNode,
    pub value: RefCell<Rc<dyn IXamlAstValueNode>>,
    type_: Rc<dyn IXamlAstTypeReference>,
}

impl XamlToArrayNode {
    pub fn new(
        line_info: &dyn IXamlLineInfo,
        array_type: Rc<dyn IXamlAstTypeReference>,
        value: Rc<dyn IXamlAstValueNode>,
    ) -> Rc<Self> {
        Rc::new(Self {
            base: XamlAstNode::new(line_info),
            value: RefCell::new(value),
            type_: array_type,
        })
    }
}

xaml_line_info_impl!(XamlToArrayNode, base);

impl IXamlAstNode for XamlToArrayNode {
    // As upstream, this node does not visit its value.
    xaml_ast_node_members!("XamlToArrayNode", value);
}

impl IXamlAstValueNode for XamlToArrayNode {
    fn type_(&self) -> Rc<dyn IXamlAstTypeReference> {
        self.type_.clone()
    }
}

pub trait IXamlWrappedMethod: 'static {
    fn name(&self) -> String;
    fn return_type(&self) -> Rc<dyn IXamlType>;
    fn declaring_type(&self) -> Rc<dyn IXamlType>;
    fn parameters_with_this(&self) -> Vec<Rc<dyn IXamlType>>;
    fn as_any(&self) -> &dyn Any;
    /// `GetType()` name, used in diagnostics.
    fn type_name(&self) -> &'static str;
    /// Dynamic interface query, e.g. for a backend's `IXamlEmitableWrappedMethod<..>`.
    fn query_interface(self: Rc<Self>, slot: &mut dyn Any) -> bool {
        let _ = slot;
        false
    }
}

pub struct XamlWrappedMethod {
    method: Rc<dyn IXamlMethod>,
    return_type: Rc<dyn IXamlType>,
    parameters_with_this: Vec<Rc<dyn IXamlType>>,
}

impl XamlWrappedMethod {
    pub fn new(method: Rc<dyn IXamlMethod>) -> Rc<Self> {
        let parameters_with_this = method.parameters_with_this();
        let return_type = method.return_type();
        Rc::new(Self {
            method,
            return_type,
            parameters_with_this,
        })
    }

    /// The wrapped method (private upstream; exposed for emitter backends).
    pub fn method(&self) -> &Rc<dyn IXamlMethod> {
        &self.method
    }
}

impl IXamlWrappedMethod for XamlWrappedMethod {
    fn name(&self) -> String {
        self.method.name()
    }
    fn return_type(&self) -> Rc<dyn IXamlType> {
        self.return_type.clone()
    }
    fn declaring_type(&self) -> Rc<dyn IXamlType> {
        self.method.declaring_type()
    }
    fn parameters_with_this(&self) -> Vec<Rc<dyn IXamlType>> {
        self.parameters_with_this.clone()
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn type_name(&self) -> &'static str {
        "XamlWrappedMethod"
    }
}

pub struct XamlWrappedMethodWithCasts {
    method: Rc<dyn IXamlWrappedMethod>,
    parameters_with_this: Vec<Rc<dyn IXamlType>>,
}

impl XamlWrappedMethodWithCasts {
    pub fn new(
        method: Rc<dyn IXamlWrappedMethod>,
        new_argument_types: Vec<Rc<dyn IXamlType>>,
    ) -> XamlResult<Rc<Self>> {
        if method.parameters_with_this().len() != new_argument_types.len() {
            return Err(XamlError::argument("Method argument count mismatch"));
        }
        Ok(Rc::new(Self {
            method,
            parameters_with_this: new_argument_types,
        }))
    }

    /// The wrapped method (private upstream; exposed for emitter backends).
    pub fn method(&self) -> &Rc<dyn IXamlWrappedMethod> {
        &self.method
    }
}

impl IXamlWrappedMethod for XamlWrappedMethodWithCasts {
    fn name(&self) -> String {
        self.method.name()
    }
    fn return_type(&self) -> Rc<dyn IXamlType> {
        self.method.return_type()
    }
    fn declaring_type(&self) -> Rc<dyn IXamlType> {
        self.method.declaring_type()
    }
    fn parameters_with_this(&self) -> Vec<Rc<dyn IXamlType>> {
        self.parameters_with_this.clone()
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn type_name(&self) -> &'static str {
        "XamlWrappedMethodWithCasts"
    }
}

/// A static-looking method that casts its arguments before calling the wrapped method.
pub struct XamlMethodWithCasts {
    method: Rc<dyn IXamlMethod>,
    base_parameters_with_this: Vec<Rc<dyn IXamlType>>,
    parameters: Vec<Rc<dyn IXamlType>>,
}

impl XamlMethodWithCasts {
    pub fn new(
        method: Rc<dyn IXamlMethod>,
        new_argument_types: Vec<Rc<dyn IXamlType>>,
    ) -> XamlResult<Rc<Self>> {
        let base_parameters_with_this = method.parameters_with_this();
        if base_parameters_with_this.len() != new_argument_types.len() {
            return Err(XamlError::argument("Method argument count mismatch"));
        }
        Ok(Rc::new(Self {
            method,
            base_parameters_with_this,
            parameters: new_argument_types,
        }))
    }

    /// The wrapped method (private upstream; exposed for emitter backends).
    pub fn method(&self) -> &Rc<dyn IXamlMethod> {
        &self.method
    }

    /// The wrapped method's parameters including `this` (private upstream).
    pub fn base_parameters_with_this(&self) -> &[Rc<dyn IXamlType>] {
        &self.base_parameters_with_this
    }
}

impl IXamlMember for XamlMethodWithCasts {
    fn name(&self) -> String {
        self.method.name()
    }
    fn declaring_type(&self) -> Rc<dyn IXamlType> {
        self.method.declaring_type()
    }
}

impl IXamlMethod for XamlMethodWithCasts {
    fn is_public(&self) -> bool {
        true
    }
    fn is_private(&self) -> bool {
        false
    }
    fn is_family(&self) -> bool {
        false
    }
    fn is_static(&self) -> bool {
        true
    }
    fn contains_generic_parameters(&self) -> bool {
        self.method.contains_generic_parameters()
    }
    fn is_generic_method(&self) -> bool {
        self.method.is_generic_method()
    }
    fn is_generic_method_definition(&self) -> bool {
        self.method.is_generic_method_definition()
    }
    fn return_type(&self) -> Rc<dyn IXamlType> {
        self.method.return_type()
    }
    fn parameters(&self) -> Vec<Rc<dyn IXamlType>> {
        self.parameters.clone()
    }
    fn make_generic_method(
        &self,
        _type_arguments: &[Rc<dyn IXamlType>],
    ) -> XamlResult<Rc<dyn IXamlMethod>> {
        Err(XamlError::invalid_operation(
            "Operation is not valid due to the current state of the object.",
        ))
    }
    fn custom_attributes(&self) -> Vec<Rc<dyn IXamlCustomAttribute>> {
        self.method.custom_attributes()
    }
    fn get_parameter_info(&self, index: usize) -> XamlResult<Rc<dyn IXamlParameterInfo>> {
        self.method.get_parameter_info(index)
    }
    fn generic_parameters(&self) -> Vec<Rc<dyn IXamlType>> {
        self.method.generic_parameters()
    }
    fn generic_arguments(&self) -> Vec<Rc<dyn IXamlType>> {
        self.method.generic_arguments()
    }
    fn equals(&self, other: &dyn IXamlMethod) -> bool {
        match other.as_any().downcast_ref::<XamlMethodWithCasts>() {
            Some(mwc) => {
                mwc.method.equals(&*self.method)
                    && xaml_types_sequence_equal(&mwc.parameters, &self.parameters)
            }
            None => false,
        }
    }
    fn get_hash_code(&self) -> u64 {
        self.method.get_hash_code()
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}

pub struct XamlDeferredContentNode {
    base: XamlAstNode,
    deferred_content_customization: Option<Rc<dyn IXamlMethod>>,
    deferred_content_customization_type_parameter: Option<Rc<dyn IXamlType>>,
    func_type: Rc<dyn IXamlType>,
    pub value: RefCell<Rc<dyn IXamlAstValueNode>>,
    type_: Rc<dyn IXamlAstTypeReference>,
}

impl XamlDeferredContentNode {
    pub fn new(
        value: Rc<dyn IXamlAstValueNode>,
        deferred_content_customization_type_parameter: Option<Rc<dyn IXamlType>>,
        config: &TransformerConfiguration,
    ) -> XamlResult<Rc<Self>> {
        let deferred_content_customization = config
            .type_mappings
            .deferred_content_executor_customization
            .clone();
        let well_known = config.well_known_types();
        let func_type = well_known.get_func_of_t(2).make_generic_type(&[
            config.type_mappings.service_provider()?,
            well_known.object.clone(),
        ])?;

        let return_type = deferred_content_customization
            .as_ref()
            .map(|m| m.return_type())
            .unwrap_or_else(|| func_type.clone());
        let type_ = XamlAstClrTypeReference::new(&*value, return_type, false);
        Ok(Rc::new(Self {
            base: XamlAstNode::new(&*value),
            deferred_content_customization,
            deferred_content_customization_type_parameter,
            func_type,
            value: RefCell::new(value),
            type_,
        }))
    }

    pub fn value(&self) -> Rc<dyn IXamlAstValueNode> {
        self.value.borrow().clone()
    }

    /// `_deferredContentCustomization` (private upstream; exposed for emitter backends).
    pub fn deferred_content_customization(&self) -> Option<&Rc<dyn IXamlMethod>> {
        self.deferred_content_customization.as_ref()
    }

    /// `_deferredContentCustomizationTypeParameter` (private upstream; exposed for emitter backends).
    pub fn deferred_content_customization_type_parameter(&self) -> Option<&Rc<dyn IXamlType>> {
        self.deferred_content_customization_type_parameter.as_ref()
    }

    /// `_funcType`: `Func<IServiceProvider, object>` (private upstream; exposed for emitter backends).
    pub fn func_type(&self) -> &Rc<dyn IXamlType> {
        &self.func_type
    }
}

xaml_line_info_impl!(XamlDeferredContentNode, base);

impl IXamlAstNode for XamlDeferredContentNode {
    xaml_ast_node_members!("XamlDeferredContentNode", value);

    fn visit_children(&self, visitor: &mut dyn IXamlAstVisitor) -> XamlResult<()> {
        visit_cell(&self.value, visitor)
    }
}

impl IXamlAstValueNode for XamlDeferredContentNode {
    fn type_(&self) -> Rc<dyn IXamlAstTypeReference> {
        self.type_.clone()
    }
}

pub struct XamlDeferredContentInitializeIntermediateRootNode {
    base: XamlAstNode,
    pub value: RefCell<Rc<dyn IXamlAstValueNode>>,
}

impl XamlDeferredContentInitializeIntermediateRootNode {
    pub fn new(value: Rc<dyn IXamlAstValueNode>) -> Rc<Self> {
        Rc::new(Self {
            base: XamlAstNode::new(&*value),
            value: RefCell::new(value),
        })
    }

    pub fn value(&self) -> Rc<dyn IXamlAstValueNode> {
        self.value.borrow().clone()
    }
}

xaml_line_info_impl!(XamlDeferredContentInitializeIntermediateRootNode, base);

impl IXamlAstNode for XamlDeferredContentInitializeIntermediateRootNode {
    xaml_ast_node_members!("XamlDeferredContentInitializeIntermediateRootNode", value);

    fn visit_children(&self, visitor: &mut dyn IXamlAstVisitor) -> XamlResult<()> {
        visit_cell(&self.value, visitor)
    }
}

impl IXamlAstValueNode for XamlDeferredContentInitializeIntermediateRootNode {
    fn type_(&self) -> Rc<dyn IXamlAstTypeReference> {
        self.value.borrow().type_()
    }
}
