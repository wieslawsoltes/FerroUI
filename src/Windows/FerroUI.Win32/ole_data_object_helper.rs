//! Contains helper methods to read from and write to an HGlobal, for
//! `IDataObject` interop.
//!
//! What a block of global memory holds for a format is computed here as
//! bytes (the text with its terminator, the list of file names, a device
//! independent bitmap with its header), by functions that run on every
//! host; the part for Windows (`imp`) moves the bytes in and out of the
//! blocks and asks the system for what only it can do (the names of a
//! `CF_HDROP` block, the pixels of a bitmap handle and of a device
//! independent bitmap).

use crate::interop::unmanaged_methods::{
    BitmapCompressionMode, ClipboardFormat, BITMAPINFOHEADER, BITMAPV5HEADER, DVASPECT, FORMATETC, LCS_GM_ABS_COLORIMETRIC,
    LCS_SRGB, SIZE_OF_DROPFILES, TYMED,
};
use ferroui_base::platform::PixelFormat;

/// The format of a data transfer for a clipboard format: its content, as a
/// block of global memory, or as a bitmap handle for `CF_BITMAP`.
pub(crate) fn to_format_etc(format_id: u16) -> FORMATETC {
    FORMATETC {
        cf_format: format_id,
        dw_aspect: DVASPECT::DVASPECT_CONTENT,
        ptd: 0,
        lindex: -1,
        tymed: if format_id == ClipboardFormat::CF_BITMAP { TYMED::TYMED_GDI } else { TYMED::TYMED_HGLOBAL },
    }
}

/// The size of what follows the header of a device independent bitmap
/// before its pixels: a colour table or three colour masks.
pub(crate) fn get_extra_header_size(header: &BITMAPINFOHEADER) -> usize {
    // https://learn.microsoft.com/en-us/windows/win32/api/wingdi/ns-wingdi-bitmapinfoheader
    match header.bi_compression {
        // If biCompression equals BI_RGB and the bitmap uses 8 bpp or less, the bitmap has a color table immediately
        // following the BITMAPINFOHEADER structure. The color table consists of an array of RGBQUAD values. The size
        // of the array is given by the biClrUsed member.
        // If biClrUsed is zero, the array contains the maximum number of colors for the given bitdepth; that is,
        // 2^biBitCount colors.
        BitmapCompressionMode::BI_RGB if header.bi_bit_count <= 8 => {
            (if header.bi_clr_used == 0 { 1usize << header.bi_bit_count } else { header.bi_clr_used as usize })
                .saturating_mul(4)
        }

        // If biCompression equals BI_BITFIELDS, the bitmap uses three DWORD color masks (red, green, and blue,
        // respectively), which specify the byte layout of the pixels. The 1 bits in each mask indicate the bits for
        // that color within the pixel.
        BitmapCompressionMode::BI_BITFIELDS => 3 * 4,

        _ => 0,
    }
}

/// The text of a block: its UTF-16 units up to the first terminator, or
/// to the end of the block when it has none.
pub(crate) fn read_string_from_bytes(bytes: &[u8]) -> String {
    let units: Vec<u16> = bytes
        .chunks_exact(2)
        .map(|unit| u16::from_le_bytes([unit[0], unit[1]]))
        .take_while(|&unit| unit != 0)
        .collect();
    String::from_utf16_lossy(&units)
}

/// The block of a text: its UTF-16 units and a terminator.
pub(crate) fn string_to_bytes(data: &str) -> Vec<u8> {
    let mut bytes: Vec<u8> = data.encode_utf16().flat_map(u16::to_le_bytes).collect();
    bytes.extend_from_slice(&[0, 0]);
    bytes
}

