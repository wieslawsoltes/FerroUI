//! An icon of the system: created from the data of an icon file (the image
//! of the directory that fits a size best) or from a bitmap.
//!
//! What chooses and converts is written as functions of values and tested
//! on every host; the type that owns the handle is Windows only.

use ferroui_base::PixelSize;

/// The size of `ICONDIR` of the reference: the three numbers of the header
/// and the first entry.
pub(crate) const ICONDIR_SIZE: usize = 6 + ICONDIRENTRY_SIZE;
/// The size of an entry of the directory of an icon file.
pub(crate) const ICONDIRENTRY_SIZE: usize = 16;

/// An entry of the directory of an icon file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(non_snake_case)]
pub(crate) struct ICONDIRENTRY {
    // Width and height are 1 - 255 or 0 for 256
    pub bWidth: u8,
    pub bHeight: u8,
    pub bColorCount: u8,
    pub bReserved: u8,
    pub wPlanes: u16,
    pub wBitCount: u16,
    pub dwBytesInRes: u32,
    pub dwImageOffset: u32,
}

impl ICONDIRENTRY {
    fn read(bytes: &[u8]) -> ICONDIRENTRY {
        ICONDIRENTRY {
            bWidth: bytes[0],
            bHeight: bytes[1],
            bColorCount: bytes[2],
            bReserved: bytes[3],
            wPlanes: u16::from_le_bytes([bytes[4], bytes[5]]),
            wBitCount: u16::from_le_bytes([bytes[6], bytes[7]]),
            dwBytesInRes: u32::from_le_bytes([bytes[8], bytes[9], bytes[10], bytes[11]]),
            dwImageOffset: u32::from_le_bytes([bytes[12], bytes[13], bytes[14], bytes[15]]),
        }
    }
}

/// The image of an icon file that was chosen for a size: where its bytes
/// are in the file, and the size its entry states.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct BestIconImage {
    pub image_offset: u32,
    pub bytes_in_res: u32,
    pub size: PixelSize,
}

/// Chooses the image of an icon file for a size, on a display of the bit
/// depth given. `None` when the data is not an icon file or its directory
/// does not fit the data.
pub(crate) fn find_best_icon_image(icon_data: &[u8], size: PixelSize, bit_depth: u32) -> Option<BestIconImage> {
    if icon_data.len() < ICONDIR_SIZE {
        return None;
    }

    let id_reserved = u16::from_le_bytes([icon_data[0], icon_data[1]]);
    let id_type = u16::from_le_bytes([icon_data[2], icon_data[3]]);
    let id_count = usize::from(u16::from_le_bytes([icon_data[4], icon_data[5]]));

    if id_reserved != 0 || id_type != 1 || id_count == 0 {
        return None;
    }

    let mut best_width: u8 = 0;
    let mut best_height: u8 = 0;

    if ICONDIRENTRY_SIZE * (id_count - 1) + ICONDIR_SIZE > icon_data.len() {
        return None;
    }

    let mut best_bytes_in_res = 0u32;
    let mut best_bit_depth = 0u32;
    let mut best_image_offset = 0u32;
    for index in 0..id_count {
        let start = 6 + index * ICONDIRENTRY_SIZE;
        let entry = ICONDIRENTRY::read(&icon_data[start..start + ICONDIRENTRY_SIZE]);
        let mut update_best_fit = false;
        let mut icon_bit_depth: u32;
        if entry.bColorCount != 0 {
            icon_bit_depth = 4;
            if entry.bColorCount < 0x10 {
                icon_bit_depth = 1;
            }
        } else {
            icon_bit_depth = u32::from(entry.wBitCount);
        }

        // If it looks like if nothing is specified at this point then set the bits per pixel to 8.
        if icon_bit_depth == 0 {
            icon_bit_depth = 8;
        }

        //  Windows rules for specifying an icon:
        //
        //  1.  The icon with the closest size match.
        //  2.  For matching sizes, the image with the closest bit depth.
        //  3.  If there is no color depth match, the icon with the closest color depth that does not exceed the display.
        //  4.  If all icon color depth > display, lowest color depth is chosen.
        //  5.  color depth of > 8bpp are all equal.
        //  6.  Never choose an 8bpp icon on an 8bpp system.

        if best_bytes_in_res == 0 {
            update_best_fit = true;
        } else {
            let best_delta = (i32::from(best_width) - size.width).abs() + (i32::from(best_height) - size.height).abs();
            let this_delta = (i32::from(entry.bWidth) - size.width).abs() + (i32::from(entry.bHeight) - size.height).abs();

            if (this_delta < best_delta)
                || (this_delta == best_delta
                    && (icon_bit_depth <= bit_depth && icon_bit_depth > best_bit_depth
                        || best_bit_depth > bit_depth && icon_bit_depth < best_bit_depth))
            {
                update_best_fit = true;
            }
        }

        if update_best_fit {
            best_width = entry.bWidth;
            best_height = entry.bHeight;
            best_image_offset = entry.dwImageOffset;
            best_bytes_in_res = entry.dwBytesInRes;
            best_bit_depth = icon_bit_depth;
        }
    }

    if best_image_offset > i32::MAX as u32 || best_bytes_in_res > i32::MAX as u32 {
        return None;
    }

    let end_offset = best_image_offset.checked_add(best_bytes_in_res)?;

    if end_offset as usize > icon_data.len() {
        return None;
    }

    Some(BestIconImage {
        image_offset: best_image_offset,
        bytes_in_res: best_bytes_in_res,
        size: PixelSize::new(i32::from(best_width), i32::from(best_height)),
    })
}

