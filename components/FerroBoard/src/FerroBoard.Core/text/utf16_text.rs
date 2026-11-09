//! Immutable keyboard text measured in UTF-16 code units.
//!
//! Editing offsets can split surrogate pairs, so a Rust `String` cannot represent
//! every intermediate editor value. This buffer owns those units directly. It
//! has no interning, runtime string services or control-layer dependency.
use std::{string::FromUtf16Error, sync::Arc};

#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct Utf16Text(Arc<[u16]>);

impl Utf16Text {
    pub fn from_units(units: impl Into<Arc<[u16]>>) -> Self {
        Self(units.into())
    }

    pub fn units(&self) -> &[u16] {
        &self.0
    }

    /// Number of UTF-16 code units, not scalar values or graphemes.
    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Converts at a UTF-8 boundary, rejecting unpaired surrogates.
    pub fn to_utf8(&self) -> Result<String, FromUtf16Error> {
        String::from_utf16(&self.0)
    }

    /// Whether two buffers share the same allocation. Text equality uses units.
    pub fn ptr_eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}

impl From<&str> for Utf16Text {
    fn from(value: &str) -> Self {
        Self(value.encode_utf16().collect())
    }
}
