//! `OrderedComparerTest` (`ReactiveUI.Tests/OrderedComparerTest.cs`).
//!
//! `comparer.Compare(x, y)` is a call of the comparison function, which gives an [`Ordering`];
//! upstream's "not null" checks are the creation of the values.

use ferroui_reactiveui::*;
use std::cmp::Ordering;
use std::rc::Rc;

/// `SecondItemValue`.
const SECOND_ITEM_VALUE: i32 = 2;

/// `TestClass`.
struct TestClass {
    value: i32,
}

fn reverse_order() -> Comparer<i32> {
    // Reverse order.
    Rc::new(|x: &i32, y: &i32| y.cmp(x))
}

#[test]
fn builder_order_by_creates_comparer() {
    let builder = OrderedComparer::<TestClass>::for_type();
    let comparer = builder.order_by(|x: &TestClass| x.value);
    assert_eq!(comparer(&TestClass { value: 1 }, &TestClass { value: 1 }), Ordering::Equal);
}

#[test]
fn builder_order_by_descending_creates_comparer() {
    let builder = OrderedComparer::<TestClass>::for_type();
    let comparer = builder.order_by_descending(|x: &TestClass| x.value);
    assert_eq!(comparer(&TestClass { value: 1 }, &TestClass { value: 1 }), Ordering::Equal);
}

#[test]
fn for_generic_returns_builder() {
    let builder = OrderedComparer::<TestClass>::for_type();
    let comparer = builder.order_by(|x: &TestClass| x.value);
    assert_eq!(comparer(&TestClass { value: 1 }, &TestClass { value: SECOND_ITEM_VALUE }), Ordering::Less);
}

#[test]
fn for_with_enumerable_returns_builder() {
    let items = [TestClass { value: 1 }];
    let builder = OrderedComparer::for_items(&items);
    let comparer = builder.order_by(|x: &TestClass| x.value);
    assert_eq!(comparer(&items[0], &items[0]), Ordering::Equal);
}

#[test]
fn order_by_sorts_ascending() {
    let comparer = OrderedComparer::<TestClass>::order_by(|x: &TestClass| x.value);
    let obj1 = TestClass { value: 1 };
    let obj2 = TestClass { value: SECOND_ITEM_VALUE };

    let result = comparer(&obj1, &obj2);

    assert_eq!(result, Ordering::Less);
}

#[test]
fn order_by_with_custom_comparer_uses_custom_comparer() {
    let comparer = OrderedComparer::<TestClass>::order_by_with(|x: &TestClass| x.value, reverse_order());
    let obj1 = TestClass { value: 1 };
    let obj2 = TestClass { value: SECOND_ITEM_VALUE };

    let result = comparer(&obj1, &obj2);

    assert_eq!(result, Ordering::Greater);
}

#[test]
fn order_by_descending_sorts_descending() {
    let comparer = OrderedComparer::<TestClass>::order_by_descending(|x: &TestClass| x.value);
    let obj1 = TestClass { value: 1 };
    let obj2 = TestClass { value: SECOND_ITEM_VALUE };

    let result = comparer(&obj1, &obj2);

    assert_eq!(result, Ordering::Greater);
}

#[test]
fn order_by_descending_with_custom_comparer_uses_custom_comparer() {
    let comparer = OrderedComparer::<TestClass>::order_by_descending_with(|x: &TestClass| x.value, reverse_order());
    let obj1 = TestClass { value: 1 };
    let obj2 = TestClass { value: SECOND_ITEM_VALUE };

    let result = comparer(&obj1, &obj2);

    assert_eq!(result, Ordering::Less);
}
