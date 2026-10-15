//! The enumerations of Direct3D 11 and DXGI the backend uses.
//!
//! An enumeration is a value of its integer type with one constant per
//! member, as in the bindings the interface definitions generate: the
//! system may hand over a value that has no member, and flags combine.

#![allow(non_camel_case_types, missing_docs)]

/// The operators of an enumeration of flags: an enumeration is a value of
/// its integer type, so flags combine as the integers do.
macro_rules! native_enum_ops {
    ($name:ident) => {
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

#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct D3D_FEATURE_LEVEL(pub i32);

impl D3D_FEATURE_LEVEL {
    pub const D3D_FEATURE_LEVEL_1_0_CORE: Self = Self(0x1000);
    pub const D3D_FEATURE_LEVEL_9_1: Self = Self(0x9100);
    pub const D3D_FEATURE_LEVEL_9_2: Self = Self(0x9200);
    pub const D3D_FEATURE_LEVEL_9_3: Self = Self(0x9300);
    pub const D3D_FEATURE_LEVEL_10_0: Self = Self(0xA000);
    pub const D3D_FEATURE_LEVEL_10_1: Self = Self(0xA100);
    pub const D3D_FEATURE_LEVEL_11_0: Self = Self(0xB000);
    pub const D3D_FEATURE_LEVEL_11_1: Self = Self(0xB100);
    pub const D3D_FEATURE_LEVEL_12_0: Self = Self(0xC000);
    pub const D3D_FEATURE_LEVEL_12_1: Self = Self(0xC100);
    pub const D3D_FEATURE_LEVEL_12_2: Self = Self(0xC200);
}

native_enum_ops!(D3D_FEATURE_LEVEL);

#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct D3D11_RESOURCE_DIMENSION(pub i32);

impl D3D11_RESOURCE_DIMENSION {
    pub const D3D11_USAGE_DEFAULT: Self = Self(0);
    pub const D3D11_USAGE_IMMUTABLE: Self = Self(1);
    pub const D3D11_USAGE_DYNAMIC: Self = Self(2);
    pub const D3D11_USAGE_STAGING: Self = Self(3);
}

native_enum_ops!(D3D11_RESOURCE_DIMENSION);

#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct D3D11_USAGE(pub i32);

impl D3D11_USAGE {
    pub const D3D11_USAGE_DEFAULT: Self = Self(0);
    pub const D3D11_USAGE_IMMUTABLE: Self = Self(1);
    pub const D3D11_USAGE_DYNAMIC: Self = Self(2);
    pub const D3D11_USAGE_STAGING: Self = Self(3);
}

native_enum_ops!(D3D11_USAGE);

/// A set of flags.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct D3D11_RESOURCE_MISC_FLAG(pub i32);

impl D3D11_RESOURCE_MISC_FLAG {
    pub const D3D11_RESOURCE_MISC_GENERATE_MIPS: Self = Self(0x0000_0001);
    pub const D3D11_RESOURCE_MISC_SHARED: Self = Self(0x0000_0002);
    pub const D3D11_RESOURCE_MISC_TEXTURECUBE: Self = Self(0x0000_0004);
    pub const D3D11_RESOURCE_MISC_DRAWINDIRECT_ARGS: Self = Self(0x0000_0010);
    pub const D3D11_RESOURCE_MISC_BUFFER_ALLOW_RAW_VIEWS: Self = Self(0x0000_0020);
    pub const D3D11_RESOURCE_MISC_BUFFER_STRUCTURED: Self = Self(0x0000_0040);
    pub const D3D11_RESOURCE_MISC_RESOURCE_CLAMP: Self = Self(0x0000_0080);
    pub const D3D11_RESOURCE_MISC_SHARED_KEYEDMUTEX: Self = Self(0x0000_0100);
    pub const D3D11_RESOURCE_MISC_GDI_COMPATIBLE: Self = Self(0x0000_0200);
    pub const D3D11_RESOURCE_MISC_SHARED_NTHANDLE: Self = Self(0x0000_0800);
    pub const D3D11_RESOURCE_MISC_RESTRICTED_CONTENT: Self = Self(0x0000_1000);
    pub const D3D11_RESOURCE_MISC_RESTRICT_SHARED_RESOURCE: Self = Self(0x0000_2000);
    pub const D3D11_RESOURCE_MISC_RESTRICT_SHARED_RESOURCE_DRIVER: Self = Self(0x0000_4000);
    pub const D3D11_RESOURCE_MISC_GUARDED: Self = Self(0x0000_8000);
    pub const D3D11_RESOURCE_MISC_TILE_POOL: Self = Self(0x0002_0000);
    pub const D3D11_RESOURCE_MISC_TILED: Self = Self(0x0004_0000);
    pub const D3D11_RESOURCE_MISC_HW_PROTECTED: Self = Self(0x0008_0000);
}

native_enum_ops!(D3D11_RESOURCE_MISC_FLAG);

/// A set of flags.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct D3D11_BIND_FLAG(pub i32);

impl D3D11_BIND_FLAG {
    pub const D3D11_BIND_VERTEX_BUFFER: Self = Self(0x0000_0001);
    pub const D3D11_BIND_INDEX_BUFFER: Self = Self(0x0000_0002);
    pub const D3D11_BIND_CONSTANT_BUFFER: Self = Self(0x0000_0004);
    pub const D3D11_BIND_SHADER_RESOURCE: Self = Self(0x0000_0008);
    pub const D3D11_BIND_STREAM_OUTPUT: Self = Self(0x0000_0010);
    pub const D3D11_BIND_RENDER_TARGET: Self = Self(0x0000_0020);
    pub const D3D11_BIND_DEPTH_STENCIL: Self = Self(0x0000_0040);
    pub const D3D11_BIND_UNORDERED_ACCESS: Self = Self(0x0000_0080);
    pub const D3D11_BIND_DECODER: Self = Self(0x0000_0200);
    pub const D3D11_BIND_VIDEO_ENCODER: Self = Self(0x0000_0400);
}

native_enum_ops!(D3D11_BIND_FLAG);

#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct DXGI_SWAP_EFFECT(pub i32);

impl DXGI_SWAP_EFFECT {
    pub const DXGI_SWAP_EFFECT_DISCARD: Self = Self(0);
    pub const DXGI_SWAP_EFFECT_SEQUENTIAL: Self = Self(1);
    pub const DXGI_SWAP_EFFECT_FLIP_SEQUENTIAL: Self = Self(3);
    pub const DXGI_SWAP_EFFECT_FLIP_DISCARD: Self = Self(4);
}

native_enum_ops!(DXGI_SWAP_EFFECT);

/// A set of flags.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct DXGI_SWAP_CHAIN_FLAG(pub i32);

impl DXGI_SWAP_CHAIN_FLAG {
    pub const DXGI_SWAP_CHAIN_FLAG_NONPREROTATED: Self = Self(1);
    pub const DXGI_SWAP_CHAIN_FLAG_ALLOW_MODE_SWITCH: Self = Self(2);
    pub const DXGI_SWAP_CHAIN_FLAG_GDI_COMPATIBLE: Self = Self(4);
    pub const DXGI_SWAP_CHAIN_FLAG_RESTRICTED_CONTENT: Self = Self(8);
    pub const DXGI_SWAP_CHAIN_FLAG_RESTRICT_SHARED_RESOURCE_DRIVER: Self = Self(16);
    pub const DXGI_SWAP_CHAIN_FLAG_DISPLAY_ONLY: Self = Self(32);
    pub const DXGI_SWAP_CHAIN_FLAG_FRAME_LATENCY_WAITABLE_OBJECT: Self = Self(64);
    pub const DXGI_SWAP_CHAIN_FLAG_FOREGROUND_LAYER: Self = Self(128);
    pub const DXGI_SWAP_CHAIN_FLAG_FULLSCREEN_VIDEO: Self = Self(256);
    pub const DXGI_SWAP_CHAIN_FLAG_YUV_VIDEO: Self = Self(512);
    pub const DXGI_SWAP_CHAIN_FLAG_HW_PROTECTED: Self = Self(1024);
    pub const DXGI_SWAP_CHAIN_FLAG_ALLOW_TEARING: Self = Self(2048);
    pub const DXGI_SWAP_CHAIN_FLAG_RESTRICTED_TO_ALL_HOLOGRAPHIC_DISPLAYS: Self = Self(4096);
}

native_enum_ops!(DXGI_SWAP_CHAIN_FLAG);

#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct DXGI_SCALING(pub i32);

impl DXGI_SCALING {
    pub const DXGI_SCALING_STRETCH: Self = Self(0);
    pub const DXGI_SCALING_NONE: Self = Self(1);
    pub const DXGI_SCALING_ASPECT_RATIO_STRETCH: Self = Self(2);
}

native_enum_ops!(DXGI_SCALING);

#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct DXGI_RESIDENCY(pub i32);

impl DXGI_RESIDENCY {
    pub const DXGI_RESIDENCY_FULLY_RESIDENT: Self = Self(1);
    pub const DXGI_RESIDENCY_RESIDENT_IN_SHARED_MEMORY: Self = Self(2);
    pub const DXGI_RESIDENCY_EVICTED_TO_DISK: Self = Self(3);
}

native_enum_ops!(DXGI_RESIDENCY);

#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct DXGI_MODE_ROTATION(pub i32);

impl DXGI_MODE_ROTATION {
    pub const DXGI_MODE_ROTATION_UNSPECIFIED: Self = Self(0);
    pub const DXGI_MODE_ROTATION_IDENTITY: Self = Self(1);
    pub const DXGI_MODE_ROTATION_ROTATE90: Self = Self(2);
    pub const DXGI_MODE_ROTATION_ROTATE180: Self = Self(3);
    pub const DXGI_MODE_ROTATION_ROTATE270: Self = Self(4);
}

native_enum_ops!(DXGI_MODE_ROTATION);

#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct DXGI_ALPHA_MODE(pub i32);

impl DXGI_ALPHA_MODE {
    pub const DXGI_ALPHA_MODE_UNSPECIFIED: Self = Self(0);
    pub const DXGI_ALPHA_MODE_PREMULTIPLIED: Self = Self(1);
    pub const DXGI_ALPHA_MODE_STRAIGHT: Self = Self(2);
    pub const DXGI_ALPHA_MODE_IGNORE: Self = Self(3);
    pub const DXGI_ALPHA_MODE_FORCE_DWORD: Self = Self(-1);
}

native_enum_ops!(DXGI_ALPHA_MODE);

#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct D3D_DRIVER_TYPE(pub i32);

impl D3D_DRIVER_TYPE {
    pub const D3D_DRIVER_TYPE_UNKNOWN: Self = Self(0);
    pub const D3D_DRIVER_TYPE_HARDWARE: Self = Self(1);
    pub const D3D_DRIVER_TYPE_REFERENCE: Self = Self(2);
    pub const D3D_DRIVER_TYPE_NULL: Self = Self(3);
    pub const D3D_DRIVER_TYPE_SOFTWARE: Self = Self(4);
    pub const D3D_DRIVER_TYPE_WARP: Self = Self(5);
}

native_enum_ops!(D3D_DRIVER_TYPE);

#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct DXGI_ERROR(pub u32);

impl DXGI_ERROR {
    pub const DXGI_ERROR_ACCESS_DENIED: Self = Self(0x887A_002B);
    pub const DXGI_ERROR_ACCESS_LOST: Self = Self(0x887A_0026);
    pub const DXGI_ERROR_ALREADY_EXISTS: Self = Self(0x887A_0036);
    pub const DXGI_ERROR_CANNOT_PROTECT_CONTENT: Self = Self(0x887A_002A);
    pub const DXGI_ERROR_DEVICE_HUNG: Self = Self(0x887A_0006);
    pub const DXGI_ERROR_DEVICE_REMOVED: Self = Self(0x887A_0005);
    pub const DXGI_ERROR_DEVICE_RESET: Self = Self(0x887A_0007);
    pub const DXGI_ERROR_DRIVER_INTERNAL_ERROR: Self = Self(0x887A_0020);
    pub const DXGI_ERROR_FRAME_STATISTICS_DISJOINT: Self = Self(0x887A_000B);
    pub const DXGI_ERROR_GRAPHICS_VIDPN_SOURCE_IN_USE: Self = Self(0x887A_000C);
    pub const DXGI_ERROR_INVALID_CALL: Self = Self(0x887A_0001);
    pub const DXGI_ERROR_MORE_DATA: Self = Self(0x887A_0003);
    pub const DXGI_ERROR_NAME_ALREADY_EXISTS: Self = Self(0x887A_002C);
    pub const DXGI_ERROR_NONEXCLUSIVE: Self = Self(0x887A_0021);
    pub const DXGI_ERROR_NOT_CURRENTLY_AVAILABLE: Self = Self(0x887A_0022);
    pub const DXGI_ERROR_NOT_FOUND: Self = Self(0x887A_0002);
    pub const DXGI_ERROR_REMOTE_CLIENT_DISCONNECTED: Self = Self(0x887A_0023);
    pub const DXGI_ERROR_REMOTE_OUTOFMEMORY: Self = Self(0x887A_0024);
    pub const DXGI_ERROR_RESTRICT_TO_OUTPUT_STALE: Self = Self(0x887A_0029);
    pub const DXGI_ERROR_SDK_COMPONENT_MISSING: Self = Self(0x887A_002D);
    pub const DXGI_ERROR_SESSION_DISCONNECTED: Self = Self(0x887A_0028);
    pub const DXGI_ERROR_UNSUPPORTED: Self = Self(0x887A_0004);
    pub const DXGI_ERROR_WAIT_TIMEOUT: Self = Self(0x887A_0027);
    pub const DXGI_ERROR_WAS_STILL_DRAWING: Self = Self(0x887A_000A);
}

native_enum_ops!(DXGI_ERROR);

/// A set of flags.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct DXGI_MWA(pub u32);

impl DXGI_MWA {
    pub const DXGI_MWA_NO_WINDOW_CHANGES: Self = Self(1);
    pub const DXGI_MWA_NO_ALT_ENTER: Self = Self(2);
    pub const DXGI_MWA_NO_PRINT_SCREEN: Self = Self(4);
}

native_enum_ops!(DXGI_MWA);

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
