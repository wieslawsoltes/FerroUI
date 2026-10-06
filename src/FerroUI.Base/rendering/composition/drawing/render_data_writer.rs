use super::{IRenderDataPayload, RenderDataOpcode};
use crate::rendering::composition::transport::BatchValue;

/// Appends the operations of a render data stream to a byte buffer.
///
/// Upstream blits unmanaged structs into a pooled array; the port encodes
/// values with [`BatchValue`] (fields in declaration order, little-endian),
/// the encoding of the batch transport, so that no `unsafe` is needed.
#[derive(Debug, Default)]
pub struct RenderDataWriter {
    buffer: Option<Vec<u8>>,
    length: usize,
}

impl RenderDataWriter {
    pub fn new() -> Self {
        Self::default()
    }

    /// The number of bytes written.
    pub fn length(&self) -> usize {
        self.length
    }

    /// The bytes written.
    pub fn written(&self) -> &[u8] {
        match &self.buffer {
            Some(buffer) => &buffer[..self.length],
            None => &[],
        }
    }

    /// The buffer, cut to `length`: bytes past a rewind are overwritten.
    /// It starts at 256 bytes and doubles when it grows.
    fn buffer(buffer: &mut Option<Vec<u8>>, length: usize) -> &mut Vec<u8> {
        let buffer = buffer.get_or_insert_with(|| Vec::with_capacity(256));
        buffer.truncate(length);
        buffer
    }

    /// Makes room for `size` more bytes and returns them.
    fn advance(&mut self, size: usize) -> &mut [u8] {
        let start = self.length;
        self.length = start + size;
        let buffer = Self::buffer(&mut self.buffer, start);
        buffer.resize(start + size, 0);
        &mut buffer[start..]
    }

    /// Appends `count` bytes for the caller to fill.
    pub fn reserve(&mut self, count: usize) -> &mut [u8] {
        self.advance(count)
    }

    /// Drops everything written after `length` bytes.
    pub fn rewind(&mut self, length: usize) {
        self.length = length;
    }

    /// Appends a value.
    pub fn write<T: BatchValue>(&mut self, value: T) {
        let buffer = Self::buffer(&mut self.buffer, self.length);
        value.write_to(buffer);
        self.length = buffer.len();
    }

    /// Appends an opcode.
    pub fn write_opcode(&mut self, opcode: RenderDataOpcode) {
        self.write(opcode);
    }

    /// Appends the opcode of a payload, then the payload.
    pub fn write_payload<T: IRenderDataPayload>(&mut self, payload: T) {
        self.write(T::OPCODE);
        self.write(payload);
    }

    /// Releases the buffer.
    pub fn dispose(&mut self) {
        self.buffer = None;
        self.length = 0;
    }
}
