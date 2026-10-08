use super::IWindowIconImpl;
use ferroui_base::platform::IBitmapImpl;
use std::io;
use std::rc::Rc;

/// Loads window icons for the platform.
pub trait IPlatformIconLoader {
    /// Loads an icon from a file.
    fn load_icon_from_file(&self, file_name: &str) -> io::Result<Rc<dyn IWindowIconImpl>>;

    /// Loads an icon from a stream.
    fn load_icon_from_stream(&self, stream: &mut dyn io::Read) -> io::Result<Rc<dyn IWindowIconImpl>>;

    /// Creates an icon from a bitmap.
    fn load_icon_from_bitmap(&self, bitmap: std::sync::Arc<ferroui_base::platform::SharedBitmapImpl>) -> Rc<dyn IWindowIconImpl>;
}
