//! The function of DirectComposition the backend calls.

use ferroui_microcom::{ComPtr, Guid, HResult, IUnknown, Interface};
use std::ffi::c_void;

// On 32-bit x86 the export has no decoration, where the "system" calling
// convention would add one to the name.
#[cfg_attr(not(target_arch = "x86"), link(name = "dcomp", kind = "raw-dylib"))]
#[cfg_attr(target_arch = "x86", link(name = "dcomp", kind = "raw-dylib", import_name_type = "undecorated"))]
extern "system" {
    fn DCompositionCreateDevice2(
        rendering_device: *mut c_void,
        iid: *const Guid,
        dcomposition_device: *mut *mut c_void,
    ) -> i32;
}

/// The functions of DirectComposition.
pub struct NativeMethods;

impl NativeMethods {
    /// `DCompositionCreateDevice2`: a composition device as the interface,
    /// for a rendering device (a Direct3D or Direct2D device) or for none.
    pub fn d_composition_create_device2<T: Interface>(
        rendering_device: Option<&ComPtr<IUnknown>>,
    ) -> Result<ComPtr<T>, HResult> {
        let iid = T::IID;
        let rendering_device = rendering_device.map_or(std::ptr::null_mut(), |device| device.as_raw().cast());
        let mut device = std::ptr::null_mut();
        // SAFETY: the rendering device is null or an object that lives
        // through the call; the identifier is a value of this frame; the
        // device is written to a pointer of this frame.
        let result = unsafe { DCompositionCreateDevice2(rendering_device, &iid, &mut device) };
        if result < 0 {
            return Err(HResult(result as u32));
        }
        // SAFETY: the call succeeded, so the pointer is an object of the
        // interface asked for, whose reference this function owns.
        unsafe { ComPtr::<T>::from_raw(device.cast()) }.ok_or(HResult::POINTER)
    }
}
