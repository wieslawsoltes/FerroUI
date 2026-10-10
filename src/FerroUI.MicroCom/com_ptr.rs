use std::ffi::c_void;
use std::fmt;
use std::marker::PhantomData;
use std::ops::Deref;
use std::ptr::NonNull;

use crate::{HResult, Interface, E_NOINTERFACE, S_OK};

/// Diagnostics of the end of a process (the Windows backend, finding 1 of
/// `docs/porting/win32-platform.md`, section 11.2): when set, every release
/// of a pointer is written to the standard error stream before and after
/// the call, so that a release that does not return is the last line.
static RELEASE_TRACE: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// Turns the trace of releases on or off.
pub fn set_release_trace(enabled: bool) {
    RELEASE_TRACE.store(enabled, std::sync::atomic::Ordering::SeqCst);
}

/// Whether releases are traced.
pub fn release_trace() -> bool {
    RELEASE_TRACE.load(std::sync::atomic::Ordering::Relaxed)
}

/// Owning, non-null COM interface pointer: holds exactly one reference.
///
/// Not `Send`/`Sync`: the native objects are UI-thread affine.
#[repr(transparent)]
pub struct ComPtr<T: Interface> {
    ptr: NonNull<T>,
    _marker: PhantomData<*mut T>,
}

impl<T: Interface> ComPtr<T> {
    /// Takes ownership of one existing reference (no `AddRef`). Returns
    /// `None` for null.
    ///
    /// # Safety
    /// `raw` must be null or a valid pointer to a COM object implementing `T`
    /// whose reference the caller owns.
    #[inline]
    pub unsafe fn from_raw(raw: *mut T) -> Option<Self> {
        NonNull::new(raw).map(|ptr| ComPtr { ptr, _marker: PhantomData })
    }

    /// `AddRef`s a borrowed pointer and wraps it. Returns `None` for null.
    ///
    /// # Safety
    /// `raw` must be null or a valid pointer to a COM object implementing `T`.
    #[inline]
    pub unsafe fn from_raw_add_ref(raw: *mut T) -> Option<Self> {
        let p = Self::from_raw(raw)?;
        p.as_unknown().add_ref();
        Some(p)
    }

    /// `AddRef`s a borrowed interface reference.
    #[inline]
    pub fn from_ref(r: &T) -> Self {
        // SAFETY: `&T` always refers to a live COM object.
        unsafe { Self::from_raw_add_ref(r.as_raw()).unwrap_unchecked() }
    }

    /// Borrowed raw pointer; the reference count is unchanged.
    #[inline]
    pub fn as_ptr(&self) -> *mut T {
        self.ptr.as_ptr()
    }

    /// Gives up ownership without releasing.
    #[inline]
    pub fn into_raw(self) -> *mut T {
        let p = self.ptr.as_ptr();
        std::mem::forget(self);
        p
    }

    /// `QueryInterface` for `U`.
    ///
    /// The native library compares against the IIDs materialized by `com.h`
    /// (see [`crate::Guid::as_com_h_materialized`]), Rust-implemented objects
    /// accept both forms; so the materialized form is tried first and the IDL
    /// form second.
    pub fn cast<U: Interface>(&self) -> Result<ComPtr<U>, HResult> {
        let mut last = E_NOINTERFACE;
        for iid in [U::IID.as_com_h_materialized(), U::IID] {
            let mut out: *mut c_void = std::ptr::null_mut();
            // SAFETY: valid object, valid out pointer.
            let hr = unsafe { self.as_unknown().query_interface_raw(&iid, &mut out) };
            if hr == S_OK {
                // SAFETY: QueryInterface returned an owned reference to `U`.
                return unsafe { ComPtr::from_raw(out as *mut U) }.ok_or(HResult::POINTER);
            }
            last = hr;
        }
        Err(HResult(last))
    }
}

impl<T: Interface> Deref for ComPtr<T> {
    type Target = T;
    #[inline]
    fn deref(&self) -> &T {
        // SAFETY: we hold a reference, so the object is alive.
        unsafe { self.ptr.as_ref() }
    }
}

impl<T: Interface> Clone for ComPtr<T> {
    #[inline]
    fn clone(&self) -> Self {
        // SAFETY: the object is alive.
        unsafe { self.as_unknown().add_ref() };
        ComPtr { ptr: self.ptr, _marker: PhantomData }
    }
}

impl<T: Interface> Drop for ComPtr<T> {
    #[inline]
    fn drop(&mut self) {
        if release_trace() {
            eprintln!("teardown: release of {} {:p} begins", T::NAME, self.ptr.as_ptr());
            // SAFETY: we own one reference.
            let left = unsafe { self.as_unknown().release() };
            eprintln!("teardown: release of {} returned {left}", T::NAME);
            return;
        }
        // SAFETY: we own one reference.
        unsafe { self.as_unknown().release() };
    }
}

impl<T: Interface> PartialEq for ComPtr<T> {
    fn eq(&self, other: &Self) -> bool {
        self.ptr == other.ptr
    }
}

impl<T: Interface> fmt::Debug for ComPtr<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ComPtr<{}>({:p})", T::NAME, self.ptr.as_ptr())
    }
}
