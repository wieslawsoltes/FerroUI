//! The provider functions of the automation core of the system,
//! `UIAutomationCore.dll` (the port of `Interop/UiaCoreProviderApi.cs`; the
//! enumerations the reference declares in that file are in `uia.idl` with
//! the others).
//!
//! The nodes are written against [`IUiaCoreProviderApi`]: on Windows the
//! functions of the system answer it, and a test puts a recorder in their
//! place for its thread, on every host. On a host that is not Windows
//! there is no automation core: a call without a recorder fails with a
//! message that says so.

use super::{IRawElementProviderSimple, StructureChangeType, TextEditChangeType};
use crate::automation::marshalling::{SafeArrayMarshaller, SafeArrayRef, VariantValue};
use ferroui_microcom::{ComPtr, HResult};
use std::cell::RefCell;
use std::rc::Rc;

/// The provider functions of the automation core.
#[allow(dead_code)] // The functions are the ones the reference declares; it does not call all of them either.
pub(crate) trait IUiaCoreProviderApi {
    /// Whether any client listens for events.
    fn uia_clients_are_listening(&self) -> bool;
    /// The result of a `WM_GETOBJECT` message for a provider; without a
    /// provider (and zero parameters) the window gives its providers up.
    fn uia_return_raw_element_provider(
        &self,
        hwnd: isize,
        w_param: usize,
        l_param: isize,
        el: Option<&IRawElementProviderSimple>,
    ) -> isize;
    /// The provider of the system for a window.
    fn uia_host_provider_from_hwnd(&self, hwnd: isize) -> Result<Option<ComPtr<IRawElementProviderSimple>>, HResult>;
    fn uia_raise_automation_event(&self, provider: Option<&IRawElementProviderSimple>, id: i32) -> i32;
    /// Borrows one-dimensional VT_BSTR data until this synchronous call returns.
    /// An implementation may retain only its own copy of the strings.
    fn uia_raise_text_edit_text_changed_event(
        &self,
        provider: Option<&IRawElementProviderSimple>,
        change_type: TextEditChangeType,
        changed_data: SafeArrayRef,
    ) -> i32;
    fn uia_raise_automation_property_changed_event(
        &self,
        provider: Option<&IRawElementProviderSimple>,
        id: i32,
        old_value: Option<&VariantValue>,
        new_value: Option<&VariantValue>,
    ) -> i32;
    fn uia_raise_structure_changed_event(
        &self,
        provider: Option<&IRawElementProviderSimple>,
        structure_change_type: StructureChangeType,
        runtime_id: Option<&[i32]>,
    ) -> i32;
    fn uia_disconnect_provider(&self, provider: Option<&IRawElementProviderSimple>) -> i32;
}

ferroui_base::ferro_thread_local! {
    /// What stands in for the automation core on this thread (a test).
    static OVERRIDE: RefCell<Option<Rc<dyn IUiaCoreProviderApi>>> = const { RefCell::new(None) };
}

/// The functions of the automation core, as the reference names them.
pub(crate) struct UiaCoreProviderApi;

#[allow(dead_code)] // As the trait.
impl UiaCoreProviderApi {
    pub const UIA_E_ELEMENTNOTENABLED: i32 = 0x8004_0200u32 as i32;

    /// Puts `api` in the place of the automation core for the calling
    /// thread, or takes it away with `None` (for tests).
    #[allow(dead_code)] // The tests call it.
    pub fn set_for_thread(api: Option<Rc<dyn IUiaCoreProviderApi>>) {
        OVERRIDE.with(|slot| *slot.borrow_mut() = api);
    }

    fn with<R>(call: impl FnOnce(&dyn IUiaCoreProviderApi) -> R) -> R {
        let api = OVERRIDE.with(|slot| slot.borrow().clone());
        match api {
            Some(api) => call(&*api),
            None => call(&system::System),
        }
    }

    pub fn uia_clients_are_listening() -> bool {
        Self::with(|api| api.uia_clients_are_listening())
    }

    pub fn uia_return_raw_element_provider(
        hwnd: isize,
        w_param: usize,
        l_param: isize,
        el: Option<&IRawElementProviderSimple>,
    ) -> isize {
        Self::with(|api| api.uia_return_raw_element_provider(hwnd, w_param, l_param, el))
    }

    pub fn uia_host_provider_from_hwnd(hwnd: isize) -> Result<Option<ComPtr<IRawElementProviderSimple>>, HResult> {
        Self::with(|api| api.uia_host_provider_from_hwnd(hwnd))
    }

    pub fn uia_raise_automation_event(provider: Option<&IRawElementProviderSimple>, id: i32) -> i32 {
        Self::with(|api| api.uia_raise_automation_event(provider, id))
    }