/// The number of bytes of a row of `width` pixels of `bits_per_pixel` bits.
pub(crate) fn framebuffer_stride(width: i32, bits_per_pixel: i32) -> i32 {
    (width * bits_per_pixel + 7) / 8
}

/// The mask bitmap of an icon made from a bitmap: one bit per pixel, rows
/// of `framebuffer_stride(width, 1)` bytes.
///
/// `source` is the pixels of the bitmap as BGRA without premultiplied
/// alpha, rows of `source_row_bytes` bytes, or `None` for a bitmap without
/// an alpha channel, whose mask has every bit set.
///
/// As the reference does, a bit is set for a pixel whose first byte (its
/// blue) is zero, and the bits of a byte are filled from the lowest.
pub(crate) fn alpha_to_mask(source: Option<(&[u8], usize)>, width: i32, height: i32) -> Vec<u8> {
    let row_bytes = framebuffer_stride(width, 1) as usize;
    let Some((source, source_row_bytes)) = source else {
        return vec![0xff; row_bytes * height as usize];
    };

    let mut mask = vec![0u8; row_bytes * height as usize];
    for y in 0..height as usize {
        let p_source = &source[y * source_row_bytes..];
        let p_dest = &mut mask[y * row_bytes..];
        for x in 0..width as usize {
            if p_source[x * 4] == 0 {
                p_dest[x / 8] |= 1 << (x % 8);
            }
        }
    }
    mask
}

#[cfg(windows)]
pub(crate) use imp::Win32Icon;

#[cfg(windows)]
mod imp {
    use super::*;
    use crate::interop::unmanaged_methods::{
        create_bitmap, create_icon_from_resource_ex, create_icon_indirect, delete_object, destroy_icon, get_last_error,
        get_system_metrics, screen_bit_depth, SystemMetric,
    };
    use ferroui_base::media::imaging::Bitmap;
    use ferroui_base::platform::{AlphaFormat, ILockedFramebuffer, PixelFormat};
    use ferroui_base::{PixelPoint, Vector};
    use std::cell::{Cell, RefCell};
    use std::io;
    use std::sync::Arc;

    const GDI_ERROR: &str = "A GDI+ error occurred.";
    /// `ERROR_NOT_ENOUGH_MEMORY`.
    const ERROR_NOT_ENOUGH_MEMORY: u32 = 8;

    thread_local! {
        static BIT_DEPTH: Cell<u32> = const { Cell::new(0) };
    }

    /// Pixel memory the pixels of a bitmap are copied to.
    struct Framebuffer {
        data: RefCell<Vec<u8>>,
        size: PixelSize,
        row_bytes: i32,
        format: PixelFormat,
        alpha_format: AlphaFormat,
    }

    impl ILockedFramebuffer for Framebuffer {
        fn address(&self) -> *mut u8 {
            self.data.borrow_mut().as_mut_ptr()
        }

        fn with_data(&self, access: &mut dyn FnMut(&mut [u8])) {
            access(&mut self.data.borrow_mut());
        }

        fn size(&self) -> PixelSize {
            self.size
        }

        fn row_bytes(&self) -> i32 {
            self.row_bytes
        }

