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

/// One interface of a Rust-implemented COM object as native code sees it (a
/// "COM callable wrapper" of the MicroCom runtime): the first word is the
/// vtable pointer, so a `*mut ComObject<T>` is the interface pointer; the
/// second leads to the object the wrapper belongs to.
///
/// An object has one wrapper per interface it was made with, as the shadow
/// of an object has in the MicroCom runtime. They share the reference count
/// and the value, and `QueryInterface` on any of them answers with the
/// wrapper of the interface asked for.
#[repr(C)]
pub struct ComObject<T> {
    vtbl: *const c_void,
    shadow: *const Shadow<T>,
}

/// An interface a Rust value is exposed through: its vtable and the test
/// for the identifiers it answers to (its own and those of its bases).
pub struct InterfaceEntry<T> {
    vtbl: *const c_void,
    matches_iid: fn(&Guid) -> bool,
    _value: PhantomData<fn(T)>,
}

impl<T> InterfaceEntry<T> {
    /// The entry of interface `I`, which `T` implements.
    pub fn of<I: ImplementedBy<T>>() -> InterfaceEntry<T> {
        InterfaceEntry { vtbl: I::VTBL as *const I::Vtbl as *const c_void, matches_iid: I::matches_iid, _value: PhantomData }
    }
}

/// The object behind its wrappers (the shadow of the MicroCom runtime): the
/// reference count, the wrappers in the order the interfaces were given,
/// and the Rust value.
struct Shadow<T> {
    refs: AtomicU32,
    wrappers: Box<[Wrapper<T>]>,
    value: T,
}

struct Wrapper<T> {
    object: ComObject<T>,
    matches_iid: fn(&Guid) -> bool,
}

impl<T> ComObject<T> {
    /// Recovers the Rust value from the `this` pointer a thunk receives:
    /// the pointer of any interface of the object.
    ///
    /// # Safety
    /// `this` must point at a live `ComObject<T>`.
    #[inline]
    pub unsafe fn value<'a>(this: *mut c_void) -> &'a T {
        &(*(*(this as *const ComObject<T>)).shadow).value
    }

    /// Current reference count (diagnostics/tests only).
    ///
    /// # Safety
    /// `this` must point at a live `ComObject<T>`.
    pub unsafe fn ref_count(this: *mut c_void) -> u32 {
        (*(*(this as *const ComObject<T>)).shadow).refs.load(Ordering::Relaxed)
    }
}

