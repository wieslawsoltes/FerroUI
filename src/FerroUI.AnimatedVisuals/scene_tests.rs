//! The composition scene in pixels: a real compositor draws it through the Vello backend into a
//! bitmap, and each frame is compared with the same frame drawn command by command, the
//! reference of these tests.

use super::*;
use crate::icons::ICON_DOCUMENTS;
use crate::{FrameCommands, VisualRenderer};
use ferroui_base::layout::ILayoutManager;
use ferroui_base::media::{DrawingContext, PlatformDrawingContext};
use ferroui_base::platform::surfaces::{FuncFramebufferRenderTarget, IFramebufferPlatformSurface, IFramebufferRenderTarget, IPlatformRenderSurface};
use ferroui_base::platform::{AlphaFormat, IPlatformRenderInterface, IRenderTargetBitmapImpl, IWriteableBitmapImpl, PixelFormat};
use ferroui_base::rendering::composition::ElementComposition;
use ferroui_base::rendering::testing::ManualRenderLoop;
use ferroui_base::threading::Dispatcher;
use ferroui_base::{FerroLocator, LocatorExtensions, PixelSize};
use ferroui_controls::testing::{CompositorTestServices, MockWindowImpl, TestServices, UnitTestApplication};
use ferroui_controls::presentation_source::IRendererFactory;
use ferroui_controls::{Border, Control, Window};
use ferroui_vello::{PlatformRenderInterface, RenderTargetBitmapImpl, VelloRenderingMode};
use serde_json::{json, Value};
use std::any::Any;
use std::sync::Arc;

const SIZE: i32 = 48;
/// The error allowed between a frame and its reference: that of the render tests of the backends.
const ALLOWED_ERROR: f64 = 0.022;

struct BitmapSurface(Arc<dyn IWriteableBitmapImpl>);

