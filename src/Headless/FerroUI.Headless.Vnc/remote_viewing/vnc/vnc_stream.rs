//! Port of `Vnc/VncStream.cs` of RemoteViewing (BSD-2-Clause, Copyright
//! (c) 2013 James F. Bellinger; `NOTICE.md` of this crate).
//!
//! The errors of a read are returned (the exceptions of the original);
//! the errors of a write are swallowed, as the original swallows them.

use super::{Version, VncError, VncFailureReason, VncRectangle, VncTransport, VncUtility};
use crate::remote_viewing::utility::{AutoClear, ReentrantLock};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

/// Provides methods for reading and sending VNC data over a stream.
pub(crate) struct VncStream {
    stream: Mutex<Option<Arc<dyn VncTransport>>>,
    sync_root: ReentrantLock,
}

#[allow(dead_code)] // the members of the original that the client of the library uses
impl VncStream {
    /// Initializes a new instance of the [`VncStream`] class.
    pub(crate) fn new() -> Self {
        Self {
            stream: Mutex::new(None),
            sync_root: ReentrantLock::new(),
        }
    }

    /// Initializes a new instance of the [`VncStream`] class over `stream`.
    pub(crate) fn with_stream(stream: Arc<dyn VncTransport>) -> Self {
        let this = Self::new();
        this.set_stream(Some(stream));
        this
    }

    fn slot(&self) -> MutexGuard<'_, Option<Arc<dyn VncTransport>>> {
        self.stream.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// Gets the underlying stream.
    pub(crate) fn stream(&self) -> Option<Arc<dyn VncTransport>> {
        self.slot().clone()
    }

    /// Sets the underlying stream.
    pub(crate) fn set_stream(&self, stream: Option<Arc<dyn VncTransport>>) {
        *self.slot() = stream;
    }

    /// Gets an object that can be used to synchronize access to the
    /// stream: a message is written whole while it is held.
    pub(crate) fn sync_root(&self) -> &ReentrantLock {
        &self.sync_root
    }

    /// Decodes `count` bytes of ISO 8859-1 from `buffer`, starting at
    /// `offset`, into a string.
    pub(crate) fn decode_string(buffer: &[u8], offset: usize, count: usize) -> String {
        buffer[offset..offset + count]
            .iter()
            .map(|byte| char::from(*byte))
            .collect()
    }

    /// Encodes characters as ISO 8859-1; a character outside it becomes
    /// `?` (the replacement of the encoding of .NET).
    pub(crate) fn encode_chars(chars: &[char]) -> Vec<u8> {
        chars
            .iter()
            .map(|c| u8::try_from(u32::from(*c)).unwrap_or(b'?'))
            .collect()
    }

    /// Encodes all the characters in the specified string into a sequence
    /// of bytes of ISO 8859-1.
    pub(crate) fn encode_string(string: &str) -> Vec<u8> {
        string
            .chars()
            .map(|c| u8::try_from(u32::from(c)).unwrap_or(b'?'))
            .collect()
    }

    /// Fails with a [`VncError`] if a specific condition is not met.
    pub(crate) fn require(
        condition: bool,
        message: &str,
        reason: VncFailureReason,
    ) -> Result<(), VncError> {
        if !condition {
            return Err(VncError::new(message, reason, None));
        }

        Ok(())
    }

    /// Fails with a [`VncError`] if a specific condition is not met.
    pub(crate) fn sanity_check(condition: bool) -> Result<(), VncError> {
        Self::require(
            condition,
            "Sanity check failed.",
            VncFailureReason::SanityCheckFailed,
        )
    }

    /// Closes the current stream and releases any resources associated
    /// with the current stream.
    pub(crate) fn close(&self) {
        if let Some(stream) = self.stream() {
            stream.close();
        }
    }

    /// Reads `count` bytes from the stream.
    pub(crate) fn receive(&self, count: usize) -> Result<Vec<u8>, VncError> {
        let mut buffer = vec![0u8; count];
        self.receive_into(&mut buffer, 0, count)?;
        Ok(buffer)
    }

