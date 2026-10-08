//! A feature of the render interface, as a caller holds it.
//!
//! Upstream `Compositor.TryGetRenderInterfaceFeature` hands the feature
//! object itself to the UI thread, which keeps it while the render thread
//! keeps the map it came from; the garbage collector shares the object. Here
//! a feature is an `Rc` in the map of the server compositor, and a second
//! handle to it held by another thread would change the same count without
//! synchronisation. So the caller gets a [`RenderInterfaceFeature`]: the
//! handle stays inside the compositor lock (a [`LockBound`]), and the feature
//! is lent from there for the length of a call.

use super::server::{LockBound, ServerCompositor};
use std::any::Any;
use std::rc::Rc;

/// A feature of the platform render interface
/// ([`Compositor::try_get_render_interface_feature`](super::Compositor::try_get_render_interface_feature)).
///
/// The handle keeps the feature alive, as the object upstream does: a caller
/// that holds it still reaches the feature of a context that has since been
/// replaced. It may be kept, moved and dropped anywhere; the feature behind
/// it is only reached inside the compositor lock, with
/// [`with`](Self::with) from outside or with [`get`](Self::get) by a caller
/// that already has the server compositor in hand (a job).
///
/// A feature is registered under the type it is asked for by. By the
/// convention of the optional features a feature asked for as a trait object
/// (`TypeId::of::<dyn IFoo>()`) is registered as an `Rc<dyn IFoo>`: `with`
/// and `get` lend the `dyn IFoo`, never the `Rc`.
pub struct RenderInterfaceFeature {
    feature: LockBound<Rc<dyn Any>>,
}

impl RenderInterfaceFeature {
    /// `feature` is a handle cloned from the map of features inside the
    /// lock, bound to the lock there.
    pub(crate) fn new(feature: LockBound<Rc<dyn Any>>) -> Self {
        Self { feature }
    }

    fn lend<T: ?Sized + 'static>(feature: &Rc<dyn Any>) -> Option<&T> {
        feature.downcast_ref::<Rc<T>>().map(|feature| &**feature)
    }

    /// Runs `f` with the feature, registered as an `Rc<T>`, inside the
    /// compositor lock, which the calling thread enters for the call.
    ///
    /// `None` when the feature is not registered as an `Rc<T>`, and once the
    /// compositor has released its server compositor.
    pub fn with<T: ?Sized + 'static, R>(&self, f: impl FnOnce(&T) -> R) -> Option<R> {
        self.feature.with(|_, feature| Self::lend::<T>(feature).map(f)).flatten()
    }

    /// The feature, registered as an `Rc<T>`, for a caller that is inside
    /// the compositor lock: `server` is what a job receives and what
    /// `Compositor::with_server` hands out. `None` when the feature is not
    /// registered as an `Rc<T>`.
    ///
    /// # Panics
    ///
    /// When `server` is not the server compositor the feature came from.
    pub fn get<'a, T: ?Sized + 'static>(&'a self, server: &'a ServerCompositor) -> Option<&'a T> {
        Self::lend::<T>(self.feature.get(server))
    }

    /// Whether the feature is registered as a value of type `T` (for a
    /// feature asked for as a trait object, `Rc<dyn IFoo>`). `false` once
    /// the compositor has released its server compositor.
    pub fn is<T: 'static>(&self) -> bool {
        self.feature.with(|_, feature| feature.is::<T>()).unwrap_or(false)
    }
}

const _: fn() = || {
    // The handle is what crosses out of the lock, so it may be held by any
    // thread.
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<RenderInterfaceFeature>();
};
