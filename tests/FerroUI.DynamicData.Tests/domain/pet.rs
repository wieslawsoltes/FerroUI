//! `Pet` (`Domain/Pet.cs`).

/// A pet: the kind of animal and the name, both optional (`Pet`).
///
/// Deviation (DEVIATIONS.md, DynamicData): a value with public fields, where upstream's class has
/// settable properties; it is only read through `PersonWithRelations::pet`.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct Pet {
    /// The kind of animal (`Animal`).
    pub animal: Option<String>,
    /// The name (`Name`).
    pub name: Option<String>,
}
