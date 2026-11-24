//! The declarations of `libdrm` and `libgbm` the DRM output uses (the port
//! of `Output/Drm.cs`).
//!
//! The structures are those of `xf86drmMode.h`, `xf86drm.h` and `gbm.h`,
//! declared `#[repr(C)]` and tested for their sizes and the offsets of
//! their fields. The reference declares platform invokes of `libdrm.so.2`
//! and `libgbm.so.1`; here both libraries are opened at run time and their
//! functions resolved into a table ([`lib_drm::LibDrm`]), as the `libinput`
//! backend does (`native_library.rs`).

#![allow(non_camel_case_types)]

/// The state of a connector (`drmModeConnection`).
///
/// A value of the library, which may be one this declaration does not
/// name: a number with the known values as constants.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct DrmModeConnection(pub u32);

impl DrmModeConnection {
    pub const DRM_MODE_CONNECTED: Self = Self(1);
    pub const DRM_MODE_DISCONNECTED: Self = Self(2);
    pub const DRM_MODE_UNKNOWNCONNECTION: Self = Self(3);
}

/// The order of the subpixels of a connector (`drmModeSubPixel`).
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct DrmModeSubPixel(pub u32);

impl DrmModeSubPixel {
    pub const DRM_MODE_SUBPIXEL_UNKNOWN: Self = Self(1);
    pub const DRM_MODE_SUBPIXEL_HORIZONTAL_RGB: Self = Self(2);
    pub const DRM_MODE_SUBPIXEL_HORIZONTAL_BGR: Self = Self(3);
    pub const DRM_MODE_SUBPIXEL_VERTICAL_RGB: Self = Self(4);
    pub const DRM_MODE_SUBPIXEL_VERTICAL_BGR: Self = Self(5);
    pub const DRM_MODE_SUBPIXEL_NONE: Self = Self(6);
}

/// The functions and structures of the two libraries (the static class
/// `LibDrm` of the reference).
pub mod lib_drm {
    use super::{DrmModeConnection, DrmModeSubPixel};
    use ferroui_base::PixelSize;
    use std::ffi::{c_char, c_int, c_uint, c_void};

    /// The handler of a vertical blank event.
    pub type DrmEventVBlankHandlerDelegate =
        unsafe extern "C" fn(fd: c_int, sequence: c_uint, tv_sec: c_uint, tv_usec: c_uint, user_data: *mut c_void);

    /// The handler of a page flip event.
    pub type DrmEventPageFlipHandlerDelegate =
        unsafe extern "C" fn(fd: c_int, sequence: c_uint, tv_sec: c_uint, tv_usec: c_uint, user_data: *mut c_void);

    /// The handler of a page flip event that names the CRTC. (The
    /// reference declares a pointer as its result; the C function returns
    /// nothing.)
    pub type DrmEventPageFlipHandler2Delegate = unsafe extern "C" fn(
        fd: c_int,
        sequence: c_uint,
        tv_sec: c_uint,
        tv_usec: c_uint,
        crtc_id: c_uint,
        user_data: *mut c_void,
    );

    /// The handler of a sequence event.
    pub type DrmEventSequenceHandlerDelegate = unsafe extern "C" fn(fd: c_int, sequence: u64, ns: u64, user_data: u64);

    /// `drmEventContext`: the handlers `drmHandleEvent` calls.
    #[repr(C)]
    #[derive(Clone, Copy, Default)]
    pub struct DrmEventContext {
        /// 4: the version of the structure with every handler below.
        pub version: c_int,
        pub vblank_handler: Option<DrmEventVBlankHandlerDelegate>,
        pub page_flip_handler: Option<DrmEventPageFlipHandlerDelegate>,
        pub page_flip_handler2: Option<DrmEventPageFlipHandler2Delegate>,
        pub sequence_handler: Option<DrmEventSequenceHandlerDelegate>,
    }

    /// `drmModeRes`: the resources of a card.
    #[repr(C)]
    #[derive(Clone, Copy, Debug)]
    pub struct drmModeRes {
        pub count_fbs: c_int,
        pub fbs: *mut u32,

        pub count_crtcs: c_int,
        pub crtcs: *mut u32,

        pub count_connectors: c_int,
        pub connectors: *mut u32,

