//! QtQml.Binding overlays the target's binding without destroying its underlying
//! expression. A stack handles overlapping controllers and out-of-order removal.
//! Both ends live in journalled slots, so retargeting rolls back atomically.
use super::*;
use crate::transaction::Transaction;

/// Qt's delayed multi-target form writes changed proxy properties to the target.
/// When a queued delivery runs with when=false, the proxy keeps its pending
/// notification until delayed mode is rebuilt. Literal targets use overlays.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Delivery {
    Immediate,
    Watching,
    Queued,
    Blocked,
}

impl Slot {
    fn group_delivery(&self) -> Option<Delivery> {
        match self.binding.as_ref()? {
            InstalledBinding::Guarded {
                delayed: Some(_),
                delivery,
                ..
            } => Some(*delivery),
            _ => None,
        }
    }
}

#[derive(Clone, Copy)]
pub(crate) struct Declaration {
    pub(crate) input: SlotId,
    pub(crate) target: SlotId,
    pub(crate) target_owner: ObjectId,
}

#[derive(Clone)]
pub(crate) struct Overlay {
    source_value: Value,
    source_binding: Option<InstalledBinding>,
    source_assigned: bool,
    stack: Vec<Entry>,
    // An imperative binding can control the target until the overlay writes again.
    pub(crate) superseded: bool,
}

#[derive(Clone)]
struct Entry {
    input: SlotId,
    frozen: Option<Value>,
}

impl Overlay {
    pub(crate) fn restores_expression(&self) -> bool {
        self.source_binding.is_some()
    }
    pub(crate) fn input(&self) -> SlotId {
        self.stack.last().expect("nonempty overlay stack").input
    }
    pub(crate) fn clear_dead_references(&mut self, dead: &BTreeSet<ObjectId>) {
        if let Some(value) = lifetime::release_value(&self.source_value, dead) {
            self.source_value = value;
        }
        for entry in &mut self.stack {
            if let Some(value) = entry
                .frozen
                .as_ref()
                .and_then(|v| lifetime::release_value(v, dead))
            {
                entry.frozen = Some(value);
            }
        }
    }
    pub(crate) fn frozen(&self) -> Option<&Value> {
        self.stack.last()?.frozen.as_ref()
    }
    pub(crate) fn immediate_input(&self) -> Option<SlotId> {
        self.frozen().is_none().then(|| self.input())
    }
    pub(crate) fn is_delayed(&self, input: SlotId) -> bool {
        self.stack
            .iter()
            .any(|entry| entry.input == input && entry.frozen.is_some())
    }
}

