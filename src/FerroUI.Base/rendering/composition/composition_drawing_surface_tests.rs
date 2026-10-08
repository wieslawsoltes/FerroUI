//! Port of `CompositionDrawingSurfaceTests.cs`.

use super::test_compositor::TestCompositor;
use super::*;
use crate::media::imaging::BitmapEncoderOptions;
use crate::platform::{
    IBitmapImpl, IExternalObjectsRenderInterfaceContextFeature, IPlatformHandle,
    IPlatformRenderInterfaceImportedImage, IPlatformRenderInterfaceImportedObject,
    IPlatformRenderInterfaceImportedSemaphore, PlatformGraphicsExternalImageProperties, PlatformHandle,
};
use crate::{PixelSize, Vector};
use std::any::Any;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

#[derive(Default)]
struct TrackingBitmapImpl {
    is_disposed: std::sync::atomic::AtomicBool,
}

impl IBitmapImpl for TrackingBitmapImpl {
    fn dpi(&self) -> Vector {
        Vector::new(96.0, 96.0)
    }

    fn pixel_size(&self) -> PixelSize {
        PixelSize::new(1, 1)
    }

    fn version(&self) -> i32 {
        1
    }

    fn save(&self, _stream: &mut dyn std::io::Write, _options: &BitmapEncoderOptions) -> std::io::Result<()> {
        unreachable!("not used by these tests")
    }