    /// Reads `count` bytes from the stream into `buffer`, starting at
    /// `offset`.
    pub(crate) fn receive_into(
        &self,
        buffer: &mut [u8],
        offset: usize,
        count: usize,
    ) -> Result<(), VncError> {
        let Some(stream) = self.stream() else {
            return Err(VncError::new(
                "Lost connection.",
                VncFailureReason::NetworkError,
                None,
            ));
        };

        let mut i = 0;
        while i < count {
            let bytes = stream
                .read(&mut buffer[offset + i..offset + count])
                .map_err(|error| {
                    VncError::new(
                        error.to_string(),
                        VncFailureReason::NetworkError,
                        Some(Box::new(error)),
                    )
                })?;
            Self::require(
                bytes > 0,
                "Lost connection.",
                VncFailureReason::NetworkError,
            )?;
            i += bytes;
        }

        Ok(())
    }

    /// Reads a byte from the stream.
    pub(crate) fn receive_byte(&self) -> Result<u8, VncError> {
        let mut value = [0u8; 1];
        self.receive_into(&mut value, 0, 1)?;
        Ok(value[0])
    }

    /// Reads a [`VncRectangle`] (8 bytes) from the stream.
    pub(crate) fn receive_rectangle(&self) -> Result<VncRectangle, VncError> {
        let x = i32::from(self.receive_uint16_be()?);
        let y = i32::from(self.receive_uint16_be()?);
        let w = i32::from(self.receive_uint16_be()?);
        let h = i32::from(self.receive_uint16_be()?);
        Ok(VncRectangle::new(x, y, w, h))
    }

    /// Reads a string of at most `max_length` bytes (0xfff in the original
    /// when it is not given) from the stream.
    pub(crate) fn receive_string(&self, max_length: usize) -> Result<String, VncError> {
        let length = self.receive_uint32_be()? as usize;
        Self::sanity_check(length <= max_length)?;
        let value = Self::decode_string(&self.receive(length)?, 0, length);
        Ok(value)
    }

    /// Reads a `u16` in big-endian encoding from the stream.
    pub(crate) fn receive_uint16_be(&self) -> Result<u16, VncError> {
        Ok(VncUtility::decode_uint16_be(&self.receive(2)?, 0))
    }

    /// Reads a `u32` in big-endian encoding from the stream.
    pub(crate) fn receive_uint32_be(&self) -> Result<u32, VncError> {
        Ok(VncUtility::decode_uint32_be(&self.receive(4)?, 0))
    }

    /// Receives version information (`RFB xxx.yyy\n`) from the stream.
    pub(crate) fn receive_version(&self) -> Result<Version, VncError> {
        let version = self.receive(12)?;
        let digits = |range: std::ops::Range<usize>| -> Option<i32> {
            let text = std::str::from_utf8(&version[range]).ok()?;
            if text.bytes().all(|b| b.is_ascii_digit()) {
                text.parse().ok()
            } else {
                None
            }
        };
        let parsed = if &version[0..4] == b"RFB " && version[7] == b'.' && version[11] == b'\n' {
            digits(4..7).zip(digits(8..11))
        } else {
            None
        };
        Self::require(
            parsed.is_some(),
            "Not using VNC protocol.",
            VncFailureReason::WrongKindOfServer,
        )?;

        let (major, minor) = parsed.unwrap_or_default();
        Ok(Version::new(major, minor))
    }

    /// Writes `buffer` to the stream.
    pub(crate) fn send(&self, buffer: &[u8]) {
        self.send_range(buffer, 0, buffer.len());
    }

    /// Writes `count` bytes of `buffer`, starting at `offset`, to the
    /// stream. Errors of the stream are ignored.
    pub(crate) fn send_range(&self, buffer: &[u8], offset: usize, count: usize) {
        if self.stream().is_none() {
            return;
        }

        let _sync = self.sync_root.lock();
        let Some(stream) = self.stream() else {
            return;
        };

        let _ = stream.write_all(&buffer[offset..offset + count]);
    }

