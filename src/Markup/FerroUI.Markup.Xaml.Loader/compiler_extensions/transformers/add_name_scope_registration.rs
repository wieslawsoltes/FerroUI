//! Port of `CompilerExtensions/Transformers/AddNameScopeRegistration.cs`.

use std::cell::RefCell;
use std::rc::Rc;

use xamlx::ast::{
    visit_cell, IXamlAstManipulationNode, IXamlAstNode, IXamlAstTypeReference, IXamlAstValueNode,
    IXamlAstVisitor, XamlAstCompilerLocalNode, XamlAstConstructableObjectNode, XamlAstExtensions,
    XamlAstLocalInitializationNodeEmitter, XamlAstNode, XamlAstNodeExtensions, XamlAstTextNode,
    XamlManipulationGroupNode, XamlPropertyAssignmentNode, XamlValueWithSideEffectNodeBase,
};
use xamlx::exceptions::XamlResult;
use xamlx::transform::{AstTransformationContext, IXamlAstTransformer};
use xamlx::type_system::IXamlType;
use xamlx::{xaml_ast_node_members, xaml_line_info_impl};

/// Registers named elements in the name scope: an assignment of the `Name` property of an
/// `INamed` type is followed by a [`FerroNameScopeRegistrationXamlIlNode`] for the same name.
/// The value of a deferred-content (template) property is wrapped in a
/// [`NestedScopeMetadataNode`], because names below it belong to another scope.
pub struct AddNameScopeRegistration;

impl IXamlAstTransformer for AddNameScopeRegistration {
    fn transform(
        &self,
        context: &AstTransformationContext,
        node: Rc<dyn IXamlAstNode>,
    ) -> XamlResult<Rc<dyn IXamlAstNode>> {
        let Some(pa) = node.cast::<XamlPropertyAssignmentNode>() else {
            return Ok(node);
        };

        if pa.property.name() == "Name"
            && pa
                .property
                .declaring_type()
                .interfaces()
                .iter()
                .any(|t| t.is("FerroUI", "INamed"))
        {
            let already_registered = context
                .first_parent_node()
                .and_then(|parent| parent.cast::<XamlManipulationGroupNode>())
                .is_some_and(|mg| {
                    mg.children
                        .borrow()
                        .iter()
                        .any(|c| c.is::<FerroNameScopeRegistrationXamlIlNode>())
                });
            if already_registered {
                return Ok(node);
            }

            let string = context.configuration().well_known_types().string.clone();
            let mut value: Option<Rc<dyn IXamlAstValueNode>> = None;
            let count = pa.values.borrow().len();
            for c in 0..count {
                let candidate = pa.values.borrow()[c].clone();
                if candidate.type_().get_clr_type()?.equals(&*string) {
                    if candidate.is::<XamlAstTextNode>() {
                        value = Some(candidate);
                    } else {
                        let local = XamlAstCompilerLocalNode::from_value(&candidate)?;
                        // Wrap original in local initialization
                        pa.values.borrow_mut()[c] = XamlAstLocalInitializationNodeEmitter::new(
                            &*candidate,
                            candidate.clone(),
                            local.clone(),
                        );
                        // Use local
                        value = Some(local);
                    }
                    break;
                }
            }

            if let Some(value) = value {
                let object_type = match context
                    .parent_nodes()
                    .into_iter()
                    .find_map(|n| n.cast::<XamlAstConstructableObjectNode>())
                {
                    Some(object) => Some(object.type_.borrow().clone().get_clr_type()?),
                    None => None,
                };
                let children: Vec<Rc<dyn IXamlAstManipulationNode>> = vec![
                    pa.clone(),
                    FerroNameScopeRegistrationXamlIlNode::new(value, object_type),
                ];
                return Ok(XamlManipulationGroupNode::new(&*pa, Some(children)));
            }
        } else {
            let deferred_attributes = &context
                .configuration()
                .type_mappings
                .deferred_content_property_attributes;
            if pa
                .property
                .custom_attributes()
                .iter()
                .any(|attr| deferred_attributes.iter().any(|d| d.equals(&*attr.type_())))
            {
                let mut values = pa.values.borrow_mut();
                if let Some(last) = values.last_mut() {
                    let wrapped = NestedScopeMetadataNode::new(last.clone());
                    *last = wrapped;
                }
            }
        }

        Ok(node)
    }
}

