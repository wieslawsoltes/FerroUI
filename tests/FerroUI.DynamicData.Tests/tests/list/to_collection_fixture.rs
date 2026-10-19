//! `ToCollectionFixture` of lists (`DynamicData.Tests/List/ToCollectionFixture.cs`).

use ferroui_dynamic_data::list::{IObservableList, ObservableListEx, SourceList};
use ferroui_reactive::ObservableExt;
use std::cell::RefCell;
use std::rc::Rc;

#[test]
fn to_collection_test() {
    let list = SourceList::<String>::new();
    let collection = list.connect().to_collection();

    let res1: Rc<RefCell<Option<Vec<String>>>> = Rc::new(RefCell::new(None));
    let res2: Rc<RefCell<Option<Vec<String>>>> = Rc::new(RefCell::new(None));

    let target = res1.clone();
    let _subscription1 = collection.subscribe_next(move |x| *target.borrow_mut() = Some(x));
    let target = res2.clone();
    let _subscription2 = collection.subscribe_next(move |x| *target.borrow_mut() = Some(x));

    list.edit(|l| l.add("1".to_owned()));
    list.edit(|l| l.add("2".to_owned()));

    assert_eq!(res1.borrow().as_ref().map(Vec::len), Some(2));
    assert_eq!(res2.borrow().as_ref().map(Vec::len), Some(2));
}
