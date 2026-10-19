//! `ViewContractAttribute` (ReactiveUI.Binding, `View/ViewContractAttribute.cs`).

/// The contract a view is registered under, so that one view model has a view per contract
/// (`ViewContractAttribute`): an option of a view registration
/// ([`ViewRegistrationOptions`](crate::ViewRegistrationOptions)).
///
/// Deviation (DEVIATIONS.md, ReactiveUI): upstream's attributes on a view class are read by the
/// source generator that builds the view dispatch; here they are plain data given to
/// [`ViewRegistrations`](crate::ViewRegistrations).
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ViewContract {
    contract: String,
}

impl ViewContract {
    /// The contract `contract` (`ViewContractAttribute(string)`).
    pub fn new(contract: impl Into<String>) -> Self {
        Self { contract: contract.into() }
    }

    /// The contract to resolve the view under (`Contract`).
    pub fn contract(&self) -> &str {
        &self.contract
    }
}