        pub count_encoders: c_int,
        pub encoders: *mut u32,

        pub(crate) min_width: u32,
        pub(crate) max_width: u32,
        pub(crate) min_height: u32,
        pub(crate) max_height: u32,
    }

    /// The kind of a mode (`DRM_MODE_TYPE_*`), a set of flags.
    #[repr(transparent)]
    #[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
    pub struct DrmModeType(pub u32);

    impl DrmModeType {
        pub const DRM_MODE_TYPE_BUILTIN: Self = Self(1 << 0);
        pub const DRM_MODE_TYPE_CLOCK_C: Self = Self((1 << 1) | Self::DRM_MODE_TYPE_BUILTIN.0);
        pub const DRM_MODE_TYPE_CRTC_C: Self = Self((1 << 2) | Self::DRM_MODE_TYPE_BUILTIN.0);
        pub const DRM_MODE_TYPE_PREFERRED: Self = Self(1 << 3);
        pub const DRM_MODE_TYPE_DEFAULT: Self = Self(1 << 4);
        pub const DRM_MODE_TYPE_USERDEF: Self = Self(1 << 5);
        pub const DRM_MODE_TYPE_DRIVER: Self = Self(1 << 6);

        /// Whether every flag of `flags` is set (`HasAllFlags`).
        pub const fn has_all_flags(self, flags: Self) -> bool {
            self.0 & flags.0 == flags.0
        }
    }

    impl std::ops::BitOr for DrmModeType {
        type Output = Self;

        fn bitor(self, rhs: Self) -> Self {
            Self(self.0 | rhs.0)
        }
    }

    /// `drmModeModeInfo`: a display mode.
    #[repr(C)]
    #[derive(Clone, Copy, Debug)]
    pub struct drmModeModeInfo {
        pub clock: u32,
        pub hdisplay: u16,
        pub hsync_start: u16,
        pub hsync_end: u16,
        pub htotal: u16,
        pub hskew: u16,
        pub vdisplay: u16,
        pub vsync_start: u16,
        pub vsync_end: u16,
        pub vtotal: u16,
        pub vscan: u16,

        pub vrefresh: u32,

        pub flags: u32,
        pub type_: DrmModeType,
        pub name: [c_char; 32],
    }

    impl Default for drmModeModeInfo {
        fn default() -> Self {
            Self {
                clock: 0,
                hdisplay: 0,
                hsync_start: 0,
                hsync_end: 0,
                htotal: 0,
                hskew: 0,
                vdisplay: 0,
                vsync_start: 0,
                vsync_end: 0,
                vtotal: 0,
                vscan: 0,
                vrefresh: 0,
                flags: 0,
                type_: DrmModeType::default(),
                name: [0; 32],
            }
        }
    }

    impl drmModeModeInfo {
        pub fn resolution(&self) -> PixelSize {
            PixelSize::new(self.hdisplay as i32, self.vdisplay as i32)
        }
    }

    /// `drmModeConnector`: a connector and its modes.
    #[repr(C)]
    #[derive(Clone, Copy, Debug)]
    pub struct drmModeConnector {
        pub connector_id: u32,
        /// Encoder currently connected to
        pub encoder_id: u32,
        pub connector_type: u32,
        pub connector_type_id: u32,
        pub connection: DrmModeConnection,
        /// HxW in millimeters
        pub mm_width: u32,
        pub mm_height: u32,
        pub subpixel: DrmModeSubPixel,

        pub count_modes: c_int,
        pub modes: *mut drmModeModeInfo,

        pub count_props: c_int,
        /// List of property ids
        pub props: *mut u32,
        /// List of property values
        pub prop_values: *mut u64,

        pub count_encoders: c_int,
        /// List of encoder ids
        pub encoders: *mut u32,
    }

    /// `drmModeEncoder`.
    #[repr(C)]
    #[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
    pub struct drmModeEncoder {
        pub encoder_id: u32,
        pub encoder_type: u32,
        pub crtc_id: u32,
        pub possible_crtcs: u32,
        pub possible_clones: u32,
    }

    /// `drmModeCrtc`.
    #[repr(C)]
    #[derive(Clone, Copy, Debug, Default)]
    pub struct drmModeCrtc {
        pub crtc_id: u32,
        /// FB id to connect to 0 = disconnect
        pub buffer_id: u32,