    pub fn uia_raise_text_edit_text_changed_event(
        provider: Option<&IRawElementProviderSimple>,
        change_type: TextEditChangeType,
        changed_data: &[String],
    ) -> i32 {
        // Match WinUI's CUIAWindow::UIARaiseTextEditTextChangedEvent: None
        // is a no-op, and its known nonempty causes are validated before
        // forwarding a SAFEARRAY(BSTR) to the system.
        match change_type {
            TextEditChangeType::None => return 0,
            TextEditChangeType::AutoCorrect
            | TextEditChangeType::Composition
            | TextEditChangeType::CompositionFinalized => {}
            _ => return ferroui_microcom::E_INVALIDARG as i32,
        }
        struct ChangedData(SafeArrayRef);
        impl Drop for ChangedData {
            fn drop(&mut self) {
                // SAFETY: this is the sole owning guard for the array. The
                // synchronous native call borrows it and copies the payload.
                unsafe { SafeArrayMarshaller::<String>::free(self.0) };
            }
        }
        let data = match SafeArrayMarshaller::<String>::convert_to_unmanaged(Some(changed_data)) {
            Ok(array) => ChangedData(array),
            Err(error) => return error.0,
        };
        Self::with(|api| api.uia_raise_text_edit_text_changed_event(provider, change_type, data.0))
    }


    pub fn uia_raise_automation_property_changed_event(
        provider: Option<&IRawElementProviderSimple>,
        id: i32,
        old_value: Option<&VariantValue>,
        new_value: Option<&VariantValue>,
    ) -> i32 {
        Self::with(|api| api.uia_raise_automation_property_changed_event(provider, id, old_value, new_value))
    }

    pub fn uia_raise_structure_changed_event(
        provider: Option<&IRawElementProviderSimple>,
        structure_change_type: StructureChangeType,
        runtime_id: Option<&[i32]>,
    ) -> i32 {
        Self::with(|api| api.uia_raise_structure_changed_event(provider, structure_change_type, runtime_id))
    }

    pub fn uia_disconnect_provider(provider: Option<&IRawElementProviderSimple>) -> i32 {
        Self::with(|api| api.uia_disconnect_provider(provider))
    }
}

#[cfg(windows)]
mod system {
    use super::{ComPtr, HResult, IRawElementProviderSimple, IUiaCoreProviderApi, SafeArrayRef, StructureChangeType, TextEditChangeType, VariantValue};
    use crate::automation::marshalling::{ComVariant, ComVariantMarshaller, SAFEARRAY};
    use std::ffi::c_void;

    #[cfg_attr(not(target_arch = "x86"), link(name = "uiautomationcore", kind = "raw-dylib"))]
    #[cfg_attr(target_arch = "x86", link(name = "uiautomationcore", kind = "raw-dylib", import_name_type = "undecorated"))]
    #[allow(dead_code)] // As the trait.
    unsafe extern "system" {
        fn UiaClientsAreListening() -> i32;
        fn UiaReturnRawElementProvider(hwnd: isize, w_param: usize, l_param: isize, el: *mut c_void) -> isize;
        fn UiaHostProviderFromHwnd(hwnd: isize, provider: *mut *mut IRawElementProviderSimple) -> i32;
        fn UiaRaiseAutomationEvent(provider: *mut c_void, id: i32) -> i32;
        fn UiaRaiseTextEditTextChangedEvent(
            provider: *mut c_void,
            change_type: i32,
            changed_data: *mut SAFEARRAY,
        ) -> i32;
        fn UiaRaiseAutomationPropertyChangedEvent(
            provider: *mut c_void,
            id: i32,
            old_value: ComVariant,
            new_value: ComVariant,
        ) -> i32;
        fn UiaRaiseStructureChangedEvent(
            provider: *mut c_void,
            structure_change_type: i32,
            runtime_id: *mut i32,
            runtime_id_len: i32,
        ) -> i32;
        fn UiaDisconnectProvider(provider: *mut c_void) -> i32;
    }

    fn raw(provider: Option<&IRawElementProviderSimple>) -> *mut c_void {
        provider.map_or(std::ptr::null_mut(), |provider| std::ptr::from_ref(provider).cast_mut().cast::<c_void>())
    }

    /// The automation core of the system.
    pub(super) struct System;

    impl IUiaCoreProviderApi for System {
        fn uia_clients_are_listening(&self) -> bool {
            // SAFETY: no arguments.
            unsafe { UiaClientsAreListening() != 0 }
        }

        fn uia_return_raw_element_provider(
            &self,
            hwnd: isize,
            w_param: usize,
            l_param: isize,
            el: Option<&IRawElementProviderSimple>,
        ) -> isize {
            // SAFETY: a window handle, the parameters of its message and a
            // live provider or null; the core takes its own reference.
            unsafe { UiaReturnRawElementProvider(hwnd, w_param, l_param, raw(el)) }
        }

        fn uia_host_provider_from_hwnd(&self, hwnd: isize) -> Result<Option<ComPtr<IRawElementProviderSimple>>, HResult> {
            let mut provider = std::ptr::null_mut();
            // SAFETY: a window handle and a place for the result, whose
            // reference becomes the pointer's.
            unsafe {
                HResult::check(UiaHostProviderFromHwnd(hwnd, &mut provider) as u32)?;
                Ok(ComPtr::from_raw(provider))
            }
        }

