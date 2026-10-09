//! Where the time of a frame of the backend goes.
//!
//! Off unless the environment variable `FERROUI_VELLO_PERF` is set when the
//! backend first draws. Its value is the threshold in milliseconds from which
//! a frame is printed on its own (`FERROUI_VELLO_PERF=8`; `1` and anything
//! that is not a number print the summary only). A frame is what is drawn
//! between two presentations of a target of a platform (the texture of a
//! window, a framebuffer): the scenes of the layers and intermediate surfaces
//! that were drawn for it are counted with it.
//!
//! The summary is printed when a backend context is disposed and by
//! [`print_summary`]; [`take_summary`] hands the totals to a benchmark.
//! Everything is written to the standard error stream.
//!
//! What is counted is listed in [`Phase`]. The times of the phases overlap
//! where one is part of another (flattening, the outline of a stroke and
//! glyph runs are parts of the scene; uploads and read backs are not parts of
//! the render call).

use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::Mutex;
use std::time::{Duration, Instant};

/// What is timed and counted.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(usize)]
pub enum Phase {
    /// A drawing context from its creation to its disposal, without the
    /// rendering of its scene: the recording of the scene.
    Scene,
    /// Curves made into lines (bytes: path elements).
    Flatten,
    /// The outline of a stroke (bytes: path elements of the outline).
    StrokeOutline,
    /// Glyph runs handed to the renderer (bytes: glyphs).
    GlyphRun,
    /// The render call of a renderer, with what it submits to the device.
    Render,
    /// Pixels read back from the device (bytes: the pixels), with the wait
    /// for the device.
    ReadBack,
    /// Pixels uploaded as a texture (bytes: the pixels).
    Upload,
    /// A texture copied into another on the device: what a scene was
    /// rendered into, into the texture of a window (bytes: the pixels).
    TextureCopy,
    /// Pixels copied or converted in memory: the image of a surface, a
    /// framebuffer written in its format (bytes: the pixels).
    PixelCopy,
    /// A scene rendered by the CPU mode (bytes: the pixels of its target).
    CpuRender,
    /// The presentation of the frame: the disposal of the session of the
    /// platform.
    Present,
    /// An intermediate surface in memory that was created (bytes: its
    /// pixels).
    SurfaceInMemory,
    /// An intermediate surface on the device that was created (bytes: its
    /// pixels).
    SurfaceOnDevice,
    /// An effect that was recorded into a scene of its own and composed as
    /// an image (bytes: the pixels of that scene).
    EffectAsImage,
    /// A box shadow that was blurred as an image (bytes: its pixels).
    ShadowAsImage,
    /// A scene brush or tile brush drawn into an intermediate surface.
    BrushSurface,
    /// A drawing context that was created (a scene).
    Context,
}

const PHASES: [Phase; 17] = [
    Phase::Scene,
    Phase::Flatten,
    Phase::StrokeOutline,
    Phase::GlyphRun,
    Phase::Render,
    Phase::ReadBack,
    Phase::Upload,
    Phase::TextureCopy,
    Phase::PixelCopy,
    Phase::CpuRender,
    Phase::Present,
    Phase::SurfaceInMemory,
    Phase::SurfaceOnDevice,
    Phase::EffectAsImage,
    Phase::ShadowAsImage,
    Phase::BrushSurface,
    Phase::Context,
];

impl Phase {
    fn name(self) -> &'static str {
        match self {
            Phase::Scene => "scene recording",
            Phase::Flatten => "curve flattening",
            Phase::StrokeOutline => "stroke outlines",
            Phase::GlyphRun => "glyph runs",
            Phase::Render => "render call",
            Phase::ReadBack => "read back",
            Phase::Upload => "texture upload",
            Phase::TextureCopy => "texture copies",
            Phase::PixelCopy => "pixel copies",
            Phase::CpuRender => "CPU scene render",
            Phase::Present => "present",
            Phase::SurfaceInMemory => "surfaces in memory",
            Phase::SurfaceOnDevice => "surfaces on device",
            Phase::EffectAsImage => "effects as images",
            Phase::ShadowAsImage => "shadows as images",
            Phase::BrushSurface => "brush surfaces",
            Phase::Context => "drawing contexts",
        }
    }
}

/// The time, the count and the bytes of one phase.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct PhaseTotal {
    /// The time spent in the phase.
    pub time: Duration,
    /// How often the phase ran.
    pub count: u64,
    /// The bytes (or elements) the phase handled.
    pub bytes: u64,
}

impl PhaseTotal {
    fn add(&mut self, other: &PhaseTotal) {
        self.time += other.time;
        self.count += other.count;
        self.bytes += other.bytes;
    }
}

/// The totals of the frames since the last [`take_summary`].
#[derive(Clone, Debug, Default)]
pub struct Summary {
    /// The frames that were presented.
    pub frames: u64,
    /// The time of every frame, from the creation of the drawing context of
    /// the target to the end of its presentation, in the order they were
    /// drawn.
    pub frame_times: Vec<Duration>,
    totals: [PhaseTotal; PHASES.len()],
}

impl Summary {
    /// The total of a phase.
    pub fn phase(&self, phase: Phase) -> PhaseTotal {
        self.totals[phase as usize]
    }