        /// Position on the framebuffer
        pub x: u32,
        pub y: u32,
        pub width: u32,
        pub height: u32,
        pub mode_valid: c_int,
        pub mode: drmModeModeInfo,

        /// Number of gamma stops
        pub gamma_size: c_int,
    }

    /// The flags of `drmModePageFlip` (`DRM_MODE_PAGE_FLIP_*`).
    #[repr(transparent)]
    #[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
    pub struct DrmModePageFlip(pub u32);

    impl DrmModePageFlip {
        pub const EVENT: Self = Self(1);
        pub const ASYNC: Self = Self(2);
        pub const ABSOLUTE: Self = Self(4);
        pub const RELATIVE: Self = Self(8);
    }

    /// The uses of a buffer object (`enum gbm_bo_flags`), a set of flags.
    #[repr(transparent)]
    #[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
    pub struct GbmBoFlags(pub u32);

    impl GbmBoFlags {
        /// Buffer is going to be presented to the screen using an API such as KMS
        pub const GBM_BO_USE_SCANOUT: Self = Self(1 << 0);
        /// Buffer is going to be used as cursor
        pub const GBM_BO_USE_CURSOR: Self = Self(1 << 1);
        /// Deprecated
        pub const GBM_BO_USE_CURSOR_64X64: Self = Self::GBM_BO_USE_CURSOR;
        /// Buffer is to be used for rendering - for example it is going to be used
        /// as the storage for a color buffer
        pub const GBM_BO_USE_RENDERING: Self = Self(1 << 2);
        /// Buffer can be used for gbm_bo_write.  This is guaranteed to work
        /// with GBM_BO_USE_CURSOR, but may not work for other combinations.
        pub const GBM_BO_USE_WRITE: Self = Self(1 << 3);
        /// Buffer is linear, i.e. not tiled.
        pub const GBM_BO_USE_LINEAR: Self = Self(1 << 4);
    }

    impl std::ops::BitOr for GbmBoFlags {
        type Output = Self;

        fn bitor(self, rhs: Self) -> Self {
            Self(self.0 | rhs.0)
        }
    }

    /// Called by `libgbm` when a buffer object with user data is destroyed.
    pub type GbmBoUserDataDestroyCallbackDelegate = unsafe extern "C" fn(bo: *mut c_void, data: *mut c_void);

    /// `union gbm_bo_handle`: the handle of a buffer object.
    #[repr(C)]
    #[derive(Clone, Copy)]
    pub union GbmBoHandle {
        pub ptr: *mut c_void,
        pub s32: i32,
        pub u32: u32,
        pub s64: i64,
        pub u64: u64,
    }

    /// The pixel formats of buffers (`GBM_FORMAT_*`).
    pub mod gbm_color_formats {
        /// The code of a format: four characters, the first in the low byte.
        pub const fn four_cc(a: u8, b: u8, c: u8, d: u8) -> u32 {
            (a as u32) | (b as u32) << 8 | (c as u32) << 16 | (d as u32) << 24
        }

        pub const GBM_FORMAT_XRGB8888: u32 = four_cc(b'X', b'R', b'2', b'4');
    }

    #[cfg(target_os = "linux")]
    pub(crate) use imp::LibDrm;

    #[cfg(target_os = "linux")]
    mod imp {
        use super::*;
        use crate::native_library::load_library;
        use std::io;
        use std::sync::OnceLock;

