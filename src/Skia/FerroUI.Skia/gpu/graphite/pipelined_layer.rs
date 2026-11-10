//! A frame layer that draws a frame while the one before it is presented
//! (desktop-performance.md, section 9.33).
//!
//! Recording a frame with many draws costs more on the render thread than
//! the frame has: Graphite sorts the draws, prepares their text and builds
//! its passes on the thread that draws and snaps. A recorder is used by one
//! thread at a time, but a context takes the recordings of any number of
//! recorders, so the work divides when the frame does, and it leaves the
//! render thread when another thread has a recorder of its own.
//!
//! A layer of this kind draws its frames in one of two ways.
//!
//! *Directly*, as an ordinary layer does: the drawing context draws on the
//! surface of the layer, which belongs to the recorder of the context.
//! Every layer begins like this.
//!
//! *Pipelined*, once its frames are late ([`PipelinePolicy`]): the drawing
//! context records the frame as a picture. When the context is disposed the
//! picture is handed to the threads of the bands, one per horizontal strip
//! of the layer ([`SkiaOptions::frame_pipelining_bands`](crate::SkiaOptions::frame_pipelining_bands),
//! two by default). Each owns a recorder: it plays the picture clipped to its
//! band onto a surface that wraps the texture of the layer, snaps its
//! recorder and keeps the recording. The render thread does not wait. It
//! draws the layer onto the target as the frame before left it, and after
//! the flush that puts this draw into the context it tells the threads to
//! insert their recordings, which each does under the lock of the context.
//! The frame is therefore on the screen one frame later than it was drawn,
//! and the render thread is free for the next one in the meantime. The
//! owner of the layer presents the last frame of a stream by itself
//! ([`ISkiaSurface::complete_pending_frame`]).
//!
//! Nothing of Skia is repeated here: the picture, the clip and the playback
//! are Skia's, and the bands differ from a single playback only in the clip
//! Skia applies.
//!
//! The order on the GPU is the order of insertion: what the recorder of the
//! context has pending when a frame ends (it is snapped and inserted before
//! the bands are sent, so that the images it made exist), the draw of the
//! layer onto the target with the flush of the tick, then the recordings of
//! the bands from the top ([`InsertOrder`]), which the next frame, the next
//! read of the layer and the next flush wait for before they add anything.
//!
//! What Graphite gives and asks, read in its source:
//!
//! - Recordings must be inserted in the order they were snapped only among
//!   the recordings of one recorder (the context is made with ordered
//!   recordings). Recordings of different recorders may be inserted in any
//!   order; a dependency between recorders is the client's to keep, which
//!   the order above does. A recording that was snapped and never inserted
//!   makes every later recording of its recorder fail to insert, so a band
//!   that fails ends the threads (their recorders go with them), and the
//!   layers of the context draw directly from then on.
//! - Every recorder has a glyph atlas, a strike cache, a text blob cache and
//!   a resource provider of its own: each band rasterizes and uploads the
//!   glyphs it draws, and the caches of the bands do not see each other.
//!   The strike cache of the CPU side is shared and locked.
//! - A surface that wraps a texture loads the texture when it is first drawn
//!   to, and the clip of a band is not the whole surface, so a band never
//!   clears or discards what the other bands drew.
//! - An image of one recorder drawn by another shows the texture as the
//!   order of insertion left it; nothing is flushed for it. That is why the
//!   recorder of the context is snapped and inserted before the bands are
//!   sent, and why no recording of that recorder is inserted while bands
//!   that were told to insert have not answered.
//!
//! A frame is drawn directly although the layer is pipelined when only the
//! recorder of the context draws it as an ordinary layer would: when it
//! leased the Skia API (what the client draws belongs to the recorder the
//! lease hands out) and when it drew with an image filter (a filter reads
//! around the pixels it writes, across the clip of a band). Such a frame
//! also sends the layer back to drawing directly for a while, and because
//! the same draws are counted in direct frames too
//! ([`ISkiaGrContext::note_direct_only_draw`]), a layer whose frames keep
//! having them never begins to pipeline.

use super::graphite_gr_context::{GraphiteGrContext, SharedContext};
use crate::drawing_context_impl::{CanvasSource, RecordedLeases};
use crate::gpu::metal::AutoReleasePool;
use crate::gpu::ISkiaSurface;
use crate::SkiaFramePipelining;
use ferroui_base::PixelSize;
use skia_safe::gpu::graphite::mtl::backend_textures;
use skia_safe::gpu::graphite::{self, InsertRecordingInfo};
use skia_safe::{
    Canvas, ClipOp, Color, ColorType, IRect, Picture, PictureRecorder, Point, Rect, SamplingOptions, Surface,
    SurfaceProps,
};
use std::cell::{Cell, RefCell};
use std::ffi::c_void;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::rc::Rc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{mpsc, Arc, Condvar, Mutex, MutexGuard, PoisonError};
use std::thread;
use std::time::{Duration, Instant};

/// The least height of a band in pixels: a layer has as many bands as it
/// was asked for and as fit it at this height ([`band_count`]).
const MIN_BAND_HEIGHT: i32 = 64;

/// How many bands a layer of `height` rows is divided into when `asked`
/// are asked for (at least one): `asked`, or as many bands of
/// [`MIN_BAND_HEIGHT`] rows as the layer has room for. `None` when that
/// would leave a layer that was asked to be divided with a single band,
/// or with none: such a layer stays an ordinary layer.
pub(crate) fn band_count(height: i32, asked: usize) -> Option<usize> {
    let asked = asked.max(1);
    let fit = usize::try_from(height / MIN_BAND_HEIGHT).unwrap_or(0);
    let bands = asked.min(fit);
    (bands >= asked.min(2)).then_some(bands)
}

/// Frames that begin further apart than this do not belong to one stream:
/// the layer forgets what it measured and draws the next one directly.
const STREAM_GAP: Duration = Duration::from_millis(100);

/// A frame that cost less than this is never late, whatever the period.
const MIN_LATE_COST: Duration = Duration::from_millis(4);

/// How many late frames in a row make a layer pipeline its frames.
const LATE_FRAMES: u32 = 4;

/// How many light frames in a row make a pipelined layer draw directly
/// again.
const LIGHT_FRAMES: u32 = 90;

