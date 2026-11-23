//! Actual offscreen GPU drawing of the existing virtualization fixtures.
//! Window/input services stay synthetic; fonts, shaping and drawing are real.
use crate::virtualization_benchmarks::VirtualizationBenchmarks;
use ferroui_base::data::TemplateBinding;
use ferroui_base::{
    media::{
        imaging::BitmapEncoderOptions, immutable::ImmutableImageBrush, Colors, DrawingContext,
        FontFamily, ISceneBrushContent, PlatformDrawingContext,
    },
    platform::*,
    rendering::{composition::drawing::RenderDataDrawingContext, ImmediateRenderer},
    PixelSize, Rect, Ref, Vector, Visual,
};
use ferroui_controls::{
    templates::{FuncControlTemplate, IControlTemplate},
    TextBlock,
};
use ferroui_controls::{testing::TestServices, Window};
use ferroui_controls_treedatagrid::primitives::TreeDataGridTextCell;
use ferroui_opengl::IGlContext;
use serde_json::{json, Value};
use std::{
    any::TypeId,
    cell::RefCell,
    ffi::c_void,
    rc::Rc,
    time::{Duration, Instant},
};

extern "C" {
    fn tdg_completion_create(device: *mut c_void) -> *mut c_void;
    fn tdg_completion_wait(completion: *mut c_void) -> i32;
    fn tdg_completion_adapter_description(completion: *mut c_void, out: *mut u8, capacity: i32) -> i32;
    fn tdg_completion_destroy(completion: *mut c_void);
}
struct NativeCompletion(*mut c_void);
impl NativeCompletion {
    fn new() -> Self {
        let completion = unsafe { tdg_completion_create(ferroui_direct2d::native_device()) };
        assert!(!completion.is_null(), "D3D11 event query creation failed");
        Self(completion)
    }
    /// The description of the adapter the native device was created on, as DXGI reports it.
    fn adapter_description(&self) -> String {
        // 128 UTF-16 units at most: three bytes each cover them.
        let mut bytes = [0u8; 384];
        let length =
            unsafe { tdg_completion_adapter_description(self.0, bytes.as_mut_ptr(), bytes.len() as i32) };
        assert!(length > 0, "The D3D11 device did not report its adapter");
        String::from_utf8_lossy(&bytes[..length as usize]).into_owned()
    }
}
impl Drop for NativeCompletion {
    fn drop(&mut self) {
        unsafe { tdg_completion_destroy(self.0) }
    }
}
enum Driver {
    Skia {
        graphics: Rc<dyn IPlatformGraphicsContext>,
        gl: Rc<dyn IGlContext>,
        gr: RefCell<Option<Rc<dyn ferroui_skia::gpu::ISkiaGrContext>>>,
    },
    Direct2D(NativeCompletion),
}
struct Backend {
    name: String,
    renderer: Rc<dyn IPlatformRenderInterface>,
    context: Rc<dyn IPlatformRenderInterfaceContext>,
    driver: Driver,
    /// The adapter as the backend names it. The adapter LUID identifies a machine and is not kept.
    description: String,
}
impl Backend {
    fn new(name: &str) -> Self {
        let (renderer, context, driver, description): (Rc<dyn IPlatformRenderInterface>, _, _, _) =
            match name {
                "skia" => {
                    let graphics =
                        ferroui_win32::open_gl::angle::D3D11AngleWin32PlatformGraphics::new(
                            None, None,
                        )
                        .create_context();
                    let current = graphics.ensure_current();
                    let feature = graphics
                        .try_get_feature(TypeId::of::<dyn IGlContext>())
                        .expect("ANGLE GL context");
                    let gl = feature
                        .downcast_ref::<Rc<dyn IGlContext>>()
                        .unwrap()
                        .clone();
                    let description = gl.gl_interface().get_string(0x1f01).expect("GL_RENDERER");
                    let lower = description.to_ascii_lowercase();
                    assert!(
                        !["warp", "swiftshader", "llvmpipe", "microsoft basic"]
                            .iter()
                            .any(|s| lower.contains(s)),
                        "Software adapter: {description}"
                    );
                    let renderer: Rc<dyn IPlatformRenderInterface> =
                        Rc::new(ferroui_skia::PlatformRenderInterface::new(None, None));
                    let context = renderer.create_backend_context(Some(graphics.clone()));
                    current.dispose();
                    (
                        renderer,
                        context,
                        Driver::Skia {
                            graphics,
                            gl,
                            gr: RefCell::new(None),
                        },
                        description,
                    )
                }
                "direct2d" => {
                    let renderer: Rc<dyn IPlatformRenderInterface> =
                        Rc::new(ferroui_direct2d::Direct2DRenderInterface);
                    let context = renderer.create_backend_context(None);
                    let completion = NativeCompletion::new();
                    let adapter = completion.adapter_description();
                    assert!(
                        !adapter.to_ascii_lowercase().contains("microsoft basic"),
                        "Software adapter: {adapter}"
                    );
                    (
                        renderer,
                        context,
                        Driver::Direct2D(completion),
                        format!("{adapter} (D3D11 / Direct2D / DirectWrite)"),
                    )
                }
                _ => panic!("FERROUI_TDG_BACKEND must be skia or direct2d"),
            };
        Self {
            name: name.into(),
            renderer,
            context,
            driver,
            description,
        }
    }
    fn current(&self) -> Option<Rc<dyn ferroui_base::reactive::IDisposable>> {
        match &self.driver {
            Driver::Skia { graphics, .. } => Some(graphics.ensure_current()),
            _ => None,
        }
    }
    fn wait(&self) {
        match &self.driver {
            Driver::Skia { gl, .. } => gl.gl_interface().finish(),
            Driver::Direct2D(completion) => assert_eq!(
                unsafe { tdg_completion_wait(completion.0) },
                0,
                "D3D11 GPU completion failed"
            ),
        }
        assert!(!self.context.is_lost(), "GPU context lost");
    }
    fn submit_frame(&self, canvas: &mut dyn IDrawingContextImpl) {
        if let Driver::Skia { gr, .. } = &self.driver {
            if gr.borrow().is_none() {
                let feature = canvas
                    .get_feature(TypeId::of::<dyn ferroui_skia::ISkiaApiLeaseFeature>())
                    .expect("Skia API lease");
                let feature = feature
                    .downcast_ref::<Rc<dyn ferroui_skia::ISkiaApiLeaseFeature>>()
                    .unwrap();
                let lease = feature.lease();
                *gr.borrow_mut() = Some(lease.gr_context().expect("Skia must use a GPU context"));
                lease.dispose();
            }
            // Flush each frame, like the window render session. Otherwise a
            // later clear could discard earlier deferred offscreen frames.
            gr.borrow().as_ref().unwrap().flush();
        }
    }
    fn services(&self) -> TestServices {
        let services = TestServices::styled_window().with_render_interface(self.renderer.clone());
        match &self.driver {
            Driver::Skia { .. } => services
                .with_font_manager_impl(Rc::new(ferroui_skia::FontManagerImpl::new()))
                .with_text_shaper_impl(Rc::new(ferroui_harfbuzz::HarfBuzzTextShaper::new())),
            Driver::Direct2D(_) => services
                .with_font_manager_impl(Rc::new(ferroui_direct2d::Direct2DFontManager::default()))
                .with_text_shaper_impl(Rc::new(ferroui_direct2d::DirectWriteTextShaper)),
        }
    }
    fn counters(&self) -> Option<[u64; 12]> {
        matches!(self.driver, Driver::Direct2D(_)).then(ferroui_direct2d::statistics)
    }
}
impl Drop for Backend {
    fn drop(&mut self) {
        let current = self.current();
        self.context.dispose();
        if let Driver::Skia { graphics, gr, .. } = &self.driver {
            gr.borrow_mut().take();
            if let Some(current) = current {
                current.dispose();
            }
            graphics.dispose();
        }
    }
}