    /// Writes a byte to the stream.
    pub(crate) fn send_byte(&self, value: u8) {
        self.send(&[value]);
    }

    /// Writes a [`VncRectangle`] (8 bytes) to the stream.
    pub(crate) fn send_rectangle(&self, region: VncRectangle) {
        let mut buffer = [0u8; 8];
        VncUtility::encode_uint16_be_into(&mut buffer, 0, region.x as u16);
        VncUtility::encode_uint16_be_into(&mut buffer, 2, region.y as u16);
        VncUtility::encode_uint16_be_into(&mut buffer, 4, region.width as u16);
        VncUtility::encode_uint16_be_into(&mut buffer, 6, region.height as u16);
        self.send(&buffer);
    }

    /// Writes a `u16` in big endian encoding to the stream.
    pub(crate) fn send_uint16_be(&self, value: u16) {
        self.send(&VncUtility::encode_uint16_be(value));
    }

    /// Writes a `u32` in big endian encoding to the stream.
    pub(crate) fn send_uint32_be(&self, value: u32) {
        self.send(&VncUtility::encode_uint32_be(value));
    }

    /// Writes a string to the stream, in ISO 8859-1, preceded by its length
    /// when `include_length` is true.
    pub(crate) fn send_string(&self, string: &str, include_length: bool) {
        let mut encoded_string = Self::encode_string(string);
        let length = encoded_string.len();
        let _sync = self.sync_root.lock();
        if include_length {
            self.send_uint32_be(length as u32);
        }

        let encoded_string = AutoClear::new(&mut encoded_string);
        self.send(encoded_string.array());
    }

    /// Writes a version (`RFB xxx.yyy\n`) to the stream.
    pub(crate) fn send_version(&self, version: Version) {
        self.send_string(
            &format!("RFB {:03}.{:03}\n", version.major, version.minor),
            false,
        );
    }
}

#[cfg(all(test, unix))]
mod tests {
    // Not from upstream.
    use super::*;
    use std::io::Read;
    use std::os::unix::net::UnixStream;

    #[test]
    fn values_are_written_and_read_in_the_order_of_the_protocol() {
        let (server, mut client) = UnixStream::pair().unwrap();
        let stream = VncStream::with_stream(Arc::new(server));

        stream.send_version(Version::new(3, 8));
        stream.send_string("abc", true);
        stream.send_rectangle(VncRectangle::new(1, 2, 3, 4));
        let mut written = [0u8; 12 + 7 + 8];
        client.read_exact(&mut written).unwrap();
        assert_eq!(b"RFB 003.008\n", &written[..12]);
        assert_eq!(&[0, 0, 0, 3, b'a', b'b', b'c'], &written[12..19]);
        assert_eq!(&[0, 1, 0, 2, 0, 3, 0, 4], &written[19..]);

        client.write_all(b"RFB 003.008\n").unwrap();
        client.write_all(&[0, 0, 0, 2, b'h', 0xe9]).unwrap();
        assert_eq!(Version::new(3, 8), stream.receive_version().unwrap());
        assert_eq!("h\u{e9}", stream.receive_string(0xfff).unwrap());

        client.write_all(b"HTTP/1.1 200").unwrap();
        assert_eq!(
            VncFailureReason::WrongKindOfServer,
            stream.receive_version().unwrap_err().reason()
        );

        drop(client);
        assert_eq!(
            VncFailureReason::NetworkError,
            stream.receive_byte().unwrap_err().reason()
        );
    }

    #[test]
    fn characters_outside_latin_1_become_question_marks() {
        assert_eq!(b"a?b".to_vec(), VncStream::encode_string("a\u{263a}b"));
        assert_eq!(b"\xe9".to_vec(), VncStream::encode_chars(&['\u{e9}']));
    }
}
