/// A lifetime that is notified while the application builder sets the
/// application up.
pub trait ISetupApplicationLifetime {
    /// Called before the platform services are set up and the application is
    /// created.
    fn before_app_init(&self);

    /// Called after the application has been created and initialized.
    fn after_app_init(&self);
}
