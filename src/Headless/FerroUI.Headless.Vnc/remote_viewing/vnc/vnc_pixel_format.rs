//! Port of `Vnc/VncPixelFormat.cs` of RemoteViewing (BSD-2-Clause,
//! Copyright (c) 2013 James F. Bellinger; `NOTICE.md` of this crate).
//!
//! Two differences from the original. `copy` converts between two pixel
//! formats of true colour, as its documentation says; the original copies
//! between equal formats only and leaves the target untouched otherwise, so
//! a client that asks for a format of its own received black pixels. The
//! overloads of `Copy` over raw pointers and `CopyFromFramebuffer` (which
//! the client of the library uses) are not ported: the slices of
//! [`copy`](VncPixelFormat::copy) take their place.

use super::{VncError, VncFailureReason, VncFramebuffer, VncRectangle, VncUtility};
use std::fmt;

/// Describes the low-level arrangement of a framebuffer pixel.
#[derive(Clone, Copy, Debug)]
pub struct VncPixelFormat {
    bits_per_pixel: i32,
    bytes_per_pixel: i32,
    bit_depth: i32,
    red_bits: i32,
    red_shift: i32,
    green_bits: i32,
    green_shift: i32,
    blue_bits: i32,
    blue_shift: i32,
    is_little_endian: bool,
    is_palettized: bool,
}

impl Default for VncPixelFormat {
    /// The format with 8 bits each of red, green, and blue channels.
    fn default() -> Self {
        Self::RGB32
    }
}

impl VncPixelFormat {
    /// A [`VncPixelFormat`] with 8 bits of red, green and blue channels: 32
    /// bits a pixel, little-endian, red shifted by 16, green by 8, blue by
    /// 0 (in memory: blue, green, red, unused).
    pub const RGB32: VncPixelFormat = VncPixelFormat {
        bits_per_pixel: 32,
        bytes_per_pixel: 4,
        bit_depth: 24,
        red_bits: 8,
        red_shift: 16,
        green_bits: 8,
        green_shift: 8,
        blue_bits: 8,
        blue_shift: 0,
        is_little_endian: true,
        is_palettized: false,
    };