/// For how many frames a layer draws directly after a frame that only the
/// recorder of the context can draw.
const HOLD_OFF_FRAMES: u32 = 120;

/// Decides, frame by frame, whether a layer pipelines its frames.
///
/// The layer has no clock of the display. What it knows is when its frames
/// begin and what each cost the render thread, and a frame is *late* when
/// its cost is at least half the time to the next one: the render timer
/// ticks with the display, so a frame that takes between one and one and a
/// half refresh intervals is followed by the next after two intervals, and
/// its cost is half to three quarters of that period. A frame that fits its
/// interval and costs more than half of it is late by the same measure;
/// pipelining it costs a frame of latency and gains nothing but headroom.
///
/// A pipelined layer goes back to drawing directly when its frames are
/// *light*, three tenths of the period or less, for a second and a half of
/// frames: the distance between the two thresholds keeps a layer from
/// switching back and forth.
pub(super) struct PipelinePolicy {
    mode: SkiaFramePipelining,
    pipelined: bool,
    last_begin: Option<Instant>,
    /// Late frames in a row, while drawing directly.
    late: u32,
    /// Light frames in a row, while pipelined.
    light: u32,
    /// Frames still to draw directly before late frames count again.
    hold_off: u32,
}

impl PipelinePolicy {
    pub(super) fn new(mode: SkiaFramePipelining) -> Self {
        Self {
            mode,
            pipelined: mode == SkiaFramePipelining::On,
            last_begin: None,
            late: 0,
            light: 0,
            hold_off: 0,
        }
    }

    pub(super) fn is_pipelined(&self) -> bool {
        self.pipelined
    }

    /// A frame begins at `now`; `previous_cost` is what the frame before it
    /// cost the render thread, or would have cost it drawn directly, when
    /// that is known. Returns whether the frame is pipelined.
    pub(super) fn frame_begins(&mut self, now: Instant, previous_cost: Option<Duration>) -> bool {
        match self.mode {
            SkiaFramePipelining::Off => return false,
            SkiaFramePipelining::On => return true,
            SkiaFramePipelining::Adaptive => {}
        }

        let period = self.last_begin.map(|last| now.saturating_duration_since(last));
        self.last_begin = Some(now);
        let Some(period) = period.filter(|period| *period <= STREAM_GAP) else {
            // The first frame, or the first after a pause: nothing is known
            // about the stream it begins.
            self.pipelined = false;
            self.late = 0;
            self.light = 0;
            return false;
        };

        if self.pipelined {
            let light = previous_cost.is_some_and(|cost| cost * 10 <= period * 3);
            self.light = if light { self.light + 1 } else { 0 };
            if self.light >= LIGHT_FRAMES {
                self.pipelined = false;
                self.light = 0;
                self.late = 0;
            }
        } else {
            self.hold_off = self.hold_off.saturating_sub(1);
            let late = previous_cost.is_some_and(|cost| cost >= MIN_LATE_COST && cost * 2 >= period);
            self.late = if late { self.late + 1 } else { 0 };
            if self.late >= LATE_FRAMES && self.hold_off == 0 {
                self.pipelined = true;
                self.late = 0;
                self.light = 0;
            }
        }
        self.pipelined
    }

    /// A frame needed the recorder of the context: the layer draws directly
    /// for the next [`HOLD_OFF_FRAMES`] frames at least. Nothing changes
    /// when the options ask for pipelined frames always.
    pub(super) fn frame_needed_direct(&mut self) {
        if self.mode != SkiaFramePipelining::Adaptive {
            return;
        }
        self.pipelined = false;
        self.late = 0;
        self.light = 0;
        self.hold_off = HOLD_OFF_FRAMES;
    }
}

extern "C" {
    /// A new `id<MTLTexture>` of the device, drawn to and sampled, with one
    /// reference that the caller owns; null when it cannot be made
    /// (`pipelined_layer_texture.m`).
    fn ferro_skia_layer_texture_new(device: *mut c_void, width: i32, height: i32, rgba: i32) -> *mut c_void;

    /// Gives back the reference of `ferro_skia_layer_texture_new`.
    fn ferro_skia_layer_texture_release(texture: *mut c_void);
}

/// The Metal texture of a pipelined layer: one owned reference to an
/// `id<MTLTexture>`. Every surface that wraps the texture holds a reference
/// of its own, so the texture outlives this one for as long as it is drawn.
struct LayerTexture(*mut c_void);

// SAFETY: the pointer is an `id<MTLTexture>`, an object Metal lets any
// thread retain, use and release. The type only hands the pointer to Skia,
// which takes a reference of its own, and releases its one reference when it
// is dropped, on whichever thread that is.
unsafe impl Send for LayerTexture {}
// SAFETY: as above; the type has no interior state.
unsafe impl Sync for LayerTexture {}

impl Drop for LayerTexture {
    fn drop(&mut self) {
        // SAFETY: the pointer came from `ferro_skia_layer_texture_new`, was
        // not null (a null one is never wrapped in this type), and its one
        // reference is given back exactly once, here.
        unsafe { ferro_skia_layer_texture_release(self.0) };
    }
}

/// Wraps the texture of a layer in a surface of `recorder`, with the colour
/// type and the surface properties of the layer. A wrapped texture is loaded
/// when it is drawn to, never cleared by the wrapping.
fn wrap_texture(
    recorder: &mut graphite::Recorder,
    texture: &LayerTexture,
    size: (i32, i32),
    color_type: ColorType,
    props: &SurfaceProps,
) -> Option<Surface> {
    // SAFETY: `texture` holds a reference to the `id<MTLTexture>` for the
    // duration of the call, and the backend texture is used only by the call
    // below, in which Graphite takes a reference of its own for the surface.
    let backend_texture = unsafe { backend_textures::make_metal(size, texture.0) };
    graphite::surfaces::wrap_backend_texture(recorder, &backend_texture, color_type, None, Some(props))
}

