//! Window icons (the port of `X11IconLoader.cs`): an icon is the data of
//! the `_NET_WM_ICON` property.

use crate::pixel_buffer::PixelBuffer;
use ferroui_base::media::imaging::{
    Bitmap, BitmapEncoderOptions, PngBitmapEncoderOptions, RenderTargetBitmap, WriteableBitmap,
};
use ferroui_base::platform::{PixelFormat, SharedBitmapImpl};
use ferroui_base::{PixelSize, Rect, Vector};
use ferroui_controls::platform::{IPlatformIconLoader, IWindowIconImpl};
use std::cell::RefCell;
use std::ffi::c_ulong;
use std::io;
use std::rc::{Rc, Weak};
use std::sync::Arc;

/// The icon loader of the X11 platform.
#[derive(Default)]
pub struct X11IconLoader;

impl X11IconLoader {
    fn load_icon(bitmap: Bitmap) -> Rc<dyn IWindowIconImpl> {
        let rv = Rc::new(X11IconData::new(&bitmap));
        bitmap.dispose();
        X11IconData::register(&rv);
        rv
    }
}

impl IPlatformIconLoader for X11IconLoader {
    fn load_icon_from_file(&self, file_name: &str) -> io::Result<Rc<dyn IWindowIconImpl>> {
        Ok(Self::load_icon(Bitmap::from_file(file_name)?))
    }

    fn load_icon_from_stream(&self, stream: &mut dyn io::Read) -> io::Result<Rc<dyn IWindowIconImpl>> {
        Ok(Self::load_icon(Bitmap::from_stream(stream)?))
    }

    fn load_icon_from_bitmap(&self, bitmap: Arc<SharedBitmapImpl>) -> Rc<dyn IWindowIconImpl> {
        let mut ms = Vec::new();
        if let Err(error) = bitmap.save(&mut ms, &BitmapEncoderOptions::Png(PngBitmapEncoderOptions::DEFAULT)) {
            panic!("Unable to save the icon bitmap: {error}");
        }
        match self.load_icon_from_stream(&mut io::Cursor::new(ms)) {
            Ok(icon) => icon,
            Err(error) => panic!("Unable to load the icon bitmap: {error}"),
        }
    }
}

thread_local! {
    /// The icons this loader made that are still alive.
    static ICONS: RefCell<Vec<Weak<X11IconData>>> = const { RefCell::new(Vec::new()) };
}

/// The largest side of an icon, in pixels: a larger bitmap is scaled down.
const MAX_ICON_SIDE: i32 = 128;

/// An icon as the items of a `_NET_WM_ICON` property: the width, the
/// height, and the pixels as premultiplied ARGB values.
pub struct X11IconData {
    width: i32,
    height: i32,
    data: Vec<c_ulong>,
}

impl X11IconData {
    pub fn new(bitmap: &Bitmap) -> Self {
        let width = bitmap.pixel_size().width.min(MAX_ICON_SIDE);
        let height = bitmap.pixel_size().height.min(MAX_ICON_SIDE);
        let size = PixelSize::new(width, height);

        let rtb = RenderTargetBitmap::new(size);
        {
            let mut ctx = rtb.create_drawing_context_with_clear(true);
            let rtb_size = rtb.size();
            ctx.draw_image(bitmap, Rect::new(0.0, 0.0, rtb_size.width, rtb_size.height));
            ctx.dispose();
        }

        let buffer = PixelBuffer::new(size);
        rtb.copy_pixels_to_framebuffer(&buffer);
        rtb.dispose();

        Self::from_pixels(width, height, &buffer.into_pixels())
    }

    /// The icon of pixels in rows of `width`.
    pub(crate) fn from_pixels(width: i32, height: i32, pixels: &[u32]) -> Self {
        let count = width.max(0) as usize * height.max(0) as usize;
        let mut data = Vec::with_capacity(count + 2);
        data.push(width as u32 as c_ulong);
        data.push(height as u32 as c_ulong);
        data.extend(pixels[..count].iter().map(|pixel| *pixel as c_ulong));
        Self { width, height, data }
    }

    /// The items of the property.
    pub fn data(&self) -> &[c_ulong] {
        &self.data
    }

    fn register(icon: &Rc<X11IconData>) {
        ICONS.with(|icons| {
            let mut icons = icons.borrow_mut();
            icons.retain(|known| known.strong_count() > 0);
            icons.push(Rc::downgrade(icon));
        });
    }

