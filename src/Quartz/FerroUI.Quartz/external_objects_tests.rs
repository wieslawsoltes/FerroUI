use super::*;
use ferroui_base::{
    platform::*, reactive::IDisposable, rendering::composition::frames::*, PixelSize, Rect,
};
use objc2::{rc::Retained, runtime::ProtocolObject};
use objc2_metal::*;
use std::{
    any::TypeId,
    ffi::c_void,
    sync::atomic::{AtomicUsize, Ordering},
    time::{Duration, Instant},
};

struct Device {
    device: Retained<ProtocolObject<dyn MTLDevice>>,
    queue: Retained<ProtocolObject<dyn MTLCommandQueue>>,
}
struct Nothing;
impl IDisposable for Nothing {
    fn dispose(&self) {}
}
impl IOptionalFeatureProvider for Device {
    fn try_get_feature(&self, _: TypeId) -> Option<Rc<dyn Any>> {
        None
    }
}
impl IPlatformGraphicsContext for Device {
    fn is_lost(&self) -> bool {
        false
    }
    fn ensure_current(&self) -> Rc<dyn IDisposable> {
        Rc::new(Nothing)
    }
    fn dispose(&self) {}
    fn as_any(&self) -> &dyn Any {
        self
    }
}
impl ferroui_metal::IMetalDevice for Device {
    fn device(&self) -> *mut c_void {
        Retained::as_ptr(&self.device).cast_mut().cast()
    }
    fn command_queue(&self) -> *mut c_void {
        Retained::as_ptr(&self.queue).cast_mut().cast()
    }
}
struct Texture(Retained<ProtocolObject<dyn MTLTexture>>);
impl IMetalExternalTexture for Texture {
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
        Retained::as_ptr(&self.0).cast_mut().cast()
    }
    fn dispose(&self) {}
}
struct Storage(Arc<AtomicUsize>);
impl Drop for Storage {
    fn drop(&mut self) {
        self.0.fetch_add(1, Ordering::SeqCst);
    }
}
fn metadata(color: FrameColor, format: FrameFormat, top_left_origin: bool) -> FrameMetadata {
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
        format,
        color,
        alpha: FrameAlpha::Opaque,
        rotation: FrameRotation::None,
        mirrored: false,
        top_left_origin,
        timestamp: Duration::ZERO,
        device: Some(DeviceIdentity::Luid(vec![7])),
        device_generation: 7,
        planes: vec![FramePlane {
            offset: 0,
            stride: 32,
            length: 256,
        }],
        retained_bytes: 256,
    }
}
#[test]
fn imported_frames_validate_convert_on_gpu_once_and_retire_after_completion() {
    let device = MTLCreateSystemDefaultDevice().expect("Metal test device");
    let queue = device.newCommandQueue().unwrap();
    let device = Rc::new(Device { device, queue });
    for format in [FrameFormat::Bgra8, FrameFormat::Rgba8] {
        let pf = if format == FrameFormat::Bgra8 {
            MTLPixelFormat::BGRA8Unorm
        } else {
            MTLPixelFormat::RGBA8Unorm
        };
        let desc = unsafe {
            MTLTextureDescriptor::texture2DDescriptorWithPixelFormat_width_height_mipmapped(
                pf, 8, 8, false,
            )
        };
        desc.setUsage(MTLTextureUsage::ShaderRead);
        desc.setStorageMode(MTLStorageMode::Shared);
        let texture = device.device.newTextureWithDescriptor(&desc).unwrap();
        // Only the test fixture is uploaded by the CPU. Production imports IOSurfaces.
        let pixels: Vec<u8> = (0..64)
            .flat_map(|i| {
                if i < 32 {
                    [200, 100, 50, 0]
                } else {
                    [40, 60, 80, 0]
                }
            })
            .collect();
        unsafe {
            texture.replaceRegion_mipmapLevel_withBytes_bytesPerRow(
                MTLRegion {
                    origin: MTLOrigin { x: 0, y: 0, z: 0 },
                    size: MTLSize {
                        width: 8,
                        height: 8,
                        depth: 1,
                    },
                },
                0,
                std::ptr::NonNull::new(pixels.as_ptr().cast_mut().cast()).unwrap(),
                32,
            );
        }
        for color in [FrameColor::Srgb, FrameColor::Rec709Full] {
            for top in [false, true] {
                let imported = ImportedImage {
                    device: device.clone(),
                    generation: 7,
                    backend_generation: unsafe { ffi::qm_generation() },
                    device_luid: Some(vec![7]),
                    imported: Rc::new(Texture(texture.clone())),
                    properties: PlatformGraphicsExternalImageProperties {
                        width: 8,
                        height: 8,
                        format: if format == FrameFormat::Bgra8 {
                            PlatformGraphicsExternalImageFormat::B8G8R8A8UNorm
                        } else {
                            PlatformGraphicsExternalImageFormat::R8G8B8A8UNorm
                        },
                        top_left_origin: top,
                        ..Default::default()
                    },
                    format: if format == FrameFormat::Bgra8 {
                        PlatformGraphicsExternalImageFormat::B8G8R8A8UNorm
                    } else {
                        PlatformGraphicsExternalImageFormat::R8G8B8A8UNorm
                    },
                    top_left_origin: top,
                };
                let budget = FrameBudget::new(Default::default());
                let m = metadata(color, format, top);
                let mut invalid = m.clone();
                invalid.device_generation += 1;
                assert_eq!(
                    imported
                        .acquire_shared_bitmap(budget.lease(invalid, ()).unwrap())
                        .err(),
                    Some(FrameError::StaleGeneration)
                );
                let mut invalid = m.clone();
                invalid.device = Some(DeviceIdentity::Luid(vec![8]));
                assert_eq!(
                    imported
                        .acquire_shared_bitmap(budget.lease(invalid, ()).unwrap())
                        .err(),
                    Some(FrameError::DeviceMismatch)
                );
                let mut invalid = m.clone();
                invalid.alpha = FrameAlpha::Straight;
                invalid.color = FrameColor::Srgb;
                assert_eq!(
                    imported
                        .acquire_shared_bitmap(budget.lease(invalid, ()).unwrap())
                        .err(),
                    Some(FrameError::UnsupportedFormat)
                );
                let mut invalid = m.clone();
                invalid.color = FrameColor::Rec709Limited;
                assert_eq!(
                    imported
                        .acquire_shared_bitmap(budget.lease(invalid, ()).unwrap())
                        .err(),
                    Some(FrameError::UnsupportedColor)
                );
                let drops = Arc::new(AtomicUsize::new(0));
                let frame = budget.lease(m, Storage(drops.clone())).unwrap();
                let counts = frame.counters();
                let shared = imported.acquire_shared_bitmap(frame).unwrap();
                assert_eq!(counts.snapshot().gpu_imports, 1);
                assert_eq!(counts.snapshot().gpu_conversions, 0);
                assert_eq!(drops.load(Ordering::SeqCst), 0);
                let target =
                    Bitmap::new(PixelSize::new(16, 16), ferroui_base::Vector::new(96., 96.));
                for _ in 0..2 {
                    let mut draw = target.drawing(false, None);
                    draw.draw_bitmap(
                        &*shared,
                        1.,
                        Rect::new(0., 0., 8., 8.),
                        Rect::new(0., 0., 16., 16.),
                    );
                    draw.dispose();
                }
                let data = target.pixels(); // Explicit test-only readback of final output.
                for (y, expected) in [
                    (2, if top { [200, 100, 50] } else { [40, 60, 80] }),
                    (13, if top { [40, 60, 80] } else { [200, 100, 50] }),
                ] {
                    let mut expected = expected;
                    if format == FrameFormat::Rgba8 {
                        expected.swap(0, 2);
                    }
                    let pixel = &data[(y * 16 + 2) * 4..][..4];
                    for i in 0..3 {
                        let v = f64::from(expected[i]) / 255.;
                        let linear = v.powf(2.4);
                        let corrected = if linear < 0.0031308 {
                            12.92 * linear
                        } else {
                            1.055 * linear.powf(1. / 2.4) - 0.055
                        };
                        let expected = if color == FrameColor::Rec709Full {
                            corrected * 255.
                        } else {
                            v * 255.
                        };
                        assert!(
                            (f64::from(pixel[i]) - expected).abs() < 1.5,
                            "{format:?} {color:?} top={top}: {pixel:?}"
                        );
                    }
                    assert_eq!(pixel[3], 255);
                }
                let deadline = Instant::now() + Duration::from_secs(3);
                while budget.gpu_reads() > 0 {
                    assert!(Instant::now() < deadline);
                    std::thread::yield_now();
                }
                assert_eq!(budget.usage().frames, 0);
                assert_eq!(budget.quarantined_reads(), 0);
                assert_eq!(drops.load(Ordering::SeqCst), 1);
                let counts = counts.snapshot();
                assert_eq!(counts.gpu_conversions, 1);
                assert_eq!(counts.preview, Default::default());
                assert_eq!(counts.probe, Default::default());
                shared.dispose();
            }
        }
    }
}

