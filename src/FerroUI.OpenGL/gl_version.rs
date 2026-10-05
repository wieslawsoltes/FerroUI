/// The flavour of the OpenGL API a context implements.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum GlProfileType {
    OpenGL,
    OpenGLES,
}

/// The version of the OpenGL API of a context.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct GlVersion {
    type_: GlProfileType,
    major: i32,
    minor: i32,
    is_compatibility_profile: bool,
}

impl GlVersion {
    /// A version of the core profile.
    pub const fn new(type_: GlProfileType, major: i32, minor: i32) -> Self {
        Self::with_compatibility_profile(type_, major, minor, false)
    }

    /// A version with an explicit choice of the compatibility profile.
    pub const fn with_compatibility_profile(
        type_: GlProfileType,
        major: i32,
        minor: i32,
        is_compatibility_profile: bool,
    ) -> Self {
        Self { type_, major, minor, is_compatibility_profile }
    }

    /// The flavour of the API.
    pub const fn type_(&self) -> GlProfileType {
        self.type_
    }

    /// The major version.
    pub const fn major(&self) -> i32 {
        self.major
    }

    /// The minor version.
    pub const fn minor(&self) -> i32 {
        self.minor
    }

    /// Whether the compatibility profile is requested. Only makes sense if
    /// the type is OpenGL and the version is at least 3.2.
    pub const fn is_compatibility_profile(&self) -> bool {
        self.is_compatibility_profile
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_short_constructor_selects_the_core_profile() {
        let version = GlVersion::new(GlProfileType::OpenGL, 4, 1);

        assert_eq!(GlProfileType::OpenGL, version.type_());
        assert_eq!(4, version.major());
        assert_eq!(1, version.minor());
        assert!(!version.is_compatibility_profile());
    }

    #[test]
    fn versions_compare_by_value() {
        assert_eq!(GlVersion::new(GlProfileType::OpenGLES, 3, 0), GlVersion::new(GlProfileType::OpenGLES, 3, 0));
        assert_ne!(
            GlVersion::new(GlProfileType::OpenGL, 3, 2),
            GlVersion::with_compatibility_profile(GlProfileType::OpenGL, 3, 2, true)
        );
        assert_ne!(GlVersion::new(GlProfileType::OpenGL, 3, 0), GlVersion::new(GlProfileType::OpenGLES, 3, 0));
    }
}
