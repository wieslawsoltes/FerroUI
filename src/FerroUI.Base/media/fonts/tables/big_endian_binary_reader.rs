// Derived from SixLabors.Fonts (Apache License 2.0), see NOTICE.md.

use std::fmt;

use super::encoding_id_extensions::Encoding;
use super::font_version::FontVersion;
use super::invalid_font_table_exception::InvalidFontTableException;
use super::missing_font_table_exception::MissingFontTableException;

/// The failures the table readers report instead of the exceptions the
/// reference implementation throws while parsing font data.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FontTableError {
    /// A read ran past the end of the data (reference: `InvalidOperationException`
    /// "End of span reached ...").
    EndOfSpan {
        /// The number of bytes that were missing.
        missing: i64,
    },
    /// An offset, count or slice was outside of the data (reference:
    /// `ArgumentOutOfRangeException`).
    ArgumentOutOfRange {
        /// The name of the offending parameter.
        param_name: &'static str,
        /// An optional description; `None` gives the generic message.
        message: Option<String>,
    },
    /// Any other invalid state (reference: `InvalidOperationException`).
    InvalidOperation(String),
    /// Invalid data in a table.
    InvalidFontTable(InvalidFontTableException),
    /// A required table is missing.
    MissingFontTable(MissingFontTableException),
}

impl FontTableError {
    pub(crate) fn out_of_range(param_name: &'static str) -> Self {
        FontTableError::ArgumentOutOfRange { param_name, message: None }
    }

    /// Whether the reference implementation reports this failure as an
    /// `InvalidOperationException` (end of span or invalid state).
    pub fn is_invalid_operation(&self) -> bool {
        matches!(self, FontTableError::EndOfSpan { .. } | FontTableError::InvalidOperation(_))
    }

    /// Whether the reference implementation reports this failure as an
    /// `ArgumentOutOfRangeException`.
    pub fn is_argument_out_of_range(&self) -> bool {
        matches!(self, FontTableError::ArgumentOutOfRange { .. })
    }
}

impl fmt::Display for FontTableError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FontTableError::EndOfSpan { missing } => write!(
                f,
                "End of span reached with {} byte{} left to read.",
                missing,
                if *missing == 1 { "" } else { "s" }
            ),
            FontTableError::ArgumentOutOfRange { param_name, message } => match message {
                Some(message) => write!(f, "{message} (Parameter '{param_name}')"),
                None => write!(
                    f,
                    "Specified argument was out of the range of valid values. (Parameter '{param_name}')"
                ),
            },
            FontTableError::InvalidOperation(message) => f.write_str(message),
            FontTableError::InvalidFontTable(error) => fmt::Display::fmt(error, f),
            FontTableError::MissingFontTable(error) => fmt::Display::fmt(error, f),
        }
    }
}

impl std::error::Error for FontTableError {}

impl From<InvalidFontTableException> for FontTableError {
    fn from(error: InvalidFontTableException) -> Self {
        FontTableError::InvalidFontTable(error)
    }
}

impl From<MissingFontTableException> for FontTableError {
    fn from(error: MissingFontTableException) -> Self {
        FontTableError::MissingFontTable(error)
    }
}

/// Binary reader using big-endian encoding over a byte slice.
///
/// No method panics on malformed input: reading past the end gives
/// [`FontTableError::EndOfSpan`].
#[derive(Clone, Debug)]
pub struct BigEndianBinaryReader<'a> {
    span: &'a [u8],
    position: usize,
    start_of_span: usize,
}

impl<'a> BigEndianBinaryReader<'a> {
    /// Initializes a new reader over `span`.
    #[inline]
    pub fn new(span: &'a [u8]) -> Self {
        Self { span, position: 0, start_of_span: 0 }
    }

    /// Gets the current position in the span.
    #[inline]
    pub fn position(&self) -> i32 {
        self.position as i32
    }

    /// Seeks within the span.
    #[inline]
    pub fn seek(&mut self, offset: i32) -> Result<(), FontTableError> {
        if offset < 0 {
            return Err(FontTableError::out_of_range("offset"));
        }

        let absolute_offset = self.start_of_span + offset as usize;

        if absolute_offset > self.span.len() {
            return Err(FontTableError::out_of_range("offset"));
        }

        self.position = absolute_offset;

        Ok(())
    }

