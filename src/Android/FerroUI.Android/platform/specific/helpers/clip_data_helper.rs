//! A data transfer of the framework as a `ClipData` of the system: what the
//! clipboard is set to and what a drag that starts in the application
//! carries (`ClipDataHelper` of the reference, which moved there from the
//! clipboard).

use crate::interop::java::{
    call_static_object, call_void, new_object, new_string_array, JavaClass, JavaLocal, JavaObject, JavaRef, JavaValue,
};
use crate::platform::android_data_format_helper::AndroidDataFormatHelper;
use ferroui_base::input::platform::ClipboardError;
use ferroui_base::input::{DataFormat, DataFormatKind, IAsyncDataTransfer, IAsyncDataTransferItem};
use ferroui_base::logging::{LogArea, LogEventLevel, Logger};
use ferroui_base::platform::storage::IStorageItem;
use std::rc::Rc;

pub(crate) const CLIP_DATA: &str = "android/content/ClipData";
const CLIP_DATA_ITEM: &str = "android/content/ClipData$Item";

pub(crate) struct ClipDataHelper;

impl ClipDataHelper {
    /// The clip data of `data_transfer`: its formats that leave the process
    /// as MIME types and one item per item with a value the system takes.
    /// `None` when no item has such a value.
    pub(crate) async fn get_clip_data(
        data_transfer: Rc<dyn IAsyncDataTransfer>,
    ) -> Result<Option<JavaObject>, ClipboardError> {
        let mime_types: Vec<String> = data_transfer
            .formats()
            .iter()
            .filter(|f| f.kind() != DataFormatKind::InProcess)
            .map(AndroidDataFormatHelper::data_format_to_mime_type)
            .collect();

        // The items are global references: a local reference does not live across an await.
        let mut first_item: Option<JavaObject> = None;
        let mut additional_items: Option<Vec<JavaObject>> = None;

        for data_transfer_item in data_transfer.items().iter() {
            let Some(clip_data_item) = Self::try_create_data_item_async(data_transfer_item.clone()).await? else {
                continue;
            };

            if first_item.is_none() {
                first_item = Some(clip_data_item);
            } else {
                additional_items.get_or_insert_with(Vec::new).push(clip_data_item);
            }
        }

        let Some(first_item) = first_item else {
            return Ok(None);
        };

        let mime_types = new_string_array(&mime_types);
        let clip_data = new_object(
            &JavaClass::find(CLIP_DATA),
            "(Ljava/lang/CharSequence;[Ljava/lang/String;Landroid/content/ClipData$Item;)V",
            &[JavaValue::Object(None), JavaValue::Object(Some(&mime_types)), JavaValue::Object(Some(&first_item))],
        );

        if let Some(additional_items) = additional_items {
            for additional_item in &additional_items {
                call_void(
                    &clip_data,
                    "addItem",
                    "(Landroid/content/ClipData$Item;)V",
                    &[JavaValue::Object(Some(additional_item))],
                );
            }
        }

        Ok(Some(clip_data.to_global()))
    }

    /// `new ClipData.Item(text)`.
    fn text_item(text: Option<&str>) -> JavaObject {
        let text: Option<JavaLocal> = text.map(crate::interop::java::new_string);
        new_object(
            &JavaClass::find(CLIP_DATA_ITEM),
            "(Ljava/lang/CharSequence;)V",
            &[JavaValue::Object(text.as_ref().map(|text| text as &dyn JavaRef))],
        )
        .to_global()
    }

    async fn try_create_data_item_async(
        item: Rc<dyn IAsyncDataTransferItem>,
    ) -> Result<Option<JavaObject>, ClipboardError> {
        let mut has_formats = false;

        // Create the item from the first format returning a supported value.
        for data_format in item.formats().iter() {
            if data_format.kind() == DataFormatKind::InProcess {
                continue;
            }

            has_formats = true;

            if DataFormat::text() == *data_format {
                let text = item.try_get_raw_async(&DataFormat::text()).await?;
                let text = text.as_ref().and_then(|text| text.downcast_ref::<String>());
                return Ok(Some(Self::text_item(text.map(String::as_str))));
            }

            if DataFormat::file() == *data_format {
                let storage_item = item.try_get_raw_async(&DataFormat::file()).await?;
                let Some(storage_item) =
                    storage_item.as_ref().and_then(|item| item.downcast_ref::<Rc<dyn IStorageItem>>())
                else {
                    continue;
                };

                let uri = call_static_object(
                    &JavaClass::find("android/net/Uri"),
                    "parse",
                    "(Ljava/lang/String;)Landroid/net/Uri;",
                    &[JavaValue::String(&storage_item.path().to_string())],
                );
                return Ok(Some(
                    new_object(
                        &JavaClass::find(CLIP_DATA_ITEM),
                        "(Landroid/net/Uri;)V",
                        &[JavaValue::Object(uri.as_ref().map(|uri| uri as &dyn JavaRef))],
                    )
                    .to_global(),
                ));
            }

            // `dataFormat is DataFormat<string>`: a format does not carry the type of its
            // values at run time here, so the value says whether it is text.
            let value = item.try_get_raw_async(data_format).await?;
            if let Some(string_value) = value.as_ref().and_then(|value| value.downcast_ref::<String>()) {
                return Ok(Some(Self::text_item(Some(string_value))));
            }
        }

        if has_formats {
            if let Some(log) = Logger::try_get(LogEventLevel::Warning, LogArea::ANDROID_PLATFORM) {
                let formats: Vec<String> = item.formats().iter().map(ToString::to_string).collect();
                log.log(
                    None,
                    &format!("No compatible value found for data transfer item with formats {}", formats.join(", ")),
                );
            }
        }

        Ok(None)
    }
}
