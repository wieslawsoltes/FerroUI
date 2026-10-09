//! The errors of the library: the exceptions the upstream code throws or
//! lets through, as one value that can be handed to the exception event of a
//! connection and to every handler subscribed to it.

use std::fmt;
use std::io;
use std::sync::Arc;

use crate::metsys_bson::BsonException;

/// What a serializer, a resolver or a transport reports. Each variant is the
/// exception of the same name in the upstream code.
#[derive(Clone, Debug)]
pub enum Error {
    /// `BsonException`: what the BSON serializer itself rejects.
    Bson(BsonException),
    /// `EndOfStreamException`: the document or the stream ended inside a
    /// value.
    EndOfStream,
    /// `InvalidOperationException`.
    InvalidOperation(String),
    /// `InvalidCastException`: a deserialized value is not of the type of
    /// the property it belongs to.
    InvalidCast(String),
    /// `KeyNotFoundException`: an identifier or a type the resolver does not
    /// know, or an element type the reader has no type for.
    KeyNotFound(String),
    /// `ArgumentException` and `ArgumentOutOfRangeException`.
    Argument(String),
    /// `OverflowException`.
    Overflow(String),
    /// `FormatException`.
    Format(String),
    /// `NullReferenceException`.
    NullReference,
    /// `IOException` and `SocketException`: what a stream or a socket
    /// reported.
    Io(Arc<io::Error>),
    /// A handler of an event panicked: the port's form of an exception
    /// thrown by a handler, with the message of the panic.
    Handler(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Bson(e) => e.fmt(f),
            Error::EndOfStream => f.write_str("Unable to read beyond the end of the stream."),
            Error::InvalidOperation(message)
            | Error::InvalidCast(message)
            | Error::KeyNotFound(message)
            | Error::Argument(message)
            | Error::Overflow(message)
            | Error::Format(message)
            | Error::Handler(message) => f.write_str(message),
            Error::NullReference => f.write_str("Object reference not set to an instance of an object."),
            Error::Io(e) => e.fmt(f),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Bson(e) => Some(e),
            Error::Io(e) => Some(&**e),
            _ => None,
        }
    }
}

impl From<BsonException> for Error {
    fn from(e: BsonException) -> Self {
        Error::Bson(e)
    }
}

impl From<io::Error> for Error {
    fn from(e: io::Error) -> Self {
        if e.kind() == io::ErrorKind::UnexpectedEof {
            Error::EndOfStream
        } else {
            Error::Io(Arc::new(e))
        }
    }
}

/// Runs a handler and turns a panic of it into [`Error::Handler`]: the `try`
/// and `catch` around a delegate invocation in the upstream code.
pub(crate) fn catch_handler(f: impl FnOnce()) -> Result<(), Error> {
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(f)).map_err(|payload| {
        let message = if let Some(text) = payload.downcast_ref::<&str>() {
            (*text).to_string()
        } else if let Some(text) = payload.downcast_ref::<String>() {
            text.clone()
        } else {
            "A handler panicked.".to_string()
        };
        Error::Handler(message)
    })
}