    #[inline]
    pub fn read_byte(&mut self) -> Result<u8, FontTableError> {
        Ok(self.take::<1>()?[0])
    }

    #[inline]
    pub fn read_sbyte(&mut self) -> Result<i8, FontTableError> {
        Ok(self.take::<1>()?[0] as i8)
    }

    pub fn read_f2dot14(&mut self) -> Result<f32, FontTableError> {
        const F2DOT14_TO_FLOAT: f32 = 16384.0;

        Ok(self.read_int16()? as f32 / F2DOT14_TO_FLOAT)
    }

    #[inline]
    pub fn read_int16(&mut self) -> Result<i16, FontTableError> {
        Ok(i16::from_be_bytes(self.take::<2>()?))
    }

    /// Reads a 16 bit value and reinterprets it as `T` (the reference's
    /// `ReadInt16<TEnum>`).
    #[inline]
    pub fn read_int16_as<T: From<u16>>(&mut self) -> Result<T, FontTableError> {
        Ok(T::from(self.read_uint16()?))
    }

    #[inline]
    pub fn read_fword(&mut self) -> Result<i16, FontTableError> {
        self.read_int16()
    }

    pub fn read_fword_array(&mut self, length: i32) -> Result<Vec<i16>, FontTableError> {
        self.read_int16_array(length)
    }

    #[inline]
    pub fn read_ufword(&mut self) -> Result<u16, FontTableError> {
        self.read_uint16()
    }

    pub fn read_fixed(&mut self) -> Result<f32, FontTableError> {
        Ok(i32::from_be_bytes(self.take::<4>()?) as f32 / 65536.0)
    }

    pub fn read_version16_dot16(&mut self) -> Result<FontVersion, FontTableError> {
        Ok(FontVersion::new(u32::from_be_bytes(self.take::<4>()?)))
    }

    #[inline]
    pub fn read_int32(&mut self) -> Result<i32, FontTableError> {
        Ok(i32::from_be_bytes(self.take::<4>()?))
    }

    #[inline]
    pub fn read_int64(&mut self) -> Result<i64, FontTableError> {
        Ok(i64::from_be_bytes(self.take::<8>()?))
    }

    #[inline]
    pub fn read_uint16(&mut self) -> Result<u16, FontTableError> {
        Ok(u16::from_be_bytes(self.take::<2>()?))
    }

    #[inline]
    pub fn read_offset16(&mut self) -> Result<u16, FontTableError> {
        self.read_uint16()
    }

    /// Reads a 16 bit value and reinterprets it as `T` (the reference's
    /// `ReadUInt16<TEnum>`).
    #[inline]
    pub fn read_uint16_as<T: From<u16>>(&mut self) -> Result<T, FontTableError> {
        Ok(T::from(self.read_uint16()?))
    }

    pub fn read_uint16_array(&mut self, length: i32) -> Result<Vec<u16>, FontTableError> {
        let length = self.ensure_available_elements(length, 2)?;
        let mut data = Vec::with_capacity(length);

        for _ in 0..length {
            data.push(self.read_uint16()?);
        }

        Ok(data)
    }

    pub fn read_uint16_array_into(&mut self, buffer: &mut [u16]) -> Result<(), FontTableError> {
        for value in buffer.iter_mut() {
            *value = self.read_uint16()?;
        }

        Ok(())
    }

    pub fn read_uint32_array(&mut self, length: i32) -> Result<Vec<u32>, FontTableError> {
        let length = self.ensure_available_elements(length, 4)?;
        let mut data = Vec::with_capacity(length);

        for _ in 0..length {
            data.push(self.read_uint32()?);
        }

        Ok(data)
    }

    pub fn read_uint8_array(&mut self, length: i32) -> Result<Vec<u8>, FontTableError> {
        let length = self.ensure_available_elements(length, 1)?;

        Ok(self.read_bytes_internal(length)?.to_vec())
    }

