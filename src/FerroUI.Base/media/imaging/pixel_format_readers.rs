//! Pixel readers: decode rows of any supported pixel format into RGBA8888 pixels.
//!
//! The reference implementation walks raw pointers. Here a reader keeps a byte
//! offset (its "address") into the source slice that is handed to every
//! [`IPixelFormatReader::read_next`] call, so reading past the end of the slice
//! panics instead of touching foreign memory.


use crate::platform::{PixelFormat, PixelFormats};
use crate::PixelSize;

/// One pixel with four 16-bit channels, stored as R, G, B, A.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub(crate) struct Rgba64Pixel {
    pub r: u16,
    pub g: u16,
    pub b: u16,
    pub a: u16,
}

impl Rgba64Pixel {
    /// The size of one pixel in bytes.
    pub const SIZE: usize = 8;

    pub const fn new(r: u16, g: u16, b: u16, a: u16) -> Self {
        Self { r, g, b, a }
    }
}

/// One pixel with four 8-bit channels, stored as R, G, B, A.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub(crate) struct Rgba8888Pixel {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Rgba8888Pixel {
    /// The size of one pixel in bytes.
    pub const SIZE: usize = 4;

    pub const fn new(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }
}

/// Reads consecutive pixels of one pixel format.
pub(crate) trait IPixelFormatReader: Default {
    /// Reads the pixel at the current address of `source` and advances.
    fn read_next(&mut self, source: &[u8]) -> Rgba8888Pixel;

    /// Moves the reader to `address`, a byte offset into the source
    /// (`row * stride` for the start of a row).
    fn reset(&mut self, address: usize);
}

const WHITE: Rgba8888Pixel = Rgba8888Pixel::new(255, 255, 255, 255);

const BLACK: Rgba8888Pixel = Rgba8888Pixel::new(0, 0, 0, 255);

#[inline]
fn read_u16(source: &[u8], address: usize) -> u16 {
    u16::from_ne_bytes([source[address], source[address + 1]])
}

#[inline]
fn gray(value: u8) -> Rgba8888Pixel {
    Rgba8888Pixel::new(value, value, value, 255)
}

/// Scales a packed channel (`value` out of `max`) to 8 bits: single precision
/// math, rounded half to even.
#[inline]
fn unpack_channel(value: u16, max: f32) -> u8 {
    ((value as f32 / max * 255.0) as f64).round_ties_even() as u8
}

/// 1 bit per pixel, most significant bit first.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct BlackWhitePixelFormatReader {
    bit: i32,
    address: usize,
}

impl IPixelFormatReader for BlackWhitePixelFormatReader {
    fn reset(&mut self, address: usize) {
        self.address = address;
        self.bit = 0;
    }

    fn read_next(&mut self, source: &[u8]) -> Rgba8888Pixel {
        let shift = 7 - self.bit;
        let value = (source[self.address] >> shift) & 1;
        self.bit += 1;
        if self.bit == 8 {
            self.address += 1;
            self.bit = 0;
        }
        if value == 1 {
            WHITE
        } else {
            BLACK
        }
    }
}

/// 2 bits per pixel, most significant bits first.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct Gray2PixelFormatReader {
    bit: i32,
    address: usize,
}

impl Gray2PixelFormatReader {
    const PALETTE: [Rgba8888Pixel; 4] = [
        BLACK,
        Rgba8888Pixel::new(0x55, 0x55, 0x55, 255),
        Rgba8888Pixel::new(0xAA, 0xAA, 0xAA, 255),
        WHITE,
    ];
}

impl IPixelFormatReader for Gray2PixelFormatReader {
    fn reset(&mut self, address: usize) {
        self.address = address;
        self.bit = 0;
    }

    fn read_next(&mut self, source: &[u8]) -> Rgba8888Pixel {
        let shift = 6 - self.bit;
        let value = (source[self.address] >> shift) & 3;
        self.bit += 2;
        if self.bit == 8 {
            self.address += 1;
            self.bit = 0;
        }

        Self::PALETTE[value as usize]
    }
}

/// 4 bits per pixel, high nibble first.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct Gray4PixelFormatReader {
    bit: i32,
    address: usize,
}

impl IPixelFormatReader for Gray4PixelFormatReader {
    fn reset(&mut self, address: usize) {
        self.address = address;
        self.bit = 0;
    }

    fn read_next(&mut self, source: &[u8]) -> Rgba8888Pixel {
        let shift = 4 - self.bit;
        let mut value = (source[self.address] >> shift) & 0xF;
        value |= value << 4;
        self.bit += 4;
        if self.bit == 8 {
            self.address += 1;
            self.bit = 0;
        }

        gray(value)
    }
}