impl IPlatformRenderSurface for BitmapSurface {
    fn as_framebuffer_surface(&self) -> Option<&dyn IFramebufferPlatformSurface> {
        Some(self)
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl IFramebufferPlatformSurface for BitmapSurface {
    fn create_framebuffer_render_target(&self) -> Rc<dyn IFramebufferRenderTarget> {
        let bitmap = self.0.clone();
        Rc::new(FuncFramebufferRenderTarget::new(move || bitmap.lock()))
    }
}

/// A window whose frames go into a bitmap, with a real compositor.
struct Host {
    bitmap: Arc<dyn IWriteableBitmapImpl>,
    render_loop: Arc<ManualRenderLoop>,
    compositor: Rc<Compositor>,
    element: Ref<Border>,
    _window: Ref<Window>,
}

impl Host {
    fn new() -> Self {
        let interface = FerroLocator::current()
            .get_service::<dyn IPlatformRenderInterface>()
            .expect("the backend of the test");
        let bitmap = interface.create_writeable_bitmap(
            PixelSize::new(SIZE, SIZE),
            Vector::new(96.0, 96.0),
            PixelFormat::RGBA8888,
            AlphaFormat::Premul,
        );
        let render_loop = ManualRenderLoop::new();
        FerroLocator::current_mutable().bind::<dyn IRendererFactory>().to_func(|| None);
        let compositor = CompositorTestServices::create_dummy_compositor(Some(render_loop.clone()));
        let window_impl = MockWindowImpl::window(f64::from(SIZE), f64::from(SIZE));
        window_impl.setup_compositor(Some(compositor.clone()));
        window_impl.setup_surfaces(vec![Arc::new(BitmapSurface(bitmap.clone()))]);
        let window = Window::with_impl(window_impl);
        let element = Border::new();
        window.set_content(Some(Control::boxed(&element)));
        window.show();
        window.layout_manager().execute_initial_layout_pass();
        Self { bitmap, render_loop, compositor, element, _window: window }
    }

    fn show(&self, scene: &CompositionScene) {
        ElementComposition::set_element_child_visual(&self.element, Some(scene.root().clone()));
        self.element.invalidate_visual();
        self.tick();
        scene.refresh();
        self.tick();
    }

    fn tick(&self) {
        Dispatcher::ui_thread().run_jobs(None);
        self.render_loop.tick();
        Dispatcher::ui_thread().run_jobs(None);
        self.render_loop.tick();
    }

    /// Layout, the loaded event and the first commits.
    fn settle(&self) {
        for _ in 0..3 {
            self._window.layout_manager().execute_layout_pass();
            self.tick();
        }
    }

    fn pixels(&self) -> Vec<u8> {
        let framebuffer = self.bitmap.lock();
        let row = framebuffer.row_bytes() as usize;
        let mut out = vec![0u8; (SIZE * SIZE * 4) as usize];
        framebuffer.with_data(&mut |data| {
            for y in 0..SIZE as usize {
                let width = SIZE as usize * 4;
                out[y * width..(y + 1) * width].copy_from_slice(&data[y * row..y * row + width]);
            }
        });
        framebuffer.dispose();
        out
    }
}

/// The frame drawn command by command, the reference.
fn reference(visual: &AnimatedVisual, progress: f64) -> Vec<u8> {
    let target: Arc<dyn IRenderTargetBitmapImpl> = Arc::new(RenderTargetBitmapImpl::new(
        PixelSize::new(SIZE, SIZE),
        Vector::new(96.0, 96.0),
        vec![VelloRenderingMode::Cpu],
    ));
    let mut commands = FrameCommands::default();
    visual.evaluate(visual.frame_at(progress), &mut commands);
    let mut renderer = VisualRenderer::new();
    let mut context = DrawingContext::owned(Box::new(PlatformDrawingContext::new(target.create_drawing_context())));
    renderer.set_color("Foreground", Some(Color::from_rgb(0, 0, 0)));
    renderer.render(&commands, visual.width, visual.height, &mut context);
    context.dispose();
    let framebuffer = target.lock();
    let row = framebuffer.row_bytes() as usize;
    let mut out = vec![0u8; (SIZE * SIZE * 4) as usize];
    framebuffer.with_data(&mut |data| {
        for y in 0..SIZE as usize {
            let width = SIZE as usize * 4;
            out[y * width..(y + 1) * width].copy_from_slice(&data[y * row..y * row + width]);
        }
    });
    framebuffer.dispose();
    out
}

/// Premultiplied `top` over `under`.
fn over(top: &[u8], under: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(top.len());
    for (top, under) in top.chunks_exact(4).zip(under.chunks_exact(4)) {
        let rest = 1.0 - f64::from(top[3]) / 255.0;
        out.extend((0..4).map(|channel| (f64::from(top[channel]) + f64::from(under[channel]) * rest).round().min(255.0) as u8));
    }
    out
}

fn rms(a: &[u8], b: &[u8]) -> f64 {
    let sum: f64 = a.iter().zip(b).map(|(a, b)| (f64::from(*a) - f64::from(*b)).powi(2)).sum();
    (sum / a.len() as f64).sqrt() / 255.0
}

/// Every other pixel of a frame as a character: its strongest channel with its alpha.
fn picture(pixels: &[u8]) -> String {
    let mut text = String::new();
    for y in (0..SIZE as usize).step_by(2) {
        for x in (0..SIZE as usize).step_by(2) {
            let pixel = &pixels[(y * SIZE as usize + x) * 4..(y * SIZE as usize + x) * 4 + 4];
            text.push(match (pixel[3], pixel[0], pixel[1], pixel[2]) {
                (0, ..) => '.',
                (a, ..) if a < 128 => ':',
                (_, r, g, b) if r > g && r > b => 'R',
                (_, r, g, b) if g > r && g > b => 'G',
                (_, r, g, b) if b > r && b > g => 'B',
                (_, r, ..) if r < 128 => 'X',
                _ => '#',
            });
        }
        text.push('\n');
    }
    text
}

fn start() -> ferroui_controls::testing::UnitTestApplicationScope {
    UnitTestApplication::start(
        TestServices::styled_window().with_render_interface(Rc::new(PlatformRenderInterface::default())),
    )
}

fn assert_frames(name: &str, visual: &Rc<AnimatedVisual>, progresses: &[f64]) {
    let host = Host::new();
    host.tick();
    // What the window shows under the scene.
    let under = host.pixels();
    let mut scene = CompositionScene::build(&host.compositor, visual);
    host.show(&scene);
    for progress in progresses {
        scene.set_progress(*progress);
        host.tick();
        let (shown, expected) = (host.pixels(), over(&reference(visual, *progress), &under));
        let error = rms(&shown, &expected);
        assert!(
            error <= ALLOWED_ERROR,
            "{name} at {progress}: error {error:.4}\nshown:\n{}expected:\n{}",
            picture(&shown),
            picture(&expected)
        );
    }
}

fn document(layers: Value) -> Rc<AnimatedVisual> {
    let text = json!({"fr": 30, "ip": 0, "op": 30, "w": SIZE, "h": SIZE, "layers": layers}).to_string();
    Rc::new(AnimatedVisual::parse(&text).unwrap())
}

fn square(x: f64, y: f64, side: f64, color: [f64; 3]) -> Value {
    json!([
        {"ty": "rc", "p": {"k": [x, y]}, "s": {"k": [side, side]}, "r": {"k": 0}},
        {"ty": "fl", "c": {"k": [color[0], color[1], color[2], 1]}, "o": {"k": 100}}
    ])
}

#[test]
fn a_moving_layer_is_an_animation_and_no_path_is_written() {
    let _app = start();
    let visual = document(json!([{"ty": 4, "ind": 1, "ip": 0, "op": 30,
        "ks": {"p": {"k": [{"t": 0, "s": [10, 20], "e": [30, 20]}, {"t": 30}]}},
        "shapes": square(0.0, 0.0, 10.0, [1.0, 0.0, 0.0])}]));
    let host = Host::new();
    let scene = CompositionScene::build(&host.compositor, &visual);
    assert_eq!(scene.animation_count(), 1, "the offset");
    assert_eq!(scene.live_value_count(), 0, "nothing is written per frame");
}

#[test]
fn a_moving_layer_matches_its_reference() {
    let _app = start();
    let visual = document(json!([{"ty": 4, "ind": 1, "ip": 0, "op": 30,
        "ks": {"p": {"k": [{"t": 0, "s": [10, 20], "e": [30, 20]}, {"t": 30}]}},
        "shapes": square(0.0, 0.0, 10.0, [1.0, 0.0, 0.0])}]));
    assert_frames("moving layer", &visual, &[0.0, 0.25, 0.5, 1.0]);
}

#[test]
fn every_winui_icon_matches_its_reference_at_every_marker() {
    let _app = start();
    for (name, json) in ICON_DOCUMENTS {
        let visual = Rc::new(AnimatedVisual::parse(json).unwrap());
        let mut progresses: Vec<f64> = visual.progress_markers().into_iter().map(|(_, progress)| progress).collect();
        progresses.sort_by(f64::total_cmp);
        progresses.dedup();
        assert_frames(name, &visual, &progresses);
    }
}

#[test]
fn each_kind_of_matte_matches_its_reference() {
    let _app = start();
    for kind in 1..=4 {
        let matte = square(14.0, 24.0, 20.0, [1.0, 1.0, 1.0]);
        let black = square(34.0, 24.0, 20.0, [0.0, 0.0, 0.0]);
        let visual = document(json!([
            {"ty": 4, "ind": 1, "td": 1, "ip": 0, "op": 30, "ks": {}, "shapes": [
                {"ty": "gr", "it": [matte[0], matte[1], {"ty": "tr"}]},
                {"ty": "gr", "it": [black[0], black[1], {"ty": "tr"}]},
            ]},
            {"ty": 4, "ind": 2, "tt": kind, "ip": 0, "op": 30, "ks": {}, "shapes": square(24.0, 24.0, 48.0, [0.0, 0.0, 1.0])}
        ]));
        assert_frames(&format!("matte {kind}"), &visual, &[0.0]);
    }
}

fn red_center(pixels: &[u8]) -> Option<(f64, f64)> {
    let size = SIZE as usize;
    let red: Vec<(usize, usize)> = (0..size)
        .flat_map(|y| (0..size).map(move |x| (x, y)))
        .filter(|(x, y)| pixels[(y * size + x) * 4] > 200 && pixels[(y * size + x) * 4 + 1] < 60)
        .collect();
    (!red.is_empty()).then(|| {
        let count = red.len() as f64;
        (red.iter().map(|p| p.0 as f64 + 0.5).sum::<f64>() / count, red.iter().map(|p| p.1 as f64 + 0.5).sum::<f64>() / count)
    })
}

#[test]
fn a_player_on_screen_shows_its_visual_through_the_scene_at_each_progress() {
    let _app = start();
    let json = json!({"fr": 30, "ip": 0, "op": 30, "w": SIZE, "h": SIZE, "layers": [{"ty": 4, "ind": 1, "ip": 0, "op": 30,
        "ks": {"p": {"k": [{"t": 0, "s": [10, 20], "e": [30, 20]}, {"t": 30}]}},
        "shapes": square(0.0, 0.0, 10.0, [1.0, 0.0, 0.0])}]}).to_string();
    let host = Host::new();
    let player = crate::AnimatedVisualPlayer::new();
    player.set_auto_play(false);
    player.set_source(Some(crate::LottieVisualSource::create_from_string(&json).upcast()));
    host.element.set_child(Some(player.clone().upcast()));
    host.settle();
    assert!(player.is_animated_visual_loaded());
    let at = |progress: f64| {
        player.set_progress(progress);
        host.tick();
        red_center(&host.pixels()).expect("the square is on screen")
    };
    for (progress, x) in [(0.0, 10.0), (0.5, 20.0), (1.0, 30.0)] {
        let center = at(progress);
        assert!((center.0 - x).abs() < 0.6 && (center.1 - 20.0).abs() < 0.6, "at {progress}: {center:?}");
    }
}

#[test]
fn each_merge_mode_keeps_its_part_of_two_shapes() {
    let _app = start();
    let rectangle = |x: f64| json!({"ty": "rc", "p": {"k": [x, 24]}, "s": {"k": [20, 20]}, "r": {"k": 0}});
    // What each mode keeps of: the first shape alone, the overlap, the second shape alone.
    for (mode, first, both, second) in
        [(1, true, true, true), (2, true, true, true), (3, true, false, false), (4, false, true, false), (5, true, false, true)]
    {
        let visual = document(json!([{"ty": 4, "ind": 1, "ip": 0, "op": 30, "ks": {}, "shapes": [{"ty": "gr", "it": [
            rectangle(18.0),
            rectangle(28.0),
            {"ty": "mm", "mm": mode},
            {"ty": "fl", "c": {"k": [1, 0, 0, 1]}, "o": {"k": 100}},
            {"ty": "tr"}
        ]}]}]));
        assert_frames(&format!("merge {mode}"), &visual, &[0.0]);
        let host = Host::new();
        let scene = CompositionScene::build(&host.compositor, &visual);
        host.show(&scene);
        let pixels = host.pixels();
        let red = |x: usize| {
            let at = (24 * SIZE as usize + x) * 4;
            pixels[at] > 200 && pixels[at + 1] < 60
        };
        assert_eq!((red(12), red(23), red(34)), (first, both, second), "merge mode {mode}");
    }
}

/// The same frames through the compositor on the device, in each GPU mode of Vello, against the
/// same reference. Without an adapter there is nothing to run them on.
#[cfg(feature = "device-tests")]
#[test]
fn mattes_and_icons_match_their_references_on_the_device() {
    if let Err(error) = ferroui_vello::gpu::VelloWgpuDevice::headless() {
        println!("skipped: no adapter: {error}");
        return;
    }
    for mode in [VelloRenderingMode::Hybrid, VelloRenderingMode::Gpu] {
        let options = ferroui_vello::VelloOptions { rendering_modes: [Some(mode), None, None], ..Default::default() };
        let _app = UnitTestApplication::start(
            TestServices::styled_window().with_render_interface(Rc::new(PlatformRenderInterface::new(options))),
        );
        for kind in 1..=4 {
            let matte = square(14.0, 24.0, 20.0, [1.0, 1.0, 1.0]);
            let black = square(34.0, 24.0, 20.0, [0.0, 0.0, 0.0]);
            let visual = document(json!([
                {"ty": 4, "ind": 1, "td": 1, "ip": 0, "op": 30, "ks": {}, "shapes": [
                    {"ty": "gr", "it": [matte[0], matte[1], {"ty": "tr"}]},
                    {"ty": "gr", "it": [black[0], black[1], {"ty": "tr"}]},
                ]},
                {"ty": 4, "ind": 2, "tt": kind, "ip": 0, "op": 30, "ks": {}, "shapes": square(24.0, 24.0, 48.0, [0.0, 0.0, 1.0])}
            ]));
            assert_frames(&format!("{mode:?}, matte {kind}"), &visual, &[0.0]);
        }
        for (name, json) in ICON_DOCUMENTS {
            let visual = Rc::new(AnimatedVisual::parse(json).unwrap());
            let mut progresses: Vec<f64> = visual.progress_markers().into_iter().map(|(_, progress)| progress).collect();
            progresses.sort_by(f64::total_cmp);
            progresses.dedup();
            assert_frames(&format!("{mode:?}, {name}"), &visual, &progresses);
        }
    }
}
