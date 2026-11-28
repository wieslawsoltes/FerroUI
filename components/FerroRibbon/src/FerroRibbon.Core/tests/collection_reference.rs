//! Canonical collection allocations, independent of native controls/markup.
use ferroribbon_core::collections::RibbonFixedItems;
use ferroribbon_core::collections::RibbonItems;
use ferroribbon_core::collections::RibbonReadOnlyList;
use std::{any::Any, rc::Rc};

#[test]
fn read_only_views_use_standard_weak_ownership_and_downcasting() {
    use ferroribbon_core::collections::RibbonReadOnlyListImpl;
    use std::rc::Weak;

    let items = RibbonItems::from_values(vec![1, 2]);
    let view: RibbonReadOnlyList<i32> = items.clone();
    let weak: Weak<dyn RibbonReadOnlyListImpl<i32>> = Rc::downgrade(&view);
    assert_eq!(Rc::strong_count(&items), 2);
    assert_eq!(Rc::weak_count(&items), 1);
    let erased: Rc<dyn Any> = view.clone();
    let mutable = erased.downcast::<RibbonItems<i32>>().unwrap();
    assert!(Rc::ptr_eq(&items, &mutable));
    mutable.set(1, 3);
    assert_eq!(view.iter().collect::<Vec<_>>(), vec![1, 3]);
    drop(mutable);
    drop(items);
    assert!(Rc::ptr_eq(&weak.upgrade().unwrap(), &view));
    drop(view);
    assert!(weak.upgrade().is_none());
}

#[test]
fn list_array_and_interface_views_expose_the_same_allocation_without_element_equality() {
    #[derive(Clone, Default)]
    struct NeverCompare;
    impl PartialEq for NeverCompare {
        fn eq(&self, _: &Self) -> bool {
            panic!("object identity must not compare contents")
        }
    }
    let list = RibbonItems::from_values(vec![NeverCompare]);
    let alias = list.clone();
    let read: RibbonReadOnlyList<NeverCompare> = list.clone();
    let actual: Rc<dyn Any> = list.clone();
    assert!(Rc::ptr_eq(
        &actual,
        &(read.clone() as Rc<dyn std::any::Any>)
    ));
    assert!(Rc::ptr_eq(&list.clone(), &alias.clone()));
    assert!(list == alias);
    assert!(list.clone().as_ref() == alias.clone().as_ref());
    assert!(list != RibbonItems::from_values(vec![NeverCompare]));
    let array = RibbonFixedItems::new(vec![NeverCompare]);
    let read: RibbonReadOnlyList<NeverCompare> = array.clone();
    let actual: Rc<dyn Any> = array.clone();
    assert!(Rc::ptr_eq(
        &actual,
        &(read.clone() as Rc<dyn std::any::Any>)
    ));
    assert!(array == array.clone());
    assert!(array != RibbonFixedItems::new(vec![NeverCompare]));
}
#[test]
fn collection_owner_survives_interface_object_and_iterator_then_releases() {
    let source: serde_json::Value = serde_json::from_str(include_str!(
        "../../../tests/fixtures/collection-reference.json"
    ))
    .unwrap();
    let element = Rc::new(7);
    let element_weak = Rc::downgrade(&element);
    let list = RibbonItems::from_values(vec![Some(element)]);
    let owner = Rc::downgrade(&list.clone());
    let view: RibbonReadOnlyList<Option<Rc<i32>>> = list.clone();
    let object = view.clone() as Rc<dyn std::any::Any>;
    let mut iter = list.iter();
    drop(list);
    drop(view);
    drop(object);
    assert!(owner.upgrade().is_some());
    assert!(element_weak.upgrade().is_some());
    assert_eq!(iter.next().flatten().as_deref(), Some(&7));
    assert!(iter.next().is_none());
    assert!(owner.upgrade().is_some());
    drop(iter);
    assert!(owner.upgrade().is_none());
    assert!(element_weak.upgrade().is_none());
    assert_eq!(source["corpus"]["iterator"]["before"], true);
    assert_eq!(source["corpus"]["iterator"]["exhausted"], true);
    assert_eq!(
        source["corpus"]["iterator"]["releasedWhenUnreachable"],
        true
    );
    let empty = RibbonItems::<Option<Rc<i32>>>::new();
    let weak = Rc::downgrade(&empty);
    let mut empty_iter = empty.iter();
    drop(empty);
    assert_eq!(
        weak.upgrade().is_some(),
        source["corpus"]["iterator"]["emptyListRetained"]
    );
    assert!(empty_iter.next().is_none());
    let empty = RibbonFixedItems::<Option<Rc<i32>>>::new(Vec::new());
    let weak = Rc::downgrade(&empty);
    let mut empty_iter = empty.iter();
    drop(empty);
    assert_eq!(
        weak.upgrade().is_some(),
        source["corpus"]["iterator"]["emptyArrayRetained"]
    );
    assert!(empty_iter.next().is_none());
    let array = RibbonFixedItems::new(vec![Some(Rc::new(9))]);
    let weak = Rc::downgrade(&array);
    let view: RibbonReadOnlyList<Option<Rc<i32>>> = array.clone();
    let object = view.clone() as Rc<dyn std::any::Any>;
    drop(array);
    drop(view);
    assert!(weak.upgrade().is_some());
    drop(object);
    assert!(weak.upgrade().is_none());
}

#[test]
fn fixed_items_use_standard_weak_ownership_and_need_no_clone_for_mutation() {
    use std::cell::Cell;
    struct Value(Option<Box<dyn FnOnce()>>);
    impl Drop for Value {
        fn drop(&mut self) {
            if let Some(callback) = self.0.take() {
                callback();
            }
        }
    }
    let values = RibbonFixedItems::new(vec![Value(None), Value(None)]);
    let weak = Rc::downgrade(&values);
    let calls = Rc::new(Cell::new(0));
    let callback_calls = calls.clone();
    let callback_owner = weak.clone();
    values.set(
        0,
        Value(Some(Box::new(move || {
            let owner = callback_owner.upgrade().unwrap();
            assert_eq!(owner.len(), 2);
            // Reentrant mutation must not overlap a retained RefCell borrow.
            owner.set(1, Value(None));
            callback_calls.set(callback_calls.get() + 1);
        }))),
    );
    values.set(0, Value(None));
    assert_eq!(calls.get(), 1);
    assert_eq!(values.len(), 2);
    drop(values);
    assert!(weak.upgrade().is_none());
}

#[test]
fn fixed_item_iterator_reads_shared_replacements_and_retains_the_owner() {
    let values = RibbonFixedItems::new(vec![1, 2]);
    let view: RibbonReadOnlyList<i32> = values.clone();
    let weak = Rc::downgrade(&values);
    let mut iterator = view.iter();
    assert_eq!(iterator.next(), Some(1));
    values.set(1, 3);
    assert_eq!(view.get(1), 3);
    drop(values);
    drop(view);
    assert!(weak.upgrade().is_some());
    assert_eq!(iterator.next(), Some(3));
    assert_eq!(iterator.next(), None);
    assert!(weak.upgrade().is_some());
    drop(iterator);
    assert!(weak.upgrade().is_none());
}
