//! Opt-in desktop timing with one rendered composition batch per scene step.
use crate::{Clock, Launch};
use ferroui_base::media::imaging::{BitmapEncoderOptions, PngBitmapEncoderOptions};
use ferroui_base::rendering::composition::ElementComposition;
use ferroui_base::threading::{Dispatcher, DispatcherPriority};
use ferroui_base::{Ref, Size};
use ferroui_controls::{Application, TopLevel, Window};
use serde_json::json;
use std::{cell::RefCell, sync::OnceLock, time::Instant};

static START: OnceLock<Instant> = OnceLock::new();

pub fn start() {
    let _ = START.set(Instant::now());
}

struct Measurement {
    frames: usize,
    warmup: i32,
    last: Option<Instant>,
    first_callback_ms: f64,
    samples: Vec<f64>,
    done: bool,
    pointer_over_frames: usize,
    viewport: Option<(Size, f64)>,
    viewport_changed: bool,
    #[cfg(feature = "direct2d")]
    initial_presents: u64,
}
impl Measurement {
    fn from_environment() -> Option<Self> {
        let frames = std::env::var("PERF_MEASURE_FRAMES")
            .ok()?
            .parse::<usize>()
            .expect("PERF_MEASURE_FRAMES must be an integer");
        assert!(frames > 0);
        let warmup = std::env::var("PERF_WARMUP_FRAMES").map_or(60, |v| {
            v.parse::<i32>()
                .expect("PERF_WARMUP_FRAMES must be an integer")
        });
        assert!(warmup >= 0);
        assert_eq!(
            Launch::get().freeze,
            0,
            "Fixed-frame measurement cannot also freeze"
        );
        Some(Self {
            frames,
            warmup,
            last: None,
            first_callback_ms: START.get_or_init(Instant::now).elapsed().as_secs_f64() * 1000.,
            samples: Vec::with_capacity(frames),
            done: false,
            pointer_over_frames: 0,
            viewport: None,
            viewport_changed: false,
            #[cfg(feature = "direct2d")]
            initial_presents: 0,
        })
    }
}
ferroui_base::ferro_thread_local! {
    static RUN: RefCell<Option<Measurement>> = RefCell::new(Measurement::from_environment());
    static NEXT_SCENE: RefCell<Option<Box<dyn FnOnce()>>> = RefCell::new(None);
}

pub(crate) fn request_next_scene(top_level: &TopLevel, next: impl FnOnce() + 'static) {
    if !RUN.with_borrow(Option::is_some) {
        top_level.request_animation_frame(move |_| next());
        return;
    }
    NEXT_SCENE.with_borrow_mut(|pending| {
        assert!(pending.is_none());
        *pending = Some(Box::new(next));
    });
    let visual = ElementComposition::get_element_visual(top_level).expect("The window composition visual");
    let batch = visual.compositor().request_commit_async();
    let dispatcher = Dispatcher::ui_thread();
    // Acceptance can precede the render pass and allows it to coalesce batches.
    // Advance the scene on the UI thread after this batch has rendered.
    batch.rendered().on_completed(move || {
        dispatcher.post(
            || {
                let next = NEXT_SCENE.with_borrow_mut(Option::take).expect("The pending scene");
                next();
            },
            DispatcherPriority::RENDER,
        );
    });
}

fn window() -> Ref<Window> {
    let app = Application::current().expect("The benchmark application");
    let lifetime = app.application_lifetime().expect("The desktop lifetime");
    lifetime
        .as_classic_desktop_style_application_lifetime()
        .unwrap()
        .main_window()
        .expect("The benchmark window")
}
fn close_window() {
    window().close();
}

/// Called before advancing the scene index, after the preceding batch rendered.
pub(crate) fn on_frame() -> bool {
    RUN.with_borrow_mut(|run| {
        let Some(run) = run else { return false; };
        if run.done { return true; }
        if Clock::frame() < run.warmup { return false; }
        let now = Instant::now();
        let window = window();
        let client = window.client_size();
        let scaling = window.render_scaling();
        let initial_viewport = *run.viewport.get_or_insert((client, scaling));
        run.viewport_changed |= initial_viewport != (client, scaling);
        if let Some(previous) = run.last.replace(now) {
            run.samples.push(now.duration_since(previous).as_secs_f64()*1000.);
            if window.is_pointer_over() { run.pointer_over_frames += 1; }
        } else {
            #[cfg(feature = "direct2d")]
            { run.initial_presents = ferroui_direct2d::statistics()[3]; }
        }
        if run.samples.len() < run.frames { return false; }
        run.done = true;
        let elapsed_ms: f64 = run.samples.iter().sum();
        let mean = elapsed_ms/run.frames as f64;
        let mut sorted = run.samples.clone();
        sorted.sort_by(f64::total_cmp);
        #[cfg(feature="direct2d")]
        let native = Some(ferroui_direct2d::statistics());
        #[cfg(not(feature="direct2d"))]
        let native: Option<[u64;12]> = None;
        #[cfg(feature = "direct2d")]
        let measured_presents = native.map(|stats| stats[3] - run.initial_presents);
        #[cfg(not(feature = "direct2d"))]
        let measured_presents: Option<u64> = None;
        let result = json!({
            "backend": if cfg!(feature="direct2d") {"direct2d"} else {"skia-angle"},
            "scenario": if Launch::get().wall {"wall"} else {"gauntlet"}, "tier": Launch::get().tier,
            "metric": "desktop scene intervals gated by rendered composition batches",
            "warmup_frames":run.warmup,"measured_frames":run.frames,"last_scene_frame":Clock::frame(),
            "elapsed_ms":elapsed_ms,"mean_ms":mean,"callback_fps":1000./mean,
            "median_ms":sorted[sorted.len()/2],"p95_ms":sorted[((sorted.len()-1) as f64*0.95).ceil() as usize],
            "first_callback_ms":run.first_callback_ms,"client_width":client.width,"client_height":client.height,
            "render_scaling":scaling,"viewport_changed_during_measurement":run.viewport_changed,
            "samples_ms":run.samples,
            "pointer_over_frames":run.pointer_over_frames,
            "native_measured_presents":measured_presents,
            "native_statistics_including_startup_and_warmup":native,
        });
        if let Ok(path) = std::env::var("PERF_RESULTS_JSON") {
            std::fs::write(path,serde_json::to_vec_pretty(&result).unwrap()).expect("Write benchmark results");
        }
        println!("PERF_RESULT {result}");
        if let Ok(path) = std::env::var("PERF_CAPTURE_PATH") {
            // Capture after timing has stopped, from the real composition tree.
            let visual = ElementComposition::get_element_visual(&window).expect("The window composition visual");
            let compositor = visual.compositor();
            let task = compositor.create_composition_visual_snapshot(&visual,window.render_scaling());
            compositor.commit();
            let pending = task.clone();
            task.on_completed(move || {
                let bitmap = pending.take_result().expect("Snapshot completed").expect("Composition snapshot");
                bitmap.save_to_file(&path,&BitmapEncoderOptions::Png(PngBitmapEncoderOptions::DEFAULT)).expect("Save composition capture");
                Dispatcher::ui_thread().post(close_window,DispatcherPriority::BACKGROUND);
            });
        } else { window.close(); }
        true
    })
}
