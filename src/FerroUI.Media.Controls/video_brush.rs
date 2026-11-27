//! The brush of a 3D material that samples the frames of a video or a camera (#608, step D7 of
//! `docs/porting/media3d.md`, section 3.11).
//!
//! A [`VideoBrush`] follows a [`FrameMailbox`]: each new frame is imported as for a
//! [`TexturePresenter`](crate::TexturePresenter) and put into the slot of the brush on the render
//! thread, and the element that shows the scene is drawn again. The scene itself does not change,
//! so a new frame uploads no mesh or record. A frame also shown flat is acquired once.
//!
//! Frames are sampled on the GPU only: a renderer without the frame on its device draws the
//! material with its colour and counts it.

use crate::TextureFrameImporter;
use ferroui_base::{
    media::{
        immutable::ImmutableImageBrush, AlignmentX, AlignmentY, IBrush, SharedBrush, Stretch, TileMode,
    },
    rendering::{
        composition::{
            CompositionDrawingSurface, ElementComposition, ICompositionGpuInterop,
        },
        scene3d::Scene3DFrameSlot,
    },
    threading::{Dispatcher, DispatcherPriority},
    utilities::ThreadBound,
    Ref, RelativePoint, RelativeRect, Visual, WeakRef,
};
use ferroui_media::{frames::*, MediaError, MediaErrorKind, MediaEvent};
use std::{
    cell::{Cell, RefCell},
    rc::{Rc, Weak},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};

/// How many frames may be on their way to the render thread (as for a `TexturePresenter`).
const MAX_IN_FLIGHT: u8 = 2;

// Producer callbacks only signal this object; the owner is read on the UI thread.
struct BrushWake {
    dispatcher: Arc<Dispatcher>,
    owner: ThreadBound<RefCell<Option<Weak<Inner>>>>,
    queued: AtomicBool,
}

impl BrushWake {
    fn signal(self: &Arc<Self>) {
        if self.queued.swap(true, Ordering::AcqRel) {
            return;
        }
        let wake = self.clone();
        self.dispatcher.post(
            move || {
                wake.queued.store(false, Ordering::Release);
                let owner = wake.owner.get().borrow().clone();
                if let Some(inner) = owner.and_then(|weak| weak.upgrade()) {
                    Inner::present_latest(&inner);
                }
            },
            DispatcherPriority::RENDER,
        );
    }

    fn cancel(&self) {
        self.owner.get().borrow_mut().take();
    }
}

struct Inner {
    slot: Arc<Scene3DFrameSlot>,
    brush: SharedBrush,
    source: Arc<FrameMailbox>,
    importer: Rc<dyn TextureFrameImporter>,
    target: RefCell<Option<WeakRef<Visual>>>,
    interop: RefCell<Option<Rc<dyn ICompositionGpuInterop>>>,
    surface: RefCell<Option<CompositionDrawingSurface>>,
    current: Cell<Option<FrameIdentity>>,
    in_flight: Cell<u8>,
    generation: Cell<u64>,
    wake: RefCell<Option<Arc<BrushWake>>>,
    subscription: RefCell<Option<FrameMailboxSubscription>>,
    errors: MediaEvent<MediaError>,
}

impl Inner {
    fn invalidate_target(&self) {
        let target = self.target.borrow().as_ref().and_then(|weak| weak.upgrade());
        if let Some(target) = target {
            target.invalidate_visual();
        }
    }

    /// Stops following the source and gives the frame back on the render thread.
    fn release(&self) {
        self.generation.set(self.generation.get().wrapping_add(1));
        if let Some(wake) = self.wake.borrow_mut().take() {
            wake.cancel();
        }
        self.subscription.borrow_mut().take();
        self.interop.borrow_mut().take();
        self.current.set(None);
        if let Some(surface) = self.surface.borrow_mut().take() {
            surface.clear_frame_slot_async(self.slot.clone());
            surface.dispose();
        }
    }

    fn fail(&self, error: MediaError) {
        self.release();
        self.invalidate_target();
        self.errors.emit(&error);
    }

