//! Pixel writers: encode RGBA8888 pixels into rows of any supported pixel format.
//!
//! The reference implementation walks raw pointers. Here a writer keeps a byte
//! offset (its "address") into the destination slice that is handed to every
//! [`IPixelFormatWriter::write_next`] call, so writing past the end of the
//! slice panics instead of touching foreign memory.


use super::pixel_format_readers::{check_buffer_length, row_address, Rgba64Pixel, Rgba8888Pixel};
use crate::platform::{AlphaFormat, PixelFormat, PixelFormats};
use crate::PixelSize;

/// Writes consecutive pixels of one pixel format.
pub(crate) trait IPixelFormatWriter: Default {
    /// Writes `pixel` at the current address of `dest` and advances.
    fn write_next(&mut self, dest: &mut [u8], pixel: Rgba8888Pixel);

    /// Moves the writer to `address`, a byte offset into the destination
    /// (`row * stride` for the start of a row).
    fn reset(&mut self, address: usize);
}

#[inline]
fn write_u16(dest: &mut [u8], address: usize, value: u16) {
    dest[address..address + 2].copy_from_slice(&value.to_ne_bytes());
}

/// The luminance of a pixel, in single precision.
#[inline]
fn luminance(pixel: Rgba8888Pixel) -> f32 {
    0.299f32 * pixel.r as f32 + 0.587f32 * pixel.g as f32 + 0.114f32 * pixel.b as f32
}

/// Rounds half to even, in double precision.
#[inline]
fn round(value: f32) -> f64 {
    (value as f64).round_ties_even()
}

/// Scales an 8-bit channel to `max`, rounded half to even.
#[inline]
fn pack_channel(value: u8, max: f32) -> i32 {
    round(value as f32 / 255.0 * max) as i32
}

/// Three bytes per pixel: R, G, B.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct Rgb24PixelFormatWriter {
    address: usize,
}

impl IPixelFormatWriter for Rgb24PixelFormatWriter {
    fn write_next(&mut self, dest: &mut [u8], pixel: Rgba8888Pixel) {
        let addr = self.address;

        dest[addr] = pixel.r;
        dest[addr + 1] = pixel.g;
        dest[addr + 2] = pixel.b;

        self.address += 3;
    }

    fn reset(&mut self, address: usize) {
        self.address = address;
    }
}

/// Four bytes per pixel: R, G, B, 255.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct Rgb32PixelFormatWriter {
    address: usize,
}

impl IPixelFormatWriter for Rgb32PixelFormatWriter {
    fn write_next(&mut self, dest: &mut [u8], pixel: Rgba8888Pixel) {
        let address = self.address;

        dest[address] = pixel.r;
        dest[address + 1] = pixel.g;
        dest[address + 2] = pixel.b;
        dest[address + 3] = 255;

        self.address += 4;
    }

    fn reset(&mut self, address: usize) {
        self.address = address;
    }
}

/// Four 16-bit channels (native byte order); the low bytes are zero.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct Rgba64PixelFormatWriter {
    address: usize,
}

impl IPixelFormatWriter for Rgba64PixelFormatWriter {
    fn write_next(&mut self, dest: &mut [u8], pixel: Rgba8888Pixel) {
        let addr = self.address;

        let value = Rgba64Pixel::new(
            (pixel.r as u16) << 8,
            (pixel.g as u16) << 8,
            (pixel.b as u16) << 8,
            (pixel.a as u16) << 8,
        );

        write_u16(dest, addr, value.r);
        write_u16(dest, addr + 2, value.g);
        write_u16(dest, addr + 4, value.b);
        write_u16(dest, addr + 6, value.a);

        self.address += Rgba64Pixel::SIZE;
    }

    fn reset(&mut self, address: usize) {
        self.address = address;
    }
}

/// Four bytes per pixel: R, G, B, A.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct Rgba8888PixelFormatWriter {
    address: usize,
}

impl IPixelFormatWriter for Rgba8888PixelFormatWriter {
    fn write_next(&mut self, dest: &mut [u8], pixel: Rgba8888Pixel) {
        let addr = self.address;

        dest[addr..addr + 4].copy_from_slice(&[pixel.r, pixel.g, pixel.b, pixel.a]);

        self.address += Rgba8888Pixel::SIZE;
    }

    fn reset(&mut self, address: usize) {
        self.address = address;
    }
}

/// Four bytes per pixel: B, G, R, A.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct Bgra8888PixelFormatWriter {
    address: usize,
}

impl IPixelFormatWriter for Bgra8888PixelFormatWriter {
    fn write_next(&mut self, dest: &mut [u8], pixel: Rgba8888Pixel) {
        let addr = self.address;

        dest[addr] = pixel.b;
        dest[addr + 1] = pixel.g;
        dest[addr + 2] = pixel.r;
        dest[addr + 3] = pixel.a;

        self.address += 4;
    }

