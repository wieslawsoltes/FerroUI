//! Port of
//! `CompilerExtensions/Transformers/FerroXamlIlControlTemplateTargetTypeMetadataTransformer.cs`.

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use xamlx::ast::{
    IXamlAstNode, IXamlAstTypeReference, IXamlAstValueNode, IXamlAstVisitor,
    XamlAstClrTypeReference, XamlAstExtensions, XamlAstNodeExtensions, XamlAstObjectNode,
    XamlAstPropertyReferenceExtensions, XamlAstTextNode, XamlAstXamlPropertyValueNode,
    XamlTypeExtensionNode, XamlValueWithSideEffectNodeBase,
};
use xamlx::exceptions::XamlResult;
use xamlx::transform::transformers::TypeReferenceResolver;
use xamlx::transform::{AstTransformationContext, IXamlAstTransformer};
use xamlx::type_system::{IXamlType, XamlTypeKey};
use xamlx::{xaml_ast_node_members, xaml_line_info_impl};

use crate::compiler_extensions::transformers::FerroXamlIlWellKnownTypesExtensions;

/// Wraps an object whose type is a control template scope (a type that carries the
/// `ControlTemplateScope` attribute itself, on a base type or on a directly listed interface)
/// in a control template target-type scope.
pub struct FerroXamlIlControlTemplateTargetTypeMetadataTransformer;

impl IXamlAstTransformer for FerroXamlIlControlTemplateTargetTypeMetadataTransformer {
    fn transform(
        &self,
        context: &AstTransformationContext,
        node: Rc<dyn IXamlAstNode>,
    ) -> XamlResult<Rc<dyn IXamlAstNode>> {
        let Some(on) = node.cast::<XamlAstObjectNode>() else {
            return Ok(node);
        };
        if !ControlTemplateScopeCache::get_or_create(context)
            .is_control_template_scope(&on.type_().get_clr_type()?)
        {
            return Ok(node);
        }

        let mut tt: Option<Rc<XamlAstXamlPropertyValueNode>> = None;
        let children = on.children.borrow().clone();
        for child in children {
            if let Some(ch) = child.cast::<XamlAstXamlPropertyValueNode>() {
                if ch.property().get_clr_property()?.name() == "TargetType" {
                    tt = Some(ch);
                    break;
                }
            }
        }

        let parent_nodes = context.parent_nodes();
        if parent_nodes
            .first()
            .is_some_and(|p| p.is::<FerroXamlIlTargetTypeMetadataNode>())
        {
            // Deja vu. I've just been in this place before
            return Ok(node);
        }

        let templatable_base_type = context.get_ferro_types().control.clone();

        let first_value = tt.and_then(|tt| tt.values.borrow().first().cloned());
        let type_extension = first_value
            .as_ref()
            .and_then(|v| v.cast::<XamlTypeExtensionNode>());
        let text_node = first_value.as_ref().and_then(|v| v.cast::<XamlAstTextNode>());

        let target_type: Rc<dyn IXamlAstTypeReference> = if let Some(tn) = type_extension {
            tn.value()
        } else if let Some(text_node) = text_node {
            TypeReferenceResolver::resolve_type_by_xml_name(
                context,
                &text_node.text(),
                false,
                &*text_node,
                true,
            )?
        } else {
            let parent_scope = parent_nodes
                .iter()
                .find_map(|n| n.cast::<FerroXamlIlTargetTypeMetadataNode>())
                .filter(|scope| scope.scope_type == ScopeTypes::Style);
            if let Some(parent_scope) = parent_scope {
                parent_scope.target_type()
            } else {
                let mut direct_parent_type: Option<Rc<dyn IXamlAstTypeReference>> = None;
                if let Some(direct_parent_node) =
                    parent_nodes.get(1).and_then(|n| n.cast::<XamlAstObjectNode>())
                {
                    if templatable_base_type
                        .is_assignable_from(&*direct_parent_node.type_().get_clr_type()?)
                    {
                        direct_parent_type = Some(direct_parent_node.type_());
                    }
                }
                match direct_parent_type {
                    Some(type_) => type_,
                    None => XamlAstClrTypeReference::new(&*node, templatable_base_type, false),
                }
            }
        };

        let value: Rc<dyn IXamlAstValueNode> = on;
        Ok(FerroXamlIlTargetTypeMetadataNode::new(
            value,
            target_type,
            ScopeTypes::ControlTemplate,
        ))
    }
}