    /// The icon data of an icon of the contract (the cast of the
    /// reference, `(X11IconData)icon`).
    ///
    /// The contract of an icon has no way to ask for its concrete type, so
    /// the icons this loader made are remembered and found again by
    /// identity. An icon of another origin, which the reference fails to
    /// cast, is converted through its encoded form.
    pub fn from_icon_impl(icon: &Rc<dyn IWindowIconImpl>) -> io::Result<Rc<X11IconData>> {
        let address = Rc::as_ptr(icon).cast::<()>();
        let known = ICONS.with(|icons| {
            icons.borrow().iter().filter_map(Weak::upgrade).find(|known| Rc::as_ptr(known).cast::<()>() == address)
        });
        if let Some(known) = known {
            return Ok(known);
        }

        let mut encoded = Vec::new();
        icon.save(&mut encoded)?;
        let bitmap = Bitmap::from_stream(&mut io::Cursor::new(encoded))?;
        let data = Rc::new(X11IconData::new(&bitmap));
        bitmap.dispose();
        Ok(data)
    }

    /// The pixels as bytes of four a pixel, in the byte order of the
    /// machine.
    fn pixel_bytes(&self) -> Vec<u8> {
        self.data[2..].iter().flat_map(|pixel| (*pixel as u32).to_ne_bytes()).collect()
    }
}

impl IWindowIconImpl for X11IconData {
    fn save(&self, output_stream: &mut dyn io::Write) -> io::Result<()> {
        let wr = WriteableBitmap::new(
            PixelSize::new(self.width, self.height),
            Vector::new(96.0, 96.0),
            Some(PixelFormat::BGRA8888),
            None,
        );
        {
            let fb = wr.lock();
            let row_bytes = fb.row_bytes().max(0) as usize;
            let source = self.pixel_bytes();
            let source_row_bytes = self.width.max(0) as usize * 4;
            fb.with_data(&mut |dest| {
                for y in 0..self.height.max(0) as usize {
                    let source_row = &source[y * source_row_bytes..(y + 1) * source_row_bytes];
                    dest[y * row_bytes..y * row_bytes + source_row_bytes].copy_from_slice(source_row);
                }
            });
            fb.dispose();
        }
        let result = wr.save(output_stream, &BitmapEncoderOptions::Png(PngBitmapEncoderOptions::DEFAULT));
        wr.dispose();
        result
    }
}

#[cfg(test)]
mod tests {
    // Not from the reference, which has no tests for this file.
    use super::*;
    use crate::pixel_buffer::bytes_to_pixels;

    #[test]
    fn the_property_data_is_the_size_and_then_the_pixels() {
        let pixels = [0xff00_0000u32, 0x80ff_0000, 0x0000_0000, 0xffff_ffff, 0x1122_3344, 0x5566_7788];
        let icon = X11IconData::from_pixels(3, 2, &pixels);
        assert_eq!(icon.data().len(), 8);
        assert_eq!(&icon.data()[..2], &[3, 2]);
        assert_eq!(icon.data()[2], 0xff00_0000);
        assert_eq!(icon.data()[7], 0x5566_7788);
        // The high bits of an item stay clear, whatever the size of a long is.
        assert!(icon.data().iter().all(|item| *item <= u32::MAX as c_ulong));
        assert_eq!(bytes_to_pixels(&icon.pixel_bytes()), pixels);
    }

    #[test]
    fn an_icon_of_the_loader_is_found_again_by_identity() {
        let icon = Rc::new(X11IconData::from_pixels(1, 1, &[0xff12_3456]));
        X11IconData::register(&icon);
        let other = Rc::new(X11IconData::from_pixels(1, 1, &[0xff65_4321]));
        X11IconData::register(&other);
        let handle: Rc<dyn IWindowIconImpl> = icon.clone();
        let found = X11IconData::from_icon_impl(&handle).expect("a registered icon");
        assert!(Rc::ptr_eq(&found, &icon));
        assert_eq!(found.data()[2], 0xff12_3456);
    }

    #[test]
    fn an_empty_icon_is_only_its_size() {
        let icon = X11IconData::from_pixels(0, 0, &[]);
        assert_eq!(icon.data(), &[0, 0]);
        assert!(icon.pixel_bytes().is_empty());
    }
}
