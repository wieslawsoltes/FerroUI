//! DirectComposition: the COM interfaces (generated from `dcomp.idl` by the
//! MicroCom generator in the build script of the crate), the structures
//! the reference declares beside them, and the function that creates a
//! device.
//!
//! The bindings are values and vtable calls, so they compile on every host;
//! the function of the system is Windows only.

#![allow(non_camel_case_types)]

#[cfg(windows)]
mod native_methods;
mod native_structs;

#[allow(
    clippy::all,
    non_snake_case,
    non_upper_case_globals,
    dead_code,
    missing_docs,
    unused_unsafe,
    unsafe_op_in_unsafe_fn
)]
mod dcomp {
    include!(concat!(env!("OUT_DIR"), "/dcomp.rs"));
}

pub use dcomp::*;
#[cfg(windows)]
pub use native_methods::NativeMethods;
pub use native_structs::*;

#[cfg(all(test, windows))]
mod d_composition_tests;
