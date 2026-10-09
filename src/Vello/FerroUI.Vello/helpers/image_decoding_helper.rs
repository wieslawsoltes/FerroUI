//! Decoding of encoded images.
//!
//! The Skia backend decodes with the codecs of Skia, which in the build of
//! this workspace are PNG, JPEG, GIF, BMP, ICO and WBMP. Here a format is
//! recognised by its first bytes and decoded by a crate of its own: PNG by
//! `png`, JPEG by `zune-jpeg`, GIF by `gif` (the first frame, as the codec
//! of Skia gives it), BMP by `zune-bmp`, and ICO and WBMP by this file: of
//! an icon it reads the directory and hands the largest image to the PNG or
//! the BMP decoder, and a wireless bitmap is a header and one bit a pixel.
//! WebP, which Skia decodes when it is built with it (this workspace does
//! not build it so), is not decoded: data in it fails to load, as data in
//! no format does.
//!
//! Every decoder gives premultiplied RGBA pixels without padding, and says
//! whether the image has no alpha, as the codec of Skia does. No decoder
//! reports a resolution: a decoded bitmap has 96 DPI, as in the Skia
//! backend.

use std::io::{self, Read};
use zune_core::bytestream::ZCursor;
use zune_core::colorspace::ColorSpace;
use zune_core::options::DecoderOptions;

/// A decoded image.
pub struct DecodedImage {
    /// Premultiplied RGBA pixels without padding.
    pub rgba: Vec<u8>,
    pub width: u32,
    pub height: u32,
    /// Whether the encoded image has no alpha: every pixel is opaque.
    pub opaque: bool,
    /// The format the image was encoded in.
    pub format: EncodedImageFormat,
}

/// The pixels of an image as a decoder gives them: premultiplied RGBA,
/// width, height, and whether the image has no alpha.
type Decoded = (Vec<u8>, u32, u32, bool);

pub(crate) fn load_error() -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, "Unable to load bitmap from provided data")
}

/// The formats the backend decodes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EncodedImageFormat {
    Png,
    Jpeg,
    Gif,
    Bmp,
    Ico,
    Wbmp,
}

const PNG_SIGNATURE: [u8; 8] = [0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a];

/// The format of encoded data by its first bytes, or `None` for data in a
/// format the backend does not decode.
pub fn encoded_format(bytes: &[u8]) -> Option<EncodedImageFormat> {
    if bytes.starts_with(&PNG_SIGNATURE) {
        Some(EncodedImageFormat::Png)
    } else if bytes.starts_with(&[0xff, 0xd8, 0xff]) {
        Some(EncodedImageFormat::Jpeg)
    } else if bytes.starts_with(b"GIF87a") || bytes.starts_with(b"GIF89a") {
        Some(EncodedImageFormat::Gif)
    } else if bytes.starts_with(b"BM") {
        Some(EncodedImageFormat::Bmp)
    } else if bytes.starts_with(&[0, 0, 1, 0]) || bytes.starts_with(&[0, 0, 2, 0]) {
        // An icon or a cursor: the same directory.
        Some(EncodedImageFormat::Ico)
    } else if wbmp_header(bytes).is_some() {
        // A wireless bitmap has no signature: it is the last format that
        // is tried, as in Skia.
        Some(EncodedImageFormat::Wbmp)
    } else {
        None
    }
}

/// Decodes an encoded image at its natural size into premultiplied RGBA
/// pixels without padding, with its width and height.
pub fn decode_image(stream: &mut dyn Read) -> io::Result<DecodedImage> {
    let mut bytes = Vec::new();
    stream.read_to_end(&mut bytes)?;

    decode_bytes(&bytes)
}

