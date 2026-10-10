//! Wraps an item of a `ClipData` of the system into an item of a data
//! transfer.

use super::android_data_format_helper::AndroidDataFormatHelper;
use super::clip_data_to_data_transfer_wrapper::ClipDataOwner;
use super::storage::android_storage_item::{self, AndroidStorageFile};
use crate::interop::java::{call_object, is_instance_of, string_of, JavaLocal, JavaObject, JavaValue};
use ferroui_base::input::platform::{ClipboardError, PlatformDataTransferItem, PlatformDataTransferItemImpl};
use ferroui_base::input::DataFormat;
use ferroui_base::logging::{LogArea, LogEventLevel, Logger};
use ferroui_base::media::imaging::Bitmap;
use ferroui_base::platform::storage::IStorageItem;
use std::any::Any;
use std::io::Read;
use std::rc::Rc;

/// `Intent.URI_INTENT_SCHEME`.
const URI_INTENT_SCHEME: i32 = 1;

pub(crate) struct ClipDataItemToDataTransferItemWrapper {
    /// The clip data item.
    item: JavaObject,
    /// The data transfer owning this item.
    owner: Rc<ClipDataOwner>,
}

/// The text of a `CharSequence`.
fn char_sequence_text(text: &JavaLocal) -> Option<String> {
    call_object(text, "toString", "()Ljava/lang/String;", &[]).map(|text| string_of(&text))
}

impl ClipDataItemToDataTransferItemWrapper {
    #[allow(clippy::new_ret_no_self)]
    pub fn new(item: JavaObject, owner: Rc<ClipDataOwner>) -> Rc<PlatformDataTransferItem> {
        PlatformDataTransferItem::new(ClipDataItemToDataTransferItemWrapper { item, owner })
    }

    fn try_get_string(&self) -> Option<String> {
        if let Some(text) = call_object(&self.item, "getText", "()Ljava/lang/CharSequence;", &[]) {
            return char_sequence_text(&text);
        }

        if let Some(html_text) = call_object(&self.item, "getHtmlText", "()Ljava/lang/String;", &[]) {
            return Some(string_of(&html_text));
        }

        if let Some(uri) = call_object(&self.item, "getUri", "()Landroid/net/Uri;", &[]) {
            return call_object(&uri, "toString", "()Ljava/lang/String;", &[]).map(|text| string_of(&text));
        }

        if let Some(intent) = call_object(&self.item, "getIntent", "()Landroid/content/Intent;", &[]) {
            return call_object(&intent, "toUri", "(I)Ljava/lang/String;", &[JavaValue::Int(URI_INTENT_SCHEME)])
                .map(|text| string_of(&text));
        }

        None
    }

    /// The URI of the item when its scheme is `file` or `content` and the
    /// context is an activity, with that activity.
    fn file_uri(&self) -> Option<(JavaObject, JavaObject)> {
        let file_uri = call_object(&self.item, "getUri", "()Landroid/net/Uri;", &[])?;
        let scheme = call_object(&file_uri, "getScheme", "()Ljava/lang/String;", &[]).map(|scheme| string_of(&scheme));
        if !matches!(scheme.as_deref(), Some("file" | "content")) {
            return None;
        }
        let activity = self.owner.context().filter(|context| is_instance_of(*context, "android/app/Activity"))?;
        Some((activity.clone(), file_uri.to_global()))
    }

    fn try_get_storage_item(&self) -> Option<Rc<dyn IStorageItem>> {
        let (activity, file_uri) = self.file_uri()?;
        Some(android_storage_item::create_item(&activity, file_uri))
    }

    /// The bytes of the file the item names (`storageFile.OpenRead()` read
    /// to its end); `None` when the item is no file.
    fn try_read_file(&self) -> std::io::Result<Option<Vec<u8>>> {
        let Some(storage_item) = self.try_get_storage_item() else {
            return Ok(None);
        };
        let Some(storage_file) = storage_item.as_any().and_then(|any| any.downcast_ref::<AndroidStorageFile>()) else {
            return Ok(None);
        };
        let mut stream = storage_file.open_read()?;
        let mut bytes = Vec::new();
        stream.read_to_end(&mut bytes)?;
        Ok(Some(bytes))
    }

    fn try_get_bitmap(&self) -> Option<Rc<Bitmap>> {
        let bitmap =
            self.try_read_file().and_then(|bytes| bytes.map(|bytes| Bitmap::from_stream(&mut bytes.as_slice())).transpose());
        match bitmap {
            Ok(bitmap) => bitmap.map(Rc::new),
            Err(ex) => {
                if let Some(log) = Logger::try_get(LogEventLevel::Warning, LogArea::ANDROID_PLATFORM) {
                    log.log(None, &format!("Could not get bitmap from clipboard: {ex}"));
                }
                None
            }
        }
    }

    fn try_get_bytes(&self) -> Option<Rc<[u8]>> {
        match self.try_read_file() {
            Ok(bytes) => bytes.map(Rc::from),
            Err(ex) => {
                if let Some(log) = Logger::try_get(LogEventLevel::Warning, LogArea::ANDROID_PLATFORM) {
                    log.log(None, &format!("Could not get bytes from clipboard: {ex}"));
                }
                None
            }
        }
    }
}

impl PlatformDataTransferItemImpl for ClipDataItemToDataTransferItemWrapper {
    fn provide_formats(&self) -> Result<Vec<DataFormat>, ClipboardError> {
        Ok(self.owner.formats()) // There's no "format per item", assume each item handle all formats
    }

    fn try_get_raw_core(&self, format: &DataFormat) -> Result<Option<Rc<dyn Any>>, ClipboardError> {
        if DataFormat::text() == *format {
            let context = self.owner.context();
            let text = call_object(
                &self.item,
                "coerceToText",
                "(Landroid/content/Context;)Ljava/lang/CharSequence;",
                &[JavaValue::Object(context.map(|context| context as &dyn crate::interop::java::JavaRef))],
            )
            .and_then(|text| char_sequence_text(&text));
            return Ok(text.map(|text| Rc::new(text) as Rc<dyn Any>));
        }

        if DataFormat::file() == *format {
            return Ok(self.try_get_storage_item().map(|item| Rc::new(item) as Rc<dyn Any>));
        }

        if DataFormat::bitmap() == *format {
            return Ok(self.try_get_bitmap().map(|bitmap| Rc::new(bitmap) as Rc<dyn Any>));
        }

        // The reference asks the format for the type of its values (text or bytes), which
        // a format does not carry at run time here: a format whose MIME type is text has
        // text, any other the bytes of the file the item names and, when it names none,
        // its text (docs/porting/DEVIATIONS.md).
        let mime_type = AndroidDataFormatHelper::data_format_to_mime_type(format);
        if AndroidDataFormatHelper::is_text_mime_type(&mime_type) {
            return Ok(self.try_get_string().map(|text| Rc::new(text) as Rc<dyn Any>));
        }

        if let Some(bytes) = self.try_get_bytes() {
            return Ok(Some(Rc::new(bytes)));
        }

        Ok(self.try_get_string().map(|text| Rc::new(text) as Rc<dyn Any>))
    }
}
