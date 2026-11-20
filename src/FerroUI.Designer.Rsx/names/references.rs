use super::literals::rewrite_literal;
use crate::{
    animation::syntax::{argument, chain, span, view_argument},
    tree::MAX_DEPTH,
    RsxCatalog,
};
use ferroui_designer_core::{SourceSpan, TextPatch};
use ferroui_markup::markup::parsers::selector_grammar::{SelectorGrammar, SelectorSyntax};
use ferroui_view_syntax::{Attribute, Node};
use std::ops::Range;
use syn::{ext::IdentExt, Expr, Ident, Lit, LitStr};

const STYLE_SOURCE: &str =
    "Edit the computed style selector in Rust source before changing this name";
const STATE_SOURCE: &str =
    "Edit the computed visual-state target in Rust source before changing this name";

pub(super) struct NameReferences<'a> {
    source: &'a str,
    catalog: &'a RsxCatalog,
    target: SourceSpan,
    declaration: Option<SourceSpan>,
    previous: Option<&'a str>,
    next: Option<&'a str>,
    patches: Vec<TextPatch>,
}

impl<'a> NameReferences<'a> {
    pub(super) fn new(
        source: &'a str,
        catalog: &'a RsxCatalog,
        target: SourceSpan,
        declaration: Option<SourceSpan>,
        previous: Option<&'a str>,
        next: Option<&'a str>,
    ) -> Self {
        Self {
            source,
            catalog,
            target,
            declaration,
            previous,
            next,
            patches: Vec::new(),
        }
    }

    pub(super) fn into_patches(self) -> Vec<TextPatch> {
        self.patches
    }

    pub(super) fn visit(&mut self, nodes: &[Node], depth: usize) -> Result<(), String> {
        if depth > MAX_DEPTH {
            return Err("The RSX name scan exceeds 96 levels".into());
        }
        for node in nodes {
            match node {
                Node::Element(node) => {
                    let owner = SourceSpan::new(
                        span(&node.path).start,
                        node.braces.close().byte_range().end,
                    );
                    let class = self.catalog.rust(&node.path);
                    for attribute in &node.attributes {
                        let Some((setter, value)) = authored_field(attribute) else {
                            continue;
                        };
                        let setter = setter.unraw().to_string();
                        let name = class.as_ref().map_or(setter == "name", |class| {
                            class.xaml_property(None, &setter).as_deref() == Some("Name")
                        });
                        if name {
                            self.check_declaration(owner, value)?;
                        }
                        if self.previous.is_some() || self.next.is_some() {
                            self.inspect_field(owner, &setter, value)?;
                        }
                    }
                    self.visit(&node.children, depth + 1)?;
                }
                Node::For(_, _, body, _) => self.visit(body, depth + 1)?,
                Node::If(_, body, otherwise, _) => {
                    self.visit(body, depth + 1)?;
                    if let Some(otherwise) = otherwise {
                        self.visit(otherwise, depth + 1)?;
                    }
                }
                Node::Expr(..) | Node::Text(..) => {}
            }
        }
        Ok(())
    }

    fn check_declaration(&self, owner: SourceSpan, value: &Expr) -> Result<(), String> {
        if owner == self.target {
            if self.declaration != Some(span(value)) {
                return Err("Edit the computed or repeated control name in Rust source".into());
            }
        } else if let Some(literal) = text(value) {
            let name = literal.value();
            if self.next == Some(&name) {
                return Err(format!(
                    "Another control in this RSX invocation already uses {name:?}"
                ));
            }
            if self.previous == Some(&name) {
                return Err(format!(
                    "{name:?} has more than one declaration in this RSX invocation"
                ));
            }
        }
        Ok(())
    }

