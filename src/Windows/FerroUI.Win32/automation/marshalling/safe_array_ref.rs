//! A reference to an array of the automation library of the system (the
//! port of `Marshalling/SafeArrayRef.cs`).
//!
//! The reference converts between arrays of its runtime and safe arrays by
//! testing the element type at run time. Here the element types are the
//! implementations of [`SafeArrayElement`], which says what an element is
//! in the array and how it gets there; the conversions are otherwise the
//! ones of the reference.

use super::com_variant::{ComVariant, VarEnum};
use super::ole_aut;
use ferroui_microcom::{ComPtr, Interface};
use std::ffi::c_void;

/// `SAFEARRAY` of the system, with the one bound of an array of one
/// dimension (the reference declares a variable-length array of bounds and
/// reads the first).
#[repr(C)]
#[derive(Debug)]
pub struct SAFEARRAY {
    /// The number of dimensions.
    pub c_dims: u16,
    /// Flags ([`ADVANCED_FEATURE_FLAGS`]).
    pub f_features: u16,
    /// The size of an array element.
    pub cb_elements: u32,
    /// The number of times the array has been locked without a
    /// corresponding unlock.
    pub c_locks: u32,
    /// The data.
    pub pv_data: *mut c_void,
    /// One bound for each dimension.
    pub rgsabound: [SAFEARRAYBOUND; 1],
}

/// `SAFEARRAYBOUND` of the system.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SAFEARRAYBOUND {
    /// The number of elements in the dimension.
    pub c_elements: u32,
    /// The lower bound of the dimension.
    pub l_lbound: i32,
}

bitflags::bitflags! {
    /// The flags of a safe array.
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    #[allow(non_camel_case_types)]
    pub struct ADVANCED_FEATURE_FLAGS: u16 {
        const FADF_AUTO = 0x0001;
        const FADF_STATIC = 0x0002;
        const FADF_EMBEDDED = 0x0004;
        const FADF_FIXEDSIZE = 0x0010;
        const FADF_RECORD = 0x0020;
        const FADF_HAVEIID = 0x0040;
        const FADF_HAVEVARTYPE = 0x0080;
        const FADF_BSTR = 0x0100;
        const FADF_UNKNOWN = 0x0200;
        const FADF_DISPATCH = 0x0400;
        const FADF_VARIANT = 0x0800;
        const FADF_RESERVED = 0xF008;
    }
}

/// A native array allocation, element conversion, or locking failure (the
/// `Win32Exception` of the reference), with the result code.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SafeArrayLockError(pub i32);

impl std::fmt::Display for SafeArrayLockError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "the safe array could not be marshalled: {:#010x}", self.0)
    }
}

impl std::error::Error for SafeArrayLockError {}

/// What a value is as an element of a safe array.
pub trait SafeArrayElement: Sized {
    /// The type of the elements of the array.
    const VAR_TYPE: VarEnum;
    /// The element as the array stores it.
    type Raw: Copy;

    /// The element for the array, which owns what it is given (a string of
    /// the system, a reference of a COM object).
    fn to_raw(&self) -> Self::Raw;

    /// Allocates an element for the array, propagating native allocation errors.
    /// Implementations that cannot fail may use the default conversion.
    fn try_to_raw(&self) -> Result<Self::Raw, SafeArrayLockError> {
        Ok(self.to_raw())
    }

    /// The value of an element, which stays the array's.
    ///
    /// # Safety
    /// `raw` is an element of a live array of the type [`Self::VAR_TYPE`].
    unsafe fn from_raw(raw: Self::Raw) -> Self;
}

macro_rules! plain_elements {
    ($($type:ty => $var_type:ident),* $(,)?) => {
        $(
            impl SafeArrayElement for $type {
                const VAR_TYPE: VarEnum = VarEnum::$var_type;
                type Raw = $type;

                fn to_raw(&self) -> $type {
                    *self
                }

                unsafe fn from_raw(raw: $type) -> $type {
                    raw
                }
            }
        )*
    };
}

