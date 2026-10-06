use super::{IRenderDataPayload, RenderDataOpcode};
use crate::rendering::composition::transport::{BatchValue, BatchValueReader};

/// Reads back what a [`RenderDataWriter`](super::RenderDataWriter) wrote.
/// Reading past the end panics, as the span slicing of upstream throws.
#[derive(Clone, Copy, Debug)]
pub struct RenderDataReader<'a> {
    buffer: &'a [u8],
    position: usize,
}

impl<'a> RenderDataReader<'a> {
    pub fn new(buffer: &'a [u8]) -> Self {
        Self { buffer, position: 0 }
    }

    /// The offset of the next byte to read.
    pub fn position(&self) -> usize {
        self.position
    }

    /// Whether every byte has been read.
    pub fn is_at_end(&self) -> bool {
        self.position >= self.buffer.len()
    }

    /// Takes the next `count` bytes.
    pub fn take(&mut self, count: usize) -> &'a [u8] {
        let span = &self.buffer[self.position..self.position + count];
        self.position += count;
        span
    }

    /// Reads the next value.
    pub fn read<T: BatchValue>(&mut self) -> T {
        T::read_from(&mut BatchValueReader::new(self.buffer, &mut self.position))
    }

    /// Reads the next value without advancing.
    pub fn peek<T: BatchValue>(&self) -> T {
        let mut position = self.position;
        T::read_from(&mut BatchValueReader::new(self.buffer, &mut position))
    }

    /// Reads the opcode of a payload, then the payload.
    pub fn read_payload<T: IRenderDataPayload>(&mut self) -> T {
        let opcode = self.read::<RenderDataOpcode>();
        debug_assert_eq!(opcode, T::OPCODE);
        self.read::<T>()
    }
}
