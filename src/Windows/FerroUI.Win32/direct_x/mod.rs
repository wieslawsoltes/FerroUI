//! Direct3D 11 and DXGI: the COM interfaces (generated from `directx.idl`
//! by the MicroCom generator in the build script of the crate), the
//! enumerations and structures the reference declares beside them, and the
//! two functions that create a factory and a device.
//!
//! The bindings are values and vtable calls, so they compile on every host;
//! the functions of the system are Windows only.

#![allow(non_camel_case_types)]

mod direct_x_enums;
mod direct_x_structs;
#[cfg(windows)]
mod direct_x_unmanaged_methods;
mod i_direct3_d11_texture_platform_surface;

#[allow(
    clippy::all,
    non_snake_case,
    non_upper_case_globals,
    dead_code,
    missing_docs,
    unused_unsafe,
    unsafe_op_in_unsafe_fn
)]
mod directx {
    include!(concat!(env!("OUT_DIR"), "/directx.rs"));
}

pub use direct_x_enums::*;
pub use direct_x_structs::*;
#[cfg(windows)]
pub use direct_x_unmanaged_methods::DirectXUnmanagedMethods;
pub use directx::*;
pub use i_direct3_d11_texture_platform_surface::*;
