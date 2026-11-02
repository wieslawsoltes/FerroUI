//! Resolve the QML visual list separately from object ownership. A QtObject can
//! belong to a component without becoming a FerroUI visual, and an object-valued
//! property does not automatically place its value in the parent's child list.
use super::*;
use std::collections::{BTreeSet, HashSet};

/// A QML child list and its last committed native collection. Retain IDs in
/// visual order so a transaction can remove previous edges without scanning
/// the model or treating an ownership parent as a visual parent.
pub(super) struct Host {
    children: Vec<ObjectId>,
    collection: Controls,
    snapshot: std::rc::Weak<Vec<Ref<Control>>>,
}

#[derive(Default)]
pub(super) struct Graph {
    parents: BTreeMap<ObjectId, ObjectId>,
    stable_hosts: bool,
    hosts: BTreeMap<ObjectId, Host>,
}

impl Graph {
    pub fn is_current(&self) -> bool {
        self.stable_hosts
            && self
                .hosts
                .values()
                .all(|h| h.snapshot.as_ptr() == Rc::as_ptr(&h.collection.snapshot()))
    }
}

pub(super) enum Update {
    Replace(Graph),
    Patch {
        parents: BTreeMap<ObjectId, Option<ObjectId>>,
        hosts: BTreeMap<ObjectId, Host>,
    },
}
impl Update {
    pub fn commit(self, graph: &mut Graph) {
        match self {
            Self::Replace(all) => *graph = all,
            Self::Patch { parents, hosts } => {
                for (child, parent) in parents {
                    if let Some(parent) = parent {
                        graph.parents.insert(child, parent);
                    } else {
                        graph.parents.remove(&child);
                    }
                }
                graph.hosts.extend(hosts);
            }
        }
    }
}

pub(super) struct Plan {
    update: Update,
    children: Vec<(Controls, Vec<Ref<Control>>)>,
    anchors: Vec<(Ref<super::anchors::AnchorCanvas>, super::anchors::Rules)>,
}

impl Plan {
    pub fn reusable(&self) -> bool {
        match &self.update {
            Update::Replace(graph) => graph.stable_hosts,
            Update::Patch { .. } => true,
        }
    }
    pub fn full(&self) -> bool {
        matches!(self.update, Update::Replace(_))
    }
}

fn child_list(
    instance: &Instance,
    nodes: &tree::Peers,
    parent: ObjectId,
) -> Result<Option<Host>, Error> {
    let node = &nodes[&parent];
    let Some(name) = node.children_property() else {
        return Ok(None);
    };
    let Value::List(entries) = instance.get(parent, name)? else {
        return Err(Error::new(format!("{name} must be an object list")));
    };
    let mut children = Vec::new();
    for entry in entries {
        let Value::Object(id) = entry else {
            return Err(Error::new(format!("{name} expects an object")));
        };
        if instance.type_name(id).is_none() {
            return Err(Error::new("foreign child in visual list"));
        }
        if nodes.contains_key(&id) {
            children.push(id);
        }
    }
    match node.content() {
        Some((_, collection)) => Ok(Some(Host {
            children,
            collection,
            snapshot: Default::default(),
        })),
        None if children.is_empty() => Ok(None),
        None => Err(Error::new(format!(
            "{} has no native visual child host",
            instance.type_name(parent).unwrap_or("object")
        ))),
    }
}

fn check_cycles(
    starts: impl Iterator<Item = ObjectId>,
    parent_of: impl Fn(ObjectId) -> Option<ObjectId>,
) -> Result<(), Error> {
    let mut finished = BTreeSet::new();
    for start in starts {
        let mut path = BTreeSet::new();
        let mut next = start;
        while !finished.contains(&next) {
            let Some(parent) = parent_of(next) else { break };
            if !path.insert(next) {
                return Err(Error::new("cycle in visual children"));
            }
            next = parent;
        }
        finished.extend(path);
    }
    Ok(())
}

fn controls(host: &Host, nodes: &tree::Peers) -> (Controls, Vec<Ref<Control>>) {
    (
        host.collection.clone(),
        host.children.iter().map(|id| nodes[id].control()).collect(),
    )
}

pub(super) fn plan(instance: &Instance, nodes: &tree::Peers) -> Result<Plan, Error> {
    let stable_hosts = nodes.values().all(|node| node.has_stable_visual_host());
    let mut graph = Graph {
        stable_hosts,
        ..Default::default()
    };
    let mut children = Vec::new();
    for &parent in nodes.keys() {
        if let Some(host) = child_list(instance, nodes, parent)? {
            for &id in &host.children {
                if id == instance.root() || graph.parents.insert(id, parent).is_some() {
                    return Err(Error::new("root or multiply parented child in visual list"));
                }
            }
            children.push(controls(&host, nodes));
            if stable_hosts {
                graph.hosts.insert(parent, host);
            }
        }
    }
    let parent_of = |child| graph.parents.get(&child).copied();
    check_cycles(graph.parents.keys().copied(), parent_of)?;
    let mut hosts = BTreeMap::new();
    for (&id, node) in nodes {
        if let Some(canvas) = node
            .anchor_host()
            .and_then(|c| c.cast::<super::anchors::AnchorCanvas>())
        {
            hosts.insert(id, (canvas, super::anchors::Rules::default()));
        }
    }
    for (&id, node) in nodes {
        if !node.uses_item_anchors() {
            continue;
        }
        let Some(rule) = super::anchors::read(instance, id, parent_of, nodes)? else {
            continue;
        };
        let parent =
            parent_of(id).ok_or_else(|| Error::new("anchored Item has no visual parent"))?;
        let (_, rules) = hosts
            .get_mut(&parent)
            .ok_or_else(|| Error::new("anchors require an Item or Rectangle layout host"))?;
        rules.insert(node.control(), rule);
    }
    for (_, rules) in hosts.values_mut() {
        rules.validate()?;
    }
    Ok(Plan {
        update: Update::Replace(graph),
        children,
        anchors: hosts.into_values().collect(),
    })
}