        fn dpi(&self) -> Vector {
            Vector::new(96.0, 96.0)
        }

        fn format(&self) -> PixelFormat {
            self.format
        }

        fn alpha_format(&self) -> AlphaFormat {
            self.alpha_format
        }

        fn dispose(&self) {}
    }

    /// # Panics
    /// Panics for a size without pixels.
    fn alloc_framebuffer(size: PixelSize, format: PixelFormat, alpha_format: AlphaFormat) -> Framebuffer {
        if size.width < 1 || size.height < 1 {
            panic!("the size of an icon bitmap is out of range: {size:?}");
        }

        let stride = framebuffer_stride(size.width, format.bits_per_pixel() as i32);
        Framebuffer {
            data: RefCell::new(vec![0u8; (size.height * stride) as usize]),
            size,
            row_bytes: stride,
            format,
            alpha_format,
        }
    }

    /// An icon of the system, destroyed when it is disposed or dropped.
    pub(crate) struct Win32Icon {
        handle: Cell<isize>,
        size: PixelSize,
        bytes: Option<Arc<[u8]>>,
    }

    impl Win32Icon {
        /// An icon (a cursor, with its hot spot) from a bitmap.
        ///
        /// # Panics
        /// Panics when the system cannot create the bitmaps or the icon.
        pub(crate) fn from_bitmap(bitmap: &Bitmap, hot_spot: PixelPoint) -> Win32Icon {
            Win32Icon { handle: Cell::new(Self::create_icon(bitmap, hot_spot)), size: PixelSize::default(), bytes: None }
        }

        /// An icon from the data of an icon file, of the image that fits
        /// `size` best (a zero stands for the icon size of the system); or,
        /// for data that is not an icon file, from the image it decodes to.
        pub(crate) fn from_data(icon_data: Arc<[u8]>, size: PixelSize) -> io::Result<Win32Icon> {
            let (mut handle, size) = Self::load_icon_from_data(&icon_data, Self::replace_zeroes_with_system_metrics(size));
            if handle == 0 {
                let bmp = Bitmap::from_stream(&mut io::Cursor::new(&icon_data[..]))?;
                handle = Self::create_icon(&bmp, PixelPoint::default());
                bmp.dispose();
            }
            Ok(Win32Icon { handle: Cell::new(handle), size, bytes: Some(icon_data) })
        }

        /// An icon of another size from the data of `original`.
        ///
        /// # Panics
        /// Panics when `original` was created from a bitmap.
        pub(crate) fn from_icon(original: &Win32Icon, size: PixelSize) -> io::Result<Win32Icon> {
            let Some(bytes) = original.bytes.clone() else {
                panic!("Original icon was created from a bitmap and cannot be copied.");
            };
            Self::from_data(bytes, size)
        }

        pub(crate) fn handle(&self) -> isize {
            self.handle.get()
        }

        pub(crate) fn size(&self) -> PixelSize {
            self.size
        }

        fn create_icon(bitmap: &Bitmap, hot_spot: PixelPoint) -> isize {
            let main_bitmap = Self::create_h_bitmap(bitmap);
            if main_bitmap == 0 {
                panic!("{GDI_ERROR}");
            }
            let alpha_bitmap = Self::alpha_to_mask(bitmap);

            let h_icon = if alpha_bitmap == 0 {
                None
            } else {
                Some(create_icon_indirect(false, hot_spot.x, hot_spot.y, alpha_bitmap, main_bitmap))
            };
            let error = get_last_error();
            delete_object(main_bitmap);
            delete_object(alpha_bitmap);

            match h_icon {
                None => panic!("{GDI_ERROR}"),
                Some(0) => panic!("The icon could not be created (error code {error})"),
                Some(h_icon) => h_icon,
            }
        }

        fn create_h_bitmap(source: &Bitmap) -> isize {
            let fb = alloc_framebuffer(source.pixel_size(), PixelFormat::BGRA8888, AlphaFormat::Unpremul);
            source.copy_pixels_to_framebuffer(&fb);
            let data = fb.data.borrow();
            create_bitmap(source.pixel_size().width, source.pixel_size().height, 1, 32, &data)
        }

