use crate::clipboard_data_format_helper::{to_data_formats, to_native_format};
use crate::clipboard_read_session::ClipboardReadSession;
use crate::frn_string::{frn_string_bytes, frn_string_to_string};
use ferroui_base::input::platform::{ClipboardError, PlatformDataTransferItem, PlatformDataTransferItemImpl};
use ferroui_base::input::DataFormat;
use crate::storage_provider_api::{NativeStorageItem, StorageProviderApi};
use ferroui_base::media::imaging::Bitmap;
use ferroui_base::platform::storage::IStorageItem;
use ferroui_base::utilities::{Uri, UriKind};
use ferroui_base::{FerroLocator, LocatorExtensions};
use std::any::Any;
use std::rc::Rc;

/// Represents a single item inside a clipboard data transfer. The session
/// is shared with the data transfer, which disposes it.
pub(crate) struct ClipboardDataTransferItem {
    session: Rc<ClipboardReadSession>,
    item_index: i32,
}

/// The UTF-16 (little endian) bytes of a string.
fn utf16_bytes(s: &str) -> Vec<u8> {
    s.encode_utf16().flat_map(u16::to_le_bytes).collect()
}

/// The string encoded by UTF-16 (little endian) bytes; a trailing odd byte
/// and invalid sequences become replacement characters.
fn utf16_string(bytes: &[u8]) -> String {
    let units = bytes.chunks_exact(2).map(|pair| u16::from_le_bytes([pair[0], pair[1]]));
    let mut s: String = char::decode_utf16(units).map(|c| c.unwrap_or(char::REPLACEMENT_CHARACTER)).collect();
    if bytes.len() % 2 != 0 {
        s.push(char::REPLACEMENT_CHARACTER);
    }
    s
}

fn try_get_file_path_uri(uri_string: Option<&str>, storage_api: &StorageProviderApi) -> Option<Uri> {
    let uri = Uri::try_create(uri_string?, UriKind::Absolute).filter(|uri| uri.scheme() == "file")?;

    // macOS may return a file reference URI (e.g. file:///.file/id=6571367.2773272/), convert it to a path URI.
    if uri.absolute_path().starts_with("/.file/id=") {
        storage_api.try_resolve_file_reference_uri(&uri)
    } else {
        Some(uri)
    }
}

impl ClipboardDataTransferItem {
    pub(crate) fn new(session: Rc<ClipboardReadSession>, item_index: i32) -> Rc<PlatformDataTransferItem> {
        PlatformDataTransferItem::new(ClipboardDataTransferItem { session, item_index })
    }

    /// A bitmap that cannot be decoded is a failure of the read, of the
    /// kind a clipboard is not expected to report (the reference throws
    /// the exception of the bitmap constructor).
    fn try_get_bitmap(&self, native_format: &str) -> Result<Option<Rc<Bitmap>>, ClipboardError> {
        let Some(bytes) = self.session.get_item_value_as_bytes(self.item_index, native_format)? else {
            return Ok(None);
        };
        let bytes = frn_string_bytes(&bytes);
        match Bitmap::from_stream(&mut std::io::Cursor::new(bytes)) {
            Ok(bitmap) => Ok(Some(Rc::new(bitmap))),
            Err(error) => Err(ClipboardError::other(format!("Unable to load the bitmap: {error}"))),
        }
    }

    fn try_get_string(&self, native_format: &str) -> Result<Option<String>, ClipboardError> {
        let text = self.session.get_item_value_as_string(self.item_index, native_format)?;
        Ok(text.and_then(|text| frn_string_to_string(&text)))
    }

    fn try_get_file(&self, native_format: &str) -> Result<Option<Rc<dyn IStorageItem>>, ClipboardError> {
        let Some(storage_api) = FerroLocator::current().get_service::<StorageProviderApi>() else {
            return Ok(None);
        };

        let uri_string = self.session.get_item_value_as_string(self.item_index, native_format)?;
        let uri_string = uri_string.and_then(|uri_string| frn_string_to_string(&uri_string));
        let Some(uri) = try_get_file_path_uri(uri_string.as_deref(), &storage_api) else {
            return Ok(None);
        };

        Ok(storage_api.try_get_storage_item(Some(&uri), false).map(NativeStorageItem::into_item))
    }

    fn try_get_bytes(&self, native_format: &str) -> Result<Option<Rc<[u8]>>, ClipboardError> {
        let bytes = self.session.get_item_value_as_bytes(self.item_index, native_format)?;
        Ok(bytes.map(|bytes| Rc::from(frn_string_bytes(&bytes))))
    }
}

impl PlatformDataTransferItemImpl for ClipboardDataTransferItem {
    fn provide_formats(&self) -> Result<Vec<DataFormat>, ClipboardError> {
        let formats = self.session.get_item_formats(self.item_index)?;
        Ok(to_data_formats(formats.as_deref(), &|format| self.session.is_text_format(format)))
    }

    /// The value type follows the format: `String` for the text format,
    /// `Rc<Bitmap>` for the bitmap format, and for every other format
    /// `String` when the native side says the format is textual and
    /// `Rc<[u8]>` when not — the same rule the formats of the item were
    /// created with.
    ///
    /// A failed native read is the error of the read: an asynchronous
    /// reader (the clipboard) gets it, a synchronous one (a drop target)
    /// panics with it.
    fn try_get_raw_core(&self, format: &DataFormat) -> Result<Option<Rc<dyn Any>>, ClipboardError> {
        let native_format = to_native_format(format);

        if DataFormat::text() == *format {
            return Ok(self.try_get_string(&native_format)?.map(|value| Rc::new(value) as Rc<dyn Any>));
        }

        if DataFormat::file() == *format {
            return Ok(self.try_get_file(&native_format)?.map(|value| Rc::new(value) as Rc<dyn Any>));
        }

        if DataFormat::bitmap() == *format {
            return Ok(self.try_get_bitmap(&native_format)?.map(|value| Rc::new(value) as Rc<dyn Any>));
        }

        if self.session.is_text_format(&native_format) {
            if let Some(string_value) = self.try_get_string(&native_format)? {
                return Ok(Some(Rc::new(string_value)));
            }

            if let Some(bytes) = self.try_get_bytes(&native_format)? {
                return Ok(Some(Rc::new(utf16_string(&bytes))));
            }

            return Ok(None);
        }

        if let Some(bytes) = self.try_get_bytes(&native_format)? {
            return Ok(Some(Rc::new(bytes)));
        }

        if let Some(string_value) = self.try_get_string(&native_format)? {
            let bytes: Rc<[u8]> = Rc::from(utf16_bytes(&string_value));
            return Ok(Some(Rc::new(bytes)));
        }

        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn utf16_conversions_round_trip() {
        let bytes = utf16_bytes("aé😀");
        assert_eq!(bytes.len(), 2 + 2 + 4);
        assert_eq!(&bytes[..2], &[0x61, 0x00]);
        assert_eq!(utf16_string(&bytes), "aé😀");
        assert_eq!(utf16_string(&[0x61, 0x00, 0x62]), "a\u{fffd}");
        assert_eq!(utf16_string(&[]), "");
    }
}
