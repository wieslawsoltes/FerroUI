use crate::GlVersion;

/// Options for creating an OpenGL context that draws into the surfaces of a compositor.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct CompositionGlContextOptions {
    gl_profiles: Option<Vec<GlVersion>>,
}

impl CompositionGlContextOptions {
    pub fn new() -> Self {
        Self::default()
    }

    /// The preferred OpenGL profiles and versions for the created context, in the order of
    /// preference. When none, the defaults of the rendering backend are used.
    pub fn gl_profiles(&self) -> Option<&[GlVersion]> {
        self.gl_profiles.as_deref()
    }

    pub fn set_gl_profiles(&mut self, value: Option<Vec<GlVersion>>) {
        self.gl_profiles = value;
    }
}
