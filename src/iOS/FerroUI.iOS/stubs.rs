//! The services of a windowing platform iOS has no counterpart for:
//! cursors, windows and window icons.

use ferroui_base::input::StandardCursorType;
use ferroui_base::media::imaging::{Bitmap, BitmapEncoderOptions, PngBitmapEncoderOptions};
use ferroui_base::platform::{ICursorFactory, ICursorImpl, SharedBitmapImpl};
use ferroui_base::PixelPoint;
use ferroui_controls::platform::{
    IPlatformIconLoader, ITopLevelImpl, ITrayIconImpl, IWindowIconImpl, IWindowImpl, IWindowingPlatform,
};
use std::any::Any;
use std::io;
use std::rc::Rc;
use std::sync::Arc;

/// A cursor factory whose cursors are nothing: iOS shows no cursor of an
/// application.
#[derive(Default)]
pub struct CursorFactoryStub;

struct CursorImplStub;

impl ICursorImpl for CursorImplStub {
    fn dispose(&self) {}

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl ICursorFactory for CursorFactoryStub {
    fn get_cursor(&self, _cursor_type: StandardCursorType) -> Rc<dyn ICursorImpl> {
        Rc::new(CursorImplStub)
    }

    fn create_cursor(&self, _cursor: &Bitmap, _hot_spot: PixelPoint) -> Rc<dyn ICursorImpl> {
        Rc::new(CursorImplStub)
    }
}

/// A windowing platform without windows: an iOS application shows views.
#[derive(Default)]
pub struct WindowingPlatformStub;

impl IWindowingPlatform for WindowingPlatformStub {
    fn create_window(&self) -> Rc<dyn IWindowImpl> {
        panic!("Specified method is not supported.");
    }

    fn create_embeddable_top_level(&self) -> Rc<dyn ITopLevelImpl> {
        panic!("Specified method is not supported.");
    }

    fn create_embeddable_window(&self) -> Rc<dyn IWindowImpl> {
        panic!("Specified method is not supported.");
    }

    fn create_tray_icon(&self) -> Option<Rc<dyn ITrayIconImpl>> {
        None
    }

    fn get_windows_z_order(&self, _windows: &[Rc<dyn IWindowImpl>], _z_order: &mut [i64]) {
        panic!("Specified method is not supported.");
    }
}

/// An icon loader whose icons keep the bytes they were loaded from.
#[derive(Default)]
pub struct PlatformIconLoaderStub;

impl IPlatformIconLoader for PlatformIconLoaderStub {
    fn load_icon_from_file(&self, file_name: &str) -> io::Result<Rc<dyn IWindowIconImpl>> {
        let mut file = std::fs::File::open(file_name)?;
        self.load_icon_from_stream(&mut file)
    }

    fn load_icon_from_stream(&self, stream: &mut dyn io::Read) -> io::Result<Rc<dyn IWindowIconImpl>> {
        let mut ms = Vec::new();
        stream.read_to_end(&mut ms)?;
        Ok(Rc::new(IconStub::new(ms)))
    }

    fn load_icon_from_bitmap(&self, bitmap: Arc<SharedBitmapImpl>) -> Rc<dyn IWindowIconImpl> {
        let mut stream = Vec::new();
        if let Err(error) = bitmap.save(&mut stream, &BitmapEncoderOptions::Png(PngBitmapEncoderOptions::DEFAULT)) {
            panic!("Unable to save the icon bitmap: {error}");
        }
        Rc::new(IconStub::new(stream))
    }
}

/// An icon that is the bytes it was loaded from.
pub struct IconStub {
    ms: Vec<u8>,
}

impl IconStub {
    /// Creates the icon over the bytes of a stream.
    pub fn new(stream: Vec<u8>) -> Self {
        Self { ms: stream }
    }
}

impl IWindowIconImpl for IconStub {
    fn save(&self, output_stream: &mut dyn io::Write) -> io::Result<()> {
        output_stream.write_all(&self.ms)
    }
}

#[cfg(test)]
mod tests {
    // Not from the reference, which has no tests of this file.
    use super::*;

    #[test]
    fn an_icon_saves_the_bytes_it_was_loaded_from() {
        let loader = PlatformIconLoaderStub;
        let icon = loader.load_icon_from_stream(&mut io::Cursor::new(vec![1, 2, 3])).unwrap();

        let mut output = Vec::new();
        icon.save(&mut output).unwrap();
        icon.save(&mut output).unwrap();

        assert_eq!(vec![1, 2, 3, 1, 2, 3], output);
    }

    #[test]
    fn a_file_that_does_not_exist_is_an_error() {
        assert!(PlatformIconLoaderStub.load_icon_from_file("/nonexistent/icon.png").is_err());
    }

    #[test]
    fn the_platform_has_no_tray_icon() {
        assert!(WindowingPlatformStub.create_tray_icon().is_none());
    }

    #[test]
    #[should_panic(expected = "Specified method is not supported.")]
    fn the_platform_creates_no_window() {
        let _ = WindowingPlatformStub.create_window();
    }

    #[test]
    fn every_cursor_is_a_cursor_that_is_nothing() {
        let cursor = CursorFactoryStub.get_cursor(StandardCursorType::Hand);
        cursor.dispose();
    }
}
