//! The surface view the framework renders to.
//!
//! The reference derives from `SurfaceView` and implements the callbacks of
//! its holder. The view itself is a class of the Java layer
//! (`FerroSurfaceView`), which forwards the callbacks with what this class
//! caches of the surface; the class that derives from this one in the
//! reference (`TopLevelImpl.SurfaceViewImpl`) is the top-level here, which
//! calls these members first, as an override calls its base.

use crate::interop::java::{call_void, new_object, JavaClass, JavaObject, JavaValue};
use crate::interop::natives::FERRO_SURFACE_VIEW;
use crate::interop::ndk::NativeWindow;
use ferroui_base::logging::{LogArea, LogEventLevel, Logger};
use ferroui_base::platform::surfaces::IPlatformRenderSurface;
use ferroui_base::PixelSize;
use ferroui_controls::platform::{INativePlatformHandleSurface, IPlatformHandle};
use std::any::Any;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::atomic::{AtomicI32, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, PoisonError};

/// What the surface view publishes of itself to the thread that renders:
/// the native window, the size of the surface and the scaling.
///
/// The reference keeps the handle of the window in a field it exchanges
/// atomically and releases the window when the surface is destroyed. Here
/// the window is counted: whoever uses it (the framebuffer of a frame, the
/// window surface of EGL) holds a reference of its own while it does, so
/// the UI thread can drop its reference while a frame is in flight.
pub(crate) struct SurfaceShared {
    native_window: Mutex<Option<NativeWindow>>,
    width: AtomicI32,
    height: AtomicI32,
    scaling: AtomicU64,
}

impl SurfaceShared {
    fn new() -> Self {
        Self {
            native_window: Mutex::new(None),
            width: AtomicI32::new(1),
            height: AtomicI32::new(1),
            scaling: AtomicU64::new(1.0f64.to_bits()),
        }
    }

    /// A reference to the native window; `None` while there is no surface.
    pub fn native_window(&self) -> Option<NativeWindow> {
        self.native_window.lock().unwrap_or_else(PoisonError::into_inner).clone()
    }

    pub fn has_native_window(&self) -> bool {
        self.native_window.lock().unwrap_or_else(PoisonError::into_inner).is_some()
    }

    pub fn size(&self) -> PixelSize {
        PixelSize::new(self.width.load(Ordering::Relaxed), self.height.load(Ordering::Relaxed))
    }

    pub fn scaling(&self) -> f64 {
        f64::from_bits(self.scaling.load(Ordering::Relaxed))
    }

    fn set_size(&self, size: PixelSize) {
        self.width.store(size.width, Ordering::Relaxed);
        self.height.store(size.height, Ordering::Relaxed);
    }

    /// Replaces the window; the reference the surface view held is released.
    fn exchange_native_window(&self, window: Option<NativeWindow>) {
        let old = std::mem::replace(&mut *self.native_window.lock().unwrap_or_else(PoisonError::into_inner), window);
        drop(old);
    }
}

/// What a callback of the surface holder carries: the surface, the size of
/// its frame when the holder has one, and the density of the display.
pub(crate) struct SurfaceProperties {
    pub surface: Option<JavaObject>,
    pub frame: Option<PixelSize>,
    pub density: f32,
}

type Handlers = RefCell<Vec<Rc<dyn Fn()>>>;

pub(crate) struct InvalidationAwareSurfaceView {
    view: JavaObject,
    shared: Arc<SurfaceShared>,
    surface_window_created: Handlers,
    surface_window_destroyed: Handlers,
}

impl InvalidationAwareSurfaceView {
    /// Creates the surface view in `context`. `handle` is the number its
    /// callbacks name the owner of the view by.
    pub fn new(context: &JavaObject, handle: i64, place_on_top: bool) -> Rc<Self> {
        let view = new_object(
            &JavaClass::find(FERRO_SURFACE_VIEW),
            "(Landroid/content/Context;JZ)V",
            &[JavaValue::Object(Some(context)), JavaValue::Long(handle), JavaValue::Boolean(place_on_top)],
        )
        .to_global();

        Rc::new(Self {
            view,
            shared: Arc::new(SurfaceShared::new()),
            surface_window_created: RefCell::new(Vec::new()),
            surface_window_destroyed: RefCell::new(Vec::new()),
        })
    }

    /// The Java view.
    pub fn view(&self) -> &JavaObject {
        &self.view
    }