#[test]
fn dropping_an_undrawn_shared_frame_releases_without_submitting_gpu_work() {
    let source = Bitmap::new(PixelSize::new(8, 8), ferroui_base::Vector::new(96., 96.));
    // No sampling occurs; the opaque retained handle only checks the undrawn lifetime path.
    let handle = source.retained_surface();
    let budget = FrameBudget::new(Default::default());
    let drops = Arc::new(AtomicUsize::new(0));
    let frame = budget
        .lease(
            metadata(FrameColor::Srgb, FrameFormat::Bgra8, true),
            Storage(drops.clone()),
        )
        .unwrap();
    let bitmap = SharedFrameBitmap::new(handle.0, frame, unsafe { ffi::qm_generation() });
    drop(bitmap);
    assert_eq!(drops.load(Ordering::SeqCst), 1);
    assert_eq!(budget.usage().frames, 0);
    assert_eq!(budget.gpu_reads(), 0);
}
struct Event(Retained<ProtocolObject<dyn MTLSharedEvent>>);
impl IMetalSharedEvent for Event {
    fn handle(&self) -> *mut c_void {
        Retained::as_ptr(&self.0).cast_mut().cast()
    }
    fn dispose(&self) {}
    fn as_any(&self) -> &dyn Any {
        self
    }
}
#[test]
fn timeline_snapshot_waits_for_producer_then_signals_after_an_independent_gpu_copy() {
    let device = MTLCreateSystemDefaultDevice().unwrap();
    let queue = device.newCommandQueue().unwrap();
    let wait = device.newSharedEvent().unwrap();
    let signal = device.newSharedEvent().unwrap();
    let desc = unsafe {
        MTLTextureDescriptor::texture2DDescriptorWithPixelFormat_width_height_mipmapped(
            MTLPixelFormat::BGRA8Unorm,
            8,
            8,
            false,
        )
    };
    desc.setUsage(MTLTextureUsage::ShaderRead);
    desc.setStorageMode(MTLStorageMode::Shared);
    let texture = device.newTextureWithDescriptor(&desc).unwrap();
    let pixels: Vec<u8> = (0..64)
        .flat_map(|i| {
            if i < 32 {
                [200, 100, 50, 255]
            } else {
                [20, 40, 60, 255]
            }
        })
        .collect();
    unsafe {
        texture.replaceRegion_mipmapLevel_withBytes_bytesPerRow(
            MTLRegion {
                origin: MTLOrigin { x: 0, y: 0, z: 0 },
                size: MTLSize {
                    width: 8,
                    height: 8,
                    depth: 1,
                },
            },
            0,
            std::ptr::NonNull::new(pixels.as_ptr().cast_mut().cast()).unwrap(),
            32,
        )
    };
    let properties = PlatformGraphicsExternalImageProperties {
        width: 8,
        height: 8,
        format: PlatformGraphicsExternalImageFormat::B8G8R8A8UNorm,
        top_left_origin: false,
        ..Default::default()
    };
    let raw = Retained::as_ptr(&texture).cast_mut().cast();
    let shared = unsafe { SharedMetalImage::from_raw_texture(raw, properties.clone()) }.unwrap();
    let mut wrong = properties.clone();
    wrong.width = 7;
    assert!(matches!(
        unsafe { SharedMetalImage::from_raw_texture(raw, wrong) },
        Err(FrameError::InvalidDimensions)
    ));
    let mut wrong = properties;
    wrong.format = PlatformGraphicsExternalImageFormat::R8G8B8A8UNorm;
    assert!(matches!(
        unsafe { SharedMetalImage::from_raw_texture(raw, wrong) },
        Err(FrameError::UnsupportedFormat)
    ));
    let external = ExternalObjects::new(
        Rc::new(Device { device, queue }),
        7,
        unsafe { ffi::qm_generation() },
        Rc::new(IdentityFeature),
    );
    let imported = external.import_shared_image(shared.clone());
    shared.dispose(); // Import owns its texture independently of the producer wrapper.
    let a: Rc<dyn IPlatformRenderInterfaceImportedSemaphore> = Rc::new(ImportedSemaphore {
        event: Rc::new(Event(wait.clone())),
    });
    let b: Rc<dyn IPlatformRenderInterfaceImportedSemaphore> = Rc::new(ImportedSemaphore {
        event: Rc::new(Event(signal.clone())),
    });
    let snapshot = imported.snapshot_with_timeline_semaphores(&a, 3, &b, 4);
    assert_eq!(
        signal.signaledValue(),
        0,
        "Copy must wait for the producer event"
    );
    wait.setSignaledValue(3);
    let deadline = Instant::now() + Duration::from_secs(5);
    while signal.signaledValue() < 4 {
        assert!(Instant::now() < deadline);
        std::thread::yield_now();
    }
    let zeros = vec![0u8; 256];
    unsafe {
        texture.replaceRegion_mipmapLevel_withBytes_bytesPerRow(
            MTLRegion {
                origin: MTLOrigin { x: 0, y: 0, z: 0 },
                size: MTLSize {
                    width: 8,
                    height: 8,
                    depth: 1,
                },
            },
            0,
            std::ptr::NonNull::new(zeros.as_ptr().cast_mut().cast()).unwrap(),
            32,
        )
    };
    let pixels = snapshot.as_any().downcast_ref::<Bitmap>().unwrap().pixels();
    assert_eq!(&pixels[..4], &[20, 40, 60, 255]);
    assert_eq!(&pixels[7 * 32..7 * 32 + 4], &[200, 100, 50, 255]);
}

