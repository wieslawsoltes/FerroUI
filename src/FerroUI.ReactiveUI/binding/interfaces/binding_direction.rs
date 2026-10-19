//! `BindingDirection`.

/// The direction of a property binding (`BindingDirection`).
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BindingDirection {
    /// One-way binding from the source to the target (`OneWay`).
    #[default]
    OneWay = 0,
    /// Two-way binding between the source and the target (`TwoWay`).
    TwoWay = 1,
    /// One-way asynchronous binding from the source to the target (`AsyncOneWay`).
    AsyncOneWay = 2,
}
