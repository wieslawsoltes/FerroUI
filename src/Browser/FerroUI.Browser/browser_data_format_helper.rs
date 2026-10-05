//! The data formats of the framework and the format strings of the page.

use ferroui_base::input::DataFormat;
use std::rc::Rc;

const FORMAT_TEXT_PLAIN: &str = "text/plain";
const FORMAT_FILES: &str = "Files";
const FORMAT_IMAGE: &str = "image/png";
const APP_PREFIX: &str = "application/frn-fmt.";

/// The data format of a format string of the page.
pub(crate) fn to_data_format(format_string: &str) -> DataFormat {
    match format_string {
        FORMAT_TEXT_PLAIN => DataFormat::text().into(),
        FORMAT_FILES => DataFormat::file().into(),
        _ if is_text_format(format_string) => DataFormat::from_system_name::<String>(format_string, APP_PREFIX).into(),
        _ => DataFormat::from_system_name::<Rc<[u8]>>(format_string, APP_PREFIX).into(),
    }
}

/// Whether the values of a format string of the page are text: the
/// `text/*` media types.
pub(crate) fn is_text_format(format: &str) -> bool {
    format.get(..5).is_some_and(|prefix| prefix.eq_ignore_ascii_case("text/"))
}

/// The format string of the page for a data format.
///
/// # Panics
/// Panics for an in-process format, which has no name outside the
/// application.
pub(crate) fn to_browser_format(format: &DataFormat) -> String {
    if DataFormat::text() == *format {
        return FORMAT_TEXT_PLAIN.to_string();
    }

    if DataFormat::file() == *format {
        return FORMAT_FILES.to_string();
    }

    if DataFormat::bitmap() == *format {
        return FORMAT_IMAGE.to_string();
    }

    format.to_system_name(APP_PREFIX)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ferroui_base::input::DataFormatKind;

    #[test]
    fn plain_text_is_the_text_format() {
        assert!(DataFormat::text() == to_data_format("text/plain"));
        assert_eq!("text/plain", to_browser_format(&DataFormat::text()));
    }

    #[test]
    fn the_bitmap_format_is_png() {
        assert_eq!("image/png", to_browser_format(&DataFormat::bitmap()));
        // A PNG of the page is a platform format of bytes; the bitmap format is added by the
        // reader of the item (see BrowserDataTransferHelper).
        let png = to_data_format("image/png");
        assert_eq!(DataFormatKind::Platform, png.kind());
        assert_eq!("image/png", png.identifier());
    }

    #[test]
    fn other_text_media_types_are_platform_formats() {
        let html = to_data_format("text/html");
        assert_eq!(DataFormatKind::Platform, html.kind());
        assert_eq!("text/html", html.identifier());
        assert_eq!("text/html", to_browser_format(&html));
        assert!(is_text_format("TEXT/uri-list"));
        assert!(!is_text_format("image/png"));
        assert!(!is_text_format("text"));
    }

    #[test]
    fn application_formats_round_trip_through_the_prefix() {
        let format: DataFormat = DataFormat::create_bytes_application_format("my-format").into();
        assert_eq!("application/frn-fmt.my-format", to_browser_format(&format));

        let parsed = to_data_format("application/frn-fmt.my-format");
        assert_eq!(DataFormatKind::Application, parsed.kind());
        assert_eq!("my-format", parsed.identifier());
        assert!(parsed == format);
    }

    #[test]
    fn files_are_the_file_format() {
        assert!(DataFormat::file() == to_data_format("Files"));
        assert_eq!("Files", to_browser_format(&DataFormat::file()));
    }

    #[test]
    #[should_panic(expected = "Cannot get system name")]
    fn in_process_formats_have_no_name_on_the_page() {
        let format: DataFormat = DataFormat::create_in_process_format::<String>("local").into();
        to_browser_format(&format);
    }
}