    fn reset(&mut self, address: usize) {
        self.address = address;
    }
}

/// Three bytes per pixel: B, G, R.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct Bgr24PixelFormatWriter {
    address: usize,
}

impl IPixelFormatWriter for Bgr24PixelFormatWriter {
    fn write_next(&mut self, dest: &mut [u8], pixel: Rgba8888Pixel) {
        let addr = self.address;

        dest[addr + 2] = pixel.r;
        dest[addr + 1] = pixel.g;
        dest[addr] = pixel.b;

        self.address += 3;
    }

    fn reset(&mut self, address: usize) {
        self.address = address;
    }
}

/// Four bytes per pixel: B, G, R, 255.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct Bgr32PixelFormatWriter {
    address: usize,
}

impl IPixelFormatWriter for Bgr32PixelFormatWriter {
    fn write_next(&mut self, dest: &mut [u8], pixel: Rgba8888Pixel) {
        let address = self.address;

        dest[address] = pixel.b;
        dest[address + 1] = pixel.g;
        dest[address + 2] = pixel.r;
        dest[address + 3] = 255;

        self.address += 4;
    }

    fn reset(&mut self, address: usize) {
        self.address = address;
    }
}

/// Four bytes per pixel: B, G, R, A.
// Not selected by any pixel format, as upstream.
#[allow(dead_code)]
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct Bgra32PixelFormatWriter {
    address: usize,
}

impl IPixelFormatWriter for Bgra32PixelFormatWriter {
    fn write_next(&mut self, dest: &mut [u8], pixel: Rgba8888Pixel) {
        let addr = self.address;

        dest[addr + 3] = pixel.a;
        dest[addr + 2] = pixel.r;
        dest[addr + 1] = pixel.g;
        dest[addr] = pixel.b;

        self.address += 4;
    }

    fn reset(&mut self, address: usize) {
        self.address = address;
    }
}

/// 16 bits per pixel (native byte order): 5 bits R, 6 bits G, 5 bits B from the top.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct Bgr565PixelFormatWriter {
    address: usize,
}

impl Bgr565PixelFormatWriter {
    fn pack(pixel: Rgba8888Pixel) -> u16 {
        (((pack_channel(pixel.r, 31.0) & 0x1F) << 11)
            | ((pack_channel(pixel.g, 63.0) & 0x3F) << 5)
            | (pack_channel(pixel.b, 31.0) & 0x1F)) as u16
    }
}

impl IPixelFormatWriter for Bgr565PixelFormatWriter {
    fn write_next(&mut self, dest: &mut [u8], pixel: Rgba8888Pixel) {
        write_u16(dest, self.address, Self::pack(pixel));

        self.address += 2;
    }

    fn reset(&mut self, address: usize) {
        self.address = address;
    }
}

/// 16 bits per pixel (native byte order): 5 bits each of R, G, B from the top.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct Bgr555PixelFormatWriter {
    address: usize,
}

impl Bgr555PixelFormatWriter {
    fn pack(pixel: Rgba8888Pixel) -> u16 {
        (((pack_channel(pixel.r, 31.0) & 0x1F) << 10)
            | ((pack_channel(pixel.g, 31.0) & 0x1F) << 5)
            | (pack_channel(pixel.b, 31.0) & 0x1F)) as u16
    }
}

impl IPixelFormatWriter for Bgr555PixelFormatWriter {
    fn write_next(&mut self, dest: &mut [u8], pixel: Rgba8888Pixel) {
        write_u16(dest, self.address, Self::pack(pixel));

        self.address += 2;
    }

    fn reset(&mut self, address: usize) {
        self.address = address;
    }
}

/// One linear 32-bit float per pixel, taken from the red channel with a 2.2 gamma.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct Gray32FloatPixelFormatWriter {
    address: usize,
}

impl Gray32FloatPixelFormatWriter {
    fn pack(pixel: Rgba8888Pixel) -> f32 {
        ((pixel.r as f32 / 255.0) as f64).powf(2.2) as f32
    }
}

impl IPixelFormatWriter for Gray32FloatPixelFormatWriter {
    fn write_next(&mut self, dest: &mut [u8], pixel: Rgba8888Pixel) {
        let addr = self.address;

        dest[addr..addr + 4].copy_from_slice(&Self::pack(pixel).to_ne_bytes());

        self.address += 4;
    }

    fn reset(&mut self, address: usize) {
        self.address = address;
    }
}

/// 1 bit per pixel, most significant bit first.
///
/// As in the reference implementation, `reset` moves the address but keeps the
/// bit position.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct BlackWhitePixelFormatWriter {
    bit: i32,
    address: usize,
}