fn decode_bytes(bytes: &[u8]) -> io::Result<DecodedImage> {
    let format = encoded_format(bytes).ok_or_else(load_error)?;
    let (rgba, width, height, opaque) = match format {
        EncodedImageFormat::Png => decode_png(bytes),
        EncodedImageFormat::Jpeg => decode_jpeg(bytes),
        EncodedImageFormat::Gif => decode_gif(bytes),
        EncodedImageFormat::Bmp => decode_bmp(bytes),
        EncodedImageFormat::Ico => decode_ico(bytes),
        EncodedImageFormat::Wbmp => decode_wbmp(bytes),
    }?;

    if width == 0 || height == 0 || rgba.len() != width as usize * height as usize * 4 {
        return Err(load_error());
    }

    Ok(DecodedImage { rgba, width, height, opaque, format })
}

fn premultiply(channel: u8, alpha: u8) -> u8 {
    ((channel as u32 * alpha as u32 + 127) / 255) as u8
}

/// Pixels of `channels` bytes (gray, gray and alpha, RGB or RGBA, not
/// premultiplied) as premultiplied RGBA.
fn to_premul_rgba(pixels: &[u8], channels: usize) -> Vec<u8> {
    let mut rgba = Vec::with_capacity(pixels.len() / channels * 4);

    for p in pixels.chunks_exact(channels) {
        let (r, g, b, a) = match channels {
            1 => (p[0], p[0], p[0], 255),
            2 => (p[0], p[0], p[0], p[1]),
            3 => (p[0], p[1], p[2], 255),
            _ => (p[0], p[1], p[2], p[3]),
        };
        rgba.extend_from_slice(&[premultiply(r, a), premultiply(g, a), premultiply(b, a), a]);
    }

    rgba
}

fn decode_png(bytes: &[u8]) -> io::Result<Decoded> {
    let mut decoder = png::Decoder::new(io::Cursor::new(bytes));
    // Palettes, low bit depths and transparency chunks are expanded and
    // sixteen bits a channel are reduced to eight.
    decoder.set_transformations(png::Transformations::normalize_to_color8());

    let mut reader = decoder.read_info().map_err(|_| load_error())?;
    let buffer_size = reader.output_buffer_size().ok_or_else(load_error)?;
    let mut buffer = vec![0u8; buffer_size];
    let info = reader.next_frame(&mut buffer).map_err(|_| load_error())?;
    let decoded = &buffer[..info.buffer_size()];

    let channels = match info.color_type {
        png::ColorType::Grayscale => 1,
        png::ColorType::GrayscaleAlpha => 2,
        png::ColorType::Rgb => 3,
        png::ColorType::Rgba => 4,
        png::ColorType::Indexed => return Err(load_error()),
    };

    Ok((to_premul_rgba(decoded, channels), info.width, info.height, channels == 1 || channels == 3))
}

fn decode_jpeg(bytes: &[u8]) -> io::Result<Decoded> {
    // The decoder converts the color spaces of JPEG (gray, YCbCr, CMYK,
    // YCCK) to the one it is asked for.
    let options = DecoderOptions::default().jpeg_set_out_colorspace(ColorSpace::RGBA);
    let mut decoder = zune_jpeg::JpegDecoder::new_with_options(ZCursor::new(bytes), options);

    let pixels = decoder.decode().map_err(|_| load_error())?;
    let info = decoder.info().ok_or_else(load_error)?;
    let channels = match decoder.output_colorspace().ok_or_else(load_error)? {
        ColorSpace::RGBA => 4,
        ColorSpace::RGB => 3,
        ColorSpace::Luma => 1,
        ColorSpace::LumaA => 2,
        _ => return Err(load_error()),
    };

    // A JPEG has no alpha: what the decoder puts in its place is opaque.
    Ok((to_premul_rgba(&pixels, channels), info.width as u32, info.height as u32, true))
}

