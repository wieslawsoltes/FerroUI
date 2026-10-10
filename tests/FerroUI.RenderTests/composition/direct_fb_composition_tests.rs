//! Port of upstream's `Composition/DirectFbCompositionTests.cs`.
//!
//! The framebuffer of the test is a bitmap of the Skia library upstream
//! (`SKBitmap`), which the test locks by its address, erases and encodes
//! itself. Here it is a writeable bitmap of the render backend with the same
//! size and formats: its lock is the locked framebuffer, erasing it clears
//! its pixels, and it is saved by the encoder of the backend.
//!
//! A platform surface of the port may be asked for its render target by
//! another thread than the one that made it, so the surface of the test
//! creates its render target when asked instead of holding one; the target
//! has no state of its own.
//!
//! `skipGpu` of `CompareImages` concerns the outputs of upstream's software
//! GL and Vulkan renderers, which the port does not render: it is dropped.
//! The scheduler of the compositor is the one the helper of the tests uses
//! for upstream's `DispatcherCompositorScheduler`.

use crate::manual_render_timer::ManualRenderTimer;
use crate::test_base::{CompareOptions, TestBase};
use crate::test_render_root::TestRenderRoot;
use ferroui_base::media::imaging::PngBitmapEncoderOptions;
use ferroui_base::media::{Brushes, MediaContext};
use ferroui_base::platform::surfaces::{
    FramebufferLockProperties, FuncFramebufferRenderTarget, IFramebufferPlatformSurface, IFramebufferRenderTarget,
    IPlatformRenderSurface,
};
use ferroui_base::platform::{AlphaFormat, ILockedFramebuffer, IPlatformRenderInterface, IWriteableBitmapImpl, PixelFormats};
use ferroui_base::rendering::composition::{CompositingRenderer, CompositionOptions, Compositor, RenderSurfaces};
use ferroui_base::rendering::{IRenderer, RenderLoop};
use ferroui_base::threading::Dispatcher;
use ferroui_base::{FerroLocator, LocatorExtensions, PixelSize, Rect, Size, Vector};
use ferroui_controls::shapes::Rectangle;
use ferroui_controls::Canvas;
use std::any::Any;
use std::io::Write;
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

fn base() -> TestBase {
    TestBase::new(r"Composition\DirectFb")
}

struct FuncFramebufferSurface {
    cb: Box<dyn Fn() -> Rc<dyn IFramebufferRenderTarget> + Send + Sync>,
}

impl FuncFramebufferSurface {
    fn new(cb: impl Fn() -> Rc<dyn IFramebufferRenderTarget> + Send + Sync + 'static) -> FuncFramebufferSurface {
        FuncFramebufferSurface { cb: Box::new(cb) }
    }
}