        ferroui_base::native_functions! {
            /// The functions of `libdrm` the output calls.
            pub struct LibDrmFunctions(lib) {
                lib::drmModeGetResources: unsafe extern "C" fn(c_int) -> *mut drmModeRes;
                lib::drmModeFreeResources: unsafe extern "C" fn(*mut drmModeRes);
                lib::drmModeGetConnector: unsafe extern "C" fn(c_int, u32) -> *mut drmModeConnector;
                lib::drmModeGetConnectorCurrent: unsafe extern "C" fn(c_int, u32) -> *mut drmModeConnector;
                lib::drmModeFreeConnector: unsafe extern "C" fn(*mut drmModeConnector);
                lib::drmModeGetEncoder: unsafe extern "C" fn(c_int, u32) -> *mut drmModeEncoder;
                lib::drmModeFreeEncoder: unsafe extern "C" fn(*mut drmModeEncoder);
                lib::drmModeGetCrtc: unsafe extern "C" fn(c_int, u32) -> *mut drmModeCrtc;
                lib::drmModeFreeCrtc: unsafe extern "C" fn(*mut drmModeCrtc);
                lib::drmModeAddFB: unsafe extern "C" fn(c_int, u32, u32, u8, u8, u32, u32, *mut u32) -> c_int;
                lib::drmModeAddFB2: unsafe extern "C" fn(c_int, u32, u32, u32, *const u32, *const u32, *const u32, *mut u32, u32) -> c_int;
                lib::drmModeSetCrtc: unsafe extern "C" fn(c_int, u32, u32, u32, u32, *mut u32, c_int, *mut drmModeModeInfo) -> c_int;
                lib::drmModeRmFB: unsafe extern "C" fn(c_int, u32) -> c_int;
                lib::drmModePageFlip: unsafe extern "C" fn(c_int, u32, u32, u32, *mut c_void) -> c_int;
                lib::drmHandleEvent: unsafe extern "C" fn(c_int, *mut DrmEventContext) -> c_int;
            }
        }

        ferroui_base::native_functions! {
            /// The functions of `libgbm` the output calls.
            pub struct LibGbmFunctions(lib) {
                lib::gbm_create_device: unsafe extern "C" fn(c_int) -> *mut c_void;
                lib::gbm_surface_create: unsafe extern "C" fn(*mut c_void, u32, u32, u32, u32) -> *mut c_void;
                lib::gbm_surface_lock_front_buffer: unsafe extern "C" fn(*mut c_void) -> *mut c_void;
                lib::gbm_surface_release_buffer: unsafe extern "C" fn(*mut c_void, *mut c_void);
                lib::gbm_surface_has_free_buffers: unsafe extern "C" fn(*mut c_void) -> c_int;
                lib::gbm_bo_get_user_data: unsafe extern "C" fn(*mut c_void) -> *mut c_void;
                lib::gbm_bo_set_user_data: unsafe extern "C" fn(*mut c_void, *mut c_void, Option<GbmBoUserDataDestroyCallbackDelegate>);
                lib::gbm_bo_get_width: unsafe extern "C" fn(*mut c_void) -> u32;
                lib::gbm_bo_get_height: unsafe extern "C" fn(*mut c_void) -> u32;
                lib::gbm_bo_get_stride: unsafe extern "C" fn(*mut c_void) -> u32;
                lib::gbm_bo_get_format: unsafe extern "C" fn(*mut c_void) -> u32;
                lib::gbm_bo_get_handle: unsafe extern "C" fn(*mut c_void) -> GbmBoHandle;
            }
        }

        /// The two libraries, opened once for the process.
        pub(crate) struct LibDrm {
            pub(crate) drm: LibDrmFunctions,
            pub(crate) gbm: LibGbmFunctions,
        }

        impl LibDrm {
            /// The libraries; an error that names the library or the
            /// function that is missing.
            pub(crate) fn get() -> io::Result<&'static LibDrm> {
                static LIBRARIES: OnceLock<Result<LibDrm, String>> = OnceLock::new();
                LIBRARIES
                    .get_or_init(|| {
                        Ok(LibDrm {
                            drm: load_library(c"libdrm.so.2", LibDrmFunctions::load)?,
                            gbm: load_library(c"libgbm.so.1", LibGbmFunctions::load)?,
                        })
                    })
                    .as_ref()
                    .map_err(|error| io::Error::new(io::ErrorKind::NotFound, error.clone()))
            }
        }

        // The functions of the reference's class, each a call of the table of its library.
        //
        // # Safety (of every function below)
        // The arguments satisfy the contract of the C function of the same name: a descriptor of
        // an open card, objects of the library that are alive, pointers to as many elements as
        // the function reads or writes.
        #[allow(clippy::missing_safety_doc, clippy::too_many_arguments)]
        impl LibDrm {
            pub(crate) unsafe fn drm_mode_get_resources(&self, fd: c_int) -> *mut drmModeRes {
                unsafe { (self.drm.drmModeGetResources)(fd) }
            }

