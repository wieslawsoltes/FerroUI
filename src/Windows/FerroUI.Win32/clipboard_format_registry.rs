//! The clipboard formats of the system and the data formats of the
//! framework: which identifier stands for which format.
//!
//! The registry is a list of pairs that starts with the formats the system
//! defines and grows by the formats that are met: an identifier the system
//! reports becomes a format of bytes named after it, and a format the
//! application puts on the clipboard is registered with the system under
//! its name.
//!
//! The list and its two lookups are written against the two calls of the
//! system they make ([`ClipboardFormatSystem`]), so that they are tested on
//! every host; the registry of the process ([`ClipboardFormatRegistry`]) is
//! the list of the UI thread over the system. The reference keeps one list
//! for the process behind a lock; a data format of the port is a value of
//! one thread, and so is every caller of the registry (OLE calls a data
//! object on the thread that made it).

use crate::interop::unmanaged_methods::ClipboardFormat;
use ferroui_base::input::DataFormat;
use ferroui_base::media::imaging::Bitmap;
use std::rc::Rc;

/// The prefix of the name an application format is registered under.
const APP_PREFIX: &str = "frn-app-fmt:";

/// The two calls of the system the registry makes.
pub(crate) trait ClipboardFormatSystem {
    /// The name of a registered format (`GetClipboardFormatName`); `None`
    /// for an identifier without one.
    fn get_clipboard_format_name(&self, id: u16) -> Option<String>;

    /// Registers a format by its name (`RegisterClipboardFormat`) and
    /// returns its identifier; 0 on failure.
    fn register_clipboard_format(&self, name: &str) -> u32;
}

/// The formats of bitmaps of the platform.
pub(crate) struct ImageFormats {
    pub png_system_data_format: DataFormat,
    pub png_mime_data_format: DataFormat,
    pub h_bitmap_data_format: DataFormat,
    pub dib_data_format: DataFormat,
    pub dib_v5_data_format: DataFormat,
}

impl ImageFormats {
    fn new() -> ImageFormats {
        let platform = |name: &str| -> DataFormat { DataFormat::from_system_name::<Rc<Bitmap>>(name, APP_PREFIX).into() };
        ImageFormats {
            png_system_data_format: platform("PNG"),
            png_mime_data_format: platform("image/png"),
            h_bitmap_data_format: platform("CF_BITMAP"),
            dib_data_format: platform("CF_DIB"),
            dib_v5_data_format: platform("CF_DIBV5"),
        }
    }

    /// Ordered from the most preferred to the least preferred
    pub fn image_formats(&self) -> [&DataFormat; 5] {
        [
            &self.png_mime_data_format,
            &self.png_system_data_format,
            &self.dib_data_format,
            &self.dib_v5_data_format,
            &self.h_bitmap_data_format,
        ]
    }

    /// Whether a format is one of the formats of bitmaps.
    pub fn contains(&self, format: &DataFormat) -> bool {
        self.image_formats().into_iter().any(|image_format| image_format == format)
    }
}

/// The name of a format of the system that has no registered name: the
/// name of its constant, or a name made of its number.
fn enum_name(id: u16) -> Option<&'static str> {
    match id {
        ClipboardFormat::CF_BITMAP => Some("CF_BITMAP"),
        ClipboardFormat::CF_DIB => Some("CF_DIB"),
        ClipboardFormat::CF_UNICODETEXT => Some("CF_UNICODETEXT"),
        ClipboardFormat::CF_HDROP => Some("CF_HDROP"),
        ClipboardFormat::CF_DIBV5 => Some("CF_DIBV5"),
        _ => None,
    }
}

/// The list of the formats that have an identifier.
pub(crate) struct ClipboardFormatList<S: ClipboardFormatSystem> {
    system: S,
    formats: Vec<(DataFormat, u16)>,
    image_formats: ImageFormats,
}

impl<S: ClipboardFormatSystem> ClipboardFormatList<S> {
    pub fn new(system: S) -> ClipboardFormatList<S> {
        let image_formats = ImageFormats::new();
        let mut list = ClipboardFormatList { system, formats: Vec::new(), image_formats };
        list.add_data_format(DataFormat::text().into(), ClipboardFormat::CF_UNICODETEXT);
        list.add_data_format(DataFormat::file().into(), ClipboardFormat::CF_HDROP);
        list.add_data_format(list.image_formats.dib_data_format.clone(), ClipboardFormat::CF_DIB);
        list.add_data_format(list.image_formats.dib_v5_data_format.clone(), ClipboardFormat::CF_DIBV5);
        list.add_data_format(list.image_formats.h_bitmap_data_format.clone(), ClipboardFormat::CF_BITMAP);
        list
    }

    pub fn image_formats(&self) -> &ImageFormats {
        &self.image_formats
    }

    fn add_data_format(&mut self, format: DataFormat, id: u16) {
        self.formats.push((format, id));
    }