/// The block of a list of file names (`CF_HDROP`): a `DROPFILES` structure
/// that says the names follow it as wide characters, the names with a
/// terminator each, and a terminator of the list.
pub(crate) fn file_names_to_bytes<'a>(file_names: impl IntoIterator<Item = &'a str>) -> Vec<u8> {
    let mut bytes = Vec::new();
    // pFiles, pt, fNC, fWide
    bytes.extend_from_slice(&SIZE_OF_DROPFILES.to_le_bytes());
    bytes.extend_from_slice(&[0; 8]);
    bytes.extend_from_slice(&0u32.to_le_bytes());
    bytes.extend_from_slice(&1u32.to_le_bytes());

    for file_name in file_names {
        bytes.extend(file_name.encode_utf16().flat_map(u16::to_le_bytes));
        bytes.extend_from_slice(&[0, 0]);
    }

    bytes.extend_from_slice(&[0, 0]);
    bytes
}

/// The four colour masks of the pixels of a bitmap, in the order alpha,
/// red, green, blue; `None` for a format the clipboard formats of bitmaps
/// do not describe (the reference fails with "not supported").
pub(crate) fn color_masks(format: Option<PixelFormat>) -> Option<[u32; 4]> {
    let format = format?;
    if format == PixelFormat::RGBA8888 {
        Some([0xff00_0000, 0x0000_00ff, 0x0000_ff00, 0x00ff_0000])
    } else if format == PixelFormat::BGRA8888 {
        Some([0xff00_0000, 0x00ff_0000, 0x0000_ff00, 0x0000_00ff])
    } else if format == PixelFormat::RGB565 {
        Some([0, 0b0000_0000_0001_1111, 0b0000_0111_1110_0000, 0b1111_1000_0000_0000])
    } else {
        None
    }
}

/// The block of a device independent bitmap (`CF_DIB`): a header that
/// says the rows run from the top, and the pixels.
pub(crate) fn dib_bytes(pixels: &[u8], width: i32, height: i32, bits_per_pixel: u16) -> Vec<u8> {
    let mut info_header = BITMAPINFOHEADER {
        bi_size_image: pixels.len() as u32,
        bi_width: width,
        bi_height: -height,
        bi_bit_count: bits_per_pixel,
        bi_planes: 1,
        bi_compression: BitmapCompressionMode::BI_RGB,
        ..Default::default()
    };
    info_header.init();

    let mut image_data = Vec::with_capacity(info_header.bi_size as usize + pixels.len());
    image_data.extend_from_slice(&info_header.to_bytes());
    image_data.extend_from_slice(pixels);
    image_data
}

/// The header of a bitmap of version 5 for pixels of the masks given.
pub(crate) fn bitmap_v5_header(
    width: i32,
    height: i32,
    bits_per_pixel: u16,
    compression: u32,
    size_image: u32,
    masks: [u32; 4],
) -> BITMAPV5HEADER {
    BITMAPV5HEADER {
        b_v5_width: width,
        b_v5_height: -height,
        b_v5_planes: 1,
        b_v5_bit_count: bits_per_pixel,
        b_v5_compression: compression,
        b_v5_size_image: size_image,
        b_v5_alpha_mask: masks[0],
        b_v5_red_mask: masks[1],
        b_v5_green_mask: masks[2],
        b_v5_blue_mask: masks[3],
        b_v5_cs_type: LCS_SRGB,
        b_v5_intent: LCS_GM_ABS_COLORIMETRIC,
    }
}

/// The block of a device independent bitmap of version 5 (`CF_DIBV5`).
pub(crate) fn dib_v5_bytes(pixels: &[u8], width: i32, height: i32, bits_per_pixel: u16, masks: [u32; 4]) -> Vec<u8> {
    let info_header = bitmap_v5_header(
        width,
        height,
        bits_per_pixel,
        if bits_per_pixel > 16 { BitmapCompressionMode::BI_BITFIELDS } else { BitmapCompressionMode::BI_RGB },
        pixels.len() as u32,
        masks,
    );

    let header = info_header.to_bytes();
    let mut image_data = Vec::with_capacity(header.len() + pixels.len());
    image_data.extend_from_slice(&header);
    image_data.extend_from_slice(pixels);
    image_data
}

