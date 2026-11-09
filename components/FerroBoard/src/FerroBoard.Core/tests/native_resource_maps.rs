//! Native insertion-order and mutation contracts for keyboard resource maps.
use ferroboard_core::{layouts::*, text::Utf16Text};

fn entries(map: &ResourceDictionary<Utf16Text, Utf16Text>) -> Vec<(String, String)> {
    map.iter()
        .unwrap()
        .map(|entry| {
            let entry = entry.unwrap().unwrap();
            (
                entry.key.unwrap().to_utf8().unwrap(),
                entry.value.unwrap().to_utf8().unwrap(),
            )
        })
        .collect()
}

#[test]
fn removal_preserves_survivor_order_and_new_entries_append() {
    let map = ResourceDictionary::default();
    for (key, value) in [("a", "A"), ("b", "B"), ("c", "C")] {
        map.add(key.into(), Some(value.into())).unwrap();
    }
    let alias = map.clone();
    assert!(alias.remove(Some(&"b".into())).unwrap());
    alias.add("d".into(), Some("D".into())).unwrap();
    alias.set("c".into(), Some("changed".into()));
    assert_eq!(
        entries(&map),
        [("a", "A"), ("c", "changed"), ("d", "D")]
            .map(|(key, value)| (key.to_owned(), value.to_owned()))
    );
}

#[test]
fn resource_iteration_snapshots_survive_mutation_without_holding_locks() {
    let map = ResourceDictionary::default();
    map.add("a".into(), Some("A".into())).unwrap();
    map.add("b".into(), Some("B".into())).unwrap();
    let mut before_remove = map.iter().unwrap();
    map.remove(Some(&"a".into())).unwrap();
    assert!(matches!(before_remove.next(), Some(Ok(Some(_)))));
    let mut before_clear = map.iter().unwrap();
    map.clear();
    assert!(matches!(before_clear.next(), Some(Ok(Some(_)))));
    assert!(entries(&map).is_empty());
}

#[test]
fn shared_resource_buffers_drop_values_outside_locks_and_keep_snapshot_ownership() {
    let frozen = ResourceReadOnlyList::from_values(vec!["retained".to_owned()]);
    let mut items = frozen.iter().unwrap();
    drop(frozen);
    assert_eq!(items.next().unwrap().unwrap().as_deref(), Some("retained"));
    assert!(items.next().is_none());
    assert!(items.next().is_none());
    use std::{cell::Cell, rc::Rc};
    struct Value {
        owner: ResourceObjectList<Rc<Value>>,
        observed: Rc<Cell<usize>>,
    }
    impl Drop for Value {
        fn drop(&mut self) {
            self.observed.set(self.owner.len());
        }
    }
    let buffer = ResourceObjectList::default();
    let observed = Rc::new(Cell::new(usize::MAX));
    buffer.push(Rc::new(Value {
        owner: buffer.clone(),
        observed: observed.clone(),
    }));
    let mut snapshot = buffer.iter();
    buffer.clear();
    assert_eq!(observed.get(), usize::MAX);
    drop(snapshot.next());
    assert_eq!(observed.get(), 0);
    assert!(!buffer.ptr_eq(&ResourceObjectList::default()));
    assert!(
        buffer
            .set(
                0,
                Rc::new(Value {
                    owner: buffer.clone(),
                    observed: observed.clone()
                })
            )
            .is_err()
    );
}