/// 8 bits per pixel.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct Gray8PixelFormatReader {
    address: usize,
}

impl IPixelFormatReader for Gray8PixelFormatReader {
    fn reset(&mut self, address: usize) {
        self.address = address;
    }

    fn read_next(&mut self, source: &[u8]) -> Rgba8888Pixel {
        let value = source[self.address];
        self.address += 1;

        gray(value)
    }
}

/// 16 bits per pixel (native byte order); the high byte is kept.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct Gray16PixelFormatReader {
    address: usize,
}

impl IPixelFormatReader for Gray16PixelFormatReader {
    fn read_next(&mut self, source: &[u8]) -> Rgba8888Pixel {
        let value16 = read_u16(source, self.address);
        self.address += 2;
        let value8 = (value16 >> 8) as u8;

        gray(value8)
    }

    fn reset(&mut self, address: usize) {
        self.address = address;
    }
}

/// One linear 32-bit float per pixel, converted with a 2.2 gamma.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct Gray32FloatPixelFormatReader {
    address: usize,
}

impl IPixelFormatReader for Gray32FloatPixelFormatReader {
    fn read_next(&mut self, source: &[u8]) -> Rgba8888Pixel {
        let a = self.address;
        let f = f32::from_ne_bytes([source[a], source[a + 1], source[a + 2], source[a + 3]]);
        let srgb = (f as f64).powf(1.0 / 2.2);
        let value = (srgb * 255.0) as u8;

        self.address += 4;

        gray(value)
    }

    fn reset(&mut self, address: usize) {
        self.address = address;
    }
}

/// Four 16-bit channels (native byte order); the high bytes are kept.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct Rgba64PixelFormatReader {
    address: usize,
}

impl IPixelFormatReader for Rgba64PixelFormatReader {
    fn read_next(&mut self, source: &[u8]) -> Rgba8888Pixel {
        let a = self.address;
        let value = Rgba64Pixel::new(
            read_u16(source, a),
            read_u16(source, a + 2),
            read_u16(source, a + 4),
            read_u16(source, a + 6),
        );

        self.address += Rgba64Pixel::SIZE;

        Rgba8888Pixel {
            a: (value.a >> 8) as u8,
            b: (value.b >> 8) as u8,
            g: (value.g >> 8) as u8,
            r: (value.r >> 8) as u8,
        }
    }

    fn reset(&mut self, address: usize) {
        self.address = address;
    }
}

/// Three bytes per pixel: R, G, B.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct Rgb24PixelFormatReader {
    address: usize,
}

impl IPixelFormatReader for Rgb24PixelFormatReader {
    fn read_next(&mut self, source: &[u8]) -> Rgba8888Pixel {
        let addr = self.address;
        self.address += 3;

        Rgba8888Pixel { r: source[addr], g: source[addr + 1], b: source[addr + 2], a: 255 }
    }

    fn reset(&mut self, address: usize) {
        self.address = address;
    }
}

/// Three bytes per pixel: B, G, R.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct Bgr24PixelFormatReader {
    address: usize,
}

impl IPixelFormatReader for Bgr24PixelFormatReader {
    fn read_next(&mut self, source: &[u8]) -> Rgba8888Pixel {
        let addr = self.address;
        self.address += 3;

        Rgba8888Pixel { r: source[addr + 2], g: source[addr + 1], b: source[addr], a: 255 }
    }

    fn reset(&mut self, address: usize) {
        self.address = address;
    }
}

/// 16 bits per pixel (native byte order): 5 bits each of R, G, B from the top.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct Bgr555PixelFormatReader {
    address: usize,
}

impl Bgr555PixelFormatReader {
    fn un_pack(value: u16) -> Rgba8888Pixel {
        let r = unpack_channel((value >> 10) & 0x1F, 31.0);
        let g = unpack_channel((value >> 5) & 0x1F, 31.0);
        let b = unpack_channel(value & 0x1F, 31.0);

        Rgba8888Pixel::new(r, g, b, 255)
    }
}

impl IPixelFormatReader for Bgr555PixelFormatReader {
    fn read_next(&mut self, source: &[u8]) -> Rgba8888Pixel {
        let value = read_u16(source, self.address);

        self.address += 2;

        Self::un_pack(value)
    }

    fn reset(&mut self, address: usize) {
        self.address = address;
    }
}

/// 16 bits per pixel (native byte order): 5 bits R, 6 bits G, 5 bits B from the top.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct Bgr565PixelFormatReader {
    address: usize,
}

