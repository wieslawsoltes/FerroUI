//! Reading the items of the clipboard and of drag operations of the page.

use crate::browser_data_format_helper::{is_text_format, to_data_format};
use crate::interop::completion_helper::PromiseError;
use crate::interop::input_helper::{self, ReadableDataValueContent};
use crate::interop::JsObject;
use crate::storage::JsStorageFile;
use ferroui_base::input::platform::ClipboardError;
use ferroui_base::input::{DataFormat, LocalBoxFuture};
use ferroui_base::media::imaging::Bitmap;
use ferroui_base::platform::storage::IStorageItem;
use std::any::Any;
use std::rc::Rc;

/// A readable data item of the page (`ReadableDataItem` of the script
/// module): an item of the clipboard, an item of a data transfer, or a
/// string. Replaced by a fake in the tests.
pub(crate) trait IReadableDataItem {
    /// The format strings of the item.
    fn formats(&self) -> Vec<String>;

    /// Reads the value of the item in a format.
    fn try_get_value_async(&self, format: &str) -> LocalBoxFuture<Result<Option<ReadableDataValueContent>, PromiseError>>;

    /// Reads the value of an item of a drag operation in a format, while
    /// its event is being dispatched.
    fn try_get_value(&self, format: &str) -> Option<ReadableDataValueContent>;
}

/// An array of readable data items of the page. Replaced by a fake in the
/// tests.
pub(crate) trait IReadableDataItems {
    fn len(&self) -> usize;
    fn get(&self, index: usize) -> Rc<dyn IReadableDataItem>;
}

/// A readable data item of the page.
pub(crate) struct JsReadableDataItem(JsObject);

impl IReadableDataItem for JsReadableDataItem {
    fn formats(&self) -> Vec<String> {
        input_helper::get_readable_data_item_formats(&self.0)
    }

    fn try_get_value_async(&self, format: &str) -> LocalBoxFuture<Result<Option<ReadableDataValueContent>, PromiseError>> {
        let item = self.0.clone();
        let format = format.to_string();
        Box::pin(async move {
            let value = input_helper::try_get_readable_data_item_value_async(&item, &format).await?;
            Ok(value.as_ref().and_then(input_helper::get_readable_data_value))
        })
    }

    fn try_get_value(&self, format: &str) -> Option<ReadableDataValueContent> {
        input_helper::try_get_readable_data_item_value(&self.0, format)
            .as_ref()
            .and_then(input_helper::get_readable_data_value)
    }
}

/// An array of readable data items of the page.
pub(crate) struct JsReadableDataItems(JsObject);

impl JsReadableDataItems {
    pub(crate) fn new(items: JsObject) -> Rc<dyn IReadableDataItems> {
        Rc::new(Self(items))
    }
}

impl IReadableDataItems for JsReadableDataItems {
    fn len(&self) -> usize {
        input_helper::get_array_length(&self.0) as usize
    }

    fn get(&self, index: usize) -> Rc<dyn IReadableDataItem> {
        Rc::new(JsReadableDataItem(input_helper::get_array_item(&self.0, index as u32)))
    }
}

/// The data formats of a readable data item.
pub(crate) fn get_readable_item_formats(readable_data_item: &dyn IReadableDataItem) -> Vec<DataFormat> {
    let format_strings = readable_data_item.formats();
    let mut formats = Vec::with_capacity(format_strings.len() + 1);
    let mut has_supported_image = false;
    for format_string in &format_strings {
        // Differs from the original, which assigns instead of accumulating and so offers the
        // bitmap only when the PNG is the last format of the item.
        has_supported_image |= format_string == "image/png";
        formats.push(to_data_format(format_string));
    }

    if has_supported_image {
        formats.push(DataFormat::bitmap().into());
    }

    formats
}

