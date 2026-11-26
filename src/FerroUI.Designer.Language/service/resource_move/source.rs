//! Draft the two source buffers and retain the identities needed for lookup validation.
use super::{Entry, LanguageService, State, SymbolKind, tool_error};
use crate::{LanguageError, LineIndex, Result, require};
use ferroui_designer_core::{Document, Edit, NodeId, TextPatch};
use ferroui_designer_project::relative_file_reference;
use ferroui_designer_tools::{
    AuthoredResourceScope, link_resource_dictionary, resource_move_fragment,
};
use std::collections::BTreeMap;

pub(super) struct Draft {
    pub service: LanguageService,
    pub from: String,
    pub to: String,
    pub destination_node: NodeId,
    pub moved: BTreeMap<NodeId, NodeId>,
}

impl Draft {
    pub fn prepare(
        service: &LanguageService,
        entry: &Entry,
        destination: &str,
        scope: AuthoredResourceScope,
    ) -> Result<Self> {
        let from = service.state(&entry.uri)?;
        let to = service.state(destination)?;
        let node = entry.declaration.node;
        let original = &from.document;
        let target = &to.document;
        let old = original.ast().node(node).expect("indexed resource node");
        let ancestry_changed = entry.uri != destination
            || original.ast().name_scope(node) != target.ast().name_scope(scope.container);
        if ancestry_changed {
            service.check_moved_context(&entry.uri, original, node, old.span)?;
        }
        check_base(original, node, true)?;
        check_base(target, scope.container, false)?;
        require(
            !to.symbols.iter().any(|other| {
                other.kind == SymbolKind::Resource
                    && other.declaration
                    && other.name == entry.declaration.name
                    && target.ast().node(other.node).and_then(|node| node.parent)
                        == Some(scope.container)
            }),
            format!(
                "The destination already defines '{}'",
                entry.declaration.name
            ),
        )?;
        let fragment = resource_move_fragment(original, node, target, scope).map_err(tool_error)?;
        let fragment = rebase_paths(service, &fragment, &entry.uri, destination)?;

        let label = "Move resource declaration";
        let mut origin_draft = original.snapshot_without_history();
        origin_draft.apply(label, vec![Edit::RemoveNode { node }])?;
        let same = entry.uri == destination;
        let mut target_draft = if same {
            origin_draft.snapshot_without_history()
        } else {
            target.snapshot_without_history()
        };
        let children = target_draft.ast().element_children(scope.container);
        target_draft.apply(
            label,
            vec![Edit::InsertChild {
                parent: scope.container,
                index: children.len(),
                source: fragment,
            }],
        )?;
        let destination_node = target_draft
            .ast()
            .element_children(scope.container)
            .last()
            .copied()
            .ok_or_else(|| {
                LanguageError("The destination did not retain the inserted resource".into())
            })?;
        let original_nodes = original
            .ast()
            .subtree(node)
            .filter(|node| node.is_element());
        let target_nodes = target_draft
            .ast()
            .subtree(destination_node)
            .filter(|node| node.is_element());
        let original_nodes = original_nodes.collect::<Vec<_>>();
        let target_nodes = target_nodes.collect::<Vec<_>>();
        require(
            original_nodes.len() == target_nodes.len()
                && original_nodes
                    .iter()
                    .zip(&target_nodes)
                    .all(|(left, right)| {
                        left.name == right.name
                            && left.namespace_uri == right.namespace_uri
                            && left.kind == right.kind
                    }),
            "The destination changed the resource's authored element identities",
        )?;
        let moved = original_nodes
            .into_iter()
            .zip(target_nodes)
            .map(|(left, right)| (left.id, right.id))
            .collect();
        let mut service = service.clone();
        if !same {
            install(&mut service, &entry.uri, from, origin_draft);
        }
        install(&mut service, destination, to, target_draft);
        Ok(Self {
            service,
            from: entry.uri.clone(),
            to: destination.into(),
            destination_node,
            moved,
        })
    }

    pub fn can_link(&self, entry: &Entry, destination: AuthoredResourceScope) -> bool {
        entry.uri != self.to
            && self.service.documents[&self.to].document.ast().root() == destination.container
    }