fn decode_gif(bytes: &[u8]) -> io::Result<Decoded> {
    let mut options = gif::DecodeOptions::new();
    options.set_color_output(gif::ColorOutput::RGBA);

    let mut decoder = options.read_info(bytes).map_err(|_| load_error())?;
    let (width, height) = (decoder.width() as usize, decoder.height() as usize);

    // The first frame, on the canvas of the image, which is transparent
    // where the frame is not.
    let frame = decoder.read_next_frame().map_err(|_| load_error())?.ok_or_else(load_error)?;
    let (left, top) = (frame.left as usize, frame.top as usize);
    let (frame_width, frame_height) = (frame.width as usize, frame.height as usize);
    if frame.buffer.len() < frame_width * frame_height * 4 {
        return Err(load_error());
    }

    // Without a transparent color a frame that covers the image leaves
    // nothing transparent.
    let opaque = frame.transparent.is_none() && left == 0 && top == 0 && frame_width >= width && frame_height >= height;

    let mut rgba = vec![0u8; width * height * 4];
    for y in 0..frame_height.min(height.saturating_sub(top)) {
        let columns = frame_width.min(width.saturating_sub(left));
        let source = &frame.buffer[y * frame_width * 4..][..columns * 4];
        let target = &mut rgba[((top + y) * width + left) * 4..][..columns * 4];

        // A pixel of a GIF is opaque or transparent.
        for (target, source) in target.chunks_exact_mut(4).zip(source.chunks_exact(4)) {
            if source[3] != 0 {
                target.copy_from_slice(&[source[0], source[1], source[2], 255]);
            }
        }
    }

    Ok((rgba, width as u32, height as u32, opaque))
}

fn decode_bmp(bytes: &[u8]) -> io::Result<Decoded> {
    let mut decoder = zune_bmp::BmpDecoder::new(ZCursor::new(bytes));
    let pixels = decoder.decode().map_err(|_| load_error())?;
    let (width, height) = decoder.dimensions().ok_or_else(load_error)?;
    let channels = match decoder.colorspace().ok_or_else(load_error)? {
        ColorSpace::RGBA => 4,
        ColorSpace::RGB => 3,
        ColorSpace::Luma => 1,
        _ => return Err(load_error()),
    };
    if pixels.len() != width * height * channels {
        return Err(load_error());
    }

    Ok((to_premul_rgba(&pixels, channels), width as u32, height as u32, channels != 4))
}

/// The header of a wireless bitmap: its width and height and where its
/// pixels begin, when the data is one. The format has no signature: the
/// type and the fixed header are zero, the width and the height follow as
/// integers of seven bits a byte, and the data is as long as the pixels
/// need.
fn wbmp_header(bytes: &[u8]) -> Option<(usize, usize, usize)> {
    if bytes.len() < 4 || bytes[0] != 0 || bytes[1] & 0x9f != 0 {
        return None;
    }

    let mut offset = 2;
    let mut read = || {
        let mut value = 0usize;
        loop {
            let byte = *bytes.get(offset)?;
            offset += 1;
            value = (value << 7) | (byte & 0x7f) as usize;
            if value > u16::MAX as usize {
                return None;
            }
            if byte & 0x80 == 0 {
                return Some(value);
            }
        }
    };
    let (width, height) = (read()?, read()?);

    (width > 0 && height > 0 && bytes.len() == offset + width.div_ceil(8) * height).then_some((width, height, offset))
}

/// Decodes a wireless bitmap: a bit a pixel, set for white.
fn decode_wbmp(bytes: &[u8]) -> io::Result<Decoded> {
    let (width, height, offset) = wbmp_header(bytes).ok_or_else(load_error)?;
    let row_bytes = width.div_ceil(8);

    let mut rgba = Vec::with_capacity(width * height * 4);
    for row in bytes[offset..].chunks_exact(row_bytes) {
        for x in 0..width {
            let value = if row[x / 8] & (0x80 >> (x % 8)) != 0 { 255 } else { 0 };
            rgba.extend_from_slice(&[value, value, value, 255]);
        }
    }

    Ok((rgba, width as u32, height as u32, true))
}

fn u16_at(bytes: &[u8], offset: usize) -> io::Result<usize> {
    let bytes = bytes.get(offset..offset + 2).ok_or_else(load_error)?;
    Ok(u16::from_le_bytes([bytes[0], bytes[1]]) as usize)
}

fn u32_at(bytes: &[u8], offset: usize) -> io::Result<usize> {
    let bytes = bytes.get(offset..offset + 4).ok_or_else(load_error)?;
    Ok(u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]) as usize)
}

