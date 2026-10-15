//! The type of a connector (the port of `DrmConnectorType.cs`).

/// specific the type of connector is HDMI-A, DVI, DisplayPort, etc.
///
/// The value the library reports (`DRM_MODE_CONNECTOR_*`), which may be
/// one this declaration does not name, as the enumeration of the reference
/// holds any number: a number with the known values as constants.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct DrmConnectorType(pub u32);

impl DrmConnectorType {
    pub const NONE: Self = Self(0);
    pub const VGA: Self = Self(1);
    pub const DVI_I: Self = Self(2);
    pub const DVI_D: Self = Self(3);
    pub const DVI_A: Self = Self(4);
    pub const COMPOSITE: Self = Self(5);
    pub const S_VIDEO: Self = Self(6);
    pub const LVDS: Self = Self(7);
    pub const COMPONENT: Self = Self(8);
    pub const DIN: Self = Self(9);
    pub const DISPLAY_PORT: Self = Self(10);
    pub const HDMI_A: Self = Self(11);
    pub const HDMI_B: Self = Self(12);
    pub const TV: Self = Self(13);
    pub const E_DP: Self = Self(14);
    pub const VIRTUAL: Self = Self(15);
    pub const DSI: Self = Self(16);
}
