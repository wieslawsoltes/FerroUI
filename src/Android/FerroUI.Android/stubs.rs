use ferroui_base::media::imaging::{BitmapEncoderOptions, PngBitmapEncoderOptions};
use ferroui_base::platform::SharedBitmapImpl;
use ferroui_controls::platform::{
    IPlatformIconLoader, ITopLevelImpl, ITrayIconImpl, IWindowIconImpl, IWindowImpl, IWindowingPlatform,
};
use std::fs::File;
use std::io;
use std::rc::Rc;
use std::sync::Arc;

/// The windowing platform of Android. There are no windows: content is
/// shown by a view in an activity.
#[derive(Default)]
pub struct WindowingPlatformStub;

impl IWindowingPlatform for WindowingPlatformStub {
    fn create_window(&self) -> Rc<dyn IWindowImpl> {
        panic!("Specified method is not supported.");
    }

    fn create_embeddable_top_level(&self) -> Rc<dyn ITopLevelImpl> {
        self.create_embeddable_window()
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

/// The icon loader of a platform without window icons: an icon keeps the
/// bytes it was loaded from, so that it can be saved.
#[derive(Default)]
pub struct PlatformIconLoaderStub;

impl IPlatformIconLoader for PlatformIconLoaderStub {
    fn load_icon_from_file(&self, file_name: &str) -> io::Result<Rc<dyn IWindowIconImpl>> {
        let mut file = File::open(file_name)?;
        self.load_icon_from_stream(&mut file)
    }

    fn load_icon_from_stream(&self, stream: &mut dyn io::Read) -> io::Result<Rc<dyn IWindowIconImpl>> {
        let mut ms = Vec::new();
        stream.read_to_end(&mut ms)?;
        Ok(Rc::new(IconStub::new(ms)))
    }

    fn load_icon_from_bitmap(&self, bitmap: Arc<SharedBitmapImpl>) -> Rc<dyn IWindowIconImpl> {
        let mut stream = Vec::new();
        // A bitmap that cannot be encoded gives an icon without bytes: the contract has no
        // failure for this member, and the icon is never shown.
        if bitmap.save(&mut stream, &BitmapEncoderOptions::Png(PngBitmapEncoderOptions::DEFAULT)).is_err() {
            stream.clear();
        }
        Rc::new(IconStub::new(stream))
    }
}

pub(crate) struct IconStub {
    ms: Vec<u8>,
}

impl IconStub {
    pub(crate) fn new(stream: Vec<u8>) -> Self {
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
    // Not from the reference, which has no tests of the stubs.
    use super::*;

    #[test]
    fn there_is_no_tray_icon() {
        assert!(WindowingPlatformStub.create_tray_icon().is_none());
    }

    #[test]
    #[should_panic(expected = "Specified method is not supported")]
    fn windows_cannot_be_created() {
        WindowingPlatformStub.create_window();
    }

    #[test]
    #[should_panic(expected = "Specified method is not supported")]
    fn embeddable_top_levels_cannot_be_created() {
        WindowingPlatformStub.create_embeddable_top_level();
    }

    #[test]
    #[should_panic(expected = "Specified method is not supported")]
    fn the_z_order_of_windows_is_not_available() {
        WindowingPlatformStub.get_windows_z_order(&[], &mut []);
    }

    #[test]
    fn an_icon_of_a_stream_saves_the_bytes_of_the_stream() {
        let loader = PlatformIconLoaderStub;
        let icon = loader.load_icon_from_stream(&mut io::Cursor::new(vec![1, 2, 3])).unwrap();
        let mut output = Vec::new();

        icon.save(&mut output).unwrap();
        icon.save(&mut output).unwrap();

        assert_eq!(output, [1, 2, 3, 1, 2, 3]);
    }

    #[test]
    fn a_missing_file_is_an_error() {
        assert!(PlatformIconLoaderStub.load_icon_from_file("/nonexistent/icon.png").is_err());
    }
}