    pub fn read_int16_array(&mut self, length: i32) -> Result<Vec<i16>, FontTableError> {
        let length = self.ensure_available_elements(length, 2)?;
        let mut data = Vec::with_capacity(length);

        for _ in 0..length {
            data.push(self.read_int16()?);
        }

        Ok(data)
    }

    pub fn read_int16_array_into(&mut self, buffer: &mut [i16]) -> Result<(), FontTableError> {
        for value in buffer.iter_mut() {
            *value = self.read_int16()?;
        }

        Ok(())
    }

    #[inline]
    pub fn read_uint8(&mut self) -> Result<u8, FontTableError> {
        self.read_byte()
    }

    pub fn read_uint24(&mut self) -> Result<i32, FontTableError> {
        let high_byte = self.read_byte()? as i32;

        Ok((high_byte << 16) | self.read_uint16()? as i32)
    }

    #[inline]
    pub fn read_uint32(&mut self) -> Result<u32, FontTableError> {
        Ok(u32::from_be_bytes(self.take::<4>()?))
    }

    #[inline]
    pub fn read_offset32(&mut self) -> Result<u32, FontTableError> {
        self.read_uint32()
    }

    /// Reads up to `count` bytes, truncating at the end of the span; the
    /// returned vector is shorter than `count` when fewer bytes remain.
    pub fn read_bytes(&mut self, count: i32) -> Vec<u8> {
        let available = (count.max(0) as usize).min(self.remaining());
        let bytes = self.span[self.position..self.position + available].to_vec();

        self.position += available;

        bytes
    }

    pub fn read_string(&mut self, bytes_to_read: i32, encoding: Encoding) -> Result<String, FontTableError> {
        let size = self.ensure_available(bytes_to_read as i64)?;

        Ok(encoding.get_string(self.read_bytes_internal(size)?))
    }

    pub fn read_tag(&mut self) -> Result<String, FontTableError> {
        Ok(Encoding::UTF8.get_string(&self.take::<4>()?))
    }

    pub fn read_offset(&mut self, size: i32) -> Result<i32, FontTableError> {
        match size {
            1 => Ok(self.read_byte()? as i32),
            2 => Ok(((self.read_byte()? as i32) << 8) | self.read_byte()? as i32),
            3 => Ok(((self.read_byte()? as i32) << 16)
                | ((self.read_byte()? as i32) << 8)
                | self.read_byte()? as i32),
            4 => Ok(((self.read_byte()? as i32) << 24)
                | ((self.read_byte()? as i32) << 16)
                | ((self.read_byte()? as i32) << 8)
                | self.read_byte()? as i32),
            _ => Err(FontTableError::InvalidOperation(
                "Operation is not valid due to the current state of the object.".to_string(),
            )),
        }
    }

    #[inline]
    fn remaining(&self) -> usize {
        self.span.len() - self.position
    }

    /// Reads exactly `N` bytes and advances.
    #[inline]
    fn take<const N: usize>(&mut self) -> Result<[u8; N], FontTableError> {
        match self.span.get(self.position..).and_then(|rest| rest.first_chunk::<N>()) {
            Some(bytes) => {
                self.position += N;
                Ok(*bytes)
            }
            None => Err(self.end_of_span(N as i64)),
        }
    }

    fn read_bytes_internal(&mut self, size: usize) -> Result<&'a [u8], FontTableError> {
        if size > self.remaining() {
            return Err(self.end_of_span(size as i64));
        }

        let bytes = &self.span[self.position..self.position + size];

        self.position += size;