/// The `bands` bands of a layer of the given size: the index of each band
/// and the rows it draws, from the top. The bands are disjoint and cover
/// the layer.
pub(crate) fn band_clips(size: (i32, i32), bands: usize) -> Vec<(usize, IRect)> {
    let (width, height) = size;
    let bands = bands.max(1);
    let mut clips = Vec::with_capacity(bands);
    for index in 0..bands {
        let top = (i64::from(height) * index as i64 / bands as i64) as i32;
        let bottom = (i64::from(height) * (index as i64 + 1) / bands as i64) as i32;
        let clip = IRect::new(0, top, width, bottom);
        if !clip.is_empty() {
            clips.push((index, clip));
        }
    }
    clips
}

/// Draws the part of a picture that lies in a band: the picture under the
/// clip of the band, which Skia applies.
fn draw_band(canvas: &Canvas, picture: &Picture, clip: IRect) {
    canvas.save();
    canvas.clip_rect(Rect::from(clip), ClipOp::Intersect, false);
    canvas.draw_picture(picture, None, None);
    canvas.restore();
}

fn insert(context: &mut graphite::Context, recording: &mut graphite::Recording) -> bool {
    let info = InsertRecordingInfo::new(recording);
    context.insert_recording(&info) == graphite::InsertStatus::Success
}

/// Why a frame was not handed to the threads.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Fallback {
    /// The frame drew what only the recorder of the context draws as an
    /// ordinary layer would (see the module).
    DirectOnly,
    /// The context was disposed or in use, or the layer was.
    Disposed,
    /// The threads could not be made, or were ended by an earlier failure.
    PoolUnavailable,
    /// The pending work of the recorder of the context could not be snapped
    /// or inserted.
    MainRecording,
}

/// The identities of the pipelined layers, by which a thread of a band keeps
/// the surface it wrapped the texture of a layer in.
static NEXT_LAYER_ID: AtomicU64 = AtomicU64::new(1);

/// A band of a frame for the thread that plays it.
struct Band {
    /// The identity of the layer.
    layer: u64,
    texture: Arc<LayerTexture>,
    size: (i32, i32),
    color_type: ColorType,
    props: SurfaceProps,
    picture: Picture,
    /// The band in the pixels of the layer.
    clip: IRect,
    /// Which band of the frame this is.
    index: usize,
    /// The place of the recording of the band among the recordings of the
    /// threads ([`InsertOrder`]).
    turn: u64,
}

// SAFETY: a band is made on the render thread and used by the one thread it
// is sent to. Its picture is immutable once recorded and reference counted
// atomically, as every Skia object a picture holds is: Skia plays one
// picture from several threads at once. The render thread keeps a reference
// to the picture until the thread has answered (`PendingFrame`), so the
// reference dropped here is not the last one and what the picture holds is
// released on the render thread. The surface properties are plain values.
unsafe impl Send for Band {}

enum BandJob {
    /// Play a band and keep its recording.
    Play(Band),
    /// The layer is gone: release the surface that wraps its texture.
    Forget { layer: u64 },
    /// Insert the recordings that were kept, and tell what became of them.
    Insert,
}

/// What a thread tells about a band it was sent.
struct BandOutput {
    /// The identity of the layer.
    layer: u64,
    index: usize,
    /// Whether the band was recorded.
    recorded: bool,
    /// Whether the recording is in the context.
    inserted: bool,
    /// How long the thread worked on the band.
    busy: Duration,
}

/// A recording a thread keeps until it is told to insert it.
struct Held {
    layer: u64,
    index: usize,
    recording: Option<graphite::Recording>,
    busy: Duration,
    turn: Turn,
}

/// The order in which the threads insert their recordings: the order the
/// bands were sent in, so the bands of a frame from the top, frame after
/// frame.
///
/// The bands of a frame write disjoint pixels, and still the frame is not
/// the same in every order: where an antialiased edge crosses the row two
/// bands meet at, Graphite covers the pixel at the crossing a little
/// differently in the band that is drawn first (measured: one pixel, six of
/// 255 in a channel). A fixed order makes a frame the same every time it is
/// drawn, and the same as the frame the recorder of the context draws band
/// by band. It costs nothing: the threads insert under the lock of the
/// context, one at a time in any case.
#[derive(Default)]
struct InsertOrder {
    /// The turn of the recording that is inserted next.
    next: Mutex<u64>,
    passed: Condvar,
}

/// The turn of a band a thread was sent. The thread waits for the turn
/// before it inserts the recording, and the turn passes to the next band
/// when this is dropped, whatever became of the recording, so that a band
/// that fails does not hold the others up.
struct Turn {
    order: Arc<InsertOrder>,
    number: u64,
}

impl Turn {
    /// Waits until every recording before this one has been inserted.
    fn wait(&self) -> MutexGuard<'_, u64> {
        let mut next = self.order.next.lock().unwrap_or_else(PoisonError::into_inner);
        while *next < self.number {
            next = self.order.passed.wait(next).unwrap_or_else(PoisonError::into_inner);
        }
        next
    }
}

impl Drop for Turn {
    fn drop(&mut self) {
        let mut next = self.wait();
        *next = (*next).max(self.number + 1);
        drop(next);
        self.order.passed.notify_all();
    }
}

/// The recorder of a band on its way to the thread of the band.
struct BandRecorder(graphite::Recorder);

// SAFETY: a Graphite recorder is owned by one thread at a time, not bound to
// the thread that made it: Skia asserts only that two threads do not use it
// at once, and recorders are made to record on other threads than the one of
// their context. A recorder made by the context has a resource provider, an
// atlas and caches of its own and shares with the context only what the
// context locks. The recorder moves to its thread right after it is made,
// before anything was drawn with it, and only that thread uses and drops it
// afterwards. The binding marks a recorder neither `Send` nor `Sync` because
// two threads must not share one.
unsafe impl Send for BandRecorder {}

/// A thread of a band: a recorder of its own and the jobs it is sent.
struct BandWorker {
    recorder: BandRecorder,
    jobs: mpsc::Receiver<BandJob>,
    results: mpsc::Sender<BandOutput>,
    /// The context the thread inserts its recordings into.
    context: Arc<SharedContext>,
    order: Arc<InsertOrder>,
}

