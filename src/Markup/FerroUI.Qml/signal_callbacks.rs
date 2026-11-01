//! Imperative signal connections. Each emission sees surviving registrations
//! up to its starting sequence number; callbacks can safely mutate the table.
use super::*;
use crate::transaction::Transaction;
use std::ops::Bound::{Excluded, Included};

type SignalKey = (ObjectId, String);
type Registrations = BTreeMap<u64, Callback>;

#[derive(Clone)]
struct Callback {
    function: ExternalValue,
    receiver: Value,
}

#[derive(Default)]
pub(crate) struct SignalCallbacks {
    by_signal: BTreeMap<SignalKey, Registrations>,
    serial: u64,
}

#[derive(Default)]
pub(crate) struct Journal(Vec<Edit>);

enum Edit {
    Added(SignalKey, u64),
    Removed(SignalKey, Vec<(u64, Callback)>),
}

impl SignalCallbacks {
    pub(crate) fn retire(&mut self, dead: &BTreeSet<ObjectId>) {
        self.by_signal.retain(|(sender, _), listeners| {
            listeners
                .retain(|_, cb| !matches!(cb.receiver,Value::Object(id) if dead.contains(&id)));
            !dead.contains(sender) && !listeners.is_empty()
        });
    }

    pub(crate) fn has(&self, object: ObjectId, signal: &str) -> bool {
        !self.by_signal.is_empty() && self.by_signal.contains_key(&(object, signal.into()))
    }
    pub(crate) fn boundary(&self) -> u64 {
        self.serial
    }

    pub(crate) fn rollback(&mut self, journal: Journal) {
        for edit in journal.0.into_iter().rev() {
            match edit {
                Edit::Added(key, serial) => self.remove(&key, serial),
                Edit::Removed(key, entries) => {
                    self.by_signal.entry(key).or_default().extend(entries)
                }
            }
        }
    }

    fn remove(&mut self, key: &SignalKey, serial: u64) {
        if let Some(entries) = self.by_signal.get_mut(key) {
            entries.remove(&serial);
            if entries.is_empty() {
                self.by_signal.remove(key);
            }
        }
    }
}

impl Transaction<'_> {
    pub(crate) fn change_signal_connection(
        &mut self,
        sender: ObjectId,
        signal: &str,
        receiver: Value,
        function: ExternalValue,
        connect: bool,
    ) -> Result<(), Error> {
        if self.in_binding() {
            return Err(Error::new("a binding cannot change signal connections"));
        }
        if self
            .model
            .node(sender)
            .and_then(|n| n.signal(signal))
            .is_none()
            || signal.starts_with("$qml:")
        {
            return Err(Error::new(format!("unknown public signal '{signal}'")));
        }
        if function.instance() != self.model.script_instance() {
            return Err(Error::new("foreign signal callback"));
        }
        self.model.validate_handles(&receiver)?;
        let key = (sender, signal.to_owned());
        let callbacks = &mut self.model.javascript.connections;
        if connect {
            callbacks.serial = callbacks
                .serial
                .checked_add(1)
                .ok_or_else(|| Error::new("signal registration sequence exhausted"))?;
            let serial = callbacks.serial;
            callbacks
                .by_signal
                .entry(key.clone())
                .or_default()
                .insert(serial, Callback { function, receiver });
            self.connection_changes.0.push(Edit::Added(key, serial));
        } else if let Some(entries) = callbacks.by_signal.get_mut(&key) {
            let matches: Vec<_> = entries
                .iter()
                .filter(|(_, c)| c.function == function && c.receiver == receiver)
                .map(|(&id, _)| id)
                .collect();
            if !matches.is_empty() {
                let removed = matches
                    .into_iter()
                    .filter_map(|id| entries.remove(&id).map(|cb| (id, cb)))
                    .collect();
                if entries.is_empty() {
                    callbacks.by_signal.remove(&key);
                }
                self.connection_changes.0.push(Edit::Removed(key, removed));
            }
        }
        Ok(())
    }

    pub(crate) fn dispatch_script_callbacks(
        &mut self,
        sender: ObjectId,
        signal: &str,
        args: &[Value],
        boundary: u64,
    ) -> Result<(), Error> {
        if boundary == 0 || self.model.javascript.connections.by_signal.is_empty() {
            return Ok(());
        }
        let key = (sender, signal.to_owned());
        let mut cursor = 0;
        while cursor < boundary {
            let Some((serial, cb)) = self
                .model
                .javascript
                .connections
                .by_signal
                .get(&key)
                .and_then(|entries| entries.range((Excluded(cursor), Included(boundary))).next())
                .map(|(s, c)| (*s, c.clone()))
            else {
                break;
            };
            cursor = serial;
            self.budget()?;
            // Resetting a worker permanently invalidates that realm's functions,
            // including callbacks retained on behalf of otherwise live QML objects.
            if !self
                .model
                .javascript
                .engine
                .as_ref()
                .is_some_and(|e| e.is_reference_live(&cb.function))
            {
                self.model.javascript.connections.remove(&key, serial);
                continue;
            }
            self.with_stack(4, |tx| {
                tx.with_external(sender, &cb.function, &mut BTreeSet::new(), |engine, ctx| {
                    engine.call_external(ctx, &cb.function, cb.receiver, args.to_vec())
                })
            })?;
        }
        Ok(())
    }
}
