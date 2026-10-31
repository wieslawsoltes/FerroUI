//! Replicated designer transactions. The operation set is the durable state: materialization is
//! deterministic regardless of transport order, and local undo submits a new compensating write.
//! A host supplies authentication, authorization, durable storage and delivery; this crate does
//! not open a socket or execute arbitrary code.
mod document;
mod model;

pub use document::DocumentReplica;
pub use model::*;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CollaborationError(pub String);
impl std::fmt::Display for CollaborationError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0)
    }
}
impl std::error::Error for CollaborationError {}
pub type Result<T> = std::result::Result<T, CollaborationError>;

#[cfg(test)]
mod tests;