#[cfg(windows)]
pub(crate) use imp::*;

#[cfg(windows)]
mod imp {
    use super::*;
    use crate::clipboard_format_registry::ClipboardFormatRegistry;
    use crate::interop::unmanaged_methods::{
        self, BgraPixels, GlobalAllocFlags, DV_E_FORMATETC, HRESULT, STGMEDIUM, STG_E_MEDIUMFULL,
    };
    use crate::win32_com::IDataObject;
    use ferroui_base::input::platform::ClipboardError;
    use ferroui_base::input::{DataFormat, DataTransferExtensions, IDataTransfer};
    use ferroui_base::logging::{LogArea, LogEventLevel, Logger};
    use ferroui_base::media::imaging::{Bitmap, BitmapEncoderOptions, PngBitmapEncoderOptions};
    use ferroui_base::platform::storage::file_io::{BclStorageItemHandle, StorageProviderHelpers};
    use ferroui_base::platform::storage::IStorageItem;
    use ferroui_base::platform::AlphaFormat;
    use ferroui_base::{PixelRect, PixelSize, Vector};
    use std::any::{Any, TypeId};
    use std::rc::Rc;

    /// What a data object holds for the file format: the items of the
    /// files and folders that exist.
    pub(crate) type StorageItems = Vec<Rc<dyn IStorageItem>>;

    /// Releases a storage medium a data object filled when dropped.
    struct MediumGuard(STGMEDIUM);

    impl Drop for MediumGuard {
        fn drop(&mut self) {
            // SAFETY: the medium was filled by a successful `GetData` and
            // is not used after this.
            unsafe { unmanaged_methods::release_stg_medium(&mut self.0) };
        }
    }

    /// The value a data object holds for a format: a `String` for text and
    /// for a format of strings, [`StorageItems`] for files, a shared
    /// bitmap for the bitmap format, shared bytes for a format of bytes.
    /// `None` when the object does not have the format; an error when it
    /// has data that cannot be read as the format.
    pub(crate) fn try_get(ole_data_object: &IDataObject, format: &DataFormat) -> Result<Option<Rc<dyn Any>>, ClipboardError> {
        let Some(format_id) = try_get_contained_format(ole_data_object, format) else {
            return Ok(None);
        };

        let mut medium = STGMEDIUM::default();
        let mut format_etc = to_format_etc(format_id);
        // SAFETY: two structures of this frame, for the duration of the
        // call.
        let result = unsafe { ole_data_object.get_data(&mut format_etc, &mut medium) };
        if result != HRESULT::S_OK {
            return Ok(None);
        }

        let medium = MediumGuard(medium);
        if medium.0.unionmember != 0 {
            if medium.0.tymed == TYMED::TYMED_HGLOBAL {
                let h_global = medium.0.unionmember;
                // SAFETY: the block of a medium the data object just
                // filled, alive until the guard releases it.
                return unsafe { read_data_from_hglobal(format, h_global, format_etc) };
            } else if medium.0.tymed == TYMED::TYMED_GDI {
                let bitmap_handle = medium.0.unionmember;
                // SAFETY: the bitmap of a medium the data object just
                // filled, alive until the guard releases it.
                return Ok(unsafe { read_data_from_gdi(bitmap_handle) });
            }
        }

        Ok(None)
    }

    fn try_get_contained_format(ole_data_object: &IDataObject, format: &DataFormat) -> Option<u16> {
        // Bitmap is not a real format, find the first matching platform format, if any.
        if DataFormat::bitmap() == *format {
            return ClipboardFormatRegistry::image_formats()
                .iter()
                .find_map(|image_format| try_get_contained_format_core(ole_data_object, image_format));
        }

        try_get_contained_format_core(ole_data_object, format)
    }

