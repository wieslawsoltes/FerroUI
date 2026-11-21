use crate::project::{accepts_control, parse_draft};
use crate::registered::RsxCatalog;
use crate::tree::{Element, MAX_DEPTH, inline_content, replacement};
use ferroui_designer_core::design_fixtures::DESIGN_NAMESPACE;
use ferroui_designer_core::{Document, Node, NodeId, NodeKind, SourceSpan, TextPatch};
use std::collections::BTreeMap;

mod counterpart_properties;

const XAML: &str = "http://schemas.microsoft.com/winfx/2006/xaml";
const COMPATIBILITY: &str = "http://schemas.openxmlformats.org/markup-compatibility/2006";

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
        let tree = XamlTree::authored(document, self)?.tree.rsx_body();
        Ok(format!("rsx! {{\n    {}\n}}", tree.replace('\n', "\n    ")))
    }

    pub fn apply_rsx(&self, document: &Document, draft: &str) -> Result<Vec<TextPatch>, String> {
        let current = XamlTree::authored(document, self)?;
        let next = parse_draft(draft, self)?;
        if let Some(changes) = current.tree.changes(&next) {
            let patches = current.plan_properties(document, changes)?;
            if !patches.is_empty() {
                let source = TextPatch::apply_all(document.source(), patches.clone())
                    .map_err(|error| error.to_string())?;
                let edited = Document::new(source).map_err(|error| error.to_string())?;
                if XamlTree::authored(&edited, self)?.tree != next {
                    return Err("The property edits do not match the authored XAML view".into());
                }
            }
            return Ok(patches);
        }
        if current.preserve_structure {
            return Err("Edit the structure in the XAML editor to keep its comments, processing instructions, and design metadata".into());
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

struct ControlOrigins {
    node: NodeId,
    properties: BTreeMap<String, Origin>,
}

pub(crate) struct XamlTree {
    pub(crate) tree: Element,
    preserve_structure: bool,
    origins: Vec<ControlOrigins>,
}

impl XamlTree {
    pub(crate) fn parse(document: &Document, catalog: &RsxCatalog) -> Result<Self, String> {
        Self::read(document, catalog, false, None)
    }

    pub(crate) fn fragment(
        document: &Document,
        catalog: &RsxCatalog,
        node: NodeId,
    ) -> Result<String, String> {
        let root = document
            .ast()
            .node(node)
            .ok_or("Expected an inserted XAML control")?;
        let tree = ParseTree::new(document, catalog, false, None).element(root, 0)?;
        Ok(tree.rsx_body())
    }

    pub(crate) fn layout(
        document: &Document,
        catalog: &RsxCatalog,
        state_classes: &BTreeMap<NodeId, (&str, bool)>,
    ) -> Result<Self, String> {
        Self::read(document, catalog, false, Some(state_classes))
    }

    fn authored(document: &Document, catalog: &RsxCatalog) -> Result<Self, String> {
        Self::read(document, catalog, true, None)
    }

    fn read(
        document: &Document,
        catalog: &RsxCatalog,
        authored: bool,
        state_classes: Option<&BTreeMap<NodeId, (&str, bool)>>,
    ) -> Result<Self, String> {
        if !document.is_valid() {
            return Err("Repair the XAML syntax before converting the view".into());
        }
        let root = document
            .ast()
            .node(document.ast().root())
            .ok_or("Expected a XAML root")?;
        let mut state = ParseTree::new(document, catalog, authored, state_classes);
        let tree = state.element(root, 0)?;
        Ok(Self {
            tree,
            origins: state.origins,
            preserve_structure: state.preserve_structure,
        })
    }
}

struct ParseTree<'a> {
    document: &'a Document,
    catalog: &'a RsxCatalog,
    origins: Vec<ControlOrigins>,
    authored: bool,
    state_classes: Option<&'a BTreeMap<NodeId, (&'a str, bool)>>,
    preserve_structure: bool,
}

impl<'a> ParseTree<'a> {
    fn new(
        document: &'a Document,
        catalog: &'a RsxCatalog,
        authored: bool,
        state_classes: Option<&'a BTreeMap<NodeId, (&'a str, bool)>>,
    ) -> Self {
        Self {
            document,
            catalog,
            origins: Vec::new(),
            authored,
            state_classes,
            preserve_structure: false,
        }
    }

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
            if self.authored
                && (attr.namespace_uri.as_deref() == Some(DESIGN_NAMESPACE)
                    || (attr.namespace_uri.as_deref() == Some(COMPATIBILITY)
                        && attr.name.ends_with(":Ignorable")
                        && attr.value.split_whitespace().all(|prefix| {
                            self.document
                                .ast()
                                .namespaces(node.id)
                                .get(prefix)
                                .is_some_and(|namespace| namespace == DESIGN_NAMESPACE)
                        })))
            {
                self.preserve_structure = true;
                continue;
            }
            let name = match attr.namespace_uri.as_deref() {
                None => attr.name.as_str(),
                Some(XAML) if attr.name.ends_with(":Name") => "Name",
                _ => return Err(format!("{} has no static RSX conversion", attr.name)),
            };
            let mut value = attr.value.as_str();
            if name == "Classes" {
                if let Some(&(marker, authored)) =
                    self.state_classes.and_then(|classes| classes.get(&node.id))
                {
                    value = value
                        .strip_suffix(marker)
                        .ok_or("The generated state scope changed")?
                        .trim_end();
                    if !authored && value.is_empty() {
                        continue;
                    }
                }
            }
            let value = class
                .literal(name)
                .and_then(|kind| kind.normalize(value))
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
        self.origins.push(ControlOrigins { node: node.id, properties: origins });
        let placement = class.children();
        let text_property = class.text_property();
        for child in node
            .children
            .iter()
            .filter_map(|id| self.document.ast().node(*id))
        {
            if self.authored && child.namespace_uri.as_deref() == Some(DESIGN_NAMESPACE) {
                self.preserve_structure = true;
                continue;
            }
            match child.kind {
                NodeKind::Element if accepts_control(placement, &element) => {
                    element.children.push(self.element(child, depth + 1)?);
                }
                NodeKind::Comment | NodeKind::ProcessingInstruction => {
                    self.preserve_structure = true
                }
                NodeKind::PropertyElement
                    if self.state_classes.is_some()
                        && child.namespace_uri == node.namespace_uri
                        && (child.local_name().split_once('.') == Some((node.local_name(), "Styles"))
                            || matches!(&class, crate::registered::Class::Builtin(name) if name == "TableView")
                                && child.local_name() == "TableView.Columns") =>
                {}
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
                    self.origins[index].properties.insert(
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
