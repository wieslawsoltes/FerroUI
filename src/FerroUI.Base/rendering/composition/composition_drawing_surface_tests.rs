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
        _image: Rc<dyn ICompositionImportableSharedGpuContextImage>,
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
