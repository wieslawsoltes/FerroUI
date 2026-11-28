//! Adds Chromium custom formats to a flat Linux clipboard offer.

use super::chromium_clipboard::{ChromiumClipboard, FormatAlias, MAP_ENABLED};
use super::{
    ClipboardError, PlatformAsyncDataTransfer, PlatformAsyncDataTransferImpl,
    PlatformAsyncDataTransferItem, PlatformAsyncDataTransferItemImpl,
};
use crate::input::{
    AsyncDataTransferItemExtensions, DataFormat, IAsyncDataTransfer, IAsyncDataTransferItem,
    LocalBoxFuture,
};
use std::any::Any;
use std::future::ready;
use std::rc::Rc;

struct ClipboardTransfer {
    source: Rc<dyn IAsyncDataTransfer>,
    web_item: Rc<PlatformAsyncDataTransferItem>,
    formats: Vec<DataFormat>,
}

impl PlatformAsyncDataTransferImpl for ClipboardTransfer {
    fn provide_formats(&self) -> Result<Vec<DataFormat>, ClipboardError> {
        Ok(self.formats.clone())
    }

    fn provide_items(&self) -> Result<Vec<Rc<dyn IAsyncDataTransferItem>>, ClipboardError> {
        let mut items = self.source.try_items()?.to_vec();
        items.push(self.web_item.clone());
        Ok(items)
    }

    fn dispose(&self, _: &PlatformAsyncDataTransfer) {
        self.source.dispose();
    }
}

struct WebItem {
    source: Rc<dyn IAsyncDataTransfer>,
    map: Rc<[u8]>,
    aliases: Vec<FormatAlias>,
}

impl PlatformAsyncDataTransferItemImpl for WebItem {
    fn provide_formats(&self) -> Result<Vec<DataFormat>, ClipboardError> {
        Ok(std::iter::once(ChromiumClipboard::Linux.map_format().into())
            .chain(self.aliases.iter().map(|alias| (*alias.slot).clone()))
            .collect())
    }

    fn try_get_raw_core_async(
        &self,
        format: &DataFormat,
    ) -> LocalBoxFuture<Result<Option<Rc<dyn Any>>, ClipboardError>> {
        if ChromiumClipboard::Linux.map_format() == *format {
            return Box::pin(ready(Ok(Some(Rc::new(self.map.clone()) as Rc<dyn Any>))));
        }
        let Some(alias) = self
            .aliases
            .iter()
            .find(|alias| *alias.slot == *format)
            .cloned()
        else {
            return Box::pin(ready(Ok(None)));
        };
        let source = self.source.clone();
        Box::pin(async move {
            for item in source.try_items()?.iter() {
                if item.try_contains(&alias.application)? {
                    let value = item.try_get_raw_async(&alias.application).await?;
                    return Ok(value
                        .and_then(|value| alias.encode(value.as_ref()))
                        .map(|bytes| Rc::new(bytes) as Rc<dyn Any>));
                }
            }
            Ok(None)
        })
    }
}

/// Preserves the transfer's native formats and adds Chromium's map and byte slots.
pub fn with_chromium_clipboard_formats(
    source: Rc<dyn IAsyncDataTransfer>,
) -> Result<Rc<dyn IAsyncDataTransfer>, ClipboardError> {
    if !MAP_ENABLED {
        return Ok(source);
    }
    let formats = source.try_formats()?;
    let Some(map) = ChromiumClipboard::Linux.create_map(&formats) else {
        return Ok(source);
    };
    let web_item = PlatformAsyncDataTransferItem::new(WebItem {
        source: source.clone(),
        map: map.bytes,
        aliases: map.aliases,
    });
    let formats = formats
        .iter()
        .chain(web_item.try_formats()?.iter())
        .cloned()
        .collect();
    Ok(PlatformAsyncDataTransfer::new(ClipboardTransfer {
        source,
        web_item,
        formats,
    }))
}
