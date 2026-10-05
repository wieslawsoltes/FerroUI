use std::ffi::c_void;
use std::marker::PhantomData;
use std::sync::atomic::{fence, AtomicU32, Ordering};

use crate::{ComPtr, Guid, IUnknownVtbl, Interface, RawHResult, E_NOINTERFACE, E_POINTER, S_OK};

/// Links an interface `Self` to a Rust type `T` that implements its
/// callbacks: supplies the vtable native code will call through.
///
/// Generated code provides
/// `unsafe impl<T: IFooImpl> ImplementedBy<T> for IFoo`.
///
/// # Safety
/// `VTBL` must be a vtable for `Self` whose thunks expect `this` to point at
/// a [`ComObject<T>`].
pub unsafe trait ImplementedBy<T>: Interface {
    const VTBL: &'static Self::Vtbl;
}

/// Heap layout of a Rust-implemented COM object (a "COM callable wrapper";
/// the equivalent of MicroCom's shadow objects / callback base class).
///
/// The first word is the vtable pointer, so a `*mut ComObject<T>` is what
/// native code sees as the interface pointer.
#[repr(C)]
pub struct ComObject<T> {
    vtbl: *const c_void,
    refs: AtomicU32,
    value: T,
}

impl<T> ComObject<T> {
    /// Recovers the Rust value from the `this` pointer a thunk receives.
    ///
    /// # Safety
    /// `this` must point at a live `ComObject<T>`.
    #[inline]
    pub unsafe fn value<'a>(this: *mut c_void) -> &'a T {
        &(*(this as *const ComObject<T>)).value
    }

    /// Current reference count (diagnostics/tests only).
    ///
    /// # Safety
    /// `this` must point at a live `ComObject<T>`.
    pub unsafe fn ref_count(this: *mut c_void) -> u32 {
        (*(this as *const ComObject<T>)).refs.load(Ordering::Relaxed)
    }
}

/// Moves `value` to the heap behind a COM vtable for interface `I` and
/// returns the (single) owning reference.
///
/// The object is destroyed — and `value` dropped — when the last reference is
/// released. Reference counting is atomic because native code may release
/// from another thread (in which case `value` is dropped on that thread);
/// the interface methods themselves are only ever invoked on the UI thread.
pub fn make_com<I, T>(value: T) -> ComPtr<I>
where
    I: ImplementedBy<T>,
    T: 'static,
{
    let obj = Box::new(ComObject {
        vtbl: I::VTBL as *const I::Vtbl as *const c_void,
        refs: AtomicU32::new(1),
        value,
    });
    // SAFETY: freshly created object with one reference; layout contract of
    // `Interface`/`ImplementedBy` makes it a valid `I`.
    unsafe { ComPtr::from_raw(Box::into_raw(obj) as *mut I).unwrap_unchecked() }
}

/// The three `IUnknown` slots for a [`ComObject<T>`] exposing interface `I`.
pub struct UnknownThunks<I, T>(PhantomData<(I, T)>);

impl<I: Interface, T> UnknownThunks<I, T> {
    pub const VTBL: IUnknownVtbl = IUnknownVtbl {
        query_interface: Self::query_interface,
        add_ref: Self::add_ref,
        release: Self::release,
    };

    unsafe extern "C" fn query_interface(
        this: *mut c_void,
        riid: *const Guid,
        ppv: *mut *mut c_void,
    ) -> RawHResult {
        if ppv.is_null() || riid.is_null() {
            return E_POINTER;
        }
        if I::matches_iid(&*riid) {
            Self::add_ref(this);
            *ppv = this;
            S_OK
        } else {
            *ppv = std::ptr::null_mut();
            E_NOINTERFACE
        }
    }

    unsafe extern "C" fn add_ref(this: *mut c_void) -> u32 {
        let obj = &*(this as *const ComObject<T>);
        obj.refs.fetch_add(1, Ordering::Relaxed) + 1
    }

    unsafe extern "C" fn release(this: *mut c_void) -> u32 {
        let obj = this as *mut ComObject<T>;
        let left = (*obj).refs.fetch_sub(1, Ordering::Release) - 1;
        if left == 0 {
            fence(Ordering::Acquire);
            drop(Box::from_raw(obj));
        }
        left
    }
}

impl IUnknownVtbl {
    /// Vtable prefix for a [`ComObject<T>`] whose most-derived interface is `I`.
    pub const fn new<I: Interface, T>() -> Self {
        UnknownThunks::<I, T>::VTBL
    }
}

unsafe impl<T> ImplementedBy<T> for crate::IUnknown {
    const VTBL: &'static IUnknownVtbl = &IUnknownVtbl::new::<crate::IUnknown, T>();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{IUnknown, E_NOINTERFACE};
    use std::rc::Rc;
    use std::cell::Cell;

    struct DropFlag(Rc<Cell<bool>>);
    impl Drop for DropFlag {
        fn drop(&mut self) {
            self.0.set(true);
        }
    }

    #[test]
    fn refcount_and_query_interface() {
        let dropped = Rc::new(Cell::new(false));
        let p: ComPtr<IUnknown> = make_com(DropFlag(dropped.clone()));
        let raw = p.as_ptr() as *mut c_void;
        unsafe {
            assert_eq!(ComObject::<DropFlag>::ref_count(raw), 1);
            let q = p.clone();
            assert_eq!(ComObject::<DropFlag>::ref_count(raw), 2);
            let r = q.cast::<IUnknown>().unwrap();
            assert_eq!(ComObject::<DropFlag>::ref_count(raw), 3);
            drop(r);
            drop(q);
            assert_eq!(ComObject::<DropFlag>::ref_count(raw), 1);

            let mut out = std::ptr::null_mut();
            let bogus = Guid::from_u128(0x1234);
            assert_eq!(p.query_interface_raw(&bogus, &mut out), E_NOINTERFACE);
            assert!(out.is_null());
        }
        assert!(!dropped.get());
        drop(p);
        assert!(dropped.get());
    }

    #[test]
    fn guid_layout_and_parse() {
        assert_eq!(std::mem::size_of::<Guid>(), 16);
        let g = Guid::parse("809c652e-7396-11d2-9771-00a0c9b4d50c").unwrap();
        assert_eq!(g, Guid::from_u128(0x809c652e_7396_11d2_9771_00a0c9b4d50c));
        assert_eq!(g.to_string(), "809c652e-7396-11d2-9771-00a0c9b4d50c");
        assert_eq!(g.as_com_h_materialized().data4, [0x97, 0x71, 0x71, 0x71, 0x71, 0x71, 0x71, 0x71]);
    }
}
