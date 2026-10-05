/// A growable big-endian byte writer for hand-crafting OpenType table
/// sub-structures (cmap subtables, layout tables, ...) in tests.
///
/// OpenType is big-endian and pervasively offset-based: a header field holds
/// the byte offset of a sub-table that is written later.
/// [`reserve_offset16`](Self::reserve_offset16) /
/// [`reserve_offset32`](Self::reserve_offset32) write a placeholder and return
/// its position so the real value can be back-patched with
/// [`patch_uint16`](Self::patch_uint16) / [`patch_uint32`](Self::patch_uint32)
/// once the target's position is known (via [`position`](Self::position)).
/// All multi-byte writes are big-endian.
#[derive(Clone, Debug, Default)]
pub(crate) struct BigEndianBuffer {
    buffer: Vec<u8>,
}

#[allow(dead_code)] // the full writer API is kept for the font tests built on top
impl BigEndianBuffer {
    pub(crate) fn new() -> Self {
        Self { buffer: Vec::with_capacity(64) }
    }

    /// The number of bytes written so far — also the offset the next write lands at.
    pub(crate) fn position(&self) -> usize {
        self.buffer.len()
    }

    /// Panics when `value` does not fit in a byte.
    pub(crate) fn uint8(&mut self, value: i32) -> &mut Self {
        self.buffer.push(u8::try_from(value).expect("value out of range for a u8"));
        self
    }

    pub(crate) fn int8(&mut self, value: i32) -> &mut Self {
        self.buffer.push(value as i8 as u8);
        self
    }

    /// Panics when `value` does not fit in 16 unsigned bits.
    pub(crate) fn uint16(&mut self, value: i32) -> &mut Self {
        let value = u16::try_from(value).expect("value out of range for a u16");
        self.buffer.extend_from_slice(&value.to_be_bytes());
        self
    }

    /// Panics when `value` does not fit in 16 signed bits.
    pub(crate) fn int16(&mut self, value: i32) -> &mut Self {
        let value = i16::try_from(value).expect("value out of range for an i16");
        self.buffer.extend_from_slice(&value.to_be_bytes());
        self
    }

    pub(crate) fn uint24(&mut self, value: i32) -> &mut Self {
        self.buffer.push(((value >> 16) & 0xFF) as u8);
        self.buffer.push(((value >> 8) & 0xFF) as u8);
        self.buffer.push((value & 0xFF) as u8);
        self
    }

    pub(crate) fn uint32(&mut self, value: u32) -> &mut Self {
        self.buffer.extend_from_slice(&value.to_be_bytes());
        self
    }

    pub(crate) fn int32(&mut self, value: i32) -> &mut Self {
        self.buffer.extend_from_slice(&value.to_be_bytes());
        self
    }

    /// Writes an F2DOT14 fixed-point value (the variation-coordinate / region format).
    pub(crate) fn f2dot14(&mut self, value: f64) -> &mut Self {
        self.int16((value * 16384.0).round_ties_even() as i32)
    }

    /// Writes a 16.16 fixed-point value.
    pub(crate) fn fixed(&mut self, value: f64) -> &mut Self {
        self.int32((value * 65536.0).round_ties_even() as i32)
    }

    /// Writes a 4-character tag (space-padded / truncated to 4 bytes).
    pub(crate) fn tag(&mut self, tag: &str) -> &mut Self {
        let mut bytes = [0x20u8; 4];

        for (i, unit) in tag.encode_utf16().take(4).enumerate() {
            bytes[i] = unit as u8;
        }

        self.bytes(&bytes)
    }

    pub(crate) fn bytes(&mut self, bytes: &[u8]) -> &mut Self {
        self.buffer.extend_from_slice(bytes);
        self
    }

    /// Writes `count` zero bytes (padding / placeholder data).
    pub(crate) fn zeros(&mut self, count: usize) -> &mut Self {
        self.buffer.resize(self.buffer.len() + count, 0);
        self
    }

    /// Writes a placeholder big-endian `u16` and returns its position for later patching.
    pub(crate) fn reserve_offset16(&mut self) -> usize {
        let position = self.buffer.len();
        self.uint16(0);
        position
    }

    /// Writes a placeholder big-endian `u32` and returns its position for later patching.
    pub(crate) fn reserve_offset32(&mut self) -> usize {
        let position = self.buffer.len();
        self.uint32(0);
        position
    }

    /// Back-patches a big-endian `u16` at a previously reserved position.
    ///
    /// Panics when `value` does not fit or the position is out of range.
    pub(crate) fn patch_uint16(&mut self, position: usize, value: i32) -> &mut Self {
        let value = u16::try_from(value).expect("value out of range for a u16");
        self.buffer[position..position + 2].copy_from_slice(&value.to_be_bytes());
        self
    }

    /// Back-patches a big-endian `u32` at a previously reserved position.
    pub(crate) fn patch_uint32(&mut self, position: usize, value: u32) -> &mut Self {
        self.buffer[position..position + 4].copy_from_slice(&value.to_be_bytes());
        self
    }

    pub(crate) fn to_array(&self) -> Vec<u8> {
        self.buffer.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn big_endian_buffer_writes_big_endian_and_patches_reserved_offsets() {
        let mut buffer = BigEndianBuffer::new();

        buffer.uint16(0x1234);
        let offset_pos = buffer.reserve_offset32();
        buffer.uint8(0xAB);
        buffer.patch_uint32(offset_pos, 0xDEADBEEF);

        assert_eq!(buffer.to_array(), [0x12, 0x34, 0xDE, 0xAD, 0xBE, 0xEF, 0xAB]);
    }

    #[test]
    fn big_endian_buffer_f2dot14_and_tag_encode_correctly() {
        let mut buffer = BigEndianBuffer::new();

        buffer.f2dot14(1.0); // 1.0 == 0x4000 in F2DOT14
        buffer.tag("head");

        let bytes = buffer.to_array();

        assert_eq!(bytes[..2], [0x40, 0x00]);
        assert_eq!(bytes[2..6], [b'h', b'e', b'a', b'd']);
    }

    #[test]
    fn remaining_writers_encode_big_endian() {
        let mut buffer = BigEndianBuffer::new();

        buffer.int8(-1).int16(-2).uint24(0x010203).int32(-3).fixed(1.5).zeros(2).tag("CFF");
        let reserved = buffer.reserve_offset16();
        let position = buffer.position();
        buffer.patch_uint16(reserved, position as i32);

        assert_eq!(
            buffer.to_array(),
            [
                0xFF, 0xFF, 0xFE, 1, 2, 3, 0xFF, 0xFF, 0xFF, 0xFD, 0x00, 0x01, 0x80, 0x00, 0, 0, b'C', b'F', b'F',
                b' ', 0, 22
            ]
        );
    }
}