        fn alpha_to_mask(source: &Bitmap) -> isize {
            let size = source.pixel_size();
            if size.width < 1 || size.height < 1 {
                panic!("the size of an icon bitmap is out of range: {size:?}");
            }

            let mask = if !source.format().is_some_and(|format| format.has_alpha()) {
                super::alpha_to_mask(None, size.width, size.height)
            } else {
                let argb_buffer = alloc_framebuffer(size, PixelFormat::BGRA8888, AlphaFormat::Unpremul);
                source.copy_pixels_to_framebuffer(&argb_buffer);
                let data = argb_buffer.data.borrow();
                super::alpha_to_mask(Some((&data, argb_buffer.row_bytes as usize)), size.width, size.height)
            };

            create_bitmap(size.width, size.height, 1, 1, &mask)
        }

        fn replace_zeroes_with_system_metrics(pixel_size: PixelSize) -> PixelSize {
            PixelSize::new(
                if pixel_size.width == 0 { get_system_metrics(SystemMetric::SM_CXICON) } else { pixel_size.width },
                if pixel_size.height == 0 { get_system_metrics(SystemMetric::SM_CYICON) } else { pixel_size.height },
            )
        }

        /// # Panics
        /// Panics when the system is out of memory for the icon.
        fn load_icon_from_data(icon_data: &[u8], size: PixelSize) -> (isize, PixelSize) {
            if icon_data.len() < ICONDIR_SIZE {
                return (0, PixelSize::default());
            }

            let mut bit_depth = BIT_DEPTH.get();
            if bit_depth == 0 {
                bit_depth = screen_bit_depth();

                // If the bitdepth is 8, make it 4 because windows does not
                // choose a 256 color icon if the display is running in 256 color mode
                // due to palette flicker.
                if bit_depth == 8 {
                    bit_depth = 4;
                }
                BIT_DEPTH.set(bit_depth);
            }

            let Some(best) = find_best_icon_image(icon_data, size, bit_depth) else {
                return (0, PixelSize::default());
            };

            // The reference copies the bytes into an aligned buffer when
            // the image starts at an offset that is not aligned; a copy is
            // always aligned.
            let start = best.image_offset as usize;
            let image = icon_data[start..start + best.bytes_in_res as usize].to_vec();
            let handle = create_icon_from_resource_ex(&image, true, 0x0003_0000, 0, 0, 0);

            if handle == 0 {
                let error = get_last_error();
                // If there we are out of GDI handles then our fallback won't work either, so throw now.
                if error == ERROR_NOT_ENOUGH_MEMORY {
                    panic!("The icon could not be created (error code {error})");
                }
            }

            (handle, best.size)
        }

        /// Writes the data the icon was created from.
        ///
        /// # Panics
        /// Panics when the icon was created from a bitmap.
        pub(crate) fn copy_to(&self, stream: &mut dyn io::Write) -> io::Result<()> {
            let Some(bytes) = &self.bytes else {
                panic!("Icon was created from a bitmap, not Win32 icon data");
            };
            stream.write_all(bytes)
        }

        pub(crate) fn dispose(&self) {
            let handle = self.handle.replace(0);
            if handle != 0 {
                destroy_icon(handle);
            }
        }
    }

