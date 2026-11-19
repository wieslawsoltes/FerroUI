use crate::animation::{AnimationStyle, RsxAnimationIssue, projected_styles};
use crate::blocks::{blocks, contains_trivia};
use crate::catalog::Children;
use crate::literal;
use crate::markup::{RsxPreviewMarkup, serialize};
use crate::registered::{Class, RsxCatalog};
use crate::source_edits::RsxEvent;
use crate::states::{self, ProjectedStates};
use crate::tree::{Element, MAX_DEPTH, replacement};
use crate::xaml::XamlTree;
use ferroui_designer_core::{Document, NodeKind, SourceSpan, TextPatch};
use ferroui_view_syntax::{Attribute, Node};
use proc_macro2::TokenStream;
use std::collections::{BTreeMap, BTreeSet};
use syn::ext::IdentExt;
use syn::spanned::Spanned;

/// The original source ranges of one projected control, in XAML element order.
pub struct RsxNode {
    pub span: SourceSpan,
    pub name: SourceSpan,
    pub properties: BTreeMap<String, SourceSpan>,
    pub expressions: BTreeMap<String, SourceSpan>,
    pub events: BTreeMap<String, RsxEvent>,
    pub(crate) fields: BTreeMap<String, SourceSpan>,
    pub(crate) class: Class,
    pub(crate) open: usize,
    pub(crate) close: usize,
    pub(crate) complete_children: bool,
}

impl RsxNode {
    /// Whether a property has a static representation in both source languages.
    pub fn supports_literal(&self, name: &str) -> bool {
        self.class.literal(name).is_some()
    }

    pub fn supports_events(&self) -> bool {
        self.class.mapped_events()
    }

    pub fn editable_children(&self) -> bool {
        self.class.children() != Children::None
            && self.complete_children
            && !self.expressions.contains_key("Content")
    }
}

/// Static markup plus the source anchors needed to edit its original Rust block.
pub struct RsxProjection<'a> {
    pub(crate) source: &'a str,
    pub(crate) catalog: &'a RsxCatalog,
    tree: Element,
    pub nodes: Vec<RsxNode>,
    pub blocks: Vec<SourceSpan>,
    /// The selected invocation, including its name and delimiters.
    pub invocation: SourceSpan,
    /// An incomplete projection can still edit its mapped literals in place.
    pub omissions: Vec<String>,
    pub omitted_count: usize,
    pub animation_issues: Vec<RsxAnimationIssue>,
    inner_trivia: bool,
    styles: BTreeMap<usize, Vec<AnimationStyle>>,
    states: BTreeMap<usize, ProjectedStates>,
}

impl RsxProjection<'_> {
    pub fn xaml(&self) -> String {
        self.preview_markup().source
    }

    pub fn preview_markup(&self) -> RsxPreviewMarkup<'_> {
        serialize(&self.tree, &self.styles, &self.states)
    }

    pub fn has_projected_animations(&self) -> bool {
        !self.styles.is_empty()
    }

    pub fn has_projected_states(&self) -> bool {
        !self.states.is_empty()
    }

    pub fn matches_xaml(&self, source: &str) -> Result<bool, String> {
        let document = Document::new(source).map_err(|error| error.to_string())?;
        Ok(self.tree == XamlTree::parse(&document, self.catalog)?.tree)
    }

    /// Structural conversion can replace a complete tree with no interior comments.
    pub fn can_replace_structure(&self) -> bool {
        self.omitted_count == 0 && !self.inner_trivia && !self.has_projected_animations() && !self.has_projected_states()
    }

    pub fn set_property(&self, index: usize, name: &str, value: &str) -> Result<TextPatch, String> {
        let node = self.nodes.get(index).ok_or("Unknown RSX element")?;
        let span = *node
            .properties
            .get(name)
            .ok_or("Edit an authored literal property")?;
        let replacement = node
            .class
            .literal(name)
            .and_then(|kind| kind.rust(value))
            .ok_or_else(|| format!("{value:?} is outside the static RSX values for {name}"))?;
        Ok(TextPatch { span, replacement })
    }

    /// Apply edited XAML to the selected RSX tree. Changes to existing values keep the
    /// original tokens outside those literals. Structural edits retain the rest of the file.
    pub fn apply_xaml(&self, draft: &str) -> Result<Vec<TextPatch>, String> {
        if self.has_projected_states() {
            return Err("Edit mapped Rust state declarations in States or in the source editor".into());
        }
        if self.has_projected_animations() {
            return Err("Edit mapped animation keys in Timeline and other Rust values in source or Properties".into());
        }
        if self.omitted_count != 0 {
            return Err(
                "Use the property inspector to edit literals in a partial RSX preview".into(),
            );
        }
        let document = Document::new(draft).map_err(|e| e.to_string())?;
        if document.ast().nodes().any(|node| {
            matches!(
                node.kind,
                NodeKind::Comment | NodeKind::ProcessingInstruction
            )
        }) {
            return Err("Keep comments and processing instructions in the authored format".into());
        }
        let draft = XamlTree::parse(&document, self.catalog)?;
        if let Some(changes) = self.tree.changes(&draft.tree) {
            return changes
                .iter()
                .map(|value| self.set_property(value.index, value.name, value.value))
                .collect();
        }
        if self.inner_trivia {
            return Err("Edit RSX structure in the Rust editor to keep its comments".into());
        }
        Ok(vec![replacement(
            self.source,
            self.nodes[0].span,
            &draft.tree.rsx_body(),
        )])
    }
}

