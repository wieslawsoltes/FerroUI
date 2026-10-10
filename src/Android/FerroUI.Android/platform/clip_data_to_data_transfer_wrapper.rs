//! Wraps a `ClipData` of the system into a data transfer.

use super::android_data_format_helper::AndroidDataFormatHelper;
use super::clip_data_item_to_data_transfer_item_wrapper::ClipDataItemToDataTransferItemWrapper;
use crate::interop::java::{call_int, call_object, string_of, JavaObject, JavaValue};
use ferroui_base::input::platform::{
    ClipboardError, PlatformDataTransfer, PlatformDataTransferImpl, PlatformDataTransferItem,
};
use ferroui_base::input::DataFormat;
use std::rc::Rc;

/// What the items of a clip data share with it: the clip data, the context
/// and the MIME types of its description.
pub(crate) struct ClipDataOwner {
    clip_data: JavaObject,
    /// The application context.
    context: Option<JavaObject>,
}

impl ClipDataOwner {
    pub fn context(&self) -> Option<&JavaObject> {
        self.context.as_ref()
    }

    /// The MIME types of the description of the clip data.
    pub fn mime_types(&self) -> Vec<String> {
        let Some(description) =
            call_object(&self.clip_data, "getDescription", "()Landroid/content/ClipDescription;", &[])
        else {
            return Vec::new();
        };
        let count = call_int(&description, "getMimeTypeCount", "()I", &[]);
        (0..count.max(0))
            .map(|i| {
                call_object(&description, "getMimeType", "(I)Ljava/lang/String;", &[JavaValue::Int(i)])
                    .map(|mime_type| string_of(&mime_type))
                    .unwrap_or_default()
            })
            .collect()
    }

    /// `Formats` of the data transfer.
    pub fn formats(&self) -> Vec<DataFormat> {
        let mime_types = self.mime_types();
        if mime_types.is_empty() {
            return Vec::new();
        }

        let mut formats = Vec::with_capacity(mime_types.len() + 1);

        let mut has_image = false;
        for mime_type in &mime_types {
            let format = AndroidDataFormatHelper::mime_type_to_data_format(mime_type);

            if !has_image {
                has_image =
                    format.identifier().as_bytes().get(..6).is_some_and(|start| start.eq_ignore_ascii_case(b"image/"));
            }
            formats.push(format);
        }

        if has_image {
            formats.push(DataFormat::bitmap().into());
        }

        formats
    }
}

pub(crate) struct ClipDataToDataTransferWrapper {
    owner: Rc<ClipDataOwner>,
}

impl ClipDataToDataTransferWrapper {
    #[allow(clippy::new_ret_no_self)]
    pub fn new(clip_data: JavaObject, context: Option<JavaObject>) -> Rc<PlatformDataTransfer> {
        PlatformDataTransfer::new(ClipDataToDataTransferWrapper { owner: Rc::new(ClipDataOwner { clip_data, context }) })
    }
}

impl PlatformDataTransferImpl for ClipDataToDataTransferWrapper {
    fn provide_formats(&self) -> Result<Vec<DataFormat>, ClipboardError> {
        Ok(self.owner.formats())
    }

    fn provide_items(&self) -> Result<Vec<Rc<PlatformDataTransferItem>>, ClipboardError> {
        let count = call_int(&self.owner.clip_data, "getItemCount", "()I", &[]);
        let mut items = Vec::with_capacity(count.max(0) as usize);

        for i in 0..count {
            let item = call_object(
                &self.owner.clip_data,
                "getItemAt",
                "(I)Landroid/content/ClipData$Item;",
                &[JavaValue::Int(i)],
            );
            if let Some(item) = item {
                items.push(ClipDataItemToDataTransferItemWrapper::new(item.to_global(), self.owner.clone()));
            }
        }

        Ok(items)
    }

    fn dispose(&self, _owner: &PlatformDataTransfer) {}
}
