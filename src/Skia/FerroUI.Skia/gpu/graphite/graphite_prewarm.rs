//! The prewarm of the GPU pipelines of Graphite off the frames (#437).
//!
//! Graphite makes the pipeline of a draw when the recording that holds the
//! draw is made (`Recorder::snap`, `DrawPass::prepareResources`, through
//! `SharedContext::findOrCreateGraphicsPipeline`), on the thread that snaps,
//! and keeps it in the global cache of the shared context, which every
//! recorder of the context reads. With a cold Metal shader cache a pipeline
//! takes tens of milliseconds: the first frame that drew the acrylic of the
//! WinUI theme stalled 160 to 455 ms.
//!
//! A prewarm records the draws onto surfaces of a recorder of its own, on
//! the render thread (drawing is cheap; the drawing context is the one of
//! the frames), and hands the recorder to a thread of its own, which snaps
//! it: the pipelines compile there, and the recording is dropped without
//! being inserted, so nothing of it reaches the GPU. A frame that needs a
//! pipeline the thread is still compiling waits for that compile (the
//! pipeline manager of Graphite shares it) rather than compiling it again.
//!
//! Graphite's own precompilation (`PrecompileContext`, `PaintOptions`) and
//! its persistent pipeline storage are not bound by the Skia binding in use
//! (skia-safe 0.153): this is the route the binding supports
//! (docs/porting/gpu-rendering.md, "Pipelines made ahead of their first
//! draw").

use crate::gpu::{ISkiaGrContext, SkiaGpuBackend};
use skia_safe::gpu::graphite;
use skia_safe::gpu::Mipmapped;
use skia_safe::{Image, ImageInfo, Surface, SurfaceProps};
use std::any::Any;
use std::cell::RefCell;
use std::time::{Duration, Instant};

/// What a prewarm records into. The uploaded textures are declared first so
/// that they are released before the recorder they live on.
struct PrewarmState {
    /// The raster images drawn so far, and their textures on the recorder
    /// (`None` when the upload failed).
    uploads: Vec<(Image, Option<Image>)>,
    recorder: graphite::Recorder,
    /// When the recorder was made: the time the prewarm took to record.
    started: Instant,
}

/// The GPU context of a Graphite prewarm: its surfaces record into a
/// recorder of its own, made by the context of the frames;
/// [`finish`](Self::finish) compiles their pipelines on a thread of its own.
pub struct GraphitePrewarmContext {
    state: RefCell<Option<PrewarmState>>,
}

impl GraphitePrewarmContext {
    pub(crate) fn new(recorder: graphite::Recorder) -> Self {
        Self { state: RefCell::new(Some(PrewarmState { uploads: Vec::new(), recorder, started: Instant::now() })) }
    }

    /// Hands the recorder to a thread of its own, which snaps it (compiling
    /// the pipelines of what was drawn) and drops the recording and the
    /// recorder. The surfaces made by the context must be released first:
    /// a surface hands its draws to the recorder when it is released. The
    /// context records nothing afterwards.
    ///
    /// Returns the thread, which answers whether the recording was made;
    /// `None` when the prewarm was finished before or the thread could not
    /// be started (the recorder is then dropped here).
    pub fn finish(&self) -> Option<std::thread::JoinHandle<bool>> {
        let PrewarmState { uploads, recorder, started } = self.state.borrow_mut().take()?;
        drop(uploads);
        let compile = PrewarmCompile { recorder, recorded: started.elapsed() };
        std::thread::Builder::new()
            .name("ferroui-graphite-prewarm".to_owned())
            .spawn(move || compile.run())
            .ok()
    }

    /// Whether the prewarm can still record.
    pub fn is_finished(&self) -> bool {
        self.state.borrow().is_none()
    }
}

impl ISkiaGrContext for GraphitePrewarmContext {
    fn backend(&self) -> SkiaGpuBackend {
        SkiaGpuBackend::Graphite
    }

