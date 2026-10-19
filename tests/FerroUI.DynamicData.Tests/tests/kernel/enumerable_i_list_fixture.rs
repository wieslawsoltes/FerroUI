//! `EnumerableIListFixture` (`DynamicData.Tests/EnumerableIListFixture.cs`).
//!
//! Upstream's `EnumerableIList<T>` wraps a `List<T>` as an `IList<T>`. A `Vec<T>` is that list in
//! the port (the module documentation of `kernel`), with the `IList` of DynamicData implemented for
//! it, so the test runs the operations of `list::IList` on a `Vec<u8>`. Each call names the trait,
//! as the inherent methods of `Vec` share some of the names.
//!
//! The bytes come from the seeded generator of the tests, not from `System.Random(1234567)`; the
//! test does not depend on their values but for its first and its last byte differing.

use crate::domain::Randomizer;
use ferroui_dynamic_data::list::IList;

#[test]
fn enumerable_i_list_tests() {
    let rng = Randomizer::new(1_234_567);
    let data: Vec<u8> = (0..40).map(|_| rng.next(0, 256) as u8).collect();

    let mut input_data = vec![0_u8; 39];
    let last_item = data[data.len() - 1];
    let first_item = data[0];
    input_data[..38].copy_from_slice(&data[1..39]);
    // `EnumerableIList.Create(listOfRandomFloats)`: the list itself, through `IList`.
    let mut fixture: Vec<u8> = input_data;
    IList::add(&mut fixture, last_item);
    IList::insert(&mut fixture, 0, first_item);

    assert_eq!(IList::count(&fixture), 41);
    assert_eq!(IList::count(&fixture), fixture.len());

    assert!(IList::index_of(&fixture, &last_item).is_some_and(|index| index > 0));

    IList::remove(&mut fixture, &last_item);

    assert_eq!(IList::count(&fixture), 40);
    assert_eq!(IList::count(&fixture), fixture.len());

    IList::remove_at(&mut fixture, 0);

    assert_eq!(IList::get(&fixture, 10), fixture[10]);

    IList::clear(&mut fixture);

    assert_eq!(IList::count(&fixture), 0);
    assert!(fixture.is_empty());
}

// Not ported: ExceptionTests - the exception types UnspecifiedIndexException, KeySelectorException and MissingKeyException are not types of the port (it panics or returns an ObservableError with their message)