impl Transaction<'_> {
    pub(crate) fn update_binding(&mut self, object: ObjectId) -> Result<(), Error> {
        self.budget()?;
        let delayed = self.model.get(object, "delayed")? == Value::Bool(true);
        let mode = match self.model.get(object, "restoreMode")? {
            Value::Number(n) if (0.0..=3.0).contains(&n) && n.fract() == 0.0 => n as u8,
            _ => return Err(Error::new("invalid Binding.restoreMode")),
        };
        let enabled = self.model.get(object, "when")? == Value::Bool(true);
        if self.model.bindings[&object].is_empty() {
            let input = self.model.key(object, "value")?;
            let target = match (enabled, self.model.get(object, "target")?) {
                (true, Value::Object(target)) => {
                    let Value::String(name) = self.model.get(object, "property")? else {
                        return Err(Error::new("Binding.property must be a property name"));
                    };
                    let property = self.model.key(object, "property")?;
                    let scope = self.model.slots[property].scope;
                    Some(
                        self.model
                            .write_target(self.model.key_in(target, &name, scope)?)?,
                    )
                }
                _ => None,
            };
            self.update_overlay(input, target, delayed, mode);
        }
        for index in 0..self.model.bindings[&object].len() {
            let declaration = self.model.bindings[&object][index];
            let live = self.model.node(declaration.target_owner).is_some();
            let input = declaration.input;
            if let Some(delivery) = self.model.slots[input].group_delivery() {
                if delayed {
                    if delivery == Delivery::Immediate {
                        self.update_overlay(input, None, false, mode);
                        self.set_group_delivery(
                            input,
                            if enabled {
                                Delivery::Watching
                            } else {
                                Delivery::Blocked
                            },
                        );
                        if enabled && live {
                            self.write(declaration.target, self.model.slots[input].value.clone())?;
                        }
                    }
                    continue;
                }
                if delivery != Delivery::Immediate {
                    self.set_group_delivery(input, Delivery::Immediate);
                }
            }
            let target = (enabled && live).then_some(declaration.target);
            self.update_overlay(input, target, delayed, mode);
        }
        Ok(())
    }

    pub(crate) fn update_overlay(
        &mut self,
        input: SlotId,
        target: Option<SlotId>,
        delayed: bool,
        mode: u8,
    ) {
        let old = self.model.slots[input].active_target;
        if old == target {
            if let Some(target) = target {
                let value = self.model.slots[input].value.clone();
                let overlay = self
                    .touched(target)
                    .overlay
                    .as_mut()
                    .expect("active overlay");
                if let Some(entry) = overlay.stack.iter_mut().find(|entry| entry.input == input) {
                    if entry.frozen.is_some() != delayed {
                        entry.frozen = delayed.then_some(value);
                        self.invalidate_overlay(target);
                    }
                }
            }
            return;
        }
        if let Some(old) = old {
            self.remove_overlay(old, input, mode);
        }
        self.touched(input).active_target = target;
        if let Some(target) = target {
            // The displaced expression no longer controls this slot. Detach it
            // before evaluating any of the sibling declarations in this group.
            self.replace_reads(target, BTreeSet::new());
            let frozen = delayed.then(|| self.model.slots[input].value.clone());
            let slot = self.touched(target);
            let overlay = slot.overlay.get_or_insert_with(|| {
                Box::new(Overlay {
                    source_value: slot.value.clone(),
                    source_binding: slot.binding.clone(),
                    source_assigned: slot.assigned,
                    stack: Vec::new(),
                    superseded: false,
                })
            });
            overlay.superseded = false;
            overlay.stack.push(Entry { input, frozen });
            slot.assigned = true;
            self.invalidate_overlay(target);
        }
    }

    fn set_group_delivery(&mut self, key: SlotId, state: Delivery) {
        if let Some(InstalledBinding::Guarded { delivery, .. }) = &mut self.touched(key).binding {
            *delivery = state;
        }
    }

    pub(crate) fn queue_group_input(&mut self, key: SlotId) {
        if matches!(
            self.model.slots[key].group_delivery(),
            Some(Delivery::Watching | Delivery::Queued)
        ) {
            self.set_group_delivery(key, Delivery::Queued);
            self.deferred
                .push(deferred::Task::Binding(self.model.properties[key].owner));
        }
    }

    pub(crate) fn publish_delayed(&mut self, object: ObjectId) -> Result<(), Error> {
        let input = self.model.key(object, "value")?;
        self.publish_input(input);
        let enabled = self.model.get(object, "when")? == Value::Bool(true);
        for index in 0..self.model.bindings[&object].len() {
            let declaration = self.model.bindings[&object][index];
            match self.model.slots[declaration.input].group_delivery() {
                Some(Delivery::Queued | Delivery::Watching) if !enabled => {
                    self.set_group_delivery(declaration.input, Delivery::Blocked);
                }
                Some(Delivery::Queued) => {
                    self.set_group_delivery(declaration.input, Delivery::Watching);
                    if self.model.node(declaration.target_owner).is_some() {
                        self.write(
                            declaration.target,
                            self.model.slots[declaration.input].value.clone(),
                        )?;
                    }
                }
                _ => self.publish_input(declaration.input),
            }
        }
        Ok(())
    }

    fn publish_input(&mut self, input: SlotId) {
        let Some(target) = self.model.slots[input].active_target else {
            return;
        };
        let value = &self.model.slots[input].value;
        let overlay = self.model.slots[target]
            .overlay
            .as_ref()
            .expect("active overlay");
        let Some(index) = overlay.stack.iter().position(|entry| {
            entry.input == input && entry.frozen.as_ref().is_some_and(|v| !v.unchanged(value))
        }) else {
            return;
        };
        let value = value.clone();
        let overlay = self
            .touched(target)
            .overlay
            .as_mut()
            .expect("active overlay");
        overlay.stack[index].frozen = Some(value);
        if index + 1 == overlay.stack.len() {
            overlay.superseded = false;
            self.touched(target).binding = None;
            self.invalidate_overlay(target);
        }
    }

    fn invalidate_overlay(&mut self, key: SlotId) {
        self.touched(key).dirty = true;
        self.pending.insert(key);
    }

    pub(crate) fn remove_overlay(&mut self, target: SlotId, input: SlotId, mode: u8) {
        let slot = self.touched(target);
        let mut overlay = slot
            .overlay
            .take()
            .expect("active binding has a target overlay");
        let was_top = overlay.input() == input;
        overlay.stack.retain(|entry| entry.input != input);
        if !overlay.stack.is_empty() {
            if was_top {
                overlay.superseded = false;
            }
            slot.overlay = Some(overlay);
            if was_top {
                self.invalidate_overlay(target);
            }
            return;
        }
        let restore_binding = mode & 1 != 0 && overlay.source_binding.is_some();
        let restore_value = mode & 2 != 0 && overlay.source_binding.is_none();
        slot.binding = if restore_binding {
            overlay.source_binding
        } else {
            None
        };
        self.replace_reads(target, BTreeSet::new());
        if restore_binding {
            self.touched(target).assigned = overlay.source_assigned;
            self.invalidate_overlay(target);
        } else {
            self.touched(target).dirty = false;
            self.pending.remove(&target);
            if restore_value {
                self.touched(target).assigned = overlay.source_assigned;
                self.set_value(target, overlay.source_value);
            }
        }
    }
}
