//! The Windows Runtime: the COM interfaces (generated from `winrt.idl` by
//! the MicroCom generator in the build script of the crate), string
//! handles, activation, and the helpers the reference declares beside them.
//!
//! The bindings are values and vtable calls, so they compile on every host;
//! the functions of the system are Windows only.

#![allow(non_camel_case_types)]

pub mod numerics;

#[cfg(windows)]
mod native_win_rt_methods;
#[cfg(windows)]
mod win_rt_api_information;
mod win_rt_color;
#[cfg(windows)]
mod win_rt_inspectable;
#[cfg(windows)]
mod win_rt_property_value;

#[cfg(windows)]
pub(crate) use native_win_rt_methods::{HStringInterop, NativeWinRTMethods};
#[cfg(windows)]
#[allow(unused_imports)] // The composition connection of stage 2c creates the queue.
pub(crate) use native_win_rt_methods::{
    DispatcherQueueOptions, DISPATCHERQUEUE_THREAD_APARTMENTTYPE, DISPATCHERQUEUE_THREAD_TYPE,
};
#[cfg(windows)]
pub(crate) use win_rt_api_information::WinRTApiInformation;
pub use win_rt_color::WinRTColor;
#[cfg(windows)]
#[allow(unused_imports)] // The composition effects of stage 2c create the values.
pub(crate) use win_rt_inspectable::WinRTInspectable;
#[cfg(windows)]
#[allow(unused_imports)]
pub(crate) use win_rt_property_value::WinRTPropertyValue;

#[allow(
    clippy::all,
    non_snake_case,
    non_upper_case_globals,
    dead_code,
    missing_docs,
    unused_unsafe,
    unsafe_op_in_unsafe_fn
)]
mod winrt {
    include!(concat!(env!("OUT_DIR"), "/winrt.rs"));
}

pub use winrt::*;

#[cfg(all(test, windows))]
mod win_rt_tests;