impl IPlatformRenderSurface for FuncFramebufferSurface {
    fn as_framebuffer_surface(&self) -> Option<&dyn IFramebufferPlatformSurface> {
        Some(self)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl IFramebufferPlatformSurface for FuncFramebufferSurface {
    fn create_framebuffer_render_target(&self) -> Rc<dyn IFramebufferRenderTarget> {
        (self.cb)()
    }
}

#[test]
fn should_only_update_clipped_rects_when_retained_fb_is_advertised_false() {
    should_only_update_clipped_rects_when_retained_fb_is_advertised(false);
}

#[test]
fn should_only_update_clipped_rects_when_retained_fb_is_advertised_true() {
    should_only_update_clipped_rects_when_retained_fb_is_advertised(true);
}

fn should_only_update_clipped_rects_when_retained_fb_is_advertised(advertised: bool) {
    let t = base();
    let timer = Arc::new(ManualRenderTimer::new());
    let compositor = Compositor::with_scheduler(
        RenderLoop::from_timer(timer.clone()),
        None,
        true,
        &MediaContext::instance().scheduler(),
        Dispatcher::ui_thread(),
        Some(CompositionOptions { use_region_dirty_rect_clipping: Some(true), ..Default::default() }),
        None,
    );

    let control = Canvas::new();
    control.set_width(200.0);
    control.set_height(200.0);
    control.set_background(Some(Brushes::yellow()));
    let r1 = Rectangle::new();
    r1.set_fill(Some(Brushes::black()));
    r1.set_width(40.0);
    r1.set_height(40.0);
    r1.set_opacity(0.6);
    Canvas::set_left(&r1, 40.0);
    Canvas::set_top(&r1, 40.0);
    control.children().add(r1.clone());
    let r2 = Rectangle::new();
    r2.set_fill(Some(Brushes::black()));
    r2.set_width(40.0);
    r2.set_height(40.0);
    r2.set_opacity(0.6);
    Canvas::set_left(&r2, 120.0);
    Canvas::set_top(&r2, 40.0);
    control.children().add(r2.clone());
    let root = TestRenderRoot::new(1.0);
    let factory = FerroLocator::current().get_required_service::<dyn IPlatformRenderInterface>();
    let fb: Arc<dyn IWriteableBitmapImpl> = factory.create_writeable_bitmap(
        PixelSize::new(200, 200),
        Vector::new(96.0, 96.0),
        PixelFormats::RGBA8888,
        AlphaFormat::Premul,
    );

    let lock_fb = {
        let fb = fb.clone();
        move || -> Rc<dyn ILockedFramebuffer> { fb.lock() }
    };

    let previous_frame_is_retained = Arc::new(AtomicBool::new(false));
    let rt = {
        let previous_frame_is_retained = previous_frame_is_retained.clone();
        move || -> Rc<dyn IFramebufferRenderTarget> {
            let previous_frame_is_retained = previous_frame_is_retained.clone();
            let lock_fb = lock_fb.clone();
            Rc::new(FuncFramebufferRenderTarget::with_scene_info(
                move |_| {
                    let props = FramebufferLockProperties {
                        previous_frame_is_retained: previous_frame_is_retained.load(Ordering::SeqCst),
                    };
                    (lock_fb(), props)
                },
                advertised,
            ))
        }
    };

    let surface: Arc<dyn IPlatformRenderSurface> = Arc::new(FuncFramebufferSurface::new(rt));
    let surfaces: RenderSurfaces = Arc::new(move || vec![surface.clone()]);
    let renderer = CompositingRenderer::new(&root.source(), &compositor, surfaces);
    root.initialize(&renderer, &control.clone().upcast());
    control.measure(Size::new(control.width(), control.height()));
    control.arrange(Rect::from_size(control.desired_size()));
    renderer.start();
    Dispatcher::ui_thread().run_jobs(None);
    timer.trigger_tick();
    let image1 = format!(
        "Should_Only_Update_Clipped_Rects_When_Retained_Fb_Is_Advertised_advertized-{}_initial",
        bool_to_string(advertised)
    );
    save_file(&t, &fb, &image1);

    // `fb.Erase(SKColor.Empty)`.
    let locked = fb.lock();
    locked.with_data(&mut |data| data.fill(0));
    locked.dispose();

    previous_frame_is_retained.store(advertised, Ordering::SeqCst);

    r1.set_fill(Some(Brushes::red()));
    r2.set_fill(Some(Brushes::green()));
    Dispatcher::ui_thread().run_jobs(None);
    timer.trigger_tick();
    let image2 = format!(
        "Should_Only_Update_Clipped_Rects_When_Retained_Fb_Is_Advertised_advertized-{}_updated",
        bool_to_string(advertised)
    );
    save_file(&t, &fb, &image2);
    t.compare_images_with(&image1, CompareOptions { skip_immediate: true, ..Default::default() });
    t.compare_images_with(&image2, CompareOptions { skip_immediate: true, ..Default::default() });

    // `using var renderer`.
    renderer.dispose();
    root.release();
    fb.dispose();
}

/// How upstream's language formats a boolean in an interpolated string.
fn bool_to_string(value: bool) -> &'static str {
    if value {
        "True"
    } else {
        "False"
    }
}

fn save_file(t: &TestBase, bmp: &Arc<dyn IWriteableBitmapImpl>, name: &str) {
    let path = t.output_path().join(format!("{name}.composited.out.png"));
    let mut f = std::fs::File::create(path).expect("the output file is created");
    bmp.save(&mut f, &PngBitmapEncoderOptions::default().into()).expect("the image is saved");
    f.flush().expect("the image is written");
}
