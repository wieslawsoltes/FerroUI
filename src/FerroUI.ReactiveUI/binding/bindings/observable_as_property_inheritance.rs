//! `ObservableAsPropertyInheritance`.

/// The modifier that upstream's code fix for `[ObservableAsProperty]` gives the property it writes
/// for a field (`ObservableAsPropertyInheritance`).
///
/// Rust has no inheritance; the enumeration is kept for the attribute's counterpart, a macro, to
/// name the same choices.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ObservableAsPropertyInheritance {
    /// No modifier (`None`).
    #[default]
    None = 0,
    /// The property is `virtual` (`Virtual`).
    Virtual = 1,
    /// The property is an `override` (`Override`).
    Override = 2,
    /// The property hides an inherited member with `new` (`New`).
    New = 3,
}
