//! Ported from `Tests.System.Reactive/Tests/UnitTest.cs` of Rx.NET.
//! Adapted: `Unit` is Rust's `()`; `Equals("")` and `Equals(null)` are dropped (a unit compares
//! only with a unit); `GetHashCode() == 0` is that hashing a unit writes nothing; `ToString()` is
//! the `Debug` format.

use ferroui_reactive::Unit;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

#[test]
#[allow(clippy::unit_cmp, clippy::unit_hash, clippy::let_unit_value, clippy::nonminimal_bool)]
fn unit() {
    let u1: Unit = ();
    let u2: Unit = ();
    assert!(u1.eq(&u2));
    assert!(u1 == u2);
    assert!(!(u1 != u2));
    let mut hasher = DefaultHasher::new();
    u1.hash(&mut hasher);
    assert_eq!(DefaultHasher::new().finish(), hasher.finish());
    assert_eq!("()", format!("{:?}", u1));
}
