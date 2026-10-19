//! `ViewMappingKey` (ReactiveUI.Binding, `View/ViewMappingKey.cs`).

use std::any::TypeId;

/// One registered view mapping: the view model it serves, under which contract (empty for the
/// default) (`ViewMappingKey`).
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) struct ViewMappingKey {
    /// The view model type the mapping resolves a view for (`ViewModelType`).
    pub(crate) view_model_type: TypeId,
    /// The contract the mapping is registered under, empty for the default (`Contract`).
    pub(crate) contract: String,
}

impl ViewMappingKey {
    /// The key of `view_model_type` under `contract` (`None` is the empty contract).
    pub(crate) fn new(view_model_type: TypeId, contract: Option<&str>) -> Self {
        Self { view_model_type, contract: contract.unwrap_or_default().to_owned() }
    }
}
