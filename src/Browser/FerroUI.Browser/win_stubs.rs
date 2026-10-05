use ferroui_base::platform::IBitmapImpl;
use ferroui_controls::platform::{IPlatformIconLoader, IWindowIconImpl};
use std::io;
use std::rc::Rc;

/// The icon loader of a platform without window icons.
#[derive(Default)]
pub struct IconLoaderStub;

struct IconStub;

impl IWindowIconImpl for IconStub {
    fn save(&self, _output_stream: &mut dyn io::Write) -> io::Result<()> {
        Ok(())
    }
}

impl IPlatformIconLoader for IconLoaderStub {
    fn load_icon_from_file(&self, _file_name: &str) -> io::Result<Rc<dyn IWindowIconImpl>> {
        Ok(Rc::new(IconStub))
    }

    fn load_icon_from_stream(&self, _stream: &mut dyn io::Read) -> io::Result<Rc<dyn IWindowIconImpl>> {
        Ok(Rc::new(IconStub))
    }

    fn load_icon_from_bitmap(&self, _bitmap: Rc<dyn IBitmapImpl>) -> Rc<dyn IWindowIconImpl> {
        Rc::new(IconStub)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_source_gives_an_icon_that_saves_nothing() {
        let loader = IconLoaderStub;
        let mut output = Vec::new();

        loader.load_icon_from_file("icon.ico").unwrap().save(&mut output).unwrap();
        loader.load_icon_from_stream(&mut io::Cursor::new(vec![1, 2, 3])).unwrap().save(&mut output).unwrap();

        assert!(output.is_empty());
    }
}