impl BandWorker {
    fn run(self) {
        let BandWorker { recorder, jobs, results, context, order } = self;
        let mut recorder = recorder.0;
        let mut held: Vec<Held> = Vec::new();
        // The surfaces that wrap the textures of the layers, by layer. They
        // belong to the recorder, so they are made, used and dropped here.
        let mut surfaces: Vec<(u64, Surface)> = Vec::new();

        while let Ok(job) = jobs.recv() {
            let pool = AutoReleasePool::new();
            match job {
                BandJob::Play(band) => {
                    let started = Instant::now();
                    let turn = Turn { order: order.clone(), number: band.turn };
                    // A panic would leave the render thread waiting for an
                    // answer that never comes; it is reported as a band
                    // that failed, which ends the threads.
                    let recording =
                        catch_unwind(AssertUnwindSafe(|| play(&mut recorder, &mut surfaces, &band))).unwrap_or(None);
                    held.push(Held { layer: band.layer, index: band.index, recording, busy: started.elapsed(), turn });
                    // The reference of the render thread outlives this one
                    // (see `Band`).
                    drop(band);
                }
                BandJob::Forget { layer } => surfaces.retain(|(id, _)| *id != layer),
                BandJob::Insert => {
                    let mut gone = false;
                    for Held { layer, index, recording, busy, turn } in held.drain(..) {
                        let recorded = recording.is_some();
                        drop(turn.wait());
                        let inserted = recording.is_some_and(|mut recording| {
                            catch_unwind(AssertUnwindSafe(|| insert(&mut context.lock(), &mut recording)))
                                .unwrap_or(false)
                        });
                        drop(turn);
                        gone |= results.send(BandOutput { layer, index, recorded, inserted, busy }).is_err();
                    }
                    if gone {
                        pool.dispose();
                        break;
                    }
                }
            }
            pool.dispose();
        }

        let pool = AutoReleasePool::new();
        drop(held);
        drop(surfaces);
        drop(recorder);
        pool.dispose();
    }
}

/// Plays a band on the recorder of its thread and snaps the recorder.
fn play(
    recorder: &mut graphite::Recorder,
    surfaces: &mut Vec<(u64, Surface)>,
    band: &Band,
) -> Option<graphite::Recording> {
    let position = match surfaces.iter().position(|(layer, _)| *layer == band.layer) {
        Some(position) => position,
        None => {
            let surface = wrap_texture(recorder, &band.texture, band.size, band.color_type, &band.props)?;
            surfaces.push((band.layer, surface));
            surfaces.len() - 1
        }
    };
    draw_band(surfaces[position].1.canvas(), &band.picture, band.clip);
    recorder.snap()
}

struct WorkerHandle {
    jobs: mpsc::Sender<BandJob>,
    thread: thread::JoinHandle<()>,
}

/// The threads that play the bands of the pipelined layers of a context,
/// made when a layer first pipelines a frame and joined when the context
/// goes.
#[derive(Default)]
pub(super) struct BandPool {
    workers: Vec<WorkerHandle>,
    results: Option<mpsc::Receiver<BandOutput>>,
    /// The threads failed or could not be made: the layers of the context
    /// draw directly from then on.
    disabled: bool,
    /// By thread: how many bands it was sent and not yet told to insert.
    unreleased: Vec<usize>,
    /// How many answers are still to come for the bands the threads were
    /// told to insert.
    released: usize,
    /// The answers that were received ahead of the layer that asks for them.
    stash: Vec<BandOutput>,
    /// The order the threads insert in, and the turn of the next band that
    /// is sent.
    order: Arc<InsertOrder>,
    next_turn: u64,
}

impl BandPool {
    /// Makes the threads, each with a recorder of the context: `bands` of
    /// them, for the first layer of the context that pipelines a frame.
    /// Returns how many there are, which is how many bands a frame can
    /// have; none when they failed or could not be made.
    fn ensure(&mut self, context: &Arc<SharedContext>, bands: usize) -> usize {
        if self.disabled {
            return 0;
        }
        if !self.workers.is_empty() {
            return self.workers.len();
        }

        let (results_sender, results) = mpsc::channel();
        for index in 0..bands.max(1) {
            let Some(recorder) = context.lock().make_recorder(None) else {
                self.disable();
                return 0;
            };
            let (jobs, job_receiver) = mpsc::channel();
            let worker = BandWorker {
                recorder: BandRecorder(recorder),
                jobs: job_receiver,
                results: results_sender.clone(),
                context: context.clone(),
                order: self.order.clone(),
            };
            let spawned =
                thread::Builder::new().name(format!("ferroui-skia-band-{index}")).spawn(move || worker.run());
            match spawned {
                Ok(thread) => self.workers.push(WorkerHandle { jobs, thread }),
                Err(_) => {
                    self.disable();
                    return 0;
                }
            }
        }
        // Only the threads hold a sender now: a wait for an answer ends when
        // none of them is left.
        self.results = Some(results);
        self.unreleased = vec![0; self.workers.len()];
        self.workers.len()
    }

    /// Sends a band to its thread. Returns whether the thread has it.
    fn play(&mut self, mut band: Band) -> bool {
        let index = band.index;
        // A band that is not sent takes no turn: no thread would pass it on.
        band.turn = self.next_turn;
        let sent = self.workers.get(index).is_some_and(|worker| worker.jobs.send(BandJob::Play(band)).is_ok());
        if sent {
            if let Some(unreleased) = self.unreleased.get_mut(index) {
                *unreleased += 1;
            }
            self.next_turn += 1;
        }
        sent
    }

    /// The next answer for a band of `layer`, of the bands the threads were
    /// told to insert: waits for it when it is not there yet. `None` when
    /// the threads are gone.
    fn receive(&mut self, layer: u64) -> Option<BandOutput> {
        if let Some(position) = self.stash.iter().position(|output| output.layer == layer) {
            return Some(self.stash.swap_remove(position));
        }
        loop {
            let output = self.results.as_ref()?.recv().ok()?;
            self.released = self.released.saturating_sub(1);
            if output.layer == layer {
                return Some(output);
            }
            self.stash.push(output);
        }
    }

    /// Tells the threads to insert the bands they were sent.
    pub(super) fn release(&mut self) {
        for index in 0..self.unreleased.len() {
            let count = std::mem::take(&mut self.unreleased[index]);
            if count == 0 {
                continue;
            }
            if self.workers.get(index).is_some_and(|worker| worker.jobs.send(BandJob::Insert).is_ok()) {
                self.released += count;
            }
        }
    }