// The reference also maps the integers of the size of a pointer to `VT_INT`
// and `VT_UINT`, whose elements are four bytes on every target, and copies
// them at their own size. Nothing passes such an array, and the mapping is
// not ported.
plain_elements! {
    i8 => VT_I1,
    i16 => VT_I2,
    i32 => VT_I4,
    i64 => VT_I8,
    u8 => VT_UI1,
    u16 => VT_UI2,
    u32 => VT_UI4,
    u64 => VT_UI8,
    f32 => VT_R4,
    f64 => VT_R8,
}

impl SafeArrayElement for bool {
    const VAR_TYPE: VarEnum = VarEnum::VT_BOOL;
    type Raw = i16;

    fn to_raw(&self) -> i16 {
        if *self {
            ComVariant::VARIANT_TRUE
        } else {
            ComVariant::VARIANT_FALSE
        }
    }

    unsafe fn from_raw(raw: i16) -> bool {
        raw != ComVariant::VARIANT_FALSE
    }
}

impl SafeArrayElement for String {
    const VAR_TYPE: VarEnum = VarEnum::VT_BSTR;
    type Raw = *mut u16;

    fn to_raw(&self) -> *mut u16 {
        ole_aut::string_to_bstr(self)
    }

    fn try_to_raw(&self) -> Result<*mut u16, SafeArrayLockError> {
        if u32::try_from(self.encode_utf16().count()).is_err() {
            return Err(SafeArrayLockError(ferroui_microcom::E_INVALIDARG as i32));
        }
        let raw = self.to_raw();
        if raw.is_null() {
            return Err(SafeArrayLockError(0x8007_000Eu32 as i32));
        }
        Ok(raw)
    }

    unsafe fn from_raw(raw: *mut u16) -> String {
        // SAFETY: an element of an array of strings is a string of the
        // library or null.
        unsafe { ole_aut::ptr_to_string_bstr(raw) }.unwrap_or_default()
    }
}

/// A COM interface: the array holds a reference of the object (the
/// reference of the port has no wrappers to make: the pointer of the
/// interface is the object as native code sees it).
impl<I: Interface> SafeArrayElement for ComPtr<I> {
    const VAR_TYPE: VarEnum = VarEnum::VT_UNKNOWN;
    type Raw = *mut c_void;

    fn to_raw(&self) -> *mut c_void {
        self.clone().into_raw().cast::<c_void>()
    }

    /// The reference unwraps objects of its own runtime only and fails for
    /// any other. Here the caller says what the pointers are: the elements
    /// of the array have to be pointers of the interface `I`.
    unsafe fn from_raw(raw: *mut c_void) -> ComPtr<I> {
        // SAFETY: the contract of the function; an element of an array of
        // interface pointers a provider filled is not null.
        unsafe { ComPtr::from_raw_add_ref(raw.cast::<I>()) }.expect("a safe array of interfaces has no null element")
    }
}

/// A reference to a safe array, or to none: a pointer, as the system passes
/// it. Nothing is freed when it goes away: [`SafeArrayRef::destroy`] frees
/// the array, as in the reference.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SafeArrayRef {
    ptr: *mut SAFEARRAY,
}

impl Default for SafeArrayRef {
    fn default() -> Self {
        SafeArrayRef { ptr: std::ptr::null_mut() }
    }
}

impl SafeArrayRef {
    /// The reference to no array.
    pub fn null() -> SafeArrayRef {
        SafeArrayRef::default()
    }

    /// Whether this refers to no array.
    pub fn is_null(&self) -> bool {
        self.ptr.is_null()
    }

    /// The array as the system passes it.
    pub fn as_ptr(&self) -> *mut SAFEARRAY {
        self.ptr
    }

    /// `Destroy`: frees the array and what its elements own.
    ///
    /// # Safety
    /// Nothing uses the array, through this reference or a copy of it,
    /// after the call.
    pub unsafe fn destroy(&self) {
        if !self.ptr.is_null() {
            // SAFETY: the contract of the function.
            unsafe { ole_aut::safe_array_destroy(self.ptr) };
        }
    }

    /// `ToArray<T>`: the elements of the array; `None` for no array.
    ///
    /// # Safety
    /// The reference is null or a live array of one dimension whose
    /// elements are of the type of `T` (for an interface: pointers of that
    /// interface).
    pub unsafe fn to_array<T: SafeArrayElement>(safearray: Option<SafeArrayRef>) -> Result<Option<Vec<T>>, SafeArrayLockError> {
        let Some(safearray) = safearray.filter(|array| !array.is_null()) else {
            return Ok(None);
        };
        // SAFETY: the contract of the function: the data holds `length`
        // elements of `T::Raw`.
        unsafe {
            Self::access_data(safearray, |data, length| {
                let data = data.cast::<T::Raw>();
                (0..length).map(|index| T::from_raw(data.add(index).read_unaligned())).collect()
            })
        }
        .map(Some)
    }