            pub(crate) unsafe fn drm_mode_free_resources(&self, res: *mut drmModeRes) {
                unsafe { (self.drm.drmModeFreeResources)(res) }
            }

            pub(crate) unsafe fn drm_mode_get_connector(&self, fd: c_int, connector: u32) -> *mut drmModeConnector {
                unsafe { (self.drm.drmModeGetConnector)(fd, connector) }
            }

            pub(crate) unsafe fn drm_mode_get_connector_current(&self, fd: c_int, connector: u32) -> *mut drmModeConnector {
                unsafe { (self.drm.drmModeGetConnectorCurrent)(fd, connector) }
            }

            pub(crate) unsafe fn drm_mode_free_connector(&self, res: *mut drmModeConnector) {
                unsafe { (self.drm.drmModeFreeConnector)(res) }
            }

            pub(crate) unsafe fn drm_mode_get_encoder(&self, fd: c_int, id: u32) -> *mut drmModeEncoder {
                unsafe { (self.drm.drmModeGetEncoder)(fd, id) }
            }

            pub(crate) unsafe fn drm_mode_free_encoder(&self, enc: *mut drmModeEncoder) {
                unsafe { (self.drm.drmModeFreeEncoder)(enc) }
            }

            pub(crate) unsafe fn drm_mode_get_crtc(&self, fd: c_int, id: u32) -> *mut drmModeCrtc {
                unsafe { (self.drm.drmModeGetCrtc)(fd, id) }
            }

            pub(crate) unsafe fn drm_mode_free_crtc(&self, enc: *mut drmModeCrtc) {
                unsafe { (self.drm.drmModeFreeCrtc)(enc) }
            }

            pub(crate) unsafe fn drm_mode_add_fb(
                &self,
                fd: c_int,
                width: u32,
                height: u32,
                depth: u8,
                bpp: u8,
                pitch: u32,
                bo_handle: u32,
                buf_id: &mut u32,
            ) -> c_int {
                unsafe { (self.drm.drmModeAddFB)(fd, width, height, depth, bpp, pitch, bo_handle, buf_id) }
            }

            pub(crate) unsafe fn drm_mode_add_fb2(
                &self,
                fd: c_int,
                width: u32,
                height: u32,
                pixel_format: u32,
                bo_handles: &[u32; 4],
                pitches: &[u32; 4],
                offsets: &[u32; 4],
                buf_id: &mut u32,
                flags: u32,
            ) -> c_int {
                unsafe {
                    (self.drm.drmModeAddFB2)(
                        fd,
                        width,
                        height,
                        pixel_format,
                        bo_handles.as_ptr(),
                        pitches.as_ptr(),
                        offsets.as_ptr(),
                        buf_id,
                        flags,
                    )
                }
            }

            pub(crate) unsafe fn drm_mode_set_crtc(
                &self,
                fd: c_int,
                crtc_id: u32,
                buffer_id: u32,
                x: u32,
                y: u32,
                connectors: *mut u32,
                count: c_int,
                mode: *mut drmModeModeInfo,
            ) -> c_int {
                unsafe { (self.drm.drmModeSetCrtc)(fd, crtc_id, buffer_id, x, y, connectors, count, mode) }
            }

            pub(crate) unsafe fn drm_mode_rm_fb(&self, fd: c_int, id: u32) -> c_int {
                unsafe { (self.drm.drmModeRmFB)(fd, id) }
            }

            pub(crate) unsafe fn drm_mode_page_flip(
                &self,
                fd: c_int,
                crtc_id: u32,
                fb_id: u32,
                flags: DrmModePageFlip,
                user_data: *mut c_void,
            ) -> c_int {
                unsafe { (self.drm.drmModePageFlip)(fd, crtc_id, fb_id, flags.0, user_data) }
            }

            pub(crate) unsafe fn drm_handle_event(&self, fd: c_int, context: *mut DrmEventContext) -> c_int {
                unsafe { (self.drm.drmHandleEvent)(fd, context) }
            }

            pub(crate) unsafe fn gbm_create_device(&self, fd: c_int) -> *mut c_void {
                unsafe { (self.gbm.gbm_create_device)(fd) }
            }

