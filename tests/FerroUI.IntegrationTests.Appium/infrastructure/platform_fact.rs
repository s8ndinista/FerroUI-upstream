//! Port of `PlatformFactAttribute.cs` and `PlatformTheoryAttribute.cs`: the
//! platforms a test runs on, and the reason a test is skipped.

/// `TestPlatforms`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TestPlatforms(u8);

impl TestPlatforms {
    pub const WINDOWS: TestPlatforms = TestPlatforms(0x01);
    pub const MAC_OS: TestPlatforms = TestPlatforms(0x02);
    pub const LINUX: TestPlatforms = TestPlatforms(0x04);
    pub const ALL: TestPlatforms = TestPlatforms(0x07);

    pub const fn or(self, other: TestPlatforms) -> TestPlatforms {
        TestPlatforms(self.0 | other.0)
    }

    pub fn has_flag(self, flag: TestPlatforms) -> bool {
        self.0 & flag.0 == flag.0
    }

    /// `IsSupported()`: whether the platform of the run is one of these.
    pub fn is_supported(self) -> bool {
        if cfg!(windows) {
            self.has_flag(TestPlatforms::WINDOWS)
        } else if cfg!(target_os = "macos") {
            self.has_flag(TestPlatforms::MAC_OS)
        } else if cfg!(target_os = "linux") {
            self.has_flag(TestPlatforms::LINUX)
        } else {
            false
        }
    }
}

/// The description of the system of the run (`RuntimeInformation.OSDescription`).
pub fn os_description() -> &'static str {
    std::env::consts::OS
}
