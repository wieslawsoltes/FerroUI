//! QtQml.Binding overlays the target's binding without destroying its underlying
//! expression. A stack handles overlapping controllers and out-of-order removal.
//! Both ends live in journalled slots, so retargeting rolls back atomically.
use super::*;
use crate::transaction::Transaction;

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
    pub(crate) fn input(&self) -> SlotId {
        self.stack.last().expect("nonempty overlay stack").input
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
        let input = self.model.key(object, "value")?;
        let old = self.model.slots[input].active_target;
        let target = match (
            self.model.get(object, "when")?,
            self.model.get(object, "target")?,
        ) {
            (Value::Bool(true), Value::Object(target)) => {
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
        let mode = match self.model.get(object, "restoreMode")? {
            Value::Number(n) if (0.0..=3.0).contains(&n) && n.fract() == 0.0 => n as u8,
            _ => return Err(Error::new("invalid Binding.restoreMode")),
        };
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
            return Ok(());
        }
        if let Some(old) = old {
            self.remove_overlay(old, input, mode);
        }
        self.touched(input).active_target = target;
        if let Some(target) = target {
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
        Ok(())
    }

    pub(crate) fn publish_delayed(&mut self, object: ObjectId) -> Result<(), Error> {
        let input = self.model.key(object, "value")?;
        let Some(target) = self.model.slots[input].active_target else {
            return Ok(());
        };
        let value = self.model.slots[input].value.clone();
        let overlay = self
            .touched(target)
            .overlay
            .as_mut()
            .expect("active overlay");
        let top = overlay.input() == input;
        if let Some(entry) = overlay
            .stack
            .iter_mut()
            .find(|entry| entry.input == input && entry.frozen.is_some())
        {
            entry.frozen = Some(value);
            if top {
                overlay.superseded = false;
                self.touched(target).binding = None;
                self.invalidate_overlay(target);
            }
        }
        Ok(())
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
