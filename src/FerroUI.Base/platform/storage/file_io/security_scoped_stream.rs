use crate::reactive::IDisposable;
use std::fs::File;
use std::io::{self, IoSlice, IoSliceMut, Read, Seek, SeekFrom, Write};
use std::rc::Rc;

/// Stream wrapper currently used by Apple platforms, where in sandboxed
/// scenario it's advised to start accessing the security scoped resource
/// before the file is used and to stop when it is closed: the security
/// scope is disposed when the stream is dropped, after the file has been
/// closed.
///
/// This is an implementation detail of the platform backends.
pub struct SecurityScopedStream {
    stream: Option<File>,
    security_scope: Rc<dyn IDisposable>,
}

impl SecurityScopedStream {
    /// Wraps a file opened inside a security scope.
    pub fn new(stream: File, security_scope: Rc<dyn IDisposable>) -> Self {
        Self { stream: Some(stream), security_scope }
    }

    fn stream(&self) -> &File {
        self.stream.as_ref().expect("the file is open until the stream is dropped")
    }

    /// The length of the file in bytes.
    pub fn length(&self) -> io::Result<u64> {
        Ok(self.stream().metadata()?.len())
    }

    /// Truncates or extends the file to `value` bytes.
    pub fn set_length(&self, value: u64) -> io::Result<()> {
        self.stream().set_len(value)
    }
}

impl Read for SecurityScopedStream {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        self.stream().read(buffer)
    }

    fn read_vectored(&mut self, buffers: &mut [IoSliceMut<'_>]) -> io::Result<usize> {
        self.stream().read_vectored(buffers)
    }
}

impl Write for SecurityScopedStream {
    fn write(&mut self, buffer: &[u8]) -> io::Result<usize> {
        self.stream().write(buffer)
    }

    fn write_vectored(&mut self, buffers: &[IoSlice<'_>]) -> io::Result<usize> {
        self.stream().write_vectored(buffers)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.stream().flush()
    }
}

impl Seek for SecurityScopedStream {
    fn seek(&mut self, position: SeekFrom) -> io::Result<u64> {
        self.stream().seek(position)
    }
}

impl Drop for SecurityScopedStream {
    fn drop(&mut self) {
        // The file is closed before the security scope ends.
        drop(self.stream.take());
        self.security_scope.dispose();
    }
}
