//! Resolve the QML visual list separately from object ownership. A QtObject can
//! belong to a component without becoming a FerroUI visual, and an object-valued
//! property does not automatically place its value in the parent's child list.
use super::*;
use std::collections::{BTreeSet, HashSet};

pub(super) struct Graph {
    parents: BTreeMap<ObjectId, ObjectId>,
    pub stable_hosts: bool,
    collections: Vec<(Controls, std::rc::Weak<Vec<Ref<Control>>>)>,
}

impl Graph {
    // Native callers can also modify the exposed child collections. FerroList
    // snapshots are copy-on-write: a weak snapshot detects changes without
    // retaining a removed control or copying the list when it is edited.
    pub fn is_current(&self) -> bool {
        self.stable_hosts
            && self
                .collections
                .iter()
                .all(|(children, previous)| previous.as_ptr() == Rc::as_ptr(&children.snapshot()))
    }
}

pub(super) struct Plan {
    pub graph: Option<Graph>,
    children: Vec<(Controls, Vec<Ref<Control>>)>,
    anchors: Vec<(Ref<super::anchors::AnchorCanvas>, super::anchors::Rules)>,
}

pub(super) fn plan(instance: &Instance, nodes: &tree::Peers) -> Result<Plan, Error> {
    let mut parents = BTreeMap::new();
    let mut plan = Vec::new();
    for (&parent, node) in nodes {
        let Some(name) = node.children_property() else {
            continue;
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
            let Some(child) = nodes.get(&id) else {
                continue;
            };
            if id == instance.root() || parents.insert(id, parent).is_some() {
                return Err(Error::new("root or multiply parented child in visual list"));
            }
            children.push(child.control());
        }
        match node.content() {
            Some((_, collection)) => plan.push((collection, children)),
            None if !children.is_empty() => {
                return Err(Error::new(format!(
                    "{} has no native visual child host",
                    instance.type_name(parent).unwrap_or("object")
                )))
            }
            None => {}
        }
    }

    // Detect cycles before changing the retained tree. Finished paths are memoized.
    let mut finished = BTreeSet::new();
    for &start in parents.keys() {
        let mut path = BTreeSet::new();
        let mut next = start;
        while !finished.contains(&next) {
            let Some(parent) = parents.get(&next) else {
                break;
            };
            if !path.insert(next) {
                return Err(Error::new("cycle in visual children"));
            }
            next = *parent;
        }
        finished.extend(path);
    }
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
        let Some(rule) = super::anchors::read(instance, id, &parents, nodes)? else {
            continue;
        };
        let parent = parents
            .get(&id)
            .ok_or_else(|| Error::new("anchored Item has no visual parent"))?;
        let (_, rules) = hosts
            .get_mut(parent)
            .ok_or_else(|| Error::new("anchors require an Item or Rectangle layout host"))?;
        rules.insert(node.control(), rule);
    }
    for (_, rules) in hosts.values_mut() {
        rules.validate()?;
    }
    let stable_hosts = nodes.values().all(|node| node.has_stable_visual_host());
    let collections = if stable_hosts {
        plan.iter()
            .map(|(children, _)| (children.clone(), std::rc::Weak::new()))
            .collect()
    } else {
        Vec::new()
    };
    Ok(Plan {
        graph: Some(Graph {
            parents,
            stable_hosts,
            collections,
        }),
        children: plan,
        anchors: hosts.into_values().collect(),
    })
}

pub(super) fn apply(mut plan: Plan) -> Option<Graph> {
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
    if let Some(graph) = &mut plan.graph {
        for (collection, snapshot) in &mut graph.collections {
            *snapshot = Rc::downgrade(&collection.snapshot());
        }
    }
    plan.graph
}

/// Patch only the owners of changed anchor lines. Properties also read by
/// siblings (baselineOffset and alignWhenCentered) use the full planner.
pub(super) fn anchor_changes(
    instance: &Instance,
    nodes: &tree::Peers,
    graph: &Graph,
    changed: BTreeSet<ObjectId>,
) -> Result<Plan, Error> {
    let mut hosts = BTreeMap::new();
    for child in changed {
        let rule = super::anchors::read(instance, child, &graph.parents, nodes)?;
        let host = graph.parents.get(&child).and_then(|parent| {
            nodes[parent]
                .anchor_host()?
                .cast::<super::anchors::AnchorCanvas>()
        });
        let Some(canvas) = host else {
            if rule.is_some() {
                return Err(Error::new(if graph.parents.contains_key(&child) {
                    "anchors require an Item or Rectangle layout host"
                } else {
                    "anchored Item has no visual parent"
                }));
            }
            continue;
        };
        let (_, rules) = hosts
            .entry(graph.parents[&child])
            .or_insert_with(|| (canvas.clone(), canvas.snapshot_rules()));
        rules.replace(nodes[&child].control(), rule);
    }
    for (_, rules) in hosts.values_mut() {
        rules.validate()?;
    }
    Ok(Plan {
        graph: None,
        children: Vec::new(),
        anchors: hosts.into_values().collect(),
    })
}