impl IPixelFormatWriter for BlackWhitePixelFormatWriter {
    fn write_next(&mut self, dest: &mut [u8], pixel: Rgba8888Pixel) {
        let addr = self.address;

        let grayscale = round(luminance(pixel));

        let value: i32 = if grayscale > 0x7F as f64 { 1 } else { 0 };

        let shift = 7 - self.bit;
        let mask = 1 << shift;

        dest[addr] = ((dest[addr] as i32 & !mask) | (value << shift)) as u8;

        self.bit += 1;

        if self.bit == 8 {
            self.address += 1;

            self.bit = 0;
        }
    }

    fn reset(&mut self, address: usize) {
        self.address = address;
    }
}

/// 2 bits per pixel, most significant bits first.
///
/// As in the reference implementation, `reset` moves the address but keeps the
/// bit position.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct Gray2PixelFormatWriter {
    bit: i32,
    address: usize,
}

impl IPixelFormatWriter for Gray2PixelFormatWriter {
    fn write_next(&mut self, dest: &mut [u8], pixel: Rgba8888Pixel) {
        let addr = self.address;
        let mut value = 0;

        let grayscale = round(luminance(pixel)) as u8;

        if grayscale > 0 && grayscale <= 0x55 {
            //01
            value = 1;
        }

        if grayscale > 0x55 && grayscale <= 0xAA {
            //10
            value = 2;
        }

        if grayscale > 0xAA {
            //11
            value = 3;
        }

        let shift = 6 - self.bit;
        let mask = 3 << shift;

        dest[addr] = ((dest[addr] as i32 & !mask) | (value << shift)) as u8;

        self.bit += 2;

        if self.bit == 8 {
            self.address += 1;
            self.bit = 0;
        }
    }

    fn reset(&mut self, address: usize) {
        self.address = address;
    }
}

/// 4 bits per pixel, high nibble first.
///
/// As in the reference implementation, `reset` moves the address but keeps the
/// bit position.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct Gray4PixelFormatWriter {
    bit: i32,
    address: usize,
}

impl IPixelFormatWriter for Gray4PixelFormatWriter {
    fn write_next(&mut self, dest: &mut [u8], pixel: Rgba8888Pixel) {
        let addr = self.address;

        let grayscale = round(luminance(pixel)) as u8;

        let value = (grayscale as f32 / 255.0 * 0xF as f32) as u8 as i32;

        let shift = 4 - self.bit;
        let mask = 0xF << shift;

        dest[addr] = ((dest[addr] as i32 & !mask) | (value << shift)) as u8;

        self.bit += 4;

        if self.bit == 8 {
            self.address += 1;
            self.bit = 0;
        }
    }

    fn reset(&mut self, address: usize) {
        self.address = address;
    }
}

/// 8 bits per pixel.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct Gray8PixelFormatWriter {
    address: usize,
}

impl IPixelFormatWriter for Gray8PixelFormatWriter {
    fn write_next(&mut self, dest: &mut [u8], pixel: Rgba8888Pixel) {
        let addr = self.address;

        let grayscale = round(luminance(pixel)) as u8;

        dest[addr] = grayscale;

        self.address += 1;
    }

    fn reset(&mut self, address: usize) {
        self.address = address;
    }
}

/// 16 bits per pixel (native byte order).
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct Gray16PixelFormatWriter {
    address: usize,
}

impl IPixelFormatWriter for Gray16PixelFormatWriter {
    fn write_next(&mut self, dest: &mut [u8], pixel: Rgba8888Pixel) {
        let addr = self.address;

        let grayscale = round(luminance(pixel) * 0x0101 as f32) as u16;

        write_u16(dest, addr, grayscale);

        self.address += 2;
    }

    fn reset(&mut self, address: usize) {
        self.address = address;
    }
}

/// Encodes whole bitmaps into any supported pixel format.
pub(crate) struct PixelFormatWriter;

impl PixelFormatWriter {
    fn write_with<T: IPixelFormatWriter>(
        pixels: &[Rgba8888Pixel],
        dest: &mut [u8],
        size: PixelSize,
        stride: i32,
        alpha_format: AlphaFormat,
        src_alpha_format: AlphaFormat,
    ) {
        let mut writer = T::default();

        let w = size.width;
        let h = size.height;
        let mut count = 0;

        for y in 0..h {
            writer.reset(row_address(stride, y));

            for _ in 0..w {
                writer.write_next(dest, Self::get_converted_pixel(pixels[count], src_alpha_format, alpha_format));
                count += 1;
            }
        }
    }

    fn get_converted_pixel(pixel: Rgba8888Pixel, source_alpha: AlphaFormat, dest_alpha: AlphaFormat) -> Rgba8888Pixel {
        if source_alpha != dest_alpha {
            if source_alpha == AlphaFormat::Premul && dest_alpha != AlphaFormat::Premul {
                return Self::convert_from_premultiplied(pixel);
            }

            if source_alpha != AlphaFormat::Premul && dest_alpha == AlphaFormat::Premul {
                return Self::convert_to_premultiplied(pixel);
            }
        }

        pixel
    }