    /// Initializes a new instance of the [`VncPixelFormat`] class.
    ///
    /// `bits_per_pixel` is the number of bits used to store a pixel
    /// (currently 8, 16, or 32); `bit_depth` the bit depth of the pixel
    /// (currently 6 or 24); the bits and shifts of each colour say how many
    /// bits represent it and how many bits left its value is shifted.
    ///
    /// # Panics
    /// When a value is out of its range (`ArgumentOutOfRangeException`).
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        bits_per_pixel: i32,
        bit_depth: i32,
        red_bits: i32,
        red_shift: i32,
        green_bits: i32,
        green_shift: i32,
        blue_bits: i32,
        blue_shift: i32,
        is_little_endian: bool,
        is_palettized: bool,
    ) -> Self {
        match Self::try_new(
            bits_per_pixel,
            bit_depth,
            red_bits,
            red_shift,
            green_bits,
            green_shift,
            blue_bits,
            blue_shift,
            is_little_endian,
            is_palettized,
        ) {
            Ok(format) => format,
            Err(parameter) => {
                panic!("Specified argument was out of the range of valid values. (Parameter '{parameter}')")
            }
        }
    }

    /// The checks of the constructor: the name of the parameter that is out
    /// of its range.
    #[allow(clippy::too_many_arguments)]
    fn try_new(
        bits_per_pixel: i32,
        bit_depth: i32,
        red_bits: i32,
        red_shift: i32,
        green_bits: i32,
        green_shift: i32,
        blue_bits: i32,
        blue_shift: i32,
        is_little_endian: bool,
        is_palettized: bool,
    ) -> Result<Self, &'static str> {
        if bits_per_pixel != 8 && bits_per_pixel != 16 && bits_per_pixel != 32 {
            return Err("bitsPerPixel");
        }

        if bit_depth != 6 && bit_depth != 24 {
            return Err("bitDepth");
        }

        let in_range = |bits: i32, shift: i32| {
            bits >= 0 && shift >= 0 && bits <= bit_depth && shift <= bit_depth
        };
        if !in_range(red_bits, red_shift) {
            return Err("redBits");
        }

        if !in_range(green_bits, green_shift) {
            return Err("greenBits");
        }

        if !in_range(blue_bits, blue_shift) {
            return Err("blueBits");
        }

        Ok(Self {
            bits_per_pixel,
            bytes_per_pixel: bits_per_pixel / 8,
            bit_depth,
            red_bits,
            red_shift,
            green_bits,
            green_shift,
            blue_bits,
            blue_shift,
            is_little_endian,
            is_palettized,
        })
    }

    /// Gets a [`VncPixelFormat`] with 8 bits of red, green and blue
    /// channels.
    pub fn rgb32() -> VncPixelFormat {
        Self::RGB32
    }

    /// Gets the number of bits used to store a pixel.
    pub fn bits_per_pixel(&self) -> i32 {
        self.bits_per_pixel
    }

    /// Gets the number of bytes used to store a pixel.
    pub fn bytes_per_pixel(&self) -> i32 {
        self.bytes_per_pixel
    }

    /// Gets the bit depth of the pixel.
    pub fn bit_depth(&self) -> i32 {
        self.bit_depth
    }

    /// Gets the number of bits used to represent red.
    pub fn red_bits(&self) -> i32 {
        self.red_bits
    }

    /// Gets the number of bits left the red value is shifted.
    pub fn red_shift(&self) -> i32 {
        self.red_shift
    }

    /// Gets the number of bits used to represent green.
    pub fn green_bits(&self) -> i32 {
        self.green_bits
    }

    /// Gets the maximum value of the red color.
    pub fn red_max(&self) -> u16 {
        ((1i32 << self.red_bits) - 1) as u16
    }

    /// Gets the maximum value of the blue color.
    pub fn blue_max(&self) -> u16 {
        ((1i32 << self.blue_bits) - 1) as u16
    }

    /// Gets the maximum value of the green color.
    pub fn green_max(&self) -> u16 {
        ((1i32 << self.green_bits) - 1) as u16
    }

    /// Gets the number of bits left the green value is shifted.
    pub fn green_shift(&self) -> i32 {
        self.green_shift
    }

    /// Gets the number of bits used to represent blue.
    pub fn blue_bits(&self) -> i32 {
        self.blue_bits
    }

    /// Gets the number of bits left the blue value is shifted.
    pub fn blue_shift(&self) -> i32 {
        self.blue_shift
    }

    /// Gets a value indicating whether the pixel is little-endian.
    pub fn is_little_endian(&self) -> bool {
        self.is_little_endian
    }

    /// Gets a value indicating whether the framebuffer stores palette
    /// indices (true) or colors (false).
    pub fn is_palettized(&self) -> bool {
        self.is_palettized
    }

    /// Gets the size of a [`VncPixelFormat`] when serialized to a byte
    /// array.
    pub(crate) const SIZE: usize = 16;

    /// Copies pixels between two byte arrays. A format conversion is
    /// performed if necessary (between formats of true colour; a target of
    /// another format is left as it is).
    ///
    /// Be sure to lock the framebuffer first to avoid tearing, if the
    /// connection is active.
    ///
    /// `source` holds the source image (`source_width` pixels wide,
    /// `source_stride` bytes between one Y coordinate and the next, in
    /// `source_format`); `source_rectangle` is the rectangle of it to copy.
    /// `target` holds the target image in the same terms; `target_x` and
    /// `target_y` are where in it the upper-left pixel goes.
    #[allow(clippy::too_many_arguments)]
    pub fn copy(
        source: &[u8],
        source_width: i32,
        source_stride: i32,
        source_format: &VncPixelFormat,
        source_rectangle: VncRectangle,
        target: &mut [u8],
        target_width: i32,
        target_stride: i32,
        target_format: &VncPixelFormat,
        target_x: i32,
        target_y: i32,
    ) {
        if source_rectangle.is_empty() {
            return;
        }

        let (x, w) = (source_rectangle.x as usize, source_rectangle.width as usize);
        let (y, h) = (
            source_rectangle.y as usize,
            source_rectangle.height as usize,
        );
        let (source_stride, target_stride) = (source_stride as usize, target_stride as usize);
        let (target_x, target_y) = (target_x as usize, target_y as usize);

        if source_format == target_format {
            let bytes_per_pixel = source_format.bytes_per_pixel as usize;
            if source_rectangle.width == source_width
                && source_width == target_width
                && source_stride == target_stride
                && target_x == 0
            {
                let source_start = source_stride * y;
                let target_start = target_stride * target_y;
                let length = target_stride * h;

                target[target_start..target_start + length]
                    .copy_from_slice(&source[source_start..source_start + length]);
            } else {
                let length = w * bytes_per_pixel;
                for iy in 0..h {
                    let source_start = (source_stride * (iy + y)) + (x * bytes_per_pixel);
                    let target_start =
                        (target_stride * (iy + target_y)) + (target_x * bytes_per_pixel);

                    target[target_start..target_start + length]
                        .copy_from_slice(&source[source_start..source_start + length]);
                }
            }
        } else if !source_format.is_palettized && !target_format.is_palettized {
            let (source_bytes, target_bytes) = (
                source_format.bytes_per_pixel as usize,
                target_format.bytes_per_pixel as usize,
            );
            for iy in 0..h {
                for ix in 0..w {
                    let source_start = (source_stride * (iy + y)) + ((x + ix) * source_bytes);
                    let target_start =
                        (target_stride * (iy + target_y)) + ((target_x + ix) * target_bytes);
                    let value = source_format
                        .read_pixel(&source[source_start..source_start + source_bytes]);
                    let converted = target_format.convert_from(value, source_format);
                    target_format.write_pixel(
                        converted,
                        &mut target[target_start..target_start + target_bytes],
                    );
                }
            }
        }
    }

    /// The value of a pixel stored in `bytes` (the bytes of one pixel).
    fn read_pixel(&self, bytes: &[u8]) -> u32 {
        let mut value = 0u32;
        if self.is_little_endian {
            for (i, byte) in bytes.iter().enumerate() {
                value |= u32::from(*byte) << (8 * i);
            }
        } else {
            for byte in bytes {
                value = (value << 8) | u32::from(*byte);
            }
        }
        value
    }

    /// Stores the value of a pixel in `bytes` (the bytes of one pixel).
    fn write_pixel(&self, value: u32, bytes: &mut [u8]) {
        let count = bytes.len();
        for (i, byte) in bytes.iter_mut().enumerate() {
            let shift = if self.is_little_endian {
                8 * i
            } else {
                8 * (count - 1 - i)
            };
            *byte = (value >> shift) as u8;
        }
    }

    /// The value of a pixel of `source_format` in this format: each colour
    /// scaled from the maximum of the source to the maximum of this format.
    fn convert_from(&self, value: u32, source_format: &VncPixelFormat) -> u32 {
        fn channel(
            value: u32,
            shift: i32,
            source_max: u16,
            target_max: u16,
            target_shift: i32,
        ) -> u32 {
            let source_max = u32::from(source_max);
            let component = if source_max == 0 {
                0
            } else {
                (value >> shift) & source_max
            };
            let scaled = if source_max == 0 {
                0
            } else {
                (component * u32::from(target_max) + source_max / 2) / source_max
            };
            scaled << target_shift
        }

        channel(
            value,
            source_format.red_shift,
            source_format.red_max(),
            self.red_max(),
            self.red_shift,
        ) | channel(
            value,
            source_format.green_shift,
            source_format.green_max(),
            self.green_max(),
            self.green_shift,
        ) | channel(
            value,
            source_format.blue_shift,
            source_format.blue_max(),
            self.blue_max(),
            self.blue_shift,
        )
    }

    /// Copies a region of the framebuffer into a bitmap of the default
    /// format ([`RGB32`](Self::RGB32)) whose rows are `stride` bytes apart;
    /// `target_x` and `target_y` are where the upper-left pixel goes.
    pub fn copy_from_framebuffer(
        source: &VncFramebuffer,
        source_rectangle: VncRectangle,
        target: &mut [u8],
        stride: i32,
        target_x: i32,
        target_y: i32,
    ) {
        if source_rectangle.is_empty() {
            return;
        }

        let buffer = source.get_buffer();
        let target_width = stride / Self::RGB32.bytes_per_pixel;
        VncPixelFormat::copy(
            &buffer,
            source.width(),
            source.stride(),
            source.pixel_format(),
            source_rectangle,
            target,
            target_width,
            stride,
            &VncPixelFormat::RGB32,
            target_x,
            target_y,
        );
    }

    /// Decodes a [`VncPixelFormat`] from a byte array, starting at
    /// `offset`.
    ///
    /// Fails when the format is not one the original accepts (its
    /// `ArgumentException` and `ArgumentOutOfRangeException`).
    pub(crate) fn decode(buffer: &[u8], offset: usize) -> Result<VncPixelFormat, VncError> {
        let bits_per_pixel = i32::from(buffer[offset]);
        let depth = i32::from(buffer[offset + 1]);
        let is_little_endian = buffer[offset + 2] == 0;
        let is_palettized = buffer[offset + 3] == 0;
        let red_bits =
            Self::bits_from_max(i32::from(VncUtility::decode_uint16_be(buffer, offset + 4)))?;
        let green_bits =
            Self::bits_from_max(i32::from(VncUtility::decode_uint16_be(buffer, offset + 6)))?;
        let blue_bits =
            Self::bits_from_max(i32::from(VncUtility::decode_uint16_be(buffer, offset + 8)))?;
        let red_shift = i32::from(buffer[offset + 10]);
        let green_shift = i32::from(buffer[offset + 11]);
        let blue_shift = i32::from(buffer[offset + 12]);

        Self::try_new(
            bits_per_pixel,
            depth,
            red_bits,
            red_shift,
            green_bits,
            green_shift,
            blue_bits,
            blue_shift,
            is_little_endian,
            is_palettized,
        )
        .map_err(|parameter| {
            VncError::new(
                format!("Specified argument was out of the range of valid values. (Parameter '{parameter}')"),
                VncFailureReason::UnsupportedPixelFormat,
                None,
            )
        })
    }

    /// Serializes this [`VncPixelFormat`] to a byte array, starting at
    /// `offset`.
    pub(crate) fn encode(&self, buffer: &mut [u8], offset: usize) {
        buffer[offset] = self.bits_per_pixel as u8;
        buffer[offset + 1] = self.bit_depth as u8;
        buffer[offset + 2] = if self.is_little_endian { 0 } else { 1 };
        buffer[offset + 3] = if self.is_palettized { 0 } else { 1 };
        VncUtility::encode_uint16_be_into(buffer, offset + 4, self.red_max());
        VncUtility::encode_uint16_be_into(buffer, offset + 6, self.green_max());
        VncUtility::encode_uint16_be_into(buffer, offset + 8, self.blue_max());
        buffer[offset + 10] = self.red_shift as u8;
        buffer[offset + 11] = self.green_shift as u8;
        buffer[offset + 12] = self.blue_shift as u8;
    }

    fn bits_from_max(max: i32) -> Result<i32, VncError> {
        if max == 0 || (max & (max + 1)) != 0 {
            return Err(VncError::new(
                "Value does not fall within the expected range.",
                VncFailureReason::UnsupportedPixelFormat,
                None,
            ));
        }

        Ok((f64::from(max + 1).ln() / 2f64.ln()).round() as i32)
    }
}

