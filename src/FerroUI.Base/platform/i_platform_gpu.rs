use super::IOptionalFeatureProvider;
use crate::reactive::IDisposable;
use std::rc::Rc;

/// Entry point to a platform GPU API (Metal, OpenGL, Vulkan, ...).
pub trait IPlatformGraphics {
    /// Whether the backend renders through one context shared by all
    /// surfaces.
    fn uses_shared_context(&self) -> bool;

    /// Creates a new graphics context.
    fn create_context(&self) -> Rc<dyn IPlatformGraphicsContext>;

    /// The shared graphics context; only valid when
    /// [`uses_shared_context`](Self::uses_shared_context) is true.
    fn get_shared_context(&self) -> Rc<dyn IPlatformGraphicsContext>;

    /// Optional features of the graphics API, if the implementation provides
    /// any.
    fn as_feature_provider(&self) -> Option<&dyn IOptionalFeatureProvider> {
        None
    }
}

/// Reports whether the platform graphics are ready to be used.
pub trait IPlatformGraphicsReadyStateFeature {
    fn is_ready(&self) -> bool;
    fn uses_contexts(&self) -> bool;
}

/// A context of a platform GPU API.
pub trait IPlatformGraphicsContext: IOptionalFeatureProvider {
    /// Whether the context is lost and cannot be used anymore.
    fn is_lost(&self) -> bool;

    /// Makes the context current. Disposing the result restores the
    /// previously current context.
    fn ensure_current(&self) -> Rc<dyn IDisposable>;

    /// Releases the context.
    fn dispose(&self);

    /// Lets the backend recover the concrete context type.
    fn as_any(&self) -> &dyn std::any::Any;
}
