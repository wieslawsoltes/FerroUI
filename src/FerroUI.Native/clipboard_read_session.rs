use crate::frn_string::to_c_string;
use crate::helpers::to_clipboard_error;
use crate::interop::*;
use ferroui_base::input::platform::ClipboardError;
use ferroui_microcom::{ComPtr, HResult};
use std::cell::RefCell;

const COR_E_OBJECTDISPOSED: u32 = 0x8013_1622;

/// Reads a native clipboard (or dragging pasteboard) as it was at one
/// change count: once the clipboard changes, the session reads nothing.
pub(crate) struct ClipboardReadSession {
    native: RefCell<Option<ComPtr<IFrnClipboard>>>,
    change_count: i64,
}

impl ClipboardReadSession {
    /// The reference constructor also says whether the session owns the
    /// native object; here every session holds its own counted reference,
    /// which is released when the session is disposed.
    pub(crate) fn new(native: ComPtr<IFrnClipboard>, change_count: i64) -> Self {
        Self { native: RefCell::new(Some(native)), change_count }
    }

    #[track_caller]
    fn native(&self) -> ComPtr<IFrnClipboard> {
        match self.native.borrow().clone() {
            Some(native) => native,
            None => panic!("Cannot access a disposed object: ClipboardReadSession"),
        }
    }

    /// `None` when the clipboard has changed; any other failure is the
    /// error of the native call, which the reference surfaces as a COM
    /// exception from whatever read the session.
    fn unless_changed<T>(result: Result<Option<T>, HResult>) -> Result<Option<T>, ClipboardError> {
        match result {
            Ok(value) => Ok(value),
            Err(error) if Self::is_com_object_disposed_exception(error) => Ok(None),
            Err(error) => Err(to_clipboard_error(error)),
        }
    }

    pub(crate) fn get_formats(&self) -> Result<Option<ComPtr<IFrnStringArray>>, ClipboardError> {
        Self::unless_changed(self.native().get_formats(self.change_count))
    }

    pub(crate) fn get_item_count(&self) -> Result<i32, ClipboardError> {
        Ok(Self::unless_changed(self.native().get_item_count(self.change_count).map(Some))?.unwrap_or(0))
    }

    pub(crate) fn get_item_formats(&self, index: i32) -> Result<Option<ComPtr<IFrnStringArray>>, ClipboardError> {
        Self::unless_changed(self.native().get_item_formats(index, self.change_count))
    }

    pub(crate) fn get_item_value_as_string(
        &self,
        index: i32,
        format: &str,
    ) -> Result<Option<ComPtr<IFrnString>>, ClipboardError> {
        Self::unless_changed(self.native().get_item_value_as_string(index, self.change_count, Some(&to_c_string(format))))
    }

    pub(crate) fn get_item_value_as_bytes(
        &self,
        index: i32,
        format: &str,
    ) -> Result<Option<ComPtr<IFrnString>>, ClipboardError> {
        Self::unless_changed(self.native().get_item_value_as_bytes(index, self.change_count, Some(&to_c_string(format))))
    }

    pub(crate) fn is_text_format(&self, format: &str) -> bool {
        self.native().is_text_format(Some(&to_c_string(format)))
    }

    // The native side returns COR_E_OBJECTDISPOSED if the clipboard has changed (the change count doesn't match).
    pub(crate) fn is_com_object_disposed_exception(error: HResult) -> bool {
        error.0 == COR_E_OBJECTDISPOSED
    }

    pub(crate) fn dispose(&self) {
        let native = self.native.borrow_mut().take();
        drop(native);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_the_changed_clipboard_error_is_swallowed() {
        assert!(ClipboardReadSession::is_com_object_disposed_exception(HResult(0x8013_1622)));
        assert!(!ClipboardReadSession::is_com_object_disposed_exception(HResult::FAIL));
        assert_eq!(ClipboardReadSession::unless_changed::<i32>(Err(HResult(0x8013_1622))), Ok(None));
        assert_eq!(ClipboardReadSession::unless_changed(Ok(Some(3))), Ok(Some(3)));
        assert_eq!(ClipboardReadSession::unless_changed::<i32>(Ok(None)), Ok(None));
    }

    #[test]
    fn other_errors_are_reported() {
        use ferroui_base::input::platform::ClipboardErrorKind;

        let error = ClipboardReadSession::unless_changed::<i32>(Err(HResult::FAIL)).expect_err("the failed call");
        assert_eq!(ClipboardErrorKind::Platform, error.kind());
        assert_eq!(Some(HResult::FAIL.0 as i32), error.code());
        assert!(error.message().starts_with("Native call failed"));
    }
}
