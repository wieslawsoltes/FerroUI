//! Wraps the `ClipDescription` of a drag in progress into a data transfer.

use super::android_data_format_helper::AndroidDataFormatHelper;
use super::clip_data_to_data_transfer_wrapper::description_mime_types;
use crate::interop::java::JavaRef;
use ferroui_base::input::platform::{
    ClipboardError, PlatformDataTransfer, PlatformDataTransferImpl, PlatformDataTransferItem,
};
use ferroui_base::input::DataFormat;
use std::rc::Rc;

/// The formats of a drag that has not dropped. The system gives the data of
/// a drag only with the drop, so the other drag events carry this wrapper:
/// the formats of the description, and no items.
pub(crate) struct ClipDescriptionToDataTransferWrapper {
    formats: Vec<DataFormat>,
}

impl ClipDescriptionToDataTransferWrapper {
    /// The formats are read now: the description belongs to the event the
    /// system passed, which is only valid during the call.
    #[allow(clippy::new_ret_no_self)]
    pub fn new(description: Option<&dyn JavaRef>) -> Rc<PlatformDataTransfer> {
        let formats = description.map_or_else(Vec::new, |description| {
            let mime_types = description_mime_types(description);
            AndroidDataFormatHelper::mime_types_to_data_formats(mime_types.iter().map(String::as_str))
        });
        PlatformDataTransfer::new(ClipDescriptionToDataTransferWrapper { formats })
    }
}

impl PlatformDataTransferImpl for ClipDescriptionToDataTransferWrapper {
    fn provide_formats(&self) -> Result<Vec<DataFormat>, ClipboardError> {
        Ok(self.formats.clone())
    }

    fn provide_items(&self) -> Result<Vec<Rc<PlatformDataTransferItem>>, ClipboardError> {
        Ok(Vec::new())
    }

    fn dispose(&self, _owner: &PlatformDataTransfer) {}
}
