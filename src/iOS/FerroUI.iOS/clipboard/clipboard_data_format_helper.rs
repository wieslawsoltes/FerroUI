//! The data formats of the framework and the uniform type identifiers of
//! the pasteboard.

use ferroui_base::input::DataFormat;
use std::rc::Rc;

const UT_TYPE_UTF8_PLAIN_TEXT: &str = "public.utf8-plain-text";
const UT_TYPE_FILE_URL: &str = "public.file-url";
pub(crate) const UT_TYPE_IMAGE: &str = "public.image";
pub(crate) const UT_TYPE_PNG: &str = "public.png";
pub(crate) const UT_TYPE_JPEG: &str = "public.jpeg";
pub(crate) const UT_TYPE_TIFF: &str = "public.tiff";
const APP_PREFIX: &str = "net.ferroui.app.uti.";

/// The data format of a type of the pasteboard. `is_text_uti` tells
/// whether the system knows a type as text.
pub fn to_data_format(type_: &str, is_text_uti: &dyn Fn(&str) -> bool) -> DataFormat {
    match type_ {
        UT_TYPE_UTF8_PLAIN_TEXT => DataFormat::text().into(),
        UT_TYPE_FILE_URL => DataFormat::file().into(),
        UT_TYPE_IMAGE | UT_TYPE_PNG | UT_TYPE_JPEG | UT_TYPE_TIFF => DataFormat::bitmap().into(),
        _ if is_text(type_, is_text_uti) => DataFormat::from_system_name::<String>(type_, APP_PREFIX).into(),
        _ => DataFormat::from_system_name::<Rc<[u8]>>(type_, APP_PREFIX).into(),
    }
}

/// The type of the pasteboard of a data format.
pub fn to_system_type(format: &DataFormat) -> String {
    if DataFormat::text() == *format {
        return UT_TYPE_UTF8_PLAIN_TEXT.to_string();
    }

    if DataFormat::file() == *format {
        return UT_TYPE_FILE_URL.to_string();
    }

    if DataFormat::bitmap() == *format {
        // Images are written to the clipboard as PNGs on iOS and macOS.
        return UT_TYPE_PNG.to_string();
    }

    format.to_system_name(APP_PREFIX)
}

/// Best effort trying to find whether a type is text. Falling back to
/// bytes is fine. A format of an application is never asked about.
pub fn is_text(type_: &str, is_text_uti: &dyn Fn(&str) -> bool) -> bool {
    let is_application = type_.len() >= APP_PREFIX.len()
        && type_.is_char_boundary(APP_PREFIX.len())
        && type_[..APP_PREFIX.len()].eq_ignore_ascii_case(APP_PREFIX);
    !is_application && is_text_uti(type_)
}

/// Whether the system knows a type as text.
#[cfg(target_os = "ios")]
pub(crate) fn is_text_uti(type_: &str) -> bool {
    crate::storage::ios_storage_provider::is_text_uniform_type(type_)
}

#[cfg(test)]
mod tests {
    // Not from the reference, which has no tests of this file.
    use super::*;
    use ferroui_base::input::DataFormatKind;

    fn text_types(type_: &str) -> bool {
        type_.contains("text") || type_.contains("html")
    }

    #[test]
    fn the_universal_formats_have_their_types() {
        assert_eq!("public.utf8-plain-text", to_system_type(&DataFormat::text()));
        assert_eq!("public.file-url", to_system_type(&DataFormat::file()));
        assert_eq!("public.png", to_system_type(&DataFormat::bitmap()));
        assert!(DataFormat::text() == to_data_format("public.utf8-plain-text", &text_types));
        assert!(DataFormat::file() == to_data_format("public.file-url", &text_types));
        for image in ["public.image", "public.png", "public.jpeg", "public.tiff"] {
            assert!(DataFormat::bitmap() == to_data_format(image, &text_types), "{image}");
        }
    }

    #[test]
    fn other_types_are_platform_formats_that_keep_their_name() {
        let html = to_data_format("public.html", &text_types);
        assert_eq!(DataFormatKind::Platform, html.kind());
        assert_eq!("public.html", to_system_type(&html));
        let data = to_data_format("com.example.data", &text_types);
        assert_eq!(DataFormatKind::Platform, data.kind());
        assert_eq!("com.example.data", to_system_type(&data));
    }

    #[test]
    fn the_formats_of_an_application_have_the_prefix_and_are_not_asked_about() {
        let format = DataFormat::create_bytes_application_format("my-format");
        let type_ = to_system_type(&format);
        assert_eq!("net.ferroui.app.uti.my-format", type_);
        let back = to_data_format(&type_, &|_| panic!("a format of the application is not asked about"));
        assert_eq!(DataFormatKind::Application, back.kind());
        assert!(format == back);
        assert!(!is_text("NET.FERROUI.APP.UTI.text", &|_| true));
        assert!(is_text("public.text", &|_| true));
        assert!(!is_text("public.text", &|_| false));
    }
}
