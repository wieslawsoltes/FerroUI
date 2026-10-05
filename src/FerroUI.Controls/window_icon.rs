use crate::platform::{IPlatformIconLoader, IWindowIconImpl};
use ferroui_base::media::imaging::Bitmap;
use ferroui_base::{FerroLocator, LocatorExtensions};
use std::io;
use std::rc::Rc;

/// Represents an icon for a window.
pub struct WindowIcon {
    platform_impl: Rc<dyn IWindowIconImpl>,
}

impl WindowIcon {
    fn loader() -> Rc<dyn IPlatformIconLoader> {
        FerroLocator::current().get_required_service::<dyn IPlatformIconLoader>()
    }

    /// Creates an icon from a bitmap.
    ///
    /// # Panics
    /// Panics when no platform icon loader is registered.
    pub fn from_bitmap(bitmap: &Bitmap) -> Rc<WindowIcon> {
        Rc::new(WindowIcon { platform_impl: Self::loader().load_icon_from_bitmap(bitmap.platform_impl().item()) })
    }

    /// Loads an icon from a file.
    ///
    /// # Panics
    /// Panics when no platform icon loader is registered.
    pub fn from_file(file_name: &str) -> io::Result<Rc<WindowIcon>> {
        Ok(Rc::new(WindowIcon { platform_impl: Self::loader().load_icon_from_file(file_name)? }))
    }

    /// Loads an icon from a stream.
    ///
    /// # Panics
    /// Panics when no platform icon loader is registered.
    pub fn from_stream(stream: &mut dyn io::Read) -> io::Result<Rc<WindowIcon>> {
        Ok(Rc::new(WindowIcon { platform_impl: Self::loader().load_icon_from_stream(stream)? }))
    }

    /// The platform implementation of the icon (internal upstream: for the
    /// windowing classes).
    pub fn platform_impl(&self) -> Rc<dyn IWindowIconImpl> {
        self.platform_impl.clone()
    }

    /// Writes the icon to a stream.
    pub fn save(&self, stream: &mut dyn io::Write) -> io::Result<()> {
        self.platform_impl.save(stream)
    }
}

/// Icons are compared by identity, like the references of the reference
/// implementation.
impl PartialEq for WindowIcon {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ferroui_base::platform::IBitmapImpl;
    use std::cell::RefCell;

    struct TestIconImpl(Vec<u8>);

    impl IWindowIconImpl for TestIconImpl {
        fn save(&self, output_stream: &mut dyn io::Write) -> io::Result<()> {
            output_stream.write_all(&self.0)
        }
    }

    #[derive(Default)]
    struct TestIconLoader {
        files: RefCell<Vec<String>>,
    }

    impl IPlatformIconLoader for TestIconLoader {
        fn load_icon_from_file(&self, file_name: &str) -> io::Result<Rc<dyn IWindowIconImpl>> {
            self.files.borrow_mut().push(file_name.to_string());
            Ok(Rc::new(TestIconImpl(file_name.as_bytes().to_vec())))
        }

        fn load_icon_from_stream(&self, stream: &mut dyn io::Read) -> io::Result<Rc<dyn IWindowIconImpl>> {
            let mut data = Vec::new();
            stream.read_to_end(&mut data)?;
            Ok(Rc::new(TestIconImpl(data)))
        }

        fn load_icon_from_bitmap(&self, _bitmap: Rc<dyn IBitmapImpl>) -> Rc<dyn IWindowIconImpl> {
            Rc::new(TestIconImpl(b"bitmap".to_vec()))
        }
    }

    fn with_loader(test: impl FnOnce(Rc<TestIconLoader>)) {
        let scope = FerroLocator::enter_scope();
        let loader = Rc::new(TestIconLoader::default());
        let service: Rc<dyn IPlatformIconLoader> = loader.clone();
        FerroLocator::current_mutable().bind::<dyn IPlatformIconLoader>().to_constant(service);
        test(loader);
        scope.dispose();
    }

    #[test]
    fn icon_from_stream_saves_what_the_loader_produced() {
        with_loader(|_| {
            let mut source: &[u8] = b"icon data";
            let icon = WindowIcon::from_stream(&mut source).unwrap();

            let mut saved = Vec::new();
            icon.save(&mut saved).unwrap();

            assert_eq!(b"icon data".to_vec(), saved);
        });
    }

    #[test]
    fn icon_from_file_asks_the_loader_for_the_file() {
        with_loader(|loader| {
            let icon = WindowIcon::from_file("app.ico").unwrap();

            let mut saved = Vec::new();
            icon.save(&mut saved).unwrap();

            assert_eq!(vec!["app.ico".to_string()], *loader.files.borrow());
            assert_eq!(b"app.ico".to_vec(), saved);
        });
    }

    #[test]
    fn icons_are_compared_by_identity() {
        with_loader(|_| {
            let a = WindowIcon::from_file("a.ico").unwrap();
            let b = WindowIcon::from_file("a.ico").unwrap();

            assert!(Some(a.clone()) == Some(a.clone()));
            assert!(Some(a) != Some(b));
        });
    }
}