    fn create_surface(&self, image_info: &ImageInfo, surface_props: &SurfaceProps) -> Option<Surface> {
        let mut state = self.state.borrow_mut();
        let state = state.as_mut()?;
        graphite::surfaces::render_target(&mut state.recorder, image_info, Mipmapped::No, Some(surface_props), None)
    }

    fn max_render_target_size(&self) -> Option<i32> {
        None
    }

    fn flush(&self) {
        // What was drawn is compiled when the prewarm is finished.
    }

    fn reset_context(&self) {}

    fn set_resource_cache_limit(&self, _max_resource_bytes: i64) {}

    fn snapshot_to_raster(&self, _surface: &mut Surface) -> Option<Image> {
        // Nothing of a prewarm is read back.
        None
    }

    fn drawable_image(&self, image: &Image, _mipmapped: bool) -> Image {
        if image.is_texture_backed() {
            return image.clone();
        }
        let mut state = self.state.borrow_mut();
        let Some(PrewarmState { uploads, recorder, .. }) = state.as_mut() else {
            return image.clone();
        };
        // An image is uploaded once per prewarm; its mipmaps (a sampler
        // state) make no pipeline of their own.
        if let Some((_, texture)) = uploads.iter().find(|(source, _)| source.unique_id() == image.unique_id()) {
            return texture.clone().unwrap_or_else(|| image.clone());
        }
        let texture = graphite::images::texture_from_image(recorder, image);
        let drawable = texture.clone().unwrap_or_else(|| image.clone());
        uploads.push((image.clone(), texture));
        drawable
    }

    fn is_lost(&self) -> bool {
        false
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// A recorder on its way to the thread that compiles its pipelines.
struct PrewarmCompile {
    recorder: graphite::Recorder,
    /// How long the prewarm took to record, on the render thread.
    recorded: Duration,
}

// SAFETY: a Graphite recorder is owned by one thread at a time, not bound to
// the thread that made it: Skia asserts only that two threads do not use it
// at once (`ASSERT_SINGLE_OWNER`), and recorders are made to record on other
// threads than the one of their context. A recorder made by
// `Context::makeRecorder` has a resource provider of its own and shares with
// the context only the shared context, whose global pipeline cache, shader
// code dictionary and pipeline manager lock what several threads use. The
// recorder moves here after its last use by the render thread (the surfaces
// made on it, and the textures it uploaded, are released before), and only
// the compiling thread uses and drops it afterwards. The binding marks a
// recorder neither `Send` nor `Sync` because two threads must not share one.
unsafe impl Send for PrewarmCompile {}

impl PrewarmCompile {
    /// Snaps the recorder (compiling the pipelines), then drops the
    /// recording uninserted and the recorder. Returns whether the recording
    /// was made.
    fn run(self) -> bool {
        let PrewarmCompile { mut recorder, recorded } = self;
        // The Metal objects Skia autoreleases on this thread.
        let pool = crate::gpu::metal::AutoReleasePool::new();
        let started = Instant::now();
        let recording = recorder.snap();
        let compiled = started.elapsed();
        let made = recording.is_some();
        drop(recording);
        drop(recorder);
        pool.dispose();
        if report_prewarm() {
            eprintln!(
                "graphite pipeline prewarm: recorded in {:.3} ms on the render thread, compiled in {:.3} ms on a thread of its own{}",
                recorded.as_secs_f64() * 1000.0,
                compiled.as_secs_f64() * 1000.0,
                if made { "" } else { " (the recording failed)" },
            );
        }
        made
    }
}

/// Whether the environment asks for the measures of the frames
/// (`FERROUI_FRAME_TIMES=1` or `FERROUI_SLOW_FRAME_MS`), which the time of
/// the prewarm goes with.
fn report_prewarm() -> bool {
    std::env::var("FERROUI_FRAME_TIMES").is_ok_and(|value| value == "1") || std::env::var_os("FERROUI_SLOW_FRAME_MS").is_some()
}
