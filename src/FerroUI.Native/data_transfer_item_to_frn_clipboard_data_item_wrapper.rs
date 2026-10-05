use crate::clipboard_data_format_helper::to_native_format;
use crate::frn_string::{FrnString, FrnStringArray};
use crate::interop::*;
use ferroui_base::input::{DataFormat, DataFormatKind, DataTransferItemExtensions, IDataTransferItem};
use ferroui_base::logging::{LogArea, LogEventLevel, Logger};
use ferroui_base::media::imaging::{BitmapEncoderOptions, PngBitmapEncoderOptions};
use ferroui_microcom::{ComPtr, HResult};
use std::ffi::{c_long, c_void, CStr};
use std::rc::Rc;

/// Exposes one item of a data transfer to native code.
pub(crate) struct DataTransferItemToFrnClipboardDataItemWrapper {
    item: Rc<dyn IDataTransferItem>,
}

impl DataTransferItemToFrnClipboardDataItemWrapper {
    pub(crate) fn new(item: Rc<dyn IDataTransferItem>) -> Self {
        Self { item }
    }

    fn find_data_format(&self, native_format: &str) -> Option<DataFormat> {
        self.item
            .formats()
            .iter()
            .find(|format| format.kind() != DataFormatKind::InProcess && to_native_format(format) == native_format)
            .cloned()
    }

    /// The value of the item for a native format. The kind of value follows
    /// the type of the stored value: a string is handed over as a string,
    /// bytes as bytes and a bitmap as its PNG encoding.
    fn get_value_core(&self, format: &str) -> Option<ComPtr<IFrnClipboardDataValue>> {
        if let Some(data_format) = self.find_data_format(format) {
            if DataFormat::text() == data_format {
                let text = self.item.try_get_value(&DataFormat::text()).unwrap_or_default();
                return Some(IFrnClipboardDataValue::from_impl(StringValue(text)));
            }

            if DataFormat::bitmap() == data_format {
                let bitmap = self.item.try_get_value(&DataFormat::bitmap())?;
                let mut encoded = Vec::new();
                if let Err(error) = bitmap.save(&mut encoded, &BitmapEncoderOptions::Png(PngBitmapEncoderOptions::DEFAULT))
                {
                    panic!("Unable to save the bitmap: {error}");
                }
                return Some(IFrnClipboardDataValue::from_impl(BytesValue(Rc::from(encoded))));
            }

            if let Some(value) = self.item.try_get_raw(&data_format) {
                if let Some(string_value) = value.downcast_ref::<String>() {
                    return Some(IFrnClipboardDataValue::from_impl(StringValue(string_value.clone())));
                }

                if let Some(bytes) = value.downcast_ref::<Rc<[u8]>>() {
                    return Some(IFrnClipboardDataValue::from_impl(BytesValue(bytes.clone())));
                }
            } else {
                return None;
            }
        }

        if let Some(logger) = Logger::try_get(LogEventLevel::Warning, LogArea::MACOS_PLATFORM) {
            logger.log(None, &format!("Unsupported data format {format}"));
        }

        None
    }
}

impl IFrnClipboardDataItemImpl for DataTransferItemToFrnClipboardDataItemWrapper {
    fn provide_formats(&self) -> Result<Option<ComPtr<IFrnStringArray>>, HResult> {
        crate::callback_base::guard(Err(HResult::FAIL), || {
            let formats = self.item.formats();
            let native_formats =
                formats.iter().filter(|f| f.kind() != DataFormatKind::InProcess).map(to_native_format);
            Ok(Some(IFrnStringArray::from_impl(FrnStringArray::new(native_formats))))
        })
    }

    fn get_value(&self, format: Option<&CStr>) -> Result<Option<ComPtr<IFrnClipboardDataValue>>, HResult> {
        crate::callback_base::guard(Err(HResult::FAIL), || {
            let format = format.map(|format| format.to_string_lossy()).unwrap_or_default();
            Ok(self.get_value_core(&format))
        })
    }
}

struct StringValue(String);

impl IFrnClipboardDataValueImpl for StringValue {
    fn is_string(&self) -> bool {
        true
    }

    fn as_string(&self) -> Option<ComPtr<IFrnString>> {
        Some(IFrnString::from_impl(FrnString::new(&self.0)))
    }

    // Asking a string value for bytes is an invalid operation; native code
    // is answered with "no bytes" instead of a panic it cannot receive.
    fn get_byte_length(&self) -> c_long {
        0
    }

    fn copy_bytes_to(&self, _buffer: *mut c_void) {}
}

struct BytesValue(Rc<[u8]>);

impl IFrnClipboardDataValueImpl for BytesValue {
    fn is_string(&self) -> bool {
        false
    }

    // Asking a bytes value for a string is an invalid operation; see above.
    fn as_string(&self) -> Option<ComPtr<IFrnString>> {
        None
    }

    fn get_byte_length(&self) -> c_long {
        self.0.len() as c_long
    }

    fn copy_bytes_to(&self, buffer: *mut c_void) {
        // SAFETY: native code passes a buffer of `get_byte_length` bytes.
        unsafe { std::ptr::copy_nonoverlapping(self.0.as_ptr(), buffer as *mut u8, self.0.len()) };
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frn_string::{frn_string_array_to_vec, frn_string_to_string};
    use ferroui_base::input::platform::PlatformDataTransferItem;

    fn wrap(item: Rc<PlatformDataTransferItem>) -> ComPtr<IFrnClipboardDataItem> {
        IFrnClipboardDataItem::from_impl(DataTransferItemToFrnClipboardDataItemWrapper::new(item))
    }

    #[test]
    fn text_items_are_offered_as_utf8_plain_text() {
        let item = wrap(PlatformDataTransferItem::create(&DataFormat::text(), "héllo".to_string()));

        let formats = item.provide_formats().unwrap().unwrap();
        assert_eq!(frn_string_array_to_vec(&formats), ["public.utf8-plain-text"]);

        let value = item.get_value(Some(c"public.utf8-plain-text")).unwrap().expect("value");
        assert!(value.is_string());
        assert_eq!(frn_string_to_string(&value.as_string().unwrap()).as_deref(), Some("héllo"));

        // A format the item does not have.
        assert!(item.get_value(Some(c"public.png")).unwrap().is_none());
    }

    #[test]
    fn bytes_items_are_copied_out() {
        let format = DataFormat::create_bytes_application_format("blob");
        let bytes: Rc<[u8]> = Rc::from(vec![1u8, 2, 3, 4, 5]);
        let item = wrap(PlatformDataTransferItem::create(&format, bytes));

        let formats = item.provide_formats().unwrap().unwrap();
        assert_eq!(frn_string_array_to_vec(&formats), ["net.ferroui.app.uti.blob"]);

        let value = item.get_value(Some(c"net.ferroui.app.uti.blob")).unwrap().expect("value");
        assert!(!value.is_string());
        assert_eq!(value.get_byte_length(), 5);
        let mut buffer = [0u8; 5];
        // SAFETY: the buffer is as long as the value says it is.
        unsafe { value.copy_bytes_to(buffer.as_mut_ptr() as *mut c_void) };
        assert_eq!(buffer, [1, 2, 3, 4, 5]);
    }

    #[test]
    fn in_process_formats_are_not_offered() {
        let format = DataFormat::create_in_process_format::<String>("private");
        let item = wrap(PlatformDataTransferItem::create(&format, "x".to_string()));
        let formats = item.provide_formats().unwrap().unwrap();
        assert!(frn_string_array_to_vec(&formats).is_empty());
    }
}
