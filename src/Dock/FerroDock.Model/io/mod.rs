//! Shared source stream handles used by the serializer contracts.
mod memory_stream;
mod stream;
pub use memory_stream::MemoryStream;
pub use stream::{Stream, StreamError, StreamIo};
#[cfg(test)]
mod tests;
