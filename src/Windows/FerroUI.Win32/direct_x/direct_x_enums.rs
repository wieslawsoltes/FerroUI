//! The enumerations of Direct3D 11 and DXGI the backend uses.
//!
//! An enumeration is a value of its integer type with one constant per
//! member, as in the bindings the interface definitions generate: the
//! system may hand over a value that has no member, and flags combine.

#![allow(non_camel_case_types, missing_docs)]

macro_rules! native_enum {
    ($(#[$attr:meta])* $name:ident : $repr:ty { $($member:ident = $value:expr,)* }) => {
        $(#[$attr])*
        #[repr(transparent)]
        #[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
        pub struct $name(pub $repr);

        impl $name {
            $(pub const $member: Self = Self($value);)*
        }

        impl ::core::ops::BitOr for $name {
            type Output = Self;
            fn bitor(self, rhs: Self) -> Self {
                Self(self.0 | rhs.0)
            }
        }

        impl ::core::ops::BitAnd for $name {
            type Output = Self;
            fn bitand(self, rhs: Self) -> Self {
                Self(self.0 & rhs.0)
            }
        }
    };
}


native_enum! {
    D3D_FEATURE_LEVEL: i32 {
        D3D_FEATURE_LEVEL_1_0_CORE = 0x1000,
        D3D_FEATURE_LEVEL_9_1 = 0x9100,
        D3D_FEATURE_LEVEL_9_2 = 0x9200,
        D3D_FEATURE_LEVEL_9_3 = 0x9300,
        D3D_FEATURE_LEVEL_10_0 = 0xA000,
        D3D_FEATURE_LEVEL_10_1 = 0xA100,
        D3D_FEATURE_LEVEL_11_0 = 0xB000,
        D3D_FEATURE_LEVEL_11_1 = 0xB100,
        D3D_FEATURE_LEVEL_12_0 = 0xC000,
        D3D_FEATURE_LEVEL_12_1 = 0xC100,
        D3D_FEATURE_LEVEL_12_2 = 0xC200,
    }
}

native_enum! {
    D3D11_RESOURCE_DIMENSION: i32 {
        D3D11_USAGE_DEFAULT = 0,
        D3D11_USAGE_IMMUTABLE = 1,
        D3D11_USAGE_DYNAMIC = 2,
        D3D11_USAGE_STAGING = 3,
    }
}

native_enum! {
    D3D11_USAGE: i32 {
        D3D11_USAGE_DEFAULT = 0,
        D3D11_USAGE_IMMUTABLE = 1,
        D3D11_USAGE_DYNAMIC = 2,
        D3D11_USAGE_STAGING = 3,
    }
}

native_enum! {
    /// A set of flags.
    D3D11_RESOURCE_MISC_FLAG: i32 {
        D3D11_RESOURCE_MISC_GENERATE_MIPS = 0x0000_0001,
        D3D11_RESOURCE_MISC_SHARED = 0x0000_0002,
        D3D11_RESOURCE_MISC_TEXTURECUBE = 0x0000_0004,
        D3D11_RESOURCE_MISC_DRAWINDIRECT_ARGS = 0x0000_0010,
        D3D11_RESOURCE_MISC_BUFFER_ALLOW_RAW_VIEWS = 0x0000_0020,
        D3D11_RESOURCE_MISC_BUFFER_STRUCTURED = 0x0000_0040,
        D3D11_RESOURCE_MISC_RESOURCE_CLAMP = 0x0000_0080,
        D3D11_RESOURCE_MISC_SHARED_KEYEDMUTEX = 0x0000_0100,
        D3D11_RESOURCE_MISC_GDI_COMPATIBLE = 0x0000_0200,
        D3D11_RESOURCE_MISC_SHARED_NTHANDLE = 0x0000_0800,
        D3D11_RESOURCE_MISC_RESTRICTED_CONTENT = 0x0000_1000,
        D3D11_RESOURCE_MISC_RESTRICT_SHARED_RESOURCE = 0x0000_2000,
        D3D11_RESOURCE_MISC_RESTRICT_SHARED_RESOURCE_DRIVER = 0x0000_4000,
        D3D11_RESOURCE_MISC_GUARDED = 0x0000_8000,
        D3D11_RESOURCE_MISC_TILE_POOL = 0x0002_0000,
        D3D11_RESOURCE_MISC_TILED = 0x0004_0000,
        D3D11_RESOURCE_MISC_HW_PROTECTED = 0x0008_0000,
    }
}

native_enum! {
    /// A set of flags.
    D3D11_BIND_FLAG: i32 {
        D3D11_BIND_VERTEX_BUFFER = 0x0000_0001,
        D3D11_BIND_INDEX_BUFFER = 0x0000_0002,
        D3D11_BIND_CONSTANT_BUFFER = 0x0000_0004,
        D3D11_BIND_SHADER_RESOURCE = 0x0000_0008,
        D3D11_BIND_STREAM_OUTPUT = 0x0000_0010,
        D3D11_BIND_RENDER_TARGET = 0x0000_0020,
        D3D11_BIND_DEPTH_STENCIL = 0x0000_0040,
        D3D11_BIND_UNORDERED_ACCESS = 0x0000_0080,
        D3D11_BIND_DECODER = 0x0000_0200,
        D3D11_BIND_VIDEO_ENCODER = 0x0000_0400,
    }
}

native_enum! {
    DXGI_SWAP_EFFECT: i32 {
        DXGI_SWAP_EFFECT_DISCARD = 0,
        DXGI_SWAP_EFFECT_SEQUENTIAL = 1,
        DXGI_SWAP_EFFECT_FLIP_SEQUENTIAL = 3,
        DXGI_SWAP_EFFECT_FLIP_DISCARD = 4,
    }
}

native_enum! {
    /// A set of flags.
    DXGI_SWAP_CHAIN_FLAG: i32 {
        DXGI_SWAP_CHAIN_FLAG_NONPREROTATED = 1,
        DXGI_SWAP_CHAIN_FLAG_ALLOW_MODE_SWITCH = 2,
        DXGI_SWAP_CHAIN_FLAG_GDI_COMPATIBLE = 4,
        DXGI_SWAP_CHAIN_FLAG_RESTRICTED_CONTENT = 8,
        DXGI_SWAP_CHAIN_FLAG_RESTRICT_SHARED_RESOURCE_DRIVER = 16,
        DXGI_SWAP_CHAIN_FLAG_DISPLAY_ONLY = 32,
        DXGI_SWAP_CHAIN_FLAG_FRAME_LATENCY_WAITABLE_OBJECT = 64,
        DXGI_SWAP_CHAIN_FLAG_FOREGROUND_LAYER = 128,
        DXGI_SWAP_CHAIN_FLAG_FULLSCREEN_VIDEO = 256,
        DXGI_SWAP_CHAIN_FLAG_YUV_VIDEO = 512,
        DXGI_SWAP_CHAIN_FLAG_HW_PROTECTED = 1024,
        DXGI_SWAP_CHAIN_FLAG_ALLOW_TEARING = 2048,
        DXGI_SWAP_CHAIN_FLAG_RESTRICTED_TO_ALL_HOLOGRAPHIC_DISPLAYS = 4096,
    }
}

native_enum! {
    DXGI_SCALING: i32 {
        DXGI_SCALING_STRETCH = 0,
        DXGI_SCALING_NONE = 1,
        DXGI_SCALING_ASPECT_RATIO_STRETCH = 2,
    }
}

native_enum! {
    DXGI_RESIDENCY: i32 {
        DXGI_RESIDENCY_FULLY_RESIDENT = 1,
        DXGI_RESIDENCY_RESIDENT_IN_SHARED_MEMORY = 2,
        DXGI_RESIDENCY_EVICTED_TO_DISK = 3,
    }
}

native_enum! {
    DXGI_MODE_ROTATION: i32 {
        DXGI_MODE_ROTATION_UNSPECIFIED = 0,
        DXGI_MODE_ROTATION_IDENTITY = 1,
        DXGI_MODE_ROTATION_ROTATE90 = 2,
        DXGI_MODE_ROTATION_ROTATE180 = 3,
        DXGI_MODE_ROTATION_ROTATE270 = 4,
    }
}

native_enum! {
    DXGI_ALPHA_MODE: i32 {
        DXGI_ALPHA_MODE_UNSPECIFIED = 0,
        DXGI_ALPHA_MODE_PREMULTIPLIED = 1,
        DXGI_ALPHA_MODE_STRAIGHT = 2,
        DXGI_ALPHA_MODE_IGNORE = 3,
        DXGI_ALPHA_MODE_FORCE_DWORD = -1,
    }
}

native_enum! {
    D3D_DRIVER_TYPE: i32 {
        D3D_DRIVER_TYPE_UNKNOWN = 0,
        D3D_DRIVER_TYPE_HARDWARE = 1,
        D3D_DRIVER_TYPE_REFERENCE = 2,
        D3D_DRIVER_TYPE_NULL = 3,
        D3D_DRIVER_TYPE_SOFTWARE = 4,
        D3D_DRIVER_TYPE_WARP = 5,
    }
}

native_enum! {
    DXGI_ERROR: u32 {
        DXGI_ERROR_ACCESS_DENIED = 0x887A_002B,
        DXGI_ERROR_ACCESS_LOST = 0x887A_0026,
        DXGI_ERROR_ALREADY_EXISTS = 0x887A_0036,
        DXGI_ERROR_CANNOT_PROTECT_CONTENT = 0x887A_002A,
        DXGI_ERROR_DEVICE_HUNG = 0x887A_0006,
        DXGI_ERROR_DEVICE_REMOVED = 0x887A_0005,
        DXGI_ERROR_DEVICE_RESET = 0x887A_0007,
        DXGI_ERROR_DRIVER_INTERNAL_ERROR = 0x887A_0020,
        DXGI_ERROR_FRAME_STATISTICS_DISJOINT = 0x887A_000B,
        DXGI_ERROR_GRAPHICS_VIDPN_SOURCE_IN_USE = 0x887A_000C,
        DXGI_ERROR_INVALID_CALL = 0x887A_0001,
        DXGI_ERROR_MORE_DATA = 0x887A_0003,
        DXGI_ERROR_NAME_ALREADY_EXISTS = 0x887A_002C,
        DXGI_ERROR_NONEXCLUSIVE = 0x887A_0021,
        DXGI_ERROR_NOT_CURRENTLY_AVAILABLE = 0x887A_0022,
        DXGI_ERROR_NOT_FOUND = 0x887A_0002,
        DXGI_ERROR_REMOTE_CLIENT_DISCONNECTED = 0x887A_0023,
        DXGI_ERROR_REMOTE_OUTOFMEMORY = 0x887A_0024,
        DXGI_ERROR_RESTRICT_TO_OUTPUT_STALE = 0x887A_0029,
        DXGI_ERROR_SDK_COMPONENT_MISSING = 0x887A_002D,
        DXGI_ERROR_SESSION_DISCONNECTED = 0x887A_0028,
        DXGI_ERROR_UNSUPPORTED = 0x887A_0004,
        DXGI_ERROR_WAIT_TIMEOUT = 0x887A_0027,
        DXGI_ERROR_WAS_STILL_DRAWING = 0x887A_000A,
    }
}

native_enum! {
    /// A set of flags.
    DXGI_MWA: u32 {
        DXGI_MWA_NO_WINDOW_CHANGES = 1,
        DXGI_MWA_NO_ALT_ENTER = 2,
        DXGI_MWA_NO_PRINT_SCREEN = 4,
    }
}

/// Whether an error of DXGI means that the device is lost.
pub trait DxgiErrorExtensions {
    fn is_device_lost_error(self) -> bool;
}

impl DxgiErrorExtensions for DXGI_ERROR {
    fn is_device_lost_error(self) -> bool {
        matches!(
            self,
            DXGI_ERROR::DXGI_ERROR_DEVICE_REMOVED
                | DXGI_ERROR::DXGI_ERROR_DEVICE_HUNG
                | DXGI_ERROR::DXGI_ERROR_DEVICE_RESET
                | DXGI_ERROR::DXGI_ERROR_NOT_CURRENTLY_AVAILABLE
        )
    }
}

#[cfg(test)]
mod tests {
    // Not from upstream.
    use super::*;

    #[test]
    fn the_values_are_the_ones_of_the_system_headers() {
        assert_eq!(0xB100, D3D_FEATURE_LEVEL::D3D_FEATURE_LEVEL_11_1.0);
        assert_eq!(0x9100, D3D_FEATURE_LEVEL::D3D_FEATURE_LEVEL_9_1.0);
        assert_eq!(5, D3D_DRIVER_TYPE::D3D_DRIVER_TYPE_WARP.0);
        assert_eq!(4, DXGI_SWAP_EFFECT::DXGI_SWAP_EFFECT_FLIP_DISCARD.0);
        assert_eq!(-1, DXGI_ALPHA_MODE::DXGI_ALPHA_MODE_FORCE_DWORD.0);
        assert_eq!(0x887A_0005, DXGI_ERROR::DXGI_ERROR_DEVICE_REMOVED.0);
        assert_eq!(
            0x28,
            (D3D11_BIND_FLAG::D3D11_BIND_SHADER_RESOURCE | D3D11_BIND_FLAG::D3D11_BIND_RENDER_TARGET).0
        );
    }

    #[test]
    fn the_errors_of_a_lost_device() {
        assert!(DXGI_ERROR::DXGI_ERROR_DEVICE_REMOVED.is_device_lost_error());
        assert!(DXGI_ERROR::DXGI_ERROR_DEVICE_HUNG.is_device_lost_error());
        assert!(DXGI_ERROR::DXGI_ERROR_DEVICE_RESET.is_device_lost_error());
        assert!(DXGI_ERROR::DXGI_ERROR_NOT_CURRENTLY_AVAILABLE.is_device_lost_error());
        assert!(!DXGI_ERROR::DXGI_ERROR_INVALID_CALL.is_device_lost_error());
        assert!(!DXGI_ERROR(0).is_device_lost_error());
    }
}