impl PartialEq for VncPixelFormat {
    fn eq(&self, format: &Self) -> bool {
        self.bits_per_pixel == format.bits_per_pixel
            && self.bit_depth == format.bit_depth
            && self.red_bits == format.red_bits
            && self.red_shift == format.red_shift
            && self.green_bits == format.green_bits
            && self.green_shift == format.green_shift
            && self.blue_bits == format.blue_bits
            && self.blue_shift == format.blue_shift
            && self.is_little_endian == format.is_little_endian
            && self.is_palettized == format.is_palettized
    }
}

impl Eq for VncPixelFormat {}

impl std::hash::Hash for VncPixelFormat {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        (self.bits_per_pixel ^ self.red_bits).hash(state);
    }
}

impl fmt::Display for VncPixelFormat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} bpp; {} depth: R {} {}; G {} {}; B {} {}; LE: {}; Palettized {}",
            self.bits_per_pixel,
            self.bit_depth,
            self.red_bits,
            self.red_shift,
            self.green_bits,
            self.green_shift,
            self.blue_bits,
            self.blue_shift,
            if self.is_little_endian {
                "True"
            } else {
                "False"
            },
            if self.is_palettized { "True" } else { "False" },
        )
    }
}

#[cfg(test)]
mod tests {
    // Not from upstream.
    use super::*;

