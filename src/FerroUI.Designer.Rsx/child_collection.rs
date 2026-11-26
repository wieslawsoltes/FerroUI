//! Read explicit panel children through the shared RSX grammar.

#[cfg(test)]
mod tests;

use crate::{
    animation::syntax::span, blocks::expression_macro, catalog::Children, registered::Class,
};
use ferroui_designer_core::SourceSpan;
use ferroui_view_syntax::{Attribute, ElementNode, Node};
use syn::{ext::IdentExt, punctuated::Punctuated, Expr, ExprMacro, Path, Token};

pub(crate) struct InlineChildren {
    pub(crate) attribute: usize,
    pub(crate) close: usize,
    pub(crate) unterminated: Option<SourceSpan>,
    pub(crate) members: Vec<CollectionMember>,
}

#[derive(Clone, Copy)]
pub(crate) struct CollectionSource {
    pub(crate) assignment: SourceSpan,
    pub(crate) close: usize,
    pub(crate) unterminated: Option<SourceSpan>,
    pub(crate) len: usize,
}

#[derive(Clone, Copy)]
pub(crate) enum ChildEnvelope {
    Collection(SourceSpan),
    Block(SourceSpan),
}

impl ChildEnvelope {
    pub(crate) fn span(self) -> SourceSpan {
        match self {
            Self::Collection(span) | Self::Block(span) => span,
        }
    }
}

pub(crate) struct CollectionMember {
    pub(crate) source: SourceSpan,
    pub(crate) control: Option<Node>,
}

impl InlineChildren {
    pub(crate) fn read(owner: &ElementNode, class: &Class) -> Option<Self> {
        if class.children() != Children::Panel || !owner.children.is_empty() {
            return None;
        }
        let mut collection = None;
        for (index, attribute) in owner.attributes.iter().enumerate() {
            let (setter, value) = match attribute {
                Attribute::Value {
                    owner: None,
                    name,
                    value,
                    ..
                } => (name, Some(value.as_ref())),
                Attribute::Call { name, arguments } => {
                    (name, arguments.first().filter(|_| arguments.len() == 1))
                }
                _ => continue,
            };
            match setter.unraw().to_string().as_str() {
                "children" if collection.is_none() => collection = Some((index, value?)),
                "children" | "child" | "content" | "items" | "items_source" | "prop" | "set"
                | "with" => return None,
                _ => {}
            }
        }
        let (attribute, expression) = collection?;
        if class.xaml_property(None, "children").is_some() {
            return None;
        }
        let (close, unterminated, members) = match expression {
            Expr::Array(array) if array.attrs.is_empty() => entries(
                &array.elems,
                array.bracket_token.span.close().byte_range().start,
            ),
            Expr::Macro(expr) if qualified_macro(expr, &["std", "vec"]) => {
                let items = expr
                    .mac
                    .parse_body_with(Punctuated::<Expr, Token![,]>::parse_terminated)
                    .ok()?;
                entries(&items, expr.mac.delimiter.span().close().byte_range().start)
            }
            _ => return None,
        };
        (!members.is_empty()).then_some(Self {
            attribute,
            close,
            unterminated,
            members,
        })
    }
}

pub(crate) fn control_expression(value: &Expr) -> Option<Node> {
    let invocation = expression_macro(value)?;
    if !(invocation.path.is_ident("rsx")
        || is_qualified(&invocation.path, &["ferroui_view", "rsx"]))
    {
        return None;
    }
    let mut nodes = ferroui_view_syntax::parse(invocation.tokens.clone()).ok()?;
    if matches!(nodes.as_slice(), [Node::Element(_)]) {
        nodes.pop()
    } else {
        None
    }
}

fn entries(
    values: &Punctuated<Expr, Token![,]>,
    close: usize,
) -> (usize, Option<SourceSpan>, Vec<CollectionMember>) {
    let unterminated = values.last().filter(|_| !values.trailing_punct()).map(span);
    let members = values
        .iter()
        .map(|value| CollectionMember {
            source: span(value),
            control: control_expression(value),
        })
        .collect();
    (close, unterminated, members)
}

fn qualified_macro(expr: &ExprMacro, names: &[&str]) -> bool {
    expr.attrs.is_empty()
        && expr.mac.path.leading_colon.is_some()
        && is_qualified(&expr.mac.path, names)
}

fn is_qualified(path: &Path, names: &[&str]) -> bool {
    path.segments.len() == names.len()
        && path
            .segments
            .iter()
            .zip(names)
            .all(|(part, name)| part.ident == *name && part.arguments.is_empty())
}