/// Marks the value of a deferred-content (template) property: names registered below it
/// belong to a nested name scope.
pub struct NestedScopeMetadataNode {
    pub base: XamlValueWithSideEffectNodeBase,
}

impl NestedScopeMetadataNode {
    pub fn new(value: Rc<dyn IXamlAstValueNode>) -> Rc<Self> {
        Rc::new(Self {
            base: XamlValueWithSideEffectNodeBase::new(&*value.clone(), value),
        })
    }

    pub fn value(&self) -> Rc<dyn IXamlAstValueNode> {
        self.base.value()
    }
}

xaml_line_info_impl!(NestedScopeMetadataNode, base);

impl IXamlAstNode for NestedScopeMetadataNode {
    xaml_ast_node_members!("NestedScopeMetadataNode", value);

    fn visit_children(&self, visitor: &mut dyn IXamlAstVisitor) -> XamlResult<()> {
        self.base.visit_children(visitor)
    }

    fn as_value_with_side_effect_node_base(&self) -> Option<&XamlValueWithSideEffectNodeBase> {
        Some(&self.base)
    }
}

impl IXamlAstValueNode for NestedScopeMetadataNode {
    fn type_(&self) -> Rc<dyn IXamlAstTypeReference> {
        self.base.type_()
    }
}

/// Registers the object on top of the evaluation stack in the current name scope under
/// [`FerroNameScopeRegistrationXamlIlNode::name`].
///
/// # What a back end has to do (upstream `FerroNameScopeRegistrationXamlIlNodeEmitter`)
///
/// The emitter handles exactly this node type. Stack: the object being initialized (the node
/// is a manipulation); it is consumed and nothing is left.
///
/// 1. store the target object in a temporary of type `object`;
/// 2. load the name scope: the runtime context field named
///    `FerroXamlIlLanguage::CONTEXT_NAME_SCOPE_FIELD_NAME` (see
///    [`crate::compiler_extensions::FerroXamlIlContextNameScopeField`]) of the context local;
/// 3. emit the `name` node as its own type (a string constant, or the local a
///    non-constant name was stored in by the `XamlAstLocalInitializationNodeEmitter` that
///    replaced the assigned value);
/// 4. load the target object back;
/// 5. call `types.i_name_scope_register` on the scope: `INameScope.Register(string, object)`.
///
/// That is `context.FerroNameScope.Register(name, target)`. `target_type` (the type of the
/// named element, when known) is not used by the emitter; it feeds the generation of typed
/// name fields.
pub struct FerroNameScopeRegistrationXamlIlNode {
    base: XamlAstNode,
    pub name: RefCell<Rc<dyn IXamlAstValueNode>>,
    pub target_type: Option<Rc<dyn IXamlType>>,
}

impl FerroNameScopeRegistrationXamlIlNode {
    pub fn new(name: Rc<dyn IXamlAstValueNode>, target_type: Option<Rc<dyn IXamlType>>) -> Rc<Self> {
        Rc::new(Self {
            base: XamlAstNode::new(&*name),
            target_type,
            name: RefCell::new(name),
        })
    }

    pub fn name(&self) -> Rc<dyn IXamlAstValueNode> {
        self.name.borrow().clone()
    }
}

xaml_line_info_impl!(FerroNameScopeRegistrationXamlIlNode, base);

impl IXamlAstNode for FerroNameScopeRegistrationXamlIlNode {
    xaml_ast_node_members!("FerroNameScopeRegistrationXamlIlNode", manipulation);

    fn visit_children(&self, visitor: &mut dyn IXamlAstVisitor) -> XamlResult<()> {
        visit_cell(&self.name, visitor)
    }
}

impl IXamlAstManipulationNode for FerroNameScopeRegistrationXamlIlNode {}