/// The value of a format from the value read from an item.
///
/// The value type follows the format: `String` for the text format,
/// `Rc<dyn IStorageItem>` for the file format (a file of the page),
/// `Rc<Bitmap>` for the bitmap format, `String` for the formats whose name
/// on the page is a `text/*` media type and `Rc<[u8]>` for the others.
/// The original decides by the data type of the requested format, which the
/// formats of the framework do not carry; this is the rule the formats of
/// an item are created with (see `to_data_format`).
///
/// A value that cannot be decoded as a bitmap is an error, as the original
/// throws.
pub(crate) fn try_get_value(
    readable_data_value: Option<ReadableDataValueContent>,
    format: &DataFormat,
) -> Result<Option<Rc<dyn Any>>, ClipboardError> {
    let Some(data) = readable_data_value else {
        return Ok(None);
    };

    // A file is the storage item of the page the script side wrapped it into.
    let data = match data {
        ReadableDataValueContent::File(item) => {
            let file: Rc<dyn IStorageItem> = JsStorageFile::new(item);
            if DataFormat::file() == *format {
                return Ok(Some(Rc::new(file)));
            }
            return Ok(None);
        }
        data => data,
    };

    if DataFormat::text() == *format {
        return Ok(match data {
            ReadableDataValueContent::String(text) => Some(Rc::new(text)),
            _ => None,
        });
    }

    if DataFormat::file() == *format {
        return Ok(None);
    }

    if DataFormat::bitmap() == *format {
        return match data {
            ReadableDataValueContent::Bytes(bytes) => match Bitmap::from_stream(&mut bytes.as_slice()) {
                Ok(bitmap) => Ok(Some(Rc::new(Rc::new(bitmap)))),
                Err(error) => Err(ClipboardError::other(error.to_string())),
            },
            _ => Ok(None),
        };
    }

    if is_string_format(format) {
        return Ok(match data {
            ReadableDataValueContent::String(text) => Some(Rc::new(text)),
            ReadableDataValueContent::Bytes(bytes) => Some(Rc::new(String::from_utf8_lossy(&bytes).into_owned())),
            ReadableDataValueContent::File(_) => None,
        });
    }

    Ok(match data {
        ReadableDataValueContent::Bytes(bytes) => Some(Rc::new(Rc::<[u8]>::from(bytes))),
        ReadableDataValueContent::String(text) => Some(Rc::new(Rc::<[u8]>::from(text.into_bytes()))),
        ReadableDataValueContent::File(_) => None,
    })
}