/// Per-context cache of "is this type a control template scope".
struct ControlTemplateScopeCache {
    control_template_scope_attribute_type: Rc<dyn IXamlType>,
    is_scope_by_type: RefCell<HashMap<XamlTypeKey, bool>>,
}

impl ControlTemplateScopeCache {
    fn get_or_create(context: &AstTransformationContext) -> Rc<ControlTemplateScopeCache> {
        if let Some(cache) = context.try_get_item::<ControlTemplateScopeCache>() {
            return cache;
        }
        let cache = Rc::new(ControlTemplateScopeCache {
            control_template_scope_attribute_type: context
                .get_ferro_types()
                .control_template_scope_attribute
                .clone(),
            is_scope_by_type: RefCell::new(HashMap::new()),
        });
        context.set_item(cache.clone());
        cache
    }

    fn has_scope_attribute(&self, type_: &dyn IXamlType) -> bool {
        type_
            .custom_attributes()
            .iter()
            .any(|attr| attr.type_().equals(&*self.control_template_scope_attribute_type))
    }

    fn is_control_template_scope_core(&self, type_: &Rc<dyn IXamlType>) -> bool {
        let mut t = Some(type_.clone());
        while let Some(current) = t {
            if self.has_scope_attribute(&*current) {
                return true;
            }
            t = current.base_type();
        }

        type_
            .interfaces()
            .iter()
            .any(|iface| self.has_scope_attribute(&**iface))
    }

    fn is_control_template_scope(&self, type_: &Rc<dyn IXamlType>) -> bool {
        let key = XamlTypeKey(type_.clone());
        if let Some(is_scope) = self.is_scope_by_type.borrow().get(&key) {
            return *is_scope;
        }
        let is_scope = self.is_control_template_scope_core(type_);
        self.is_scope_by_type.borrow_mut().insert(key, is_scope);
        is_scope
    }
}

/// `FerroXamlIlTargetTypeMetadataNode.ScopeTypes`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum ScopeTypes {
    Style = 1,
    ControlTemplate = 2,
    Transitions = 3,
    Container = 4,
}

impl ScopeTypes {
    /// `Enum.ToString()`.
    pub fn name(self) -> &'static str {
        match self {
            ScopeTypes::Style => "Style",
            ScopeTypes::ControlTemplate => "ControlTemplate",
            ScopeTypes::Transitions => "Transitions",
            ScopeTypes::Container => "Container",
        }
    }
}

/// Wraps the value of a style, control template, transitions or container scope and records
/// the type its properties are looked up on. As upstream, the node only visits its value.
pub struct FerroXamlIlTargetTypeMetadataNode {
    pub base: XamlValueWithSideEffectNodeBase,
    pub target_type: RefCell<Rc<dyn IXamlAstTypeReference>>,
    pub scope_type: ScopeTypes,
}

impl FerroXamlIlTargetTypeMetadataNode {
    pub fn new(
        value: Rc<dyn IXamlAstValueNode>,
        target_type: Rc<dyn IXamlAstTypeReference>,
        type_: ScopeTypes,
    ) -> Rc<Self> {
        Rc::new(Self {
            base: XamlValueWithSideEffectNodeBase::new(&*value.clone(), value),
            target_type: RefCell::new(target_type),
            scope_type: type_,
        })
    }

    pub fn value(&self) -> Rc<dyn IXamlAstValueNode> {
        self.base.value()
    }

    pub fn target_type(&self) -> Rc<dyn IXamlAstTypeReference> {
        self.target_type.borrow().clone()
    }
}

xaml_line_info_impl!(FerroXamlIlTargetTypeMetadataNode, base);

impl IXamlAstNode for FerroXamlIlTargetTypeMetadataNode {
    xaml_ast_node_members!("FerroXamlIlTargetTypeMetadataNode", value);

    fn visit_children(&self, visitor: &mut dyn IXamlAstVisitor) -> XamlResult<()> {
        self.base.visit_children(visitor)
    }

    fn as_value_with_side_effect_node_base(&self) -> Option<&XamlValueWithSideEffectNodeBase> {
        Some(&self.base)
    }
}

impl IXamlAstValueNode for FerroXamlIlTargetTypeMetadataNode {
    fn type_(&self) -> Rc<dyn IXamlAstTypeReference> {
        self.base.type_()
    }
}
