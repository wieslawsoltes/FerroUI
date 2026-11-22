//! One end of a pipe as a stream (the port of `Pipe2Stream.cs`).
//!
//! The reference wraps the descriptor in a `PipeStream` and reads or writes
//! it asynchronously on a pool thread. Here the stream is a file over the
//! owned descriptor, read and written with the blocking calls of the
//! system on the thread that has it: a thread of its own for a transfer
//! that must not hold up the UI thread, the UI thread itself where the
//! contract is synchronous.

use std::fs::File;
use std::io::{self, Read, Write};
use std::os::fd::{AsFd, OwnedFd};
use std::time::{Duration, Instant};

use super::unsafe_native_methods::wait_readable;

/// The direction of a pipe end.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PipeDirection {
    /// The reading end.
    In,
    /// The writing end.
    Out,
}

/// A stream backed by a file descriptor from `pipe2()`. The descriptor is closed when the
/// stream is dropped.
pub struct Pipe2Stream {
    file: File,
    direction: PipeDirection,
}

impl Pipe2Stream {
    pub fn new(fd: OwnedFd, direction: PipeDirection) -> Self {
        Self { file: File::from(fd), direction }
    }

    pub fn direction(&self) -> PipeDirection {
        self.direction
    }

    /// Reads until the other end is closed (`CopyToAsync` into a memory stream).
    pub fn read_to_end(mut self) -> io::Result<Vec<u8>> {
        let mut bytes = Vec::new();
        Read::read_to_end(&mut self, &mut bytes)?;
        Ok(bytes)
    }

    /// Reads optional clipboard data without waiting indefinitely for its owner.
    pub(crate) fn read_to_end_bounded(mut self, max_bytes: usize, timeout: Duration) -> io::Result<Vec<u8>> {
        let deadline = Instant::now() + timeout;
        let mut bytes = Vec::new();
        let mut buffer = [0; 4096];
        loop {
            wait_readable(self.file.as_fd(), deadline)?;
            let count = match self.read(&mut buffer) {
                Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
                result => result?,
            };
            if count == 0 {
                return Ok(bytes);
            }
            if count > max_bytes - bytes.len() {
                return Err(io::ErrorKind::InvalidData.into());
            }
            bytes.extend_from_slice(&buffer[..count]);
        }
    }
}

impl Read for Pipe2Stream {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        if self.direction != PipeDirection::In {
            return Err(io::Error::new(io::ErrorKind::Unsupported, "The stream does not support reading."));
        }
        self.file.read(buffer)
    }
}

impl Write for Pipe2Stream {
    fn write(&mut self, buffer: &[u8]) -> io::Result<usize> {
        if self.direction != PipeDirection::Out {
            return Err(io::Error::new(io::ErrorKind::Unsupported, "The stream does not support writing."));
        }
        self.file.write(buffer)
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    // Not from the reference, which has no tests for this file.
    use super::*;
    use crate::server::interop::unsafe_native_methods::pipe2_cloexec;

    #[test]
    fn what_is_written_to_one_end_is_read_from_the_other_until_it_closes() {
        let (read, write) = pipe2_cloexec().unwrap();
        let writer = std::thread::spawn(move || {
            let mut stream = Pipe2Stream::new(write, PipeDirection::Out);
            // More than a pipe holds, so that the writer waits for the reader.
            let chunk = vec![7u8; 4096];
            for _ in 0..64 {
                stream.write_all(&chunk).unwrap();
            }
        });
        let bytes = Pipe2Stream::new(read, PipeDirection::In).read_to_end().unwrap();
        writer.join().unwrap();
        assert_eq!(bytes.len(), 64 * 4096);
        assert!(bytes.iter().all(|byte| *byte == 7));
    }

    #[test]
    fn an_end_is_used_in_its_direction_only() {
        let (read, write) = pipe2_cloexec().unwrap();
        let mut reading = Pipe2Stream::new(read, PipeDirection::In);
        let mut writing = Pipe2Stream::new(write, PipeDirection::Out);
        assert_eq!(reading.direction(), PipeDirection::In);
        assert!(reading.write(&[1]).is_err());
        assert!(writing.read(&mut [0]).is_err());
    }

    #[test]
    fn a_write_to_a_pipe_nobody_reads_fails() {
        let (read, write) = pipe2_cloexec().unwrap();
        drop(read);
        let mut writing = Pipe2Stream::new(write, PipeDirection::Out);
        assert!(writing.write_all(&[1, 2, 3]).is_err());
    }

    #[test]
    fn bounded_reads_accept_eof_and_enforce_the_byte_limit() {
        for (data, limit) in [(b"".as_slice(), 0), (b"abcd", 4), (b"abcd", 3)] {
            let (read, write) = pipe2_cloexec().unwrap();
            Pipe2Stream::new(write, PipeDirection::Out).write_all(data).unwrap();
            let result = Pipe2Stream::new(read, PipeDirection::In).read_to_end_bounded(limit, Duration::from_secs(1));
            if data.len() <= limit {
                assert_eq!(result.unwrap(), data);
            } else {
                assert_eq!(result.unwrap_err().kind(), io::ErrorKind::InvalidData);
            }
        }
    }

    #[test]
    fn bounded_reads_terminate_after_a_partial_write_without_eof() {
        let (read, write) = pipe2_cloexec().unwrap();
        let mut writer = Pipe2Stream::new(write, PipeDirection::Out);
        writer.write_all(b"partial").unwrap();
        let result = Pipe2Stream::new(read, PipeDirection::In).read_to_end_bounded(64, Duration::from_millis(30));
        assert_eq!(result.unwrap_err().kind(), io::ErrorKind::TimedOut);
        drop(writer);
    }
}