            pub(crate) unsafe fn gbm_surface_create(
                &self,
                device: *mut c_void,
                width: u32,
                height: u32,
                format: u32,
                flags: GbmBoFlags,
            ) -> *mut c_void {
                unsafe { (self.gbm.gbm_surface_create)(device, width, height, format, flags.0) }
            }

            pub(crate) unsafe fn gbm_surface_lock_front_buffer(&self, surface: *mut c_void) -> *mut c_void {
                unsafe { (self.gbm.gbm_surface_lock_front_buffer)(surface) }
            }

            pub(crate) unsafe fn gbm_surface_release_buffer(&self, surface: *mut c_void, bo: *mut c_void) {
                unsafe { (self.gbm.gbm_surface_release_buffer)(surface, bo) }
            }

            pub(crate) unsafe fn gbm_surface_has_free_buffers(&self, surface: *mut c_void) -> c_int {
                unsafe { (self.gbm.gbm_surface_has_free_buffers)(surface) }
            }

            pub(crate) unsafe fn gbm_bo_get_user_data(&self, bo: *mut c_void) -> *mut c_void {
                unsafe { (self.gbm.gbm_bo_get_user_data)(bo) }
            }

            pub(crate) unsafe fn gbm_bo_set_user_data(
                &self,
                bo: *mut c_void,
                user_data: *mut c_void,
                on_free: Option<GbmBoUserDataDestroyCallbackDelegate>,
            ) {
                unsafe { (self.gbm.gbm_bo_set_user_data)(bo, user_data, on_free) }
            }

            pub(crate) unsafe fn gbm_bo_get_width(&self, bo: *mut c_void) -> u32 {
                unsafe { (self.gbm.gbm_bo_get_width)(bo) }
            }

            pub(crate) unsafe fn gbm_bo_get_height(&self, bo: *mut c_void) -> u32 {
                unsafe { (self.gbm.gbm_bo_get_height)(bo) }
            }

            pub(crate) unsafe fn gbm_bo_get_stride(&self, bo: *mut c_void) -> u32 {
                unsafe { (self.gbm.gbm_bo_get_stride)(bo) }
            }

            pub(crate) unsafe fn gbm_bo_get_format(&self, bo: *mut c_void) -> u32 {
                unsafe { (self.gbm.gbm_bo_get_format)(bo) }
            }

