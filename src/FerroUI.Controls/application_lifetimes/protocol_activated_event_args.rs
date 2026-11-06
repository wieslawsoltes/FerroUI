use super::ActivationKind;
use url::Url;

/// The arguments of an activation in which the application is passed a URI
/// to open. Convert them to
/// [`ActivatedEventArgs`](super::ActivatedEventArgs) to raise the event.
#[derive(Clone)]
pub struct ProtocolActivatedEventArgs {
    uri: Url,
}

impl ProtocolActivatedEventArgs {
    /// Creates the arguments with the URI the application was activated
    /// with.
    pub fn new(uri: Url) -> Self {
        Self { uri }
    }

    /// The kind of activation: [`ActivationKind::OpenUri`].
    pub fn kind(&self) -> ActivationKind {
        ActivationKind::OpenUri
    }

    /// The URI the application was activated with.
    pub fn uri(&self) -> &Url {
        &self.uri
    }
}