/// Allocates the object with its wrappers and returns the wrapper of the
/// first interface, with the one reference of the object.
fn new_object<T>(value: T, interfaces: impl Iterator<Item = InterfaceEntry<T>>) -> *mut ComObject<T> {
    let wrappers: Box<[Wrapper<T>]> = interfaces
        .map(|entry| Wrapper {
            object: ComObject { vtbl: entry.vtbl, shadow: std::ptr::null() },
            matches_iid: entry.matches_iid,
        })
        .collect();
    let shadow = Box::into_raw(Box::new(Shadow { refs: AtomicU32::new(1), wrappers, value }));
    // SAFETY: the allocation above, which nothing else refers to yet; the
    // wrappers live in an allocation of their own that does not move for as
    // long as the shadow lives, so pointers to them stay valid.
    unsafe {
        for wrapper in (*shadow).wrappers.iter_mut() {
            wrapper.object.shadow = shadow;
        }
        &raw mut (*shadow).wrappers[0].object
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
    make_com_with(value, [])
}

/// Moves `value` to the heap as a COM object that implements interface `I`
/// and every interface of `more`, and returns the (single) owning reference
/// as `I`.
///
/// `QueryInterface` on any interface of the object answers for all of them
/// (and for their bases): the first interface in the order `I`, `more` that
/// answers to the identifier asked for. `IUnknown` is therefore always the
/// pointer of `I`, whichever interface is asked, which is the identity of
/// the object. The count of references is one for the object, as in the
/// MicroCom runtime, whose shadow of an object holds one wrapper per
/// interface. The object is destroyed as [`make_com`] describes.
pub fn make_com_with<I, T, const N: usize>(value: T, more: [InterfaceEntry<T>; N]) -> ComPtr<I>
where
    I: ImplementedBy<T>,
    T: 'static,
{
    let first = new_object(value, std::iter::once(InterfaceEntry::of::<I>()).chain(more));
    // SAFETY: freshly created object with one reference; the first wrapper
    // carries the vtable of `I`, and the layout contract of
    // `Interface`/`ImplementedBy` makes it a valid `I`.
    unsafe { ComPtr::from_raw(first as *mut I).unwrap_unchecked() }
}

/// The three `IUnknown` slots for a [`ComObject<T>`] exposing interface `I`.
pub struct UnknownThunks<I, T>(PhantomData<(I, T)>);

impl<I: Interface, T> UnknownThunks<I, T> {
    pub const VTBL: IUnknownVtbl = IUnknownVtbl {
        query_interface: Self::query_interface,
        add_ref: Self::add_ref,
        release: Self::release,
    };

    unsafe extern "system" fn query_interface(
        this: *mut c_void,
        riid: *const Guid,
        ppv: *mut *mut c_void,
    ) -> RawHResult {
        if ppv.is_null() || riid.is_null() {
            return E_POINTER;
        }
        let shadow = (*(this as *const ComObject<T>)).shadow;
        for wrapper in (*shadow).wrappers.iter() {
            if (wrapper.matches_iid)(&*riid) {
                Self::add_ref(this);
                *ppv = &raw const wrapper.object as *mut c_void;
                return S_OK;
            }
        }
        *ppv = std::ptr::null_mut();
        E_NOINTERFACE
    }

    unsafe extern "system" fn add_ref(this: *mut c_void) -> u32 {
        let shadow = &*(*(this as *const ComObject<T>)).shadow;
        shadow.refs.fetch_add(1, Ordering::Relaxed) + 1
    }

    unsafe extern "system" fn release(this: *mut c_void) -> u32 {
        let shadow = (*(this as *const ComObject<T>)).shadow as *mut Shadow<T>;
        let left = (*shadow).refs.fetch_sub(1, Ordering::Release) - 1;
        if left == 0 {
            fence(Ordering::Acquire);
            drop(Box::from_raw(shadow));
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

    // Two interfaces written by hand, as the generator writes them: a
    // vtable that begins with the one of `IUnknown`, thunks that recover
    // the value from `this`, and the link to the implementing type.
    macro_rules! test_interface {
        ($name:ident, $vtbl:ident, $imp:ident, $method:ident, $iid:expr) => {
            #[repr(C)]
            struct $name {
                vtbl: *const $vtbl,
            }

            #[repr(C)]
            struct $vtbl {
                base: IUnknownVtbl,
                $method: unsafe extern "system" fn(this: *mut c_void) -> u32,
            }

            trait $imp {
                fn $method(&self) -> u32;
            }

            unsafe impl Interface for $name {
                const IID: Guid = Guid::from_u128($iid);
                const NAME: &'static str = stringify!($name);
                type Vtbl = $vtbl;

                fn matches_iid(iid: &Guid) -> bool {
                    *iid == Self::IID || IUnknown::matches_iid(iid)
                }
            }

            impl $name {
                fn $method(&self) -> u32 {
                    unsafe { ((*self.vtbl).$method)(self as *const _ as *mut c_void) }
                }
            }

            unsafe impl<T: $imp> ImplementedBy<T> for $name {
                const VTBL: &'static $vtbl = &$vtbl {
                    base: IUnknownVtbl::new::<$name, T>(),
                    $method: {
                        unsafe extern "system" fn thunk<T: $imp>(this: *mut c_void) -> u32 {
                            <T as $imp>::$method(ComObject::<T>::value(this))
                        }
                        thunk::<T>
                    },
                };
            }
        };
    }

    test_interface!(IFirst, IFirstVtbl, IFirstImpl, first, 0x11111111_0000_0000_0000_000000000001);
    test_interface!(ISecond, ISecondVtbl, ISecondImpl, second, 0x22222222_0000_0000_0000_000000000002);
    test_interface!(IThird, IThirdVtbl, IThirdImpl, third, 0x33333333_0000_0000_0000_000000000003);

    struct Both(u32, #[allow(dead_code)] DropFlag);

    impl IFirstImpl for Both {
        fn first(&self) -> u32 {
            self.0 + 1
        }
    }

    impl ISecondImpl for Both {
        fn second(&self) -> u32 {
            self.0 + 2
        }
    }

    impl IThirdImpl for Both {
        fn third(&self) -> u32 {
            self.0 + 3
        }
    }

    #[test]
    fn an_object_with_several_interfaces_answers_for_each_of_them() {
        let dropped = Rc::new(Cell::new(false));
        let first: ComPtr<IFirst> =
            make_com_with(Both(40, DropFlag(dropped.clone())), [InterfaceEntry::of::<ISecond>(), InterfaceEntry::of::<IThird>()]);
        let raw = first.as_ptr() as *mut c_void;
        assert_eq!(41, first.first());

        // Every interface is reached from every other, and each call lands
        // in the one value.
        let second = first.cast::<ISecond>().unwrap();
        assert_eq!(42, second.second());
        let third = second.cast::<IThird>().unwrap();
        assert_eq!(43, third.third());
        let back = third.cast::<IFirst>().unwrap();
        assert_eq!(41, back.first());
        assert_eq!(back.as_ptr(), first.as_ptr());
        assert_ne!(second.as_ptr() as *mut c_void, raw);
        assert_ne!(third.as_ptr() as *mut c_void, second.as_ptr() as *mut c_void);

        // One count for the object, whichever interface is held.
        unsafe {
            assert_eq!(4, ComObject::<Both>::ref_count(raw));
            assert_eq!(4, ComObject::<Both>::ref_count(third.as_ptr() as *mut c_void));
        }

        // The identity of the object: `IUnknown` is the same pointer from
        // every interface.
        let unknowns = [
            first.cast::<IUnknown>().unwrap(),
            second.cast::<IUnknown>().unwrap(),
            third.cast::<IUnknown>().unwrap(),
        ];
        assert!(unknowns.iter().all(|unknown| unknown.as_ptr() as *mut c_void == raw));
        drop(unknowns);

        // An interface the object was not made with.
        let mut out = std::ptr::null_mut();
        unsafe {
            assert_eq!(second.as_unknown().query_interface_raw(&Guid::from_u128(0x1234), &mut out), E_NOINTERFACE);
        }
        assert!(out.is_null());

        // The value lives while any interface is held, and is dropped with
        // the last, whichever that is.
        drop(first);
        drop(back);
        drop(third);
        assert!(!dropped.get());
        assert_eq!(42, second.second());
        drop(second);
        assert!(dropped.get());
    }

    #[test]
    fn an_object_with_one_interface_does_not_answer_for_another() {
        let dropped = Rc::new(Cell::new(false));
        let first: ComPtr<IFirst> = make_com(Both(1, DropFlag(dropped.clone())));
        assert_eq!(2, first.first());
        assert_eq!(Err(crate::HResult(E_NOINTERFACE)), first.cast::<ISecond>().map(|_| ()));
        let unknown = first.cast::<IUnknown>().unwrap();
        assert_eq!(unknown.as_ptr() as *mut c_void, first.as_ptr() as *mut c_void);
        drop(first);
        assert!(!dropped.get());
        drop(unknown);
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
