//! The structures of Direct3D 11 and DXGI the backend uses.
//!
//! The layouts are the ones the reference declares. Two of them are not the
//! layouts of the system headers there (`DXGI_MODE_DESC` and
//! `DXGI_SWAP_CHAIN_DESC` declare 16-bit members where the headers have 32
//! bits); they are kept as declared and nothing of the backend passes them
//! to the system: the swap chains are created from `DXGI_SWAP_CHAIN_DESC1`.

#![allow(non_camel_case_types, non_snake_case, missing_docs)]

use super::{
    D3D11_BIND_FLAG, D3D11_RESOURCE_MISC_FLAG, D3D11_USAGE, DXGI_ALPHA_MODE, DXGI_FORMAT, DXGI_MODE_ROTATION,
    DXGI_MODE_SCALING, DXGI_MODE_SCANLINE_ORDER, DXGI_SCALING, DXGI_SWAP_EFFECT,
};
use crate::interop::unmanaged_methods::{POINT, RECT};

/// A handle of the system.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct HANDLE(pub isize);

impl HANDLE {
    pub const INVALID_VALUE: HANDLE = HANDLE(-1);

    pub const NULL: HANDLE = HANDLE(0);
}

impl std::fmt::Display for HANDLE {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DXGI_ADAPTER_DESC {
    pub description: [u16; 128],
    pub vendor_id: u32,
    pub device_id: u32,
    pub sub_sys_id: u32,
    pub revision: u32,
    pub dedicated_video_memory: usize,
    pub dedicated_system_memory: usize,
    pub shared_system_memory: usize,
    pub adapter_luid: u64,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DXGI_ADAPTER_DESC1 {
    pub description: [u16; 128],
    pub vendor_id: u32,
    pub device_id: u32,
    pub sub_sys_id: u32,
    pub revision: u32,
    pub dedicated_video_memory: usize,
    pub dedicated_system_memory: usize,
    pub shared_system_memory: usize,
    pub adapter_luid: u64,
    pub flags: u32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct DXGI_FRAME_STATISTICS {
    pub present_count: u32,
    pub present_refresh_count: u32,
    pub sync_refresh_count: u32,
    pub sync_qpc_time: u64,
    pub sync_gpu_time: u64,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DXGI_GAMMA_CONTROL_CAPABILITIES {
    pub scale_and_offset_supported: i32,
    pub max_converted_value: f32,
    pub min_converted_value: f32,
    pub num_gamma_control_points: u32,
    pub control_point_positions: [f32; 1025],
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DXGI_MAPPED_RECT {
    pub pitch: i32,
    pub p_bits: *mut u8,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct DXGI_MODE_DESC {
    pub width: u16,
    pub height: u16,
    pub refresh_rate: DXGI_RATIONAL,
    pub format: DXGI_FORMAT,
    pub scanline_ordering: DXGI_MODE_SCANLINE_ORDER,
    pub scaling: DXGI_MODE_SCALING,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DXGI_OUTPUT_DESC {
    pub device_name: [u16; 32],
    pub desktop_coordinates: RECT,
    pub attached_to_desktop: i32, // BOOL maps to int.
    pub rotation: DXGI_MODE_ROTATION,
    pub monitor: HANDLE,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DXGI_PRESENT_PARAMETERS {
    pub dirty_rects_count: u32,
    pub p_dirty_rects: *mut RECT,
    pub p_scroll_rect: *mut RECT,
    pub p_scroll_offset: *mut POINT,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct DXGI_RATIONAL {
    pub numerator: u16,
    pub denominator: u16,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct DXGI_RGB {
    pub red: f32,
    pub green: f32,
    pub blue: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct DXGI_RGBA {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct DXGI_SAMPLE_DESC {
    pub count: u32,
    pub quality: u32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct DXGI_SURFACE_DESC {
    pub width: u32,
    pub height: u32,
    pub format: DXGI_FORMAT,
    pub sample_desc: DXGI_SAMPLE_DESC,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct DXGI_SWAP_CHAIN_DESC {
    pub buffer_desc: DXGI_MODE_DESC,
    pub sample_desc: DXGI_SAMPLE_DESC,
    pub buffer_usage: u32,
    pub buffer_count: u16,
    pub output_window: isize,
    pub windowed: i32,
    pub swap_effect: DXGI_SWAP_EFFECT,
    pub flags: u16,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct DXGI_SWAP_CHAIN_DESC1 {
    pub width: u32,
    pub height: u32,
    pub format: DXGI_FORMAT,
    pub stereo: i32, // BOOL maps to int.
    pub sample_desc: DXGI_SAMPLE_DESC,
    pub buffer_usage: u32,
    pub buffer_count: u32,
    pub scaling: DXGI_SCALING,
    pub swap_effect: DXGI_SWAP_EFFECT,
    pub alpha_mode: DXGI_ALPHA_MODE,
    pub flags: u32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct DXGI_SWAP_CHAIN_FULLSCREEN_DESC {
    pub refresh_rate: DXGI_RATIONAL,
    pub scanline_ordering: DXGI_MODE_SCANLINE_ORDER,
    pub scaling: DXGI_MODE_SCALING,
    pub windowed: i32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct D3D11_TEXTURE2D_DESC {
    pub width: u32,
    pub height: u32,
    pub mip_levels: u32,
    pub array_size: u32,
    pub format: DXGI_FORMAT,
    pub sample_desc: DXGI_SAMPLE_DESC,
    pub usage: D3D11_USAGE,
    pub bind_flags: D3D11_BIND_FLAG,
    pub cpu_access_flags: u32,
    pub misc_flags: D3D11_RESOURCE_MISC_FLAG,
}

#[cfg(test)]
mod tests {
    // Not from upstream: the sizes the system headers give the structures
    // the backend passes to the system, on a 64-bit target.
    use super::*;
    use std::mem::size_of;

    #[cfg(target_pointer_width = "64")]
    #[test]
    fn the_structures_passed_to_the_system_have_the_sizes_of_the_headers() {
        assert_eq!(304, size_of::<DXGI_ADAPTER_DESC>());
        assert_eq!(312, size_of::<DXGI_ADAPTER_DESC1>());
        assert_eq!(48, size_of::<DXGI_SWAP_CHAIN_DESC1>());
        assert_eq!(44, size_of::<D3D11_TEXTURE2D_DESC>());
        assert_eq!(96, size_of::<DXGI_OUTPUT_DESC>());
        assert_eq!(32, size_of::<DXGI_PRESENT_PARAMETERS>());
        assert_eq!(32, size_of::<DXGI_FRAME_STATISTICS>());
        assert_eq!(16, size_of::<DXGI_MAPPED_RECT>());
    }

    #[test]
    fn the_handle_values() {
        assert_eq!(-1, HANDLE::INVALID_VALUE.0);
        assert_eq!(HANDLE(0), HANDLE::NULL);
        assert_eq!("7", HANDLE(7).to_string());
    }
}