#[test]
fn renderer_queue_faults_invalidate_generations_and_recover_without_releasing_old_faults_into_new_contexts(
) {
    const FLAG: &str = "FERRO_QUARTZ_ISOLATED_RECOVERY_TEST";
    if std::env::var_os(FLAG).is_none() {
        assert!(std::process::Command::new(std::env::current_exe().unwrap()).args(["--exact","external_objects::tests::renderer_queue_faults_invalidate_generations_and_recover_without_releasing_old_faults_into_new_contexts","--nocapture"]).env(FLAG,"1").status().unwrap().success());
        return;
    }
    let device = MTLCreateSystemDefaultDevice().unwrap();
    let pointer = Retained::as_ptr(&device).cast_mut().cast();
    let old = unsafe { ffi::qm_configure(pointer) };
    let old_storage = Bitmap::new(PixelSize::new(37, 37), ferroui_base::Vector::new(96., 96.));

    let desc = unsafe {
        MTLTextureDescriptor::texture2DDescriptorWithPixelFormat_width_height_mipmapped(
            MTLPixelFormat::BGRA8Unorm,
            8,
            8,
            false,
        )
    };
    desc.setUsage(MTLTextureUsage::ShaderRead);
    let source = device.newTextureWithDescriptor(&desc).unwrap();
    let budget = FrameBudget::new(Default::default());
    let drops = Arc::new(AtomicUsize::new(0));
    let frame = budget
        .lease(
            metadata(FrameColor::Srgb, FrameFormat::Bgra8, true),
            Storage(drops.clone()),
        )
        .unwrap();
    let bitmap = SharedFrameBitmap::new(Retained::as_ptr(&source).cast_mut().cast(), frame, old);
    assert_eq!(unsafe { ffi::qm_is_lost(old) }, 0);
    unsafe { ffi::qm_record_fault(old) };
    assert_ne!(unsafe { ffi::qm_is_lost(old) }, 0);
    let new = unsafe { ffi::qm_configure(pointer) };
    assert!(new > old);
    assert_ne!(unsafe { ffi::qm_is_lost(old) }, 0);
    assert_eq!(unsafe { ffi::qm_is_lost(new) }, 0);
    assert!(
        bitmap.bitmap().is_none(),
        "An undrawn old-generation frame cannot enter the recovered queue"
    );
    drop(bitmap);
    assert_eq!(drops.load(Ordering::SeqCst), 1);
    assert_eq!(budget.gpu_reads(), 0);
    unsafe { ffi::qm_record_fault(old) }; // Late completion from the failed queue.
    assert_eq!(unsafe { ffi::qm_is_lost(new) }, 0);
    drop(old_storage); // An owner from the failed queue dies after recovery.
    let mut before = [0u64; 9];
    unsafe { ffi::qm_statistics(before.as_mut_ptr(), before.len()) };
    let fresh = Bitmap::new(PixelSize::new(37, 37), ferroui_base::Vector::new(96., 96.));
    let mut after = [0u64; 9];
    unsafe { ffi::qm_statistics(after.as_mut_ptr(), after.len()) };
    assert_eq!(
        after[8],
        before[8] + 1,
        "Old-generation storage must not enter the replacement queue's pool"
    );
    drop(fresh);
    let b = Bitmap::new(PixelSize::new(8, 8), ferroui_base::Vector::new(96., 96.));
    let mut draw = b.drawing(false, None);
    draw.clear(ferroui_base::media::Colors::RED);
    draw.dispose();
    assert_eq!(&b.pixels()[..4], &[0, 0, 255, 255]);
    assert_eq!(unsafe { ffi::qm_is_lost(new) }, 0);
}

