use std::ffi::c_void;

use crate::{Guid, RawHResult};

/// Implemented (by generated code) for every COM interface type.
///
/// # Safety
/// `Self` must be `#[repr(C)]` with a single field: a pointer to
/// `Self::Vtbl`, and `Self::Vtbl` must start with the vtable of the base
/// interface (ultimately [`IUnknownVtbl`]).
pub unsafe trait Interface: Sized + 'static {
    /// The interface's IID as written in the IDL.
    const IID: Guid;
    /// IDL name, for diagnostics.
    const NAME: &'static str;
    type Vtbl: 'static;

    /// `true` if `iid` names this interface or one of its bases (including
    /// `IUnknown`). Both the IDL value and the value materialized by `com.h`
    /// (see [`Guid::as_com_h_materialized`]) are accepted.
    fn matches_iid(iid: &Guid) -> bool;

    #[inline]
    fn as_unknown(&self) -> &IUnknown {
        // SAFETY: guaranteed by the trait contract.
        unsafe { &*(self as *const Self as *const IUnknown) }
    }

    #[inline]
    fn as_raw(&self) -> *mut Self {
        self as *const Self as *mut Self
    }
}

#[repr(C)]
pub struct IUnknownVtbl {
    pub query_interface: unsafe extern "C" fn(
        this: *mut c_void,
        riid: *const Guid,
        ppv: *mut *mut c_void,
    ) -> RawHResult,
    pub add_ref: unsafe extern "C" fn(this: *mut c_void) -> u32,
    pub release: unsafe extern "C" fn(this: *mut c_void) -> u32,
}

/// The root interface. A `*mut IUnknown` is a native `IUnknown*`.
#[repr(C)]
pub struct IUnknown {
    vtbl: *const IUnknownVtbl,
}

unsafe impl Interface for IUnknown {
    const IID: Guid = Guid::from_u128(0x00000000_0000_0000_C000_000000000046);
    const NAME: &'static str = "IUnknown";
    type Vtbl = IUnknownVtbl;

    fn matches_iid(iid: &Guid) -> bool {
        *iid == Self::IID || *iid == Self::IID.as_com_h_materialized()
    }
}

impl IUnknown {
    /// # Safety
    /// Raw COM call; `ppv` must be valid for writes.
    #[inline]
    pub unsafe fn query_interface_raw(&self, riid: &Guid, ppv: *mut *mut c_void) -> RawHResult {
        ((*self.vtbl).query_interface)(self as *const _ as *mut c_void, riid, ppv)
    }

    /// # Safety
    /// Raw reference-count manipulation.
    #[inline]
    pub unsafe fn add_ref(&self) -> u32 {
        ((*self.vtbl).add_ref)(self as *const _ as *mut c_void)
    }

    /// # Safety
    /// Raw reference-count manipulation; `self` may be dangling afterwards.
    #[inline]
    pub unsafe fn release(&self) -> u32 {
        ((*self.vtbl).release)(self as *const _ as *mut c_void)
    }
}

/// Implementation-side counterpart of [`IUnknown`]: the root of every
/// generated `I...Impl` trait hierarchy. It has no methods — reference
/// counting and `QueryInterface` are provided by [`crate::ComObject`] — and
/// is blanket-implemented for every type.
pub trait IUnknownImpl {}
impl<T: ?Sized> IUnknownImpl for T {}
