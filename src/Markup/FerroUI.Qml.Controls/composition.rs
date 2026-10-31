//! Resolve the QML visual list separately from object ownership. A QtObject can
//! belong to a component without becoming a FerroUI visual, and an object-valued
//! property does not automatically place its value in the parent's child list.
use super::*;
use std::collections::BTreeSet;

pub(super) type Plan = Vec<(Controls, Vec<Ref<Control>>)>;

pub(super) fn plan(
    instance: &Instance,
    nodes: &BTreeMap<ObjectId, Box<dyn NativeObject>>,
) -> Result<Plan, Error> {
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
    Ok(plan)
}

pub(super) fn apply(plan: Plan) {
    // Remove transferred children from all old parents before inserting into new ones.
    for (collection, children) in &plan {
        for child in collection.to_vec() {
            if !children.contains(&child) {
                collection.remove(&child);
            }
        }
    }
    for (collection, children) in plan {
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
}

pub(super) fn reconcile(
    instance: &Instance,
    nodes: &BTreeMap<ObjectId, Box<dyn NativeObject>>,
) -> Result<(), Error> {
    apply(plan(instance, nodes)?);
    Ok(())
}