    fn try_get_contained_format_core(ole_data_object: &IDataObject, format: &DataFormat) -> Option<u16> {
        debug_assert!(*format != DataFormat::bitmap());

        let format_id = ClipboardFormatRegistry::get_or_add_format(format);
        let mut format_etc = to_format_etc(format_id);
        // SAFETY: a structure of this frame, for the duration of the call.
        (unsafe { ole_data_object.query_get_data(&mut format_etc) } == HRESULT::S_OK).then_some(format_id)
    }

    fn bitmap_from_bgra(pixels: BgraPixels) -> Rc<dyn Any> {
        let bitmap = Bitmap::from_pixels(
            PixelFormat::BGRA8888,
            AlphaFormat::Opaque,
            &pixels.pixels,
            PixelSize::new(pixels.width, pixels.height),
            Vector::new(96.0, 96.0),
            pixels.width * 4,
        );
        Rc::new(Rc::new(bitmap))
    }

    /// # Safety
    /// `bitmap_handle` is a live bitmap that is selected into no context.
    unsafe fn read_data_from_gdi(bitmap_handle: isize) -> Option<Rc<dyn Any>> {
        // SAFETY: the contract of this function.
        unsafe { unmanaged_methods::hbitmap_to_bgra(bitmap_handle) }.map(bitmap_from_bgra)
    }

    /// The value of a block of global memory for a format.
    ///
    /// # Safety
    /// `h_global` is a live block of global memory.
    pub(crate) unsafe fn read_data_from_hglobal(
        format: &DataFormat,
        h_global: isize,
        format_etc: FORMATETC,
    ) -> Result<Option<Rc<dyn Any>>, ClipboardError> {
        // SAFETY (of every read below): the contract of this function.
        if DataFormat::text() == *format {
            return Ok(unsafe { read_string_from_hglobal(h_global) }.map(|text| Rc::new(text) as Rc<dyn Any>));
        }

        if DataFormat::file() == *format {
            let items: StorageItems = unsafe { unmanaged_methods::drag_query_file_names(h_global) }
                .iter()
                .filter_map(|file_name| StorageProviderHelpers::try_create_bcl_storage_item(Some(file_name)))
                .map(|storage_item| match storage_item {
                    BclStorageItemHandle::Folder(folder) => folder as Rc<dyn IStorageItem>,
                    BclStorageItemHandle::File(file) => file as Rc<dyn IStorageItem>,
                })
                .collect();
            return Ok(Some(Rc::new(items)));
        }

        if DataFormat::bitmap() == *format {
            let Some(data) = (unsafe { read_bytes_from_hglobal(h_global) }) else {
                return Ok(None);
            };
            if format_etc.cf_format == ClipboardFormat::CF_DIB {
                let Some(source_header) = BITMAPINFOHEADER::from_bytes(&data) else {
                    return Ok(None);
                };
                let extra_source_header_size = get_extra_header_size(&source_header);
                return Ok(unmanaged_methods::dib_to_bgra(&data, extra_source_header_size).map(bitmap_from_bgra));
            } else {
                // The reference decodes the bytes whatever the format is;
                // bytes that are not an encoded image are its exception.
                let bitmap = Bitmap::from_stream(&mut data.as_slice()).map_err(|error| {
                    ClipboardError::other(format!("The bitmap of the data object could not be decoded: {error}"))
                })?;
                return Ok(Some(Rc::new(Rc::new(bitmap))));
            }
        }

        if format.data_type() == Some(TypeId::of::<String>()) {
            return Ok(unsafe { read_string_from_hglobal(h_global) }.map(|text| Rc::new(text) as Rc<dyn Any>));
        }

        if format.data_type() == Some(TypeId::of::<Rc<[u8]>>()) {
            return Ok(unsafe { read_bytes_from_hglobal(h_global) }.map(|bytes| Rc::new(Rc::<[u8]>::from(bytes)) as Rc<dyn Any>));
        }

        Ok(None)
    }

