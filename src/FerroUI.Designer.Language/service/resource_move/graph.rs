//! Check authored resource visibility across the original and proposed project graphs.
use super::{Entry, LanguageService, Occurrence, SymbolKind, source::Draft, tool_error};
use crate::{LanguageError, Result, require};
use ferroui_designer_core::{Document, MarkupValue, Node, NodeId};
use ferroui_designer_tools::{
    ComponentResourceContext, ComponentResourceSources, IncludedResourceDocument,
    ScopedResourceResolution, component_property_attribute, component_theme_key,
    scoped_resource_resolution,
};
use std::collections::BTreeSet;

pub(super) fn reference_target(
    service: &LanguageService,
    uri: &str,
    occurrence: &Occurrence,
) -> Result<(String, Occurrence)> {
    let ambiguous = || {
        LanguageError(
            "Open the exact authored resource declaration, including its theme layer".into(),
        )
    };
    let sources = Sources(service);
    let lookup = sources.lookup(uri, occurrence.node, &occurrence.name, None)?;
    let [candidate] = lookup.candidates.as_slice() else {
        return Err(ambiguous());
    };
    let target = candidate
        .uri
        .filter(|_| !lookup.external)
        .ok_or_else(ambiguous)?;
    let definition = service
        .state(target)?
        .symbols
        .iter()
        .find(|symbol| {
            symbol.declaration
                && symbol.kind == SymbolKind::Resource
                && symbol.name == occurrence.name
                && symbol.node == candidate.node
        })
        .ok_or_else(ambiguous)?;
    Ok((target.into(), definition.clone()))
}

pub(super) fn require_relocated_includes_acyclic(draft: &Draft) -> Result<()> {
    if draft.from == draft.to {
        return Ok(());
    }
    let state = draft.service.state(&draft.to)?;
    let ast = state.document.ast();
    for edge in state.resource_edges.values().flatten().filter(|edge| {
        edge.node == draft.destination_node
            || ast.is_descendant_of(edge.node, draft.destination_node)
    }) {
        if let Some(source) = &edge.source {
            let uri = draft.service.resource_uri(&draft.to, source)?;
            require(
                !include_closure(&draft.service, &uri)?.contains(draft.to.as_str()),
                "The relocated resource would create an include cycle",
            )?;
        }
    }
    Ok(())
}

pub(super) fn check_destination_link(draft: &Draft, entry: &Entry) -> Result<()> {
    let included = include_closure(&draft.service, &draft.to)?;
    require(
        !included.contains(entry.uri.as_str()),
        "Linking the destination would create a resource include cycle",
    )?;
    let state = draft.service.state(&entry.uri)?;
    let document = &state.document;
    let ast = document.ast();
    let sources = Sources(&draft.service);
    let already_linked = state.resource_edges.values().flatten().any(|edge| {
        let Some(node) = ast.node(edge.node) else {
            return false;
        };
        let Some(parent) = node.parent.and_then(|id| ast.node(id)) else {
            return false;
        };
        let in_scope = parent.id == entry.scope.container
            || (parent.parent == Some(entry.scope.container)
                && super::super::dictionary_property(document, parent, "MergedDictionaries"));
        in_scope
            && sources
                .include(&entry.uri, document, node)
                .is_some_and(|include| include.uri == draft.to)
    });
    require(
        !already_linked,
        "The original scope already links this dictionary and the move would change resource precedence",
    )
}

pub(super) fn validate(
    original: &LanguageService,
    draft: &Draft,
    entry: &Entry,
    linked: bool,
) -> Result<()> {
    let next = &draft.service;
    let mut keys = BTreeSet::from([entry.declaration.name.as_str()]);
    let source = original.state(&entry.uri)?;
    keys.extend(
        source
            .symbols
            .iter()
            .filter(|symbol| {
                symbol.declaration
                    && symbol.kind == SymbolKind::Resource
                    && draft.moved.contains_key(&symbol.node)
            })
            .map(|symbol| symbol.name.as_str()),
    );
    if linked {
        for uri in include_closure(next, &draft.to)? {
            keys.extend(
                next.documents[uri]
                    .symbols
                    .iter()
                    .filter(|symbol| symbol.declaration && symbol.kind == SymbolKind::Resource)
                    .map(|symbol| symbol.name.as_str()),
            );
        }
    }
    let root = original.workspace_root(&entry.uri);
    let mut themes = BTreeSet::from(["Default".to_owned(), "Light".to_owned(), "Dark".to_owned()]);
    for (uri, state) in &original.documents {
        if original.workspace_root(uri) != root {
            continue;
        }
        for edge in state
            .resource_edges
            .values()
            .flatten()
            .filter(|edge| edge.kind != super::super::ResourceEdgeKind::Collection)
        {
            if let Some(theme) = state
                .document
                .ast()
                .node(edge.node)
                .and_then(|node| component_theme_key(&state.document, node))
            {
                themes.insert(theme);
            }
        }
    }
    let contexts = std::iter::once(None)
        .chain(themes.into_iter().map(|theme| {
            Some(ComponentResourceContext {
                theme: Some(theme),
                inherited_themes: Vec::new(),
            })
        }))
        .collect::<Vec<_>>();
    let proof = Proof {
        original: Sources(original),
        candidate: Sources(next),
        draft,
        contexts,
    };
    // Preserve the dictionary's original entry point even if external callers are not
    // indexed. For example, an app can load this dictionary from Rust.
    proof.check(&entry.uri, entry.scope.owner, &entry.declaration.name)?;

    for (uri, state) in &original.documents {
        if original.workspace_root(uri) != root {
            continue;
        }
        if !state.document.is_valid() {
            super::super::source_review::check_incomplete(uri, state, |text| {
                keys.iter().any(|key| text.contains(key))
            })?;
            continue;
        }
        let uses = state
            .symbols
            .iter()
            .filter(|symbol| {
                !symbol.declaration
                    && symbol.kind == SymbolKind::Resource
                    && (keys.contains(symbol.name.as_str())
                        || (*uri == entry.uri && draft.moved.contains_key(&symbol.node)))
            })
            .map(|symbol| (symbol.node, symbol.name.as_str()))
            .collect::<BTreeSet<_>>();
        for (node, key) in uses {
            proof.check(uri, node, key)?;
        }
    }
    Ok(())
}