/// Decodes the largest image of an icon or a cursor: the first of the
/// largest ones in the directory, as the codec of Skia chooses.
fn decode_ico(bytes: &[u8]) -> io::Result<Decoded> {
    const DIRECTORY: usize = 6;
    const ENTRY: usize = 16;

    let count = u16_at(bytes, 4)?;
    let mut best: Option<(usize, &[u8])> = None;

    for index in 0..count {
        let entry = bytes.get(DIRECTORY + index * ENTRY..DIRECTORY + (index + 1) * ENTRY).ok_or_else(load_error)?;
        // A width or height of 0 stands for 256.
        let side = |value: u8| if value == 0 { 256 } else { value as usize };
        let area = side(entry[0]) * side(entry[1]);
        let (size, offset) = (u32_at(entry, 8)?, u32_at(entry, 12)?);

        // An entry whose data is not in the file is passed over.
        let Some(data) = offset.checked_add(size).and_then(|end| bytes.get(offset..end)) else {
            continue;
        };
        if best.is_none_or(|(best_area, _)| area > best_area) {
            best = Some((area, data));
        }
    }

    // An image of an icon has alpha, whatever it is encoded as.
    let (_, data) = best.ok_or_else(load_error)?;
    let (rgba, width, height, _) =
        if data.starts_with(&PNG_SIGNATURE) { decode_png(data) } else { decode_ico_bitmap(data) }?;

    Ok((rgba, width, height, false))
}

/// Decodes a bitmap of an icon: a bitmap without its file header, twice as
/// high as the picture, whose second half is a mask of one bit a pixel that
/// is set where the picture is transparent.
fn decode_ico_bitmap(data: &[u8]) -> io::Result<Decoded> {
    const FILE_HEADER: usize = 14;

    let header_size = u32_at(data, 0)?;
    if header_size < 40 || data.len() < header_size {
        return Err(load_error());
    }
    let (width, double_height) = (u32_at(data, 4)?, u32_at(data, 8)?);
    let (bits_per_pixel, compression) = (u16_at(data, 14)?, u32_at(data, 16)?);
    let colors_used = u32_at(data, 32)?;
    let height = double_height / 2;
    if width == 0 || height == 0 || width > 1 << 15 || height > 1 << 15 {
        return Err(load_error());
    }

    let palette_colors = match bits_per_pixel {
        1 | 4 | 8 if colors_used != 0 => colors_used.min(1 << bits_per_pixel),
        1 | 4 | 8 => 1 << bits_per_pixel,
        _ => 0,
    };
    // Three masks follow a header of forty bytes of a bitmap with bit
    // fields.
    let masks = if compression == 3 && header_size == 40 { 12 } else { 0 };
    let pixels_offset = header_size + masks + palette_colors * 4;
    let row_bytes = (width * bits_per_pixel).div_ceil(32) * 4;
    let mask_row_bytes = width.div_ceil(32) * 4;
    let mask_offset = pixels_offset + row_bytes * height;

    // Rows are stored from the bottom up.
    let rows = |offset: usize, row_bytes: usize| -> io::Result<Vec<&[u8]>> {
        (0..height)
            .map(|y| {
                let start = offset + (height - 1 - y) * row_bytes;
                data.get(start..start + row_bytes).ok_or_else(load_error)
            })
            .collect()
    };

    if bits_per_pixel == 32 && compression == 0 {
        // Blue, green, red and alpha: the picture has its alpha, and the
        // mask is not looked at.
        let mut rgba = Vec::with_capacity(width * height * 4);
        for row in rows(pixels_offset, row_bytes)? {
            for p in row[..width * 4].chunks_exact(4) {
                rgba.extend_from_slice(&[premultiply(p[2], p[3]), premultiply(p[1], p[3]), premultiply(p[0], p[3]), p[3]]);
            }
        }
        return Ok((rgba, width as u32, height as u32, false));
    }

    // The picture as a bitmap file: a file header, the header with the
    // height of the picture, and the rest as it is.
    let picture_end = mask_offset.min(data.len());
    let mut file = Vec::with_capacity(FILE_HEADER + picture_end);
    file.extend_from_slice(b"BM");
    file.extend_from_slice(&((FILE_HEADER + picture_end) as u32).to_le_bytes());
    file.extend_from_slice(&[0; 4]);
    file.extend_from_slice(&((FILE_HEADER + pixels_offset) as u32).to_le_bytes());
    file.extend_from_slice(&data[..picture_end]);
    file[FILE_HEADER + 8..FILE_HEADER + 12].copy_from_slice(&(height as u32).to_le_bytes());
    // The size of the pixels, which counted the mask.
    file[FILE_HEADER + 20..FILE_HEADER + 24].copy_from_slice(&[0; 4]);

    let (mut rgba, decoded_width, decoded_height, _) = decode_bmp(&file)?;
    if (decoded_width as usize, decoded_height as usize) != (width, height) {
        return Err(load_error());
    }

    // The mask, when the file has one.
    if let Ok(mask) = rows(mask_offset, mask_row_bytes) {
        for (pixels, mask) in rgba.chunks_exact_mut(width * 4).zip(mask) {
            for (x, pixel) in pixels.chunks_exact_mut(4).enumerate() {
                if mask[x / 8] & (0x80 >> (x % 8)) != 0 {
                    pixel.copy_from_slice(&[0; 4]);
                }
            }
        }
    }

    Ok((rgba, width as u32, height as u32, false))
}