    /// Waits until the bands the threads were told to insert are in the
    /// context. Every recording of the recorder of the context is inserted
    /// after this: what it draws may be what a frame in the threads sampled,
    /// drawn anew.
    pub(super) fn wait_for_released(&mut self) {
        while self.released > 0 {
            let Some(output) = self.results.as_ref().and_then(|results| results.recv().ok()) else {
                self.released = 0;
                break;
            };
            self.released -= 1;
            self.stash.push(output);
        }
    }

    /// Tells the threads that a layer is gone. Its frames were received
    /// before (`PipelinedLayer::release`); an answer that was not, because
    /// the context was in use then, is dropped here when it has arrived.
    fn forget(&mut self, layer: u64) {
        self.stash.retain(|output| output.layer != layer);
        for worker in &self.workers {
            // A thread that ended has dropped its surfaces already.
            let _ = worker.jobs.send(BandJob::Forget { layer });
        }
    }

    /// Ends the threads for good.
    fn disable(&mut self) {
        self.disabled = true;
        self.shut_down();
    }

    /// Ends the threads and waits for them: each drops its surfaces and its
    /// recorder before it ends.
    fn shut_down(&mut self) {
        self.results = None;
        self.unreleased.clear();
        self.released = 0;
        self.stash.clear();
        // Every thread is told to end before any is waited for: a thread
        // that ends passes on the turns of the recordings it holds, and may
        // wait for a turn another thread holds.
        let threads: Vec<_> = self.workers.drain(..).map(|WorkerHandle { jobs, thread }| {
            drop(jobs);
            thread
        }).collect();
        for thread in threads {
            let _ = thread.join();
        }
    }
}

impl Drop for BandPool {
    fn drop(&mut self) {
        self.shut_down();
    }
}

/// What pipelining a frame needs of the context
/// (`GraphiteGrContext::with_band_parts`).
pub(super) struct BandParts<'a> {
    pub(super) pool: &'a mut BandPool,
    pub(super) recorder: &'a mut graphite::Recorder,
    pub(super) context: &'a Arc<SharedContext>,
    pub(super) recording_failed: &'a mut bool,
}

/// A frame whose bands were sent and whose answers were not received yet.
struct PendingFrame {
    clips: Vec<(usize, IRect)>,
    sent: usize,
    /// Held until the bands have answered, so that what the picture holds is
    /// released on the render thread, and for the bands that fail.
    picture: Picture,
}

/// Receives the answers for the bands of a frame, which waits until the
/// threads have inserted them. Returns the bands that are not in the
/// context, and how long the threads worked on the frame.
fn collect_bands(
    pool: &mut BandPool,
    recording_failed: &mut bool,
    layer: u64,
    pending: &PendingFrame,
) -> (Vec<IRect>, Duration) {
    // The bands of a frame that no flush has told to insert yet: a frame
    // that was drawn and not shown.
    pool.release();
    // By the index of the band; the indices of the bands of a frame are
    // below their number.
    let mut inserted = vec![false; pending.clips.len()];
    let mut busy = Duration::ZERO;
    for _ in 0..pending.sent {
        let Some(output) = pool.receive(layer) else {
            break;
        };
        busy += output.busy;
        if output.inserted {
            if let Some(done) = inserted.get_mut(output.index) {
                *done = true;
            }
        } else if output.recorded {
            // The context refuses what it could not add, and may hold a part
            // of it: it reports itself lost, as after any recording that
            // failed to insert.
            *recording_failed = true;
        }
    }

    let missing: Vec<IRect> = pending
        .clips
        .iter()
        .filter(|(index, _)| !inserted.get(*index).copied().unwrap_or(false))
        .map(|(_, clip)| *clip)
        .collect();
    if !missing.is_empty() {
        // A recorder whose recording was not inserted cannot insert another,
        // and a thread that failed once is not trusted again: the threads
        // end, and the layers of the context draw directly.
        pool.disable();
    }
    (missing, busy)
}

/// What a layer is in the middle of.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Frame {
    /// No frame is being drawn.
    None,
    /// A direct frame has begun; its drawing context has not asked for the
    /// surface yet.
    DirectBegun,
    /// A direct frame is drawn on the surface.
    Direct,
    /// A pipelined frame is recorded.
    Recorded,
}

/// What is known of the frame before the one that begins.
#[derive(Clone, Copy)]
enum LastFrame {
    /// Drawn directly, from `begun`: its cost ends with the flush after it.
    Direct { begun: Instant },
    /// Recorded: what drawing it directly would have cost, as far as the
    /// layer can tell (the recording of the picture and the work of the
    /// threads on the frame collected last).
    Recorded { cost: Duration },
}

struct PipelinedLayer {
    context: Rc<GraphiteGrContext>,
    id: u64,
    size: (i32, i32),
    /// How many bands a pipelined frame of the layer is divided into, when
    /// the context has that many threads.
    bands: usize,
    color_type: ColorType,
    props: SurfaceProps,
    /// The texture wrapped on the recorder of the context: what the layer
    /// is read and blitted through, and what a direct frame is drawn on.
    surface: RefCell<Option<Surface>>,
    texture: RefCell<Option<Arc<LayerTexture>>>,
    policy: RefCell<PipelinePolicy>,
    frame: Cell<Frame>,
    last_frame: Cell<Option<LastFrame>>,
    /// [`GraphiteGrContext::direct_only_draws`] when the layer last looked.
    direct_only_draws: Cell<u64>,
    /// How long the threads worked on the frame collected last.
    busy: Cell<Duration>,
    /// The frame the threads are playing.
    pending: RefCell<Option<PendingFrame>>,
}

impl PipelinedLayer {
    fn release(&self) {
        self.flush_pending();
        let had_surface = self.surface.borrow_mut().take().is_some();
        if had_surface {
            let _ = self.context.with_band_parts(|parts| parts.pool.forget(self.id));
        }
        *self.texture.borrow_mut() = None;
    }

    /// Whether only the recorder of the context draws what was drawn with
    /// the context since the layer last asked.
    fn had_direct_only_draws(&self) -> bool {
        let draws = self.context.direct_only_draws();
        self.direct_only_draws.replace(draws) != draws
    }

