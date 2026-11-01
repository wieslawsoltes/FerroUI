//! The value pattern of a node (the port of `AutomationNode.Value.cs`).

use super::automation_node::{AutomationNode, ELEMENT_NOT_ENABLED};
use super::interop::IValueProviderImpl;
use super::marshalling::string_to_bstr;
use ferroui_controls::automation::provider::IValueProvider;
use ferroui_controls::automation::ElementNotEnabledError;
use ferroui_microcom::{HResult, COR_E_INVALIDOPERATION};
use std::rc::Rc;

impl IValueProviderImpl for AutomationNode {
    /// `value` is null or a string that ends with a null: the contract of
    /// the method of the interface, which the callers of a COM object
    /// hold.
    fn set_value(&self, value: *const u16) -> Result<(), HResult> {
        let value: Option<String> = if value.is_null() {
            None
        } else {
            // SAFETY: the contract of the method: a null-terminated string
            // that lives through the call.
            unsafe {
                let mut length = 0usize;
                while *value.add(length) != 0 {
                    length += 1;
                }
                Some(String::from_utf16_lossy(std::slice::from_raw_parts(value, length)))
            }
        };
        self.invoke_sync_provider(move |x: &Rc<dyn IValueProvider>| {
            // An element that is not enabled has the result code of the
            // system; any other failure of the provider (a value that is
            // read only) is an operation that is not valid.
            x.set_value(value.as_deref()).map_err(|error| {
                if error.downcast_ref::<ElementNotEnabledError>().is_some() {
                    ELEMENT_NOT_ENABLED
                } else {
                    HResult(COR_E_INVALIDOPERATION)
                }
            })
        })
    }

    /// The value as a string of the system, which the caller frees; null
    /// for no value.
    fn get_value(&self) -> Result<*mut u16, HResult> {
        let value = self.invoke_sync_provider(|x: &Rc<dyn IValueProvider>| Ok(x.value()))?;
        Ok(value.map_or(std::ptr::null_mut(), |value| string_to_bstr(&value)))
    }

    fn get_is_read_only(&self) -> Result<i32, HResult> {
        self.invoke_sync_provider(|x: &Rc<dyn IValueProvider>| Ok(i32::from(x.is_read_only())))
    }
}