struct Proof<'a> {
    original: Sources<'a>,
    candidate: Sources<'a>,
    draft: &'a Draft,
    contexts: Vec<Option<ComponentResourceContext>>,
}

impl Proof<'_> {
    fn check(&self, uri: &str, node: NodeId, key: &str) -> Result<()> {
        let (destination, relocated) = self.draft.map_node(uri, node);
        for context in &self.contexts {
            let before = self.original.lookup(uri, node, key, context.as_ref())?;
            let after = self
                .candidate
                .lookup(destination, relocated, key, context.as_ref())?;
            if before.external || after.external {
                return Err(LanguageError(format!(
                    "An unresolved resource provider prevents checking '{key}' in {uri}"
                )));
            }
            let previous = before
                .candidates
                .iter()
                .map(|item| {
                    self.draft
                        .map_node(item.uri.unwrap_or(item.address), item.node)
                })
                .collect::<BTreeSet<_>>();
            let proposed = after
                .candidates
                .iter()
                .map(|item| (item.uri.unwrap_or(item.address), item.node))
                .collect::<BTreeSet<_>>();
            if previous != proposed || before.may_be_missing != after.may_be_missing {
                let theme = context
                    .as_ref()
                    .and_then(|context| context.theme.as_deref());
                let theme = theme.map_or_else(String::new, |name| format!(" under {name}"));
                return Err(LanguageError(format!(
                    "Moving the resource would change the resolution of '{key}'{theme} in {uri}"
                )));
            }
        }
        Ok(())
    }
}

/// The language service owns indexed buffers and compiled path identities. The portable
/// resolver owns strict lexical lookup and the order of theme and merged providers.
struct Sources<'a>(&'a LanguageService);

impl Sources<'_> {
    fn lookup(
        &self,
        uri: &str,
        node: NodeId,
        key: &str,
        context: Option<&ComponentResourceContext>,
    ) -> Result<ScopedResourceResolution<'_>> {
        let (address, state) = self
            .0
            .documents
            .get_key_value(uri)
            .ok_or_else(|| LanguageError(format!("Resource document is not indexed: {uri}")))?;
        scoped_resource_resolution(&state.document, address, node, key, context, self)
            .map_err(tool_error)
    }
}

impl ComponentResourceSources for Sources<'_> {
    fn include(
        &self,
        address: &str,
        document: &Document,
        node: &Node,
    ) -> Option<IncludedResourceDocument<'_>> {
        let name = node.local_name();
        if !matches!(name, "ResourceInclude" | "MergeResourceInclude")
            || !super::super::native_resource_type(name, node.namespace_uri.as_deref())
        {
            return None;
        }
        let source = component_property_attribute(document, node.id, "Source").ok()??;
        let MarkupValue::Literal(value) = source.markup().ok()? else {
            return None;
        };
        let uri = self.0.resource_uri(address, &value).ok()?;
        let (uri, state) = self.0.documents.get_key_value(&uri)?;
        state
            .document
            .is_valid()
            .then_some(IncludedResourceDocument {
                address: uri,
                uri,
                document: &state.document,
            })
    }
}

fn include_closure<'a>(service: &'a LanguageService, root: &str) -> Result<BTreeSet<&'a str>> {
    let (root, _) = service
        .documents
        .get_key_value(root)
        .ok_or_else(|| LanguageError("The destination dictionary is not indexed".into()))?;
    let mut checked = BTreeSet::new();
    let mut pending = vec![root.as_str()];
    while let Some(uri) = pending.pop() {
        if !checked.insert(uri) {
            continue;
        }
        let state = service.state(uri)?;
        for edge in state.resource_edges.values().flatten() {
            if let Some(source) = &edge.source {
                let target = service.resource_uri(uri, source)?;
                let (file, state) = service.documents.get_key_value(&target).ok_or_else(|| {
                    LanguageError(format!(
                        "Index the included dictionary before linking this destination: {source}"
                    ))
                })?;
                require(
                    state.document.is_valid(),
                    format!("Repair the included dictionary before linking it: {file}"),
                )?;
                pending.push(file);
            }
        }
    }
    Ok(checked)
}
