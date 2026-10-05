/// Contains the arguments for the startup event of a controlled application
/// lifetime.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ControlledApplicationLifetimeStartupEventArgs {
    args: Vec<String>,
}

impl ControlledApplicationLifetimeStartupEventArgs {
    /// Creates the arguments with the command line arguments that were
    /// passed to the application.
    pub fn new<S: Into<String>>(args: impl IntoIterator<Item = S>) -> Self {
        Self { args: args.into_iter().map(Into::into).collect() }
    }

    /// The command line arguments that were passed to the application.
    pub fn args(&self) -> &[String] {
        &self.args
    }
}
