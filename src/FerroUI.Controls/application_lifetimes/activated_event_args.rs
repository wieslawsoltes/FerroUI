use super::{ActivationKind, FileActivatedEventArgs, ProtocolActivatedEventArgs};

/// Event arguments for the activated and deactivated events of an
/// activatable lifetime.
///
/// The arguments of the more specific activations are reached through the
/// `as_*` methods.
#[derive(Clone)]
pub struct ActivatedEventArgs {
    kind: ActivationKind,
    protocol: Option<ProtocolActivatedEventArgs>,
    file: Option<FileActivatedEventArgs>,
}

impl ActivatedEventArgs {
    /// Creates the arguments of an activation of the given kind.
    pub fn new(kind: ActivationKind) -> Self {
        Self { kind, protocol: None, file: None }
    }

    /// The kind of activation.
    pub fn kind(&self) -> ActivationKind {
        self.kind
    }

    /// The arguments as those of a protocol activation, if that is what
    /// they are.
    pub fn as_protocol_activated(&self) -> Option<&ProtocolActivatedEventArgs> {
        self.protocol.as_ref()
    }

    /// The arguments as those of a file activation, if that is what they
    /// are.
    pub fn as_file_activated(&self) -> Option<&FileActivatedEventArgs> {
        self.file.as_ref()
    }
}

impl From<ProtocolActivatedEventArgs> for ActivatedEventArgs {
    fn from(value: ProtocolActivatedEventArgs) -> Self {
        Self { kind: value.kind(), protocol: Some(value), file: None }
    }
}

impl From<FileActivatedEventArgs> for ActivatedEventArgs {
    fn from(value: FileActivatedEventArgs) -> Self {
        Self { kind: value.kind(), protocol: None, file: Some(value) }
    }
}