/// Selects the innermost `rsx!` enclosing the UTF-8 caret, or the first invocation.
pub fn project_rust(source: &str, caret: usize) -> Result<RsxProjection<'_>, String> {
    RsxCatalog::built_in().project(source, caret)
}

impl RsxCatalog {
    /// Project the selected invocation using the imported registry for its Cargo package.
    pub fn project<'a>(
        &'a self,
        source: &'a str,
        caret: usize,
    ) -> Result<RsxProjection<'a>, String> {
        let blocks = blocks(source)?;
        let block = blocks
            .iter()
            .filter(|b| b.span.start <= caret && caret <= b.span.end)
            .min_by_key(|b| b.span.len())
            .or_else(|| blocks.first())
            .ok_or("Open a Rust file containing a ferroui_view::rsx! invocation")?;
        let stream = block.group.stream();
        let (tree, state) = parse_body(stream.clone(), self)?;
        let inner_trivia = contains_trivia(source, stream, state.nodes[0].span);
        Ok(RsxProjection {
            source,
            catalog: self,
            tree,
            nodes: state.nodes,
            blocks: blocks.iter().map(|block| block.span).collect(),
            invocation: block.span,
            omissions: state.omissions,
            omitted_count: state.omitted_count,
            animation_issues: state.animation_issues,
            inner_trivia,
            styles: state.styles,
            states: state.states,
        })
    }
}

pub(crate) fn parse_draft(source: &str, catalog: &RsxCatalog) -> Result<Element, String> {
    let tokens: TokenStream = source
        .parse()
        .map_err(|e| format!("Invalid RSX tokens: {e}"))?;
    let draft: syn::ExprMacro =
        syn::parse2(tokens.clone()).map_err(|e| format!("Expected an rsx! expression: {e}"))?;
    if draft
        .mac
        .path
        .segments
        .last()
        .is_none_or(|segment| segment.ident != "rsx")
    {
        return Err("Expected an rsx! expression".into());
    }
    let (tree, state) = parse_body(draft.mac.tokens, catalog)?;
    if !state.styles.is_empty() || !state.states.is_empty() {
        return Err("Edit native animation and state declarations in their Rust source".into());
    }
    if state.omitted_count != 0 {
        return Err(format!(
            "The RSX draft must be fully convertible: {}",
            state.omissions.join("; ")
        ));
    }
    if contains_trivia(source, tokens, SourceSpan::new(0, source.len())) {
        return Err("Keep comments in the authored format".into());
    }
    Ok(tree)
}

#[derive(Default)]
struct State {
    nodes: Vec<RsxNode>,
    omissions: Vec<String>,
    omitted_count: usize,
    styles: BTreeMap<usize, Vec<AnimationStyle>>,
    states: BTreeMap<usize, ProjectedStates>,
    animation_issues: Vec<RsxAnimationIssue>,
}

impl State {
    fn omit(&mut self, description: String) {
        self.omitted_count += 1;
        if self.omissions.len() < 8 && !self.omissions.contains(&description) {
            self.omissions.push(description);
        }
    }