        fn uia_raise_automation_event(&self, provider: Option<&IRawElementProviderSimple>, id: i32) -> i32 {
            // SAFETY: a live provider or null.
            unsafe { UiaRaiseAutomationEvent(raw(provider), id) }
        }

        fn uia_raise_text_edit_text_changed_event(
            &self,
            provider: Option<&IRawElementProviderSimple>,
            change_type: TextEditChangeType,
            changed_data: SafeArrayRef,
        ) -> i32 {
            // SAFETY: the provider is live through this synchronous call.
            // UiaCoreProviderApi validates the cause and owns the one-dimensional
            // VT_BSTR array until the system has copied its changed-data strings.
            unsafe { UiaRaiseTextEditTextChangedEvent(raw(provider), change_type.0, changed_data.as_ptr()) }
        }

        fn uia_raise_automation_property_changed_event(
            &self,
            provider: Option<&IRawElementProviderSimple>,
            id: i32,
            old_value: Option<&VariantValue>,
            new_value: Option<&VariantValue>,
        ) -> i32 {
            // A value the variant does not carry is the null variant: the
            // values of this call are the ones the node chose to pass.
            let old_value = ComVariantMarshaller::convert_to_unmanaged(old_value).unwrap_or_else(|_| ComVariant::null());
            let new_value = ComVariantMarshaller::convert_to_unmanaged(new_value).unwrap_or_else(|_| ComVariant::null());
            // SAFETY: a live provider or null and two variants, which the
            // core copies; they are freed below, as the marshaller of the
            // reference frees them after the call.
            unsafe {
                let result = UiaRaiseAutomationPropertyChangedEvent(raw(provider), id, old_value, new_value);
                ComVariantMarshaller::free(old_value);
                ComVariantMarshaller::free(new_value);
                result
            }
        }

        fn uia_raise_structure_changed_event(
            &self,
            provider: Option<&IRawElementProviderSimple>,
            structure_change_type: StructureChangeType,
            runtime_id: Option<&[i32]>,
        ) -> i32 {
            let mut runtime_id: Option<Vec<i32>> = runtime_id.map(<[i32]>::to_vec);
            let (pointer, length) = match &mut runtime_id {
                Some(runtime_id) => (runtime_id.as_mut_ptr(), runtime_id.len() as i32),
                None => (std::ptr::null_mut(), 0),
            };
            // SAFETY: a live provider or null, and the identifier with its
            // length, which lives through the call.
            unsafe { UiaRaiseStructureChangedEvent(raw(provider), structure_change_type.0, pointer, length) }
        }

        fn uia_disconnect_provider(&self, provider: Option<&IRawElementProviderSimple>) -> i32 {
            // SAFETY: a live provider or null.
            unsafe { UiaDisconnectProvider(raw(provider)) }
        }
    }
}

#[cfg(not(windows))]
mod system {
    use super::{ComPtr, HResult, IRawElementProviderSimple, IUiaCoreProviderApi, SafeArrayRef, StructureChangeType, TextEditChangeType, VariantValue};

    /// No automation core: this host is not Windows.
    pub(super) struct System;

    fn not_windows() -> ! {
        panic!("the automation core (UIAutomationCore.dll) is a library of Windows: this host has none")
    }

    impl IUiaCoreProviderApi for System {
        fn uia_clients_are_listening(&self) -> bool {
            not_windows()
        }

        fn uia_return_raw_element_provider(&self, _: isize, _: usize, _: isize, _: Option<&IRawElementProviderSimple>) -> isize {
            not_windows()
        }

        fn uia_host_provider_from_hwnd(&self, _: isize) -> Result<Option<ComPtr<IRawElementProviderSimple>>, HResult> {
            not_windows()
        }

        fn uia_raise_automation_event(&self, _: Option<&IRawElementProviderSimple>, _: i32) -> i32 {
            not_windows()
        }

        fn uia_raise_text_edit_text_changed_event(
            &self,
            _: Option<&IRawElementProviderSimple>,
            _: TextEditChangeType,
            _: SafeArrayRef,
        ) -> i32 {
            not_windows()
        }

        fn uia_raise_automation_property_changed_event(
            &self,
            _: Option<&IRawElementProviderSimple>,
            _: i32,
            _: Option<&VariantValue>,
            _: Option<&VariantValue>,
        ) -> i32 {
            not_windows()
        }

        fn uia_raise_structure_changed_event(
            &self,
            _: Option<&IRawElementProviderSimple>,
            _: StructureChangeType,
            _: Option<&[i32]>,
        ) -> i32 {
            not_windows()
        }

        fn uia_disconnect_provider(&self, _: Option<&IRawElementProviderSimple>) -> i32 {
            not_windows()
        }
    }
}