    fn get_format_system_name(&self, id: u16) -> String {
        if let Some(name) = self.system.get_clipboard_format_name(id) {
            return name;
        }
        if let Some(name) = enum_name(id) {
            return name.to_string();
        }
        format!("Unknown_Format_{id}")
    }

    /// The format of an identifier of the system; an identifier that is
    /// met for the first time becomes a format of bytes.
    pub fn get_or_add_format_from_id(&mut self, id: u16) -> DataFormat {
        if let Some((format, _)) = self.formats.iter().find(|(_, format_id)| *format_id == id) {
            return format.clone();
        }

        let system_name = self.get_format_system_name(id);
        let format: DataFormat = DataFormat::from_system_name::<Rc<[u8]>>(&system_name, APP_PREFIX).into();
        self.add_data_format(format.clone(), id);
        format
    }

    /// The identifier of a format, registered with the system when it is
    /// met for the first time.
    ///
    /// # Panics
    /// Panics when the system does not register the format, and for a
    /// format that has no name for the system (an in-process format, or
    /// the bitmap format: callers pass an effective platform format).
    pub fn get_or_add_format(&mut self, format: &DataFormat) -> u16 {
        debug_assert!(*format != DataFormat::bitmap()); // Callers must pass an effective platform type

        if let Some((_, id)) = self.formats.iter().find(|(known, _)| known == format) {
            return *id;
        }

        let system_name = format.to_system_name(APP_PREFIX);
        let result = self.system.register_clipboard_format(&system_name);
        if result == 0 {
            panic!("The clipboard format {system_name:?} could not be registered.");
        }

        let id = result as u16;
        self.add_data_format(format.clone(), id);
        id
    }
}

#[cfg(windows)]
pub(crate) use imp::ClipboardFormatRegistry;

#[cfg(windows)]
mod imp {
    use super::*;
    use crate::interop::unmanaged_methods;
    use std::cell::RefCell;

    struct System;

    impl ClipboardFormatSystem for System {
        fn get_clipboard_format_name(&self, id: u16) -> Option<String> {
            unmanaged_methods::get_clipboard_format_name(id)
        }

        fn register_clipboard_format(&self, name: &str) -> u32 {
            unmanaged_methods::register_clipboard_format(name)
        }
    }

    thread_local! {
        static FORMATS: RefCell<ClipboardFormatList<System>> = RefCell::new(ClipboardFormatList::new(System));
    }

    /// The registry of the thread, over the system.
    pub(crate) struct ClipboardFormatRegistry;

    impl ClipboardFormatRegistry {
        pub fn png_system_data_format() -> DataFormat {
            FORMATS.with(|formats| formats.borrow().image_formats().png_system_data_format.clone())
        }

        pub fn png_mime_data_format() -> DataFormat {
            FORMATS.with(|formats| formats.borrow().image_formats().png_mime_data_format.clone())
        }

        pub fn h_bitmap_data_format() -> DataFormat {
            FORMATS.with(|formats| formats.borrow().image_formats().h_bitmap_data_format.clone())
        }

        pub fn dib_data_format() -> DataFormat {
            FORMATS.with(|formats| formats.borrow().image_formats().dib_data_format.clone())
        }

        pub fn dib_v5_data_format() -> DataFormat {
            FORMATS.with(|formats| formats.borrow().image_formats().dib_v5_data_format.clone())
        }

        /// Ordered from the most preferred to the least preferred
        pub fn image_formats() -> Vec<DataFormat> {
            FORMATS.with(|formats| formats.borrow().image_formats().image_formats().into_iter().cloned().collect())
        }

        /// Whether a format is one of the formats of bitmaps.
        pub fn is_image_format(format: &DataFormat) -> bool {
            FORMATS.with(|formats| formats.borrow().image_formats().contains(format))
        }

        pub fn get_or_add_format_from_id(id: u16) -> DataFormat {
            FORMATS.with(|formats| formats.borrow_mut().get_or_add_format_from_id(id))
        }

