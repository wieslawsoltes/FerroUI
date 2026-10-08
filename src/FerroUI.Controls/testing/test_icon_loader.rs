use crate::platform::{IPlatformIconLoader, IWindowIconImpl};
use std::io;
use std::rc::Rc;

/// An icon loader for tests (see
/// [`TestServices::with_icon_loader`](super::TestServices::with_icon_loader)):
/// an icon saves the bytes it was loaded from. An icon loaded from a file
/// saves the name of the file, and an icon created from a bitmap saves
/// nothing.
#[derive(Default)]
pub struct TestIconLoader;

struct TestIconImpl(Vec<u8>);

impl IWindowIconImpl for TestIconImpl {
    fn save(&self, output_stream: &mut dyn io::Write) -> io::Result<()> {
        output_stream.write_all(&self.0)
    }
}

impl IPlatformIconLoader for TestIconLoader {
    fn load_icon_from_file(&self, file_name: &str) -> io::Result<Rc<dyn IWindowIconImpl>> {
        Ok(Rc::new(TestIconImpl(file_name.as_bytes().to_vec())))
    }

    fn load_icon_from_stream(&self, stream: &mut dyn io::Read) -> io::Result<Rc<dyn IWindowIconImpl>> {
        let mut data = Vec::new();
        stream.read_to_end(&mut data)?;
        Ok(Rc::new(TestIconImpl(data)))
    }

    fn load_icon_from_bitmap(&self, _bitmap: std::sync::Arc<ferroui_base::platform::SharedBitmapImpl>) -> Rc<dyn IWindowIconImpl> {
        Rc::new(TestIconImpl(Vec::new()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::{TestServices, UnitTestApplication};
    use crate::WindowIcon;
    use ferroui_base::{FerroLocator, LocatorExtensions};

    #[test]
    fn the_icon_loader_of_the_services_loads_the_icons_of_the_application() {
        let _app = UnitTestApplication::start(TestServices::new().with_icon_loader(Rc::new(TestIconLoader)));

        let mut source: &[u8] = b"icon data";
        let icon = WindowIcon::from_stream(&mut source).unwrap();
        let mut saved = Vec::new();
        icon.save(&mut saved).unwrap();
        assert_eq!(b"icon data".to_vec(), saved);

        let icon = WindowIcon::from_file("app.ico").unwrap();
        let mut saved = Vec::new();
        icon.save(&mut saved).unwrap();
        assert_eq!(b"app.ico".to_vec(), saved);
    }

    #[test]
    fn services_without_an_icon_loader_register_none() {
        let _app = UnitTestApplication::start(TestServices::new());

        assert!(FerroLocator::current().get_service::<dyn IPlatformIconLoader>().is_none());
    }
}
