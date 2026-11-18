use crate::project::{accepts_control, parse_draft};
use crate::registered::RsxCatalog;
use crate::tree::{Element, MAX_DEPTH, escape_xml, inline_content, replacement};
use ferroui_designer_core::{Document, Node, NodeKind, SourceSpan, TextPatch};
use std::collections::BTreeMap;

const XAML: &str = "http://schemas.microsoft.com/winfx/2006/xaml";

/// Produce an RSX expression with a statically representable view's controls and literals.
pub fn xaml_to_rsx(document: &Document) -> Result<String, String> {
    RsxCatalog::built_in().xaml_to_rsx(document)
}

/// Apply an edited RSX expression to XAML. Literal replacements leave unaffected source
/// bytes intact, including XML quoting, namespaces, whitespace and comments.
pub fn apply_rsx(document: &Document, draft: &str) -> Result<Vec<TextPatch>, String> {
    RsxCatalog::built_in().apply_rsx(document, draft)
}

impl RsxCatalog {
    pub fn xaml_to_rsx(&self, document: &Document) -> Result<String, String> {
        let tree = XamlTree::parse(document, self)?.tree.rsx_body();
        Ok(format!("rsx! {{\n    {}\n}}", tree.replace('\n', "\n    ")))
    }

    pub fn apply_rsx(&self, document: &Document, draft: &str) -> Result<Vec<TextPatch>, String> {
        let current = XamlTree::parse(document, self)?;
        let next = parse_draft(draft, self)?;
        if let Some(changes) = current.tree.changes(&next) {
            return changes
                .iter()
                .map(|change| {
                    let origin = current.origins[change.index][change.name];
                    let value = if origin.quote.is_none() {
                        inline_content(change.value).ok_or_else(|| {
                            format!("Use a {} attribute for this value", change.name)
                        })?
                    } else {
                        change.value.to_owned()
                    };
                    Ok(TextPatch {
                        span: origin.span,
                        replacement: escape_xml(&value, origin.quote),
                    })
                })
                .collect();
        }
        if current.inner_trivia {
            return Err("Edit the structure in the XAML editor to keep its comments and processing instructions".into());
        }
        let span = document
            .ast()
            .node(document.ast().root())
            .expect("valid root")
            .span;
        Ok(vec![replacement(document.source(), span, &next.xaml())])
    }
}

#[derive(Clone, Copy)]
struct Origin {
    span: SourceSpan,
    quote: Option<char>,
}

pub(crate) struct XamlTree {
    pub(crate) tree: Element,
    pub(crate) inner_trivia: bool,
    origins: Vec<BTreeMap<String, Origin>>,
}

impl XamlTree {
    pub(crate) fn parse(document: &Document, catalog: &RsxCatalog) -> Result<Self, String> {
        if !document.is_valid() {
            return Err("Repair the XAML syntax before converting the view".into());
        }
        let root = document
            .ast()
            .node(document.ast().root())
            .ok_or("Expected a XAML root")?;
        let mut state = ParseTree {
            document,
            catalog,
            origins: Vec::new(),
            inner_trivia: false,
        };
        let tree = state.element(root, 0)?;
        Ok(Self {
            tree,
            origins: state.origins,
            inner_trivia: state.inner_trivia,
        })
    }
}

struct ParseTree<'a> {
    document: &'a Document,
    catalog: &'a RsxCatalog,
    origins: Vec<BTreeMap<String, Origin>>,
    inner_trivia: bool,
}

impl ParseTree<'_> {
    fn element(&mut self, node: &Node, depth: usize) -> Result<Element, String> {
        if depth > MAX_DEPTH || self.origins.len() >= 2000 {
            return Err("RSX conversion supports up to 2000 controls and 96 levels".into());
        }
        let class = self
            .catalog
            .xaml(node.local_name(), node.namespace_uri.as_deref())
            .filter(|_| node.kind == NodeKind::Element)
            .ok_or_else(|| format!("{} has no static RSX conversion", node.name))?;
        let mut element = Element::new(class.clone());
        let mut origins = BTreeMap::new();
        for attr in &node.attributes {
            if attr.is_namespace() {
                continue;
            }
            let name = match attr.namespace_uri.as_deref() {
                None => attr.name.as_str(),
                Some(XAML) if attr.name.ends_with(":Name") => "Name",
                _ => return Err(format!("{} has no static RSX conversion", attr.name)),
            };
            let value = class
                .literal(name)
                .and_then(|kind| kind.normalize(&attr.value))
                .ok_or_else(|| format!("{class}.{name} needs a supported static RSX literal"))?;
            element.properties.insert(name.into(), value);
            if origins
                .insert(
                    name.into(),
                    Origin {
                        span: attr.value_span,
                        quote: Some(attr.quote),
                    },
                )
                .is_some()
            {
                return Err(format!("Repeated {class}.{name}"));
            }
        }
        let index = self.origins.len();
        self.origins.push(origins);
        let placement = class.children();
        let text_property = class.text_property();
        for child in node
            .children
            .iter()
            .filter_map(|id| self.document.ast().node(*id))
        {
            match child.kind {
                NodeKind::Element if accepts_control(placement, &element) => {
                    element.children.push(self.element(child, depth + 1)?);
                }
                NodeKind::Comment | NodeKind::ProcessingInstruction => self.inner_trivia = true,
                NodeKind::Text if child.text.as_deref().unwrap_or("").trim().is_empty() => (),
                NodeKind::Text => {
                    let name = text_property
                        .filter(|name| {
                            element.children.is_empty() && !element.properties.contains_key(*name)
                        })
                        .ok_or_else(|| {
                            format!("{class} contains mixed content outside the static RSX tree")
                        })?;
                    let value =
                        child
                            .text
                            .as_deref()
                            .and_then(inline_content)
                            .ok_or_else(|| {
                                format!("Use a {name} attribute to convert mixed or multiline text")
                            })?;
                    element.properties.insert(name.into(), value);
                    self.origins[index].insert(
                        name.into(),
                        Origin {
                            span: child.span,
                            quote: None,
                        },
                    );
                }
                _ => {
                    return Err(format!(
                        "{class} contains content outside the static RSX tree"
                    ));
                }
            }
        }
        Ok(element)
    }
}