pub(super) fn apply(mut plan: Plan) -> Update {
    // A structural change in one delegate need not edit every sibling collection.
    plan.children.retain(|(collection, children)| {
        collection.count() != children.len()
            || children
                .iter()
                .enumerate()
                .any(|(i, child)| collection.get(i) != *child)
    });
    // Remove transferred children from all old parents before inserting into new ones.
    for (collection, children) in &plan.children {
        let retained: HashSet<_> = children.iter().collect();
        let mut index = 0;
        for child in collection.to_vec() {
            if retained.contains(&child) {
                index += 1;
            } else {
                collection.remove_at(index);
            }
        }
    }
    for (collection, children) in plan.children {
        for (target, child) in children.into_iter().enumerate() {
            if target == collection.count() {
                collection.add(child);
                continue;
            }
            if collection.get(target) == child {
                continue;
            }
            match collection.index_of(&child) {
                Some(old) => collection.move_item(old, target),
                None => collection.insert(target, child),
            }
        }
    }
    for (host, rules) in plan.anchors {
        host.update(rules);
    }
    let changed = match &mut plan.update {
        Update::Replace(graph) => &mut graph.hosts,
        Update::Patch { hosts, .. } => hosts,
    };
    for host in changed.values_mut() {
        host.snapshot = Rc::downgrade(&host.collection.snapshot());
    }
    plan.update
}

type AnchorHosts = BTreeMap<ObjectId, (Ref<super::anchors::AnchorCanvas>, super::anchors::Rules)>;

fn host_rules<'a>(
    parent: Option<ObjectId>,
    nodes: &tree::Peers,
    hosts: &'a mut AnchorHosts,
) -> Option<&'a mut super::anchors::Rules> {
    let parent = parent?;
    if let std::collections::btree_map::Entry::Vacant(entry) = hosts.entry(parent) {
        let canvas = nodes[&parent]
            .anchor_host()?
            .cast::<super::anchors::AnchorCanvas>()?;
        let rules = canvas.snapshot_rules();
        entry.insert((canvas, rules));
    }
    Some(&mut hosts.get_mut(&parent).expect("registered anchor host").1)
}

/// Validate the final relation for all edited lists together. A transfer may
/// update its destination before its source. Pure reorders preserve the parent
/// graph and all previously checked anchor relationships.
pub(super) fn changes(
    instance: &Instance,
    nodes: &tree::Peers,
    graph: &Graph,
    lists: BTreeSet<ObjectId>,
    mut anchors: BTreeSet<ObjectId>,
) -> Result<Plan, Error> {
    let mut parents = BTreeMap::new();
    let mut hosts = BTreeMap::new();
    let mut children = Vec::new();
    for parent in &lists {
        if let Some(old) = graph.hosts.get(parent) {
            parents.extend(old.children.iter().map(|&child| (child, None)));
        }
    }
    for parent in lists {
        if let Some(host) = child_list(instance, nodes, parent)? {
            for &child in &host.children {
                let previous = parents
                    .get(&child)
                    .copied()
                    .unwrap_or_else(|| graph.parents.get(&child).copied());
                if child == instance.root() || previous.is_some() {
                    return Err(Error::new("root or multiply parented child in visual list"));
                }
                parents.insert(child, Some(parent));
            }
            children.push(controls(&host, nodes));
            hosts.insert(parent, host);
        }
    }
    parents.retain(|child, new| graph.parents.get(child) != new.as_ref());
    let parent_of = |child| {
        parents
            .get(&child)
            .copied()
            .unwrap_or_else(|| graph.parents.get(&child).copied())
    };
    check_cycles(parents.keys().copied(), parent_of)?;

    // Anchors can refer only to parents and siblings. Recheck old and new
    // sibling groups for changed edges, including a child's own detached rule.
    let mut families = BTreeSet::new();
    for (&child, &next) in &parents {
        families.extend(graph.parents.get(&child).copied());
        families.extend(next);
        anchors.insert(child);
    }
    for parent in families {
        if let Some(host) = hosts.get(&parent) {
            anchors.extend(host.children.iter().copied());
        }
    }
    let mut rules = AnchorHosts::new();
    for child in anchors {
        let node = &nodes[&child];
        if !node.uses_item_anchors() {
            continue;
        }
        let parent = parent_of(child);
        let old = graph.parents.get(&child).copied();
        if old != parent {
            if let Some(rules) = host_rules(old, nodes, &mut rules) {
                rules.replace(node.control(), None);
            }
        }
        let rule = super::anchors::read(instance, child, parent_of, nodes)?;
        if let Some(rules) = host_rules(parent, nodes, &mut rules) {
            rules.replace(node.control(), rule);
        } else if rule.is_some() {
            return Err(Error::new(if parent.is_none() {
                "anchored Item has no visual parent"
            } else {
                "anchors require an Item or Rectangle layout host"
            }));
        }
    }
    for (_, rules) in rules.values_mut() {
        rules.validate()?;
    }
    Ok(Plan {
        update: Update::Patch { parents, hosts },
        children,
        anchors: rules.into_values().collect(),
    })
}