    /// # Safety
    /// `h_global` is a live block of global memory.
    unsafe fn read_string_from_hglobal(h_global: isize) -> Option<String> {
        // SAFETY: the contract of this function.
        unsafe { unmanaged_methods::read_global(h_global) }.map(|bytes| read_string_from_bytes(&bytes))
    }

    /// The bytes of a block of global memory.
    ///
    /// # Safety
    /// `h_global` is a live block of global memory.
    pub(crate) unsafe fn read_bytes_from_hglobal(h_global: isize) -> Option<Vec<u8>> {
        // SAFETY: the contract of this function.
        unsafe { unmanaged_methods::read_global(h_global) }
    }

    /// The pixels of a bitmap in its own format, row after row without
    /// padding, with its size and the bits of a pixel.
    fn copy_bitmap_pixels(bitmap: &Bitmap) -> (Vec<u8>, PixelSize, u16) {
        let pixel_size = bitmap.pixel_size();
        let bpp = bitmap.format().map_or(0, |format| format.bits_per_pixel());
        let stride = (bpp / 8) as i32 * pixel_size.width;
        let mut buffer = vec![0u8; (stride * pixel_size.height).max(0) as usize];
        if !buffer.is_empty() {
            bitmap.copy_pixels(PixelRect::new(0, 0, pixel_size.width, pixel_size.height), &mut buffer, stride);
        }
        (buffer, pixel_size, bpp as u16)
    }

    /// Writes the value a data transfer has for a format to a block of
    /// global memory: to `h_global` when it is a block, else to a block
    /// that is allocated and returned through it. Returns a result code.
    ///
    /// # Safety
    /// `h_global` is 0 or a live block of global memory.
    pub(crate) unsafe fn write_data_to_hglobal(data_transfer: &dyn IDataTransfer, format: &DataFormat, h_global: &mut isize) -> u32 {
        // SAFETY (of every write below): the contract of this function.
        if DataFormat::text() == *format {
            let text = data_transfer.try_get_value(&DataFormat::text());
            return unsafe { write_bytes_to_hglobal(h_global, &string_to_bytes(&text.unwrap_or_default())) };
        }

        if DataFormat::file() == *format {
            let files = data_transfer.try_get_values(&DataFormat::file()).unwrap_or_default();

            let file_names: Vec<String> = files.iter().filter_map(|file| file.try_get_local_path()).collect();

            return unsafe { write_bytes_to_hglobal(h_global, &file_names_to_bytes(file_names.iter().map(String::as_str))) };
        }

        if ClipboardFormatRegistry::dib_data_format() == *format || ClipboardFormatRegistry::dib_v5_data_format() == *format {
            if let Some(bitmap) = data_transfer.try_get_value(&DataFormat::bitmap()) {
                let is_v5 = ClipboardFormatRegistry::dib_v5_data_format() == *format;
                let (buffer, pixel_size, bpp) = copy_bitmap_pixels(&bitmap);

                let image_data = if !is_v5 {
                    dib_bytes(&buffer, pixel_size.width, pixel_size.height, bpp)
                } else {
                    // The reference fails with "not supported" for pixels
                    // it has no masks for.
                    let Some(masks) = color_masks(bitmap.format()) else {
                        return DV_E_FORMATETC;
                    };
                    dib_v5_bytes(&buffer, pixel_size.width, pixel_size.height, bpp, masks)
                };
                return unsafe { write_bytes_to_hglobal(h_global, &image_data) };
            }
        }

        if ClipboardFormatRegistry::png_system_data_format() == *format || ClipboardFormatRegistry::png_mime_data_format() == *format {
            if let Some(bitmap) = data_transfer.try_get_value(&DataFormat::bitmap()) {
                let mut stream = Vec::new();
                if bitmap.save(&mut stream, &BitmapEncoderOptions::Png(PngBitmapEncoderOptions::DEFAULT)).is_err() {
                    return DV_E_FORMATETC;
                }

                return unsafe { write_bytes_to_hglobal(h_global, &stream) };
            }
            return DV_E_FORMATETC;
        }

        // A format of strings or of bytes. The type of a format of the
        // reference is the type of the format object; a format of the port
        // carries it when it was created with one, and the value says it
        // otherwise.
        let value = data_transfer.get_items(format).find_map(|item| item.try_get_raw(format));
        let is_string_format = format.data_type() == Some(TypeId::of::<String>());
        let is_bytes_format = format.data_type() == Some(TypeId::of::<Rc<[u8]>>());
        let untyped = format.data_type().is_none();

        if is_string_format || untyped {
            if let Some(string_value) = value.as_ref().and_then(|value| value.downcast_ref::<String>()) {
                return unsafe { write_bytes_to_hglobal(h_global, &string_to_bytes(string_value)) };
            }
            if is_string_format {
                return DV_E_FORMATETC;
            }
        }

        if is_bytes_format || untyped {
            if let Some(bytes) = value.as_ref().and_then(|value| value.downcast_ref::<Rc<[u8]>>()) {
                return unsafe { write_bytes_to_hglobal(h_global, bytes) };
            }
            if is_bytes_format {
                return DV_E_FORMATETC;
            }
        }

        if let Some(logger) = Logger::try_get(LogEventLevel::Warning, LogArea::WIN32_PLATFORM) {
            logger.log(None, &format!("Unsupported data format {format}"));
        }

        DV_E_FORMATETC
    }

