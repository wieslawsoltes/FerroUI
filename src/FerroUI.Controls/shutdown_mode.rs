/// Describes the possible values for the shutdown mode of the classic
/// desktop style application lifetime.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum ShutdownMode {
    /// Indicates an implicit call to shutdown when the last window closes.
    #[default]
    OnLastWindowClose,

    /// Indicates an implicit call to shutdown when the main window closes.
    OnMainWindowClose,

    /// Indicates that the application only exits on an explicit call to
    /// shutdown.
    OnExplicitShutdown,
}
