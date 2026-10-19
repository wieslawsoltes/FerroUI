//! `ChainedComparerTest` (`ReactiveUI.Tests/ChainedComparerTest.cs`).

use ferroui_reactiveui::*;
use std::cmp::Ordering;
use std::rc::Rc;

/// `TestClass`.
#[derive(Default)]
struct TestClass {
    priority: i32,
    value: i32,
}

#[test]
fn compare_both_null_returns_zero() {
    // A nullable string is an `Option<String>`; the ordinal comparison finds two `None` equal.
    let comparer = ChainedComparer::<Option<String>>::new(None, |x, y| x.cmp(y));

    let result = comparer.compare(&None, &None);

    assert_eq!(result, Ordering::Equal);
}

#[test]
fn compare_chained_comparers_works_correctly() {
    const LOWER_VALUE: i32 = 10;
    const HIGHER_VALUE: i32 = 20;
    const HIGHER_PRIORITY: i32 = 2;
    const OVERRIDDEN_VALUE: i32 = 5;

    let parent: Comparer<TestClass> = Rc::new(|x: &TestClass, y: &TestClass| x.priority.cmp(&y.priority));
    let comparer = ChainedComparer::new(Some(parent), |x: &TestClass, y: &TestClass| x.value.cmp(&y.value));

    let obj1 = TestClass { priority: 1, value: LOWER_VALUE };
    let obj2 = TestClass { priority: 1, value: HIGHER_VALUE };
    let obj3 = TestClass { priority: HIGHER_PRIORITY, value: OVERRIDDEN_VALUE };

    let result1 = comparer.compare(&obj1, &obj2);
    let result2 = comparer.compare(&obj1, &obj3);

    assert_eq!(result1, Ordering::Less);
    assert_eq!(result2, Ordering::Less);
}

#[test]
fn compare_no_parent_uses_comparison() {
    const SECOND_VALUE: i32 = 2;
    let comparer = ChainedComparer::<i32>::new(None, |x, y| x.cmp(y));

    let result = comparer.compare(&1, &SECOND_VALUE);

    assert_eq!(result, Ordering::Less);
}

#[test]
fn compare_parent_returns_non_zero_uses_parent_result() {
    const SECOND_VALUE: i32 = 2;
    let parent: Comparer<i32> = Rc::new(|x: &i32, y: &i32| x.cmp(y));
    let comparer = ChainedComparer::new(Some(parent), |_: &i32, _: &i32| Ordering::Equal);

    let result = comparer.compare(&1, &SECOND_VALUE);

    assert_eq!(result, Ordering::Less);
}

#[test]
fn compare_parent_returns_zero_uses_comparison() {
    const SECOND_VALUE: i32 = 2;
    let parent: Comparer<TestClass> = Rc::new(|_: &TestClass, _: &TestClass| Ordering::Equal);
    let comparer = ChainedComparer::new(Some(parent), |x: &TestClass, y: &TestClass| x.value.cmp(&y.value));

    let result = comparer.compare(&TestClass { value: 1, ..TestClass::default() }, &TestClass { value: SECOND_VALUE, ..TestClass::default() });

    assert_eq!(result, Ordering::Less);
}