    /// Makes a bitmap handle of the bitmap of a data transfer, for the
    /// format `CF_BITMAP`, and returns it through `h_global_bitmap`.
    /// Returns a result code.
    pub(crate) fn write_data_to_gdi(data_transfer: &dyn IDataTransfer, format: &DataFormat, h_global_bitmap: &mut isize) -> u32 {
        if ClipboardFormatRegistry::h_bitmap_data_format() == *format {
            if let Some(bitmap) = data_transfer.try_get_value(&DataFormat::bitmap()) {
                let (buffer, pixel_size, bpp) = copy_bitmap_pixels(&bitmap);
                let Some(masks) = color_masks(bitmap.format()) else {
                    return DV_E_FORMATETC;
                };

                let bitmap_info_header = bitmap_v5_header(
                    pixel_size.width,
                    pixel_size.height,
                    bpp,
                    BitmapCompressionMode::BI_BITFIELDS,
                    buffer.len() as u32,
                    masks,
                );

                let hbitmap = unmanaged_methods::pixels_to_hbitmap(&buffer, pixel_size.width, pixel_size.height, &bitmap_info_header);
                if hbitmap == 0 {
                    return DV_E_FORMATETC;
                }

                *h_global_bitmap = hbitmap;

                return HRESULT::S_OK;
            }
        }

        if let Some(logger) = Logger::try_get(LogEventLevel::Warning, LogArea::WIN32_PLATFORM) {
            logger.log(None, &format!("Unsupported gdi data format {format}"));
        }

        DV_E_FORMATETC
    }

