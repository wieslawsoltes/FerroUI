use super::DocumentTextWriter;
use std::io;

/// Lossless object-safe UTF-16 sink. Unlike `std::io::Write`, its input consists
/// of code units, not encoded bytes, so a chunk may end inside a surrogate pair.
/// Successful calls consume the entire slice. On failure, preceding successful
/// calls remain written; transactional sinks should provide their own boundary.
pub trait Utf16TextWriter {
    fn write_utf16(&mut self, text: &[u16]) -> io::Result<()>;
}

impl Utf16TextWriter for Vec<u16> {
    fn write_utf16(&mut self, text: &[u16]) -> io::Result<()> {
        self.extend_from_slice(text);
        Ok(())
    }
}

impl Utf16TextWriter for DocumentTextWriter {
    fn write_utf16(&mut self, text: &[u16]) -> io::Result<()> {
        DocumentTextWriter::write_utf16(self, text).map_err(io::Error::other)
    }
}
