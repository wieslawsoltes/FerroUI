//! A window of the macOS platform drawn by the Vello backend: the platform
//! is brought up, a window is created through the windowing contract, and
//! frames of shapes are drawn into it through the render interface, the
//! backend context and the render target of the window.
//!
//! ```text
//! cargo run -p ferroui-vello --features hybrid,gpu --example vello_window                  # the first mode of the default order
//! cargo run -p ferroui-vello --features hybrid,gpu --example vello_window -- --mode gpu    # hybrid, gpu or cpu
//! cargo run -p ferroui-vello --example vello_window -- --software                          # the framebuffer of the platform, CPU mode
//! cargo run -p ferroui-vello --features hybrid,gpu --example vello_window -- --smoke       # draws its frames and exits
//! ```
//!
//! With `--smoke` the example draws a number of frames on a timer (the
//! shapes move from frame to frame), resizes the window half way, closes
//! it and exits; the exit code is 0 only if every frame was drawn, the
//! resize was seen and the window reported being closed. Without it the
//! window stays until it is closed.
//!
//! The Vello backend draws no text yet, so there is none.

#[cfg(not(target_os = "macos"))]
fn main() {
    eprintln!("vello_window: this example only runs on macOS");
}

#[cfg(target_os = "macos")]
fn main() -> std::process::ExitCode {
    macos::run()
}

#[cfg(target_os = "macos")]
mod macos {
    use ferroui_base::input::{FocusManager, IInputRoot, InputElement};
    use ferroui_base::media::immutable::{
        ImmutableGradientStop, ImmutableLinearGradientBrush, ImmutablePen, ImmutableSolidColorBrush,
    };
    use ferroui_base::media::{BoxShadows, Color, Colors, GradientSpreadMethod, PenLineCap, PenLineJoin};
    use ferroui_base::platform::{
        IPlatformGraphics, IPlatformGraphicsContext, IPlatformRenderInterface, IPlatformRenderInterfaceContext,
        IRenderTarget, RenderTargetSceneInfo,
    };
    use ferroui_base::rendering::composition::CompositionTransparencyLevel;
    use ferroui_base::threading::{CancellationTokenSource, Dispatcher, DispatcherPriority, DispatcherTimer};
    use ferroui_base::{
        FerroLocator, LocatorExtensions, Matrix, PixelPoint, PixelSize, Rect, Ref, RelativePoint, RelativeUnit,
        RoundedRect, Size,
    };
    use ferroui_controls::platform::{IWindowImpl, IWindowingPlatform};
    use ferroui_controls::WindowResizeReason;
    use ferroui_native::{FerroNativePlatform, FerroNativePlatformOptions, FerroNativeRenderingMode};
    use ferroui_vello::{VelloOptions, VelloPlatform, VelloRenderingMode};
    use std::cell::{Cell, RefCell};
    use std::process::ExitCode;
    use std::rc::Rc;
    use std::time::Duration;

    /// The frames a smoke run draws.
    const SMOKE_FRAMES: u32 = 12;

    /// The smallest input root: one input element and no focus manager.
    struct InputRoot {
        root: Ref<InputElement>,
        pointer_over_element: RefCell<Option<Ref<InputElement>>>,
        cursor_element: RefCell<Option<Ref<InputElement>>>,
    }

    impl IInputRoot for InputRoot {
        fn focus_manager(&self) -> Option<Rc<FocusManager>> {
            None
        }

        fn pointer_over_element(&self) -> Option<Ref<InputElement>> {
            self.pointer_over_element.borrow().clone()
        }

        fn set_pointer_over_element(&self, value: Option<Ref<InputElement>>) {
            let old = self.pointer_over_element.replace(value);
            drop(old);
        }

        fn cursor_element(&self) -> Option<Ref<InputElement>> {
            self.cursor_element.borrow().clone()
        }

        fn set_cursor_element(&self, value: Option<Ref<InputElement>>) {
            let old = self.cursor_element.replace(value);
            drop(old);
        }

        fn root_element(&self) -> Ref<InputElement> {
            self.root.clone()
        }

        fn focus_root(&self) -> Ref<InputElement> {
            self.root.clone()
        }

        fn pointer_over_invalidated(&self) {}
    }

    /// What the callbacks share.
    struct State {
        window: Rc<dyn IWindowImpl>,
        context: Rc<dyn IPlatformRenderInterfaceContext>,
        render_target: RefCell<Option<Rc<dyn IRenderTarget>>>,
        frames: Cell<u32>,
        resizes: Cell<u32>,
        closed: Cell<bool>,
    }

