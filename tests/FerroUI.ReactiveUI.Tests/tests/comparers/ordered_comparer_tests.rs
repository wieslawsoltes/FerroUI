//! `OrderedComparerTests` (`ReactiveUI.Tests/Comparers/OrderedComparerTests.cs`).
//!
//! A comparer is a shared comparison function ([`Comparer`]); `List.Sort(comparer)` is
//! `sort_by(|x, y| comparer(x, y))`, a stable sort where .NET's is not, which the expected orders
//! do not depend on. `StringComparer.Ordinal` compares the characters, and
//! `StringComparer.OrdinalIgnoreCase` their upper-case forms.

use ferroui_reactiveui::*;
use std::cmp::Ordering;
use std::rc::Rc;

/// `TiedSalary`.
const TIED_SALARY: i32 = 100;
/// `TiedAge`.
const TIED_AGE: i32 = 35;

fn ordinal() -> Comparer<String> {
    Rc::new(|x: &String, y: &String| x.cmp(y))
}

fn ordinal_ignore_case() -> Comparer<String> {
    Rc::new(|x: &String, y: &String| x.to_uppercase().cmp(&y.to_uppercase()))
}

fn strings(items: &[&str]) -> Vec<String> {
    items.iter().map(|item| (*item).to_owned()).collect()
}

fn sort<T>(items: &mut [T], comparer: &Comparer<T>) {
    items.sort_by(|x, y| comparer(x, y));
}

/// `SequenceEqual(.., comparer)`.
fn sequence_equal<T>(left: &[T], right: &[T], comparer: &Comparer<T>) -> bool {
    left.len() == right.len() && left.iter().zip(right).all(|(x, y)| comparer(x, y) == Ordering::Equal)
}

#[test]
fn chain_onto_regular_i_comparables() {
    let mut items = strings(&["aaa", "AAA", "abb", "aaaa"]);
    let comparer = ordinal_ignore_case();

    sort(&mut items, &comparer);
    assert!(sequence_equal(&items, &strings(&["AAA", "aaa", "aaaa", "abb"]), &ordinal_ignore_case()));

    sort(&mut items, &comparer.then_by_descending_with(|x: &String| x.clone(), ordinal()));
    assert!(sequence_equal(&items, &strings(&["aaa", "AAA", "aaaa", "abb"]), &ordinal()));
}

#[test]
fn custom_comparer_test() {
    let mut items = strings(&["aaa", "AAA", "abb", "aaaa"]);

    sort(&mut items, &OrderedComparer::<String>::order_by_with(|x: &String| x.clone(), ordinal()));
    assert_eq!(items, strings(&["AAA", "aaa", "aaaa", "abb"]));

    sort(&mut items, &OrderedComparer::<String>::order_by_descending(|x: &String| x.len()).then_by_with(|x: &String| x.clone(), ordinal()));
    assert_eq!(items, strings(&["aaaa", "AAA", "aaa", "abb"]));

    sort(&mut items, &OrderedComparer::<String>::order_by(|x: &String| x.len()).then_by_with(|x: &String| x.clone(), ordinal()));
    assert_eq!(items, strings(&["AAA", "aaa", "abb", "aaaa"]));

    sort(&mut items, &OrderedComparer::<String>::order_by(|x: &String| x.len()).then_by_with(|x: &String| x.clone(), ordinal_ignore_case()));
    assert!(sequence_equal(&items, &strings(&["AAA", "AAA", "abb", "aaaa"]), &ordinal_ignore_case()));
}

/// `Employee`.
#[derive(Debug, Clone, PartialEq)]
struct Employee {
    age: i32,
    name: Option<String>,
    salary: i32,
}

#[test]
fn smoke_test() {
    const ADAM_AGE: i32 = 50;
    const ADAM_SALARY: i32 = 125;
    const ALICE_AGE: i32 = 25;
    const BOB_AGE: i32 = 30;
    const BOB_SALARY: i32 = 75;

    let employee = |name: &str, age: i32, salary: i32| Rc::new(Employee { name: Some(name.to_owned()), age, salary });
    let adam = employee("Adam", ADAM_AGE, ADAM_SALARY);
    let alice = employee("Alice", ALICE_AGE, TIED_SALARY);
    let bob = employee("Bob", BOB_AGE, BOB_SALARY);
    let carol = employee("Carol", TIED_AGE, TIED_SALARY);
    let xavier = employee("Xavier", TIED_AGE, TIED_SALARY);

    let mut employees = vec![adam.clone(), alice.clone(), bob.clone(), carol.clone(), xavier.clone()];

    sort(&mut employees, &OrderedComparer::<Rc<Employee>>::order_by(|x: &Rc<Employee>| x.name.clone()));
    assert_eq!(employees, vec![adam.clone(), alice.clone(), bob.clone(), carol.clone(), xavier.clone()]);

    sort(
        &mut employees,
        &OrderedComparer::<Rc<Employee>>::order_by_descending(|x: &Rc<Employee>| x.age).then_by(|x: &Rc<Employee>| x.name.clone()),
    );
    assert_eq!(employees, vec![adam.clone(), carol.clone(), xavier.clone(), bob.clone(), alice.clone()]);

    let ignore_case: Comparer<Option<String>> =
        Rc::new(|x: &Option<String>, y: &Option<String>| x.as_deref().map(str::to_uppercase).cmp(&y.as_deref().map(str::to_uppercase)));
    sort(
        &mut employees,
        &OrderedComparer::<Rc<Employee>>::order_by_descending(|x: &Rc<Employee>| x.salary)
            .then_by_with(|x: &Rc<Employee>| x.name.clone(), ignore_case),
    );
    assert_eq!(employees, vec![adam.clone(), alice.clone(), carol.clone(), xavier.clone(), bob.clone()]);

    sort(
        &mut employees,
        &OrderedComparer::<Rc<Employee>>::order_by_descending(|x: &Rc<Employee>| x.age)
            .then_by_descending(|x: &Rc<Employee>| x.salary)
            .then_by(|x: &Rc<Employee>| x.name.clone()),
    );
    assert_eq!(employees, vec![adam, carol, xavier, bob, alice]);
}

#[test]
fn works_with_anonymous_types() {
    let source = ["abc", "bcd", "cde"];
    let mut items: Vec<(char, String)> = source.iter().map(|x| (x.chars().next().unwrap(), (*x).to_owned())).collect();

    let comparer = OrderedComparer::for_items(&items).order_by(|x: &(char, String)| x.0);
    sort(&mut items, &comparer);
    assert_eq!(items.iter().map(|x| x.0).collect::<String>(), "abc");
}
