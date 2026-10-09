//! The remote protocol: the wire protocol of the previewer and of remote
//! rendering.
//!
//! - The messages ([`viewport`], [`input`], [`designer`] and
//!   [`HtmlTransportStartedMessage`]). Each message class has an identifier
//!   ([`FerroRemoteMessageGuidAttribute`]) by which the two ends name it; a
//!   resolver ([`IMessageTypeResolver`], [`DefaultMessageTypeResolver`])
//!   maps identifiers to classes.
//! - The serialization of a message as a BSON document ([`metsys_bson`]).
//! - The transports: the contract of a connection
//!   ([`IFerroRemoteTransportConnection`], whose module states the threading
//!   contract of the port), the connection over a pair of byte streams, the
//!   transport over TCP ([`BsonTcpTransport`], [`TcpTransportBase`]) and the
//!   wrapper that stashes events and queues sends
//!   ([`TransportConnectionWrapper`]).
//!
//! The library is a leaf, as the upstream project is: it uses nothing of the
//! framework, so that a tool that only talks the protocol can use it alone.

pub mod metsys_bson;

mod assembly;
mod bson_stream_transport;
mod bson_tcp_transport;
mod default_message_type_resolver;
mod design_messages;
mod error;
mod event_stash;
mod ferro_remote_message_guid_attribute;
mod guid;
mod i_message_type_resolver;
mod i_transport;
mod input_messages;
mod task;
mod tcp_transport_base;
mod transport_connection_wrapper;
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
pub use bson_tcp_transport::BsonTcpTransport;
pub use default_message_type_resolver::DefaultMessageTypeResolver;
pub use error::Error;
pub use ferro_remote_message_guid_attribute::{FerroRemoteMessage, FerroRemoteMessageGuidAttribute};
pub use guid::Guid;
pub use i_message_type_resolver::IMessageTypeResolver;
pub use i_transport::{
    exception_handler, message_handler, ExceptionHandler, Handler, HandlerToken, IFerroRemoteTransportConnection,
    Message, MessageHandler,
};
pub use task::{Task, TaskCompletionSource};
pub use tcp_transport_base::{DisposableServer, DisposeCallback, TcpTransportBase};
pub use transport_connection_wrapper::TransportConnectionWrapper;
pub use transport_messages::HtmlTransportStartedMessage;