    fn present_latest(this: &Rc<Inner>) {
        if this.in_flight.get() >= MAX_IN_FLIGHT {
            return;
        }
        let Some(frame) = this.source.latest() else {
            if this.current.take().is_some() {
                if let Some(surface) = this.surface.borrow().as_ref() {
                    surface.clear_frame_slot_async(this.slot.clone());
                }
                this.invalidate_target();
            }
            return;
        };
        let identity = frame.metadata().identity;
        if this.current.get() == Some(identity) {
            return;
        }
        let interop = this.interop.borrow().clone();
        let surface = this.surface.borrow().clone();
        let (Some(interop), Some(surface)) = (interop, surface) else {
            return;
        };
        if interop.is_lost() {
            this.fail(MediaError::new(MediaErrorKind::DeviceLost, "Graphics device was lost"));
            return;
        }
        let imported = match this.importer.import(&*interop, &frame) {
            Ok(imported) => imported,
            Err(error) => {
                this.fail(error);
                return;
            }
        };
        let validity = frame.clone();
        let task =
            surface.update_frame_slot_async(&*imported.image, frame, imported.producer_ready, this.slot.clone());
        // Import, update and disposal are ordered jobs of the render thread.
        imported.image.dispose_async();
        this.current.set(Some(identity));
        this.in_flight.set(this.in_flight.get() + 1);
        let complete = task.clone();
        let weak = Rc::downgrade(this);
        let generation = this.generation.get();
        task.on_completed(move || {
            let Some(this) = weak.upgrade() else { return };
            if this.generation.get() != generation {
                return;
            }
            this.in_flight.set(this.in_flight.get().saturating_sub(1));
            if !validity.is_current() {
                Inner::present_latest(&this);
                return;
            }
            if let Some(error) = complete.exception() {
                this.fail(MediaError::new(MediaErrorKind::Unsupported, error.to_string()));
                return;
            }
            this.invalidate_target();
            Inner::present_latest(&this);
        });
    }
}

impl Drop for Inner {
    fn drop(&mut self) {
        self.release();
    }
}

/// A brush for a 3D material that samples the frames of a [`FrameMailbox`]: see the module.
///
/// Give [`brush`](Self::brush) to a material and [`attach`](Self::attach) the element that shows
/// the scene. The brush follows the source until it is detached or dropped.
pub struct VideoBrush(Rc<Inner>);

impl VideoBrush {
    /// A brush of the frames of `source`, imported by `importer`, placed by `stretch`.
    pub fn new(source: Arc<FrameMailbox>, importer: Rc<dyn TextureFrameImporter>, stretch: Stretch) -> VideoBrush {
        let tile = ImmutableImageBrush::new(
            None,
            AlignmentX::Center,
            AlignmentY::Center,
            Some(RelativeRect::FILL),
            1.0,
            None,
            RelativePoint::TOP_LEFT,
            Some(RelativeRect::FILL),
            stretch,
            TileMode::None,
            None,
        );
        let slot = Scene3DFrameSlot::new();
        let brush = SharedBrush::from_frame_slot(&tile, slot.clone()).expect("an image brush is a tile brush");
        VideoBrush(Rc::new(Inner {
            slot,
            brush,
            source,
            importer,
            target: RefCell::new(None),
            interop: RefCell::new(None),
            surface: RefCell::new(None),
            current: Cell::new(None),
            in_flight: Cell::new(0),
            generation: Cell::new(0),
            wake: RefCell::new(None),
            subscription: RefCell::new(None),
            errors: MediaEvent::default(),
        }))
    }

    /// The brush to give a material (`DiffuseMaterial::set_brush` and the others).
    pub fn brush(&self) -> Rc<dyn IBrush> {
        Rc::new(self.0.brush.clone())
    }

    /// What went wrong with a frame; the brush has then stopped following its source.
    pub fn errors(&self) -> &MediaEvent<MediaError> {
        &self.0.errors
    }

