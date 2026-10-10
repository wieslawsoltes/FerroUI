//! The platform graphics of the Wayland backend (the port of
//! `WaylandPlatformGraphics.cs`): a holder the compositor of the framework
//! is created with, whose backend the worker binds once the globals of a
//! connection are known and takes away when the connection is lost.

use ferroui_base::platform::{
    IOptionalFeatureProvider, IPlatformGraphics, IPlatformGraphicsContext, IPlatformGraphicsReadyStateFeature,
};
use ferroui_base::utilities::ThreadBound;
use std::any::{Any, TypeId};
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, PoisonError, Weak};

/// A GPU backend of a connection: an object of the worker thread.
///
/// The reference also asks it for the render surface of a `WSurface`
/// (`CreateRenderSurface`). A render surface of the port is a handle the UI
/// thread makes without knowing the backend (the number of the worker's
/// surface), and the render target it creates on the worker is where the
/// backend is asked.
pub trait IWaylandGraphics {
    fn create_context(&self) -> Rc<dyn IPlatformGraphicsContext>;

    /// Releases the backend.
    fn dispose(&self);

    /// Lets the render surfaces recover the concrete backend.
    fn as_any(&self) -> &dyn Any;
}

/// The platform graphics the compositor of the framework holds.
///
/// Shared between the UI thread, which creates it, and the worker, which
/// binds the backend and renders: the backend is kept for the worker
/// thread, and the two states the render interface asks about at every
/// frame are atomics.
pub struct WaylandPlatformGraphics {
    this: Weak<WaylandPlatformGraphics>,
    inner: Mutex<Option<ThreadBound<Rc<dyn IWaylandGraphics>>>>,
    is_ready: AtomicBool,
    uses_contexts: AtomicBool,
    generation: AtomicU64,
}

impl WaylandPlatformGraphics {
    pub fn new() -> Arc<Self> {
        Arc::new_cyclic(|this| Self {
            this: this.clone(),
            inner: Mutex::new(None),
            is_ready: AtomicBool::new(false),
            uses_contexts: AtomicBool::new(false),
            generation: AtomicU64::new(0),
        })
    }

    /// Currently-bound backend, or `None` if none (and on a thread other than the worker).
    pub fn current_graphics(&self) -> Option<Rc<dyn IWaylandGraphics>> {
        let inner = self.inner.lock().unwrap_or_else(PoisonError::into_inner);
        match &*inner {
            Some(graphics) if graphics.is_on_thread() => Some(graphics.get().clone()),
            _ => None,
        }
    }

    /// A number that changes with every backend that is bound or taken away: the cache key
    /// for callers that need to invalidate per-backend state across reconnects (the
    /// reference compares the identity of the backend).
    pub fn generation(&self) -> u64 {
        self.generation.load(Ordering::Acquire)
    }

    pub fn is_ready(&self) -> bool {
        self.is_ready.load(Ordering::Acquire)
    }

    pub fn uses_contexts(&self) -> bool {
        self.uses_contexts.load(Ordering::Acquire)
    }

    /// Takes the backend away: the connection is lost. On the worker thread.
    pub fn reset(&self) {
        self.is_ready.store(false, Ordering::Release);
        let previous = self.replace(None);
        if let Some(previous) = previous {
            previous.dispose();
        }
    }

    /// Binds the backend of a connection, or none (software rendering). On the worker thread.
    pub fn initialize(&self, graphics: Option<Rc<dyn IWaylandGraphics>>) {
        let previous = self.replace(graphics);
        if let Some(previous) = previous {
            previous.dispose();
        }
        self.is_ready.store(true, Ordering::Release);
    }

    fn replace(&self, graphics: Option<Rc<dyn IWaylandGraphics>>) -> Option<Rc<dyn IWaylandGraphics>> {
        let mut inner = self.inner.lock().unwrap_or_else(PoisonError::into_inner);
        self.uses_contexts.store(graphics.is_some(), Ordering::Release);
        self.generation.fetch_add(1, Ordering::AcqRel);
        let previous = std::mem::replace(&mut *inner, graphics.map(ThreadBound::new));
        previous.filter(ThreadBound::is_on_thread).map(ThreadBound::into_inner)
    }
}

