//! Identity-bearing JavaScript data objects. The engine owns fields and cycles.
//! Handles held by Rust callers keep their objects reachable. A mark-and-sweep
//! pass subtracts heap-internal references to find roots without retaining cycles.
use super::*;
use std::rc::Weak;

#[derive(Debug, PartialEq, Eq, Hash)]
struct Identity {
    engine: u64,
    index: u64,
    array: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Record(Rc<Identity>);

impl Record {
    pub fn is_array(&self) -> bool {
        self.0.array
    }
}

struct Entry {
    token: Weak<Identity>,
    fields: BTreeMap<String, Value>,
}

#[derive(Default)]
pub(crate) struct Heap {
    next: u64,
    collect_at: usize,
    entries: BTreeMap<u64, Entry>,
}

#[derive(Default)]
pub(crate) struct Journal {
    first_new: u64,
    previous: BTreeMap<(u64, String), Option<Value>>,
}

impl Heap {
    pub(crate) fn create(
        &mut self,
        engine: u64,
        array: bool,
        fields: BTreeMap<String, Value>,
    ) -> Record {
        let index = self.next;
        self.next += 1;
        let token = Rc::new(Identity {
            engine,
            index,
            array,
        });
        self.entries.insert(
            index,
            Entry {
                token: Rc::downgrade(&token),
                fields,
            },
        );
        Record(token)
    }

    pub(crate) fn journal(&self) -> Journal {
        Journal {
            first_new: self.next,
            previous: BTreeMap::new(),
        }
    }

    pub(crate) fn rollback(&mut self, journal: Journal) {
        self.entries.split_off(&journal.first_new);
        for ((id, key), value) in journal.previous {
            let fields = &mut self.entries.get_mut(&id).expect("journalled object").fields;
            if let Some(old) = value {
                fields.insert(key, old);
            } else {
                fields.remove(&key);
            }
        }
    }

    pub(crate) fn maybe_collect(&mut self) {
        if self.entries.len() < self.collect_at.max(64) {
            return;
        }
        // Rc handles also appear inside records (possibly nested in a typed
        // list). Only references outside the heap are live roots.
        let mut internal = BTreeMap::<u64, usize>::new();
        for entry in self.entries.values() {
            for value in entry.fields.values() {
                references(value, &mut |id| *internal.entry(id).or_default() += 1);
            }
        }
        let mut stack = self
            .entries
            .iter()
            .filter_map(|(id, e)| {
                (e.token.strong_count() > internal.get(id).copied().unwrap_or_default())
                    .then_some(*id)
            })
            .collect::<Vec<_>>();
        let mut marked = BTreeSet::new();
        while let Some(id) = stack.pop() {
            if marked.insert(id) {
                if let Some(entry) = self.entries.get(&id) {
                    for value in entry.fields.values() {
                        references(value, &mut |child| stack.push(child));
                    }
                }
            }
        }
        self.entries.retain(|id, _| marked.contains(id));
        self.collect_at = self.entries.len().saturating_mul(2).max(64);
    }
}

fn references(value: &Value, visit: &mut impl FnMut(u64)) {
    match value {
        Value::Record(record) => visit(record.0.index),
        Value::List(items) => {
            for item in items {
                references(item, visit);
            }
        }
        _ => (),
    }
}

impl Instance {
    pub fn record_entries(&self, record: &Record) -> Result<&BTreeMap<String, Value>, Error> {
        if record.0.engine != self.identity {
            return Err(Error::new("foreign JavaScript object"));
        }
        self.records
            .entries
            .get(&record.0.index)
            .map(|e| &e.fields)
            .ok_or_else(|| Error::new("unknown JavaScript object"))
    }

    pub fn new_record(&mut self, fields: BTreeMap<String, Value>) -> Result<Record, Error> {
        for value in fields.values() {
            self.validate_handles(value)?;
        }
        let record = self.records.create(self.identity, false, fields);
        self.records.maybe_collect();
        Ok(record)
    }

    pub fn set_record_property(
        &mut self,
        record: &Record,
        key: &str,
        value: Value,
    ) -> Result<(), Error> {
        self.transaction(|tx| tx.set_record_field(record, key, value))
    }
}

impl transaction::Transaction<'_> {
    pub(crate) fn new_record(&mut self, fields: BTreeMap<String, Value>) -> Value {
        Value::Record(
            self.model
                .records
                .create(self.model.identity, false, fields),
        )
    }

    pub(crate) fn set_record_field(
        &mut self,
        record: &Record,
        key: &str,
        value: Value,
    ) -> Result<(), Error> {
        self.model.validate_handles(&value)?;
        if key == "__proto__" {
            return Err(Error::new("JavaScript prototype mutation is unsupported"));
        }
        if record.is_array() {
            return self.write_array(record, key, value);
        }
        self.store_field(record, key, Some(value))
    }

    pub(crate) fn store_field(
        &mut self,
        record: &Record,
        key: &str,
        next: Option<Value>,
    ) -> Result<(), Error> {
        let previous = self.model.record_entries(record)?.get(key);
        let id = record.0.index;
        if id < self.record_changes.first_new {
            self.record_changes
                .previous
                .entry((id, key.into()))
                .or_insert_with(|| previous.cloned());
        }
        let fields = &mut self
            .model
            .records
            .entries
            .get_mut(&id)
            .expect("validated object")
            .fields;
        match next {
            Some(value) => {
                fields.insert(key.into(), value);
            }
            None => {
                fields.remove(key);
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn collection_releases_cycles_but_keeps_host_roots() {
        let doc = ir::parse("import QtQml\nQtObject {}").unwrap();
        let mut vm = Instance::new(&doc, &TypeRegistry::new(), BTreeMap::new()).unwrap();
        let kept = vm.new_record(BTreeMap::new()).unwrap();
        let child = vm.new_record(BTreeMap::new()).unwrap();
        vm.set_record_property(&kept, "child", Value::Record(child.clone()))
            .unwrap();
        let token = Rc::downgrade(&child.0);
        drop(child);
        for _ in 0..256 {
            let garbage = vm.new_record(BTreeMap::new()).unwrap();
            vm.set_record_property(&garbage, "self", Value::Record(garbage.clone()))
                .unwrap();
        }
        assert!(token.upgrade().is_some());
        assert!(vm.records.entries.len() < 65);
        assert!(vm.record_entries(&kept).unwrap().contains_key("child"));
    }
}
