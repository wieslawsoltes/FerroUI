use crate::clipboard_data_format_helper::to_data_formats;
use crate::clipboard_data_transfer_item::ClipboardDataTransferItem;
use crate::clipboard_read_session::ClipboardReadSession;
use ferroui_base::input::platform::chromium_clipboard::ChromiumClipboard;
use ferroui_base::input::platform::{
    ClipboardError, PlatformDataTransfer, PlatformDataTransferImpl, PlatformDataTransferItem,
};
use ferroui_base::input::DataFormat;
use std::cell::OnceCell;
use std::rc::Rc;

/// The data of a native clipboard (or dragging pasteboard) read through a
/// clipboard session, which the data transfer owns.
pub(crate) struct ClipboardDataTransfer {
    session: Rc<ClipboardReadSession>,
    read_web_formats: bool,
    items: OnceCell<Vec<Rc<PlatformDataTransferItem>>>,
}

impl ClipboardDataTransfer {
    pub(crate) fn new(session: ClipboardReadSession) -> Rc<PlatformDataTransfer> {
        Self::wrap(session, false)
    }

    pub(crate) fn for_clipboard(session: ClipboardReadSession) -> Rc<PlatformDataTransfer> {
        Self::wrap(session, true)
    }

    fn wrap(session: ClipboardReadSession, read_web_formats: bool) -> Rc<PlatformDataTransfer> {
        PlatformDataTransfer::new(ClipboardDataTransfer {
            session: Rc::new(session),
            read_web_formats,
            items: OnceCell::new(),
        })
    }
}

impl PlatformDataTransferImpl for ClipboardDataTransfer {
    fn provide_formats(&self) -> Result<Vec<DataFormat>, ClipboardError> {
        let formats = self.session.get_formats()?;
        let mut formats = to_data_formats(formats.as_deref(), &|format| {
            self.session.is_text_format(format)
        });
        if self.read_web_formats && ChromiumClipboard::MacOs.find_map(&formats).is_some() {
            for item in self.provide_items()? {
                for format in item.try_formats()?.iter() {
                    if !formats.contains(format) {
                        formats.push(format.clone());
                    }
                }
            }
        }
        Ok(formats)
    }

    fn provide_items(&self) -> Result<Vec<Rc<PlatformDataTransferItem>>, ClipboardError> {
        if let Some(items) = self.items.get() {
            return Ok(items.clone());
        }
        let item_count = self.session.get_item_count()?;
        let items = self.items.get_or_init(|| {
            (0..item_count)
                .map(|index| {
                    ClipboardDataTransferItem::new(
                        self.session.clone(),
                        index,
                        self.read_web_formats,
                    )
                })
                .collect()
        });
        Ok(items.clone())
    }

    fn dispose(&self, _owner: &PlatformDataTransfer) {
        self.session.dispose();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frn_string::{FrnString, FrnStringArray};
    use crate::interop::*;
    use ferroui_base::input::DataTransferExtensions;
    use ferroui_microcom::{ComPtr, HResult};
    use std::cell::Cell;
    use std::ffi::CStr;

    const FIXTURE: [&[(&str, &str)]; 2] = [
        &[
            ("public.utf8-plain-text", "ab\ncd"),
            ("net.ferroui.app.uti.Direct", "native text"),
            (
                "org.w3.web-custom-format.map",
                r#"{
                "application/frn-fmt.text.426f78":"org.w3.web-custom-format.type-0",
                "application/frn-fmt.text.4d697373":"org.w3.web-custom-format.type-1",
                "application/frn-fmt.bytes.456d707479":"org.w3.web-custom-format.type-2"
            }"#,
            ),
            ("org.w3.web-custom-format.type-0", "1"),
            ("org.w3.web-custom-format.type-2", ""),
        ],
        &[("org.w3.web-custom-format.type-1", "foreign item")],
    ];

    struct PasteboardFixture(Cell<i64>);

