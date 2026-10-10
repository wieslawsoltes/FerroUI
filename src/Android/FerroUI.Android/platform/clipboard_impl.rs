//! The clipboard of the system.

use super::android_data_format_helper::AndroidDataFormatHelper;
use super::clip_data_to_data_transfer_wrapper::ClipDataToDataTransferWrapper;
use crate::interop::java::{
    call_object, call_static_object, call_void, new_object, new_string_array, JavaClass, JavaLocal, JavaObject,
    JavaRef, JavaValue,
};
use crate::interop::natives::sdk_int;
use ferroui_base::input::platform::{ClipboardError, IClipboardImpl, PlatformDataTransfer};
use ferroui_base::input::{DataFormat, DataFormatKind, IAsyncDataTransfer, IAsyncDataTransferItem, LocalBoxFuture};
use ferroui_base::logging::{LogArea, LogEventLevel, Logger};
use ferroui_base::platform::storage::IStorageItem;
use std::panic::{catch_unwind, resume_unwind, AssertUnwindSafe};
use std::rc::Rc;

const CLIP_DATA: &str = "android/content/ClipData";
const CLIP_DATA_ITEM: &str = "android/content/ClipData$Item";

/// Runs `operation` now and returns its outcome as a completed future; a
/// panic (an exception of a call into the system) is raised when the
/// future is awaited, like a faulted task.
fn completed<T: 'static>(
    operation: impl FnOnce() -> Result<T, ClipboardError>,
) -> LocalBoxFuture<Result<T, ClipboardError>> {
    match catch_unwind(AssertUnwindSafe(operation)) {
        Ok(value) => Box::pin(std::future::ready(value)),
        Err(payload) => Box::pin(async move { resume_unwind(payload) }),
    }
}

pub(crate) struct ClipboardImpl {
    clipboard_manager: Option<JavaObject>,
    context: Option<JavaObject>,
}

impl ClipboardImpl {
    /// The clipboard of `context` (`context.GetSystemService(Context.ClipboardService)`).
    pub fn new(context: &JavaObject) -> Rc<ClipboardImpl> {
        let clipboard_manager = call_object(
            context,
            "getSystemService",
            "(Ljava/lang/String;)Ljava/lang/Object;",
            &[JavaValue::String("clipboard")],
        )
        .map(|manager| manager.to_global());
        Rc::new(ClipboardImpl { clipboard_manager, context: Some(context.clone()) })
    }

    fn try_get_data(&self) -> Option<Rc<PlatformDataTransfer>> {
        let clip_data = call_object(
            self.clipboard_manager.as_ref()?,
            "getPrimaryClip",
            "()Landroid/content/ClipData;",
            &[],
        )?;
        Some(ClipDataToDataTransferWrapper::new(clip_data.to_global(), self.context.clone()))
    }

    async fn set_data(
        clipboard_manager: Option<JavaObject>,
        data_transfer: Rc<dyn IAsyncDataTransfer>,
    ) -> Result<(), ClipboardError> {
        let Some(clipboard_manager) = clipboard_manager else {
            return Ok(());
        };

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
            Self::clear(Some(&clipboard_manager));
            return Ok(());
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

        call_void(
            &clipboard_manager,
            "setPrimaryClip",
            "(Landroid/content/ClipData;)V",
            &[JavaValue::Object(Some(&clip_data))],
        );
        Ok(())
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
                    &[JavaValue::String(storage_item.path().original_string())],
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
                log.log(None, &format!(
                    "No compatible value found for data transfer item with formats {}",
                    formats.join(", ")
                ));
            }
        }

        Ok(None)
    }

    fn clear(clipboard_manager: Option<&JavaObject>) {
        let Some(clipboard_manager) = clipboard_manager else {
            return;
        };

        if sdk_int() >= 28 {
            call_void(clipboard_manager, "clearPrimaryClip", "()V", &[]);
        } else {
            let empty = crate::interop::java::new_string("");
            let clip_data = call_static_object(
                &JavaClass::find(CLIP_DATA),
                "newPlainText",
                "(Ljava/lang/CharSequence;Ljava/lang/CharSequence;)Landroid/content/ClipData;",
                &[JavaValue::Object(None), JavaValue::Object(Some(&empty))],
            );
            call_void(
                clipboard_manager,
                "setPrimaryClip",
                "(Landroid/content/ClipData;)V",
                &[JavaValue::Object(clip_data.as_ref().map(|clip_data| clip_data as &dyn JavaRef))],
            );
        }
    }
}

impl IClipboardImpl for ClipboardImpl {
    fn try_get_data_async(&self) -> LocalBoxFuture<Result<Option<Rc<dyn IAsyncDataTransfer>>, ClipboardError>> {
        completed(|| Ok(self.try_get_data().map(|data| data as Rc<dyn IAsyncDataTransfer>)))
    }

    fn set_data_async(&self, data_transfer: Rc<dyn IAsyncDataTransfer>) -> LocalBoxFuture<Result<(), ClipboardError>> {
        Box::pin(Self::set_data(self.clipboard_manager.clone(), data_transfer))
    }

    fn clear_async(&self) -> LocalBoxFuture<Result<(), ClipboardError>> {
        completed(|| {
            Self::clear(self.clipboard_manager.as_ref());
            Ok(())
        })
    }
}
