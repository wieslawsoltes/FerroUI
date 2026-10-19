//! `DummyObjectClass1`.

use super::IDummyInterface;
use std::fmt;

/// A type of the tests; its text is the name of its type, as `object.ToString` gives it.
#[derive(Debug, Default)]
pub struct DummyObjectClass1;

impl IDummyInterface for DummyObjectClass1 {}

impl fmt::Display for DummyObjectClass1 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(std::any::type_name::<Self>())
    }
}