    impl PasteboardFixture {
        fn check_count(&self, count: i64) -> Result<(), HResult> {
            if self.0.get() == count {
                Ok(())
            } else {
                Err(HResult(0x8013_1622))
            }
        }
    }

    impl IFrnClipboardImpl for PasteboardFixture {
        fn get_formats(&self, count: i64) -> Result<Option<ComPtr<IFrnStringArray>>, HResult> {
            self.check_count(count)?;
            Ok(Some(IFrnStringArray::from_impl(FrnStringArray::new(
                FIXTURE
                    .iter()
                    .flat_map(|item| item.iter().map(|(name, _)| *name)),
            ))))
        }

        fn get_item_count(&self, count: i64) -> Result<i32, HResult> {
            self.check_count(count)?;
            Ok(FIXTURE.len() as i32)
        }

        fn get_item_formats(
            &self,
            index: i32,
            count: i64,
        ) -> Result<Option<ComPtr<IFrnStringArray>>, HResult> {
            self.check_count(count)?;
            let item = FIXTURE.get(index as usize).ok_or(HResult::INVALIDARG)?;
            Ok(Some(IFrnStringArray::from_impl(FrnStringArray::new(
                item.iter().map(|(name, _)| *name),
            ))))
        }

        fn get_item_value_as_string(
            &self,
            index: i32,
            count: i64,
            format: Option<&CStr>,
        ) -> Result<Option<ComPtr<IFrnString>>, HResult> {
            self.get_item_value_as_bytes(index, count, format)
        }

        fn get_item_value_as_bytes(
            &self,
            index: i32,
            count: i64,
            format: Option<&CStr>,
        ) -> Result<Option<ComPtr<IFrnString>>, HResult> {
            self.check_count(count)?;
            let item = FIXTURE.get(index as usize).ok_or(HResult::INVALIDARG)?;
            let format = format
                .and_then(|name| name.to_str().ok())
                .unwrap_or_default();
            Ok(item
                .iter()
                .find(|(name, _)| *name == format)
                .map(|(_, value)| IFrnString::from_impl(FrnString::new(value))))
        }

        fn clear(&self) -> Result<i64, HResult> {
            Err(HResult::FAIL)
        }
        fn get_change_count(&self) -> Result<i64, HResult> {
            Ok(self.0.get())
        }
        fn set_data(&self, _: Option<&IFrnClipboardDataSource>) -> Result<(), HResult> {
            Err(HResult::FAIL)
        }
        fn is_text_format(&self, format: Option<&CStr>) -> bool {
            format == Some(c"public.utf8-plain-text")
        }
    }

    #[test]
    fn clipboard_reads_each_items_map_and_rejects_a_changed_pasteboard() {
        let fixture = Rc::new(PasteboardFixture(Cell::new(7)));
        let native = IFrnClipboard::from_impl(fixture.clone());
        let clipboard =
            ClipboardDataTransfer::for_clipboard(ClipboardReadSession::new(native.clone(), 7));
        let drag = ClipboardDataTransfer::new(ClipboardReadSession::new(native, 7));
        let marker = DataFormat::create_string_application_format("Box");
        let miss = DataFormat::create_string_application_format("Miss");
        let direct = DataFormat::create_string_application_format("Direct");
        let empty = DataFormat::create_bytes_application_format("Empty");
        assert!(clipboard.contains(&marker));
        assert!(!clipboard.contains(&miss));
        assert!(!drag.contains(&marker));
        assert_eq!(clipboard.try_get_text().as_deref(), Some("ab\ncd"));
        assert_eq!(clipboard.try_get_value(&marker).as_deref(), Some("1"));
        assert_eq!(
            clipboard.try_get_value(&direct).as_deref(),
            Some("native text")
        );
        assert!(clipboard.try_get_value(&empty).unwrap().is_empty());
        fixture.0.set(8);
        assert_eq!(clipboard.try_get_value(&marker), None);
    }
}
