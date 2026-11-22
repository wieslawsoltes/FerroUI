use crate::data_transfer_item_to_frn_clipboard_data_item_wrapper::DataTransferItemToFrnClipboardDataItemWrapper;
use crate::interop::*;
use ferroui_base::input::platform::chromium_clipboard::ChromiumClipboard;
use ferroui_base::input::{DataFormat, DataFormatKind, IDataTransfer};
use ferroui_microcom::{ComPtr, HResult};
use std::cell::RefCell;
use std::rc::Rc;

/// Exposes a data transfer to native code as a clipboard data source. The
/// data transfer is disposed when native code releases the source.
pub(crate) struct DataTransferToFrnClipboardDataSourceWrapper {
    data_transfer: RefCell<Option<Rc<dyn IDataTransfer>>>,
    items: RefCell<Option<Vec<ComPtr<IFrnClipboardDataItem>>>>,
    publish_web_formats: bool,
}

impl DataTransferToFrnClipboardDataSourceWrapper {
    pub(crate) fn new(data_transfer: Rc<dyn IDataTransfer>) -> Self {
        Self {
            data_transfer: RefCell::new(Some(data_transfer)),
            items: RefCell::new(None),
            publish_web_formats: false,
        }
    }

    pub(crate) fn for_clipboard(data_transfer: Rc<dyn IDataTransfer>) -> Self {
        let mut source = Self::new(data_transfer);
        source.publish_web_formats = true;
        source
    }

    fn data_transfer(&self) -> Rc<dyn IDataTransfer> {
        match self.data_transfer.borrow().clone() {
            Some(data_transfer) => data_transfer,
            None => panic!("Cannot access a disposed object: DataTransferToFrnClipboardDataSourceWrapper"),
        }
    }

    fn items(&self) -> Vec<ComPtr<IFrnClipboardDataItem>> {
        if let Some(items) = self.items.borrow().as_ref() {
            return items.clone();
        }

        let transfer = self.data_transfer();
        let source_items = transfer.items();
        let clipboard = ChromiumClipboard::MacOs;
        let mut web_item = if self.publish_web_formats {
            let mut formats: Vec<_> = transfer
                .formats()
                .iter()
                .filter(|format| format.kind() == DataFormatKind::Platform)
                .cloned()
                .collect();
            let reserved = formats.len();
            let first_text_item = source_items.iter().position(|item| {
                item.formats()
                    .iter()
                    .any(|format| DataFormat::text() == *format)
            });
            source_items
                .iter()
                .enumerate()
                .filter(|(index, _)| first_text_item.is_none_or(|text_index| *index == text_index))
                .find_map(|(index, item)| {
                    formats.truncate(reserved);
                    formats.extend(
                        item.formats()
                            .iter()
                            .filter(|format| format.kind() == DataFormatKind::Application)
                            .cloned(),
                    );
                    clipboard.create_map(&formats).map(|map| (index, map))
                })
        } else {
            None
        };
        // Chromium reads each type from the first item that offers it. Keep
        // a text item's map and data slots together with that text.
        let items: Vec<_> = source_items
            .iter()
            .enumerate()
            .map(|(index, item)| {
                let map = if web_item.as_ref().is_some_and(|(owner, _)| *owner == index) {
                    web_item.take().map(|(_, map)| map)
                } else {
                    None
                };
                IFrnClipboardDataItem::from_impl(
                    DataTransferItemToFrnClipboardDataItemWrapper::new(item.clone(), map),
                )
            })
            .collect();
        *self.items.borrow_mut() = Some(items.clone());
        if items.is_empty() {
            self.destroyed();
        }

        items
    }

    fn destroyed(&self) {
        let data_transfer = self.data_transfer.borrow_mut().take();
        if let Some(data_transfer) = data_transfer {
            data_transfer.dispose();
        }
    }
}

impl Drop for DataTransferToFrnClipboardDataSourceWrapper {
    fn drop(&mut self) {
        self.destroyed();
    }
}

impl IFrnClipboardDataSourceImpl for DataTransferToFrnClipboardDataSourceWrapper {
    fn get_item_count(&self) -> i32 {
        crate::callback_base::guard(0, || self.items().len() as i32)
    }

    fn get_item(&self, index: i32) -> Result<Option<ComPtr<IFrnClipboardDataItem>>, HResult> {
        crate::callback_base::guard(Err(HResult::FAIL), || {
            match self.items().get(index as usize) {
                Some(item) if index >= 0 => Ok(Some(item.clone())),
                _ => panic!("Index was outside the bounds of the array."),
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frn_string::frn_string_array_to_vec;
    use ferroui_base::input::{DataTransfer, DataTransferItem};
    use std::ffi::c_void;

    #[test]
    fn clipboard_publishes_one_map_with_the_first_text_items_slots() {
        let map = "org.w3.web-custom-format.map";
        let source = DataTransfer::new();
        let occupied = DataFormat::create_bytes_platform_format("org.w3.web-custom-format.type-0");
        source.add(DataTransferItem::create(&occupied, Some(Rc::from([0xa5]))));
        let item = DataTransferItem::new();
        item.set_text(Some("ab\ncd"));
        let marker = DataFormat::create_string_application_format("Box");
        item.set(&marker, Some("1".to_owned()));
        source.add(item);
        let last = DataTransferItem::new();
        last.set_text(Some("separate"));
        last.set(
            &DataFormat::create_string_application_format("Whole"),
            Some("1".to_owned()),
        );
        source.add(last);

        let native = IFrnClipboardDataSource::from_impl(
            DataTransferToFrnClipboardDataSourceWrapper::for_clipboard(source.clone()),
        );
        assert_eq!(native.get_item_count(), 3);
        let formats = |index| {
            let item = native.get_item(index).unwrap().unwrap();
            frn_string_array_to_vec(&item.provide_formats().unwrap().unwrap())
        };
        assert_eq!(formats(0), ["org.w3.web-custom-format.type-0"]);
        assert_eq!(
            formats(1),
            [
                "public.utf8-plain-text",
                "net.ferroui.app.uti.Box",
                map,
                "org.w3.web-custom-format.type-1"
            ]
        );
        assert_eq!(
            formats(2),
            ["public.utf8-plain-text", "net.ferroui.app.uti.Whole"]
        );
        let item = native.get_item(1).unwrap().unwrap();
        let value = item
            .get_value(Some(c"org.w3.web-custom-format.type-1"))
            .unwrap()
            .unwrap();
        assert!(!value.is_string());
        assert_eq!(value.get_byte_length(), 1);
        let mut byte = 0u8;
        // SAFETY: the destination is as large as the data value.
        unsafe { value.copy_bytes_to((&mut byte as *mut u8).cast::<c_void>()) };
        assert_eq!(byte, b'1');

        let drag = IFrnClipboardDataSource::from_impl(
            DataTransferToFrnClipboardDataSourceWrapper::new(source),
        );
        let item = drag.get_item(1).unwrap().unwrap();
        let formats = item.provide_formats().unwrap().unwrap();
        assert_eq!(
            frn_string_array_to_vec(&formats),
            ["public.utf8-plain-text", "net.ferroui.app.uti.Box"]
        );
    }
}
