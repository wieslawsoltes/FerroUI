//! Focused selected-System defect reproducer. No Ribbon model/control implementation involved.
use ferroribbon_core::collections::ObservableCollection;
use std::rc::Rc;
#[derive(Clone)]
struct Value {
    value: i32,
    compare: Option<Rc<dyn Fn()>>,
}
impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        if let Some(compare) = &self.compare {
            compare();
        }
        self.value == other.value
    }
}
#[test]
fn source_system_collection_equality_reentry_allows_append() {
    let source: serde_json::Value = serde_json::from_str(include_str!("../nodes.json")).unwrap();
    assert_eq!(
        source["collectionTrace"],
        serde_json::json!(["equals", "result:true", "count:2"])
    );
    let items = Rc::new(ObservableCollection::new());
    let weak = Rc::downgrade(&items);
    items.add(Value {
        value: 1,
        compare: Some(Rc::new(move || {
            weak.upgrade().unwrap().add(Value {
                value: 2,
                compare: None,
            })
        })),
    });
    // Comparisons now run without a storage borrow, preserving the source callback.
    let contained = items.contains(&Value {
        value: 1,
        compare: None,
    });
    assert!(contained);
    assert_eq!(items.count(), 2);
}