#[derive(Clone, Copy, Debug)]
enum Scenario {
    Stationary,
    VerticalSmall,
    VerticalRow,
    HorizontalSmall,
    HorizontalColumn,
    HorizontalFar,
}
impl Scenario {
    fn name(self) -> &'static str {
        match self {
            Self::Stationary => "stationary_redraw",
            Self::VerticalSmall => "vertical_small_scrolls",
            Self::VerticalRow => "vertical_row_scrolls",
            Self::HorizontalSmall => "horizontal_small_scrolls",
            Self::HorizontalColumn => "horizontal_column_scrolls",
            Self::HorizontalFar => "horizontal_far_column_scrolls",
        }
    }
    fn setup(self, fixture: &mut VirtualizationBenchmarks) {
        match self {
            Self::Stationary | Self::VerticalSmall | Self::VerticalRow => {
                fixture.setup_vertical_small_scrolls()
            }
            Self::HorizontalSmall => fixture.setup_horizontal_small_scrolls(),
            Self::HorizontalColumn => fixture.setup_horizontal_column_scrolls(),
            Self::HorizontalFar => fixture.setup_horizontal_far_column_scrolls(),
        }
        fixture
            .window()
            .set_font_family(FontFamily::new("Segoe UI"));
        fixture.window().set_font_size(12.);
        fixture.window().update_layout();
        assert!(fixture.realized_row_count() > 0);
    }
    fn offset(self, frame: usize) -> Vector {
        match self {
            Self::Stationary => Vector::ZERO,
            Self::VerticalSmall => Vector::new(0., (frame % 100_000) as f64),
            Self::VerticalRow => Vector::new(0., ((frame % 8_000) * 24) as f64),
            Self::HorizontalSmall => Vector::new((frame % 18_000) as f64, 0.),
            Self::HorizontalColumn => Vector::new(if frame % 2 == 0 { 100. } else { 200. }, 0.),
            Self::HorizontalFar => Vector::new(if frame % 2 == 0 { 90_000. } else { 90_100. }, 0.),
        }
    }
}
#[derive(Clone, Copy)]
enum DrawPath {
    Recorded,
    Immediate,
}
impl DrawPath {
    fn name(self) -> &'static str {
        match self {
            Self::Recorded => "recorded",
            Self::Immediate => "immediate",
        }
    }
}
fn draw_frame(
    backend: &Backend,
    target: &dyn IDrawingContextLayerImpl,
    window: &Ref<Window>,
    path: DrawPath,
) -> (Duration, Duration) {
    let mut canvas = target.create_drawing_context();
    canvas.clear(Colors::WHITE);
    let recording;
    let replay;
    let started = Instant::now();
    match path {
        DrawPath::Immediate => {
            let mut core = PlatformDrawingContext::borrowed(&mut *canvas);
            let mut drawing = DrawingContext::new(&mut core);
            ImmediateRenderer::render(&mut drawing, window.upcast_ref::<Visual>());
            drop(drawing);
            recording = Duration::ZERO;
            replay = started.elapsed();
        }
        DrawPath::Recorded => {
            // Use the production render-data recorder/replay, including its
            // elimination of identity transforms and opacity=1 scopes. Record
            // every frame: this does not claim retained-scene cache savings.
            let mut recorder = RenderDataDrawingContext::new(None);
            {
                let mut drawing = DrawingContext::new(&mut recorder);
                ImmediateRenderer::render(&mut drawing, window.upcast_ref::<Visual>());
            }
            let content = recorder
                .get_immediate_scene_brush_content(
                    Rc::new(ImmutableImageBrush::from_bitmap(None)),
                    Some(Rect::new(0., 0., 800., 500.)),
                    true,
                )
                .expect("The populated grid must record drawing commands");
            recording = started.elapsed();
            let began = Instant::now();
            content.render(&mut *canvas, None);
            content.dispose();
            replay = began.elapsed();
        }
    }
    backend.submit_frame(&mut *canvas);
    canvas.dispose();
    (recording, replay)
}
fn text_cell_template() -> Rc<dyn IControlTemplate> {
    FuncControlTemplate::for_type::<TreeDataGridTextCell>(|_, _| {
        let text = TextBlock::new();
        text.set_font_family(FontFamily::new("Segoe UI"));
        text.set_font_size(12.);
        text.set_foreground(Some(ferroui_base::media::Brushes::black()));
        text.bind_binding(
            TextBlock::text_property().as_property(),
            &TemplateBinding::new(TreeDataGridTextCell::value_property().as_property()),
        );
        text.upcast()
    })
}

