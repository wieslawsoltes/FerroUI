//! Port of upstream's `Media/DrawingContentTests.cs`.
//!
//! Verifies that replacing a resource inside a drawing is reflected in what a
//! consuming control renders. The initial frame is intentionally ignored
//! (covered elsewhere); only the frame produced after the change is compared
//! against the reference image.
//!
//! `skipGpu` of upstream's comparisons concerns upstream's Mesa GL outputs,
//! which the port does not render: the argument is dropped.
//!
//! Upstream's frame buffer is a bitmap of the Skia library, which encodes
//! itself into the output file. The test crate does not depend on that
//! library, so the frame buffer is a block of memory in the same format and
//! the file is written by a bitmap of the port created from that memory.

use crate::manual_render_timer::ManualRenderTimer;
use crate::test_base::{CompareOptions, TestBase};
use crate::test_render_root::TestRenderRoot;
use ferroui_base::media::imaging::{Bitmap, PngBitmapEncoderOptions};
use ferroui_base::media::{Brushes, DrawingBrush, DrawingImage, GeometryDrawing, RectangleGeometry, Stretch};
use ferroui_base::platform::surfaces::{
    FuncFramebufferRenderTarget, IFramebufferPlatformSurface, IFramebufferRenderTarget, IPlatformRenderSurface,
};
use ferroui_base::platform::{AlphaFormat, ILockedFramebuffer, PixelFormat};
use ferroui_base::rendering::composition::{CompositingRenderer, Compositor, ICompositorScheduler, RenderSurfaces};
use ferroui_base::rendering::{IRenderer, RenderLoop};
use ferroui_base::threading::{Dispatcher, DispatcherPriority};
use ferroui_base::{IntoRef, PixelSize, Rect, Ref, Vector};
use ferroui_controls::{Border, Control, Image};
use std::any::Any;
use std::rc::Rc;
use std::sync::Arc;

fn base() -> TestBase {
    TestBase::new(r"Media\DrawingContent")
}

/// The scheduler of upstream's unit tests (`DispatcherCompositorScheduler`).
struct DispatcherCompositorScheduler;

impl ICompositorScheduler for DispatcherCompositorScheduler {
    fn commit_requested(&self, compositor: &Rc<Compositor>) {
        let compositor = compositor.clone();
        Dispatcher::ui_thread().post_local(
            move || {
                compositor.commit();
            },
            DispatcherPriority::UI_THREAD_RENDER,
        );
    }
}

/// The frame buffer the surface renders to: the address of its memory, which the test owns for as long as the
/// renderer lives.
struct FuncFramebufferSurface {
    address: usize,
    size: PixelSize,
    row_bytes: i32,
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
        let (address, size, row_bytes) = (self.address, self.size, self.row_bytes);
        Rc::new(FuncFramebufferRenderTarget::new(move || {
            Rc::new(ferroui_skia::LockedFramebuffer::new(
                address as *mut u8,
                size,
                row_bytes,
                Vector::new(96.0, 96.0),
                PixelFormat::RGBA8888,
                AlphaFormat::Premul,
                None,
            )) as Rc<dyn ILockedFramebuffer>
        }))
    }
}

#[test]
fn drawing_brush_reflects_replaced_inner_brush() {
    let t = base();
    let drawing = GeometryDrawing::new();
    drawing.set_brush(Some(Brushes::blue()));
    drawing.set_geometry(RectangleGeometry::with_rect(Rect::new(0.0, 0.0, 100.0, 100.0)));

    let target = Border::new();
    target.set_width(100.0);
    target.set_height(100.0);
    let background = DrawingBrush::with_drawing(drawing.clone());
    background.set_stretch(Stretch::Fill);
    target.set_background(Some(background.into()));

    render_change(&t, &target, || drawing.set_brush(Some(Brushes::red())), "DrawingBrush_Reflects_Replaced_Inner_Brush");
    t.compare_images_with(
        "DrawingBrush_Reflects_Replaced_Inner_Brush",
        CompareOptions { skip_immediate: true, ..Default::default() },
    );
}

#[test]
fn drawing_image_reflects_replaced_inner_brush() {
    let t = base();
    let drawing = GeometryDrawing::new();
    drawing.set_brush(Some(Brushes::blue()));
    drawing.set_geometry(RectangleGeometry::with_rect(Rect::new(0.0, 0.0, 100.0, 100.0)));

    let target = Image::new();
    target.set_width(100.0);
    target.set_height(100.0);
    target.set_stretch(Stretch::Fill);
    target.set_source(Some(DrawingImage::with_drawing(drawing.clone()).into()));

    render_change(&t, &target, || drawing.set_brush(Some(Brushes::red())), "DrawingImage_Reflects_Replaced_Inner_Brush");
    t.compare_images_with(
        "DrawingImage_Reflects_Replaced_Inner_Brush",
        CompareOptions { skip_immediate: true, ..Default::default() },
    );
}

// Renders the target once, applies the change, renders again and writes the second frame to the
// standard composited output path so it can be compared against the committed reference image.
fn render_change(t: &TestBase, target: impl IntoRef<Control>, change: impl FnOnce(), test_name: &str) {
    let target: Ref<Control> = target.into_ref();
    let timer = Arc::new(ManualRenderTimer::new());
    let scheduler: Rc<dyn ICompositorScheduler> = Rc::new(DispatcherCompositorScheduler);
    let compositor = Compositor::with_scheduler(
        RenderLoop::from_timer(timer.clone()),
        None,
        true,
        &scheduler,
        Dispatcher::ui_thread(),
        None,
        None,
    );

    let root = TestRenderRoot::new(1.0);
    let size = PixelSize::new(100, 100);
    let row_bytes = 100 * 4;
    let mut frame_buffer = vec![0u8; (row_bytes * size.height) as usize];

    let surface: Arc<dyn IPlatformRenderSurface> =
        Arc::new(FuncFramebufferSurface { address: frame_buffer.as_mut_ptr() as usize, size, row_bytes });
    let surfaces: RenderSurfaces = Arc::new(move || vec![surface.clone()]);

    {
        let renderer = CompositingRenderer::new(&root.source(), &compositor, surfaces);
        root.initialize(&renderer, &target);
        renderer.start();
        Dispatcher::ui_thread().run_jobs(None);
        timer.trigger_tick();

        change();
        Dispatcher::ui_thread().run_jobs(None);
        timer.trigger_tick();
        renderer.dispose();
    }
    // What upstream leaves to the garbage collector.
    root.set_child(None);
    root.release();

    std::fs::create_dir_all(t.output_path()).expect("the output directory is created");
    let path = t.output_path().join(format!("{test_name}.composited.out.png"));
    let data = Bitmap::from_pixels(
        PixelFormat::RGBA8888,
        AlphaFormat::Premul,
        &frame_buffer,
        size,
        Vector::new(96.0, 96.0),
        row_bytes,
    );
    data.save_to_file(path.to_str().expect("the path is text"), &PngBitmapEncoderOptions::default().into())
        .expect("the image is saved");
    data.dispose();
}
