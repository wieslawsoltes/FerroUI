//! Temporary native values passed into a V8 turn are pinned until its result has
//! been decoded. Only a live JS proxy can extend that pin beyond the turn.
use ferroui_qml::{Error, Record, RowId, Value};
use std::collections::{HashMap, HashSet};
use std::rc::Rc;

// Bounds a single uninterrupted JS job (WeakRef targets stay alive for the
// whole job) and workers without weak-reference support. Never evict live refs.
const MAX_HOST_PROXIES: usize = 65_536;

#[derive(Clone, PartialEq, Eq, Hash)]
enum Key {
    Record(Record),
    Row(RowId),
    DelegateRow(ferroui_qml::ObjectId),
    Namespace(usize),
}
impl Key {
    fn of(value: &Value) -> Result<Self, Error> {
        match value {
            Value::Record(record) => Ok(Self::Record(record.clone())),
            Value::ModelRow(id) => Ok(Self::Row(*id)),
            Value::DelegateRow(id) => Ok(Self::DelegateRow(*id)),
            Value::Namespace(namespace) => Ok(Self::Namespace(Rc::as_ptr(namespace) as usize)),
            _ => Err(Error::new("value does not have a dynamic host identity")),
        }
    }
}
struct Entry {
    value: Value,
    lease: Option<u64>,
}
#[derive(Default)]
pub(super) struct HostLeases {
    ids: HashMap<Key, u64>,
    entries: HashMap<u64, Entry>,
    unclaimed: HashSet<u64>,
}
impl HostLeases {
    pub fn project(&mut self, value: &Value, next: &mut u64) -> Result<u64, Error> {
        let key = Key::of(value)?;
        if let Some(id) = self.ids.get(&key) {
            return Ok(*id);
        }
        if self.entries.len() >= MAX_HOST_PROXIES {
            return Err(Error::new("WebScene live host proxy limit exceeded"));
        }
        *next += 1;
        let id = *next;
        self.ids.insert(key, id);
        self.entries.insert(
            id,
            Entry {
                value: value.clone(),
                lease: None,
            },
        );
        self.unclaimed.insert(id);
        Ok(id)
    }
    pub fn get(&self, id: u64) -> Option<Value> {
        self.entries.get(&id).map(|entry| entry.value.clone())
    }

    /// Positive revisions acquire a proxy, negative revisions retire it.
    /// V8 may finalize an older proxy after the same host value was re-read.
    pub fn update(&mut self, id: u64, revision: i64) {
        let Some(entry) = self.entries.get_mut(&id) else {
            return;
        };
        if revision > 0 {
            let current = revision as u64;
            if entry.lease.is_none_or(|old| current >= old) {
                entry.lease = Some(current);
                self.unclaimed.remove(&id);
            }
        } else if entry.lease == Some(revision.unsigned_abs()) {
            entry.lease = None;
            self.unclaimed.insert(id);
        }
    }
    /// Called only after the outermost turn decodes the result, so incoming
    /// references and reentrant callbacks cannot race a retirement.
    pub fn finish(&mut self) {
        for id in self.unclaimed.drain() {
            if let Some(entry) = self.entries.remove(&id) {
                self.ids
                    .remove(&Key::of(&entry.value).expect("interned host value"));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ferroui_qml::Namespace;

    #[test]
    fn unobserved_handles_and_stale_finalizers_do_not_pin_or_revoke_live_values() {
        let mut table = HostLeases::default();
        let mut next = 0;
        let namespace = Rc::new(Namespace::default());
        let weak = Rc::downgrade(&namespace);
        let v = Value::Namespace(namespace);
        let id = table.project(&v, &mut next).unwrap();
        table.finish();
        let replacement = table.project(&v, &mut next).unwrap();
        assert_ne!(id, replacement);
        table.update(replacement, 4);
        table.finish();
        assert_eq!(table.project(&v, &mut next).unwrap(), replacement);
        table.update(replacement, 5);
        table.update(replacement, -4);
        table.finish();
        drop(v);
        assert!(weak.upgrade().is_some());
        table.update(replacement, -5);
        assert!(table.get(replacement).is_some());
        table.finish();
        assert!(weak.upgrade().is_none());
    }
    #[test]
    fn a_full_table_rejects_new_identities_without_evicting_live_references() {
        let mut table = HostLeases::default();
        let mut next = 0;
        for _ in 0..MAX_HOST_PROXIES {
            let id = table
                .project(&Value::Namespace(Rc::new(Namespace::default())), &mut next)
                .unwrap();
            table.update(id, 1);
        }
        let last = next;
        let error = table
            .project(&Value::Namespace(Rc::new(Namespace::default())), &mut next)
            .unwrap_err();
        assert!(error.message.contains("limit"));
        assert_eq!(next, last);
        table.finish();
        assert!(table.get(1).is_some());
        assert!(table.get(last).is_some());
    }
}
