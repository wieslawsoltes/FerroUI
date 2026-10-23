//! The type functions of the automation core of the system (the port of
//! `Interop/UiaCoreTypesApi.cs`).

#[cfg(windows)]
use ferroui_microcom::Guid;
use ferroui_microcom::{ComPtr, HResult, IUnknown};

/// The kind of identifier a lookup asks for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(i32)]
#[allow(dead_code)] // The kinds are the ones the reference declares.
pub(crate) enum AutomationIdType {
    Property,
    Pattern,
    Event,
    ControlType,
    TextAttribute,
}

/// The type functions and the result codes of the automation core.
pub(crate) struct UiaCoreTypesApi;

#[allow(dead_code)] // The codes are the ones the reference declares; the nodes answer two of them.
impl UiaCoreTypesApi {
    pub const UIA_E_ELEMENTNOTENABLED: i32 = 0x8004_0200u32 as i32;
    pub const UIA_E_ELEMENTNOTAVAILABLE: i32 = 0x8004_0201u32 as i32;
    pub const UIA_E_NOCLICKABLEPOINT: i32 = 0x8004_0202u32 as i32;
    pub const UIA_E_PROXYASSEMBLYNOTLOADED: i32 = 0x8004_0203u32 as i32;

    /// The native reserved value returned for attributes that vary inside a
    /// text range. Its IUnknown identity is defined by UIAutomationCore.
    pub fn reserved_mixed_attribute_value() -> Result<ComPtr<IUnknown>, HResult> {
        reserved::get(true)
    }

    /// The native reserved value returned for attributes a provider does not
    /// support; it is distinct from both a mixed value and an empty VARIANT.
    pub fn reserved_not_supported_value() -> Result<ComPtr<IUnknown>, HResult> {
        reserved::get(false)
    }

    /// `UiaLookupId`: the number of the identifier of `guid`; zero when
    /// the core does not know it.
    #[cfg(windows)]
    pub fn uia_lookup_id(type_: AutomationIdType, guid: &Guid) -> i32 {
        #[cfg_attr(not(target_arch = "x86"), link(name = "uiautomationcore", kind = "raw-dylib"))]
        #[cfg_attr(target_arch = "x86", link(name = "uiautomationcore", kind = "raw-dylib", import_name_type = "undecorated"))]
        unsafe extern "system" {
            fn UiaLookupId(type_: i32, guid: *const Guid) -> i32;
        }
        // SAFETY: a kind of identifier and an identifier that lives
        // through the call.
        unsafe { UiaLookupId(type_ as i32, guid) }
    }
}

#[cfg(windows)]
mod reserved {
    use super::{ComPtr, HResult, IUnknown};

    #[cfg_attr(not(target_arch = "x86"), link(name = "uiautomationcore", kind = "raw-dylib"))]
    #[cfg_attr(target_arch = "x86", link(name = "uiautomationcore", kind = "raw-dylib", import_name_type = "undecorated"))]
    unsafe extern "system" {
        fn UiaGetReservedMixedAttributeValue(value: *mut *mut IUnknown) -> i32;
        fn UiaGetReservedNotSupportedValue(value: *mut *mut IUnknown) -> i32;
    }

    pub(super) fn get(mixed: bool) -> Result<ComPtr<IUnknown>, HResult> {
        let mut result = std::ptr::null_mut();
        // SAFETY: the automation core writes an owned IUnknown reference into
        // this valid output slot; successful ownership passes to ComPtr.
        unsafe {
            let hr = if mixed {
                UiaGetReservedMixedAttributeValue(&mut result)
            } else {
                UiaGetReservedNotSupportedValue(&mut result)
            };
            HResult::check(hr as u32)?;
            ComPtr::from_raw(result).ok_or(HResult::FAIL)
        }
    }
}

/// The test backend on non-Windows hosts needs stable, distinct attribute
/// identities, just as its OleAut stand-in supplies real BSTR/SAFEARRAY layout.
/// These two immutable COM objects retain one process-lifetime reference and
/// contain no framework state. They are not compiled into the Windows backend.
#[cfg(not(windows))]
mod reserved {
    use super::{ComPtr, HResult, IUnknown};
    use std::sync::OnceLock;

    static MIXED: OnceLock<usize> = OnceLock::new();
    static NOT_SUPPORTED: OnceLock<usize> = OnceLock::new();

    pub(super) fn get(mixed: bool) -> Result<ComPtr<IUnknown>, HResult> {
        let slot = if mixed { &MIXED } else { &NOT_SUPPORTED };
        let identity = *slot.get_or_init(|| {
            let value: ComPtr<IUnknown> = ferroui_microcom::make_com(());
            value.into_raw() as usize
        });
        // SAFETY: the process-owned reference above keeps this immutable COM
        // object alive. Its reference count is atomic and holds no UI state.
        unsafe { ComPtr::from_raw_add_ref(identity as *mut IUnknown) }.ok_or(HResult::FAIL)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_kinds_of_identifier_and_the_result_codes_are_those_of_the_system() {
        assert_eq!(0, AutomationIdType::Property as i32);
        assert_eq!(4, AutomationIdType::TextAttribute as i32);
        assert_eq!(0x8004_0200, UiaCoreTypesApi::UIA_E_ELEMENTNOTENABLED as u32);
        assert_eq!(0x8004_0201, UiaCoreTypesApi::UIA_E_ELEMENTNOTAVAILABLE as u32);
        assert_eq!(0x8004_0202, UiaCoreTypesApi::UIA_E_NOCLICKABLEPOINT as u32);
        assert_eq!(0x8004_0203, UiaCoreTypesApi::UIA_E_PROXYASSEMBLYNOTLOADED as u32);
    }
}