    pub fn link(
        &self,
        original: &LanguageService,
        entry: &Entry,
        highest_priority: bool,
    ) -> Result<Self> {
        let reference = relative_file_reference(&self.from, &self.to)?;
        require(
            original.resource_uri(&self.from, &reference)? == self.to,
            "The destination has no matching relative XAML include path",
        )?;
        if let Some(resources) = original.compiled_resources(&self.from) {
            require(
                !resources.contains_source(&self.from)
                    || resources.resolve(&self.from, &reference)? == Some(self.to.as_str()),
                "Register the destination in the source dictionary's compiled XAML group before moving this resource",
            )?;
        }
        let state = self.service.state(&self.from)?;
        let mut document = state.document.snapshot_without_history();
        link_resource_dictionary(&mut document, entry.scope, &reference, highest_priority)
            .map_err(tool_error)?;
        let mut service = self.service.clone();
        install(&mut service, &self.from, state, document);
        Ok(Self {
            service,
            from: self.from.clone(),
            to: self.to.clone(),
            destination_node: self.destination_node,
            moved: self.moved.clone(),
        })
    }

    pub fn documents(&self) -> impl Iterator<Item = (&str, &Document)> {
        std::iter::once(self.from.as_str())
            .chain((self.from != self.to).then_some(self.to.as_str()))
            .map(|uri| (uri, &self.service.documents[uri].document))
    }

    pub fn map_node<'a>(&'a self, uri: &'a str, node: NodeId) -> (&'a str, NodeId) {
        if uri == self.from {
            if let Some(&moved) = self.moved.get(&node) {
                return (&self.to, moved);
            }
        }
        (uri, node)
    }
}

fn install(service: &mut LanguageService, uri: &str, previous: &State, document: Document) {
    let index = LineIndex::new(document.source());
    service.set_state(
        uri.into(),
        Some(State::indexed(
            document,
            index,
            previous.version,
            previous.opened,
        )),
    );
}

fn check_base(document: &Document, node: NodeId, subtree: bool) -> Result<()> {
    let ast = document.ast();
    let with_base = |node: &ferroui_designer_core::Node| {
        node.attributes.iter().any(|attr| attr.name == "xml:base")
    };
    let mut ancestry = std::iter::successors(ast.node(node), |node| {
        node.parent.and_then(|id| ast.node(id))
    });
    require(
        !(ancestry.any(with_base) || (subtree && ast.subtree(node).any(with_base))),
        "An XML base URI prevents checking relative paths for this resource move",
    )
}

fn rebase_paths(
    service: &LanguageService,
    fragment: &Document,
    from: &str,
    to: &str,
) -> Result<String> {
    if from == to {
        return Ok(fragment.source().into());
    }
    let compiled = service.compiled_resources(to);
    let mut patches = Vec::new();
    for site in super::super::file_move_literals::paths(fragment, service.schema_for(from)) {
        let suffix = if site.value.starts_with("resm:") {
            &['#'][..]
        } else {
            &['?', '#'][..]
        };
        let split = site.value.find(suffix).unwrap_or(site.value.len());
        let (path, suffix) = site.value.split_at(split);
        if path.is_empty() {
            continue;
        }
        if absolute_address(path) && !path.starts_with("resm:") && !path.starts_with("ferres:") {
            continue;
        }
        let resolved = service.resource_uri(from, path)?;
        if service
            .resource_uri(to, path)
            .is_ok_and(|candidate| candidate == resolved)
        {
            continue;
        }
        require(
            resolved.starts_with("file:"),
            format!("Cannot preserve the resource address '{path}' at its destination"),
        )?;
        let reference = relative_file_reference(to, &resolved)?;
        if let Some(resources) = &compiled {
            require(
                !resources.contains_source(to)
                    || resources.resolve(to, &reference)? == Some(resolved.as_str()),
                format!("The destination's compiled XAML group cannot resolve '{path}'"),
            )?;
        }
        require(
            service.resource_uri(to, &reference)? == resolved,
            format!("The resource address '{path}' changes meaning at the destination"),
        )?;
        let replacement = format!("{reference}{suffix}");
        patches.extend(site.patches(&replacement));
    }
    Ok(TextPatch::apply_all(fragment.source(), patches)?)
}

fn absolute_address(path: &str) -> bool {
    path.split_once(':').is_some_and(|(scheme, _)| {
        !scheme.is_empty()
            && scheme.bytes().enumerate().all(|(index, byte)| {
                byte.is_ascii_alphabetic()
                    || (index > 0 && (byte.is_ascii_digit() || b"+-.".contains(&byte)))
            })
    })
}
