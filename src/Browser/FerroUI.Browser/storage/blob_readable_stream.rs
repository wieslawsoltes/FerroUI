use crate::interop::promise_helper::JsError;
use crate::interop::{stream_helper, JsObject};
use std::io::{self, Read, Seek, SeekFrom};

/// A read stream over a `Blob` of the page.
///
/// The page reads a blob only asynchronously: [`read_async`](Self::read_async)
/// reads a slice of it as upstream's `ReadAsync` does. The synchronous
/// [`Read`] the storage contracts hand out is served from the content that
/// [`load_async`](Self::load_async) brought into memory; before that it
/// fails, as upstream's `Read` does.
pub struct BlobReadableStream {
    js_reference: Option<JsObject>,
    position: u64,
    length: u64,
    content: Option<Vec<u8>>,
}

fn disposed() -> io::Error {
    io::Error::other("Cannot access a disposed object. Object name: 'BlobReadableStream'.")
}

impl BlobReadableStream {
    /// The stream over the blob `js_stream_reference`.
    pub fn new(js_stream_reference: JsObject) -> Self {
        let length = stream_helper::byte_length(&js_stream_reference);
        Self::with_length(js_stream_reference, length)
    }

    fn with_length(js_stream_reference: JsObject, length: u64) -> Self {
        Self { js_reference: Some(js_stream_reference), position: 0, length, content: None }
    }

    fn js_reference(&self) -> io::Result<&JsObject> {
        self.js_reference.as_ref().ok_or_else(disposed)
    }

    pub fn can_read(&self) -> bool {
        true
    }

    pub fn can_seek(&self) -> bool {
        false
    }

    pub fn can_write(&self) -> bool {
        false
    }

    /// The size of the blob in bytes.
    pub fn length(&self) -> u64 {
        self.length
    }

    pub fn position(&self) -> u64 {
        self.position
    }

    /// Reads up to `buffer.len()` bytes at the position into `buffer` and
    /// returns how many were read; `0` at the end of the blob.
    pub async fn read_async(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        let num_bytes_to_read = (buffer.len() as u64).min(self.length.saturating_sub(self.position)) as usize;
        let js_reference = self.js_reference()?.clone();
        let bytes_read = stream_helper::slice_async(&js_reference, self.position, num_bytes_to_read)
            .await
            .map_err(io::Error::from)?;
        self.complete_read(&bytes_read, num_bytes_to_read, buffer)
    }

    fn complete_read(&mut self, bytes_read: &[u8], num_bytes_to_read: usize, buffer: &mut [u8]) -> io::Result<usize> {
        if bytes_read.len() != num_bytes_to_read {
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "Failed to read the requested number of bytes from the stream.",
            ));
        }

        self.position += bytes_read.len() as u64;
        buffer[..bytes_read.len()].copy_from_slice(bytes_read);

        Ok(bytes_read.len())
    }

    /// Reads the whole blob into memory, after which the stream can be read
    /// synchronously through [`Read`].
    pub async fn load_async(&mut self) -> Result<(), JsError> {
        if self.content.is_some() {
            return Ok(());
        }
        let Some(js_reference) = self.js_reference.clone() else {
            return Ok(());
        };
        let content = stream_helper::slice_async(&js_reference, 0, self.length as usize).await?;
        self.set_content(content);
        Ok(())
    }

    fn set_content(&mut self, content: Vec<u8>) {
        // The blob is immutable; a shorter result can only mean the read failed half way.
        self.length = content.len() as u64;
        self.content = Some(content);
    }

    /// Releases the blob.
    pub fn dispose(&mut self) {
        self.js_reference = None;
        self.content = None;
    }
}

impl Read for BlobReadableStream {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        self.js_reference()?;
        let Some(content) = &self.content else {
            return Err(io::Error::other("Browser supports only ReadAsync"));
        };
        let start = self.position.min(content.len() as u64) as usize;
        let count = buffer.len().min(content.len() - start);
        buffer[..count].copy_from_slice(&content[start..start + count]);
        self.position += count as u64;
        Ok(count)
    }
}

impl Seek for BlobReadableStream {
    fn seek(&mut self, position: SeekFrom) -> io::Result<u64> {
        let position = match position {
            SeekFrom::Current(offset) => self.position as i64 + offset,
            SeekFrom::End(offset) => self.length as i64 + offset,
            SeekFrom::Start(offset) => offset as i64,
        };
        if position < 0 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "An attempt was made to move the position before the beginning of the stream.",
            ));
        }
        self.position = position as u64;
        Ok(self.position)
    }
}

#[cfg(test)]
mod tests {
    // Not from upstream (upstream has no tests of the browser streams): the parts that do not
    // need a page.
    use super::*;

    fn stream(content: Option<&[u8]>, length: u64) -> BlobReadableStream {
        let mut stream = BlobReadableStream::with_length(JsObject::NULL, length);
        if let Some(content) = content {
            stream.set_content(content.to_vec());
        }
        stream
    }

    #[test]
    fn read_before_load_fails_as_upstream_read_does() {
        let mut stream = stream(None, 4);
        let error = stream.read(&mut [0; 4]).unwrap_err();
        assert_eq!(error.to_string(), "Browser supports only ReadAsync");
    }

    #[test]
    fn read_after_load_returns_the_content_and_advances() {
        let mut stream = stream(Some(b"hello"), 5);
        let mut buffer = [0u8; 3];
        assert_eq!(stream.read(&mut buffer).unwrap(), 3);
        assert_eq!(&buffer, b"hel");
        assert_eq!(stream.position(), 3);
        let mut rest = Vec::new();
        stream.read_to_end(&mut rest).unwrap();
        assert_eq!(rest, b"lo");
        assert_eq!(stream.read(&mut buffer).unwrap(), 0);
    }

    #[test]
    fn seek_moves_relative_to_each_origin() {
        let mut stream = stream(Some(b"0123456789"), 10);
        assert_eq!(stream.seek(SeekFrom::Start(4)).unwrap(), 4);
        assert_eq!(stream.seek(SeekFrom::Current(2)).unwrap(), 6);
        assert_eq!(stream.seek(SeekFrom::End(-1)).unwrap(), 9);
        let mut buffer = [0u8; 4];
        assert_eq!(stream.read(&mut buffer).unwrap(), 1);
        assert_eq!(buffer[0], b'9');
        assert!(stream.seek(SeekFrom::Current(-20)).is_err());
    }

    #[test]
    fn short_slice_is_an_end_of_stream_error() {
        let mut stream = stream(None, 10);
        let mut buffer = [0u8; 4];
        let error = stream.complete_read(b"ab", 4, &mut buffer).unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::UnexpectedEof);
        assert_eq!(stream.complete_read(b"abcd", 4, &mut buffer).unwrap(), 4);
        assert_eq!(stream.position(), 4);
    }

    #[test]
    fn disposed_stream_fails_to_read() {
        let mut stream = stream(Some(b"abc"), 3);
        stream.dispose();
        assert!(stream.read(&mut [0; 1]).is_err());
    }
}