        Ok(bytes)
    }

    /// Rejects negative sizes and sizes larger than what remains.
    fn ensure_available(&self, size: i64) -> Result<usize, FontTableError> {
        // Negative sizes are rejected the same way the reference's unsigned
        // comparison rejects them.
        if size < 0 {
            return Err(self.end_of_span(size as i32 as u32 as i64));
        }

        if size > self.remaining() as i64 {
            return Err(self.end_of_span(size));
        }

        Ok(size as usize)
    }

    /// Validates that `count` elements of `element_size` bytes are available
    /// before any allocation, so a hostile count cannot trigger a huge
    /// allocation.
    fn ensure_available_elements(&self, count: i32, element_size: i64) -> Result<usize, FontTableError> {
        if count < 0 {
            return Err(FontTableError::ArgumentOutOfRange {
                param_name: "count",
                message: Some("count must be non-negative.".to_string()),
            });
        }

        let size = count as i64 * element_size;

        if size > self.remaining() as i64 {
            return Err(self.end_of_span(size));
        }

        Ok(count as usize)
    }

    fn end_of_span(&self, size: i64) -> FontTableError {
        FontTableError::EndOfSpan { missing: size - self.remaining() as i64 }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_big_endian_values() {
        let data = [0x12, 0x34, 0xFF, 0xFE, 0x00, 0x01, 0x80, 0x00, 0xAB];
        let mut reader = BigEndianBinaryReader::new(&data);

        assert_eq!(reader.read_uint16(), Ok(0x1234));
        assert_eq!(reader.read_int16(), Ok(-2));
        assert_eq!(reader.read_fixed(), Ok(1.5));
        assert_eq!(reader.position(), 8);
        assert_eq!(reader.read_sbyte(), Ok(0xABu8 as i8));
        assert_eq!(reader.position(), 9);
    }

    #[test]
    fn reading_past_the_end_reports_the_missing_bytes() {
        let data = [1u8, 2, 3];
        let mut reader = BigEndianBinaryReader::new(&data);

        let error = reader.read_uint32().unwrap_err();

        assert_eq!(error, FontTableError::EndOfSpan { missing: 1 });
        assert!(error.is_invalid_operation());
        assert_eq!(error.to_string(), "End of span reached with 1 byte left to read.");
        // A failed read does not advance.
        assert_eq!(reader.position(), 0);
        assert_eq!(
            reader.read_int64().unwrap_err().to_string(),
            "End of span reached with 5 bytes left to read."
        );
    }

    #[test]
    fn seek_validates_the_offset() {
        let data = [0u8; 4];
        let mut reader = BigEndianBinaryReader::new(&data);

        assert!(reader.seek(4).is_ok());
        assert!(reader.read_byte().is_err());
        assert!(reader.seek(5).unwrap_err().is_argument_out_of_range());
        assert!(reader.seek(-1).unwrap_err().is_argument_out_of_range());
        assert_eq!(reader.position(), 4);
    }

    #[test]
    fn array_reads_validate_the_count_before_allocating() {
        let data = [0u8, 1, 0, 2];
        let mut reader = BigEndianBinaryReader::new(&data);

        assert!(reader.read_uint16_array(-1).unwrap_err().is_argument_out_of_range());
        assert_eq!(
            reader.read_uint32_array(i32::MAX),
            Err(FontTableError::EndOfSpan { missing: i32::MAX as i64 * 4 - 4 })
        );
        assert_eq!(reader.read_uint16_array(2), Ok(vec![1, 2]));
    }

    #[test]
    fn read_bytes_truncates_at_the_end() {
        let data = [1u8, 2, 3];
        let mut reader = BigEndianBinaryReader::new(&data);

        assert_eq!(reader.read_bytes(-5), Vec::<u8>::new());
        assert_eq!(reader.read_bytes(2), vec![1, 2]);
        assert_eq!(reader.read_bytes(10), vec![3]);
        assert_eq!(reader.read_bytes(10), Vec::<u8>::new());
    }

    #[test]
    fn reads_tags_strings_offsets_and_uint24() {
        let data = [b'h', b'e', b'a', b'd', 0x00, 0x41, 0x01, 0x02, 0x03];
        let mut reader = BigEndianBinaryReader::new(&data);

        assert_eq!(reader.read_tag().unwrap(), "head");
        assert_eq!(reader.read_string(2, Encoding::BigEndianUnicode).unwrap(), "A");
        assert_eq!(reader.read_uint24(), Ok(0x010203));
        assert!(reader.seek(6).is_ok());
        assert_eq!(reader.read_offset(3), Ok(0x010203));
        assert!(reader.read_offset(5).unwrap_err().is_invalid_operation());
        assert!(reader.read_string(-1, Encoding::UTF8).is_err());
    }
}