    fn inspect_field(
        &mut self,
        owner: SourceSpan,
        setter: &str,
        expression: &Expr,
    ) -> Result<(), String> {
        let owns_target = encloses(owner, self.target);
        let target_is_ancestor = encloses(self.target, owner) && !owns_target;
        match setter {
            "styles" if owns_target || target_is_ancestor => {
                for style in array(expression).ok_or(STYLE_SOURCE)? {
                    self.style(style, owns_target, SelectorScope::View)?;
                }
            }
            "visual_states" if owns_target && owner != self.target => {
                for group in array(expression).ok_or(STATE_SOURCE)? {
                    self.group(group)?;
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn style(
        &mut self,
        expression: &Expr,
        owns_target: bool,
        scope: SelectorScope,
    ) -> Result<(), String> {
        // Nested selectors below a template part live in that template's own subtree.
        if scope == SelectorScope::Template {
            return Ok(());
        }
        let (factory, methods) = chain(unwrapped(expression)).ok_or(STYLE_SOURCE)?;
        let literal = view_argument(factory, "style")
            .and_then(text)
            .ok_or(STYLE_SOURCE)?;
        let value = literal.value();
        let (syntax, names) = SelectorGrammar::name_ranges(&value)
            .map_err(|error| format!("Repair the style selector in Rust source: {error}"))?;
        let mut edits = Vec::new();
        for name in names {
            if name.in_template || (!owns_target && !name.ancestor) {
                continue;
            }
            if self.reference(&value[name.range.clone()], "style selector")? {
                if scope == SelectorScope::Mixed {
                    return Err("Split the style's view and template alternatives before changing this name".into());
                }
                edits.push(name.range);
            }
        }
        self.add_patches(literal, &value, edits)?;
        let child_scope = scope.children(&syntax);
        for method in methods {
            if let Some(nested) = argument(method, "nested") {
                self.style(nested, owns_target, child_scope)?;
            }
        }
        Ok(())
    }

    fn group(&mut self, expression: &Expr) -> Result<(), String> {
        let (factory, methods) = chain(unwrapped(expression)).ok_or(STATE_SOURCE)?;
        view_argument(factory, "visual_state_group").ok_or(STATE_SOURCE)?;
        for state in methods
            .into_iter()
            .filter_map(|method| argument(method, "state"))
        {
            let (factory, methods) = chain(unwrapped(state)).ok_or(STATE_SOURCE)?;
            view_argument(factory, "visual_state").ok_or(STATE_SOURCE)?;
            for method in methods
                .into_iter()
                .filter(|method| method.method == "set_on")
            {
                let literal = method.args.first().and_then(text).ok_or(STATE_SOURCE)?;
                let value = literal.value();
                if self.reference(&value, "visual state")? {
                    self.add_patches(literal, &value, std::iter::once(0..value.len()))?;
                }
            }
        }
        Ok(())
    }

    fn reference(&self, value: &str, kind: &str) -> Result<bool, String> {
        if self.next == Some(value) {
            return Err(format!(
                "The new name {value:?} would capture an existing {kind} reference"
            ));
        }
        if self.previous != Some(value) {
            return Ok(false);
        }
        if self.next.is_none() {
            return Err(format!("{value:?} is referenced by a {kind}. Rename it or update the reference before resetting the name"));
        }
        Ok(true)
    }

    fn add_patches(
        &mut self,
        literal: &LitStr,
        value: &str,
        ranges: impl IntoIterator<Item = Range<usize>>,
    ) -> Result<(), String> {
        let mut ranges = ranges.into_iter().peekable();
        if let Some(next) = self.next.filter(|_| ranges.peek().is_some()) {
            self.patches.extend(rewrite_literal(
                self.source,
                span(literal),
                value,
                ranges,
                next,
            )?);
        }
        Ok(())
    }
}

/// A nested style inherits the template boundary reached by its parent selector.
#[derive(Clone, Copy, Eq, PartialEq)]
enum SelectorScope {
    View,
    Template,
    Mixed,
}

impl SelectorScope {
    fn children(self, syntax: &[SelectorSyntax]) -> Self {
        let (mut view, mut template) = (false, false);
        for alternative in syntax.split(|node| matches!(node, SelectorSyntax::Comma)) {
            if alternative
                .iter()
                .any(|node| matches!(node, SelectorSyntax::Template))
            {
                template = true;
            } else {
                view = true;
            }
        }
        match (view, template, self) {
            (false, true, _) => Self::Template,
            (true, true, _) => Self::Mixed,
            _ => self,
        }
    }
}

fn encloses(owner: SourceSpan, child: SourceSpan) -> bool {
    owner.start <= child.start && child.end <= owner.end
}

fn authored_field(attribute: &Attribute) -> Option<(&Ident, &Expr)> {
    match attribute {
        Attribute::Value {
            owner: None,
            name,
            value,
            ..
        } => Some((name, value)),
        Attribute::Call { name, arguments } if arguments.len() == 1 => Some((name, &arguments[0])),
        _ => None,
    }
}

fn array(expression: &Expr) -> Option<&syn::punctuated::Punctuated<Expr, syn::Token![,]>> {
    let Expr::Array(array) = unwrapped(expression) else {
        return None;
    };
    Some(&array.elems)
}

fn text(expression: &Expr) -> Option<&LitStr> {
    let Expr::Lit(value) = unwrapped(expression) else {
        return None;
    };
    let Lit::Str(value) = &value.lit else {
        return None;
    };
    Some(value)
}

fn unwrapped(mut expression: &Expr) -> &Expr {
    loop {
        expression = match expression {
            Expr::Group(value) => &value.expr,
            Expr::Paren(value) => &value.expr,
            Expr::Reference(value) => &value.expr,
            _ => return expression,
        };
    }
}
