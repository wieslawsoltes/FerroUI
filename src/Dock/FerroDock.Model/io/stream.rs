// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use ferroui_system::IDisposable;
use std::{
    fmt,
    io::{self, Read, Seek, SeekFrom, Write},
    rc::Rc,
};

/// Source stream failures represented through native I/O errors.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StreamError {
    /// The source stream has been closed or disposed.
    Closed,
    /// The requested operation is not supported by this stream.
    NotSupported,
    /// A source position or length argument exceeds its supported range.
    ArgumentOutOfRange,
    /// Seeking would move before the stream's beginning or overflow its position.
    InvalidSeek,
    /// A fixed backing array cannot grow to the requested length.
    FixedCapacity,
}
impl fmt::Display for StreamError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Closed => "Cannot access a closed Stream.",
            Self::NotSupported => "Stream does not support this operation.",
            Self::ArgumentOutOfRange => "Stream argument is out of range.",
            Self::InvalidSeek => "Attempted to seek outside the supported stream position range.",
            Self::FixedCapacity => "Memory stream is not expandable.",
        })
    }
}
impl std::error::Error for StreamError {}
impl From<StreamError> for io::Error {
    fn from(value: StreamError) -> Self {
        let kind = match value {
            StreamError::Closed => io::ErrorKind::BrokenPipe,
            StreamError::NotSupported | StreamError::FixedCapacity => io::ErrorKind::Unsupported,
            StreamError::ArgumentOutOfRange => io::ErrorKind::InvalidInput,
            StreamError::InvalidSeek => io::ErrorKind::Other,
        };
        Self::new(kind, value)
    }
}

/// Shared System.IO.Stream contract at the model boundary.
/// Implementations retain source identity and choose their own close/disposal behavior.
/// All methods take the shared instance, so callbacks can close the same stream handle.
pub trait Stream: IDisposable {
    /// Whether reading is currently supported.
    fn can_read(&self) -> bool;
    /// Whether writing is currently supported.
    fn can_write(&self) -> bool;
    /// Whether seeking is currently supported.
    fn can_seek(&self) -> bool;
    /// Gets the source Length, failing when unsupported or closed.
    fn len(&self) -> io::Result<u64>;
    /// Native convenience query over the source Length.
    fn is_empty(&self) -> io::Result<bool> {
        Ok(self.len()? == 0)
    }
    /// Gets the current source Position.
    fn position(&self) -> io::Result<u64>;
    /// Sets the source Position without changing Length.
    fn set_position(&self, value: i64) -> io::Result<()>;
    /// Sets the source Length.
    fn set_len(&self, value: i64) -> io::Result<()>;
    /// Reads up to the supplied buffer length, returning zero at the end.
    fn read(&self, buffer: &mut [u8]) -> io::Result<usize>;
    /// Writes the entire supplied slice, matching the source void Write contract.
    fn write(&self, buffer: &[u8]) -> io::Result<()>;
    /// Seeks relative to the beginning, current position or end.
    fn seek(&self, from: SeekFrom) -> io::Result<u64>;
    /// Flushes the original stream according to its concrete implementation.
    fn flush(&self) -> io::Result<()>;
    /// Closes the original shared stream; existing aliases observe that same state.
    fn close(&self) -> io::Result<()>;
}

/// Native I/O adapter over the same source stream, without copying or closing it.
/// Dropping this adapter does not call Close or Dispose.
#[derive(Clone)]
pub struct StreamIo(Rc<dyn Stream>);
impl StreamIo {
    /// Retains the supplied original stream reference.
    pub fn new(stream: Rc<dyn Stream>) -> Self {
        Self(stream)
    }
    /// Gets the same source stream reference.
    pub fn stream(&self) -> Rc<dyn Stream> {
        self.0.clone()
    }
}
impl Read for StreamIo {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        self.0.read(buffer)
    }
}
impl Write for StreamIo {
    fn write(&mut self, buffer: &[u8]) -> io::Result<usize> {
        self.0.write(buffer)?;
        Ok(buffer.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        self.0.flush()
    }
}
impl Seek for StreamIo {
    fn seek(&mut self, from: SeekFrom) -> io::Result<u64> {
        self.0.seek(from)
    }
}