        pub fn get_or_add_format(format: &DataFormat) -> u16 {
            FORMATS.with(|formats| formats.borrow_mut().get_or_add_format(format))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ferroui_base::input::DataFormatKind;
    use std::any::TypeId;
    use std::cell::RefCell;

    /// A system with the formats registered through it, numbered from
    /// 0xC000 as the system numbers them.
    #[derive(Default)]
    struct FakeSystem {
        names: RefCell<Vec<String>>,
        fail: bool,
    }

    impl ClipboardFormatSystem for &FakeSystem {
        fn get_clipboard_format_name(&self, id: u16) -> Option<String> {
            self.names.borrow().get(usize::from(id).checked_sub(0xC000)?).cloned()
        }

        fn register_clipboard_format(&self, name: &str) -> u32 {
            if self.fail {
                return 0;
            }
            let mut names = self.names.borrow_mut();
            let index = match names.iter().position(|known| known.eq_ignore_ascii_case(name)) {
                Some(index) => index,
                None => {
                    names.push(name.to_string());
                    names.len() - 1
                }
            };
            0xC000 + index as u32
        }
    }

    #[test]
    fn the_formats_of_the_system_have_their_identifiers() {
        let system = FakeSystem::default();
        let mut list = ClipboardFormatList::new(&system);
        assert_eq!(list.get_or_add_format(&DataFormat::text()), 13);
        assert_eq!(list.get_or_add_format(&DataFormat::file()), 15);
        let (dib, dib_v5, h_bitmap) = (
            list.image_formats().dib_data_format.clone(),
            list.image_formats().dib_v5_data_format.clone(),
            list.image_formats().h_bitmap_data_format.clone(),
        );
        assert_eq!(list.get_or_add_format(&dib), 8);
        assert_eq!(list.get_or_add_format(&dib_v5), 17);
        assert_eq!(list.get_or_add_format(&h_bitmap), 2);
        assert_eq!(list.get_or_add_format_from_id(13), *DataFormat::text());
        assert_eq!(list.get_or_add_format_from_id(15), *DataFormat::file());
        assert_eq!(list.get_or_add_format_from_id(2), h_bitmap);
        assert!(system.names.borrow().is_empty(), "nothing was registered");
    }

    #[test]
    fn the_image_formats_are_ordered_by_preference() {
        let system = FakeSystem::default();
        let list = ClipboardFormatList::new(&system);
        let names: Vec<&str> = list.image_formats().image_formats().iter().map(|format| format.identifier()).collect();
        assert_eq!(names, ["image/png", "PNG", "CF_DIB", "CF_DIBV5", "CF_BITMAP"]);
        assert!(list.image_formats().image_formats().iter().all(|format| format.kind() == DataFormatKind::Platform));
        assert!(list.image_formats().contains(&list.image_formats().png_system_data_format));
        assert!(!list.image_formats().contains(&DataFormat::text()));
    }

    #[test]
    fn an_application_format_is_registered_under_the_prefix_once() {
        let system = FakeSystem::default();
        let mut list = ClipboardFormatList::new(&system);
        let format = DataFormat::create_string_application_format("my-format");
        let id = list.get_or_add_format(&format);
        assert_eq!(id, 0xC000);
        assert_eq!(*system.names.borrow(), ["frn-app-fmt:my-format"]);
        assert_eq!(list.get_or_add_format(&format), id);
        assert_eq!(system.names.borrow().len(), 1);

        // The format that was registered is the one its identifier gives
        // back, with the type of its values.
        let back = list.get_or_add_format_from_id(id);
        assert_eq!(back, *format);
        assert_eq!(back.data_type(), Some(TypeId::of::<String>()));
    }

    #[test]
    fn a_platform_format_is_registered_under_its_own_name() {
        let system = FakeSystem::default();
        let mut list = ClipboardFormatList::new(&system);
        let png = list.image_formats().png_system_data_format.clone();
        let id = list.get_or_add_format(&png);
        assert_eq!(*system.names.borrow(), ["PNG"]);
        assert_eq!(list.get_or_add_format_from_id(id), png);
    }

    #[test]
    fn an_identifier_of_the_system_becomes_a_format_of_bytes() {
        let system = FakeSystem::default();
        system.names.borrow_mut().extend(["HTML Format".to_string(), "frn-app-fmt:other.app-format".to_string()]);
        let mut list = ClipboardFormatList::new(&system);

        let html = list.get_or_add_format_from_id(0xC000);
        assert_eq!((html.kind(), html.identifier()), (DataFormatKind::Platform, "HTML Format"));
        assert_eq!(html.data_type(), Some(TypeId::of::<Rc<[u8]>>()));
        // Its identifier is known from then on without the system.
        assert_eq!(list.get_or_add_format(&html), 0xC000);

        let application = list.get_or_add_format_from_id(0xC001);
        assert_eq!((application.kind(), application.identifier()), (DataFormatKind::Application, "other.app-format"));
    }

    #[test]
    fn an_identifier_without_a_name_is_named_after_its_number() {
        let system = FakeSystem::default();
        let mut list = ClipboardFormatList::new(&system);
        // CF_TEXT: a format of the system the list does not start with.
        let format = list.get_or_add_format_from_id(1);
        assert_eq!((format.kind(), format.identifier()), (DataFormatKind::Platform, "Unknown_Format_1"));
    }

    #[test]
    #[should_panic(expected = "could not be registered")]
    fn a_format_the_system_does_not_register_fails() {
        let system = FakeSystem { fail: true, ..Default::default() };
        let mut list = ClipboardFormatList::new(&system);
        list.get_or_add_format(&DataFormat::create_bytes_application_format("x"));
    }
}
