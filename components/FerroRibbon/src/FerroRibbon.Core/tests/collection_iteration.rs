use ferroribbon_core::collections::ObservableCollection;
use ferroui_system::{
    component_model::INotifyPropertyChanged,
};
use serde_json::{Value, json};
use std::{
    panic::{AssertUnwindSafe, catch_unwind},
    rc::Rc,
};
fn source() -> Value {
    serde_json::from_str(include_str!(
        "../../../tests/fixtures/collection-iteration.json"
    ))
    .unwrap()
}
#[test]
fn source_collection_iterator_mutation_matrix_preserves_failure_timing() {
    for row in source()["cases"].as_array().unwrap() {
        let items = ObservableCollection::from_items(vec![1, 2]);
        let mut iterator = items.iter();
        let mut prefix = vec![];
        match row["position"].as_str().unwrap() {
            "one" => prefix.push(iterator.next().unwrap()),
            "end" => prefix.extend(iterator.by_ref()),
            _ => {}
        }
        let op = row["operation"].as_str().unwrap();
        if op == "notify-throw" {
            items.property_changed().add(Rc::new(|_| panic!("handler")));
        }
        let mutation = catch_unwind(AssertUnwindSafe(|| match op {
            "add" | "notify-throw" => items.add(3),
            "set-same" => items.set(0, 1),
            "set" => items.set(0, 9),
            "remove" => items.remove_at(0),
            "remove-missing" => {
                items.remove(&99);
            }
            "move" => items.move_item(0, 1),
            "move-same" => items.move_item(0, 0),
            "clear" => items.clear(),
            "insert-invalid" => items.insert(99, 3),
            "move-invalid-old" => items.move_item(99, 0),
            "move-invalid-new" => items.move_item(0, 99),
            _ => panic!("unknown reference operation"),
        }));
        assert_eq!(mutation.is_err(), !row["mutationError"].is_null(), "{row}");
        let mut rest = vec![];
        let outcome = catch_unwind(AssertUnwindSafe(|| rest.extend(iterator.by_ref())));
        assert_eq!(outcome.is_err(), !row["error"].is_null(), "{row}");
        assert_eq!(json!(prefix), row["prefix"], "{row}");
        assert_eq!(json!(rest), row["rest"], "{row}");
        assert_eq!(json!(items.to_vec()), row["items"], "{row}");
    }
}
#[test]
fn empty_iterator_and_deferred_iterator_creation_match_source() {
    let data = source();
    let empty = ObservableCollection::<i32>::new();
    let mut iterator = empty.iter();
    empty.clear();
    assert!(data["emptyError"].is_null());
    empty.add(1);
    assert_eq!(
        iterator.next().is_some(),
        data["emptyMovedNext"].as_bool().unwrap()
    );
    let items = ObservableCollection::from_items(vec![1]);
    let mut state = None;
    let owner = items.clone();
    let lazy = std::iter::from_fn(move || {
        let iter = state.get_or_insert_with(|| owner.iter());
        iter.next()
    });
    items.add(2);
    assert_eq!(json!(lazy.collect::<Vec<_>>()), data["lazyValues"]);
}
