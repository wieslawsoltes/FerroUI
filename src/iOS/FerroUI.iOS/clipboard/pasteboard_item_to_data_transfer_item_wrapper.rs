//! An item of the pasteboard as an item of a data transfer.

/// The text of bytes in UTF-16, the low byte first (`Encoding.Unicode`).
pub fn utf16_string(bytes: &[u8]) -> String {
    let units = bytes.chunks_exact(2).map(|pair| u16::from_le_bytes([pair[0], pair[1]]));
    let mut s: String = char::decode_utf16(units).map(|c| c.unwrap_or(char::REPLACEMENT_CHARACTER)).collect();
    if bytes.len() % 2 != 0 {
        s.push(char::REPLACEMENT_CHARACTER);
    }
    s
}

/// The bytes of a text in UTF-16, the low byte first.
pub fn utf16_bytes(s: &str) -> Vec<u8> {
    s.encode_utf16().flat_map(u16::to_le_bytes).collect()
}

#[cfg(target_os = "ios")]
pub(crate) use uikit::PasteboardItemToDataTransferItemWrapper;

#[cfg(target_os = "ios")]
mod uikit {
    use super::{utf16_bytes, utf16_string};
    use crate::clipboard::clipboard_data_format_helper::{
        is_text, is_text_uti, to_data_format, to_system_type, UT_TYPE_IMAGE, UT_TYPE_JPEG, UT_TYPE_PNG, UT_TYPE_TIFF,
    };
    use crate::storage::ios_storage_item::create_item;
    use ferroui_base::input::platform::{ClipboardError, PlatformDataTransferItem, PlatformDataTransferItemImpl};
    use ferroui_base::input::DataFormat;
    use ferroui_base::media::imaging::Bitmap;
    use ferroui_base::platform::storage::IStorageItem;
    use objc2::rc::Retained;
    use objc2::runtime::AnyObject;
    use objc2_foundation::{NSData, NSDictionary, NSString, NSURL};
    use objc2_ui_kit::UIImage;
    use std::any::Any;
    use std::rc::Rc;

    pub(crate) struct PasteboardItemToDataTransferItemWrapper {
        // key: the type, value: the value of the type
        item: Retained<NSDictionary<NSString, AnyObject>>,
    }

    impl PasteboardItemToDataTransferItemWrapper {
        pub(crate) fn new(item: Retained<NSDictionary<NSString, AnyObject>>) -> Rc<PlatformDataTransferItem> {
            PlatformDataTransferItem::new(PasteboardItemToDataTransferItemWrapper { item })
        }

        fn try_get_value(&self, type_: &str) -> Option<Retained<AnyObject>> {
            self.item.objectForKey(&NSString::from_str(type_))
        }

        fn bitmap_of(bytes: &NSData) -> Result<Option<Rc<dyn Any>>, ClipboardError> {
            match Bitmap::from_stream(&mut std::io::Cursor::new(bytes.to_vec())) {
                Ok(bitmap) => Ok(Some(Rc::new(Rc::new(bitmap)))),
                Err(error) => Err(ClipboardError::other(format!("Unable to load the bitmap: {error}"))),
            }
        }

        fn try_get_bitmap(&self) -> Result<Option<Rc<dyn Any>>, ClipboardError> {
            let mut image_value = self.try_get_value(UT_TYPE_PNG).or_else(|| self.try_get_value(UT_TYPE_JPEG));
            // A PNG or a JPEG is kept as it is: an image or data, and a
            // bitmap is made of either.
            if image_value.is_none() {
                image_value = self.try_get_value(UT_TYPE_IMAGE).or_else(|| self.try_get_value(UT_TYPE_TIFF));
                // Data is converted to an image first, as a bitmap is not
                // made of a TIFF directly.
                if let Some(image_data) = image_value.as_ref().and_then(|value| value.downcast_ref::<NSData>()) {
                    image_value = UIImage::imageWithData(image_data).map(|image| Retained::into_super(Retained::into_super(image)));
                }
            }

            let Some(image_value) = image_value else {
                return Ok(None);
            };
            if let Some(image) = image_value.downcast_ref::<UIImage>() {
                return match image.png_representation() {
                    Some(png_data) => Self::bitmap_of(&png_data),
                    None => Ok(None),
                };
            }
            match image_value.downcast_ref::<NSData>() {
                Some(data) => Self::bitmap_of(data),
                None => Ok(None),
            }
        }

        fn try_convert_to_string(value: &AnyObject) -> Option<String> {
            if let Some(str) = value.downcast_ref::<NSString>() {
                return Some(str.to_string());
            }
            value.downcast_ref::<NSData>().map(|data| utf16_string(&data.to_vec()))
        }

        fn try_convert_to_bytes(value: &AnyObject) -> Option<Rc<[u8]>> {
            if let Some(data) = value.downcast_ref::<NSData>() {
                return Some(Rc::from(data.to_vec()));
            }
            value.downcast_ref::<NSString>().map(|str| Rc::from(utf16_bytes(&str.to_string())))
        }
    }

    impl PlatformDataTransferItemImpl for PasteboardItemToDataTransferItemWrapper {
        fn provide_formats(&self) -> Result<Vec<DataFormat>, ClipboardError> {
            Ok(self.item.allKeys().iter().map(|type_| to_data_format(&type_.to_string(), &is_text_uti)).collect())
        }

        fn try_get_raw_core(&self, format: &DataFormat) -> Result<Option<Rc<dyn Any>>, ClipboardError> {
            // Images are handled without the type of the format, as an
            // item may have several types of images.
            if DataFormat::bitmap() == *format {
                return self.try_get_bitmap();
            }

            let type_ = to_system_type(format);
            let Some(value) = self.try_get_value(&type_) else {
                return Ok(None);
            };

            if DataFormat::text() == *format {
                return Ok(value.downcast_ref::<NSString>().map(|text| Rc::new(text.to_string()) as Rc<dyn Any>));
            }

            if DataFormat::file() == *format {
                let file_path_url = value.downcast_ref::<NSURL>().and_then(|url| url.filePathURL());
                return Ok(file_path_url.map(|url| {
                    let item: Rc<dyn IStorageItem> = create_item(url, None);
                    Rc::new(item) as Rc<dyn Any>
                }));
            }

            // The reference asks the format for the type of its values;
            // a format of the port does not carry one, so the type of
            // the pasteboard decides, as when the format was made of it.
            if is_text(&type_, &is_text_uti) {
                return Ok(Self::try_convert_to_string(&value).map(|text| Rc::new(text) as Rc<dyn Any>));
            }

            Ok(Self::try_convert_to_bytes(&value).map(|bytes| Rc::new(bytes) as Rc<dyn Any>))
        }
    }
}

#[cfg(test)]
mod tests {
    // Not from the reference, which has no tests of this file.
    use super::*;

    #[test]
    fn text_and_bytes_convert_through_utf16() {
        assert_eq!(vec![0x61, 0x00, 0x3D, 0xD8, 0x00, 0xDE], utf16_bytes("a\u{1F600}"));
        assert_eq!("a\u{1F600}", utf16_string(&utf16_bytes("a\u{1F600}")));
        // Half a code unit at the end, and half a surrogate pair.
        assert_eq!("a\u{FFFD}", utf16_string(&[0x61, 0x00, 0x62]));
        assert_eq!("\u{FFFD}", utf16_string(&[0x3D, 0xD8]));
        assert_eq!("", utf16_string(&[]));
    }
}