    impl State {
        /// Draws a frame of shapes; `frame` moves them.
        fn paint(&self) {
            if self.closed.get() {
                return;
            }
            let frame = self.frames.get();
            let client_size = self.window.client_size();
            let scaling = self.window.render_scaling();
            let pixel_size = PixelSize::from_size(client_size, scaling);
            if pixel_size.width <= 0 || pixel_size.height <= 0 {
                return;
            }

            let render_target = {
                let mut slot = self.render_target.borrow_mut();
                match slot.as_ref() {
                    Some(render_target) => render_target.clone(),
                    None => {
                        let render_target = self.context.create_render_target(&self.window.surfaces());
                        println!("Render target created: {:?}", render_target.properties());
                        *slot = Some(render_target.clone());
                        render_target
                    }
                }
            };

            let scene_info = RenderTargetSceneInfo::new(pixel_size, scaling, CompositionTransparencyLevel::None);
            let (mut context, _properties) = render_target.create_drawing_context(&scene_info);

            context.clear(Colors::WHITE);
            context.set_transform(Matrix::create_scale(scaling, scaling));

            let (width, height) = (client_size.width, client_size.height);
            let gradient = ImmutableLinearGradientBrush::new(
                &[
                    ImmutableGradientStop::new(0.0, Color::from_rgb(0x1e, 0x3a, 0x8a)),
                    ImmutableGradientStop::new(0.5, Color::from_rgb(0x7c, 0x3a, 0xed)),
                    ImmutableGradientStop::new(1.0, Color::from_rgb(0xf5, 0x9e, 0x0b)),
                ],
                1.0,
                None,
                None,
                GradientSpreadMethod::Pad,
                Some(RelativePoint::new(0.0, 0.0, RelativeUnit::Relative)),
                Some(RelativePoint::new(1.0, 1.0, RelativeUnit::Relative)),
                None,
            );
            context.draw_rectangle(
                Some(&gradient),
                None,
                RoundedRect::from_rect(Rect::new(0.0, 0.0, width, height)),
                &BoxShadows::default(),
            );

            // A card with rounded corners, clipped content and a layer of
            // half opacity inside it.
            let card = Rect::new(width * 0.15, height * 0.2, width * 0.7, height * 0.6);
            let fill = ImmutableSolidColorBrush::new(Color::from_argb(0xe6, 0xff, 0xff, 0xff));
            let outline = ImmutablePen::from_uint32(0xff11_1827, 3.0);
            context.draw_rectangle(Some(&fill), Some(&outline), RoundedRect::from_radius(card, 24.0), &BoxShadows::default());

            context.push_clip_rounded(RoundedRect::from_radius(card, 24.0));
            context.push_opacity(0.5, None);
            let step = f64::from(frame % SMOKE_FRAMES) / f64::from(SMOKE_FRAMES);
            let ellipse = Rect::new(card.x + card.width * step - 40.0, card.y + card.height * 0.5 - 40.0, 120.0, 80.0);
            context.draw_ellipse(Some(&ImmutableSolidColorBrush::new(Color::from_rgb(0x0e, 0x96, 0x88))), None, ellipse);
            context.draw_rectangle(
                Some(&ImmutableSolidColorBrush::new(Color::from_rgb(0xdc, 0x26, 0x26))),
                None,
                RoundedRect::from_rect(Rect::new(ellipse.x + 60.0, ellipse.y + 30.0, 90.0, 60.0)),
                &BoxShadows::default(),
            );
            context.pop_opacity();
            context.pop_clip();

            let dashed = ImmutablePen::new(
                Some(Rc::new(ImmutableSolidColorBrush::new(Color::from_rgb(0x11, 0x18, 0x27)))),
                4.0,
                None,
                PenLineCap::Round,
                PenLineJoin::Round,
                10.0,
            );
            context.draw_line(
                Some(&dashed),
                ferroui_base::Point::new(width * 0.15, height * 0.9),
                ferroui_base::Point::new(width * (0.15 + 0.7 * step), height * 0.9),
            );

            context.dispose();

            self.frames.set(frame + 1);
            if frame < 3 {
                println!(
                    "Frame #{frame}: {}x{} px (client {}x{}, scaling {scaling})",
                    pixel_size.width, pixel_size.height, client_size.width, client_size.height
                );
            }
        }
    }

    fn mode_argument() -> Option<VelloRenderingMode> {
        let arguments: Vec<String> = std::env::args().collect();
        let value = arguments.iter().position(|argument| argument == "--mode").and_then(|index| arguments.get(index + 1))?;
        match value.as_str() {
            "cpu" => Some(VelloRenderingMode::Cpu),
            "hybrid" => Some(VelloRenderingMode::Hybrid),
            "gpu" => Some(VelloRenderingMode::Gpu),
            other => panic!("--mode takes cpu, hybrid or gpu, not {other}"),
        }
    }

