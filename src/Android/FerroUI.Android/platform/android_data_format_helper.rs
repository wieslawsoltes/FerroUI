//! Data formats of the framework as MIME types of the clipboard of the
//! system, and back.

use ferroui_base::input::{DataFormat, DataFormatOf};
use std::rc::Rc;

const APP_PREFIX: &str = "application/frn-fmt.";
const MIME_TYPE_IMAGE_PNG: &str = "image/png";

/// `ClipDescription.MIMETYPE_TEXT_PLAIN`.
pub(crate) const MIMETYPE_TEXT_PLAIN: &str = "text/plain";
/// `ClipDescription.MIMETYPE_TEXT_URILIST`.
pub(crate) const MIMETYPE_TEXT_URILIST: &str = "text/uri-list";

fn starts_with_ignore_case(text: &str, prefix: &str) -> bool {
    text.as_bytes().get(..prefix.len()).is_some_and(|start| start.eq_ignore_ascii_case(prefix.as_bytes()))
}

pub(crate) struct AndroidDataFormatHelper;

impl AndroidDataFormatHelper {
    pub fn mime_type_to_data_format(mime_type: &str) -> DataFormat {
        if mime_type == MIMETYPE_TEXT_PLAIN {
            return DataFormat::text().into();
        }

        if mime_type == MIMETYPE_TEXT_URILIST {
            return DataFormat::file().into();
        }

        if starts_with_ignore_case(mime_type, "image/") {
            return DataFormat::bitmap().into();
        }

        if Self::is_text_mime_type(mime_type) {
            let format: DataFormatOf<String> = DataFormat::from_system_name(mime_type, APP_PREFIX);
            return format.into();
        }

        let format: DataFormatOf<Rc<[u8]>> = DataFormat::from_system_name(mime_type, APP_PREFIX);
        format.into()
    }

    pub fn data_format_to_mime_type(format: &DataFormat) -> String {
        if DataFormat::text() == *format {
            return MIMETYPE_TEXT_PLAIN.to_string();
        }

        if DataFormat::file() == *format {
            return MIMETYPE_TEXT_URILIST.to_string();
        }

        if DataFormat::bitmap() == *format {
            return MIME_TYPE_IMAGE_PNG.to_string();
        }

        format.to_system_name(APP_PREFIX)
    }

    /// Whether a MIME type is one whose format has text as its value: what
    /// [`mime_type_to_data_format`](Self::mime_type_to_data_format) makes a
    /// format of text of.
    ///
    /// Not from the reference, which asks the format for the type of its
    /// values; a format does not carry that type at run time here.
    pub fn is_text_mime_type(mime_type: &str) -> bool {
        starts_with_ignore_case(mime_type, "text/")
    }
}

#[cfg(test)]
mod tests {
    // Not from the reference, which has no tests of the helper.
    use super::*;
    use ferroui_base::input::DataFormatKind;

    #[test]
    fn the_three_universal_formats_have_their_mime_types() {
        let to = AndroidDataFormatHelper::data_format_to_mime_type;
        assert_eq!(to(&DataFormat::text()), "text/plain");
        assert_eq!(to(&DataFormat::file()), "text/uri-list");
        assert_eq!(to(&DataFormat::bitmap()), "image/png");

        let from = AndroidDataFormatHelper::mime_type_to_data_format;
        assert_eq!(from("text/plain"), *DataFormat::text());
        assert_eq!(from("text/uri-list"), *DataFormat::file());
        assert_eq!(from("image/png"), *DataFormat::bitmap());
        assert_eq!(from("IMAGE/jpeg"), *DataFormat::bitmap());
    }

    #[test]
    fn an_application_format_has_the_prefix_and_a_platform_format_its_name() {
        let application = DataFormat::create_string_application_format("my-app.thing");
        let mime_type = AndroidDataFormatHelper::data_format_to_mime_type(&application);
        assert_eq!(mime_type, "application/frn-fmt.my-app.thing");
        let back = AndroidDataFormatHelper::mime_type_to_data_format(&mime_type);
        assert_eq!(back, *application);
        assert_eq!(back.kind(), DataFormatKind::Application);

        let html = AndroidDataFormatHelper::mime_type_to_data_format("text/html");
        assert_eq!(html.kind(), DataFormatKind::Platform);
        assert_eq!(html.identifier(), "text/html");
        assert_eq!(AndroidDataFormatHelper::data_format_to_mime_type(&html), "text/html");

        let bytes = AndroidDataFormatHelper::mime_type_to_data_format("application/octet-stream");
        assert_eq!(bytes.kind(), DataFormatKind::Platform);
        assert_eq!(bytes.identifier(), "application/octet-stream");
    }

    #[test]
    fn text_mime_types_are_formats_of_text() {
        assert!(AndroidDataFormatHelper::is_text_mime_type("text/html"));
        assert!(AndroidDataFormatHelper::is_text_mime_type("TEXT/x-thing"));
        assert!(!AndroidDataFormatHelper::is_text_mime_type("application/json"));
        assert!(!AndroidDataFormatHelper::is_text_mime_type("tex"));
    }
}
