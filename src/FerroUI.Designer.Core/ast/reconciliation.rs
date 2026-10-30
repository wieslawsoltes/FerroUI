use super::{Ast, Node, NodeId, NodeKind};
use std::collections::{HashMap, HashSet, VecDeque};
use std::hash::{Hash, Hasher};

impl Ast {
    /// Match named nodes within corresponding name scopes, then unchanged
    /// subtree fingerprints, then sibling position under matched parents.
    /// Explicit edits may pin identities by source offsets, preventing ambiguity
    /// when duplicate siblings move.
    pub(crate) fn reconcile(
        &mut self,
        old: &Ast,
        old_source: &str,
        source: &str,
        next_id: &mut NodeId,
        pins: &[(usize, NodeId)],
    ) {
        let mut mapped = HashMap::<NodeId, NodeId>::new();
        let mut used = HashSet::new();
        let by_offset: HashMap<_, _> = self
            .nodes()
            .map(|node| (node.span.start, node.id))
            .collect();
        for &(offset, old_id) in pins {
            if let (Some(old_node), Some(&new_id)) = (old.node(old_id), by_offset.get(&offset)) {
                let node = self.node(new_id).unwrap();
                if compatible(node, old_node)
                    && !mapped.contains_key(&new_id)
                    && used.insert(old_id)
                {
                    mapped.insert(new_id, old_id);
                }
            }
        }
        // Property edits and many structural edits already pin every surviving
        // token. Compatibility and one-to-one ownership were checked above, so
        // no semantic candidate search or subtree fingerprinting is needed.
        // Still remap: moving identical siblings can reorder IDs without
        // changing the source bytes or allocating any new identities.
        if mapped.len() == self.nodes.len() {
            self.remap(&mapped);
            return;
        }
        let old_named = names_by_scope(old);
        let new_named = names_by_scope(self);
        // Document roots represent the same outer name scope even if their
        // control types changed. Template scopes are matched separately below.
        reconcile_names(
            self.root,
            old.root,
            &new_named,
            &old_named,
            &mut mapped,
            &mut used,
        );
        if !mapped.contains_key(&self.root)
            && compatible(self.node(self.root).unwrap(), old.node(old.root).unwrap())
            && used.insert(old.root)
        {
            mapped.insert(self.root, old.root);
        }
        // Match sibling groups in source order. Fingerprints are assembled bottom
        // up from each token exactly once; even deeply nested source has linear
        // hashing cost. They only select identity candidates, never validate XML.
        let old_fingerprints = fingerprints(old, old_source);
        let new_fingerprints = fingerprints(self, source);
        reconcile_siblings(
            &self.roots,
            &old.roots,
            self,
            old,
            &new_fingerprints,
            &old_fingerprints,
            &mut mapped,
            &mut used,
            next_id,
        );
        for node in self
            .nodes_in_source_order()
            .filter(|node| node.is_element())
        {
            let parent = node.id;
            if super::defines_name_scope(node) {
                if let Some(&old_scope) = mapped.get(&parent).filter(|id| old.node(**id).is_some())
                {
                    reconcile_names(
                        parent,
                        old_scope,
                        &new_named,
                        &old_named,
                        &mut mapped,
                        &mut used,
                    );
                }
            }
            let new_children = &node.children;
            let old_children = mapped
                .get(&parent)
                .and_then(|id| old.node(*id))
                .map(|node| node.children.as_slice())
                .unwrap_or(&[]);
            reconcile_siblings(
                new_children,
                old_children,
                self,
                old,
                &new_fingerprints,
                &old_fingerprints,
                &mut mapped,
                &mut used,
                next_id,
            );
        }
        self.remap(&mapped);
    }
    pub(crate) fn allocate(&mut self, next_id: &mut NodeId) {
        // Fresh parsing assigns exactly 1..=N. The first document allocation
        // can keep that index directly rather than rebuilding it unchanged.
        // Noninitial counters must retain the ordinary allocation path.
        if *next_id == 1
            && self.nodes.first_key_value().is_some_and(|(&id, _)| id == 1)
            && self
                .nodes
                .last_key_value()
                .is_some_and(|(&id, _)| id == self.nodes.len() as NodeId)
        {
            *next_id += self.nodes.len() as NodeId;
            return;
        }
        let map = self
            .nodes
            .keys()
            .map(|old| {
                let id = *next_id;
                *next_id += 1;
                (*old, id)
            })
            .collect();
        self.remap(&map);
    }
    fn remap(&mut self, map: &HashMap<NodeId, NodeId>) {
        self.root = map[&self.root];
        self.roots.iter_mut().for_each(|id| *id = map[id]);
        self.nodes = std::mem::take(&mut self.nodes)
            .into_values()
            .map(|mut node| {
                node.id = map[&node.id];
                node.parent = node.parent.map(|id| map[&id]);
                node.children.iter_mut().for_each(|id| *id = map[id]);
                (node.id, node)
            })
            .collect();
    }
}

