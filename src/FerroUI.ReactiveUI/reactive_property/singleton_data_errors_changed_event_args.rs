//! `SingletonDataErrorsChangedEventArgs` (`ReactiveUI.Core/ReactiveProperty/SingletonDataErrorsChangedEventArgs.cs`).

use ferroui_system::component_model::DataErrorsChangedEventArgs;

/// The argument of the `ErrorsChanged` event that a reactive property raises for its `Value`
/// (`SingletonDataErrorsChangedEventArgs`), shared so that raising it allocates nothing.
#[derive(Debug, Clone, Copy, Default)]
pub struct SingletonDataErrorsChangedEventArgs;

impl SingletonDataErrorsChangedEventArgs {
    /// The argument for the `Value` property (`Value`).
    pub const VALUE: DataErrorsChangedEventArgs = DataErrorsChangedEventArgs::from_static("Value");
}