struct IdentityFeature;
impl IMetalExternalObjectsFeature for IdentityFeature {
    fn supported_image_handle_types(&self) -> Vec<String> {
        vec![]
    }
    fn supported_semaphore_types(&self) -> Vec<String> {
        vec![]
    }
    fn device_luid(&self) -> Option<Vec<u8>> {
        Some(vec![7])
    }
    fn get_synchronization_capabilities(
        &self,
        _: &str,
    ) -> CompositionGpuImportedImageSynchronizationCapabilities {
        CompositionGpuImportedImageSynchronizationCapabilities::TIMELINE_SEMAPHORES
    }
    fn import_image(
        &self,
        _: Rc<dyn IPlatformHandle>,
        _: PlatformGraphicsExternalImageProperties,
    ) -> Rc<dyn IMetalExternalTexture> {
        unreachable!()
    }
    fn import_shared_event(&self, _: Rc<dyn IPlatformHandle>) -> Rc<dyn IMetalSharedEvent> {
        unreachable!()
    }
    fn submit_wait(&self, _: &dyn IMetalSharedEvent, _: u64) {
        unreachable!()
    }
    fn submit_signal(&self, _: &dyn IMetalSharedEvent, _: u64) {
        unreachable!()
    }
}

#[test]
#[cfg(feature = "media-test-fixtures")]
fn native_exceptions_are_contained_and_the_gpu_context_recovers() {
    const FLAG: &str = "FERRO_QUARTZ_ISOLATED_EXCEPTION_TEST";
    if std::env::var_os(FLAG).is_none() {
        assert!(std::process::Command::new(std::env::current_exe().unwrap())
            .args(["--exact", "external_objects::tests::native_exceptions_are_contained_and_the_gpu_context_recovers", "--nocapture"])
            .env(FLAG, "1").status().unwrap().success());
        return;
    }
    extern "C" {
        fn qm_fixture_exception() -> i32;
        fn qm_fixture_fail_allocation();
    }
    let device = MTLCreateSystemDefaultDevice().unwrap();
    let raw = Retained::as_ptr(&device).cast_mut().cast();
    let old = unsafe { ffi::qm_configure(raw) };
    assert_ne!(old, 0);
    assert_eq!(unsafe { qm_fixture_exception() }, 0);
    assert_ne!(unsafe { ffi::qm_is_lost(old) }, 0);
    let new = unsafe { ffi::qm_configure(raw) };
    assert!(new > old);
    assert_eq!(unsafe { ffi::qm_is_lost(new) }, 0);
    let bitmap =
        Bitmap::try_new(PixelSize::new(8, 8), ferroui_base::Vector::new(96., 96.)).unwrap();
    let mut draw = bitmap.drawing(false, None);
    draw.clear(ferroui_base::media::Colors::RED);
    draw.dispose();
    assert_eq!(&bitmap.pixels()[..4], &[0, 0, 255, 255]);
    // Image upload needs a temporary GPU surface. NULL here formerly reached CFRelease(NULL).
    let image = bitmap.image();
    unsafe {
        qm_fixture_fail_allocation();
    }
    bitmap
        .context(|p| unsafe { ffi::q_draw_image(p, image.0, 0., 0., 8., 8., 0., 0., 8., 8., 1.) });
    assert_ne!(unsafe { ffi::qm_is_lost(new) }, 0);
    assert!(unsafe { ffi::qm_configure(raw) } > new);
    let fresh = Bitmap::try_new(PixelSize::new(8, 8), ferroui_base::Vector::new(96., 96.)).unwrap();
    let mut draw = fresh.drawing(false, None);
    draw.clear(ferroui_base::media::Colors::BLUE);
    draw.dispose();
    assert_eq!(&fresh.pixels()[..4], &[255, 0, 0, 255]);
}
