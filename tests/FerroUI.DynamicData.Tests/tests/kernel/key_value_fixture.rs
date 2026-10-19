//! `KeyValueFixture` (`DynamicData.Tests/Kernal/KeyValueFixture.cs`): a key and its item.
//!
//! `KeyValuePair<TKey, TValue>` is a tuple `(K, V)` in the port.

use crate::domain::Person;
use std::rc::Rc;

#[test]
fn create() {
    let person = Person::new("Person", 10);
    let kv: (String, Rc<Person>) = ("Person".to_string(), person.clone());

    assert_eq!(kv.0, "Person");
    assert_eq!(kv.1, person);
}
