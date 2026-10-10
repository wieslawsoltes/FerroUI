use ferroui_base::media::imaging::{BitmapEncoderOptions, PngBitmapEncoderOptions};
use ferroui_base::platform::SharedBitmapImpl;
use ferroui_controls::platform::{IPlatformIconLoader, IWindowIconImpl};
use std::cell::RefCell;
use std::fs::File;
use std::io;
use std::rc::Rc;
use std::sync::Arc;

/// An icon loader for a platform that never shows an icon. The platform
/// registers [`PlatformIconLoaderStub`](crate::PlatformIconLoaderStub), as
/// the reference does; this class is kept as the reference has it.
#[derive(Default)]
pub struct PlatformIconLoader;

impl IPlatformIconLoader for PlatformIconLoader {
    fn load_icon_from_file(&self, file_name: &str) -> io::Result<Rc<dyn IWindowIconImpl>> {
        let mut file = File::open(file_name)?;
        Ok(Rc::new(FakeIcon::new(&mut file)?))
    }

    fn load_icon_from_stream(&self, stream: &mut dyn io::Read) -> io::Result<Rc<dyn IWindowIconImpl>> {
        Ok(Rc::new(FakeIcon::new(stream)?))
    }

    fn load_icon_from_bitmap(&self, bitmap: Arc<SharedBitmapImpl>) -> Rc<dyn IWindowIconImpl> {
        let mut stream = Vec::new();
        // A bitmap that cannot be encoded gives an icon without bytes: the contract has no
        // failure for this member, and the icon is never shown.
        if bitmap.save(&mut stream, &BitmapEncoderOptions::Png(PngBitmapEncoderOptions::DEFAULT)).is_err() {
            stream.clear();
        }
        // As the reference, which hands over the stream at its end: the icon has no bytes.
        Rc::new(FakeIcon { stream: RefCell::new(io::Cursor::new(Vec::new())) })
    }
}

/// Stores the icon created as a stream to support saving even though an
/// icon is never shown.
///
/// As in the reference, the stream is read from where it stands: the icon
/// saves its bytes once, because nothing rewinds its stream.
pub(crate) struct FakeIcon {
    stream: RefCell<io::Cursor<Vec<u8>>>,
}

impl FakeIcon {
    pub(crate) fn new(stream: &mut dyn io::Read) -> io::Result<Self> {
        let mut bytes = Vec::new();
        stream.read_to_end(&mut bytes)?;
        // The copy leaves the stream of the icon at its end.
        let mut cursor = io::Cursor::new(bytes);
        cursor.set_position(cursor.get_ref().len() as u64);
        Ok(Self { stream: RefCell::new(cursor) })
    }
}

impl IWindowIconImpl for FakeIcon {
    fn save(&self, output_stream: &mut dyn io::Write) -> io::Result<()> {
        io::copy(&mut *self.stream.borrow_mut(), output_stream).map(|_| ())
    }
}

#[cfg(test)]
mod tests {
    // Not from the reference, which has no tests of the loader.
    use super::*;

    #[test]
    fn an_icon_saves_what_is_left_of_its_stream_which_is_nothing() {
        let icon = PlatformIconLoader.load_icon_from_stream(&mut io::Cursor::new(vec![1, 2, 3])).unwrap();
        let mut output = Vec::new();

        icon.save(&mut output).unwrap();

        assert!(output.is_empty());
    }
}