    impl Drop for Win32Icon {
        fn drop(&mut self) {
            self.dispose();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// An icon file with a directory of the entries given (width, height,
    /// colour count, bit count), each image four bytes long.
    fn icon_file(entries: &[(u8, u8, u8, u16)]) -> Vec<u8> {
        let mut data = vec![0, 0, 1, 0];
        data.extend_from_slice(&(entries.len() as u16).to_le_bytes());
        let images = 6 + entries.len() * ICONDIRENTRY_SIZE;
        for (index, (width, height, color_count, bit_count)) in entries.iter().enumerate() {
            data.extend_from_slice(&[*width, *height, *color_count, 0, 1, 0]);
            data.extend_from_slice(&bit_count.to_le_bytes());
            data.extend_from_slice(&4u32.to_le_bytes());
            data.extend_from_slice(&((images + index * 4) as u32).to_le_bytes());
        }
        for index in 0..entries.len() {
            data.extend_from_slice(&[index as u8; 4]);
        }
        data
    }

    fn best(data: &[u8], size: i32, bit_depth: u32) -> Option<(i32, u32)> {
        find_best_icon_image(data, PixelSize::new(size, size), bit_depth).map(|best| (best.size.width, best.image_offset))
    }

    #[test]
    fn the_image_with_the_closest_size_is_chosen() {
        let data = icon_file(&[(16, 16, 0, 32), (32, 32, 0, 32), (48, 48, 0, 32)]);
        let images = 6 + 3 * 16;
        assert_eq!(best(&data, 16, 32), Some((16, images as u32)));
        assert_eq!(best(&data, 32, 32), Some((32, images as u32 + 4)));
        assert_eq!(best(&data, 40, 32), Some((32, images as u32 + 4)));
        assert_eq!(best(&data, 41, 32), Some((48, images as u32 + 8)));
        // Of two images as close, the first stays.
        assert_eq!(best(&data, 24, 32), Some((16, images as u32)));
        // An image of 256 pixels states 0: far from every size asked for.
        let with_large = icon_file(&[(0, 0, 0, 32), (64, 64, 0, 32)]);
        assert_eq!(best(&with_large, 256, 32).map(|(width, _)| width), Some(64));
    }

    #[test]
    fn of_images_of_one_size_the_closest_bit_depth_is_chosen() {
        let images = (6 + 3 * 16) as u32;
        let data = icon_file(&[(32, 32, 16, 0), (32, 32, 0, 8), (32, 32, 0, 32)]);
        // A display of 32 bits: the deepest image that does not exceed it.
        assert_eq!(best(&data, 32, 32), Some((32, images + 8)));
        // A display of 4 bits (what a display of 8 counts as): the image of
        // 16 colours; the deeper ones exceed the display.
        assert_eq!(best(&data, 32, 4), Some((32, images)));
        // Every image exceeds the display: the lowest depth.
        let deep = icon_file(&[(32, 32, 0, 32), (32, 32, 0, 24)]);
        assert_eq!(best(&deep, 32, 4), Some((32, (6 + 2 * 16) as u32 + 4)));
        // A colour count under 16 is one bit; no count and no depth is 8.
        let low = icon_file(&[(32, 32, 0, 0), (32, 32, 2, 0)]);
        assert_eq!(best(&low, 32, 1), Some((32, (6 + 2 * 16) as u32 + 4)));
    }

    #[test]
    fn data_that_is_not_an_icon_file_has_no_image() {
        assert_eq!(best(&[], 16, 32), None);
        assert_eq!(best(&[0; 21], 16, 32), None);
        let mut data = icon_file(&[(16, 16, 0, 32)]);
        assert!(best(&data, 16, 32).is_some());
        // A reserved word, a type that is not an icon, no entries.
        for (index, value) in [(0, 1), (2, 2), (4, 0)] {
            let mut changed = data.clone();
            changed[index] = value;
            assert_eq!(best(&changed, 16, 32), None, "byte {index}");
        }
        // More entries than the data holds.
        let mut many = data.clone();
        many[4] = 9;
        assert_eq!(best(&many, 16, 32), None);
        // An image that ends after the data, and one whose end overflows.
        data[14..18].copy_from_slice(&5u32.to_le_bytes());
        assert_eq!(best(&data, 16, 32), None);
        data[14..18].copy_from_slice(&u32::MAX.to_le_bytes());
        assert_eq!(best(&data, 16, 32), None);
        // A PNG file is not an icon file.
        let png = [0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a, 0, 0, 0, 13, b'I', b'H', b'D', b'R', 0, 0, 0, 1, 0, 0];
        assert_eq!(best(&png, 16, 32), None);
    }

    #[test]
    fn the_rows_of_a_framebuffer_are_whole_bytes() {
        assert_eq!(framebuffer_stride(16, 32), 64);
        assert_eq!(framebuffer_stride(16, 1), 2);
        assert_eq!(framebuffer_stride(17, 1), 3);
        assert_eq!(framebuffer_stride(1, 1), 1);
    }

    #[test]
    fn the_mask_of_a_bitmap_without_alpha_has_every_bit_set() {
        assert_eq!(alpha_to_mask(None, 9, 2), vec![0xff; 4]);
    }

    #[test]
    fn the_mask_marks_the_pixels_whose_first_byte_is_zero() {
        // Two rows of nine pixels; rows of the source are 40 bytes long.
        let mut source = vec![0xffu8; 80];
        source[0] = 0; // row 0, pixel 0
        source[3 * 4] = 0; // row 0, pixel 3
        source[8 * 4] = 0; // row 0, pixel 8
        source[40 + 7 * 4] = 0; // row 1, pixel 7
        let mask = alpha_to_mask(Some((&source, 40)), 9, 2);
        assert_eq!(mask, vec![0b0000_1001, 0b0000_0001, 0b1000_0000, 0]);
    }
}
