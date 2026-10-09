//! The remote protocol: the wire protocol of the previewer and of remote
//! rendering.
//!
//! This crate holds the serialization of a message as a BSON document
//! ([`metsys_bson`]). The library is a leaf, as the upstream project is: it
//! uses nothing of the framework, so that a tool that only talks the
//! protocol can use it alone.

pub mod metsys_bson;

mod error;
mod guid;

#[cfg(test)]
mod metsys_bson_tests;

pub use error::Error;
pub use guid::Guid;
