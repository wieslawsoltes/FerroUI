use ferroui_base::media::imaging::{BitmapEncoderOptions, PngBitmapEncoderOptions};
use ferroui_base::platform::{IBitmapImpl, IPlatformRenderInterface};
use ferroui_base::{FerroLocator, LocatorExtensions};
use ferroui_controls::platform::{IPlatformIconLoader, IWindowIconImpl};
use std::io;
use std::rc::Rc;

// OSX doesn't have a concept of *window* icon.
// Icons in the title bar are only shown if there is
// an opened file (on disk) associated with the current window
// see https://stackoverflow.com/a/7038671/2231814
#[derive(Default)]
pub struct IconLoader;

struct IconStub {
    bitmap: Rc<dyn IBitmapImpl>,
}

impl IWindowIconImpl for IconStub {
    fn save(&self, output_stream: &mut dyn io::Write) -> io::Result<()> {
        self.bitmap.save(output_stream, &BitmapEncoderOptions::Png(PngBitmapEncoderOptions::DEFAULT))
    }
}

fn render_interface() -> Rc<dyn IPlatformRenderInterface> {
    FerroLocator::current().get_required_service::<dyn IPlatformRenderInterface>()
}

impl IPlatformIconLoader for IconLoader {
    fn load_icon_from_file(&self, file_name: &str) -> io::Result<Rc<dyn IWindowIconImpl>> {
        Ok(Rc::new(IconStub { bitmap: render_interface().load_bitmap_from_file(file_name)? }))
    }

    fn load_icon_from_stream(&self, stream: &mut dyn io::Read) -> io::Result<Rc<dyn IWindowIconImpl>> {
        Ok(Rc::new(IconStub { bitmap: render_interface().load_bitmap(stream)? }))
    }

    /// # Panics
    /// Panics when the bitmap cannot be encoded or decoded again, where the
    /// reference implementation throws.
    fn load_icon_from_bitmap(&self, bitmap: Rc<dyn IBitmapImpl>) -> Rc<dyn IWindowIconImpl> {
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
