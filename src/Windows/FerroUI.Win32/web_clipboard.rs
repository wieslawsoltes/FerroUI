//! The Windows OLE adapter for Chromium's custom clipboard format map.

use crate::ole_data_object_helper;
use crate::win32_com::IDataObject;
use ferroui_base::input::platform::chromium_clipboard::{
    ChromiumClipboard, FormatAlias, MAX_MAP_BYTES,
};
use ferroui_base::input::platform::ClipboardError;
use ferroui_base::input::{
    DataFormat, DataTransferExtensions, DataTransferItem, IDataTransfer, IDataTransferItem,
};
use ferroui_base::reactive::IDisposable;
use std::any::Any;
use std::rc::Rc;

struct ClipboardTransfer {
    source: Rc<dyn IDataTransfer>,
    web_item: Rc<DataTransferItem>,
    formats: Rc<[DataFormat]>,
}

impl IDataTransfer for ClipboardTransfer {
    fn formats(&self) -> Rc<[DataFormat]> {
        self.formats.clone()
    }

    fn items(&self) -> Rc<[Rc<dyn IDataTransferItem>]> {
        self.source
            .items()
            .iter()
            .cloned()
            .chain(std::iter::once(
                self.web_item.clone() as Rc<dyn IDataTransferItem>
            ))
            .collect()
    }
}

impl IDisposable for ClipboardTransfer {
    fn dispose(&self) {
        self.source.dispose();
    }
}

/// Adds browser aliases while preserving the native formats and their values.
pub(crate) fn publish(
    source: Rc<dyn IDataTransfer>,
    original: &[DataFormat],
) -> Rc<dyn IDataTransfer> {
    let Some(map) = ChromiumClipboard::Windows.create_map(original) else {
        return source;
    };
    let item = DataTransferItem::new();
    for alias in map.aliases {
        let source = source.clone();
        let slot = alias.slot.clone();
        item.set_with(&slot, move || {
            let value = source
                .get_items(&alias.application)
                .next()?
                .try_get_raw(&alias.application)?;
            alias.encode(value.as_ref())
        });
    }

    item.set(&ChromiumClipboard::Windows.map_format(), Some(map.bytes));
    let formats = original
        .iter()
        .chain(item.formats().iter())
        .cloned()
        .collect();
    Rc::new(ClipboardTransfer {
        source,
        web_item: item,
        formats,
    })
}

pub(crate) fn read_map(object: &IDataObject, originals: &[DataFormat]) -> Vec<FormatAlias> {
    let clipboard = ChromiumClipboard::Windows;
    let Some(map) = clipboard.find_map(originals) else {
        return Vec::new();
    };
    let Some(data) = ole_data_object_helper::try_get_bounded_bytes(object, &map, MAX_MAP_BYTES)
    else {
        return Vec::new();
    };
    clipboard.parse_map(&data, originals)
}

pub(crate) fn read_alias(
    object: &IDataObject,
    alias: &FormatAlias,
) -> Result<Option<Rc<dyn Any>>, ClipboardError> {
    Ok(ole_data_object_helper::try_get(object, &alias.slot)?.and_then(|value| alias.decode(value)))
}