    /// Follows the source for `target`, the element that shows the scene (a `Viewport3D`), which
    /// is drawn again for each frame. `false` when `target` is not in a composed visual tree.
    pub fn attach(&self, target: &Ref<Visual>) -> bool {
        let Some(element) = ElementComposition::get_element_visual(target) else {
            return false;
        };
        self.0.release();
        *self.0.target.borrow_mut() = Some(target.downgrade());
        let compositor = element.compositor().clone();
        let generation = self.0.generation.get();
        let weak = Rc::downgrade(&self.0);
        Dispatcher::ui_thread().spawn_local(async move {
            let interop = compositor.try_get_composition_gpu_interop().await;
            let Some(inner) = weak.upgrade().filter(|inner| inner.generation.get() == generation) else {
                return;
            };
            let Some(interop) = interop else {
                inner.fail(MediaError::new(
                    MediaErrorKind::Unsupported,
                    "Renderer has no shared texture importer",
                ));
                return;
            };
            *inner.surface.borrow_mut() = Some(compositor.create_drawing_surface());
            Inner::follow(&inner, interop);
        });
        true
    }

    /// Stops following the source and gives the frame back.
    pub fn detach(&self) {
        self.0.release();
        self.0.target.borrow_mut().take();
    }
}

impl Inner {
    fn follow(this: &Rc<Inner>, interop: Rc<dyn ICompositionGpuInterop>) {
        *this.interop.borrow_mut() = Some(interop);
        let wake = Arc::new(BrushWake {
            dispatcher: Dispatcher::ui_thread(),
            owner: ThreadBound::new(RefCell::new(Some(Rc::downgrade(this)))),
            queued: AtomicBool::new(false),
        });
        let signal = wake.clone();
        *this.subscription.borrow_mut() = Some(this.source.subscribe(move || signal.signal()));
        *this.wake.borrow_mut() = Some(wake);
        Inner::present_latest(this);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ImportedTextureFrame;
    use ferroui_base::media::imaging::BitmapEncoderOptions;
    use ferroui_base::platform::*;
    use ferroui_base::rendering::composition::*;
    use ferroui_base::{PixelSize, Vector};
    use ferroui_controls::testing::{CompositorTestServices, TestServices};
    use std::any::Any;

    struct LeaseBitmap(#[allow(dead_code)] FrameLease);
    impl IBitmapImpl for LeaseBitmap {
        fn dpi(&self) -> Vector {
            Vector::new(96., 96.)
        }
        fn pixel_size(&self) -> PixelSize {
            PixelSize::new(4, 4)
        }
        fn version(&self) -> i32 {
            1
        }
        fn save(&self, _: &mut dyn std::io::Write, _: &BitmapEncoderOptions) -> std::io::Result<()> {
            unreachable!()
        }
        fn dispose(&self) {}
        fn as_any(&self) -> &dyn Any {
            self
        }
    }
    struct Imported;
    impl IPlatformRenderInterfaceImportedObject for Imported {
        fn dispose(&self) {}
    }
    impl IPlatformRenderInterfaceImportedImage for Imported {
        fn acquire_shared_bitmap(&self, frame: FrameLease) -> Result<Arc<SharedBitmapImpl>, FrameError> {
            Ok(Arc::new(LeaseBitmap(frame)))
        }
        fn snapshot_with_keyed_mutex(&self, _: u32, _: u32) -> Arc<SharedBitmapImpl> {
            unreachable!()
        }
        fn snapshot_with_semaphores(
            &self,
            _: &Rc<dyn IPlatformRenderInterfaceImportedSemaphore>,
            _: &Rc<dyn IPlatformRenderInterfaceImportedSemaphore>,
        ) -> Arc<SharedBitmapImpl> {
            unreachable!()
        }
        fn snapshot_with_timeline_semaphores(
            &self,
            _: &Rc<dyn IPlatformRenderInterfaceImportedSemaphore>,
            _: u64,
            _: &Rc<dyn IPlatformRenderInterfaceImportedSemaphore>,
            _: u64,
        ) -> Arc<SharedBitmapImpl> {
            unreachable!()
        }
        fn snapshot_with_automatic_sync(&self) -> Arc<SharedBitmapImpl> {
            unreachable!()
        }
    }
    struct Feature;
    impl IExternalObjectsRenderInterfaceContextFeature for Feature {
        fn supported_image_handle_types(&self) -> Vec<String> {
            vec!["Fake".into()]
        }
        fn supported_semaphore_types(&self) -> Vec<String> {
            vec![]
        }
        fn import_image(
            &self,
            _: Rc<dyn IPlatformHandle>,
            _: PlatformGraphicsExternalImageProperties,
        ) -> Rc<dyn IPlatformRenderInterfaceImportedImage> {
            Rc::new(Imported)
        }
        fn import_shared_image(
            &self,
            _: Arc<dyn ICompositionImportableSharedGpuContextImage>,
        ) -> Rc<dyn IPlatformRenderInterfaceImportedImage> {
            unreachable!()
        }
        fn import_semaphore(&self, _: Rc<dyn IPlatformHandle>) -> Rc<dyn IPlatformRenderInterfaceImportedSemaphore> {
            unreachable!()
        }
        fn get_synchronization_capabilities(&self, _: &str) -> CompositionGpuImportedImageSynchronizationCapabilities {
            CompositionGpuImportedImageSynchronizationCapabilities::AUTOMATIC
        }
        fn device_uuid(&self) -> Option<Vec<u8>> {
            None
        }
        fn device_luid(&self) -> Option<Vec<u8>> {
            None
        }
    }
    struct Importer(Cell<usize>);
    impl TextureFrameImporter for Importer {
        fn import(
            &self,
            interop: &dyn ICompositionGpuInterop,
            _: &FrameLease,
        ) -> Result<ImportedTextureFrame, MediaError> {
            self.0.set(self.0.get() + 1);
            let image = interop.import_image(
                Rc::new(PlatformHandle::new(1, Some("Fake"))),
                PlatformGraphicsExternalImageProperties {
                    width: 4,
                    height: 4,
                    format: PlatformGraphicsExternalImageFormat::B8G8R8A8UNorm,
                    memory_size: 64,
                    memory_offset: 0,
                    top_left_origin: true,
                    vulkan_properties: None,
                    dma_buf_properties: None,
                },
            );
            let producer_ready = GpuCompletion::new();
            producer_ready.complete();
            Ok(ImportedTextureFrame { image, producer_ready })
        }
    }
    fn metadata(sequence: u64) -> FrameMetadata {
        FrameMetadata {
            identity: FrameIdentity { source: 1, generation: 1, sequence },
            width: 4,
            height: 4,
            visible: FrameRegion { x: 0, y: 0, width: 4, height: 4 },
            format: FrameFormat::Bgra8,
            color: FrameColor::Srgb,
            alpha: FrameAlpha::Opaque,
            rotation: FrameRotation::None,
            mirrored: false,
            top_left_origin: true,
            timestamp: std::time::Duration::ZERO,
            device: None,
            device_generation: 1,
            planes: vec![FramePlane { offset: 0, stride: 16, length: 64 }],
            retained_bytes: 64,
        }
    }

    #[test]
    fn a_video_brush_puts_the_latest_frame_into_its_slot_and_gives_it_back() {
        let services = CompositorTestServices::start(TestServices::styled_window());
        let compositor = services.compositor();
        let interop = CompositionInterop::new(compositor, Rc::new(Feature));
        let source = Arc::new(FrameMailbox::new(1, 1));
        let budget = FrameBudget::new(Default::default());
        let importer = Rc::new(Importer(Cell::new(0)));
        let video = VideoBrush::new(source.clone(), importer.clone(), Stretch::Fill);
        let shared = video.brush().to_shared().expect("a shared brush");
        let slot = shared.frame_slot().expect("the slot of the brush").clone();
        assert!(slot.current().is_none(), "no frame yet");

        *video.0.surface.borrow_mut() = Some(compositor.create_drawing_surface());
        Inner::follow(&video.0, interop);
        source.publish(budget.lease(metadata(1), ()).unwrap()).unwrap();
        Inner::present_latest(&video.0);
        Inner::present_latest(&video.0); // The pending frame is not imported twice.
        assert_eq!(1, importer.0.get());
        services.run_jobs();
        assert_eq!(1, slot.current().expect("the frame").1.identity.sequence);
        assert_eq!(0, video.0.in_flight.get());

        source.publish(budget.lease(metadata(2), ()).unwrap()).unwrap();
        Inner::present_latest(&video.0);
        services.run_jobs();
        assert_eq!(2, slot.current().expect("the next frame").1.identity.sequence);
        assert_eq!(2, importer.0.get());

        video.detach();
        services.run_jobs();
        assert!(slot.current().is_none(), "the frame is given back");
        source.clear();
        drop(video);
        services.run_jobs();
        assert_eq!(0, budget.usage().frames);
    }
}
