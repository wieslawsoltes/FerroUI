use ferroribbon_core::{
    RibbonString,
    collections::{RibbonItems, RibbonReadOnlyList},
    state::RibbonState,
};
use serde_json::{Value, json};
use std::{
    cell::Cell,
    panic::{AssertUnwindSafe, catch_unwind},
    rc::{Rc, Weak},
};

fn strings(values: Vec<Option<RibbonString>>) -> Value {
    json!(
        values
            .into_iter()
            .map(|v| v.map(|v| v.to_utf8().unwrap()))
            .collect::<Vec<_>>()
    )
}
#[test]
fn source_state_aliases_and_live_queries_match_all_48_mutation_traces() {
    let rows: Value =
        serde_json::from_str(include_str!("../../../tests/fixtures/shared-items.json")).unwrap();
    for row in rows.as_array().unwrap() {
        let action = row["action"].as_str().unwrap();
        let values = if row["empty"].as_bool().unwrap() {
            vec![]
        } else if action == "normalize" {
            vec![Some("a".into()), None, Some("".into()), Some("b".into())]
        } else {
            vec![Some("a".into()), Some("b".into())]
        };
        let list = RibbonItems::from_values(values);
        let state = RibbonState::new();
        state.set_quick_access_item_ids(Some(list.clone()));
        state.set_recent_search_ids(Some(list.clone()));
        let mut active = list.iter();
        let mut prefix = vec![];
        match row["phase"].as_str().unwrap() {
            "new" => {}
            "first" => {
                if let Some(v) = active.next() {
                    prefix.push(v);
                }
            }
            "end" => prefix.extend(&mut active),
            _ => unreachable!(),
        }
        let mutation = catch_unwind(AssertUnwindSafe(|| match action {
            "add" => list.add(Some("c".into())),
            "set" => list.set(0, Some("a".into())),
            "insert" => list.insert(0, Some("c".into())),
            "remove" => {
                list.remove(&Some("a".into()));
            }
            "missing" => {
                list.remove(&Some("absent".into()));
            }
            "clear" => list.clear(),
            "normalize" | "normalizeKeep" => state.normalize(),
            _ => unreachable!(),
        }));
        // C# exception names are a development-fixture observation, not runtime types.
        let mutation_error = mutation.err().map(|_| "ArgumentOutOfRangeException");
        let mut suffix = vec![];
        let query = catch_unwind(AssertUnwindSafe(|| suffix.extend(active)));
        let query_error = query.err().map(|_| "InvalidOperationException");
        assert_eq!(strings(prefix), row["prefix"], "{row}");
        assert_eq!(json!(mutation_error), row["mutationError"], "{row}");
        assert_eq!(strings(suffix), row["suffix"], "{row}");
        assert_eq!(json!(query_error), row["queryError"], "{row}");
        assert_eq!(strings(list.to_vec()), row["values"], "{row}");
        assert_eq!(
            json!(Rc::ptr_eq(
                &state.quick_access_item_ids().unwrap(),
                &state.recent_search_ids().unwrap()
            )),
            row["same"]
        );
    }
}

#[test]
fn plain_values_need_neither_default_nor_equality_and_readonly_view_owns_the_actual_rc() {
    #[derive(Clone)]
    struct Payload(u32);
    let list = RibbonItems::from_values(vec![Payload(7)]);
    let view: RibbonReadOnlyList<Payload> = list.clone();
    let actual: Rc<dyn std::any::Any> = list.clone();
    assert!(Rc::ptr_eq(
        &(view.clone() as Rc<dyn std::any::Any>),
        &actual
    ));
    let weak = Rc::downgrade(&list);
    drop(list);
    assert_eq!(view.iter().next().unwrap().0, 7);
    assert!(weak.upgrade().is_some());
    drop(view);
    drop(actual);
    assert!(weak.upgrade().is_none());
}

#[test]
fn replacing_removing_and_clearing_release_values_after_the_mutation_borrow() {
    struct OnDrop {
        owner: Weak<RibbonItems<Self>>,
        calls: Rc<Cell<usize>>,
        armed: bool,
    }
    impl Drop for OnDrop {
        fn drop(&mut self) {
            if self.armed {
                self.calls.set(self.calls.get() + 1);
                if let Some(owner) = self.owner.upgrade() {
                    // Reenter mutation from the dropped value without a storage borrow.
                    owner.add(OnDrop {
                        owner: Weak::new(),
                        calls: self.calls.clone(),
                        armed: false,
                    });
                }
            }
        }
    }
    let list = RibbonItems::new();
    let calls = Rc::new(Cell::new(0));
    let push = || {
        list.add(OnDrop {
            owner: Rc::downgrade(&list),
            calls: calls.clone(),
            armed: true,
        })
    };
    push();
    list.set(
        0,
        OnDrop {
            owner: Weak::new(),
            calls: calls.clone(),
            armed: false,
        },
    );
    assert_eq!(calls.get(), 1);
    push();
    list.remove_at(list.len() - 1);
    assert_eq!(calls.get(), 2);
    push();
    list.clear();
    assert_eq!(calls.get(), 3);
    assert_eq!(list.len(), 1);
    let weak = Rc::downgrade(&list);
    drop(list);
    assert!(weak.upgrade().is_none());
}
