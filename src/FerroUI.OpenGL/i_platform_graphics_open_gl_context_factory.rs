use crate::{GlVersion, IGlContext};
use std::rc::Rc;

/// Creates OpenGL contexts for a platform.
pub trait IPlatformGraphicsOpenGlContextFactory {
    /// Creates a context of the first of `versions` the platform supports,
    /// or of the platform's default version when `versions` is `None`.
    fn create_context(&self, versions: Option<&[GlVersion]>) -> Rc<dyn IGlContext>;
}