    /// The beginning of a frame. Returns whether it is pipelined.
    fn begin_frame(&self, now: Instant) -> bool {
        let previous_cost = match self.last_frame.get() {
            Some(LastFrame::Direct { begun }) => {
                self.context.last_flush_end().and_then(|end| end.checked_duration_since(begun))
            }
            Some(LastFrame::Recorded { cost }) => Some(cost),
            None => None,
        };
        let direct_only = self.had_direct_only_draws();

        let (was_pipelined, pipelined) = {
            let mut policy = self.policy.borrow_mut();
            if direct_only {
                policy.frame_needed_direct();
            }
            (policy.is_pipelined(), policy.frame_begins(now, previous_cost))
        };

        if pipelined {
            if !was_pipelined {
                ferroui_base::perf_count!(SkiaLayersBeganToPipeline);
            }
            self.frame.set(Frame::Recorded);
        } else {
            // The frame is drawn on top of the one the threads are playing.
            self.flush_pending();
            self.frame.set(Frame::DirectBegun);
            self.last_frame.set(Some(LastFrame::Direct { begun: now }));
        }
        pipelined
    }

    /// The end of a pipelined frame: the picture goes to the threads, or is
    /// played on the recorder of the context.
    fn finish_frame(&self, picture: Option<Picture>, started: Instant, leases: &RecordedLeases) {
        self.frame.set(Frame::None);
        let Some(picture) = picture else {
            return;
        };
        let recorded = started.elapsed();

        let direct_only = self.had_direct_only_draws() || leases.used();
        let played = if direct_only { Err(Fallback::DirectOnly) } else { self.play_bands(&picture) };
        match played {
            Ok(()) => {
                ferroui_base::perf_count!(SkiaFramesPipelined);
            }
            Err(reason) => {
                ferroui_base::perf_count!(SkiaPipelinedFramesPlayedDirectly);
                self.flush_pending();
                self.play_on_main(&picture, None);
                if reason != Fallback::Disposed {
                    self.policy.borrow_mut().frame_needed_direct();
                }
            }
        }
        self.last_frame.set(Some(LastFrame::Recorded { cost: recorded + self.busy.get() }));
    }

    /// Receives the frame the threads are playing, which waits until it is
    /// in the context: what is drawn or read next comes after it.
    fn flush_pending(&self) {
        let Some(previous) = self.pending.borrow_mut().take() else {
            return;
        };
        let collected = self
            .context
            .with_band_parts(|parts| collect_bands(parts.pool, parts.recording_failed, self.id, &previous));
        // `None`: the context is gone, and the threads with it.
        let Some((missing, busy)) = collected else {
            return;
        };
        self.busy.set(busy);
        if !missing.is_empty() {
            self.play_on_main(&previous.picture, Some(missing.as_slice()));
            self.policy.borrow_mut().frame_needed_direct();
        }
    }

    /// Hands the picture to the threads. `Err` tells why it was not.
    fn play_bands(&self, picture: &Picture) -> Result<(), Fallback> {
        let Some(texture) = self.texture.borrow().clone() else {
            return Err(Fallback::Disposed);
        };

        // The frame before this one is in the threads; its recordings come
        // before this frame's.
        self.flush_pending();

        let played = self.context.with_band_parts(|parts| {
            let BandParts { pool, recorder, context, recording_failed } = parts;
            let threads = pool.ensure(context, self.bands);
            if threads == 0 {
                return Err(Fallback::PoolUnavailable);
            }
            // A band has a thread of its own: a layer that asks for more
            // bands than the first pipelined layer of the context did has
            // as many as there are threads.
            let clips = band_clips(self.size, self.bands.min(threads));

            // What the recorder of the context has pending comes before the
            // bands: an earlier frame it drew on the texture, and the images
            // it made that the picture draws. It comes after the bands of
            // other layers that were told to insert (`wait_for_released`).
            pool.wait_for_released();
            let Some(mut pending) = recorder.snap() else {
                return Err(Fallback::MainRecording);
            };
            let inserted = insert(&mut context.lock(), &mut pending);
            drop(pending);
            if !inserted {
                *recording_failed = true;
                return Err(Fallback::MainRecording);
            }

            let mut sent = 0;
            for (index, clip) in &clips {
                let band = Band {
                    layer: self.id,
                    texture: texture.clone(),
                    size: self.size,
                    color_type: self.color_type,
                    props: self.props,
                    picture: picture.clone(),
                    clip: *clip,
                    index: *index,
                    turn: 0,
                };
                if pool.play(band) {
                    sent += 1;
                }
            }
            // A band that was not sent is found missing when the frame is
            // collected, and played on the recorder of the context then.
            *self.pending.borrow_mut() = Some(PendingFrame { clips, sent, picture: picture.clone() });
            Ok(())
        });

        played.unwrap_or(Err(Fallback::Disposed))
    }

    /// Plays the picture on the recorder of the context, as an ordinary
    /// layer draws its frame: all of it, or the given bands of it.
    fn play_on_main(&self, picture: &Picture, bands: Option<&[IRect]>) {
        let Some(mut surface) = self.surface.borrow().clone() else {
            return;
        };
        let canvas = surface.canvas();
        canvas.restore_to_count(1);
        canvas.reset_matrix();
        match bands {
            None => {
                canvas.draw_picture(picture, None, None);
            }
            Some(bands) => {
                for clip in bands {
                    draw_band(canvas, picture, *clip);
                }
            }
        }
    }
}

impl Drop for PipelinedLayer {
    fn drop(&mut self) {
        self.release();
    }
}

/// The surface of a frame layer that can pipeline its frames (see the
/// module).
pub(crate) struct PipelinedLayerSurface(Rc<PipelinedLayer>);

