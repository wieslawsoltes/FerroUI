//! `ConverterTypePair` (`ReactiveUI.Binding/Bindings/Converters/ConverterTypePair.cs`).

use crate::ConverterType;
use std::fmt;

/// The source and target types of a typed converter: the key of
/// [`BindingTypeConverterRegistry`](crate::BindingTypeConverterRegistry) (`ConverterTypePair`).
// Deviation (DEVIATIONS.md, ReactiveUI): upstream's record struct is internal; it is public here
// because the shallow copy of the registry that its tests call names it.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct ConverterTypePair {
    /// The type converted from (`FromType`).
    pub from_type: ConverterType,
    /// The type converted to (`ToType`).
    pub to_type: ConverterType,
}

impl ConverterTypePair {
    /// The pair of `from_type` and `to_type`.
    pub fn new(from_type: ConverterType, to_type: ConverterType) -> Self {
        ConverterTypePair { from_type, to_type }
    }
}

impl fmt::Debug for ConverterTypePair {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ConverterTypePair: {} -> {}", self.from_type, self.to_type)
    }
}
