//! Resolve style operands using the runtime grammar and the active Cargo schema.
mod resolve;

use crate::{binding, PropertyInfo, Schema, TypeInfo, FRAMEWORK_NAMESPACE};
use ferroui_designer_core::{Document, Node, NodeKind, SourceSpan, XmlText};
use ferroui_markup_syntax::{character_reader::is_white_space, selector_grammar::SelectorGrammar};
use std::{collections::BTreeSet, ops::Range};

pub(super) struct References {
    pub span: SourceSpan,
    pub types: Vec<TypeUse>,
    pub properties: Vec<PropertyUse>,
    pub unparsed: bool,
}

pub(super) struct TypeUse {
    pub span: SourceSpan,
    pub spelling: String,
    pub namespace: Option<String>,
}

pub(super) struct PropertyUse {
    pub span: SourceSpan,
    pub spelling: String,
    pub member: Option<PropertyInfo>,
}

pub(super) fn references(schema: &Schema, document: &Document, node: &Node) -> Option<References> {
    let source = Source::for_node(schema, document, node)?;
    let mut result = References {
        span: source.span,
        types: Vec::new(),
        properties: Vec::new(),
        unparsed: true,
    };
    if let Ok((syntax, operands)) = SelectorGrammar::operand_ranges(source.selector()) {
        let nesting = resolve::contains_nesting(&syntax)
            .then(|| enclosing_target(schema, document, source.owner))
            .flatten();
        if resolve::references(
            schema,
            document,
            &source,
            nesting,
            &syntax,
            operands,
            &mut result,
        ) {
            result.unparsed = false;
        } else {
            result.types.clear();
            result.properties.clear();
        }
    }
    Some(result)
}

pub(super) fn target_type<'s>(
    schema: &'s Schema,
    document: &Document,
    style: &Node,
) -> Option<&'s TypeInfo> {
    let source = Source::for_node(schema, document, style).or_else(|| {
        style
            .children
            .iter()
            .filter_map(|id| document.ast().node(*id))
            .find_map(|child| {
                Source::for_node(schema, document, child)
                    .filter(|source| source.owner.id == style.id)
            })
    })?;
    let syntax = SelectorGrammar::parse(source.selector()).ok()?;
    let nesting = resolve::contains_nesting(&syntax)
        .then(|| enclosing_target(schema, document, style))
        .flatten();
    resolve::target_type(schema, document, &source, nesting, &syntax)
}

fn enclosing_target<'s>(
    schema: &'s Schema,
    document: &Document,
    style: &Node,
) -> Option<&'s TypeInfo> {
    let ast = document.ast();
    let mut parent = style.parent;
    while let Some(node) = parent.and_then(|id| ast.node(id)) {
        if is_style(schema, node) {
            return target_type(schema, document, node);
        }
        if schema
            .type_info(&node.name, node.namespace_uri.as_deref())
            .is_some_and(|ty| {
                ty.namespace == FRAMEWORK_NAMESPACE
                    && matches!(ty.name.as_str(), "ControlTheme" | "ControlTemplate")
            })
        {
            return binding::type_reference(
                schema,
                document,
                node.id,
                node.attribute("TargetType")?,
            );
        }
        parent = node.parent;
    }
    None
}

pub(super) fn is_style(schema: &Schema, node: &Node) -> bool {
    let mut seen = BTreeSet::new();
    let mut current = schema.type_info(&node.name, node.namespace_uri.as_deref());
    while let Some(ty) = current {
        if ty.name == "Style" && ty.namespace == FRAMEWORK_NAMESPACE {
            return true;
        }
        if !seen.insert((&ty.namespace, &ty.name)) {
            break;
        }
        current = schema.base_type(ty);
    }
    false
}

struct Source<'a> {
    owner: &'a Node,
    scope: &'a Node,
    text: XmlText<'a>,
    span: SourceSpan,
    selector_range: Range<usize>,
}

impl<'a> Source<'a> {
    fn new(owner: &'a Node, scope: &'a Node, span: SourceSpan, text: XmlText<'a>) -> Self {
        let left = text.value().trim_start_matches(is_white_space);
        let start = text.value().len() - left.len();
        let end = start + left.trim_end_matches(is_white_space).len();
        Self {
            owner,
            scope,
            text,
            span,
            selector_range: start..end,
        }
    }

    fn selector(&self) -> &str {
        &self.text.value()[self.selector_range.clone()]
    }

    fn operand_span(&self, range: Range<usize>) -> Option<SourceSpan> {
        let start = self.selector_range.start;
        self.text
            .source_span(start + range.start..start + range.end)
    }

    fn for_node(schema: &Schema, document: &'a Document, node: &'a Node) -> Option<Self> {
        let source = document.valid_source();
        if let Some(attribute) = node
            .attribute_info("Selector")
            .filter(|_| is_style(schema, node))
        {
            let span = attribute.value_span;
            return Some(Self::new(
                node,
                node,
                span,
                XmlText::attribute(source, span)?,
            ));
        }
        if node.kind != NodeKind::PropertyElement || !node.name.ends_with(".Selector") {
            return None;
        }
        let ast = document.ast();
        let owner = ast.node(node.parent?)?;
        if !is_style(schema, owner) || schema.element_property(owner, node)?.name != "Selector" {
            return None;
        }
        let mut text = node
            .children
            .iter()
            .filter_map(|id| ast.node(*id))
            .filter(|child| {
                child.kind != NodeKind::Comment
                    && !(child.kind == NodeKind::Text
                        && child
                            .text
                            .as_deref()
                            .is_some_and(|text| text.trim().is_empty()))
            });
        let body = text.next()?;
        if body.kind != NodeKind::Text || text.next().is_some() {
            return None;
        }
        let span = body.span;
        Some(Self::new(
            owner,
            node,
            span,
            XmlText::text_node(source, span)?,
        ))
    }
}
