//! QML lifetime changes run as owner-thread transactions. Nodes remain readable
//! during destruction handlers; committed tombstones release the native payload.
use super::*;
use std::time::Duration;

impl Instance {
    pub fn destroy_object(&mut self, id: ObjectId) -> Result<(), Error> {
        self.transaction(|tx| tx.destroy_object(id))
    }
    pub fn schedule_destroy(&mut self, id: ObjectId, delay: Duration) -> Result<(), Error> {
        self.transaction(|tx| tx.schedule_destroy(id, delay))
    }
    pub(crate) fn retire_objects(&mut self, dead: &BTreeSet<ObjectId>) {
        if dead.is_empty() {
            return;
        }
        self.dynamic.created.retain(|id| !dead.contains(id));
        self.javascript.connections.retire(dead);
        self.javascript.deferred.retire(dead);
        self.listeners.retain(|_, list| {
            list.retain(|e| !dead.contains(&e.receiver));
            !list.is_empty()
        });
        self.watched.retain(|(id, _)| !dead.contains(id));
        self.changes
            .retain(|p, _| !dead.contains(&self.properties[*p].owner));
        if dead.iter().any(|id| self.models.contains_key(id)) {
            self.workers.retire_model_grants(dead);
        }
        for &id in dead {
            self.dynamic.deadlines.remove(&id);
            self.dynamic.templates.remove(&id);
            self.dynamic.contexts.remove(&id);
            self.models.remove(&id);
            self.bindings.remove(&id);
            self.timers.remove(id);
            self.workers.unregister(id);
            let node = self.nodes[id.index]
                .data
                .take()
                .expect("retiring staged object");
            for &slot in node.properties.values() {
                let s = &mut self.slots[slot];
                s.value = Value::Undefined;
                s.binding = None;
                s.overlay = None;
                s.reads.clear();
                s.dependents.clear();
                let p = &mut self.properties[slot];
                p.alias = None;
                p.name.clear();
                p.ty.clear();
            }
        }
        while self.nodes.last().is_some_and(|n| !n.live) {
            self.nodes.pop();
        }
        while self
            .properties
            .last()
            .is_some_and(|p| p.owner.index >= self.nodes.len())
        {
            self.properties.pop();
            self.slots.pop();
        }
    }
}

pub(crate) fn release_value(v: &Value, dead: &BTreeSet<ObjectId>) -> Option<Value> {
    match v {
        Value::Object(id) if dead.contains(id) => Some(Value::Null),
        Value::Method(method) if dead.contains(&method.receiver()) => Some(Value::Null),
        Value::List(items) => {
            let mut copy: Option<Vec<Value>> = None;
            for (index, item) in items.iter().enumerate() {
                if let Some(v) = release_value(item, dead) {
                    copy.get_or_insert_with(|| items.clone())[index] = v;
                }
            }
            copy.map(Value::List)
        }
        _ => None,
    }
}

