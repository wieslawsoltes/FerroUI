use super::*;
use crate::{
    gpu::{graphite::GraphiteGrContext, ISkiaGrContext},
    i_drawable_bitmap_impl::try_get_drawable_bitmap,
};
use ferroui_base::rendering::composition::frames::*;
use skia_safe::{
    gpu::graphite::mtl::{context_factory, BackendContext},
    AlphaType, Color, ImageInfo, Paint, Rect, SurfaceProps,
};
use std::cell::RefCell;
use std::{
    ffi::c_void,
    sync::atomic::{AtomicUsize, Ordering},
    time::{Duration, Instant},
};
extern "C" {
    fn ferro_frame_test_create(alpha: f64) -> *mut c_void;
    fn ferro_frame_test_device(p: *mut c_void) -> *mut c_void;
    fn ferro_frame_test_queue(p: *mut c_void) -> *mut c_void;
    fn ferro_frame_test_texture(p: *mut c_void) -> *mut c_void;
    fn ferro_frame_test_forget_texture(p: *mut c_void);
    fn ferro_frame_test_allocation_size(p: *mut c_void) -> u64;
    fn ferro_frame_test_device_id(p: *mut c_void) -> u64;
    fn ferro_frame_test_block(p: *mut c_void);
    fn ferro_frame_test_unblock(p: *mut c_void);
    fn ferro_frame_test_release(p: *mut c_void);
}
struct Owner(usize);
struct BlockedQueue(Arc<Owner>);
impl Drop for BlockedQueue {
    fn drop(&mut self) {
        unsafe {
            ferro_frame_test_unblock(self.0 .0 as *mut c_void);
        }
    }
}
impl Drop for Owner {
    fn drop(&mut self) {
        unsafe {
            ferro_frame_test_release(self.0 as *mut c_void);
        }
    }
}
struct NativeTexture(Arc<Owner>);
impl IMetalExternalTexture for NativeTexture {
    fn width(&self) -> i32 {
        8
    }
    fn height(&self) -> i32 {
        8
    }
    fn samples(&self) -> i32 {
        1
    }
    fn handle(&self) -> *mut c_void {
        unsafe { ferro_frame_test_texture(self.0 .0 as *mut c_void) }
    }
    fn dispose(&self) {}
}
struct Feature(Vec<u8>);
impl IMetalExternalObjectsFeature for Feature {
    fn device_luid(&self) -> Option<Vec<u8>> {
        Some(self.0.clone())
    }
    fn supported_image_handle_types(&self) -> Vec<String> {
        vec![]
    }
    fn supported_semaphore_types(&self) -> Vec<String> {
        vec![]
    }
    fn get_synchronization_capabilities(
        &self,
        _: &str,
    ) -> CompositionGpuImportedImageSynchronizationCapabilities {
        CompositionGpuImportedImageSynchronizationCapabilities::AUTOMATIC
    }
    fn import_image(
        &self,
        _: Rc<dyn ferroui_base::platform::IPlatformHandle>,
        _: ferroui_base::platform::PlatformGraphicsExternalImageProperties,
    ) -> Rc<dyn IMetalExternalTexture> {
        unreachable!()
    }
    fn import_shared_event(
        &self,
        _: Rc<dyn ferroui_base::platform::IPlatformHandle>,
    ) -> Rc<dyn IMetalSharedEvent> {
        unreachable!()
    }
    fn submit_wait(&self, _: &dyn IMetalSharedEvent, _: u64) {
        unreachable!()
    }
    fn submit_signal(&self, _: &dyn IMetalSharedEvent, _: u64) {
        unreachable!()
    }
}
struct Resource {
    _owner: Arc<Owner>,
    drops: Arc<AtomicUsize>,
}
impl Drop for Resource {
    fn drop(&mut self) {
        self.drops.fetch_add(1, Ordering::SeqCst);
    }
}
#[test]
fn native_shared_metal_frame_samples_without_snapshot_and_retires_after_gpu() {
    for (alpha, color) in [
        (1.0, FrameColor::Srgb),
        (0.5, FrameColor::Srgb),
        (1.0, FrameColor::Rec709Full),
    ] {
        let opaque = unsafe { ferro_frame_test_create(alpha) };
        assert!(!opaque.is_null(), "Actual native Metal device required");
        let owner = Arc::new(Owner(opaque as usize));
        let queue = unsafe { ferro_frame_test_queue(opaque) } as usize;
        let device = unsafe { ferro_frame_test_device(opaque) };
        let id = unsafe { ferro_frame_test_device_id(opaque) }
            .to_ne_bytes()
            .to_vec();
        let native = unsafe { BackendContext::new(device, queue as *mut c_void) };
        let gr = Rc::new(
            GraphiteGrContext::new(context_factory::make_metal(&native, None).unwrap()).unwrap(),
        );
        gr.set_frame_completion_handler(Box::new(move |completion| unsafe {
            super::super::frame_completion::schedule(queue as *mut c_void, completion);
        }));
        let imported = ImportedImage {
            context: Rc::new(RefCell::new(Some(gr.clone()))),
            feature: Rc::new(Feature(id.clone())),
            texture: Rc::new(NativeTexture(owner.clone())),
            color_type: ColorType::BGRA8888,
            top_left_origin: true,
        };
        let drops = Arc::new(AtomicUsize::new(0));
        let budget = FrameBudget::new(FrameBudgetLimits::default());
        let frame = budget
            .lease(
                FrameMetadata {
                    identity: FrameIdentity {
                        source: 1,
                        generation: 1,
                        sequence: 1,
                    },
                    width: 8,
                    height: 8,
                    visible: FrameRegion {
                        x: 0,
                        y: 0,
                        width: 8,
                        height: 8,
                    },
                    format: FrameFormat::Bgra8,
                    color,
                    alpha: if alpha == 1. {
                        FrameAlpha::Opaque
                    } else {
                        FrameAlpha::Premultiplied
                    },
                    rotation: FrameRotation::None,
                    mirrored: false,
                    top_left_origin: true,
                    timestamp: Duration::ZERO,
                    device: Some(DeviceIdentity::Luid(id)),
                    device_generation: gr.frame_context_id().unwrap(),
                    planes: vec![FramePlane {
                        offset: 0,
                        stride: 32,
                        length: 256,
                    }],
                    retained_bytes: unsafe { ferro_frame_test_allocation_size(opaque) },
                },
                Resource {
                    _owner: owner.clone(),
                    drops: drops.clone(),
                },
            )
            .unwrap();
        let counters = frame.counters();
        let mut stale = frame.metadata().clone();
        stale.device_generation += 1;
        let stale = budget.lease(stale, ()).unwrap();
        assert!(matches!(
            imported.acquire_shared_bitmap(stale),
            Err(FrameError::StaleGeneration)
        ));
        let bitmap = imported.acquire_shared_bitmap(frame.clone()).unwrap();
        // The Skia Metal wrapper must own its native texture view. Keep only
        // source storage after import, so the fixture cannot mask a missing
        // native texture retain inside the direct bridge.
        unsafe {
            ferro_frame_test_forget_texture(opaque);
        }
        let info = ImageInfo::new((8, 8), ColorType::RGBA8888, AlphaType::Premul, None);
        let mut output = gr.create_surface(&info, &SurfaceProps::default()).unwrap();
        output.canvas().clear(Color::BLACK);
        unsafe {
            ferro_frame_test_block(opaque);
        }
        let blocked = BlockedQueue(owner.clone());
        try_get_drawable_bitmap(&*bitmap).unwrap().draw(
            Some(&*gr),
            output.canvas(),
            &Rect::from_wh(8., 8.),
            &Rect::from_wh(8., 8.),
            SamplingOptions::default(),
            &Paint::default(),
        );
        gr.flush();
        bitmap.dispose();
        drop(bitmap);
        drop(frame);
        assert_eq!(
            budget.usage().frames,
            1,
            "submitted GPU read must own decoder storage"
        );
        assert_eq!(drops.load(Ordering::SeqCst), 0);
        drop(blocked);
        counters
            .cpu_transfer(
                PixelTransferPurpose::Diagnostic,
                256,
                PresentationRoute::SharedTexture,
            )
            .unwrap();
        let raster = gr.snapshot_to_raster(&mut output).unwrap();
        let mut pixels = vec![0u8; 256];
        counters
            .cpu_transfer(
                PixelTransferPurpose::Diagnostic,
                256,
                PresentationRoute::SharedTexture,
            )
            .unwrap();
        assert!(raster.read_pixels(
            &info,
            &mut pixels,
            32,
            (0, 0),
            skia_safe::image::CachingHint::Disallow
        ));
        for pixel in pixels.chunks_exact(4) {
            let convert = |value: f64| {
                let linear = (value / 255.0).powf(2.4);
                let srgb = if linear <= 0.0031308 {
                    12.92 * linear
                } else {
                    1.055 * linear.powf(1.0 / 2.4) - 0.055
                };
                (srgb * 255.0).round() as i16
            };
            let expected = if color == FrameColor::Rec709Full {
                [convert(64.), convert(128.), convert(191.), 255]
            } else {
                [
                    (64. * alpha) as i16,
                    (128. * alpha) as i16,
                    (191. * alpha) as i16,
                    255,
                ]
            };
            for (actual, expected) in pixel.iter().zip(expected) {
                assert!((*actual as i16 - expected).abs() <= 1, "{pixel:?}");
            }
        }
        let deadline = Instant::now() + Duration::from_secs(3);
        while budget.usage().frames != 0 && Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(1));
        }
        assert_eq!(drops.load(Ordering::SeqCst), 1);
        let counts = counters.snapshot();
        assert_eq!(counts.gpu_imports, 1);
        assert_eq!(counts.gpu_copies, 0);
        assert_eq!(counts.preview, PixelTransferCount::default());
        assert!(counts.diagnostic.bytes > 0);
        assert_eq!(counts.diagnostic.attempts, 2);
        assert_eq!(counts.diagnostic.bytes, 512);
        gr.dispose();
    }
}