impl PipelinedLayerSurface {
    /// A layer of the given size on a texture of `device` (an
    /// `id<MTLDevice>`), with the surface properties of an ordinary layer.
    /// Its pipelined frames are divided into `bands` bands, or as many as
    /// fit it ([`band_count`]). `None` when `mode` is
    /// [`SkiaFramePipelining::Off`], when the layer is too low to be
    /// divided, and when the texture cannot be made: the layer is then an
    /// ordinary one.
    pub(crate) fn try_new(
        context: &Rc<GraphiteGrContext>,
        device: *mut c_void,
        size: PixelSize,
        props: &SurfaceProps,
        mode: SkiaFramePipelining,
        bands: usize,
    ) -> Option<Self> {
        if mode == SkiaFramePipelining::Off || device.is_null() || context.is_disposed() {
            return None;
        }
        // The size of the surface of an ordinary layer.
        let (width, height) = (size.width.max(1), size.height.max(1));
        let bands = band_count(height, bands)?;

        // The colour type of an ordinary layer, in the format of the texture.
        let color_type = ColorType::N32;
        let rgba = if color_type == ColorType::RGBA8888 {
            1
        } else if color_type == ColorType::BGRA8888 {
            0
        } else {
            return None;
        };

        // SAFETY: `device` is the `id<MTLDevice>` of the GPU that asks for
        // the layer, alive for the call; the function has no other
        // precondition and returns null when it makes no texture.
        let texture = unsafe { ferro_skia_layer_texture_new(device, width, height, rgba) };
        if texture.is_null() {
            return None;
        }
        let texture = Arc::new(LayerTexture(texture));

        let mut surface = context
            .with_recorder(|recorder| wrap_texture(recorder, &texture, (width, height), color_type, props))?;
        // A new texture holds nothing defined, and no recorder clears a
        // texture it wraps. The clear is the first work of the recorder of
        // the context for the texture, so it is inserted before any band.
        surface.canvas().clear(Color::TRANSPARENT);

        Some(Self(Rc::new(PipelinedLayer {
            context: context.clone(),
            id: NEXT_LAYER_ID.fetch_add(1, Ordering::Relaxed),
            size: (width, height),
            bands,
            color_type,
            props: *props,
            surface: RefCell::new(Some(surface)),
            texture: RefCell::new(Some(texture)),
            policy: RefCell::new(PipelinePolicy::new(mode)),
            frame: Cell::new(Frame::None),
            last_frame: Cell::new(None),
            direct_only_draws: Cell::new(context.direct_only_draws()),
            busy: Cell::new(Duration::ZERO),
            pending: RefCell::new(None),
        })))
    }
}

impl PipelinedLayerSurface {
    /// How many bands the layer was made to divide a pipelined frame into.
    #[cfg(test)]
    pub(crate) fn bands(&self) -> usize {
        self.0.bands
    }
}

impl ISkiaSurface for PipelinedLayerSurface {
    /// The surface of the layer on the recorder of the context. Outside the
    /// drawing of a direct frame this is a read of the layer: the frame the
    /// threads are playing is received first, so the reader sees every frame
    /// that has ended. A read while a frame is drawn sees what a direct
    /// frame has drawn so far and nothing of a recorded one, so it sends the
    /// layer back to drawing directly.
    fn surface(&self) -> Surface {
        let layer = &self.0;
        match layer.frame.get() {
            Frame::DirectBegun => layer.frame.set(Frame::Direct),
            Frame::None => layer.flush_pending(),
            Frame::Direct | Frame::Recorded => {
                ferroui_base::perf_count!(SkiaPipelinedLayerReadsInFrames);
                layer.flush_pending();
                layer.policy.borrow_mut().frame_needed_direct();
            }
        }
        layer.surface.borrow().clone().expect("PipelinedLayerSurface has been disposed")
    }

    fn can_blit(&self) -> bool {
        true
    }

    /// Draws the layer onto the canvas, which draws with the recorder of
    /// the context: the texture itself is drawn, not a copy of it. The draw
    /// is inserted with the recording of that recorder by the next flush,
    /// and a frame the threads are playing is inserted after that flush.
    fn blit(&self, canvas: &Canvas) {
        let Some(mut surface) = self.0.surface.borrow().clone() else {
            return;
        };
        let image = match graphite::surfaces::as_image(&surface) {
            Some(image) => image,
            None => surface.image_snapshot(),
        };
        self.0.context.note_layer_blit();
        let old_matrix = canvas.local_to_device();
        canvas.reset_matrix();
        canvas.draw_image_with_sampling_options(image, Point::new(0.0, 0.0), SamplingOptions::default(), None);
        canvas.set_matrix(&old_matrix);
    }

    fn dispose(&self) {
        self.0.release();
    }

    fn begin_recorded_frame(&self) -> Option<CanvasSource> {
        let layer = self.0.clone();
        if layer.surface.borrow().is_none() {
            return None;
        }
        let started = Instant::now();
        if !layer.begin_frame(started) {
            return None;
        }

        let (width, height) = layer.size;
        let bounds = Rect::new(0.0, 0.0, width as f32, height as f32);
        let mut recorder = PictureRecorder::new();
        recorder.begin_recording(bounds, false);
        let leases = Rc::new(RecordedLeases::new(bounds));

        let frame_leases = leases.clone();
        Some(CanvasSource::Recorder {
            recorder,
            on_finished: Box::new(move |picture| layer.finish_frame(picture, started, &frame_leases)),
            leases: Some(leases),
        })
    }

    fn end_drawn_frame(&self) {
        let layer = &self.0;
        if matches!(layer.frame.get(), Frame::DirectBegun | Frame::Direct) {
            layer.frame.set(Frame::None);
        }
    }

    fn has_pending_frame(&self) -> bool {
        self.0.pending.borrow().is_some()
    }

