//! How a decoded font reaches the active backend.

use std::io::{Read, Result};

/// A stream over a decoded font that hands its buffer over instead of copying it.
///
/// Every backend makes a typeface with `IFontManagerImpl::try_create_glyph_typeface_from_stream`,
/// which reads the font with `read_to_end` into an empty vector. This type moves its buffer into
/// that vector, so the font is not copied on the way in, the application does not have to name
/// its backend, and a backend needs nothing added for web fonts. A backend that reads in another
/// way still works, through [`read`](Read::read), with a copy.
pub struct FontDataStream {
    data: Vec<u8>,
    position: usize,
}

impl FontDataStream {
    pub fn new(data: Vec<u8>) -> Self {
        Self { data, position: 0 }
    }
}

impl Read for FontDataStream {
    fn read(&mut self, buffer: &mut [u8]) -> Result<usize> {
        let rest = self.data.get(self.position..).unwrap_or_default();
        let count = rest.len().min(buffer.len());
        buffer[..count].copy_from_slice(&rest[..count]);
        self.position += count;
        Ok(count)
    }

    fn read_to_end(&mut self, buffer: &mut Vec<u8>) -> Result<usize> {
        if buffer.is_empty() && self.position == 0 {
            std::mem::swap(buffer, &mut self.data);
            self.position = buffer.len();
            return Ok(buffer.len());
        }
        let rest = self.data.get(self.position..).unwrap_or_default();
        buffer.extend_from_slice(rest);
        let count = rest.len();
        self.position += count;
        Ok(count)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_buffer_moves_into_an_empty_vector() {
        let data = vec![1u8, 2, 3, 4];
        let address = data.as_ptr();
        let mut stream = FontDataStream::new(data);
        let mut target = Vec::new();
        assert_eq!(stream.read_to_end(&mut target).unwrap(), 4);
        assert_eq!(target.as_ptr(), address, "no copy");
        assert_eq!(target, [1, 2, 3, 4]);
        assert_eq!(stream.read(&mut [0; 4]).unwrap(), 0, "nothing is left");
    }

    #[test]
    fn other_ways_of_reading_still_work() {
        let mut stream = FontDataStream::new(vec![1, 2, 3, 4]);
        let mut first = [0u8; 3];
        assert_eq!(stream.read(&mut first).unwrap(), 3);
        assert_eq!(first, [1, 2, 3]);
        let mut rest = vec![9];
        assert_eq!(stream.read_to_end(&mut rest).unwrap(), 1);
        assert_eq!(rest, [9, 4]);
    }
}
