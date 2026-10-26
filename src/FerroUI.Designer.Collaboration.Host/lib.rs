//! Hosted collaboration built on the designer's deterministic operation set. Acknowledgements
//! follow durable commits; member credentials bind each writer to one actor. The optional MCP
//! service reuses FerroUI's existing transport and exposes only room tools.
mod model;
mod rooms;
mod storage;
pub use model::*;
pub use rooms::RoomStore;
pub use storage::{load_private, save_private};
#[cfg(any(feature = "client", feature = "server"))]
mod endpoint;
#[cfg(any(feature = "client", feature = "server"))]
pub use endpoint::{validate_endpoint_url, validate_public_endpoint_url};
#[cfg(feature = "client")]
mod client;
#[cfg(feature = "client")]
pub use client::{McpPeer, PeerTransport, ReconnectingPeer};
#[cfg(feature = "server")]
mod service;
#[cfg(feature = "server")]
pub use service::{CollaborationService, HostedServer};

#[cfg(test)]
mod tests;