/// Whether the values of a format are strings: see [`try_get_value`].
fn is_string_format(format: &DataFormat) -> bool {
    match format.kind() {
        ferroui_base::input::DataFormatKind::Platform => is_text_format(format.identifier()),
        _ => false,
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use std::cell::RefCell;

    /// A readable data item with fixed formats and values.
    pub(crate) struct FakeReadableDataItem {
        pub(crate) formats: Vec<String>,
        pub(crate) values: Vec<(String, FakeValue)>,
        pub(crate) reads: RefCell<Vec<String>>,
    }

    #[derive(Clone)]
    pub(crate) enum FakeValue {
        String(&'static str),
        Bytes(Vec<u8>),
        Rejected(&'static str),
    }

    impl FakeReadableDataItem {
        pub(crate) fn new(formats: &[&str], values: &[(&str, FakeValue)]) -> Rc<Self> {
            Rc::new(Self {
                formats: formats.iter().map(|format| format.to_string()).collect(),
                values: values.iter().map(|(format, value)| (format.to_string(), value.clone())).collect(),
                reads: RefCell::new(Vec::new()),
            })
        }

        fn value(&self, format: &str) -> Result<Option<ReadableDataValueContent>, PromiseError> {
            self.reads.borrow_mut().push(format.to_string());
            match self.values.iter().find(|(candidate, _)| candidate == format).map(|(_, value)| value) {
                Some(FakeValue::String(text)) => Ok(Some(ReadableDataValueContent::String(text.to_string()))),
                Some(FakeValue::Bytes(bytes)) => Ok(Some(ReadableDataValueContent::Bytes(bytes.clone()))),
                Some(FakeValue::Rejected(message)) => Err(PromiseError::new("Error", *message)),
                None => Ok(None),
            }
        }
    }

    impl IReadableDataItem for FakeReadableDataItem {
        fn formats(&self) -> Vec<String> {
            self.formats.clone()
        }

        fn try_get_value_async(
            &self,
            format: &str,
        ) -> LocalBoxFuture<Result<Option<ReadableDataValueContent>, PromiseError>> {
            Box::pin(std::future::ready(self.value(format)))
        }

        fn try_get_value(&self, format: &str) -> Option<ReadableDataValueContent> {
            self.value(format).ok().flatten()
        }
    }

    /// A fixed list of readable data items.
    pub(crate) struct FakeReadableDataItems(pub(crate) Vec<Rc<FakeReadableDataItem>>);

    impl IReadableDataItems for FakeReadableDataItems {
        fn len(&self) -> usize {
            self.0.len()
        }

        fn get(&self, index: usize) -> Rc<dyn IReadableDataItem> {
            self.0[index].clone()
        }
    }

    fn string_of(value: Option<Rc<dyn Any>>) -> Option<String> {
        value.and_then(|value| value.downcast_ref::<String>().cloned())
    }

    fn bytes_of(value: Option<Rc<dyn Any>>) -> Option<Vec<u8>> {
        value.and_then(|value| value.downcast_ref::<Rc<[u8]>>().map(|bytes| bytes.to_vec()))
    }

    #[test]
    fn the_formats_of_an_item_are_its_format_strings() {
        let item = FakeReadableDataItem::new(&["text/plain", "text/html", "application/frn-fmt.custom"], &[]);
        let formats = get_readable_item_formats(&*item);
        assert_eq!(3, formats.len());
        assert!(DataFormat::text() == formats[0]);
        assert_eq!("text/html", formats[1].identifier());
        assert_eq!("custom", formats[2].identifier());
    }

    #[test]
    fn a_png_adds_the_bitmap_format_wherever_it_is() {
        let item = FakeReadableDataItem::new(&["image/png", "text/plain"], &[]);
        let formats = get_readable_item_formats(&*item);
        assert_eq!(3, formats.len());
        assert!(DataFormat::bitmap() == formats[2]);

        let item = FakeReadableDataItem::new(&["text/plain"], &[]);
        assert!(!get_readable_item_formats(&*item).iter().any(|format| DataFormat::bitmap() == *format));
    }

    #[test]
    fn no_value_is_no_value() {
        assert!(try_get_value(None, &DataFormat::text().into()).unwrap().is_none());
    }

    #[test]
    fn text_is_a_string_value() {
        let text: DataFormat = DataFormat::text().into();
        let value = try_get_value(Some(ReadableDataValueContent::String("hello".into())), &text).unwrap();
        assert_eq!(Some("hello".to_string()), string_of(value));

        let value = try_get_value(Some(ReadableDataValueContent::Bytes(b"hello".to_vec())), &text).unwrap();
        assert!(value.is_none());
    }

    #[test]
    fn text_media_types_are_strings_and_bytes_are_decoded_as_utf8() {
        let html = to_data_format("text/html");
        let value = try_get_value(Some(ReadableDataValueContent::String("<b>a</b>".into())), &html).unwrap();
        assert_eq!(Some("<b>a</b>".to_string()), string_of(value));

        let value =
            try_get_value(Some(ReadableDataValueContent::Bytes("é".as_bytes().to_vec())), &html).unwrap();
        assert_eq!(Some("é".to_string()), string_of(value));

        let value = try_get_value(Some(ReadableDataValueContent::Bytes(vec![0x61, 0xff])), &html).unwrap();
        assert_eq!(Some("a\u{fffd}".to_string()), string_of(value));
    }

    #[test]
    fn other_formats_are_bytes_and_strings_are_encoded_as_utf8() {
        let custom = to_data_format("application/frn-fmt.custom");
        let value = try_get_value(Some(ReadableDataValueContent::Bytes(vec![1, 2, 3])), &custom).unwrap();
        assert_eq!(Some(vec![1, 2, 3]), bytes_of(value));

        let value = try_get_value(Some(ReadableDataValueContent::String("é".into())), &custom).unwrap();
        assert_eq!(Some("é".as_bytes().to_vec()), bytes_of(value));
    }

    #[test]
    fn a_bitmap_is_only_read_from_bytes() {
        // Decoding needs the render interface of a platform; the headless browser tests read
        // bitmaps.
        let bitmap: DataFormat = DataFormat::bitmap().into();
        assert!(try_get_value(Some(ReadableDataValueContent::String("png".into())), &bitmap).unwrap().is_none());
    }
}
