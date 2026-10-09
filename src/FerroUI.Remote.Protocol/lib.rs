//! The remote protocol: the wire protocol of the previewer and of remote
//! rendering.
//!
//! - The messages ([`viewport`], [`input`], [`designer`] and
//!   [`HtmlTransportStartedMessage`]). Each message class has an identifier
//!   ([`FerroRemoteMessageGuidAttribute`]) by which the two ends name it; a
//!   resolver ([`IMessageTypeResolver`], [`DefaultMessageTypeResolver`])
//!   maps identifiers to classes.
//! - The serialization of a message as a BSON document ([`metsys_bson`]).
//!
//! The library is a leaf, as the upstream project is: it uses nothing of the
//! framework, so that a tool that only talks the protocol can use it alone.

pub mod metsys_bson;

mod assembly;
mod default_message_type_resolver;
mod design_messages;
mod error;
mod ferro_remote_message_guid_attribute;
mod guid;
mod i_message_type_resolver;
mod input_messages;
mod transport_messages;
mod viewport_messages;

// The two key enumerations of the base library are compiled into this
// library, as the upstream project file does (`<Compile Include>` of
// `Input/Key.cs` and `Input/PhysicalKey.cs`, which put the types in the
// input namespace of the protocol when they are built here): the library
// stays a leaf and the numbers of the keys cannot drift apart.
#[path = "../FerroUI.Base/input/key.rs"]
mod key;
#[path = "../FerroUI.Base/input/physical_key.rs"]
mod physical_key;

#[cfg(test)]
mod metsys_bson_tests;

/// `FerroUI.Remote.Protocol.Viewport`.
pub mod viewport {
    pub use crate::viewport_messages::*;
}

/// `FerroUI.Remote.Protocol.Input`.
pub mod input {
    pub use crate::input_messages::*;
    pub use crate::key::{Key, ParseKeyError};
    pub use crate::physical_key::{ParsePhysicalKeyError, PhysicalKey};
}

/// `FerroUI.Remote.Protocol.Designer`.
pub mod designer {
    pub use crate::design_messages::*;
}

pub use assembly::{Assembly, ExportedType, ASSEMBLY};
pub use default_message_type_resolver::DefaultMessageTypeResolver;
pub use error::Error;
pub use ferro_remote_message_guid_attribute::{FerroRemoteMessage, FerroRemoteMessageGuidAttribute};
pub use guid::Guid;
pub use i_message_type_resolver::IMessageTypeResolver;
pub use transport_messages::HtmlTransportStartedMessage;