    fn complete_pending_frame(&self) {
        self.0.flush_pending();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const FRAME: Duration = Duration::from_micros(16_667);

    /// Runs `frames` frames `period` apart, each costing `cost`, and returns
    /// what the policy answered for each.
    fn run(policy: &mut PipelinePolicy, now: &mut Instant, frames: u32, period: Duration, cost: Duration) -> Vec<bool> {
        (0..frames)
            .map(|_| {
                *now += period;
                policy.frame_begins(*now, Some(cost))
            })
            .collect()
    }

    fn adaptive() -> (PipelinePolicy, Instant) {
        let mut policy = PipelinePolicy::new(SkiaFramePipelining::Adaptive);
        let now = Instant::now();
        // The first frame of a stream has no period.
        assert!(!policy.frame_begins(now, None));
        (policy, now)
    }

    #[test]
    fn a_layer_pipelines_after_four_late_frames_in_a_row() {
        let (mut policy, mut now) = adaptive();
        // 27 ms of drawing every other tick of a 60 Hz display.
        let answers = run(&mut policy, &mut now, 5, FRAME * 2, Duration::from_millis(27));
        assert_eq!(vec![false, false, false, true, true], answers);
    }

    #[test]
    fn a_late_frame_costs_half_its_period_and_four_milliseconds_at_least() {
        let (mut policy, mut now) = adaptive();
        // Less than half the period.
        assert!(run(&mut policy, &mut now, 20, FRAME, Duration::from_millis(8)).iter().all(|pipelined| !pipelined));
        // Half the period of a 240 Hz display, and under the floor.
        let short = Duration::from_micros(4_167);
        assert!(run(&mut policy, &mut now, 20, short, Duration::from_millis(3)).iter().all(|pipelined| !pipelined));
        // A frame in time between the late ones begins the count again.
        for _ in 0..5 {
            assert!(run(&mut policy, &mut now, 3, FRAME, Duration::from_millis(12)).iter().all(|pipelined| !pipelined));
            assert!(!run(&mut policy, &mut now, 1, FRAME, Duration::from_millis(2))[0]);
        }
        // A frame whose cost is not known is not late.
        now += FRAME;
        assert!(!policy.frame_begins(now, None));
        assert_eq!(vec![false, false, false, true], run(&mut policy, &mut now, 4, FRAME, Duration::from_millis(12)));
    }

    #[test]
    fn a_pipelined_layer_draws_directly_again_after_ninety_light_frames() {
        let (mut policy, mut now) = adaptive();
        assert!(run(&mut policy, &mut now, 4, FRAME, Duration::from_millis(12))[3]);

        // Between the thresholds the layer stays as it is.
        assert!(run(&mut policy, &mut now, 200, FRAME, Duration::from_millis(7)).iter().all(|pipelined| *pipelined));
        // Light frames, with one that is not among them.
        assert!(run(&mut policy, &mut now, 89, FRAME, Duration::from_millis(4)).iter().all(|pipelined| *pipelined));
        assert!(run(&mut policy, &mut now, 1, FRAME, Duration::from_millis(7))[0]);
        let answers = run(&mut policy, &mut now, 90, FRAME, Duration::from_millis(4));
        assert!(answers[..89].iter().all(|pipelined| *pipelined));
        assert!(!answers[89]);
        assert!(!policy.is_pipelined());
    }

    #[test]
    fn a_pause_begins_a_new_stream_that_is_drawn_directly() {
        let (mut policy, mut now) = adaptive();
        assert!(run(&mut policy, &mut now, 4, FRAME, Duration::from_millis(12))[3]);

        now += STREAM_GAP + Duration::from_millis(1);
        assert!(!policy.frame_begins(now, Some(Duration::from_millis(12))));
        assert_eq!(vec![false, false, false, true], run(&mut policy, &mut now, 4, FRAME, Duration::from_millis(12)));
    }

    #[test]
    fn a_frame_that_needs_the_recorder_of_the_context_holds_the_layer_off() {
        let (mut policy, mut now) = adaptive();
        assert!(run(&mut policy, &mut now, 4, FRAME, Duration::from_millis(12))[3]);

        policy.frame_needed_direct();
        assert!(!policy.is_pipelined());
        let answers = run(&mut policy, &mut now, HOLD_OFF_FRAMES + 1, FRAME, Duration::from_millis(12));
        assert!(answers[..HOLD_OFF_FRAMES as usize - 1].iter().all(|pipelined| !pipelined));
        assert!(answers[HOLD_OFF_FRAMES as usize - 1]);

        // A layer whose frames keep needing it never pipelines.
        let (mut policy, mut now) = adaptive();
        for _ in 0..(HOLD_OFF_FRAMES * 3) {
            policy.frame_needed_direct();
            assert!(!run(&mut policy, &mut now, 1, FRAME, Duration::from_millis(12))[0]);
        }
    }

    #[test]
    fn the_options_can_ask_for_pipelined_frames_always_or_never() {
        let mut now = Instant::now();

        let mut policy = PipelinePolicy::new(SkiaFramePipelining::On);
        assert!(policy.is_pipelined());
        assert!(policy.frame_begins(now, None));
        policy.frame_needed_direct();
        assert!(run(&mut policy, &mut now, 3, STREAM_GAP * 2, Duration::ZERO).iter().all(|pipelined| *pipelined));

        let mut policy = PipelinePolicy::new(SkiaFramePipelining::Off);
        assert!(!run(&mut policy, &mut now, 10, FRAME, Duration::from_millis(30)).iter().any(|pipelined| *pipelined));
    }

    #[test]
    fn the_threads_insert_in_the_order_the_bands_were_sent_in() {
        let order = Arc::new(InsertOrder::default());
        let inserted = Arc::new(Mutex::new(Vec::new()));

        // The later a turn, the sooner its thread asks for it.
        let threads: Vec<_> = (0..6u64)
            .rev()
            .map(|number| {
                let turn = Turn { order: order.clone(), number };
                let inserted = inserted.clone();
                let thread = thread::spawn(move || {
                    drop(turn.wait());
                    inserted.lock().unwrap().push(number);
                });
                thread::sleep(Duration::from_millis(2));
                thread
            })
            .collect();
        for thread in threads {
            thread.join().unwrap();
        }

        assert_eq!(vec![0, 1, 2, 3, 4, 5], *inserted.lock().unwrap());
    }

    #[test]
    fn a_turn_that_is_dropped_passes_to_the_next() {
        let order = Arc::new(InsertOrder::default());

        // A band that failed: its turn is dropped without a recording.
        let failed = Turn { order: order.clone(), number: 0 };
        let next = Turn { order: order.clone(), number: 1 };
        let waiting = thread::spawn(move || {
            drop(next.wait());
        });
        thread::sleep(Duration::from_millis(2));
        assert!(!waiting.is_finished(), "the turn before it has not passed");
        drop(failed);
        waiting.join().unwrap();

        assert_eq!(2, *order.next.lock().unwrap());
    }
}