    fn convert_to_premultiplied(pixel: Rgba8888Pixel) -> Rgba8888Pixel {
        let factor = pixel.a as f32 / 255.0;

        Rgba8888Pixel {
            r: (pixel.r as f32 * factor) as u8,
            g: (pixel.g as f32 * factor) as u8,
            b: (pixel.b as f32 * factor) as u8,
            a: pixel.a,
        }
    }

    fn convert_from_premultiplied(pixel: Rgba8888Pixel) -> Rgba8888Pixel {
        let factor = 1.0f32 / (pixel.a as f32 / 255.0);

        Rgba8888Pixel {
            r: (pixel.r as f32 * factor) as u8,
            g: (pixel.g as f32 * factor) as u8,
            b: (pixel.b as f32 * factor) as u8,
            a: pixel.a,
        }
    }

    /// Encodes `size` pixels from `pixels` (row by row, alpha stored as
    /// `src_alpha_format`) into `dest` as `format` / `alpha_format`, rows
    /// `stride` bytes apart.
    ///
    /// Panics when `dest` or `pixels` is too small.
    pub fn write(
        pixels: &[Rgba8888Pixel],
        dest: &mut [u8],
        size: PixelSize,
        stride: i32,
        format: PixelFormat,
        alpha_format: AlphaFormat,
        src_alpha_format: AlphaFormat,
    ) {
        check_buffer_length("destination", dest.len(), size, stride, format);

        macro_rules! write_with {
            ($writer:ty) => {
                Self::write_with::<$writer>(pixels, dest, size, stride, alpha_format, src_alpha_format)
            };
        }

        match format {
            PixelFormats::RGB565 => write_with!(Bgr565PixelFormatWriter),
            PixelFormats::RGBA8888 => write_with!(Rgba8888PixelFormatWriter),
            PixelFormats::BGRA8888 => write_with!(Bgra8888PixelFormatWriter),
            PixelFormats::BLACK_WHITE => write_with!(BlackWhitePixelFormatWriter),
            PixelFormats::GRAY2 => write_with!(Gray2PixelFormatWriter),
            PixelFormats::GRAY4 => write_with!(Gray4PixelFormatWriter),
            PixelFormats::GRAY8 => write_with!(Gray8PixelFormatWriter),
            PixelFormats::GRAY16 => write_with!(Gray16PixelFormatWriter),
            PixelFormats::GRAY32_FLOAT => write_with!(Gray32FloatPixelFormatWriter),
            PixelFormats::RGBA64 => write_with!(Rgba64PixelFormatWriter),
            PixelFormats::RGB24 => write_with!(Rgb24PixelFormatWriter),
            PixelFormats::RGB32 => write_with!(Rgb32PixelFormatWriter),
            PixelFormats::BGR24 => write_with!(Bgr24PixelFormatWriter),
            PixelFormats::BGR32 => write_with!(Bgr32PixelFormatWriter),
            PixelFormats::BGR555 => write_with!(Bgr555PixelFormatWriter),
            PixelFormats::BGR565 => write_with!(Bgr565PixelFormatWriter),
            #[allow(unreachable_patterns)]
            _ => panic!("Pixel format {format} is not supported"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::bitmap_memory::BitmapMemory;
    use super::super::pixel_format_readers::*;
    use super::*;

    const WHITE: Rgba8888Pixel = Rgba8888Pixel { a: 255, b: 255, g: 255, r: 255 };

    const BLACK: Rgba8888Pixel = Rgba8888Pixel { a: 255, b: 0, g: 0, r: 0 };

    const ZERO: Rgba8888Pixel = Rgba8888Pixel { r: 0, g: 0, b: 0, a: 0 };

    /// Writes `pixel` with `writer` and reads it back with `reader`.
    fn round_trip<W: IPixelFormatWriter, R: IPixelFormatReader>(
        memory: &mut BitmapMemory,
        writer: &mut W,
        reader: &mut R,
        pixel: Rgba8888Pixel,
    ) -> Rgba8888Pixel {
        writer.write_next(memory.data_mut(), pixel);
        reader.read_next(memory.data())
    }

    #[test]
    fn should_write_bgr555() {
        let mut bitmap_memory = BitmapMemory::new(PixelFormats::BGR555, AlphaFormat::Unpremul, PixelSize::new(10, 10));

        let mut pixel_writer = Bgr555PixelFormatWriter::default();
        let mut pixel_reader = Bgr555PixelFormatReader::default();

        pixel_writer.reset(0);
        pixel_reader.reset(0);

        let m = &mut bitmap_memory;
        let (w, r) = (&mut pixel_writer, &mut pixel_reader);

        assert_eq!(Rgba8888Pixel { r: 255, a: 255, ..ZERO }, round_trip(m, w, r, Rgba8888Pixel { r: 255, ..ZERO }));
        assert_eq!(Rgba8888Pixel { g: 255, a: 255, ..ZERO }, round_trip(m, w, r, Rgba8888Pixel { g: 255, ..ZERO }));
        assert_eq!(Rgba8888Pixel { b: 255, a: 255, ..ZERO }, round_trip(m, w, r, Rgba8888Pixel { b: 255, ..ZERO }));
    }

    #[test]
    fn should_write_bgra8888() {
        let mut source_memory = BitmapMemory::new(PixelFormats::BGRA8888, AlphaFormat::Unpremul, PixelSize::new(3, 1));

        let mut source_writer = Bgra8888PixelFormatWriter::default();
        let mut source_reader = Bgra8888PixelFormatReader::default();

        source_writer.reset(0);
        source_reader.reset(0);

        let m = &mut source_memory;
        let (w, r) = (&mut source_writer, &mut source_reader);

        assert_eq!(Rgba8888Pixel { r: 255, ..ZERO }, round_trip(m, w, r, Rgba8888Pixel { r: 255, ..ZERO }));
        assert_eq!(Rgba8888Pixel { g: 255, ..ZERO }, round_trip(m, w, r, Rgba8888Pixel { g: 255, ..ZERO }));
        assert_eq!(Rgba8888Pixel { b: 255, ..ZERO }, round_trip(m, w, r, Rgba8888Pixel { b: 255, ..ZERO }));
    }

    #[test]
    fn should_write_rgba8888() {
        let mut source_memory = BitmapMemory::new(PixelFormats::RGBA8888, AlphaFormat::Unpremul, PixelSize::new(10, 10));

        let mut pixel_writer = Rgba8888PixelFormatWriter::default();
        let mut pixel_reader = Rgba8888PixelFormatReader::default();

        pixel_writer.reset(0);
        pixel_reader.reset(0);

        let m = &mut source_memory;
        let (w, r) = (&mut pixel_writer, &mut pixel_reader);

        let pixel = Rgba8888Pixel { r: 255, g: 125, b: 125, a: 125 };
        assert_eq!(pixel, round_trip(m, w, r, pixel));

        let pixel = Rgba8888Pixel { r: 125, g: 255, b: 125, a: 125 };
        assert_eq!(pixel, round_trip(m, w, r, pixel));

        let pixel = Rgba8888Pixel { r: 125, g: 125, b: 255, a: 125 };
        assert_eq!(pixel, round_trip(m, w, r, pixel));
    }

    #[test]
    fn should_write_rgb24() {
        let mut source_memory = BitmapMemory::new(PixelFormats::RGB24, AlphaFormat::Unpremul, PixelSize::new(10, 10));

        let mut pixel_writer = Rgb24PixelFormatWriter::default();
        let mut pixel_reader = Rgb24PixelFormatReader::default();

        pixel_writer.reset(0);
        pixel_reader.reset(0);

        let m = &mut source_memory;
        let (w, r) = (&mut pixel_writer, &mut pixel_reader);

        assert_eq!(
            Rgba8888Pixel { r: 255, g: 125, b: 125, a: 255 },
            round_trip(m, w, r, Rgba8888Pixel { r: 255, g: 125, b: 125, a: 0 })
        );
        assert_eq!(
            Rgba8888Pixel { r: 125, g: 255, b: 125, a: 255 },
            round_trip(m, w, r, Rgba8888Pixel { r: 125, g: 255, b: 125, a: 0 })
        );
        assert_eq!(
            Rgba8888Pixel { r: 125, g: 125, b: 255, a: 255 },
            round_trip(m, w, r, Rgba8888Pixel { r: 125, g: 125, b: 255, a: 0 })
        );
    }

    #[test]
    fn should_write_rgba64() {
        let mut source_memory = BitmapMemory::new(PixelFormats::RGBA64, AlphaFormat::Unpremul, PixelSize::new(10, 10));

        let mut pixel_writer = Rgba64PixelFormatWriter::default();
        let mut pixel_reader = Rgba64PixelFormatReader::default();

        pixel_writer.reset(0);
        pixel_reader.reset(0);

        let m = &mut source_memory;
        let (w, r) = (&mut pixel_writer, &mut pixel_reader);

        let pixel = Rgba8888Pixel { r: 255, g: 125, b: 125, a: 125 };
        assert_eq!(pixel, round_trip(m, w, r, pixel));

        let pixel = Rgba8888Pixel { r: 125, g: 255, b: 125, a: 125 };
        assert_eq!(pixel, round_trip(m, w, r, pixel));

        let pixel = Rgba8888Pixel { r: 125, g: 125, b: 255, a: 125 };
        assert_eq!(pixel, round_trip(m, w, r, pixel));
    }

    #[test]
    fn should_write_bgr565() {
        let mut bitmap_memory = BitmapMemory::new(PixelFormats::BGR565, AlphaFormat::Unpremul, PixelSize::new(10, 10));

        let mut pixel_writer = Bgr565PixelFormatWriter::default();
        let mut pixel_reader = Bgr565PixelFormatReader::default();

        pixel_writer.reset(0);
        pixel_reader.reset(0);

        let m = &mut bitmap_memory;
        let (w, r) = (&mut pixel_writer, &mut pixel_reader);

        assert_eq!(Rgba8888Pixel { r: 255, a: 255, ..ZERO }, round_trip(m, w, r, Rgba8888Pixel { r: 255, ..ZERO }));
        assert_eq!(Rgba8888Pixel { g: 255, a: 255, ..ZERO }, round_trip(m, w, r, Rgba8888Pixel { g: 255, ..ZERO }));
        assert_eq!(Rgba8888Pixel { b: 255, a: 255, ..ZERO }, round_trip(m, w, r, Rgba8888Pixel { b: 255, ..ZERO }));
    }

    #[test]
    fn should_write_gray32_float() {
        let mut bitmap_memory =
            BitmapMemory::new(PixelFormats::GRAY32_FLOAT, AlphaFormat::Unpremul, PixelSize::new(10, 10));

        let mut pixel_writer = Gray32FloatPixelFormatWriter::default();
        let mut pixel_reader = Gray32FloatPixelFormatReader::default();

        pixel_writer.reset(0);
        pixel_reader.reset(0);

        let m = &mut bitmap_memory;
        let (w, r) = (&mut pixel_writer, &mut pixel_reader);

        assert_eq!(
            Rgba8888Pixel { r: 255, g: 255, b: 255, a: 255 },
            round_trip(m, w, r, Rgba8888Pixel { r: 255, ..ZERO })
        );
        assert_eq!(
            Rgba8888Pixel { r: 125, g: 125, b: 125, a: 255 },
            round_trip(m, w, r, Rgba8888Pixel { r: 125, ..ZERO })
        );
        assert_eq!(Rgba8888Pixel { a: 255, ..ZERO }, round_trip(m, w, r, ZERO));
    }

    #[test]
    fn should_write_black_white() {
        let mut bitmap_memory =
            BitmapMemory::new(PixelFormats::BLACK_WHITE, AlphaFormat::Unpremul, PixelSize::new(10, 10));

        let mut pixel_writer = BlackWhitePixelFormatWriter::default();
        let mut pixel_reader = BlackWhitePixelFormatReader::default();

        pixel_writer.reset(0);
        pixel_reader.reset(0);

        let m = &mut bitmap_memory;
        let (w, r) = (&mut pixel_writer, &mut pixel_reader);

        assert_eq!(WHITE, round_trip(m, w, r, WHITE));
        assert_eq!(BLACK, round_trip(m, w, r, BLACK));
    }

    #[test]
    fn should_write_gray2() {
        let palette = [
            BLACK,
            Rgba8888Pixel { a: 255, b: 0x55, g: 0x55, r: 0x55 },
            Rgba8888Pixel { a: 255, b: 0xAA, g: 0xAA, r: 0xAA },
            WHITE,
        ];

        let mut bitmap_memory = BitmapMemory::new(PixelFormats::GRAY2, AlphaFormat::Unpremul, PixelSize::new(10, 10));

        let mut pixel_writer = Gray2PixelFormatWriter::default();
        let mut pixel_reader = Gray2PixelFormatReader::default();

        pixel_writer.reset(0);
        pixel_reader.reset(0);

        let m = &mut bitmap_memory;
        let (w, r) = (&mut pixel_writer, &mut pixel_reader);

        assert_eq!(palette[0], round_trip(m, w, r, palette[0]));
        assert_eq!(palette[1], round_trip(m, w, r, palette[1]));
        assert_eq!(palette[2], round_trip(m, w, r, palette[2]));
        assert_eq!(palette[3], round_trip(m, w, r, palette[3]));
    }

    #[test]
    fn should_write_gray4() {
        let mut bitmap_memory = BitmapMemory::new(PixelFormats::GRAY4, AlphaFormat::Unpremul, PixelSize::new(10, 10));

        let mut pixel_writer = Gray4PixelFormatWriter::default();
        let mut pixel_reader = Gray4PixelFormatReader::default();

        pixel_writer.reset(0);
        pixel_reader.reset(0);

        let m = &mut bitmap_memory;
        let (w, r) = (&mut pixel_writer, &mut pixel_reader);

        let pixel = Rgba8888Pixel { r: 255, ..ZERO };
        assert_eq!(get_gray4(pixel), round_trip(m, w, r, pixel));

        let pixel = Rgba8888Pixel { r: 17, ..ZERO };
        assert_eq!(get_gray4(pixel), round_trip(m, w, r, pixel));

        assert_eq!(Rgba8888Pixel { a: 255, ..ZERO }, round_trip(m, w, r, ZERO));
    }

    fn get_gray4(pixel: Rgba8888Pixel) -> Rgba8888Pixel {
        let grayscale = round(luminance(pixel)) as u8;

        let mut value = (grayscale as f32 / 255.0 * 0xF as f32) as u8;

        value |= value << 4;

        Rgba8888Pixel::new(value, value, value, 255)
    }

    #[test]
    fn should_write_gray8() {
        let mut bitmap_memory = BitmapMemory::new(PixelFormats::GRAY8, AlphaFormat::Unpremul, PixelSize::new(10, 10));

        let mut pixel_writer = Gray8PixelFormatWriter::default();
        let mut pixel_reader = Gray8PixelFormatReader::default();

        pixel_writer.reset(0);
        pixel_reader.reset(0);

        let m = &mut bitmap_memory;
        let (w, r) = (&mut pixel_writer, &mut pixel_reader);

        let pixel = Rgba8888Pixel { r: 255, ..ZERO };
        assert_eq!(get_gray8(pixel), round_trip(m, w, r, pixel));

        let pixel = Rgba8888Pixel { r: 120, ..ZERO };
        assert_eq!(get_gray8(pixel), round_trip(m, w, r, pixel));

        assert_eq!(get_gray8(Rgba8888Pixel { a: 255, ..ZERO }), round_trip(m, w, r, ZERO));
    }

    fn get_gray8(pixel: Rgba8888Pixel) -> Rgba8888Pixel {
        let value = round(luminance(pixel)) as u8;

        Rgba8888Pixel::new(value, value, value, 255)
    }

    #[test]
    fn should_write_gray16() {
        let mut bitmap_memory = BitmapMemory::new(PixelFormats::GRAY16, AlphaFormat::Unpremul, PixelSize::new(10, 10));

        let mut pixel_writer = Gray16PixelFormatWriter::default();
        let mut pixel_reader = Gray16PixelFormatReader::default();

        pixel_writer.reset(0);
        pixel_reader.reset(0);

        let m = &mut bitmap_memory;
        let (w, r) = (&mut pixel_writer, &mut pixel_reader);

        let pixel = Rgba8888Pixel { r: 255, ..ZERO };
        assert_eq!(get_gray16(pixel), round_trip(m, w, r, pixel));

        let pixel = Rgba8888Pixel { r: 120, ..ZERO };
        assert_eq!(get_gray16(pixel), round_trip(m, w, r, pixel));

        assert_eq!(get_gray16(Rgba8888Pixel { a: 255, ..ZERO }), round_trip(m, w, r, ZERO));
    }

    fn get_gray16(pixel: Rgba8888Pixel) -> Rgba8888Pixel {
        let grayscale = round(luminance(pixel) * 0x0101 as f32) as u16;

        let value = (grayscale >> 8) as u8;

        Rgba8888Pixel::new(value, value, value, 255)
    }

    // Additional coverage (not part of the reference suite).

    #[test]
    fn writes_exact_bytes() {
        let pixel = Rgba8888Pixel::new(1, 2, 3, 4);
        let mut dest = [9u8; 8];

        Rgb24PixelFormatWriter::default().write_next(&mut dest, pixel);
        assert_eq!(dest[..4], [1, 2, 3, 9]);
        Bgr24PixelFormatWriter::default().write_next(&mut dest, pixel);
        assert_eq!(dest[..4], [3, 2, 1, 9]);
        Rgb32PixelFormatWriter::default().write_next(&mut dest, pixel);
        assert_eq!(dest[..5], [1, 2, 3, 255, 9]);
        Bgr32PixelFormatWriter::default().write_next(&mut dest, pixel);
        assert_eq!(dest[..5], [3, 2, 1, 255, 9]);
        Bgra32PixelFormatWriter::default().write_next(&mut dest, pixel);
        assert_eq!(dest[..5], [3, 2, 1, 4, 9]);
        Bgra8888PixelFormatWriter::default().write_next(&mut dest, pixel);
        assert_eq!(dest[..5], [3, 2, 1, 4, 9]);
        Rgba8888PixelFormatWriter::default().write_next(&mut dest, pixel);
        assert_eq!(dest[..5], [1, 2, 3, 4, 9]);

        Rgba64PixelFormatWriter::default().write_next(&mut dest, pixel);
        let channels: Vec<u16> = dest.chunks(2).map(|c| u16::from_ne_bytes([c[0], c[1]])).collect();
        assert_eq!(channels, [0x0100, 0x0200, 0x0300, 0x0400]);

        Bgr565PixelFormatWriter::default().write_next(&mut dest, Rgba8888Pixel::new(255, 0, 255, 0));
        assert_eq!(u16::from_ne_bytes([dest[0], dest[1]]), 0xF81F);
        Bgr555PixelFormatWriter::default().write_next(&mut dest, Rgba8888Pixel::new(255, 0, 255, 0));
        assert_eq!(u16::from_ne_bytes([dest[0], dest[1]]), 0x7C1F);
    }

    #[test]
    fn packed_bits_keep_neighbours() {
        let mut dest = [0xFFu8; 2];
        let mut writer = BlackWhitePixelFormatWriter::default();
        for pixel in [BLACK, WHITE, BLACK, BLACK, WHITE, WHITE, WHITE, BLACK, BLACK] {
            writer.write_next(&mut dest, pixel);
        }
        assert_eq!(dest, [0b0100_1110, 0b0111_1111]);

        let mut dest = [0xFFu8; 1];
        let mut writer = Gray2PixelFormatWriter::default();
        writer.write_next(&mut dest, BLACK);
        writer.write_next(&mut dest, Rgba8888Pixel::new(0x60, 0x60, 0x60, 255));
        assert_eq!(dest, [0b00_10_11_11]);

        let mut dest = [0xFFu8; 1];
        let mut writer = Gray4PixelFormatWriter::default();
        writer.write_next(&mut dest, Rgba8888Pixel::new(0x88, 0x88, 0x88, 255));
        assert_eq!(dest, [0x8F]);
    }

    #[test]
    fn reset_keeps_bit_position_of_packed_writers() {
        // Width 3 at 1 bit per pixel: the second row starts at bit 3 of its
        // first byte, exactly like the reference implementation.
        let pixels = [WHITE; 6];
        let mut dest = [0u8; 8];
        PixelFormatWriter::write(
            &pixels,
            &mut dest,
            PixelSize::new(3, 2),
            4,
            PixelFormats::BLACK_WHITE,
            AlphaFormat::Opaque,
            AlphaFormat::Opaque,
        );
        assert_eq!(dest, [0b1110_0000, 0, 0, 0, 0b0001_1100, 0, 0, 0]);
    }

    #[test]
    fn converts_alpha() {
        let pixels = [Rgba8888Pixel::new(255, 128, 0, 128)];
        let mut dest = [0u8; 4];
        let size = PixelSize::new(1, 1);

        PixelFormatWriter::write(
            &pixels,
            &mut dest,
            size,
            4,
            PixelFormats::RGBA8888,
            AlphaFormat::Premul,
            AlphaFormat::Unpremul,
        );
        // 255 * (128 / 255) = 128, 128 * (128 / 255) = 64.25 -> 64 (truncated)
        assert_eq!(dest, [128, 64, 0, 128]);

        PixelFormatWriter::write(
            &pixels,
            &mut dest,
            size,
            4,
            PixelFormats::RGBA8888,
            AlphaFormat::Premul,
            AlphaFormat::Opaque,
        );
        assert_eq!(dest, [128, 64, 0, 128]);

        let premultiplied = [Rgba8888Pixel::new(128, 64, 0, 128)];
        PixelFormatWriter::write(
            &premultiplied,
            &mut dest,
            size,
            4,
            PixelFormats::BGRA8888,
            AlphaFormat::Unpremul,
            AlphaFormat::Premul,
        );
        // Single precision: 64 * (1 / (128 / 255)) = 127.49999 -> 127 and
        // 128 * (1 / (128 / 255)) = 254.99998 -> 254 (truncated).
        assert_eq!(dest, [0, 127, 254, 128]);

        // Opaque <-> Unpremul and equal formats leave the pixel alone.
        for (dest_alpha, src_alpha) in [
            (AlphaFormat::Opaque, AlphaFormat::Unpremul),
            (AlphaFormat::Unpremul, AlphaFormat::Opaque),
            (AlphaFormat::Premul, AlphaFormat::Premul),
        ] {
            PixelFormatWriter::write(&pixels, &mut dest, size, 4, PixelFormats::RGBA8888, dest_alpha, src_alpha);
            assert_eq!(dest, [255, 128, 0, 128]);
        }
    }

    #[test]
    fn round_trips_every_format_through_dispatch() {
        // Eight pixels per row keep the packed formats byte aligned (see
        // `reset_keeps_bit_position_of_packed_writers`).
        let mut pixels = [BLACK; 16];
        for i in [0, 3, 5, 6, 9, 15] {
            pixels[i] = WHITE;
        }
        let size = PixelSize::new(8, 2);

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
            let mut memory = BitmapMemory::new(format, AlphaFormat::Opaque, size);
            let row_bytes = memory.row_bytes();
            PixelFormatWriter::write(
                &pixels,
                memory.data_mut(),
                size,
                row_bytes,
                format,
                AlphaFormat::Opaque,
                AlphaFormat::Opaque,
            );

            let mut read = [ZERO; 16];
            PixelFormatReader::read(&mut read, memory.data(), size, row_bytes, format);
            assert_eq!(pixels, read, "{format}");
        }
    }

    #[test]
    #[should_panic(expected = "destination buffer too small")]
    fn short_destination_panics() {
        PixelFormatWriter::write(
            &[WHITE; 4],
            &mut [0u8; 15],
            PixelSize::new(2, 2),
            8,
            PixelFormats::RGBA8888,
            AlphaFormat::Opaque,
            AlphaFormat::Opaque,
        );
    }
}