impl crate::transaction::Transaction<'_> {
    fn dynamic_root(&self, id: ObjectId) -> Result<(), Error> {
        if self.in_binding() {
            return Err(Error::new("a binding cannot destroy objects"));
        }
        if self.model.node(id).is_none() || !self.model.dynamic.contexts.contains_key(&id) {
            return Err(Error::new(
                "only a live dynamically created root can be destroyed",
            ));
        }
        Ok(())
    }
    pub(crate) fn schedule_destroy(&mut self, id: ObjectId, delay: Duration) -> Result<(), Error> {
        self.dynamic_root(id)?;
        let deadline = self
            .model
            .timers
            .now
            .checked_add(delay)
            .ok_or_else(|| Error::new("destruction deadline overflow"))?;
        let queue = &mut self.model.dynamic.deadlines;
        self.structure
            .deadlines
            .entry(id)
            .or_insert_with(|| queue.get(&id).copied());
        queue
            .entry(id)
            .and_modify(|old| *old = (*old).min(deadline))
            .or_insert(deadline);
        Ok(())
    }
    pub(crate) fn destroy_method(&mut self, id: ObjectId, args: &[Value]) -> Result<Value, Error> {
        let delay = match args {
            [] => Duration::ZERO,
            [Value::Number(n)] if n.is_finite() && (0.0..=2147483647.).contains(n) => {
                Duration::from_millis(*n as u64)
            }
            _ => {
                return Err(Error::new(
                    "destroy expects an optional nonnegative millisecond delay",
                ))
            }
        };
        self.schedule_destroy(id, delay)?;
        Ok(Value::Undefined)
    }
    pub(crate) fn destroy_object(&mut self, id: ObjectId) -> Result<(), Error> {
        self.dynamic_root(id)?;
        let mut dead = BTreeSet::new();
        let mut stack = vec![id];
        while let Some(object) = stack.pop() {
            if dead.insert(object) {
                self.budget()?;
                self.dispatch(object, "Component.destruction", vec![])?;
                self.settle()?;
                stack.extend_from_slice(self.model.children(object));
            }
        }
        for &object in &dead {
            if self.model.bindings.contains(&object) {
                let input = self.model.key(object, "value")?;
                if let Some(target) = self.model.slots[input].active_target {
                    let Value::Number(mode) = self.model.get(object, "restoreMode")? else {
                        unreachable!()
                    };
                    self.remove_overlay(target, input, mode as u8);
                    self.touched(input).active_target = None;
                }
            }
        }
        if let Some(parent) = self.model.parent(id) {
            let children = &mut self.model.nodes[parent.index].children;
            self.structure
                .children
                .entry(parent)
                .or_insert_with(|| children.clone());
            children.retain(|child| !dead.contains(child));
        }
        for object in &dead {
            let keys: Vec<_> = self.model.nodes[object.index]
                .properties
                .values()
                .copied()
                .collect();
            for key in keys {
                self.replace_reads(key, BTreeSet::new());
                self.pending.remove(&key);
                for dep in self.model.slots[key].dependents.clone() {
                    self.touched(dep).reads.remove(&key);
                }
                self.touched(key).dependents.clear();
            }
        }
        for key in 0..self.model.slots.len() {
            let prop = &self.model.properties[key];
            if dead.contains(&prop.owner) || self.model.node(prop.owner).is_none() {
                continue;
            }
            let alias_dead = match prop.alias {
                Some(Alias::Object(id)) => dead.contains(&id),
                Some(Alias::Slot(t)) => dead.contains(&self.model.properties[t].owner),
                None => false,
            };
            let visual = self.model.nodes[prop.owner.index]
                .visual_children
                .as_deref()
                == Some(prop.name.as_str());
            let value = &self.model.slots[key].value;
            let next = if alias_dead {
                Some(Value::Null)
            } else if visual {
                if let Value::List(items) = value {
                    items.iter().any(|v|matches!(v,Value::Object(child) if dead.contains(child))).then(||Value::List(items.iter().filter(|v| !matches!(v,Value::Object(child) if dead.contains(child))).cloned().collect()))
                } else {
                    None
                }
            } else {
                release_value(value, &dead)
            };
            if alias_dead {
                let alias = self.model.properties[key]
                    .alias
                    .take()
                    .expect("invalidated alias");
                self.structure.aliases.push((key, alias));
            }
            if matches!(self.model.slots[key].binding,Some(InstalledBinding::PropertyRead(t)) if dead.contains(&self.model.properties[t].owner))
            {
                self.touched(key).binding = None;
                self.touched(key).dirty = false;
                self.pending.remove(&key);
            }
            if let Some(value) = next {
                self.set_value(key, value);
            }
        }
        self.model.dynamic.revision = self
            .model
            .dynamic
            .revision
            .checked_add(1)
            .ok_or_else(|| Error::new("tree revision exhausted"))?;
        for &object in &dead {
            self.model.nodes[object.index].live = false;
        }
        self.structure.removed.extend(dead);
        Ok(())
    }
}