            pub(crate) unsafe fn gbm_bo_get_handle(&self, bo: *mut c_void) -> GbmBoHandle {
                unsafe { (self.gbm.gbm_bo_get_handle)(bo) }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    // Not from the reference, which has no tests for this file. The sizes
    // and offsets are those of the C declarations (xf86drmMode.h,
    // xf86drm.h, gbm.h) on the architectures the platform runs on.
    use super::lib_drm::*;
    use super::*;
    use std::mem::{offset_of, size_of};

    #[test]
    fn the_mode_info_is_the_kernels_mode_info_of_68_bytes() {
        assert_eq!(size_of::<drmModeModeInfo>(), 68);
        assert_eq!(offset_of!(drmModeModeInfo, hdisplay), 4);
        assert_eq!(offset_of!(drmModeModeInfo, vdisplay), 14);
        assert_eq!(offset_of!(drmModeModeInfo, vscan), 22);
        assert_eq!(offset_of!(drmModeModeInfo, vrefresh), 24);
        assert_eq!(offset_of!(drmModeModeInfo, flags), 28);
        assert_eq!(offset_of!(drmModeModeInfo, type_), 32);
        assert_eq!(offset_of!(drmModeModeInfo, name), 36);
    }

    #[test]
    fn the_encoder_and_the_crtc_have_the_layout_of_the_library() {
        assert_eq!(size_of::<drmModeEncoder>(), 20);
        assert_eq!(offset_of!(drmModeEncoder, possible_crtcs), 12);
        assert_eq!(size_of::<drmModeCrtc>(), 100);
        assert_eq!(offset_of!(drmModeCrtc, mode_valid), 24);
        assert_eq!(offset_of!(drmModeCrtc, mode), 28);
        assert_eq!(offset_of!(drmModeCrtc, gamma_size), 96);
    }

    #[cfg(target_pointer_width = "64")]
    #[test]
    fn the_structures_with_pointers_have_the_layout_of_the_library_on_64_bits() {
        assert_eq!(size_of::<drmModeRes>(), 80);
        assert_eq!(offset_of!(drmModeRes, crtcs), 24);
        assert_eq!(offset_of!(drmModeRes, count_connectors), 32);
        assert_eq!(offset_of!(drmModeRes, connectors), 40);
        assert_eq!(offset_of!(drmModeRes, count_encoders), 48);
        assert_eq!(offset_of!(drmModeRes, encoders), 56);
        assert_eq!(offset_of!(drmModeRes, min_width), 64);
        assert_eq!(offset_of!(drmModeRes, max_height), 76);

        assert_eq!(size_of::<drmModeConnector>(), 88);
        assert_eq!(offset_of!(drmModeConnector, connection), 16);
        assert_eq!(offset_of!(drmModeConnector, subpixel), 28);
        assert_eq!(offset_of!(drmModeConnector, count_modes), 32);
        assert_eq!(offset_of!(drmModeConnector, modes), 40);
        assert_eq!(offset_of!(drmModeConnector, count_props), 48);
        assert_eq!(offset_of!(drmModeConnector, props), 56);
        assert_eq!(offset_of!(drmModeConnector, prop_values), 64);
        assert_eq!(offset_of!(drmModeConnector, count_encoders), 72);
        assert_eq!(offset_of!(drmModeConnector, encoders), 80);

        assert_eq!(size_of::<DrmEventContext>(), 40);
        assert_eq!(offset_of!(DrmEventContext, vblank_handler), 8);
        assert_eq!(offset_of!(DrmEventContext, page_flip_handler), 16);
        assert_eq!(offset_of!(DrmEventContext, page_flip_handler2), 24);
        assert_eq!(offset_of!(DrmEventContext, sequence_handler), 32);
    }

    #[cfg(target_pointer_width = "32")]
    #[test]
    fn the_structures_with_pointers_have_the_layout_of_the_library_on_32_bits() {
        assert_eq!(size_of::<drmModeRes>(), 48);
        assert_eq!(offset_of!(drmModeRes, connectors), 20);
        assert_eq!(offset_of!(drmModeRes, min_width), 32);
        assert_eq!(size_of::<drmModeConnector>(), 60);
        assert_eq!(offset_of!(drmModeConnector, modes), 36);
        assert_eq!(offset_of!(drmModeConnector, encoders), 56);
        assert_eq!(size_of::<DrmEventContext>(), 20);
    }

    #[test]
    fn the_handle_of_a_buffer_object_is_a_union_of_eight_bytes() {
        assert_eq!(size_of::<GbmBoHandle>(), 8);
        let handle = GbmBoHandle { u64: 0x1_0000_0007 };
        // SAFETY: every field of the union is plain data of the same storage.
        assert_eq!(unsafe { handle.u32 }, 7);
    }

    #[test]
    fn the_flags_and_the_format_have_the_values_of_the_headers() {
        // GBM_FORMAT_XRGB8888 of gbm.h: __gbm_fourcc_code('X', 'R', '2', '4').
        assert_eq!(gbm_color_formats::GBM_FORMAT_XRGB8888, 0x3432_5258);
        assert_eq!((GbmBoFlags::GBM_BO_USE_SCANOUT | GbmBoFlags::GBM_BO_USE_RENDERING).0, 5);
        assert_eq!(GbmBoFlags::GBM_BO_USE_CURSOR_64X64, GbmBoFlags::GBM_BO_USE_CURSOR);
        assert_eq!(DrmModePageFlip::EVENT.0, 1);
        assert_eq!(DrmModeType::DRM_MODE_TYPE_CLOCK_C.0, 3);
        assert!((DrmModeType::DRM_MODE_TYPE_PREFERRED | DrmModeType::DRM_MODE_TYPE_DRIVER)
            .has_all_flags(DrmModeType::DRM_MODE_TYPE_PREFERRED));
        assert!(!DrmModeType::DRM_MODE_TYPE_DRIVER.has_all_flags(DrmModeType::DRM_MODE_TYPE_PREFERRED));
        assert_eq!(DrmModeConnection::DRM_MODE_CONNECTED.0, 1);
        assert_eq!(DrmModeSubPixel::DRM_MODE_SUBPIXEL_NONE.0, 6);
    }
}
