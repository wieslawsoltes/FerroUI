use super::Node;
use crate::controls::NameScopeRef;
use crate::data::core::expression_nodes::{
    CastTarget, DataContextNode, ExpressionNode, FerroPropertyAccessorNode, LogicalAncestorElementNode,
    LogicalNotNode, NamedElementNode, PropertyAccessorNode, ReflectionIndexerNode, StreamNode, TemplatedParentNode,
    TypeCastNode, VisualAncestorElementNode,
};
use crate::data::core::ExpressionParseException;
use crate::data::{RelativeSource, RelativeSourceMode, TreeType};
use crate::{FerroPropertyRegistry, TypeInfo};
use std::rc::Rc;

/// Resolves a type name (namespace, name) of a binding path.
pub type TypeResolver = Rc<dyn Fn(Option<&str>, &str) -> Option<CastTarget>>;

/// Creates expression nodes from the syntax of a binding path.
pub struct ExpressionNodeFactory;

fn error(message: String) -> ExpressionParseException {
    ExpressionParseException::new(0, message)
}

impl ExpressionNodeFactory {
    /// Appends the nodes for `ast_nodes` to `result`. Returns whether the
    /// path is rooted (selects its own source).
    pub fn create_from_ast(
        ast_nodes: &[Node],
        type_resolver: Option<&TypeResolver>,
        name_scope: Option<&NameScopeRef>,
        result: &mut Vec<Rc<dyn ExpressionNode>>,
    ) -> Result<bool, ExpressionParseException> {
        let mut negated = 0;
        let mut is_rooted = false;

        for ast_node in ast_nodes {
            let node: Option<Rc<dyn ExpressionNode>> = match ast_node {
                Node::Ancestor { namespace, type_name, level } => {
                    is_rooted = true;
                    let type_ = match type_name.as_deref().filter(|n| !n.is_empty()) {
                        Some(name) => Some(Self::lookup_class(type_resolver, namespace.as_deref(), name)?),
                        None => None,
                    };
                    Some(LogicalAncestorElementNode::new(type_, (*level).max(0) as usize))
                }
                Node::AttachedPropertyName { accepts_null, namespace, type_name, property_name } => {
                    let type_ = Self::lookup_class(type_resolver, Some(namespace), type_name)?;
                    let property = FerroPropertyRegistry::instance()
                        .find_registered(type_, property_name)
                        .ok_or_else(|| error(format!("Cannot find property {type_}.{property_name}.")))?;
                    Some(FerroPropertyAccessorNode::new(property, *accepts_null))
                }
                Node::EmptyExpression => None,
                Node::Indexer { arguments } => Some(ReflectionIndexerNode::new(arguments.clone())),
                Node::Name { name } => {
                    is_rooted = true;
                    Some(NamedElementNode::new(name_scope, name))
                }
                Node::Not => {
                    negated += 1;
                    None
                }
                Node::PropertyName { accepts_null, property_name } => {
                    Some(PropertyAccessorNode::new_dynamic(property_name, *accepts_null))
                }
                Node::SelfNode => {
                    is_rooted = true;
                    None
                }
                Node::Stream => Some(StreamNode::new_dynamic()),
                Node::TypeCast { namespace, type_name } => {
                    Some(TypeCastNode::new(Self::lookup_type(type_resolver, Some(namespace), type_name)?))
                }
            };
            if let Some(node) = node {
                result.push(node);
            }
        }

        for _ in 0..negated {
            result.push(LogicalNotNode::new());
        }
        Ok(is_rooted)
    }

    /// The source node for a relative source; none for the self mode.
    pub fn create_relative_source(source: &RelativeSource) -> Option<Rc<dyn ExpressionNode>> {
        let level = (source.ancestor_level() - 1).max(0) as usize;
        match source.mode() {
            RelativeSourceMode::DataContext => Some(DataContextNode::new()),
            RelativeSourceMode::TemplatedParent => Some(TemplatedParentNode::new()),
            RelativeSourceMode::SelfMode => None,
            RelativeSourceMode::FindAncestor => match source.tree() {
                TreeType::Logical => Some(LogicalAncestorElementNode::new(source.ancestor_type(), level)),
                TreeType::Visual => Some(VisualAncestorElementNode::new(source.ancestor_type(), level)),
            },
        }
    }

    fn lookup_type(
        type_resolver: Option<&TypeResolver>,
        namespace: Option<&str>,
        name: &str,
    ) -> Result<CastTarget, ExpressionParseException> {
        type_resolver
            .and_then(|r| r(namespace, name))
            .ok_or_else(|| error(format!("Unable to resolve type '{}:{}'.", namespace.unwrap_or(""), name)))
    }

    fn lookup_class(
        type_resolver: Option<&TypeResolver>,
        namespace: Option<&str>,
        name: &str,
    ) -> Result<&'static TypeInfo, ExpressionParseException> {
        match Self::lookup_type(type_resolver, namespace, name)? {
            CastTarget::Class(t) => Ok(t),
            CastTarget::Value(t) => Err(error(format!("Type '{t}' is not a class."))),
        }
    }
}