impl IPlatformGraphics for WaylandPlatformGraphics {
    fn uses_shared_context(&self) -> bool {
        false
    }

    /// # Panics
    /// Panics without a backend (the `InvalidOperationException` of the reference).
    fn create_context(&self) -> Rc<dyn IPlatformGraphicsContext> {
        match self.current_graphics() {
            Some(graphics) => graphics.create_context(),
            None => panic!("Operation is not valid due to the current state of the object."),
        }
    }

    /// # Panics
    /// Always: the graphics have no shared context (the `NotSupportedException` of the
    /// reference).
    fn get_shared_context(&self) -> Rc<dyn IPlatformGraphicsContext> {
        panic!("Specified method is not supported.")
    }

    fn as_feature_provider(&self) -> Option<&dyn IOptionalFeatureProvider> {
        Some(self)
    }
}

impl IOptionalFeatureProvider for WaylandPlatformGraphics {
    fn try_get_feature(&self, feature_type: TypeId) -> Option<Rc<dyn Any>> {
        if feature_type == TypeId::of::<dyn IPlatformGraphicsReadyStateFeature>() {
            let feature: Arc<dyn IPlatformGraphicsReadyStateFeature> = self.this.upgrade()?;
            return Some(Rc::new(feature));
        }
        None
    }
}

impl IPlatformGraphicsReadyStateFeature for WaylandPlatformGraphics {
    fn is_ready(&self) -> bool {
        WaylandPlatformGraphics::is_ready(self)
    }

    fn uses_contexts(&self) -> bool {
        WaylandPlatformGraphics::uses_contexts(self)
    }
}

#[cfg(test)]
mod tests {
    // Not from the reference, which has no tests for this file.
    use super::*;
    use std::cell::Cell;

    struct Backend {
        disposed: Rc<Cell<u32>>,
    }

    impl IWaylandGraphics for Backend {
        fn create_context(&self) -> Rc<dyn IPlatformGraphicsContext> {
            panic!("not asked in this test")
        }

        fn dispose(&self) {
            self.disposed.set(self.disposed.get() + 1);
        }

        fn as_any(&self) -> &dyn Any {
            self
        }
    }

    #[test]
    fn the_holder_is_ready_once_initialized_and_uses_contexts_only_with_a_backend() {
        let graphics = WaylandPlatformGraphics::new();
        assert!(!graphics.is_ready());
        assert!(!graphics.uses_contexts());
        assert!(graphics.current_graphics().is_none());

        // Software rendering: ready, without contexts.
        graphics.initialize(None);
        assert!(graphics.is_ready());
        assert!(!graphics.uses_contexts());

        let disposed = Rc::new(Cell::new(0));
        let generation = graphics.generation();
        graphics.initialize(Some(Rc::new(Backend { disposed: disposed.clone() })));
        assert!(graphics.is_ready());
        assert!(graphics.uses_contexts());
        assert!(graphics.current_graphics().is_some());
        assert_ne!(graphics.generation(), generation);

        graphics.reset();
        assert!(!graphics.is_ready());
        assert!(!graphics.uses_contexts());
        assert!(graphics.current_graphics().is_none());
        assert_eq!(disposed.get(), 1);
    }

    #[test]
    fn the_ready_state_is_a_feature_of_the_graphics() {
        let graphics = WaylandPlatformGraphics::new();
        let features = graphics.as_feature_provider().expect("the graphics have features");
        let feature = features.try_get_shared::<dyn IPlatformGraphicsReadyStateFeature>().expect("the ready state");
        assert!(!feature.is_ready());
        graphics.initialize(None);
        assert!(feature.is_ready());
        assert!(!feature.uses_contexts());
    }

    #[test]
    fn another_thread_sees_the_state_and_not_the_backend() {
        let graphics = WaylandPlatformGraphics::new();
        graphics.initialize(Some(Rc::new(Backend { disposed: Rc::new(Cell::new(0)) })));
        let shared = graphics.clone();
        let (ready, has_backend) =
            std::thread::spawn(move || (shared.is_ready(), shared.current_graphics().is_some())).join().unwrap();
        assert!(ready);
        assert!(!has_backend);
    }
}