    fn element(
        &mut self,
        node: &Node,
        depth: usize,
        catalog: &RsxCatalog,
    ) -> Result<Element, String> {
        if depth > MAX_DEPTH || self.nodes.len() >= 2000 {
            return Err("RSX previews support up to 2000 controls and 96 levels".into());
        }
        let Node::Element(node) = node else {
            return Err("A preview root must be a static FerroUI control".into());
        };
        let name = node.path.span().byte_range();
        let class = catalog
            .rust(&node.path)
            .ok_or("Use a FerroUI control or a registered RSX mapping")?;
        let mut result = Element::new(class.clone());
        let index = self.nodes.len();
        self.nodes.push(RsxNode {
            span: SourceSpan::new(name.start, node.braces.close().byte_range().end),
            name: SourceSpan::new(name.start, name.end),
            properties: BTreeMap::new(),
            expressions: BTreeMap::new(),
            events: BTreeMap::new(),
            fields: BTreeMap::new(),
            class: class.clone(),
            open: node.braces.open().byte_range().end,
            close: node.braces.close().byte_range().start,
            complete_children: true,
        });
        let mut seen = BTreeSet::new();
        let mut state_fields = Vec::new();
        for attribute in &node.attributes {
            let (owner, name, value) = match attribute {
                Attribute::Value {
                    owner, name, value, ..
                } => (owner, name, value),
                Attribute::Call { name, arguments } => {
                    let setter = name.unraw().to_string();
                    if setter.starts_with("on_") {
                        let span = arguments
                            .first()
                            .map_or_else(|| name.span(), Spanned::span)
                            .byte_range();
                        self.nodes[index].events.insert(
                            setter,
                            RsxEvent {
                                value: SourceSpan::new(span.start, span.end),
                                field: None,
                            },
                        );
                    }
                    self.omit(format!("{class} builder call"));
                    continue;
                }
            };
            let setter = name.unraw().to_string();
            let owner = owner.as_ref().map(|owner| owner.unraw().to_string());
            if owner.is_none() && setter == "visual_states" && class.xaml_property(None, &setter).is_none() {
                state_fields.push(value.as_ref());
                continue;
            }
            let value_span = value.span().byte_range();
            let value_span = SourceSpan::new(value_span.start, value_span.end);
            let field = SourceSpan::new(name.span().byte_range().start, value_span.end);
            if owner.is_none() && setter == "styles" && class.xaml_property(None, &setter).is_none() {
                if let Some(projected) = projected_styles(value, catalog) {
                    self.animation_issues.extend(projected.unavailable);
                    if !projected.styles.is_empty() {
                        self.styles.entry(index).or_default().extend(projected.styles);
                        if projected.omitted == 0 { continue; }
                    }
                }
                self.omit(format!("{class}.styles expression"));
                continue;
            }
            if class.mapped_events() && owner.is_none() && setter.starts_with("on_") {
                let field = (!self.nodes[index].events.contains_key(&setter)).then_some(field);
                self.nodes[index].events.insert(
                    setter.clone(),
                    RsxEvent {
                        value: value_span,
                        field,
                    },
                );
                self.omit(format!("{class}.{setter}"));
                continue;
            }
            let Some(key) = class.xaml_property(owner.as_deref(), &setter) else {
                self.omit(format!("{class}.{setter}"));
                continue;
            };
            if !seen.insert(key.clone()) {
                result.properties.remove(&key);
                self.nodes[index].properties.remove(&key);
                self.nodes[index]
                    .expressions
                    .insert(key.clone(), value_span);
                self.omit(format!("Repeated {class}.{key}"));
                continue;
            }
            if let Some(text) = class.literal(&key).and_then(|kind| kind.read_rust(value)) {
                let start = match attribute {
                    Attribute::Value {
                        owner: Some(owner), ..
                    } => owner.span().byte_range().start,
                    _ => field.start,
                };
                self.nodes[index]
                    .fields
                    .insert(key.clone(), SourceSpan::new(start, field.end));
                self.nodes[index].properties.insert(key.clone(), value_span);
                result.properties.insert(key, text);
            } else {
                self.nodes[index]
                    .expressions
                    .insert(key.clone(), value_span);
                self.omit(format!("{class}.{key} expression"));
            }
        }
        let placement = class.children();
        let text_property = class
            .text_property()
            .filter(|name| node.children.len() == 1 && !seen.contains(*name));
        for child in &node.children {
            if let (Some(name), Node::Text(value, _)) = (text_property, child) {
                let span = value.span().byte_range();
                let span = SourceSpan::new(span.start, span.end);
                self.nodes[index].fields.insert(name.into(), span);
                self.nodes[index].properties.insert(name.into(), span);
                result
                    .properties
                    .insert(name.into(), literal::text_value(&value.value()));
                continue;
            }
            match child {
                Node::Element(element)
                    if catalog.rust(&element.path).is_some()
                        && accepts_control(placement, &result) =>
                {
                    result
                        .children
                        .push(self.element(child, depth + 1, catalog)?)
                }
                _ => {
                    self.nodes[index].complete_children = false;
                    self.omit(format!("{class} child"));
                }
            }
        }
        if !state_fields.is_empty() {
            let (groups, issues) = states::project(&state_fields, &result, index, catalog);
            if let Some(groups) = groups {
                self.states.insert(index, groups);
            }
            if !issues.is_empty() {
                self.animation_issues.extend(issues);
                self.omit(format!("{class}.visual_states expression"));
            }
        }
        Ok(result)
    }
}

pub(crate) fn accepts_control(placement: Children, element: &Element) -> bool {
    placement == Children::Panel
        || (matches!(placement, Children::Single | Children::Content)
            && element.children.is_empty()
            && !element.properties.contains_key("Content"))
}

fn parse_body(tokens: TokenStream, catalog: &RsxCatalog) -> Result<(Element, State), String> {
    let nodes = ferroui_view_syntax::parse(tokens).map_err(|e| format!("Invalid RSX: {e}"))?;
    let [root] = nodes.as_slice() else {
        return Err("RSX preview needs one root control; wrap fragments in a panel".into());
    };
    let mut state = State::default();
    let tree = state.element(root, 0, catalog)?;
    Ok((tree, state))
}