impl Bgr565PixelFormatReader {
    fn un_pack(value: u16) -> Rgba8888Pixel {
        let r = unpack_channel((value >> 11) & 0x1F, 31.0);
        let g = unpack_channel((value >> 5) & 0x3F, 63.0);
        let b = unpack_channel(value & 0x1F, 31.0);

        Rgba8888Pixel::new(r, g, b, 255)
    }
}

impl IPixelFormatReader for Bgr565PixelFormatReader {
    fn read_next(&mut self, source: &[u8]) -> Rgba8888Pixel {
        let value = read_u16(source, self.address);

        self.address += 2;

        Self::un_pack(value)
    }

    fn reset(&mut self, address: usize) {
        self.address = address;
    }
}

/// Four bytes per pixel: R, G, B, A.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct Rgba8888PixelFormatReader {
    address: usize,
}

impl IPixelFormatReader for Rgba8888PixelFormatReader {
    fn read_next(&mut self, source: &[u8]) -> Rgba8888Pixel {
        let a = self.address;
        let value = Rgba8888Pixel::new(source[a], source[a + 1], source[a + 2], source[a + 3]);

        self.address += Rgba8888Pixel::SIZE;

        value
    }

    fn reset(&mut self, address: usize) {
        self.address = address;
    }
}

/// Four bytes per pixel: R, G, B and one ignored byte.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct Rgb32PixelFormatReader {
    address: usize,
}

impl IPixelFormatReader for Rgb32PixelFormatReader {
    fn read_next(&mut self, source: &[u8]) -> Rgba8888Pixel {
        let a = self.address;
        // The fourth byte is ignored but still part of the pixel.
        let _ = source[a + 3];

        let value = Rgba8888Pixel::new(source[a], source[a + 1], source[a + 2], 255);

        self.address += Rgba8888Pixel::SIZE;

        value
    }

    fn reset(&mut self, address: usize) {
        self.address = address;
    }
}

/// Four bytes per pixel: B, G, R, A.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct Bgra8888PixelFormatReader {
    address: usize,
}

impl IPixelFormatReader for Bgra8888PixelFormatReader {
    fn read_next(&mut self, source: &[u8]) -> Rgba8888Pixel {
        let addr = self.address;

        self.address += 4;

        Rgba8888Pixel::new(source[addr + 2], source[addr + 1], source[addr], source[addr + 3])
    }

    fn reset(&mut self, address: usize) {
        self.address = address;
    }
}

/// Four bytes per pixel: B, G, R and one ignored byte.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct Bgr32PixelFormatReader {
    address: usize,
}

impl IPixelFormatReader for Bgr32PixelFormatReader {
    fn read_next(&mut self, source: &[u8]) -> Rgba8888Pixel {
        let a = self.address;
        // The fourth byte is ignored but still part of the pixel.
        let _ = source[a + 3];

        let value = Rgba8888Pixel::new(source[a + 2], source[a + 1], source[a], 255);

        self.address += Rgba8888Pixel::SIZE;

        value
    }

    fn reset(&mut self, address: usize) {
        self.address = address;
    }
}

/// The byte offset of row `y` (`stride * y`).
///
/// Panics when the offset is negative: slices cannot be walked backwards.
#[inline]
pub(crate) fn row_address(stride: i32, y: i32) -> usize {
    let address = stride as i64 * y as i64;
    assert!(address >= 0, "negative row offset (stride {stride}, row {y})");
    address as usize
}

/// Panics with a clear message when `len` bytes cannot hold `size` pixels of
/// `format` laid out with `stride`.
pub(crate) fn check_buffer_length(what: &str, len: usize, size: PixelSize, stride: i32, format: PixelFormat) {
    if size.width <= 0 || size.height <= 0 {
        return;
    }

    let row = (size.width as i64 * format.bits_per_pixel() as i64 + 7) / 8;
    let required = stride as i64 * (size.height as i64 - 1) + row;
    assert!(
        stride >= 0 && required <= len as i64,
        "{what} buffer too small: {len} bytes, but {size} pixels of {format} with stride {stride} need {required}",
        size = format_args!("{}x{}", size.width, size.height),
    );
}

/// Decodes whole bitmaps of any supported pixel format.
pub(crate) struct PixelFormatReader;