    #[test]
    fn the_format_round_trips_through_its_encoding() {
        let mut buffer = [0u8; VncPixelFormat::SIZE];
        VncPixelFormat::RGB32.encode(&mut buffer, 0);
        assert_eq!(
            [32, 24, 0, 1, 0, 255, 0, 255, 0, 255, 16, 8, 0, 0, 0, 0],
            buffer
        );
        assert_eq!(
            VncPixelFormat::RGB32,
            VncPixelFormat::decode(&buffer, 0).unwrap()
        );
    }

    #[test]
    fn a_maximum_that_is_not_a_power_of_two_less_one_is_rejected() {
        let mut buffer = [0u8; VncPixelFormat::SIZE];
        VncPixelFormat::RGB32.encode(&mut buffer, 0);
        buffer[5] = 254;
        assert_eq!(
            VncFailureReason::UnsupportedPixelFormat,
            VncPixelFormat::decode(&buffer, 0).unwrap_err().reason()
        );
    }

    #[test]
    fn equal_formats_copy_rows() {
        let source: Vec<u8> = (0..32).collect();
        let mut target = vec![0u8; 8];
        VncPixelFormat::copy(
            &source,
            4,
            16,
            &VncPixelFormat::RGB32,
            VncRectangle::new(1, 1, 2, 1),
            &mut target,
            2,
            8,
            &VncPixelFormat::RGB32,
            0,
            0,
        );
        assert_eq!(vec![20, 21, 22, 23, 24, 25, 26, 27], target);
    }

    #[test]
    fn other_formats_are_converted() {
        // Red, green and blue of RGB32 into big-endian 16 bits of 5, 6 and 5 bits.
        let rgb565 = VncPixelFormat::new(16, 24, 5, 11, 6, 5, 5, 0, false, false);
        let source = [
            0x00u8, 0x00, 0xff, 0x00, 0x00, 0xff, 0x00, 0x00, 0xff, 0x00, 0x00, 0x00,
        ];
        let mut target = vec![0u8; 6];
        VncPixelFormat::copy(
            &source,
            3,
            12,
            &VncPixelFormat::RGB32,
            VncRectangle::new(0, 0, 3, 1),
            &mut target,
            3,
            6,
            &rgb565,
            0,
            0,
        );
        assert_eq!(vec![0xf8, 0x00, 0x07, 0xe0, 0x00, 0x1f], target);
    }
}
