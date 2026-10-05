use crate::frn_string::frn_string_to_string;
use crate::interop::*;
use ferroui_base::input::DataFormat;

// TODO hide native types behind the native clipboard abstraction, so this side won't depend on macOS.
const NS_PASTEBOARD_TYPE_STRING: &str = "public.utf8-plain-text";
const NS_PASTEBOARD_TYPE_PNG: &str = "public.png";
const APP_PREFIX: &str = "net.ferroui.app.uti.";

// The file format ("public.file-url") is not mapped yet: storage items are
// not ported, so there is no file data format. A file URL on the pasteboard
// is seen as a platform format.

/// The data formats of a list of native formats.
pub(crate) fn to_data_formats(
    native_formats: Option<&IFrnStringArray>,
    is_text_format: &dyn Fn(&str) -> bool,
) -> Vec<DataFormat> {
    let Some(native_formats) = native_formats else {
        return Vec::new();
    };

    (0..native_formats.get_count())
        .map(|c| {
            let native_format = native_formats.get(c).ok().flatten();
            let native_format = native_format.and_then(|s| frn_string_to_string(&s)).unwrap_or_default();
            to_data_format(&native_format, is_text_format)
        })
        .collect()
}

/// The data format of a native format: the universal formats for the
/// pasteboard types that have one, otherwise a string format when the
/// native side says the type is textual and a bytes format when not.
pub(crate) fn to_data_format(native_format: &str, is_text_format: &dyn Fn(&str) -> bool) -> DataFormat {
    match native_format {
        NS_PASTEBOARD_TYPE_STRING => DataFormat::text().into(),
        NS_PASTEBOARD_TYPE_PNG => DataFormat::bitmap().into(),
        _ if is_text_format(native_format) => DataFormat::from_system_name::<String>(native_format, APP_PREFIX).into(),
        _ => DataFormat::from_system_name::<std::rc::Rc<[u8]>>(native_format, APP_PREFIX).into(),
    }
}

pub(crate) fn to_native_format(format: &DataFormat) -> String {
    if DataFormat::text() == *format {
        return NS_PASTEBOARD_TYPE_STRING.to_string();
    }

    if DataFormat::bitmap() == *format {
        return NS_PASTEBOARD_TYPE_PNG.to_string();
    }

    format.to_system_name(APP_PREFIX)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frn_string::FrnStringArray;
    use ferroui_base::input::DataFormatKind;

    fn is_text(format: &str) -> bool {
        format.contains("text")
    }

    #[test]
    fn universal_formats_map_to_their_pasteboard_types() {
        assert_eq!(to_native_format(&DataFormat::text()), "public.utf8-plain-text");
        assert_eq!(to_native_format(&DataFormat::bitmap()), "public.png");
        assert!(DataFormat::text() == to_data_format("public.utf8-plain-text", &is_text));
        assert!(DataFormat::bitmap() == to_data_format("public.png", &is_text));
    }

    #[test]
    fn other_formats_round_trip_through_their_system_name() {
        let platform = to_data_format("public.html-text", &is_text);
        assert_eq!(platform.kind(), DataFormatKind::Platform);
        assert_eq!(to_native_format(&platform), "public.html-text");

        let application = DataFormat::create_bytes_application_format("my-format");
        let native = to_native_format(&application);
        assert_eq!(native, "net.ferroui.app.uti.my-format");
        let back = to_data_format(&native, &is_text);
        assert_eq!(back.kind(), DataFormatKind::Application);
        assert!(application == back);
    }

    #[test]
    fn native_format_lists_are_converted_in_order() {
        let array = IFrnStringArray::from_impl(FrnStringArray::new(["public.utf8-plain-text", "public.png", "x.y"]));
        let formats = to_data_formats(Some(&array), &is_text);
        assert_eq!(formats.len(), 3);
        assert!(DataFormat::text() == formats[0]);
        assert!(DataFormat::bitmap() == formats[1]);
        assert_eq!(formats[2].identifier(), "x.y");
        assert!(to_data_formats(None, &is_text).is_empty());
    }
}