    pub fn shared(&self) -> &Arc<SurfaceShared> {
        &self.shared
    }

    pub fn size(&self) -> PixelSize {
        self.shared.size()
    }

    pub fn scaling(&self) -> f64 {
        self.shared.scaling()
    }

    /// Subscribes to the creation of the window of the surface.
    pub fn surface_window_created(&self, handler: Rc<dyn Fn()>) {
        self.surface_window_created.borrow_mut().push(handler);
    }

    /// Subscribes to the destruction of the window of the surface.
    pub fn surface_window_destroyed(&self, handler: Rc<dyn Fn()>) {
        self.surface_window_destroyed.borrow_mut().push(handler);
    }

    fn raise(handlers: &Handlers) {
        let snapshot = handlers.borrow().clone();
        for handler in snapshot {
            handler();
        }
    }

    pub fn dispose(&self) {
        call_void(&self.view, "dispose", "()V", &[]);

        self.release_native_window_handle();
        self.surface_window_created.borrow_mut().clear();
        self.surface_window_destroyed.borrow_mut().clear();
    }

    pub fn surface_changed(&self, properties: &SurfaceProperties, format: i32, width: i32, height: i32) {
        if let Some(logger) = Logger::try_get(LogEventLevel::Verbose, LogArea::ANDROID_PLATFORM) {
            logger.log(None, &format!("InvalidationAwareSurfaceView Changed. Format:{format} Size:{width} x {height}"));
        }
        self.cache_surface_properties(properties);
    }

    pub fn surface_created(&self, properties: &SurfaceProperties) {
        if let Some(logger) = Logger::try_get(LogEventLevel::Verbose, LogArea::ANDROID_PLATFORM) {
            logger.log(None, "InvalidationAwareSurfaceView Created");
        }
        self.cache_surface_properties(properties);
        Self::raise(&self.surface_window_created);
    }

    pub fn surface_destroyed(&self) {
        if let Some(logger) = Logger::try_get(LogEventLevel::Verbose, LogArea::ANDROID_PLATFORM) {
            logger.log(None, "InvalidationAwareSurfaceView Destroyed");
        }
        self.release_native_window_handle();
        self.shared.set_size(PixelSize::new(1, 1));
        Self::raise(&self.surface_window_destroyed);
    }

    pub fn surface_redraw_needed(&self) {
        if let Some(logger) = Logger::try_get(LogEventLevel::Verbose, LogArea::ANDROID_PLATFORM) {
            logger.log(None, "InvalidationAwareSurfaceView RedrawNeeded");
        }
    }

    pub fn surface_redraw_needed_async(&self) {
        if let Some(logger) = Logger::try_get(LogEventLevel::Verbose, LogArea::ANDROID_PLATFORM) {
            logger.log(None, "InvalidationAwareSurfaceView RedrawNeededAsync");
        }
    }

    fn cache_surface_properties(&self, properties: &SurfaceProperties) {
        let new_handle = properties.surface.as_ref().and_then(NativeWindow::from_surface);
        self.shared.exchange_native_window(new_handle);

        self.shared.set_size(properties.frame.unwrap_or(PixelSize::new(1, 1)));
        self.shared.scaling.store(f64::from(properties.density).to_bits(), Ordering::Relaxed);
    }

    fn release_native_window_handle(&self) {
        self.shared.exchange_native_window(None);
    }
}

/// The surface view as a surface of the top-level: the handle of its
/// native window, for a renderer that takes one.
pub(crate) struct SurfaceViewHandle {
    shared: Arc<SurfaceShared>,
}

impl SurfaceViewHandle {
    pub fn new(shared: Arc<SurfaceShared>) -> Self {
        Self { shared }
    }
}

impl IPlatformHandle for SurfaceViewHandle {
    /// The address of the native window, zero without one. The address is
    /// only good while the surface exists.
    fn handle(&self) -> isize {
        self.shared.native_window().map_or(0, |window| window.handle())
    }

    fn handle_descriptor(&self) -> Option<&str> {
        Some("SurfaceView")
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl IPlatformRenderSurface for SurfaceViewHandle {
    fn is_ready(&self) -> bool {
        self.shared.has_native_window()
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl INativePlatformHandleSurface for SurfaceViewHandle {
    fn size(&self) -> PixelSize {
        self.shared.size()
    }

    fn scaling(&self) -> f64 {
        self.shared.scaling()
    }
}
