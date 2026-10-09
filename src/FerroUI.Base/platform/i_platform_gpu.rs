use super::IOptionalFeatureProvider;
use crate::reactive::IDisposable;
use std::rc::Rc;

/// Entry point to a platform GPU API (Metal, OpenGL, Vulkan, ...).
///
/// The object is shared by threads, as in the reference, where the platform
/// registers one object that the UI thread finds among its services and the
/// thread that renders asks for its context. It is therefore held in an
/// `Arc` and is `Send + Sync`: the platform creates it on its thread, the
/// compositor hands a handle to the server side, and either side may clone,
/// call and drop its handle without the other. An implementation holds only
/// what two threads may share; what belongs to one thread (a display or a
/// context that is an `Rc`) it keeps bound to that thread.
///
/// A context it creates is an object of the thread that asked for it.
pub trait IPlatformGraphics: Send + Sync {
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
///
/// Asked by the thread that renders, at every frame, about a state the
/// thread of the platform changes: shared by the two as the platform
/// graphics are (`Send + Sync`). The platform graphics announce the feature
/// through their optional features as an `Arc<dyn IPlatformGraphicsReadyStateFeature>`
/// (`try_get_shared` of the optional features).
pub trait IPlatformGraphicsReadyStateFeature: Send + Sync {
    fn is_ready(&self) -> bool;
    fn uses_contexts(&self) -> bool;
}

const _: fn() = || {
    // The two contracts the thread of the platform and the thread that
    // renders hold a handle to.
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<std::sync::Arc<dyn IPlatformGraphics>>();
    assert_send_sync::<std::sync::Arc<dyn IPlatformGraphicsReadyStateFeature>>();
};

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

/// The graphics context a GPU object belongs to was lost.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PlatformGraphicsContextLostException;

impl std::fmt::Display for PlatformGraphicsContextLostException {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("The platform graphics context was lost.")
    }
}

impl std::error::Error for PlatformGraphicsContextLostException {}