/// The size a JPEG is decoded at when it is wanted `scale` times as large
/// as it is, as the codec of Skia chooses it: libjpeg decodes at a number
/// of eighths of the size, and the codec takes the number nearest to the
/// scale.
pub fn jpeg_scaled_dimensions(width: u32, height: u32, scale: f64) -> (u32, u32) {
    let eighths: u32 = if scale >= 0.9375 {
        8
    } else if scale >= 0.8125 {
        7
    } else if scale >= 0.6875 {
        6
    } else if scale >= 0.5625 {
        5
    } else if scale >= 0.4375 {
        4
    } else if scale >= 0.3125 {
        3
    } else if scale >= 0.1875 {
        2
    } else {
        1
    };

    // Rounded up, as libjpeg computes the size of its output.
    ((width * eighths).div_ceil(8).max(1), (height * eighths).div_ceil(8).max(1))
}

/// Premultiplied RGBA pixels reduced to a smaller size, every pixel the
/// mean of the area of the image it covers: what decoding a JPEG at a
/// number of eighths of its size comes to, which the decoder of this
/// backend does not do itself.
pub fn reduce_by_area(rgba: &[u8], width: usize, height: usize, new_width: usize, new_height: usize) -> Vec<u8> {
    // The pixels of the image a pixel of the result covers along an axis,
    // each with the share of it that is covered.
    let spans = |length: usize, new_length: usize| -> Vec<Vec<(usize, f64)>> {
        let ratio = length as f64 / new_length as f64;
        (0..new_length)
            .map(|index| {
                let (start, end) = (index as f64 * ratio, ((index + 1) as f64 * ratio).min(length as f64));
                (start.floor() as usize..(end.ceil() as usize).min(length))
                    .map(|pixel| (pixel, (end.min(pixel as f64 + 1.0) - start.max(pixel as f64)).max(0.0)))
                    .filter(|(_, share)| *share > 0.0)
                    .collect()
            })
            .collect()
    };
    let (columns, rows) = (spans(width, new_width), spans(height, new_height));

    let mut reduced = Vec::with_capacity(new_width * new_height * 4);
    for row in &rows {
        for column in &columns {
            let (mut sum, mut total) = ([0.0f64; 4], 0.0f64);
            for (y, share_y) in row {
                for (x, share_x) in column {
                    let share = share_x * share_y;
                    let pixel = &rgba[(y * width + x) * 4..][..4];
                    for (channel, value) in sum.iter_mut().zip(pixel) {
                        *channel += *value as f64 * share;
                    }
                    total += share;
                }
            }
            reduced.extend(sum.map(|channel| (channel / total.max(f64::MIN_POSITIVE)).round().clamp(0.0, 255.0) as u8));
        }
    }

    reduced
}
