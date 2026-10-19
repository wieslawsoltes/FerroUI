//! `SingletonPropertyChangedEventArgs` (`ReactiveUI.Core/ReactiveProperty/SingletonPropertyChangedEventArgs.cs`).

/// The names of the properties whose changes a reactive property raises again and again
/// (`SingletonPropertyChangedEventArgs`).
///
/// Deviation (DEVIATIONS.md, ReactiveUI): upstream caches `PropertyChangedEventArgs` objects. The
/// `PropertyChanged` event of `ferroui-system` carries the name of the property itself
/// (`Event<str>`), so the shared arguments are the names, which allocate nothing.
#[derive(Debug, Clone, Copy, Default)]
pub struct SingletonPropertyChangedEventArgs;

impl SingletonPropertyChangedEventArgs {
    /// The name of the `Value` property (`Value`).
    pub const VALUE: &'static str = "Value";

    /// The name of the `HasErrors` property of `INotifyDataErrorInfo` (`HasErrors`).
    pub const HAS_ERRORS: &'static str = "HasErrors";

    /// The name of the `ErrorMessage` property (`ErrorMessage`).
    pub const ERROR_MESSAGE: &'static str = "ErrorMessage";
}