    /// `TryCreate`: an array with the elements of `managed`. `None` for no
    /// values; the type of the elements is [`SafeArrayElement::VAR_TYPE`].
    ///
    /// The reference tells the element type from the values, and has a
    /// second entry point for arrays of COM interfaces, whose type the
    /// values do not show. Here both are this function: the type says it.
    pub fn try_create<T: SafeArrayElement>(managed: Option<&[T]>) -> Result<Option<(SafeArrayRef, VarEnum)>, SafeArrayLockError> {
        let Some(managed) = managed else {
            return Ok(None);
        };
        let count = u32::try_from(managed.len())
            .map_err(|_| SafeArrayLockError(ferroui_microcom::E_INVALIDARG as i32))?;
        let pointer = ole_aut::safe_array_create(T::VAR_TYPE.0, count);
        if pointer.is_null() {
            return Err(SafeArrayLockError(0x8007_000Eu32 as i32));
        }
        let mut pending = PendingSafeArray(SafeArrayRef { ptr: pointer });
        let lock = SafeArrayLock::new(pending.0)?;
        // SAFETY: the allocated array is locked, zero initialized by OleAut,
        // and has `count` slots. Validate the public element implementation's
        // raw size before writing, then transfer each successfully allocated
        // string/reference directly to the array. The guards unlock and free
        // all initialized slots if a later allocation fails or conversion panics.
        unsafe {
            if (*pointer).cb_elements as usize != std::mem::size_of::<T::Raw>() {
                return Err(SafeArrayLockError(ferroui_microcom::E_INVALIDARG as i32));
            }
            let data = (*pointer).pv_data.cast::<T::Raw>();
            for (index, value) in managed.iter().enumerate() {
                data.add(index).write_unaligned(value.try_to_raw()?);
            }
        }
        drop(lock);
        let array = std::mem::take(&mut pending.0);
        Ok(Some((array, T::VAR_TYPE)))
    }

    /// `AccessData`: the data and the number of elements of a locked
    /// array.
    ///
    /// # Safety
    /// `safearray` is a live array of one dimension.
    unsafe fn access_data<R>(
        safearray: SafeArrayRef,
        accessor: impl FnOnce(*const c_void, usize) -> R,
    ) -> Result<R, SafeArrayLockError> {
        let _lock = SafeArrayLock::new(safearray)?;
        // SAFETY: the array is live and locked. The guard also releases the
        // lock if copying an element invokes a conversion that unwinds.
        unsafe {
            debug_assert_eq!((*safearray.ptr).c_dims, 1);
            let data = (*safearray.ptr).pv_data;
            let length = (*safearray.ptr).rgsabound[0].c_elements;
            Ok(accessor(data, length as usize))
        }
    }
}


/// Owns an array until every slot has been initialized successfully.
struct PendingSafeArray(SafeArrayRef);

impl Drop for PendingSafeArray {
    fn drop(&mut self) {
        // SAFETY: this guard owns the array. Any inner lock guard has already
        // dropped; the still-zero slots hold no strings or COM references.
        unsafe { self.0.destroy() };
    }
}

struct SafeArrayLock(SafeArrayRef);

impl SafeArrayLock {
    fn new(array: SafeArrayRef) -> Result<Self, SafeArrayLockError> {
        // SAFETY: creation and access supply a live array, as their contracts
        // require. A failed lock acquires no ownership or element references.
        let result = unsafe { ole_aut::safe_array_lock(array.ptr) };
        if result != 0 {
            Err(SafeArrayLockError(result))
        } else {
            Ok(Self(array))
        }
    }
}

impl Drop for SafeArrayLock {
    fn drop(&mut self) {
        // SAFETY: construction acquired this lock, and the array's owner
        // outlives the guard. The lock is released exactly once.
        unsafe { ole_aut::safe_array_unlock(self.0.ptr) };
    }
}