impl PixelFormatReader {
    /// Whether [`read`](Self::read) can decode `format`.
    #[allow(clippy::match_like_matches_macro)]
    pub fn supports_format(format: PixelFormat) -> bool {
        match format {
            PixelFormats::RGB565
            | PixelFormats::RGBA8888
            | PixelFormats::BGRA8888
            | PixelFormats::BLACK_WHITE
            | PixelFormats::GRAY2
            | PixelFormats::GRAY4
            | PixelFormats::GRAY8
            | PixelFormats::GRAY16
            | PixelFormats::GRAY32_FLOAT
            | PixelFormats::RGBA64
            | PixelFormats::RGB24
            | PixelFormats::RGB32
            | PixelFormats::BGR24
            | PixelFormats::BGR32
            | PixelFormats::BGR555
            | PixelFormats::BGR565 => true,
            #[allow(unreachable_patterns)]
            _ => false,
        }
    }

    fn read_with<T: IPixelFormatReader>(pixels: &mut [Rgba8888Pixel], source: &[u8], size: PixelSize, stride: i32) {
        let mut reader = T::default();

        let w = size.width;
        let h = size.height;
        let mut count = 0;

        for y in 0..h {
            reader.reset(row_address(stride, y));

            for _ in 0..w {
                pixels[count] = reader.read_next(source);
                count += 1;
            }
        }
    }

    /// Decodes `size` pixels of `format` from `source` (rows `stride` bytes
    /// apart) into `pixels`, row by row.
    ///
    /// Panics when `source` or `pixels` is too small.
    pub fn read(pixels: &mut [Rgba8888Pixel], source: &[u8], size: PixelSize, stride: i32, format: PixelFormat) {
        check_buffer_length("source", source.len(), size, stride, format);

        match format {
            PixelFormats::RGB565 => Self::read_with::<Bgr565PixelFormatReader>(pixels, source, size, stride),
            PixelFormats::RGBA8888 => Self::read_with::<Rgba8888PixelFormatReader>(pixels, source, size, stride),
            PixelFormats::BGRA8888 => Self::read_with::<Bgra8888PixelFormatReader>(pixels, source, size, stride),
            PixelFormats::BLACK_WHITE => Self::read_with::<BlackWhitePixelFormatReader>(pixels, source, size, stride),
            PixelFormats::GRAY2 => Self::read_with::<Gray2PixelFormatReader>(pixels, source, size, stride),
            PixelFormats::GRAY4 => Self::read_with::<Gray4PixelFormatReader>(pixels, source, size, stride),
            PixelFormats::GRAY8 => Self::read_with::<Gray8PixelFormatReader>(pixels, source, size, stride),
            PixelFormats::GRAY16 => Self::read_with::<Gray16PixelFormatReader>(pixels, source, size, stride),
            PixelFormats::GRAY32_FLOAT => Self::read_with::<Gray32FloatPixelFormatReader>(pixels, source, size, stride),
            PixelFormats::RGBA64 => Self::read_with::<Rgba64PixelFormatReader>(pixels, source, size, stride),
            PixelFormats::RGB24 => Self::read_with::<Rgb24PixelFormatReader>(pixels, source, size, stride),
            PixelFormats::RGB32 => Self::read_with::<Rgb32PixelFormatReader>(pixels, source, size, stride),
            PixelFormats::BGR24 => Self::read_with::<Bgr24PixelFormatReader>(pixels, source, size, stride),
            PixelFormats::BGR32 => Self::read_with::<Bgr32PixelFormatReader>(pixels, source, size, stride),
            PixelFormats::BGR555 => Self::read_with::<Bgr555PixelFormatReader>(pixels, source, size, stride),
            PixelFormats::BGR565 => Self::read_with::<Bgr565PixelFormatReader>(pixels, source, size, stride),
            #[allow(unreachable_patterns)]
            _ => panic!("Pixel format {format} is not supported"),
        }
    }
}

