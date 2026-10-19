//! Tests of `collections::object_model::ObservableCollection`.

use crate::collections::object_model::ObservableCollection;
use crate::collections::specialized::{NotifyCollectionChangedAction, NotifyCollectionChangedEventArgs};
use crate::component_model::INotifyPropertyChanged;
use std::cell::RefCell;
use std::rc::Rc;

#[test]
fn notifies_in_the_order_of_dotnet() {
    let collection = ObservableCollection::from_items(vec![1, 2]);
    let log = Rc::new(RefCell::new(Vec::new()));
    let target = log.clone();
    collection.property_changed().add(Rc::new(move |name: &str| target.borrow_mut().push(format!("property {name}"))));
    let target = log.clone();
    collection.add_collection_changed(Rc::new(move |args: &NotifyCollectionChangedEventArgs<i32>| {
        target.borrow_mut().push(format!("{:?} {:?} {:?} {} {}", args.action, args.new_items, args.old_items, args.new_starting_index, args.old_starting_index));
    }));
    collection.add(3);
    collection.move_item(0, 2);
    collection.set(0, 9);
    collection.remove_at(1);
    collection.clear();
    assert_eq!(
        *log.borrow(),
        vec![
            "property Count", "property Item[]", "Add [3] [] 2 -1",
            "property Item[]", "Move [1] [1] 2 0",
            "property Item[]", "Replace [9] [2] 0 0",
            "property Count", "property Item[]", "Remove [] [3] -1 1",
            "property Count", "property Item[]", "Reset [] [] -1 -1",
        ]
    );
    assert_eq!(NotifyCollectionChangedAction::Reset, NotifyCollectionChangedAction::Reset);
}

#[test]
fn a_change_during_a_notification_with_two_handlers_panics() {
    let collection = ObservableCollection::<i32>::new();
    let inner = collection.clone();
    collection.add_collection_changed(Rc::new(move |args: &NotifyCollectionChangedEventArgs<i32>| {
        if args.action == NotifyCollectionChangedAction::Add {
            inner.remove_at(0);
        }
    }));
    collection.add_collection_changed(Rc::new(|_: &NotifyCollectionChangedEventArgs<i32>| {}));
    assert!(std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| collection.add(1))).is_err());
}
