//! The options of ANGLE.

use ferroui_opengl::{GlProfileType, GlVersion};

/// The Direct3D API ANGLE renders with.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PlatformApi {
    DirectX9,
    DirectX11,
}

/// The options of the rendering mode that draws with OpenGL ES through
/// ANGLE.
#[derive(Clone, Debug, PartialEq)]
pub struct AngleOptions {
    /// The versions of OpenGL ES a context is asked for, in order.
    pub gl_profiles: Vec<GlVersion>,

    /// The Direct3D APIs that may be used, in order; `None` is Direct3D 11
    /// alone.
    pub allowed_platform_apis: Option<Vec<PlatformApi>>,
}

impl AngleOptions {
    /// The profiles of OpenGL ES among [`gl_profiles`](Self::gl_profiles):
    /// what a display of ANGLE is asked for.
    pub fn open_gl_es_profiles(&self) -> Vec<GlVersion> {
        self.gl_profiles.iter().copied().filter(|x| x.type_() == GlProfileType::OpenGLES).collect()
    }
}

impl Default for AngleOptions {
    fn default() -> Self {
        Self {
            gl_profiles: vec![GlVersion::new(GlProfileType::OpenGLES, 3, 0), GlVersion::new(GlProfileType::OpenGLES, 2, 0)],
            allowed_platform_apis: None,
        }
    }
}

#[cfg(test)]
mod tests {
    // Not from upstream.
    use super::*;

    #[test]
    fn the_default_options_ask_for_open_gl_es_3_then_2_on_direct3d_11() {
        let options = AngleOptions::default();

        assert_eq!(
            vec![GlVersion::new(GlProfileType::OpenGLES, 3, 0), GlVersion::new(GlProfileType::OpenGLES, 2, 0)],
            options.gl_profiles
        );
        assert_eq!(None, options.allowed_platform_apis);
    }

    #[test]
    fn only_the_profiles_of_open_gl_es_are_asked_of_a_display() {
        let es3 = GlVersion::new(GlProfileType::OpenGLES, 3, 0);
        let options =
            AngleOptions { gl_profiles: vec![GlVersion::new(GlProfileType::OpenGL, 4, 0), es3], ..Default::default() };

        assert_eq!(vec![es3], options.open_gl_es_profiles());
    }
}