// Additional coverage (the reference suite has no tests dedicated to this file;
// the readers are exercised by the writer and transcoder tests).
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn supports_every_known_format() {
        for format in [
            PixelFormats::RGB565,
            PixelFormats::RGBA8888,
            PixelFormats::BGRA8888,
            PixelFormats::BLACK_WHITE,
            PixelFormats::GRAY2,
            PixelFormats::GRAY4,
            PixelFormats::GRAY8,
            PixelFormats::GRAY16,
            PixelFormats::GRAY32_FLOAT,
            PixelFormats::RGBA64,
            PixelFormats::RGB24,
            PixelFormats::RGB32,
            PixelFormats::BGR24,
            PixelFormats::BGR32,
            PixelFormats::BGR555,
            PixelFormats::BGR565,
        ] {
            assert!(PixelFormatReader::supports_format(format));
        }
    }

    #[test]
    fn reads_packed_bits_most_significant_first() {
        let mut bw = BlackWhitePixelFormatReader::default();
        let source = [0b1010_0000u8, 0b1000_0000];
        let read: Vec<_> = (0..9).map(|_| bw.read_next(&source)).collect();
        assert_eq!(read, [WHITE, BLACK, WHITE, BLACK, BLACK, BLACK, BLACK, BLACK, WHITE]);

        let mut gray2 = Gray2PixelFormatReader::default();
        let source = [0b00_01_10_11u8];
        let read: Vec<_> = (0..4).map(|_| gray2.read_next(&source)).collect();
        assert_eq!(read, [BLACK, gray(0x55), gray(0xAA), WHITE]);

        let mut gray4 = Gray4PixelFormatReader::default();
        let source = [0x3Cu8];
        assert_eq!(gray4.read_next(&source), gray(0x33));
        assert_eq!(gray4.read_next(&source), gray(0xCC));
    }

    #[test]
    fn reads_sixteen_bit_formats() {
        let mut source = Vec::new();
        source.extend_from_slice(&0xF800u16.to_ne_bytes());
        source.extend_from_slice(&0x07E0u16.to_ne_bytes());
        source.extend_from_slice(&0x001Fu16.to_ne_bytes());

        let mut pixels = [Rgba8888Pixel::default(); 3];
        // Rgb565 is decoded exactly like Bgr565.
        for format in [PixelFormats::RGB565, PixelFormats::BGR565] {
            PixelFormatReader::read(&mut pixels, &source, PixelSize::new(3, 1), 6, format);
            assert_eq!(
                pixels,
                [
                    Rgba8888Pixel::new(255, 0, 0, 255),
                    Rgba8888Pixel::new(0, 255, 0, 255),
                    Rgba8888Pixel::new(0, 0, 255, 255)
                ]
            );
        }

        let mut source = Vec::new();
        source.extend_from_slice(&0x7C00u16.to_ne_bytes());
        source.extend_from_slice(&0x0210u16.to_ne_bytes());
        PixelFormatReader::read(&mut pixels, &source, PixelSize::new(2, 1), 4, PixelFormats::BGR555);
        assert_eq!(pixels[0], Rgba8888Pixel::new(255, 0, 0, 255));
        // 16 / 31 * 255 = 131.6 -> 132
        assert_eq!(pixels[1], Rgba8888Pixel::new(0, 132, 132, 255));

        let source = 0xABCDu16.to_ne_bytes();
        PixelFormatReader::read(&mut pixels, &source, PixelSize::new(1, 1), 2, PixelFormats::GRAY16);
        assert_eq!(pixels[0], gray(0xAB));
    }

    #[test]
    fn reads_rows_at_stride() {
        // 2x2 Rgb24 with one padding byte per row.
        let source = [1, 2, 3, 4, 5, 6, 99, 99, 7, 8, 9, 10, 11, 12];
        let mut pixels = [Rgba8888Pixel::default(); 4];
        PixelFormatReader::read(&mut pixels, &source, PixelSize::new(2, 2), 8, PixelFormats::RGB24);
        assert_eq!(
            pixels,
            [
                Rgba8888Pixel::new(1, 2, 3, 255),
                Rgba8888Pixel::new(4, 5, 6, 255),
                Rgba8888Pixel::new(7, 8, 9, 255),
                Rgba8888Pixel::new(10, 11, 12, 255)
            ]
        );

        PixelFormatReader::read(&mut pixels, &source, PixelSize::new(2, 1), 8, PixelFormats::BGR24);
        assert_eq!(pixels[0], Rgba8888Pixel::new(3, 2, 1, 255));

        PixelFormatReader::read(&mut pixels, &source, PixelSize::new(2, 1), 8, PixelFormats::RGB32);
        assert_eq!(pixels[..2], [Rgba8888Pixel::new(1, 2, 3, 255), Rgba8888Pixel::new(5, 6, 99, 255)]);

        PixelFormatReader::read(&mut pixels, &source, PixelSize::new(2, 1), 8, PixelFormats::BGR32);
        assert_eq!(pixels[..2], [Rgba8888Pixel::new(3, 2, 1, 255), Rgba8888Pixel::new(99, 6, 5, 255)]);
    }

    #[test]
    #[should_panic(expected = "source buffer too small")]
    fn short_source_panics() {
        let mut pixels = [Rgba8888Pixel::default(); 4];
        PixelFormatReader::read(&mut pixels, &[0u8; 15], PixelSize::new(2, 2), 8, PixelFormats::RGBA8888);
    }
}
