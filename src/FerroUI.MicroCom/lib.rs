//! ferroui-microcom
//!
//! Minimal COM runtime used by the bindings that `microcom-codegen` generates
//! from MicroCom IDL files (the Rust counterpart of the MicroCom runtime and its callback base class).
//!
//! Two directions are covered:
//!
//! * **native -> Rust proxies**: an interface type such as `IFrnWindow` is a
//!   `#[repr(C)]` struct whose only field is the vtable pointer, i.e. a
//!   `*mut IFrnWindow` *is* the native interface pointer. [`ComPtr<T>`] owns
//!   one reference to such an object (`AddRef` on clone, `Release` on drop)
//!   and derefs to `&T`, on which the generated methods live.
//! * **Rust -> native callbacks** (COM callable wrappers): [`ComObject<T>`]
//!   is a heap object whose first word is a vtable pointer, followed by an
//!   atomic reference count and the Rust value. Generated code provides the
//!   vtables (`extern "C"` thunks) for every `T` implementing the interface's
//!   `...Impl` trait through [`ImplementedBy`].
//!
//! ABI: Itanium C++ single-inheritance layout — slot 0..2 are
//! `QueryInterface`, `AddRef`, `Release`, followed by the methods of each
//! interface in the chain in declaration order; `this` is the first argument
//! of every slot and the C calling convention is used.

mod com_object;
mod com_ptr;
mod guid;
mod hresult;
mod unknown;

pub use com_object::{make_com, ComObject, ImplementedBy};
pub use com_ptr::ComPtr;
pub use guid::Guid;
pub use hresult::{
    HResult, RawHResult, COR_E_INVALIDOPERATION, COR_E_OBJECTDISPOSED, E_ABORT, E_FAIL, E_HANDLE,
    E_INVALIDARG, E_NOINTERFACE, E_NOTIMPL, E_POINTER, E_UNEXPECTED, S_OK,
};
pub use unknown::{IUnknown, IUnknownImpl, IUnknownVtbl, Interface};

/// Result of a COM call returning `HRESULT`.
pub type ComResult<T> = Result<T, HResult>;