fn compatible(a: &Node, b: &Node) -> bool {
    a.kind == b.kind && a.local_name() == b.local_name() && a.namespace_uri == b.namespace_uri
}
// Matching keys borrow names from the two immutable snapshots. Renaming a
// prefix does not change semantic identity or allocate a key for every sibling.
type MatchKey<'a> = (NodeKind, Option<&'a str>, &'a str);
fn match_key(node: &Node) -> MatchKey<'_> {
    (node.kind, node.namespace_uri.as_deref(), node.local_name())
}

type NamedKey<'a> = (MatchKey<'a>, &'a str);
type ScopeNames<'a> = HashMap<NodeId, HashMap<NamedKey<'a>, Option<NodeId>>>;

/// Compute inherited name scopes in one preorder pass. Walking each node's
/// ancestors independently would be quadratic for deeply nested documents.
fn names_by_scope(ast: &Ast) -> ScopeNames<'_> {
    let mut names = ScopeNames::new();
    for (node, scope) in ast.nodes_with_name_scopes() {
        if let Some(name) = node.xaml_name() {
            names
                .entry(scope)
                .or_default()
                .entry((match_key(node), name))
                .and_modify(|id| *id = None)
                .or_insert(Some(node.id));
        }
    }
    names
}

fn reconcile_names(
    new_scope: NodeId,
    old_scope: NodeId,
    new_names: &ScopeNames<'_>,
    old_names: &ScopeNames<'_>,
    mapped: &mut HashMap<NodeId, NodeId>,
    used: &mut HashSet<NodeId>,
) {
    let (Some(new_names), Some(old_names)) = (new_names.get(&new_scope), old_names.get(&old_scope))
    else {
        return;
    };
    for (key, &new_id) in new_names {
        if let (Some(new_id), Some(&Some(old_id))) = (new_id, old_names.get(key)) {
            if !mapped.contains_key(&new_id) && used.insert(old_id) {
                mapped.insert(new_id, old_id);
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn reconcile_siblings(
    new_children: &[NodeId],
    old_children: &[NodeId],
    new: &Ast,
    old: &Ast,
    new_hashes: &HashMap<NodeId, u64>,
    old_hashes: &HashMap<NodeId, u64>,
    mapped: &mut HashMap<NodeId, NodeId>,
    used: &mut HashSet<NodeId>,
    next_id: &mut NodeId,
) {
    let mut exact = HashMap::<u64, VecDeque<NodeId>>::new();
    let mut kinds = HashMap::<MatchKey<'_>, VecDeque<NodeId>>::new();
    for &id in old_children {
        if used.contains(&id) {
            continue;
        }
        exact.entry(old_hashes[&id]).or_default().push_back(id);
        kinds
            .entry(match_key(old.node(id).unwrap()))
            .or_default()
            .push_back(id);
    }
    // First reserve every unchanged sibling before assigning changed siblings,
    // so an insertion cannot steal an existing sibling's identity.
    for &id in new_children {
        if mapped.contains_key(&id) {
            continue;
        }
        if let Some(candidates) = exact.get_mut(&new_hashes[&id]) {
            while let Some(old_id) = candidates.pop_front() {
                if !used.contains(&old_id)
                    && compatible(new.node(id).unwrap(), old.node(old_id).unwrap())
                {
                    mapped.insert(id, old_id);
                    used.insert(old_id);
                    break;
                }
            }
        }
    }
    for &id in new_children {
        if mapped.contains_key(&id) {
            continue;
        }
        if let Some(candidates) = kinds.get_mut(&match_key(new.node(id).unwrap())) {
            while let Some(old_id) = candidates.pop_front() {
                if used.insert(old_id) {
                    mapped.insert(id, old_id);
                    break;
                }
            }
        }
        mapped.entry(id).or_insert_with(|| {
            let id = *next_id;
            *next_id += 1;
            id
        });
    }
}

fn fingerprints(ast: &Ast, source: &str) -> HashMap<NodeId, u64> {
    let order: Vec<_> = ast.nodes_in_source_order().collect();
    let mut hashes = HashMap::<NodeId, u64>::with_capacity(order.len());
    for node in order.into_iter().rev() {
        let mut hash = std::collections::hash_map::DefaultHasher::new();
        node.kind.hash(&mut hash);
        node.namespace_uri.hash(&mut hash);
        node.local_name().hash(&mut hash);
        if node.is_element() {
            source[node.opening_span.range()].hash(&mut hash);
            for child in &node.children {
                hashes[child].hash(&mut hash);
            }
            if let Some(closing) = node.closing_span {
                source[closing.range()].hash(&mut hash);
            }
        } else {
            source[node.span.range()].hash(&mut hash);
        }
        hashes.insert(node.id, hash.finish());
    }
    hashes
}