    /// Copies bytes to a block of global memory: to `h_global` when it is
    /// a block, which has to have room for them, else to a new block.
    ///
    /// # Safety
    /// `h_global` is 0 or a live block of global memory.
    pub(crate) unsafe fn write_bytes_to_hglobal(h_global: &mut isize, data: &[u8]) -> u32 {
        let required_size = data.len();

        if *h_global == 0 {
            *h_global = unmanaged_methods::global_alloc(GlobalAllocFlags::GHND, required_size);
        }

        let available = unmanaged_methods::global_size(*h_global);
        if required_size > available {
            return STG_E_MEDIUMFULL;
        }

        // SAFETY: a live block by the contract of this function, or the
        // block that was just allocated.
        if unsafe { unmanaged_methods::write_global(*h_global, data) } || data.is_empty() {
            HRESULT::S_OK
        } else {
            STG_E_MEDIUMFULL
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_format_is_asked_for_as_content_in_global_memory() {
        let format_etc = to_format_etc(ClipboardFormat::CF_UNICODETEXT);
        assert_eq!(format_etc.cf_format, 13);
        assert_eq!(format_etc.dw_aspect, DVASPECT::DVASPECT_CONTENT);
        assert_eq!(format_etc.ptd, 0);
        assert_eq!(format_etc.lindex, -1);
        assert_eq!(format_etc.tymed, TYMED::TYMED_HGLOBAL);
        assert_eq!(to_format_etc(0xC123).tymed, TYMED::TYMED_HGLOBAL);
    }

    #[test]
    fn a_bitmap_handle_is_asked_for_as_a_gdi_object() {
        assert_eq!(to_format_etc(ClipboardFormat::CF_BITMAP).tymed, TYMED::TYMED_GDI);
    }

    #[test]
    fn text_round_trips_through_its_block() {
        for text in ["", "a", "Zażółć gęślą jaźń", "two\r\nlines", "\u{1F600} outside the basic plane"] {
            let bytes = string_to_bytes(text);
            assert_eq!(bytes.len(), (text.encode_utf16().count() + 1) * 2);
            assert_eq!(&bytes[bytes.len() - 2..], [0, 0]);
            assert_eq!(read_string_from_bytes(&bytes), text);
        }
    }

    #[test]
    fn text_ends_at_its_terminator_or_at_the_end_of_the_block() {
        // A block is often larger than the text: what follows the
        // terminator is not text.
        let mut bytes = string_to_bytes("abc");
        bytes.extend_from_slice(&[b'x', 0, b'y', 0]);
        assert_eq!(read_string_from_bytes(&bytes), "abc");
        // Without a terminator the text is the block; an odd byte at the
        // end is not a character.
        assert_eq!(read_string_from_bytes(&[b'a', 0, b'b', 0, b'c']), "ab");
        assert_eq!(read_string_from_bytes(&[]), "");
    }

    #[test]
    fn file_names_follow_a_drop_files_structure() {
        let bytes = file_names_to_bytes(["C:\\a.txt", "D:\\ż"]);
        // The offset of the names, no point, the client area flag off,
        // wide characters.
        assert_eq!(&bytes[0..4], 20u32.to_le_bytes());
        assert_eq!(&bytes[4..16], [0; 12]);
        assert_eq!(&bytes[16..20], 1u32.to_le_bytes());
        let units: Vec<u16> = bytes[20..].chunks_exact(2).map(|unit| u16::from_le_bytes([unit[0], unit[1]])).collect();
        let names: Vec<String> = units.split(|&unit| unit == 0).map(String::from_utf16_lossy).collect();
        // Two names, then the empty name that ends the list (and the
        // empty slice after its terminator).
        assert_eq!(names, ["C:\\a.txt", "D:\\ż", "", ""]);
        assert_eq!(bytes.len(), 20 + ("C:\\a.txt".len() + 1 + 4 + 1 + 1) * 2);
    }

    #[test]
    fn an_empty_list_of_file_names_is_a_structure_and_a_terminator() {
        let bytes = file_names_to_bytes([]);
        assert_eq!(bytes.len(), 22);
        assert_eq!(&bytes[20..], [0, 0]);
    }

    fn header(compression: u32, bit_count: u16, clr_used: u32) -> BITMAPINFOHEADER {
        BITMAPINFOHEADER { bi_compression: compression, bi_bit_count: bit_count, bi_clr_used: clr_used, ..Default::default() }
    }

    #[test]
    fn the_colour_table_of_a_bitmap_follows_its_header() {
        assert_eq!(get_extra_header_size(&header(BitmapCompressionMode::BI_RGB, 8, 0)), 256 * 4);
        assert_eq!(get_extra_header_size(&header(BitmapCompressionMode::BI_RGB, 4, 0)), 16 * 4);
        assert_eq!(get_extra_header_size(&header(BitmapCompressionMode::BI_RGB, 1, 0)), 2 * 4);
        assert_eq!(get_extra_header_size(&header(BitmapCompressionMode::BI_RGB, 8, 17)), 17 * 4);
        assert_eq!(get_extra_header_size(&header(BitmapCompressionMode::BI_RGB, 24, 0)), 0);
        assert_eq!(get_extra_header_size(&header(BitmapCompressionMode::BI_RGB, 32, 0)), 0);
        assert_eq!(get_extra_header_size(&header(BitmapCompressionMode::BI_BITFIELDS, 32, 0)), 12);
        assert_eq!(get_extra_header_size(&header(BitmapCompressionMode::BI_BITFIELDS, 16, 0)), 12);
        assert_eq!(get_extra_header_size(&header(BitmapCompressionMode::BI_JPEG, 0, 0)), 0);
    }

    #[test]
    fn the_masks_follow_the_order_of_the_channels() {
        // Alpha, red, green, blue.
        assert_eq!(color_masks(Some(PixelFormat::BGRA8888)), Some([0xff00_0000, 0x00ff_0000, 0x0000_ff00, 0x0000_00ff]));
        assert_eq!(color_masks(Some(PixelFormat::RGBA8888)), Some([0xff00_0000, 0x0000_00ff, 0x0000_ff00, 0x00ff_0000]));
        assert_eq!(color_masks(Some(PixelFormat::RGB565)), Some([0, 0x001f, 0x07e0, 0xf800]));
        assert_eq!(color_masks(Some(PixelFormat::RGB32)), None);
        assert_eq!(color_masks(None), None);
    }

    #[test]
    fn a_device_independent_bitmap_is_a_header_and_the_pixels_from_the_top() {
        let pixels: Vec<u8> = (0..2 * 3 * 4).collect();
        let bytes = dib_bytes(&pixels, 2, 3, 32);
        assert_eq!(bytes.len(), 40 + pixels.len());
        let header = BITMAPINFOHEADER::from_bytes(&bytes).unwrap();
        assert_eq!(header.bi_size, 40);
        assert_eq!((header.bi_width, header.bi_height), (2, -3));
        assert_eq!((header.bi_planes, header.bi_bit_count), (1, 32));
        assert_eq!(header.bi_compression, BitmapCompressionMode::BI_RGB);
        assert_eq!(header.bi_size_image, 24);
        assert_eq!(&bytes[40..], pixels);
        // What the reader of such a block skips before the pixels.
        assert_eq!(get_extra_header_size(&header), 0);
    }

    #[test]
    fn a_bitmap_of_version_5_carries_its_masks() {
        let pixels = [0u8; 16];
        let masks = color_masks(Some(PixelFormat::BGRA8888)).unwrap();
        let bytes = dib_v5_bytes(&pixels, 2, 2, 32, masks);
        assert_eq!(bytes.len(), 124 + 16);
        let u32_at = |offset: usize| u32::from_le_bytes([bytes[offset], bytes[offset + 1], bytes[offset + 2], bytes[offset + 3]]);
        assert_eq!(u32_at(0), 124);
        assert_eq!(u32_at(4) as i32, 2);
        assert_eq!(u32_at(8) as i32, -2);
        assert_eq!(u32_at(12), 1 | (32 << 16));
        assert_eq!(u32_at(16), BitmapCompressionMode::BI_BITFIELDS);
        assert_eq!(u32_at(20), 16);
        assert_eq!([u32_at(40), u32_at(44), u32_at(48), u32_at(52)], [0x00ff_0000, 0x0000_ff00, 0x0000_00ff, 0xff00_0000]);
        assert_eq!(u32_at(56), 0x7352_4742);
        assert_eq!(u32_at(108), 8);
        // 16 bits a pixel are not described by masks.
        let small = dib_v5_bytes(&[0u8; 8], 2, 2, 16, color_masks(Some(PixelFormat::RGB565)).unwrap());
        assert_eq!(u32::from_le_bytes([small[16], small[17], small[18], small[19]]), BitmapCompressionMode::BI_RGB);
    }
}
