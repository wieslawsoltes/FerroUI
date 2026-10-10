//! Constants of the platform: handle descriptors and the versions of
//! Windows the backend tells apart.

/// A version of Windows: major, minor and build number, compared in that
/// order.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Version {
    /// The major version.
    pub major: u32,
    /// The minor version.
    pub minor: u32,
    /// The build number.
    pub build: u32,
}

impl Version {
    /// Creates a version from its major and minor numbers.
    pub const fn new(major: u32, minor: u32) -> Version {
        Version { major, minor, build: 0 }
    }

    /// Creates a version from its major, minor and build numbers.
    pub const fn with_build(major: u32, minor: u32, build: u32) -> Version {
        Version { major, minor, build }
    }
}

impl std::fmt::Display for Version {
    /// The numbers with points between them; the build number when the
    /// version has one.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}", self.major, self.minor)?;
        if self.build != 0 {
            write!(f, ".{}", self.build)?;
        }
        Ok(())
    }
}

/// The constants of the platform.
pub struct PlatformConstants;

impl PlatformConstants {
    /// The handle descriptor of a window handle.
    pub const WINDOW_HANDLE_TYPE: &'static str = "HWND";
    /// The handle descriptor of a cursor handle.
    pub const CURSOR_HANDLE_TYPE: &'static str = "HCURSOR";

    /// Windows 10.
    pub const WINDOWS10: Version = Version::new(10, 0);
    /// Windows 10 Anniversary Update.
    pub const WINDOWS10_1607: Version = Version::with_build(10, 0, 14393);
    /// Windows 8.
    pub const WINDOWS8: Version = Version::new(6, 2);
    /// Windows 8.1.
    pub const WINDOWS8_1: Version = Version::new(6, 3);
    /// Windows 7.
    pub const WINDOWS7: Version = Version::new(6, 1);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn versions_compare_by_major_minor_and_build() {
        assert!(PlatformConstants::WINDOWS8 < PlatformConstants::WINDOWS8_1);
        assert!(PlatformConstants::WINDOWS8_1 < PlatformConstants::WINDOWS10);
        assert!(PlatformConstants::WINDOWS10 < PlatformConstants::WINDOWS10_1607);
        assert!(Version::with_build(10, 0, 22000) >= PlatformConstants::WINDOWS10_1607);
        assert!(Version::with_build(6, 3, 9600) < PlatformConstants::WINDOWS10);
        assert_eq!(PlatformConstants::WINDOW_HANDLE_TYPE, "HWND");
        assert_eq!(PlatformConstants::CURSOR_HANDLE_TYPE, "HCURSOR");
    }
}