fn env_count(name: &str, default: usize) -> usize {
    let value = std::env::var(name).map_or(default, |v| v.parse().expect(name));
    assert!(value > 0);
    value
}
fn stats(samples: &[f64]) -> Value {
    let mean = samples.iter().sum::<f64>() / samples.len() as f64;
    let sd = (samples.iter().map(|v| (v - mean).powi(2)).sum::<f64>()
        / (samples.len() - 1).max(1) as f64)
        .sqrt();
    let mut sorted = samples.to_vec();
    sorted.sort_by(f64::total_cmp);
    json!({"mean":mean,"sd":sd,"median":sorted[sorted.len()/2],"p95":sorted[((sorted.len()-1) as f64*0.95).ceil() as usize],"samples":samples})
}

#[test]
#[ignore = "Real Windows GPU TDG measurements; run alone with --release --test-threads=1"]
fn run_renderer_benchmarks() {
    assert!(
        !cfg!(feature = "count-allocations"),
        "Timing runs must disable allocation counting"
    );
    let name = std::env::var("FERROUI_TDG_BACKEND").expect("FERROUI_TDG_BACKEND=skia or direct2d");
    // Checked before anything is measured: a run must not end without a place for its results. The
    // empty file is removed again, so a failed run leaves none.
    let output =
        std::env::var("FERROUI_TDG_RESULTS_JSON").expect("FERROUI_TDG_RESULTS_JSON output path");
    if let Err(error) = std::fs::File::create(&output).map(drop).and_then(|()| std::fs::remove_file(&output)) {
        panic!("FERROUI_TDG_RESULTS_JSON={output} cannot be written: {error}");
    }
    let iterations = env_count("FERROUI_BENCH_ITERATIONS", 15);
    let warmup = env_count("FERROUI_BENCH_WARMUP", 3);
    let frames = env_count("FERROUI_TDG_FRAMES_PER_BATCH", 20);
    let path = match std::env::var("FERROUI_TDG_DRAW_PATH").as_deref() {
        Ok("immediate") => DrawPath::Immediate,
        Ok("recorded") | Err(_) => DrawPath::Recorded,
        _ => panic!("FERROUI_TDG_DRAW_PATH must be recorded or immediate"),
    };
    let backend = Backend::new(&name);
    println!("TDG_GPU_BACKEND {} {}", name, backend.description);
    let mut rows = Vec::new();
    for neutral in [false, true] {
        for scenario in [
            Scenario::Stationary,
            Scenario::VerticalSmall,
            Scenario::VerticalRow,
            Scenario::HorizontalSmall,
            Scenario::HorizontalColumn,
            Scenario::HorizontalFar,
        ] {
            if std::env::var("FERROUI_BENCH_FILTER").is_ok_and(|f| {
                !f.split(',')
                    .any(|part| scenario.name().contains(part.trim()))
            }) {
                continue;
            }
            let current = backend.current();
            let mut fixture = VirtualizationBenchmarks::with_services(neutral, backend.services());
            fixture.set_renderer_cell_template(text_cell_template());
            scenario.setup(&mut fixture);
            let text_cells = fixture
                .window()
                .upcast_ref::<Visual>()
                .get_visual_descendants()
                .filter_map(|v| v.cast::<TextBlock>())
                .filter(|t| t.text().is_some_and(|s| !s.is_empty()) && t.bounds().width > 0.)
                .count();
            assert!(text_cells > 0, "The fixture must draw actual cell text");
            let target = backend.context.create_offscreen_render_target(
                PixelSize::new(800, 500),
                Vector::new(1., 1.),
                true,
            );
            let mut frame = 0;
            let mut total_samples = Vec::new();
            let mut layout_samples = Vec::new();
            let mut submit_samples = Vec::new();
            let mut recording_samples = Vec::new();
            let mut replay_samples = Vec::new();
            let _ = draw_frame(&backend, &*target, fixture.window(), path);
            backend.wait();
            if let Ok(directory) = std::env::var("FERROUI_TDG_CAPTURE_DIR") {
                std::fs::create_dir_all(&directory).unwrap();
                let path = std::path::Path::new(&directory).join(format!(
                    "{}-{}-neutral-{}.png",
                    backend.name,
                    scenario.name(),
                    neutral
                ));
                let image = target.create_shared_snapshot();
                image
                    .save(
                        &mut std::fs::File::create(path).unwrap(),
                        &BitmapEncoderOptions::Png(Default::default()),
                    )
                    .unwrap();
            }
            let counters = backend.counters();
            let profile_before = ferroui_direct2d::profiling_statistics();
            for batch in 0..warmup + iterations {
                backend.wait();
                let began = Instant::now();
                let mut layout = Duration::ZERO;
                let mut recording = Duration::ZERO;
                let mut replay = Duration::ZERO;
                for _ in 0..frames {
                    let start = Instant::now();
                    fixture.scroll().set_offset(scenario.offset(frame));
                    fixture.window().update_layout();
                    layout += start.elapsed();
                    let (recorded, replayed) =
                        draw_frame(&backend, &*target, fixture.window(), path);
                    recording += recorded;
                    replay += replayed;
                    frame += 1;
                }
                let submitted = began.elapsed();
                backend.wait();
                let total = began.elapsed();
                if batch >= warmup {
                    total_samples.push(total.as_secs_f64() * 1000. / frames as f64);
                    layout_samples.push(layout.as_secs_f64() * 1000. / frames as f64);
                    submit_samples.push(submitted.as_secs_f64() * 1000. / frames as f64);
                    recording_samples.push(recording.as_secs_f64() * 1000. / frames as f64);
                    replay_samples.push(replay.as_secs_f64() * 1000. / frames as f64);
                }
            }
            if let (Some(before), Some(after)) = (counters, backend.counters()) {
                assert_eq!(
                    after[4], before[4],
                    "Timed drawing read pixels back to the CPU"
                );
                assert_eq!(after[8], before[8], "Native renderer errors during timing");
            }
            println!(
                "TDG_GPU_RESULT {} neutral={} mean_ms={:.6}",
                scenario.name(),
                neutral,
                total_samples.iter().sum::<f64>() / iterations as f64
            );
            let profile_after = ferroui_direct2d::profiling_statistics();
            let profile: Vec<_> = (0..profile_after.0.len()).map(|i| json!({"nanoseconds":profile_after.0[i]-profile_before.0[i],"calls":profile_after.1[i]-profile_before.1[i]})).collect();
            rows.push(json!({"scenario":scenario.name(),"neutral_source":neutral,"realized_rows":fixture.realized_row_count(),"text_cells":text_cells,"native_profile":profile,
                "total_ms_per_frame":stats(&total_samples),"layout_ms_per_frame":stats(&layout_samples),
                "submission_ms_per_frame":stats(&submit_samples),"recording_ms_per_frame":stats(&recording_samples),"replay_ms_per_frame":stats(&replay_samples)}));
            target.dispose();
            fixture.global_cleanup();
            drop(fixture);
            if let Some(current) = current {
                current.dispose();
            }
        }
    }
    assert!(!rows.is_empty());
    let report = json!({"backend":name,"draw_path":path.name(),"driver":backend.description,
        "viewport_pixels":[800,500],"scale":1,"font":"Segoe UI","font_size":12,"warmup_batches":warmup,
        "measured_batches":iterations,"frames_per_batch":frames,"gpu_completion":"wait once per batch; no presentation/vsync",
        "rows":rows});
    std::fs::write(output, serde_json::to_string_pretty(&report).unwrap()).unwrap();
}