    /// The summary as text: the frames, their times and a line a phase.
    pub fn report(&self) -> String {
        let mut text = String::new();
        let mut times: Vec<f64> = self.frame_times.iter().map(|time| time.as_secs_f64() * 1000.0).collect();
        times.sort_by(f64::total_cmp);
        let at = |q: f64| if times.is_empty() { 0.0 } else { times[((times.len() - 1) as f64 * q).round() as usize] };
        let total: f64 = times.iter().sum();
        text.push_str(&format!(
            "Vello frames: {}; ms per frame: median {:.3}, mean {:.3}, p95 {:.3}, max {:.3}\n",
            self.frames,
            at(0.5),
            if times.is_empty() { 0.0 } else { total / times.len() as f64 },
            at(0.95),
            at(1.0),
        ));
        let frames = self.frames.max(1) as f64;
        for phase in PHASES {
            let phase_total = self.totals[phase as usize];
            if phase_total.count == 0 && phase_total.time.is_zero() {
                continue;
            }
            let ms = phase_total.time.as_secs_f64() * 1000.0;
            text.push_str(&format!(
                "  {:<20} {:>10.3} ms a frame ({:>5.1} % of the frame time), {:>8.1} a frame, {:>12.0} bytes a frame\n",
                phase.name(),
                ms / frames,
                if total > 0.0 { ms / total * 100.0 } else { 0.0 },
                phase_total.count as f64 / frames,
                phase_total.bytes as f64 / frames,
            ));
        }
        text
    }
}

struct State {
    threshold: Option<Duration>,
    /// What was counted since the last frame ended.
    current: [PhaseTotal; PHASES.len()],
    summary: Summary,
}

static ENABLED: AtomicU8 = AtomicU8::new(0);
static STATE: Mutex<Option<State>> = Mutex::new(None);

/// Whether the phases are counted.
pub fn enabled() -> bool {
    match ENABLED.load(Ordering::Relaxed) {
        1 => false,
        2 => true,
        _ => {
            let value = std::env::var("FERROUI_VELLO_PERF").ok();
            match value {
                Some(value) => {
                    let threshold =
                        value.parse::<f64>().ok().filter(|ms| *ms > 1.0).map(|ms| Duration::from_secs_f64(ms / 1000.0));
                    enable(threshold);
                    true
                }
                None => {
                    ENABLED.store(1, Ordering::Relaxed);
                    false
                }
            }
        }
    }
}

/// Turns the counting on, whatever the environment says: for a benchmark.
/// A frame that takes `threshold` or longer is printed on its own.
pub fn enable(threshold: Option<Duration>) {
    let mut state = STATE.lock().unwrap_or_else(|e| e.into_inner());
    if state.is_none() {
        *state = Some(State { threshold, current: Default::default(), summary: Summary::default() });
    }
    ENABLED.store(2, Ordering::Relaxed);
}

fn with_state(f: impl FnOnce(&mut State)) {
    if let Some(state) = STATE.lock().unwrap_or_else(|e| e.into_inner()).as_mut() {
        f(state);
    }
}

/// Counts one run of a phase that handled `bytes`.
pub fn count(phase: Phase, bytes: u64) {
    if enabled() {
        with_state(|state| {
            let total = &mut state.current[phase as usize];
            total.count += 1;
            total.bytes += bytes;
        });
    }
}

/// Adds time to a phase without counting a run.
pub fn add_time(phase: Phase, time: Duration) {
    if enabled() {
        with_state(|state| state.current[phase as usize].time += time);
    }
}

/// Times a phase from its creation to its drop.
pub struct Scope {
    phase: Phase,
    bytes: u64,
    start: Option<Instant>,
}

/// Starts timing one run of a phase that handles `bytes`.
pub fn scope(phase: Phase, bytes: u64) -> Scope {
    Scope { phase, bytes, start: enabled().then(Instant::now) }
}

impl Scope {
    /// Replaces the bytes the run handled, when they are known only later.
    pub fn set_bytes(&mut self, bytes: u64) {
        self.bytes = bytes;
    }
}

impl Drop for Scope {
    fn drop(&mut self) {
        if let Some(start) = self.start {
            let elapsed = start.elapsed();
            with_state(|state| {
                let total = &mut state.current[self.phase as usize];
                total.time += elapsed;
                total.count += 1;
                total.bytes += self.bytes;
            });
        }
    }
}

/// The start of a frame of a target of a platform, if frames are timed.
pub fn frame_start() -> Option<Instant> {
    enabled().then(Instant::now)
}

/// Ends the frame that started at `start`: what was counted since the frame
/// before belongs to it.
pub fn frame_end(start: Option<Instant>, target: &str, width: u32, height: u32) {
    let Some(start) = start else { return };
    let elapsed = start.elapsed();
    with_state(|state| {
        let current = std::mem::take(&mut state.current);
        if state.threshold.is_some_and(|threshold| elapsed >= threshold) {
            let mut line = format!(
                "Vello frame {} ({target}, {width} by {height}): {:.3} ms;",
                state.summary.frames,
                elapsed.as_secs_f64() * 1000.0
            );
            for phase in PHASES {
                let total = current[phase as usize];
                if total.count > 0 || !total.time.is_zero() {
                    line.push_str(&format!(
                        " {} {:.3} ms x{} ({} bytes);",
                        phase.name(),
                        total.time.as_secs_f64() * 1000.0,
                        total.count,
                        total.bytes
                    ));
                }
            }
            eprintln!("{line}");
        }
        state.summary.frames += 1;
        state.summary.frame_times.push(elapsed);
        for (total, frame) in state.summary.totals.iter_mut().zip(current.iter()) {
            total.add(frame);
        }
    });
}

/// The totals since the last call, which start over.
pub fn take_summary() -> Summary {
    let mut summary = Summary::default();
    with_state(|state| summary = std::mem::take(&mut state.summary));
    summary
}

/// Prints the totals since the last summary to the standard error stream,
/// when anything was counted, and starts them over.
pub fn print_summary() {
    if !enabled() {
        return;
    }
    let summary = take_summary();
    if summary.frames > 0 {
        eprint!("{}", summary.report());
    }
}
