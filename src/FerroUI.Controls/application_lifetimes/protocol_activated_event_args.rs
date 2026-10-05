use super::ActivationKind;
use ferroui_base::utilities::Uri;

/// The arguments of an activation in which the application is passed a URI
/// to open. Convert them to
/// [`ActivatedEventArgs`](super::ActivatedEventArgs) to raise the event.
#[derive(Clone)]
pub struct ProtocolActivatedEventArgs {
    uri: Uri,
}

impl ProtocolActivatedEventArgs {
    /// Creates the arguments with the URI the application was activated
    /// with.
    pub fn new(uri: Uri) -> Self {
        Self { uri }
    }

    /// The kind of activation: [`ActivationKind::OpenUri`].
    pub fn kind(&self) -> ActivationKind {
        ActivationKind::OpenUri
    }

    /// The URI the application was activated with.
    pub fn uri(&self) -> &Uri {
        &self.uri
    }
}