    pub fn run() -> ExitCode {
        let software = std::env::args().any(|argument| argument == "--software");
        let smoke = std::env::args().any(|argument| argument == "--smoke");
        let vello_options = mode_argument().map(VelloOptions::with_rendering_mode).unwrap_or_default();

        let options = FerroNativePlatformOptions {
            rendering_mode: if software {
                vec![FerroNativeRenderingMode::Software]
            } else {
                vec![FerroNativeRenderingMode::Metal]
            },
            ..FerroNativePlatformOptions::default()
        };
        let platform = FerroNativePlatform::initialize(options);
        platform.setup_application_name_with(Some("FerroUI Vello window"));
        VelloPlatform::initialize_with_options(vello_options);

        let locator = FerroLocator::current();
        let graphics = locator.get_service::<std::sync::Arc<dyn IPlatformGraphics>>();
        println!(
            "Platform: {}, platform graphics: {}, rendering modes in order: {:?}",
            if software { "software" } else { "metal" },
            graphics.is_some(),
            vello_options.rendering_mode_order()
        );
        if !software && graphics.is_none() {
            eprintln!("vello_window failed: Metal is not available");
            return ExitCode::FAILURE;
        }

        let graphics_context: Option<Rc<dyn IPlatformGraphicsContext>> =
            graphics.as_ref().map(|graphics| graphics.create_context());
        let render_interface = locator.get_required_service::<dyn IPlatformRenderInterface>();
        let context = render_interface.create_backend_context(graphics_context.clone());

        let windowing_platform = locator.get_required_service::<dyn IWindowingPlatform>();
        let window = windowing_platform.create_window();
        let state = Rc::new(State {
            window: window.clone(),
            context,
            render_target: RefCell::new(None),
            frames: Cell::new(0),
            resizes: Cell::new(0),
            closed: Cell::new(false),
        });
        let loop_cancellation = CancellationTokenSource::new();

        window.set_input_root(Rc::new(InputRoot {
            root: InputElement::new(),
            pointer_over_element: RefCell::new(None),
            cursor_element: RefCell::new(None),
        }));

        let weak = Rc::downgrade(&state);
        window.set_paint(Some(Rc::new(move |_rect| {
            if let Some(state) = weak.upgrade() {
                state.paint();
            }
        })));
        let weak = Rc::downgrade(&state);
        window.set_resized(Some(Rc::new(move |size: Size, reason: WindowResizeReason| {
            if let Some(state) = weak.upgrade() {
                state.resizes.set(state.resizes.get() + 1);
                println!("Resized: {}x{} ({reason:?})", size.width, size.height);
            }
        })));
        let weak = Rc::downgrade(&state);
        let cancel = loop_cancellation.clone();
        window.set_closed(Some(Rc::new(move || {
            println!("Closed");
            if let Some(state) = weak.upgrade() {
                state.closed.set(true);
                let render_target = state.render_target.borrow_mut().take();
                if let Some(render_target) = render_target {
                    render_target.dispose();
                }
            }
            cancel.cancel();
        })));

        window.set_title(Some("FerroUI Vello window"));
        window.resize(Size::new(640.0, 400.0), WindowResizeReason::Application);
        window.move_(PixelPoint::new(200, 200));
        window.show(true, false);
        println!("Window shown: client size {:?}, scaling {}", window.client_size(), window.render_scaling());

        // The frames of a smoke run: one every 40 ms, a resize half way,
        // and the window closed after the last.
        let smoke_frames = Rc::new(Cell::new(0u32));
        let _smoke_timer = smoke.then(|| {
            let (state, window, drawn) = (state.clone(), window.clone(), smoke_frames.clone());
            DispatcherTimer::run(
                move || {
                    if state.closed.get() {
                        return false;
                    }
                    if drawn.get() == SMOKE_FRAMES / 2 {
                        window.resize(Size::new(800.0, 520.0), WindowResizeReason::Application);
                    }
                    state.paint();
                    drawn.set(drawn.get() + 1);
                    if drawn.get() < SMOKE_FRAMES {
                        return true;
                    }
                    println!("Smoke frames drawn: closing the window");
                    window.dispose();
                    false
                },
                Duration::from_millis(40),
                DispatcherPriority::NORMAL,
            )
        });

        println!("Entering main loop");
        Dispatcher::ui_thread().main_loop(&loop_cancellation.token());
        println!("Main loop exited");

        let (frames, resizes, closed) = (state.frames.get(), state.resizes.get(), state.closed.get());
        println!("Summary: {frames} frame(s), {resizes} resize(s), closed={closed}, lost={}", state.context.is_lost());

        let lost = state.context.is_lost();
        state.context.dispose();
        drop(state);
        drop(window);
        if let Some(graphics_context) = graphics_context {
            graphics_context.dispose();
        }
        platform.dispose();

        if smoke && (smoke_frames.get() < SMOKE_FRAMES || frames < SMOKE_FRAMES || resizes == 0 || !closed || lost) {
            eprintln!("smoke run failed: expected {SMOKE_FRAMES} frames, a resize and a closed window on a device that was not lost");
            return ExitCode::FAILURE;
        }
        ExitCode::SUCCESS
    }
}
