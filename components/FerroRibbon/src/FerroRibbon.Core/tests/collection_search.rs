use ferroui_system::collections::object_model::{CollectionItemView, ObservableCollection};
use serde_json::Value;
use std::{
    cell::{Cell, RefCell},
    panic::{AssertUnwindSafe, catch_unwind, resume_unwind},
    rc::Rc,
};
fn corpus() -> Value {
    serde_json::from_str(include_str!(
        "../../../tests/fixtures/collection-search.json"
    ))
    .unwrap()
}
fn panic_text(error: &Box<dyn std::any::Any + Send>) -> &str {
    error
        .downcast_ref::<String>()
        .map(String::as_str)
        .or_else(|| error.downcast_ref::<&str>().copied())
        .unwrap_or("unrecognized panic")
}
fn outcome(run: impl FnOnce() -> String) -> String {
    match catch_unwind(AssertUnwindSafe(run)) {
        Ok(v) => v,
        Err(e) => {
            let message = panic_text(&e);
            if message.contains("comparison") {
                "InvalidOperationException".into()
            } else if message.contains("index out of range") {
                "ArgumentOutOfRangeException".into()
            } else {
                resume_unwind(e)
            }
        }
    }
}
#[derive(Clone)]
struct Item {
    id: i32,
    callback: Option<Rc<dyn Fn(i32)>>,
}
impl PartialEq for Item {
    fn eq(&self, other: &Self) -> bool {
        if let Some(callback) = &self.callback {
            callback(self.id);
        }
        self.id == other.id
    }
}
type Reference = Option<Rc<Item>>;
fn reference(id: i32, callback: Option<Rc<dyn Fn(i32)>>) -> Reference {
    Some(Rc::new(Item { id, callback }))
}
#[test]
fn source_reference_array_search_matrix_matches_mutation_and_error_timing() {
    for row in corpus()["cases"].as_array().unwrap() {
        let items = Rc::new(ObservableCollection::<Reference>::new_managed());
        let trace = Rc::new(RefCell::new(vec![]));
        let log = trace.clone();
        let weak = Rc::downgrade(&items);
        let mutation = row["mutation"].as_str().unwrap().to_owned();
        let changed = Cell::new(false);
        let callback = Rc::new(move |value| {
            log.borrow_mut().push(format!("equals:{value}"));
            if changed.replace(true) {
                return;
            }
            let items = weak.upgrade().unwrap();
            let callback = items.get(0).unwrap().callback.clone();
            let make = |id| reference(id, callback.clone());
            match mutation.as_str() {
                "append" => items.add(make(9)),
                "grow" => {
                    for _ in 0..8 {
                        items.add(make(9));
                    }
                    items.set(1, make(9));
                }
                "replace-next" => items.set(1, make(9)),
                "remove-first" => items.remove_at(0),
                "clear" => items.clear(),
                "insert-first" => items.insert(0, make(9)),
                "move-last" => items.move_item(1, 0),
                "replace-current" => items.set(0, make(9)),
                "remove-last" => items.remove_at(1),
                "nested" => {
                    let result = items.index_of(&reference(2, None)).map_or(-1, |v| v as i32);
                    log.borrow_mut().push(format!("nested:{result}"));
                }
                "throw" => {
                    items.add(make(9));
                    panic!("comparison");
                }
                _ => panic!("unrecognized mutation"),
            }
        });
        items.add(reference(1, Some(callback.clone())));
        items.add(reference(2, Some(callback)));
        let query = reference(9, None);
        let result = outcome(|| match row["operation"].as_str().unwrap() {
            "contains" => items.contains(&query).to_string(),
            "index" => items.index_of(&query).map_or(-1, |v| v as i32).to_string(),
            "remove" => items.remove(&query).to_string(),
            _ => panic!("unrecognized operation"),
        });
        assert_eq!(result, row["result"].as_str().unwrap(), "{row}");
        assert_eq!(serde_json::json!(*trace.borrow()), row["trace"], "{row}");
        assert_eq!(
            serde_json::json!(items.iter().map(|v| v.map(|v| v.id)).collect::<Vec<_>>()),
            row["items"],
            "{row}"
        );
    }
}
#[test]
fn source_cleared_reference_slots_do_not_invoke_equals() {
    for row in corpus()["nullCases"].as_array().unwrap() {
        let items = Rc::new(ObservableCollection::<Reference>::new_managed());
        let weak = Rc::downgrade(&items);
        let mutation = row["mutation"].as_str().unwrap().to_owned();
        let changed = Cell::new(false);
        let callback = Rc::new(move |_| {
            if changed.replace(true) {
                return;
            }
            let items = weak.upgrade().unwrap();
            if mutation == "clear" {
                items.clear();
            } else {
                items.remove_at(if mutation == "remove-last" { 1 } else { 0 });
            }
        });
        items.add(reference(1, Some(callback)));
        items.add(reference(2, None));
        let result = items.index_of(&reference(2, None)).map_or(-1, |v| v as i32);
        assert_eq!(serde_json::json!(result), row["result"]);
        assert_eq!(serde_json::json!(items.count()), row["count"]);
    }
}
// One native representation deliberately serves both source structs; policy is
// declared explicitly, not inferred from Rust needs_drop or current marker value.
#[derive(Clone, Default)]
struct ValueItem {
    id: i32,
    _marker: Option<Rc<()>>,
}
type ValueCallback = Rc<dyn Fn(i32)>;
ferroui_system::ferro_thread_local! {static VALUE_COMPARE:RefCell<Option<ValueCallback>>=RefCell::new(None);}
impl PartialEq for ValueItem {
    fn eq(&self, other: &Self) -> bool {
        let callback = VALUE_COMPARE.with(|c| c.borrow().clone());
        if let Some(callback) = callback {
            callback(self.id);
        }
        self.id == other.id
    }
}
#[test]
fn source_reference_containing_and_plain_struct_slots_remain_distinct() {
    for row in corpus()["valueCases"].as_array().unwrap() {
        let managed = row["managed"].as_bool().unwrap();
        fn viewed_equals(left: &CollectionItemView<'_, ValueItem>, right: &ValueItem) -> bool {
            let callback = VALUE_COMPARE.with(|c| c.borrow().clone());
            if let Some(callback) = callback {
                callback(left.read().id);
            }
            left.read().id == right.id
        }
        let items = if managed {
            ObservableCollection::from_items_managed_with_comparer(vec![], viewed_equals)
        } else {
            ObservableCollection::from_items_unmanaged_with_comparer(vec![], viewed_equals)
        };
        let make = move |id| ValueItem {
            id,
            _marker: managed.then(|| Rc::new(())),
        };
        items.add(make(1));
        items.add(make(2));
        let target = items.clone();
        let trace = Rc::new(RefCell::new(vec![]));
        let output = trace.clone();
        let mutation = row["mutation"].as_str().unwrap().to_owned();
        let changed = Cell::new(false);
        VALUE_COMPARE.with(|c| {
            *c.borrow_mut() = Some(Rc::new(move |value| {
                output.borrow_mut().push(format!("equals:{value}"));
                if changed.replace(true) {
                    return;
                }
                match mutation.as_str() {
                    "append" => target.add(make(9)),
                    "grow" => {
                        for _ in 0..8 {
                            target.add(make(9));
                        }
                        target.set(1, make(9));
                    }
                    "replace-next" => target.set(1, make(9)),
                    "remove-first" => target.remove_at(0),
                    "remove-last" => target.remove_at(1),
                    "clear" => target.clear(),
                    "insert-first" => target.insert(0, make(9)),
                    "move-last" => target.move_item(1, 0),
                    "replace-current" => target.set(0, make(9)),
                    "nested" => {
                        let result = target.index_of(&make(2)).map_or(-1, |v| v as i32);
                        output.borrow_mut().push(format!("nested:{result}"));
                    }
                    "throw" => {
                        target.add(make(9));
                        panic!("comparison");
                    }
                    _ => panic!("unrecognized mutation"),
                }
            }))
        });
        let query = make(row["query"].as_i64().unwrap() as i32);
        let result = outcome(|| match row["operation"].as_str().unwrap() {
            "contains" => items.contains(&query).to_string(),
            "index" => items.index_of(&query).map_or(-1, |v| v as i32).to_string(),
            "remove" => items.remove(&query).to_string(),
            _ => panic!("unrecognized operation"),
        });
        VALUE_COMPARE.with(|c| c.borrow_mut().take());
        assert_eq!(result, row["result"].as_str().unwrap(), "{row}");
        assert_eq!(serde_json::json!(*trace.borrow()), row["trace"], "{row}");
        assert_eq!(
            serde_json::json!(items.iter().map(|v| v.id).collect::<Vec<_>>()),
            row["items"],
            "{row}"
        );
    }
}
