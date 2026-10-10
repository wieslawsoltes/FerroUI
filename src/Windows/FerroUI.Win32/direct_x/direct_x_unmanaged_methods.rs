//! The functions of DXGI and Direct3D 11 the backend calls.

use super::{ID3D11Device, IDXGIAdapter1, IDXGIFactory, IDXGIFactory1, D3D_DRIVER_TYPE, D3D_FEATURE_LEVEL};
use ferroui_microcom::{ComPtr, Guid, HResult, Interface, RawHResult};
use std::ffi::c_void;

#[link(name = "dxgi", kind = "raw-dylib")]
extern "system" {
    fn CreateDXGIFactory(riid: *const Guid, pp_factory: *mut *mut c_void) -> RawHResult;
    fn CreateDXGIFactory1(riid: *const Guid, pp_factory: *mut *mut c_void) -> RawHResult;
}

#[link(name = "d3d11", kind = "raw-dylib")]
extern "system" {
    fn D3D11CreateDevice(
        adapter: *mut c_void,
        driver_type: D3D_DRIVER_TYPE,
        software: isize,
        flags: u32,
        p_feature_levels: *const D3D_FEATURE_LEVEL,
        feature_levels: u32,
        sdk_version: u32,
        pp_device: *mut *mut c_void,
        p_feature_level: *mut D3D_FEATURE_LEVEL,
        pp_immediate_context: *mut *mut c_void,
    ) -> RawHResult;
}

/// The functions of DXGI and Direct3D 11.
///
pub struct DirectXUnmanagedMethods;

impl DirectXUnmanagedMethods {
    /// `CreateDXGIFactory` for the factory of the first version of DXGI:
    /// what the timer of the swap chain mode enumerates the outputs with.
    /// The failure of the call is the error.
    pub fn create_dxgi_factory() -> Result<Option<ComPtr<IDXGIFactory>>, HResult> {
        let mut factory = std::ptr::null_mut();
        // SAFETY: the identifier is the one of the interface the result is
        // read as, and the pointer is valid for the one pointer written.
        HResult::check(unsafe { CreateDXGIFactory(&IDXGIFactory::IID, &mut factory) })?;
        // SAFETY: the call succeeded, so the pointer is null or a reference
        // to a factory this call owns.
        Ok(unsafe { ComPtr::from_raw(factory.cast()) })
    }

    /// `CreateDXGIFactory1` for the first version of the factory. The
    /// failure of the call is the error (the reference lets the runtime
    /// throw for it).
    pub fn create_dxgi_factory1() -> Result<Option<ComPtr<IDXGIFactory1>>, HResult> {
        let mut factory = std::ptr::null_mut();
        // SAFETY: the identifier is the one of the interface the result is
        // read as, and the pointer is valid for the one pointer written.
        HResult::check(unsafe { CreateDXGIFactory1(&IDXGIFactory1::IID, &mut factory) })?;
        // SAFETY: the call succeeded, so the pointer is null or a reference
        // to a factory this call owns.
        Ok(unsafe { ComPtr::from_raw(factory.cast()) })
    }

    /// `D3D11CreateDevice` on an adapter (or on the default adapter of the
    /// driver type) for the first of the feature levels the adapter has.
    /// The immediate context is not asked for.
    pub fn d3d11_create_device(
        adapter: Option<&IDXGIAdapter1>,
        driver_type: D3D_DRIVER_TYPE,
        flags: u32,
        feature_levels: &[D3D_FEATURE_LEVEL],
        sdk_version: u32,
    ) -> Result<(Option<ComPtr<ID3D11Device>>, D3D_FEATURE_LEVEL), HResult> {
        let mut device = std::ptr::null_mut();
        let mut feature_level = D3D_FEATURE_LEVEL::default();
        // SAFETY: the adapter is null or a live adapter; the feature levels
        // are read from a slice of the length given; the two out pointers
        // are valid for the value each receives, and no context is asked
        // for.
        HResult::check(unsafe {
            D3D11CreateDevice(
                adapter.map_or(std::ptr::null_mut(), |adapter| adapter.as_raw().cast()),
                driver_type,
                0,
                flags,
                feature_levels.as_ptr(),
                feature_levels.len() as u32,
                sdk_version,
                &mut device,
                &mut feature_level,
                std::ptr::null_mut(),
            )
        })?;
        // SAFETY: the call succeeded, so the pointer is null or a reference
        // to a device this call owns.
        Ok((unsafe { ComPtr::from_raw(device.cast()) }, feature_level))
    }
}