    fn dispose(&self) {
        self.is_disposed.store(true, std::sync::atomic::Ordering::SeqCst)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

#[derive(Default)]
struct FakeImportedImage {
    last_snapshot: RefCell<Option<std::sync::Arc<TrackingBitmapImpl>>>,
}

impl FakeImportedImage {
    fn snapshot(&self) -> std::sync::Arc<crate::platform::SharedBitmapImpl> {
        let snapshot = std::sync::Arc::new(TrackingBitmapImpl::default());
        *self.last_snapshot.borrow_mut() = Some(snapshot.clone());
        snapshot
    }
}

impl IPlatformRenderInterfaceImportedObject for FakeImportedImage {
    fn dispose(&self) {}
}

impl IPlatformRenderInterfaceImportedImage for FakeImportedImage {
    fn snapshot_with_keyed_mutex(&self, _acquire_index: u32, _release_index: u32) -> std::sync::Arc<crate::platform::SharedBitmapImpl> {
        self.snapshot()
    }

    fn snapshot_with_semaphores(
        &self,
        _wait_for_semaphore: &Rc<dyn IPlatformRenderInterfaceImportedSemaphore>,
        _signal_semaphore: &Rc<dyn IPlatformRenderInterfaceImportedSemaphore>,
    ) -> std::sync::Arc<crate::platform::SharedBitmapImpl> {
        self.snapshot()
    }

    fn snapshot_with_timeline_semaphores(
        &self,
        _wait_for_semaphore: &Rc<dyn IPlatformRenderInterfaceImportedSemaphore>,
        _wait_for_value: u64,
        _signal_semaphore: &Rc<dyn IPlatformRenderInterfaceImportedSemaphore>,
        _signal_value: u64,
    ) -> std::sync::Arc<crate::platform::SharedBitmapImpl> {
        self.snapshot()
    }

    fn snapshot_with_automatic_sync(&self) -> std::sync::Arc<crate::platform::SharedBitmapImpl> {
        self.snapshot()
    }
}

#[derive(Default)]
struct FakeExternalObjectsFeature {
    image: Rc<FakeImportedImage>,
}

impl IExternalObjectsRenderInterfaceContextFeature for FakeExternalObjectsFeature {
    fn supported_image_handle_types(&self) -> Vec<String> {
        vec!["Fake".to_owned()]
    }

    fn supported_semaphore_types(&self) -> Vec<String> {
        Vec::new()
    }

    fn import_image(
        &self,
        _handle: Rc<dyn IPlatformHandle>,
        _properties: PlatformGraphicsExternalImageProperties,
    ) -> Rc<dyn IPlatformRenderInterfaceImportedImage> {
        self.image.clone()
    }

    fn import_shared_image(
        &self,
        _image: std::sync::Arc<dyn ICompositionImportableSharedGpuContextImage>,
    ) -> Rc<dyn IPlatformRenderInterfaceImportedImage> {
        self.image.clone()
    }

    fn import_semaphore(&self, _handle: Rc<dyn IPlatformHandle>) -> Rc<dyn IPlatformRenderInterfaceImportedSemaphore> {
        panic!("Specified method is not supported.")
    }

    fn get_synchronization_capabilities(&self, _image_handle_type: &str) -> CompositionGpuImportedImageSynchronizationCapabilities {
        CompositionGpuImportedImageSynchronizationCapabilities::AUTOMATIC
    }

    fn device_uuid(&self) -> Option<Vec<u8>> {
        None
    }

    fn device_luid(&self) -> Option<Vec<u8>> {
        None
    }
}

fn import(services: &TestCompositor, feature: &Rc<FakeExternalObjectsFeature>) -> Rc<dyn ICompositionImportedGpuImage> {
    let interop = CompositionInterop::new(&services.compositor, feature.clone());
    interop.import_image(
        Rc::new(PlatformHandle::new(1, Some("Fake"))),
        PlatformGraphicsExternalImageProperties { width: 1, height: 1, ..Default::default() },
    )
}

#[test]
fn update_processed_after_dispose_should_dispose_snapshot_instead_of_orphaning_it() {
    // A commit batch is processed on the render thread in serialization order:
    // the dispose list is written (and therefore processed) BEFORE queued server
    // jobs. This means the perfectly legal user-code order
    //     surface.update_async(image); surface.dispose();
    // executes on the render thread as dispose() -> update_with_automatic_sync().
    // Without a disposed-guard, the update stores a fresh snapshot into the
    // already-disposed surface, and nothing ever disposes it again.
    let services = TestCompositor::new();
    let compositor = &services.compositor;

    let feature = Rc::new(FakeExternalObjectsFeature::default());
    let imported = import(&services, &feature);

    services.run_jobs();
    assert!(imported.import_completed().is_completed_successfully());

    let surface = compositor.create_drawing_surface();

    let update = surface.update_async(&*imported);
    surface.dispose();

    services.run_jobs();
    assert!(update.is_completed_successfully(), "{:?}", update.exception().map(|e| e.to_string()));

    let snapshot = feature.image.last_snapshot.borrow().clone();
    let snapshot = snapshot.expect("a snapshot was taken");
    assert!(
        snapshot.is_disposed.load(std::sync::atomic::Ordering::SeqCst),
        "The snapshot taken by an update processed after the surface was disposed \
         must be disposed on the render thread instead of being orphaned."
    );
}

#[test]
fn update_before_dispose_in_separate_batches_should_dispose_snapshot_with_the_surface() {
    // Baseline: when the update is processed in an earlier batch than the dispose,
    // the surface's dispose() releases the stored snapshot. This already works and
    // must keep working with the disposed-guard in place.
    let services = TestCompositor::new();
    let compositor = &services.compositor;

    let feature = Rc::new(FakeExternalObjectsFeature::default());
    let imported = import(&services, &feature);

    services.run_jobs();

    let surface = compositor.create_drawing_surface();

    let update = surface.update_async(&*imported);
    services.run_jobs();
    assert!(update.is_completed_successfully(), "{:?}", update.exception().map(|e| e.to_string()));

    let snapshot = feature.image.last_snapshot.borrow().clone();
    let snapshot = snapshot.expect("a snapshot was taken");
    assert!(!snapshot.is_disposed.load(std::sync::atomic::Ordering::SeqCst));

    surface.dispose();
    services.run_jobs();

    assert!(snapshot.is_disposed.load(std::sync::atomic::Ordering::SeqCst));
}

/// Not from upstream: an update queued after the dispose of the surface has
/// been processed by an earlier batch still reaches the disposed server
/// surface, which disposes the snapshot; the id of the surface is not
/// handed to another object meanwhile.
#[test]
fn update_after_dispose_in_an_earlier_batch_should_dispose_snapshot() {
    let services = TestCompositor::new();
    let compositor = &services.compositor;

    let feature = Rc::new(FakeExternalObjectsFeature::default());
    let imported = import(&services, &feature);

    services.run_jobs();

    let surface = compositor.create_drawing_surface();
    services.run_jobs();

    surface.dispose();
    services.run_jobs();

    // An object created after the dispose must not get the id of the
    // surface while the surface is alive.
    let other = compositor.create_drawing_surface();
    assert_ne!(ICompositionObject::server(&**other), ICompositionObject::server(&**surface));

    let update = surface.update_async(&*imported);
    services.run_jobs();
    assert!(update.is_completed_successfully(), "{:?}", update.exception().map(|e| e.to_string()));

    let snapshot = feature.image.last_snapshot.borrow().clone();
    let snapshot = snapshot.expect("a snapshot was taken");
    assert!(snapshot.is_disposed.load(std::sync::atomic::Ordering::SeqCst));
    assert!(services.server::<server::ServerCompositionDrawingSurface>(ICompositionObject::server(&**other)).bitmap_ref().is_none());
}

/// Not from upstream: dropping a disposed surface releases its server
/// object, and the id can then be reused.
#[test]
fn dropping_a_disposed_surface_releases_its_server_object() {
    let services = TestCompositor::new();
    let compositor = &services.compositor;
    services.run_jobs();
    let baseline = compositor.server().object_count();

    let surface = compositor.create_drawing_surface();
    services.run_jobs();
    assert_eq!(baseline + 1, compositor.server().object_count());

    surface.dispose();
    services.run_jobs();
    // The disposed server object is kept while the surface is alive.
    assert_eq!(baseline + 1, compositor.server().object_count());

    drop(surface);
    // The release rides on a later batch.
    let _other = compositor.create_drawing_surface();
    services.run_jobs();
    assert_eq!(baseline + 1, compositor.server().object_count());
}

// The render-thread mode. Not from upstream, where the two threads are a
// property of the platform set-up and not of a test.
//
// The fakes below are objects of the server side: a test hands them to the
// interop whole and they stay inside the compositor lock. What they do is
// read from a log the two threads share.

use crate::media::MediaContext;
use crate::rendering::testing::{ManualRenderLoop, MockPlatformRenderInterface};
use crate::threading::Dispatcher;
use std::sync::{Arc, Mutex};
use std::thread::{self, ThreadId};

#[derive(Default)]
struct InteropLog {
    /// The thread and the handle of every import.
    imports: Mutex<Vec<(ThreadId, isize, Option<String>)>>,
    shared_imports: Mutex<Vec<ThreadId>>,
    snapshots: Mutex<Vec<ThreadId>>,
    disposals: Mutex<Vec<ThreadId>>,
    last_snapshot: Mutex<Option<Arc<TrackingBitmapImpl>>>,
}

struct LoggingImportedImage {
    log: Arc<InteropLog>,
}

impl LoggingImportedImage {
    fn snapshot(&self) -> Arc<crate::platform::SharedBitmapImpl> {
        let snapshot = Arc::new(TrackingBitmapImpl::default());
        self.log.snapshots.lock().unwrap().push(thread::current().id());
        *self.log.last_snapshot.lock().unwrap() = Some(snapshot.clone());
        snapshot
    }
}

impl IPlatformRenderInterfaceImportedObject for LoggingImportedImage {
    fn dispose(&self) {
        self.log.disposals.lock().unwrap().push(thread::current().id());
    }
}

impl IPlatformRenderInterfaceImportedImage for LoggingImportedImage {
    fn snapshot_with_keyed_mutex(&self, _acquire_index: u32, _release_index: u32) -> Arc<crate::platform::SharedBitmapImpl> {
        self.snapshot()
    }

    fn snapshot_with_semaphores(
        &self,
        _wait_for_semaphore: &Rc<dyn IPlatformRenderInterfaceImportedSemaphore>,
        _signal_semaphore: &Rc<dyn IPlatformRenderInterfaceImportedSemaphore>,
    ) -> Arc<crate::platform::SharedBitmapImpl> {
        self.snapshot()
    }

    fn snapshot_with_timeline_semaphores(
        &self,
        _wait_for_semaphore: &Rc<dyn IPlatformRenderInterfaceImportedSemaphore>,
        _wait_for_value: u64,
        _signal_semaphore: &Rc<dyn IPlatformRenderInterfaceImportedSemaphore>,
        _signal_value: u64,
    ) -> Arc<crate::platform::SharedBitmapImpl> {
        self.snapshot()
    }

    fn snapshot_with_automatic_sync(&self) -> Arc<crate::platform::SharedBitmapImpl> {
        self.snapshot()
    }
}

struct LoggingExternalObjectsFeature {
    log: Arc<InteropLog>,
}

impl IExternalObjectsRenderInterfaceContextFeature for LoggingExternalObjectsFeature {
    fn supported_image_handle_types(&self) -> Vec<String> {
        vec!["Fake".to_owned()]
    }

    fn supported_semaphore_types(&self) -> Vec<String> {
        Vec::new()
    }

    fn import_image(
        &self,
        handle: Rc<dyn IPlatformHandle>,
        _properties: PlatformGraphicsExternalImageProperties,
    ) -> Rc<dyn IPlatformRenderInterfaceImportedImage> {
        self.log.imports.lock().unwrap().push((
            thread::current().id(),
            handle.handle(),
            handle.handle_descriptor().map(str::to_owned),
        ));
        Rc::new(LoggingImportedImage { log: self.log.clone() })
    }

    fn import_shared_image(
        &self,
        _image: Arc<dyn ICompositionImportableSharedGpuContextImage>,
    ) -> Rc<dyn IPlatformRenderInterfaceImportedImage> {
        self.log.shared_imports.lock().unwrap().push(thread::current().id());
        Rc::new(LoggingImportedImage { log: self.log.clone() })
    }

    fn import_semaphore(&self, _handle: Rc<dyn IPlatformHandle>) -> Rc<dyn IPlatformRenderInterfaceImportedSemaphore> {
        panic!("Specified method is not supported.")
    }

    fn get_synchronization_capabilities(&self, _image_handle_type: &str) -> CompositionGpuImportedImageSynchronizationCapabilities {
        CompositionGpuImportedImageSynchronizationCapabilities::AUTOMATIC
    }

    fn device_uuid(&self) -> Option<Vec<u8>> {
        None
    }

    fn device_luid(&self) -> Option<Vec<u8>> {
        None
    }
}

/// An image the two threads share, in the shape of a texture of a shared
/// context: the identifier is a value either thread reads and the disposal
/// clears; the part that stands for the context of the caller is bound to
/// the thread that made the image.
struct FakeSharedImage {
    texture_id: std::sync::atomic::AtomicI32,
    context: crate::utilities::ThreadBound<Rc<Cell<bool>>>,
}

impl FakeSharedImage {
    fn new(texture_id: i32) -> Arc<FakeSharedImage> {
        Arc::new(FakeSharedImage {
            texture_id: std::sync::atomic::AtomicI32::new(texture_id),
            context: crate::utilities::ThreadBound::new(Rc::new(Cell::new(false))),
        })
    }

    fn texture_id(&self) -> i32 {
        self.texture_id.load(std::sync::atomic::Ordering::SeqCst)
    }
}

impl ICompositionImportableSharedGpuContextImage for FakeSharedImage {
    fn dispose(&self) {
        self.texture_id.store(0, std::sync::atomic::Ordering::SeqCst);
        // The context is used only where it lives.
        if self.context.is_on_thread() {
            self.context.get().set(true);
        }
    }
}

/// A compositor in the render-thread mode, with the scopes it lives in.
struct RenderThreadServices {
    // Declared first: dropped before the scopes below.
    compositor: Rc<Compositor>,
    render_loop: Arc<ManualRenderLoop>,
    _locator_scope: Rc<dyn crate::reactive::IDisposable>,
    _dispatcher_scope: crate::threading::UnitTestDispatcherScope,
}

impl RenderThreadServices {
    fn new() -> RenderThreadServices {
        let dispatcher_scope = Dispatcher::unit_test_scope();
        let (locator_scope, _) = MockPlatformRenderInterface::install();
        let render_loop = ManualRenderLoop::new();
        let compositor = Compositor::with_render_thread(
            render_loop.clone(),
            None,
            false,
            &MediaContext::instance().scheduler(),
            Dispatcher::ui_thread(),
            None,
            None,
        );
        RenderThreadServices { compositor, render_loop, _locator_scope: locator_scope, _dispatcher_scope: dispatcher_scope }
    }

    /// Commits the pending changes on this thread and renders a frame on
    /// another one, which is returned.
    fn run_jobs(&self) -> ThreadId {
        Dispatcher::ui_thread().run_jobs(None);
        let render_loop = self.render_loop.clone();
        let render_thread = thread::spawn(move || {
            render_loop.tick();
            thread::current().id()
        })
        .join()
        .expect("the render thread ran");
        assert_ne!(thread::current().id(), render_thread);
        Dispatcher::ui_thread().run_jobs(None);
        render_thread
    }

    fn interop(&self, log: &Arc<InteropLog>) -> Rc<CompositionInterop> {
        // The feature is handed over whole: no clone stays on this thread.
        CompositionInterop::new(&self.compositor, Rc::new(LoggingExternalObjectsFeature { log: log.clone() }))
    }
}

#[test]
fn an_image_is_imported_and_disposed_on_the_render_thread() {
    let services = RenderThreadServices::new();
    assert!(services.compositor.renders_on_render_thread());
    let log = Arc::new(InteropLog::default());
    let interop = services.interop(&log);

    // What this thread reads from the feature is read inside the lock.
    assert_eq!(vec!["Fake".to_owned()], interop.supported_image_handle_types());
    assert!(!interop.is_lost());

    let handle = Rc::new(PlatformHandle::new(7, Some("Fake")));
    let imported = interop.import_image(
        handle.clone(),
        PlatformGraphicsExternalImageProperties { width: 1, height: 1, ..Default::default() },
    );
    assert!(!imported.import_completed().is_completed());
    assert!(log.imports.lock().unwrap().is_empty());

    let render_thread = services.run_jobs();
    assert!(
        imported.import_completed().is_completed_successfully(),
        "{:?}",
        imported.import_completed().exception().map(|e| e.to_string())
    );
    // The import ran on the render thread, with the contents of the handle;
    // the handle of the caller never left this thread.
    assert_eq!(vec![(render_thread, 7, Some("Fake".to_owned()))], *log.imports.lock().unwrap());
    assert_eq!(1, Rc::strong_count(&handle));
    assert!(!imported.is_lost());

    // The job of the disposal holds the server part: the object of the
    // caller and the interop may go first.
    let disposed = imported.dispose_async();
    drop(imported);
    drop(interop);
    assert!(log.disposals.lock().unwrap().is_empty());

    let render_thread = services.run_jobs();
    assert!(disposed.is_completed_successfully());
    assert_eq!(vec![render_thread], *log.disposals.lock().unwrap());
}

#[test]
fn a_drawing_surface_is_updated_on_the_render_thread() {
    let services = RenderThreadServices::new();
    let log = Arc::new(InteropLog::default());
    let interop = services.interop(&log);
    let imported = interop.import_image(
        Rc::new(PlatformHandle::new(1, Some("Fake"))),
        PlatformGraphicsExternalImageProperties { width: 1, height: 1, ..Default::default() },
    );
    let surface = services.compositor.create_drawing_surface();
    services.run_jobs();
    assert!(imported.import_completed().is_completed_successfully());

    let update = surface.update_async(&*imported);
    assert!(!update.is_completed());
    let render_thread = services.run_jobs();
    assert!(update.is_completed_successfully(), "{:?}", update.exception().map(|e| e.to_string()));
    assert_eq!(vec![render_thread], *log.snapshots.lock().unwrap());

    let snapshot = log.last_snapshot.lock().unwrap().clone().expect("a snapshot was taken");
    assert!(!snapshot.is_disposed.load(std::sync::atomic::Ordering::SeqCst));

    // The other means of synchronisation take the same way.
    let update = surface.update_with_keyed_mutex_async(&*imported, 1, 0);
    let render_thread = services.run_jobs();
    assert!(update.is_completed_successfully(), "{:?}", update.exception().map(|e| e.to_string()));
    assert_eq!(Some(&render_thread), log.snapshots.lock().unwrap().last());
    // The surface let go of the first snapshot.
    assert!(snapshot.is_disposed.load(std::sync::atomic::Ordering::SeqCst));

    let snapshot = log.last_snapshot.lock().unwrap().clone().expect("a snapshot was taken");
    surface.dispose();
    services.run_jobs();
    assert!(snapshot.is_disposed.load(std::sync::atomic::Ordering::SeqCst));
}

#[test]
fn an_update_with_a_disposed_image_fails_on_the_render_thread() {
    let services = RenderThreadServices::new();
    let log = Arc::new(InteropLog::default());
    let interop = services.interop(&log);
    let surface = services.compositor.create_drawing_surface();

    let imported = interop.import_image(
        Rc::new(PlatformHandle::new(1, Some("Fake"))),
        PlatformGraphicsExternalImageProperties { width: 1, height: 1, ..Default::default() },
    );
    // The jobs run in the order they were sent: the import, the disposal,
    // and an update that finds nothing to take a snapshot of.
    let disposed = imported.dispose_async();
    let update = surface.update_async(&*imported);
    services.run_jobs();
    assert!(imported.import_completed().is_completed_successfully());
    assert!(disposed.is_completed_successfully());
    assert_eq!(1, log.disposals.lock().unwrap().len());
    assert!(update.is_faulted());
    assert!(log.snapshots.lock().unwrap().is_empty());
}

#[test]
fn an_image_of_a_shared_context_is_imported_on_the_render_thread() {
    let services = RenderThreadServices::new();
    let log = Arc::new(InteropLog::default());
    let interop = services.interop(&log);

    // The image is shared: the caller keeps it, the render thread is handed
    // it for the import.
    let image = FakeSharedImage::new(11);
    let imported = interop.import_shared_image(image.clone());
    assert!(!imported.import_completed().is_completed());
    assert!(log.shared_imports.lock().unwrap().is_empty());

    let render_thread = services.run_jobs();
    assert!(
        imported.import_completed().is_completed_successfully(),
        "{:?}",
        imported.import_completed().exception().map(|e| e.to_string())
    );
    assert_eq!(vec![render_thread], *log.shared_imports.lock().unwrap());
    // The import let go of its reference: the image is the caller's alone.
    assert_eq!(1, Arc::strong_count(&image));
    assert_eq!(11, image.texture_id());

    // The imported image takes the same way as one of a handle.
    let surface = services.compositor.create_drawing_surface();
    let update = surface.update_async(&*imported);
    let render_thread = services.run_jobs();
    assert!(update.is_completed_successfully(), "{:?}", update.exception().map(|e| e.to_string()));
    assert_eq!(vec![render_thread], *log.snapshots.lock().unwrap());

    let disposed = imported.dispose_async();
    let render_thread = services.run_jobs();
    assert!(disposed.is_completed_successfully());
    assert_eq!(vec![render_thread], *log.disposals.lock().unwrap());

    // The caller disposes its image on its thread, where its context is.
    image.dispose();
    assert_eq!(0, image.texture_id());
    assert!(image.context.get().get());
}

/// Not from upstream: an image the caller let go of before the import ran
/// is released by the render thread, which does not touch the part bound to
/// the thread of the caller.
#[test]
fn an_image_of_a_shared_context_dropped_by_the_caller_is_released_on_the_render_thread() {
    let services = RenderThreadServices::new();
    let log = Arc::new(InteropLog::default());
    let interop = services.interop(&log);

    let image = FakeSharedImage::new(3);
    let context = image.context.get().clone();
    let imported = interop.import_shared_image(image);
    let render_thread = services.run_jobs();
    assert!(imported.import_completed().is_completed_successfully());
    assert_eq!(vec![render_thread], *log.shared_imports.lock().unwrap());
    // The part of this thread was left alone: not used, and not dropped.
    assert!(!context.get());
    assert_eq!(2, Rc::strong_count(&context));
}

/// Not from upstream: where the server runs on the thread of the
/// compositor, an image of a shared context is imported as before.
#[test]
fn an_image_of_a_shared_context_is_imported_on_the_thread_of_the_compositor() {
    let services = TestCompositor::new();
    let log = Arc::new(InteropLog::default());
    let interop =
        CompositionInterop::new(&services.compositor, Rc::new(LoggingExternalObjectsFeature { log: log.clone() }));

    let image = FakeSharedImage::new(5);
    let imported = interop.import_shared_image(image.clone());
    services.run_jobs();
    assert!(imported.import_completed().is_completed_successfully());
    assert_eq!(vec![thread::current().id()], *log.shared_imports.lock().unwrap());
}
