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
//! On Graphite Vulkan (#1365) the layer's texture is an image of `ferroui-vulkan`
//! ([`LayerTexture`]), and the recorders of the threads draw on no shared
//! target: the bands insert from deferred canvases
//! ([`BandInsert::with_shared_target`]).
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
use super::AutoReleasePool;
use crate::gpu::ISkiaSurface;
use crate::{SkiaFramePipelining, MAX_FRAME_PIPELINING_BANDS};
use ferroui_base::PixelSize;
#[cfg(target_vendor = "apple")]
use skia_safe::gpu::graphite::mtl::backend_textures;
use skia_safe::gpu::graphite::{self, InsertRecordingInfo};
use skia_safe::{
    AlphaType, BlendMode, Canvas, ClipOp, Color, ColorType, IRect, Image, Paint, Picture, Point, Rect,
    SamplingOptions, Surface, SurfaceProps,
};
use std::cell::{Cell, RefCell};
#[cfg(target_vendor = "apple")]
use std::ffi::c_void;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::rc::Rc;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use super::band_queue;
use std::sync::{Arc, Condvar, Mutex, MutexGuard, PoisonError};
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

/// Frames with a pause longer than this between them do not belong to one
/// stream: the layer forgets what it measured and draws the next one
/// directly. The pause is the time between the beginnings of two frames
/// that the first of them did not cost: a frame that takes longer than
/// this to draw is not a pause, it is the frame the policy is there for.
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
        // What the frame before cost is not part of the pause: frames that
        // each take longer than the gap, one after the other, are a stream.
        let paused = |period: &Duration| period.saturating_sub(previous_cost.unwrap_or_default()) > STREAM_GAP;
        let Some(period) = period.filter(|period| !paused(period)) else {
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

#[cfg(target_vendor = "apple")]
extern "C" {
    /// A new `id<MTLTexture>` of the device, drawn to and sampled, with one
    /// reference that the caller owns; null when it cannot be made
    /// (`pipelined_layer_texture.m`).
    fn ferro_skia_layer_texture_new(device: *mut c_void, width: i32, height: i32, rgba: i32) -> *mut c_void;

    /// Gives back the reference of `ferro_skia_layer_texture_new`.
    fn ferro_skia_layer_texture_release(texture: *mut c_void);
}

/// The texture of a pipelined layer, which the recorder of the context and
/// the threads of the bands wrap in surfaces of their own.
pub(crate) struct LayerTexture(LayerTextureKind);

enum LayerTextureKind {
    /// One owned reference to an `id<MTLTexture>`. Every surface that wraps
    /// the texture holds a reference of its own, so the texture outlives
    /// this one for as long as it is drawn.
    #[cfg(target_vendor = "apple")]
    Metal(*mut c_void),
    /// An image of `ferroui-vulkan` and the one Graphite backend texture of
    /// it that every wrap copies: the copies share Graphite's record of the
    /// image's layout, so every recorder that draws on or inserts onto the
    /// image sees the layout the last insertion left (the insertions run
    /// one at a time, under the lock of the context). Nothing of the GPU's
    /// work holds the image, so the layer hands it to the context when it
    /// lets it go ([`GraphiteGrContext::retire`]), which destroys it after
    /// the GPU has finished with it. The backend texture is declared first:
    /// it goes before the image it describes.
    #[cfg(feature = "graphite-vulkan")]
    Vulkan {
        texture: graphite::BackendTexture,
        /// Held for its lifetime: the image the backend texture describes.
        #[allow(dead_code)]
        image: ferroui_vulkan::VulkanOffscreenImage,
    },
}

// SAFETY: on Metal the pointer is an `id<MTLTexture>`, an object Metal lets
// any thread retain, use and release; the value only hands the pointer to
// Skia, which takes a reference of its own, and releases its one reference
// when it is dropped, on whichever thread that is. On Vulkan the image is
// `Send + Sync`, and the backend texture is a description of it with a
// reference-counted (atomically) record of its layout, which is read and
// changed only by insertions, under the lock of the context; copies of it are
// made on any thread, which only adds a reference to that record.
unsafe impl Send for LayerTexture {}
// SAFETY: as above; the value has no interior state of its own.
unsafe impl Sync for LayerTexture {}

impl LayerTexture {
    /// A texture of the Metal device `device` (an `id<MTLDevice>`, alive for
    /// the call); `None` when Metal makes none.
    #[cfg(target_vendor = "apple")]
    pub(crate) fn metal(device: *mut c_void, width: i32, height: i32, rgba: bool) -> Option<Self> {
        if device.is_null() {
            return None;
        }
        // SAFETY: `device` is the `id<MTLDevice>` of the GPU that asks for
        // the layer, alive for the call; the function has no other
        // precondition and returns null when it makes no texture.
        let texture = unsafe { ferro_skia_layer_texture_new(device, width, height, i32::from(rgba)) };
        (!texture.is_null()).then(|| Self(LayerTextureKind::Metal(texture)))
    }

    /// A texture of the device of `vulkan`: an image with the usage Graphite
    /// asks for (input attachment besides the default usage), owned by the
    /// queue family of the device's main queue, in the layout `UNDEFINED`
    /// (the first draw of the layer clears it). Made without a command and
    /// without a wait; `None` when the device makes no image.
    #[cfg(feature = "graphite-vulkan")]
    pub(crate) fn vulkan(
        vulkan: &dyn ferroui_vulkan::IVulkanPlatformGraphicsContext,
        width: i32,
        height: i32,
        rgba: bool,
    ) -> Option<Self> {
        use ferroui_vulkan::ash::vk;
        let format = if rgba { vk::Format::R8G8B8A8_UNORM } else { vk::Format::B8G8R8A8_UNORM };
        let image = ferroui_vulkan::VulkanOffscreenImage::new(
            vulkan,
            PixelSize::new(width, height),
            format.as_raw() as u32,
            vk::ImageUsageFlags::INPUT_ATTACHMENT.as_raw(),
        )
        .ok()?;
        let queue_family = vulkan.device().graphics_queue_family_index();
        let texture = crate::gpu::vulkan::graphite_backend_texture(&image.image_info(), queue_family);
        Some(Self(LayerTextureKind::Vulkan { texture, image }))
    }

    /// The Graphite backend texture of the texture, of the given size, for a
    /// wrap or for its description.
    fn backend_texture(&self, size: (i32, i32)) -> graphite::BackendTexture {
        match &self.0 {
            // SAFETY: the value holds a reference to the `id<MTLTexture>` for
            // the duration of the call, and the backend texture is used only
            // while the value is alive (a wrap, in which Graphite takes a
            // reference of its own, or a read of its description).
            #[cfg(target_vendor = "apple")]
            LayerTextureKind::Metal(texture) => unsafe { backend_textures::make_metal(size, *texture) },
            #[cfg(feature = "graphite-vulkan")]
            LayerTextureKind::Vulkan { texture, .. } => {
                let _ = size;
                graphite::BackendTexture::from_backend_texture(texture)
            }
        }
    }

    /// Whether the GPU's work keeps the texture alive by itself (Metal): a
    /// Vulkan image is destroyed when the value goes, so the layer lets the
    /// context keep it until the GPU has finished with it.
    fn is_held_by_the_gpu(&self) -> bool {
        match &self.0 {
            #[cfg(target_vendor = "apple")]
            LayerTextureKind::Metal(_) => true,
            #[cfg(feature = "graphite-vulkan")]
            LayerTextureKind::Vulkan { .. } => false,
        }
    }
}

impl Drop for LayerTexture {
    fn drop(&mut self) {
        match &self.0 {
            // SAFETY: the pointer came from `ferro_skia_layer_texture_new`,
            // was not null (a null one is never wrapped in this type), and
            // its one reference is given back exactly once, here.
            #[cfg(target_vendor = "apple")]
            LayerTextureKind::Metal(texture) => unsafe { ferro_skia_layer_texture_release(*texture) },
            // The fields go by themselves: the backend texture, then the image.
            #[cfg(feature = "graphite-vulkan")]
            LayerTextureKind::Vulkan { .. } => {}
        }
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
    let backend_texture = texture.backend_texture(size);
    graphite::surfaces::wrap_backend_texture(recorder, &backend_texture, color_type, None, Some(props))
}

/// The `bands` bands of a layer of the given size as equal strips, which is what a layer begins with
/// ([`BandBalance`]): the index of each band and the rows it draws, from the top. The bands are disjoint and
/// cover the layer.
pub(crate) fn band_clips(size: (i32, i32), bands: usize) -> Vec<(usize, IRect)> {
    let (width, height) = size;
    BandBalance::new(height, bands).clips(width)
}

/// The rows the cost of the frames of a layer is kept for at a time ([`BandBalance`]); the row two bands meet
/// at is a multiple of it once the bands were moved.
const BALANCE_ROWS: i32 = 16;

/// How many frames the time of the bands is averaged over before the bands are looked at.
const BALANCE_FRAMES: u32 = 8;

/// The frames after a move that are not counted: a band draws the glyphs that came into it for the first
/// time, which its recorder rasterizes and uploads.
const BALANCE_SETTLE_FRAMES: u32 = 2;

/// The time of the threads on a frame below which the bands stay where they are: nothing waits for the
/// slowest band of such a frame.
const MIN_BALANCED_COST: Duration = Duration::from_millis(2);

/// How much longer than the mean of the bands the slowest band works before the bands are moved.
const BALANCE_SLACK: f64 = 0.15;

/// How much of what is kept about the rows inside a band goes back to an even spread at each look: a frame
/// tells the time of a band, not where in the band it went, and what was learned from other bands grows old.
const BALANCE_FORGET: f64 = 0.05;

/// By how much a move has to lower the time of the slowest band against the mean for the next look to follow
/// at once.
const BALANCE_GAIN: f64 = 0.03;

/// The most looks that are skipped after moves that did not help.
const MAX_BALANCE_HOLD: u32 = 64;

/// The share of the time between two frames of a layer that its slowest band works, from which the layer
/// takes a band more ([`BandBalance::recount`]). A frame is in the context when its slowest band is; a band
/// that works for most of the time between two frames is what the next frame waits for.
const GROW_SHARE: f64 = 0.6;

/// How many looks in a row the slowest band has to work that long before the layer takes a band more.
const GROW_LOOKS: u32 = 3;

/// By how much a band more has to lower the time of the slowest band. A band that does not is given back,
/// and the looks that follow take none, twice as many each time: a cost that cannot be divided stays in
/// one band however many there are.
const GROW_GAIN: f64 = 0.1;

/// The share of the time between two frames that the bands may work, one fewer of them, for the layer to
/// give a band back: well below [`GROW_SHARE`], so that a layer does not take and give back in turn.
const SHRINK_SHARE: f64 = 0.4;

/// What a band of one band fewer is taken to work more than its even share of the work of all of them.
const SHRINK_MARGIN: f64 = 1.25;

/// How many looks in a row the bands have to be that light before the layer gives a band back.
const SHRINK_LOOKS: u32 = 8;

/// The frames after a change of the number of bands that are not counted: every band draws glyphs that are
/// new to its recorder.
const COUNT_SETTLE_FRAMES: u32 = 4;

/// The bands a layer may take when it is asked for `asked` of them (docs/porting/desktop-performance.md,
/// section 9.92). A layer that was asked for the default number follows what its frames cost: it may take
/// bands up to the performance cores of the machine less the UI thread and the render thread, and two more
/// than the default at most. A number an application chose is kept as it is.
pub(crate) fn most_bands(asked: usize) -> usize {
    if asked != crate::skia_options::DEFAULT_FRAME_PIPELINING_BANDS {
        return asked;
    }
    asked.max(performance_cores().saturating_sub(2).min(asked + 2)).min(crate::skia_options::MAX_FRAME_PIPELINING_BANDS)
}

/// The performance cores of the machine: the threads that record bands are worth their cost only on a core
/// that is as fast as the one the frame waits on.
fn performance_cores() -> usize {
    static CORES: std::sync::OnceLock<usize> = std::sync::OnceLock::new();
    *CORES.get_or_init(|| {
        #[cfg(any(target_os = "macos", target_os = "ios"))]
        if let Some(cores) = performance_cores_of_the_system() {
            return cores;
        }
        // Without a count of the system: half the logical processors.
        thread::available_parallelism().map_or(1, |count| count.get() / 2).max(1)
    })
}

/// `hw.perflevel0.physicalcpu`: the cores of the fastest kind.
#[cfg(any(target_os = "macos", target_os = "ios"))]
fn performance_cores_of_the_system() -> Option<usize> {
    use std::ffi::{c_char, c_int};
    unsafe extern "C" {
        fn sysctlbyname(
            name: *const c_char,
            old: *mut c_void,
            old_length: *mut usize,
            new: *mut c_void,
            new_length: usize,
        ) -> c_int;
    }
    let mut cores: i32 = 0;
    let mut length = size_of::<i32>();
    // SAFETY: the name is a C string, the buffer is an `i32` of the length passed with it, which is what
    // the system writes for this name; nothing is set.
    let status = unsafe {
        sysctlbyname(
            c"hw.perflevel0.physicalcpu".as_ptr(),
            (&raw mut cores).cast(),
            &raw mut length,
            std::ptr::null_mut(),
            0,
        )
    };
    (status == 0 && cores > 0).then_some(cores as usize)
}

/// The rows the bands of a layer meet at, moved until the threads work as long as each other.
///
/// The bands begin as equal strips. What a frame costs is rarely spread evenly over the rows of a window, and
/// a frame is in the context when its slowest band is, so equal strips leave one thread waiting for another
/// (measured on a screen whose lower half holds most of the text: one band of two took three times the time
/// of the other, and the frame took the time of the slow one).
///
/// The threads tell how long each band took. From that the layer keeps what a strip of [`BALANCE_ROWS`] rows
/// costs: the strips of a band are scaled so that they add up to the time of the band, which is all a frame
/// tells. When the slowest band works [`BALANCE_SLACK`] longer than the mean, the bands are moved to the rows
/// that divide the kept cost evenly. The next frames are measured with other bands and correct the strips
/// again, so the rows close in on the cost wherever it lies, and the bands rest when what is kept agrees with
/// what is measured. A cost that cannot be divided (a strip of the layer lower than a band may be) would move
/// the bands for nothing: after a move that did not help, the looks that follow are skipped, twice as many
/// each time.
struct BandBalance {
    height: i32,
    /// The rows the bands meet at, from the top: one fewer than there are bands.
    edges: Vec<i32>,
    /// What each strip of [`BALANCE_ROWS`] rows costs a frame, in seconds, as far as the frames told.
    model: Vec<f64>,
    /// The time of each band over the frames since the last look, in seconds.
    busy: Vec<f64>,
    /// The frames `busy` adds up.
    frames: u32,
    /// The frames still left out after a move.
    settle: u32,
    /// The time of the slowest band against the mean when the bands were last moved, until the look after.
    before_move: Option<f64>,
    /// The looks that were skipped after the last move that did not help.
    hold: u32,
    /// The looks still to skip.
    skip: u32,
    /// The fewest and the most bands the layer has: the number it was asked for, and what it may take when
    /// its frames cost more than that many bands record in time ([`Self::recount`]). Equal for a layer
    /// whose number of bands is fixed.
    least: usize,
    most: usize,
    /// The time between the frames since the last look, in seconds, and how many frames told it.
    period: f64,
    periods: u32,
    /// The looks in a row at which the slowest band worked for most of the time between two frames.
    late: u32,
    /// The looks in a row at which one band fewer would have been enough.
    light: u32,
    /// The time of the slowest band before the layer took the band it took last, until the look after.
    grown: Option<f64>,
    /// The looks at which no band is taken after a band that did not help, and those still to pass.
    count_hold: u32,
    count_skip: u32,
}

impl BandBalance {
    /// `bands` equal strips of a layer of `height` rows.
    fn new(height: i32, bands: usize) -> Self {
        let bands = bands.max(1);
        let edges = (1..bands).map(|index| (i64::from(height) * index as i64 / bands as i64) as i32).collect();
        let strips = usize::try_from((height + BALANCE_ROWS - 1) / BALANCE_ROWS).unwrap_or(0).max(1);
        Self {
            height,
            edges,
            model: vec![1.0; strips],
            busy: vec![0.0; bands],
            frames: 0,
            settle: 0,
            before_move: None,
            hold: 0,
            skip: 0,
            least: bands,
            most: bands,
            period: 0.0,
            periods: 0,
            late: 0,
            light: 0,
            grown: None,
            count_hold: 0,
            count_skip: 0,
        }
    }

    /// `least` equal strips of a layer of `height` rows that may become up to `most`.
    fn following(height: i32, least: usize, most: usize) -> Self {
        let mut balance = Self::new(height, least);
        balance.most = most.max(balance.least);
        balance
    }

    /// Whether the layer has `least` bands at least and `most` at most.
    fn follows(&self, least: usize, most: usize) -> bool {
        self.least == least.max(1) && self.most == most.max(least.max(1))
    }

    fn bands(&self) -> usize {
        self.busy.len()
    }

    /// The index of each band and the rows it draws in a layer `width` pixels wide, from the top.
    fn clips(&self, width: i32) -> Vec<(usize, IRect)> {
        let mut clips = Vec::with_capacity(self.bands());
        self.clips_into(width, &mut clips);
        clips
    }

    /// [`clips`](Self::clips) into `clips`, which is emptied first: a frame reuses the vector of
    /// the frame before.
    fn clips_into(&self, width: i32, clips: &mut Vec<(usize, IRect)>) {
        clips.clear();
        let mut top = 0;
        for index in 0..self.bands() {
            let bottom = self.edges.get(index).copied().unwrap_or(self.height);
            let clip = IRect::new(0, top, width, bottom);
            if !clip.is_empty() {
                clips.push((index, clip));
            }
            top = bottom;
        }
    }

    /// How long the threads worked on the bands of a frame, by band. Returns whether the bands were moved.
    #[cfg(test)]
    fn frame(&mut self, busy: &[Duration]) -> bool {
        self.frame_after(busy, None)
    }

    /// How long the threads worked on the bands of a frame, by band, and the time since the frame before
    /// it when the two follow each other. Returns whether the bands are other ones for the next frame:
    /// moved, or one more or fewer.
    fn frame_after(&mut self, busy: &[Duration], period: Option<Duration>) -> bool {
        if busy.len() != self.bands() || (busy.len() < 2 && self.most < 2) {
            return false;
        }
        if self.settle > 0 {
            self.settle -= 1;
            return false;
        }
        for (sum, band) in self.busy.iter_mut().zip(busy) {
            *sum += band.as_secs_f64();
        }
        if let Some(period) = period {
            self.period += period.as_secs_f64();
            self.periods += 1;
        }
        self.frames += 1;
        if self.frames < BALANCE_FRAMES {
            return false;
        }
        let frames = f64::from(self.frames);
        let busy: Vec<f64> = self.busy.iter().map(|sum| sum / frames).collect();
        let period = (self.periods > 0).then(|| self.period / f64::from(self.periods));
        self.busy.fill(0.0);
        self.frames = 0;
        self.period = 0.0;
        self.periods = 0;
        if period.is_some_and(|period| self.recount(&busy, period)) {
            return true;
        }
        busy.len() >= 2 && self.look(&busy)
    }

    /// Looks at the time a frame took in each band against the time between two frames, and takes a band
    /// more or gives one back. Returns whether the number of bands changed.
    ///
    /// The layer has no clock of the display, as [`PipelinePolicy`] has none: what it knows is how long
    /// its bands work and how far apart its frames are. A slowest band that works for
    /// [`GROW_SHARE`] of that time is what the frames wait for, whether they come one display interval
    /// apart or two; bands that would fit [`SHRINK_SHARE`] of it with one fewer are more than the frame
    /// needs, and each costs a thread its fixed work for every frame.
    fn recount(&mut self, busy: &[f64], period: f64) -> bool {
        if self.most <= self.least || period <= 0.0 {
            return false;
        }
        let bands = busy.len();
        let slowest = busy.iter().copied().fold(0.0, f64::max);
        let total: f64 = busy.iter().sum();

        // The band taken last has to have lowered the slowest band.
        if let Some(before) = self.grown.take() {
            if slowest > before * (1.0 - GROW_GAIN) && bands > self.least {
                self.count_hold = (self.count_hold * 2).clamp(1, MAX_BALANCE_HOLD);
                self.count_skip = self.count_hold;
                self.resize(bands - 1);
                return true;
            }
            self.count_hold = 0;
        }

        let late = slowest >= period * GROW_SHARE && slowest >= MIN_LATE_COST.as_secs_f64();
        self.late = if late { self.late + 1 } else { 0 };
        if self.count_skip > 0 {
            self.count_skip -= 1;
        } else if self.late >= GROW_LOOKS && bands < self.most && self.before_move.is_none() {
            self.grown = Some(slowest);
            self.resize(bands + 1);
            return true;
        }

        let light = bands > self.least && total / (bands - 1) as f64 * SHRINK_MARGIN <= period * SHRINK_SHARE;
        self.light = if light { self.light + 1 } else { 0 };
        if self.light >= SHRINK_LOOKS {
            self.resize(bands - 1);
            return true;
        }
        false
    }

    /// Divides the layer into `bands` bands, at the rows that divide the cost kept so far evenly.
    fn resize(&mut self, bands: usize) {
        self.busy = vec![0.0; bands.clamp(self.least, self.most)];
        self.edges = if self.bands() > 1 { self.balanced_edges() } else { Vec::new() };
        self.frames = 0;
        self.period = 0.0;
        self.periods = 0;
        self.settle = COUNT_SETTLE_FRAMES;
        self.before_move = None;
        self.hold = 0;
        self.skip = 0;
        self.late = 0;
        self.light = 0;
    }

    /// Looks at the time a frame took in each band, and moves the bands when one of them is slow.
    fn look(&mut self, busy: &[f64]) -> bool {
        let mean = busy.iter().sum::<f64>() / busy.len() as f64;
        let slowest = busy.iter().copied().fold(0.0, f64::max);
        if mean <= 0.0 {
            return false;
        }
        self.learn(busy);
        if let Some(before) = self.before_move.take() {
            if slowest / mean > before * (1.0 - BALANCE_GAIN) {
                self.hold = (self.hold * 2).clamp(1, MAX_BALANCE_HOLD);
                self.skip = self.hold;
            } else {
                self.hold = 0;
            }
        }
        if self.skip > 0 {
            self.skip -= 1;
            return false;
        }
        let light = mean * (busy.len() as f64) < MIN_BALANCED_COST.as_secs_f64();
        if light || slowest <= mean * (1.0 + BALANCE_SLACK) {
            return false;
        }
        let edges = self.balanced_edges();
        if edges == self.edges {
            return false;
        }
        self.edges = edges;
        self.before_move = Some(slowest / mean);
        self.settle = BALANCE_SETTLE_FRAMES;
        true
    }

    /// The band each strip of the model is in: the band its middle row is in.
    fn bands_of_strips(&self) -> Vec<usize> {
        (0..self.model.len())
            .map(|strip| {
                let middle = strip as i32 * BALANCE_ROWS + BALANCE_ROWS / 2;
                self.edges.partition_point(|edge| *edge <= middle)
            })
            .collect()
    }

    /// Scales the strips of each band to the time the band took.
    fn learn(&mut self, busy: &[f64]) {
        let bands = self.bands_of_strips();
        let mut kept = vec![0.0; busy.len()];
        let mut strips = vec![0usize; busy.len()];
        for (strip, band) in bands.iter().enumerate() {
            kept[*band] += self.model[strip];
            strips[*band] += 1;
        }
        for (strip, band) in bands.iter().enumerate() {
            let even = busy[*band] / strips[*band].max(1) as f64;
            let scaled = if kept[*band] > 0.0 { self.model[strip] * busy[*band] / kept[*band] } else { even };
            self.model[strip] = scaled + (even - scaled) * BALANCE_FORGET;
        }
    }

    /// The rows that divide the kept cost evenly, each at the end of a strip and every band of
    /// [`MIN_BAND_HEIGHT`] rows at least.
    fn balanced_edges(&self) -> Vec<i32> {
        let bands = self.bands();
        let total: f64 = self.model.iter().sum();
        let mut edges: Vec<i32> = Vec::with_capacity(bands - 1);
        let (mut strip, mut above) = (0usize, 0.0);
        for index in 1..bands {
            let wanted = total * index as f64 / bands as f64;
            while strip < self.model.len() && above + self.model[strip] <= wanted {
                above += self.model[strip];
                strip += 1;
            }
            // The row of equal cost is inside this strip: the nearer end of the strip.
            let inside = match self.model.get(strip) {
                Some(cost) if *cost > 0.0 => (wanted - above) / cost,
                _ => 0.0,
            };
            let row = (strip as i32 + i32::from(inside > 0.5)) * BALANCE_ROWS;
            let least = edges.last().copied().unwrap_or(0) + MIN_BAND_HEIGHT;
            let most = self.height - (bands - index) as i32 * MIN_BAND_HEIGHT;
            edges.push(row.clamp(least, most.max(least)));
        }
        edges
    }
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
    /// With the band textures: the band's texture is new (its rows moved, or the layer's texture holds
    /// a frame the textures of the bands do not), and starts from these images, each with the rows of
    /// the layer it holds; drawn before the band's part of the picture.
    sources: Option<Vec<(IRect, BandImage)>>,
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
    /// The texture of the band, once its recording is in the context (`BandInsert::BandTextures`).
    image: Option<BandImage>,
}

/// A recording a thread keeps until it is told to insert it.
struct Held {
    layer: u64,
    index: usize,
    recording: Option<graphite::Recording>,
    busy: Duration,
    /// Where and when the recording goes in.
    placement: Placement,
    /// The texture the band was drawn on (`BandInsert::BandTextures`).
    image: Option<Image>,
}

/// Where and when a thread inserts the recording of a band.
enum Placement {
    /// In the order the bands were sent ([`BandInsert::Ordered`]).
    Turn(Turn),
    /// On the layer's texture at row `top` of the layer, when it is ready ([`BandInsert::Deferred`]).
    At { top: i32 },
    /// As it is, on the band's own texture, when it is ready ([`BandInsert::BandTextures`]).
    Own,
}

/// The texture of a band of a layer on the thread that draws it (`BandInsert::BandTextures`): a
/// surface of the thread's recorder of the band's size, kept while the band keeps its rows.
struct BandTexture {
    layer: u64,
    index: usize,
    /// The rows of the layer the texture holds.
    rows: IRect,
    surface: Surface,
}

/// The image of the texture of a band, on its way between the thread of the band and the render
/// thread (`BandInsert::BandTextures`).
struct BandImage(Image);

// SAFETY: an image is immutable and reference counted atomically, as every Skia object a picture
// holds is. The texture of a band's image is drawn on only by the thread of the band, and read by
// another recorder (the render thread's, or another band's when the bands move) only after the
// recording that drew it was inserted, which the order of the frame gives (see the module): the
// render thread reads it after it received the band's answer, and sends it to another band only with
// a later frame.
unsafe impl Send for BandImage {}

/// How the threads insert the recordings of the bands of a frame (#1363, a prototype: the order
/// stays the default until the deferred insertion passes the gate and the pixel tests).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum BandInsert {
    /// Each band draws on the layer's texture and the recordings go in in the order the bands
    /// were sent ([`InsertOrder`]).
    Ordered,
    /// Each band records on a deferred canvas of its own size, which the insertion places on the
    /// layer's texture at the band's row: no shared target while the threads record, and the
    /// recordings go in as they are ready, with no turn to wait for. Asked for with
    /// `FERROUI_SKIA_BAND_INSERT=deferred`.
    Deferred,
    /// Each band draws on a texture of its own, of its size, kept from frame to frame while the band
    /// keeps its rows, and its recording goes in as it is ready. The layer is the bands' textures
    /// side by side: it is blitted band by band, and copied onto the layer's texture only to be read
    /// or drawn directly. When the bands move, a band's new texture starts from a copy, on the GPU,
    /// of the rows it takes over from the textures of the bands before. Asked for with
    /// `FERROUI_SKIA_BAND_INSERT=textures` (#1363, variant B).
    BandTextures,
}

impl BandInsert {
    /// What the environment asks for when the threads are made; ordered unless it says otherwise.
    fn from_environment() -> Self {
        match std::env::var("FERROUI_SKIA_BAND_INSERT") {
            Ok(value) if value.trim() == "deferred" => Self::Deferred,
            Ok(value) if value.trim() == "textures" => Self::BandTextures,
            _ => Self::Ordered,
        }
    }

    /// The mode where the recorders of the threads may draw on one target at the same time
    /// (`shared`), or may not: Graphite on Vulkan does not allow it (#1365), and there the ordered
    /// insertion, whose bands all draw on the layer's texture, becomes the deferred one, which
    /// draws on no shared target. The other modes draw on targets of their own already.
    pub(crate) fn with_shared_target(self, shared: bool) -> Self {
        match self {
            Self::Ordered if !shared => Self::Deferred,
            mode => mode,
        }
    }
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
    jobs: band_queue::Receiver<BandJob>,
    results: band_queue::Sender<BandOutput>,
    /// The context the thread inserts its recordings into.
    context: Arc<SharedContext>,
    order: Arc<InsertOrder>,
    /// Counts the recordings this thread inserted, until a submit takes them.
    unsubmitted: Arc<AtomicUsize>,
    insert_mode: BandInsert,
}

impl BandWorker {
    fn run(self) {
        let BandWorker { recorder, jobs, results, context, order, unsubmitted, insert_mode } = self;
        let mut recorder = recorder.0;
        let mut held: Vec<Held> = Vec::new();
        // The surfaces that wrap the textures of the layers, by layer, with the textures (which go after
        // the surfaces that wrap them). They
        // belong to the recorder, so they are made, used and dropped here.
        let mut surfaces: BandSurfaces = Vec::new();
        // The textures of the bands of the layers this thread plays, with the band textures
        // (`BandInsert::BandTextures`): made, drawn and dropped here.
        let mut band_textures: Vec<BandTexture> = Vec::new();

        while let Some(job) = jobs.recv() {
            let pool = AutoReleasePool::new();
            match job {
                BandJob::Play(band) => {
                    let started = Instant::now();
                    let placement = match insert_mode {
                        BandInsert::Ordered => Placement::Turn(Turn { order: order.clone(), number: band.turn }),
                        BandInsert::Deferred => Placement::At { top: band.clip.top },
                        BandInsert::BandTextures => Placement::Own,
                    };
                    // A panic would leave the render thread waiting for an
                    // answer that never comes; it is reported as a band
                    // that failed, which ends the threads.
                    let played = catch_unwind(AssertUnwindSafe(|| match insert_mode {
                        BandInsert::Ordered => {
                            play(&mut recorder, &mut surfaces, &band).map(|recording| (recording, None))
                        }
                        BandInsert::Deferred => {
                            play_deferred(&mut recorder, &mut surfaces, &band).map(|recording| (recording, None))
                        }
                        BandInsert::BandTextures => play_band_texture(&mut recorder, &mut band_textures, &band)
                            .map(|(recording, image)| (recording, Some(image))),
                    }))
                    .unwrap_or(None);
                    let (recording, image) = match played {
                        Some((recording, image)) => (Some(recording), image),
                        None => (None, None),
                    };
                    let busy = started.elapsed();
                    let (layer, index) = (band.layer, band.index);
                    held.push(Held { layer, index, recording, busy, placement, image });
                    // The reference of the render thread outlives this one
                    // (see `Band`).
                    drop(band);
                }
                BandJob::Forget { layer } => {
                    surfaces.retain(|(id, ..)| *id != layer);
                    band_textures.retain(|texture| texture.layer != layer);
                }
                BandJob::Insert => {
                    let mut gone = false;
                    for Held { layer, index, recording, busy, placement, image } in held.drain(..) {
                        let recorded = recording.is_some();
                        if let Placement::Turn(turn) = &placement {
                            drop(turn.wait());
                        }
                        let target = surfaces.iter().find(|(id, ..)| *id == layer).map(|(_, surface, _)| surface);
                        let inserted = recording.is_some_and(|mut recording| {
                            catch_unwind(AssertUnwindSafe(|| match (&placement, target) {
                                (Placement::Turn(_) | Placement::Own, _) => insert(&mut context.lock(), &mut recording),
                                (Placement::At { top }, Some(target)) => {
                                    insert_at(&mut context.lock(), &mut recording, target, *top)
                                }
                                (Placement::At { .. }, None) => false,
                            }))
                            .unwrap_or(false)
                        });
                        if inserted {
                            unsubmitted.fetch_add(1, Ordering::Release);
                        }
                        // The turn passes on whatever became of the recording.
                        drop(placement);
                        // The texture of a band whose recording is not in the context shows nothing of the frame.
                        let image = image.filter(|_| inserted).map(BandImage);
                        gone |= results.send(BandOutput { layer, index, recorded, inserted, busy, image }).is_err();
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
        drop(band_textures);
        drop(recorder);
        pool.dispose();
    }
}

/// The surfaces a thread of the bands wrapped the textures of the layers in, by layer, each with its
/// texture: the texture outlives the surface that wraps it, also on the thread.
type BandSurfaces = Vec<(u64, Surface, Arc<LayerTexture>)>;

/// Plays a band on the recorder of its thread and snaps the recorder.
fn play(
    recorder: &mut graphite::Recorder,
    surfaces: &mut BandSurfaces,
    band: &Band,
) -> Option<graphite::Recording> {
    let position = match surfaces.iter().position(|(layer, ..)| *layer == band.layer) {
        Some(position) => position,
        None => {
            let surface = wrap_texture(recorder, &band.texture, band.size, band.color_type, &band.props)?;
            surfaces.push((band.layer, surface, band.texture.clone()));
            surfaces.len() - 1
        }
    };
    draw_band(surfaces[position].1.canvas(), &band.picture, band.clip);
    recorder.snap()
}

/// Plays a band on a deferred canvas of the band's size, its rows moved to the top, and snaps the
/// recorder; [`insert_at`] places the recording on the layer's texture. The surface that wraps the
/// texture is made for the insertion if the thread has none yet.
fn play_deferred(
    recorder: &mut graphite::Recorder,
    surfaces: &mut BandSurfaces,
    band: &Band,
) -> Option<graphite::Recording> {
    if !surfaces.iter().any(|(layer, ..)| *layer == band.layer) {
        let surface = wrap_texture(recorder, &band.texture, band.size, band.color_type, &band.props)?;
        surfaces.push((band.layer, surface, band.texture.clone()));
    }
    let texture_info = band.texture.backend_texture(band.size).info();
    let image_info =
        skia_safe::ImageInfo::new((band.clip.width(), band.clip.height()), band.color_type, AlphaType::Premul, None);
    {
        let canvas = recorder.make_deferred_canvas(&image_info, &texture_info)?;
        canvas.translate((0.0, -(band.clip.top as f32)));
        canvas.clip_rect(Rect::from(band.clip), ClipOp::Intersect, false);
        canvas.draw_picture(&band.picture, None, None);
    }
    recorder.snap()
}

/// Plays a band on its own texture (`BandInsert::BandTextures`) and snaps the recorder: the texture
/// the thread keeps for the band while it keeps its rows, or a new one of the band's size when the
/// band has sources, which are copied onto it first (a draw that replaces the pixels, on the GPU).
/// Returns the recording and the image of the texture.
fn play_band_texture(
    recorder: &mut graphite::Recorder,
    textures: &mut Vec<BandTexture>,
    band: &Band,
) -> Option<(graphite::Recording, Image)> {
    let kept = textures.iter().position(|texture| {
        texture.layer == band.layer && texture.index == band.index && texture.rows == band.clip
    });
    let position = match (kept, &band.sources) {
        (Some(position), None) => position,
        _ => {
            // A band without a kept texture always has sources; one without is drawn on a cleared texture.
            textures.retain(|texture| !(texture.layer == band.layer && texture.index == band.index));
            let info = skia_safe::ImageInfo::new(
                (band.clip.width(), band.clip.height()),
                band.color_type,
                AlphaType::Premul,
                None,
            );
            let mipmapped = skia_safe::gpu::Mipmapped::No;
            let surface = graphite::surfaces::render_target(recorder, &info, mipmapped, Some(&band.props), None)?;
            textures.push(BandTexture { layer: band.layer, index: band.index, rows: band.clip, surface });
            textures.len() - 1
        }
    };
    let surface = &mut textures[position].surface;
    let canvas = surface.canvas();
    canvas.save();
    canvas.translate((0.0, -(band.clip.top as f32)));
    if let Some(sources) = &band.sources {
        canvas.clear(Color::TRANSPARENT);
        let mut copy = Paint::default();
        copy.set_blend_mode(BlendMode::Src);
        for (rows, image) in sources {
            canvas.draw_image_with_sampling_options(
                &image.0,
                Point::new(rows.left as f32, rows.top as f32),
                SamplingOptions::default(),
                Some(&copy),
            );
        }
    }
    canvas.clip_rect(Rect::from(band.clip), ClipOp::Intersect, false);
    canvas.draw_picture(&band.picture, None, None);
    canvas.restore();
    let recording = recorder.snap()?;
    let surface = &textures[position].surface;
    let image = graphite::surfaces::as_image(surface)?;
    Some((recording, image))
}

/// Inserts a recording of a deferred canvas onto `target` at row `top` (`fTargetSurface`,
/// `fTargetTranslation`, which skia-safe does not set; through `skia-bindings`, as for
/// `fRequireOrderedRecordings`, desktop-performance.md section 9.17).
fn insert_at(context: &mut graphite::Context, recording: &mut graphite::Recording, target: &Surface, top: i32) -> bool {
    // The wrapper holds the pointer to the native structure and a marker of no size: of the size of a pointer,
    // the pointer is at offset 0. A surface handle is `repr(transparent)` over the pointer to its native surface.
    const _: () = assert!(
        std::mem::size_of::<InsertRecordingInfo<'static>>() == std::mem::size_of::<*mut ()>()
    );
    let mut info = InsertRecordingInfo::new(recording);
    // SAFETY: by the assertion the first word of the wrapper is the native structure it owns; the target surface
    // outlives the insertion, which takes a reference of its own.
    unsafe {
        let native =
            *(&mut info as *mut InsertRecordingInfo).cast::<*mut skia_bindings::skgpu_graphite_InsertRecordingInfo>();
        (*native).fTargetSurface = *(target as *const Surface).cast::<*mut skia_bindings::SkSurface>();
        (*native).fTargetTranslation = skia_bindings::SkIPoint { fX: 0, fY: top };
    }
    context.insert_recording(&info) == graphite::InsertStatus::Success
}

struct WorkerHandle {
    jobs: band_queue::Sender<BandJob>,
    thread: thread::JoinHandle<()>,
}

/// The threads that play the bands of the pipelined layers of a context,
/// made when a layer first pipelines a frame and joined when the context
/// goes.
#[derive(Default)]
pub(super) struct BandPool {
    workers: Vec<WorkerHandle>,
    results: Option<band_queue::Receiver<BandOutput>>,
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
    /// The band recordings the threads inserted that no submit has taken yet.
    unsubmitted: Arc<AtomicUsize>,
    /// How the threads insert, chosen when they are made (`BandInsert::from_environment`), unless
    /// a test chose before.
    insert_mode: Option<BandInsert>,
    /// The recorders of the threads may not draw on one target at the same time (Graphite on
    /// Vulkan): see [`BandInsert::with_shared_target`].
    no_shared_target: bool,
}

impl BandPool {
    /// Takes the count of the band recordings inserted since the last submit: what a submit now
    /// sends to the GPU (`GraphiteGrContext::submit_inserted_bands`, and every flush).
    pub(super) fn take_unsubmitted(&self) -> usize {
        self.unsubmitted.swap(0, Ordering::Acquire)
    }

    /// How the threads insert, once they were made.
    pub(super) fn insert_mode(&self) -> Option<BandInsert> {
        self.insert_mode.map(|mode| mode.with_shared_target(!self.no_shared_target))
    }

    /// Tells the pool that the recorders of its threads may not draw on one target at the same
    /// time (Graphite on Vulkan), before the threads are made.
    #[cfg_attr(not(feature = "graphite-vulkan"), allow(dead_code))]
    pub(super) fn forbid_shared_target(&mut self) {
        self.no_shared_target = true;
    }

    /// Chooses how threads made from now on insert (for the tests).
    #[cfg(test)]
    pub(super) fn set_insert_mode(&mut self, mode: BandInsert) {
        self.insert_mode = Some(mode);
    }

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

        let (results_sender, results) = band_queue::channel();
        let insert_mode = self
            .insert_mode
            .get_or_insert_with(BandInsert::from_environment)
            .with_shared_target(!self.no_shared_target);
        for index in 0..bands.max(1) {
            let Some(recorder) = context.lock().make_recorder(None) else {
                self.disable();
                return 0;
            };
            let (jobs, job_receiver) = band_queue::channel();
            let worker = BandWorker {
                recorder: BandRecorder(recorder),
                jobs: job_receiver,
                results: results_sender.clone(),
                context: context.clone(),
                order: self.order.clone(),
                unsubmitted: self.unsubmitted.clone(),
                insert_mode,
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
            let output = self.results.as_ref()?.recv()?;
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
            let Some(output) = self.results.as_ref().and_then(|results| results.recv()) else {
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
    /// The bands of the frame that were played: those the frame's dirty rows touch.
    clips: Vec<(usize, IRect)>,
    /// How many bands the layer has; `clips` has fewer when the frame left clean bands out.
    bands: usize,
    sent: usize,
    /// The time since the frame before this one was handed to the threads, when the two follow each
    /// other.
    period: Option<Duration>,
    /// Held until the bands have answered, so that what the picture holds is
    /// released on the render thread, and for the bands that fail.
    picture: Picture,
    /// The bands drew on textures of their own (`BandInsert::BandTextures`).
    band_textures: bool,
}

/// With the band textures: the bands of a frame that its dirty rows touch, and those whose texture is
/// new (`fresh`: their rows moved, or the textures do not hold the frame the layer shows), which are
/// played to fill it. A band that keeps its texture and that the frame cannot change is left out.
fn band_texture_clips(
    mut clips: Vec<(usize, IRect)>,
    rows: Option<(i32, i32)>,
    fresh: impl Fn(usize, IRect) -> bool,
) -> Vec<(usize, IRect)> {
    clips.retain(|(index, clip)| {
        rows.is_none_or(|(top, bottom)| clip.top < bottom && clip.bottom > top) || fresh(*index, *clip)
    });
    clips
}

/// With the band textures: the band textures of the frame before (by band, each with the rows it
/// holds) that a new texture of `rows` copies from: those that hold any of its rows.
fn band_texture_sources(kept: &[Option<(IRect, Image)>], rows: IRect) -> Vec<(IRect, BandImage)> {
    kept.iter()
        .flatten()
        .filter(|(held, _)| held.top < rows.bottom && held.bottom > rows.top)
        .map(|(held, image)| (*held, BandImage(image.clone())))
        .collect()
}

/// The bands of a frame that its dirty rows touch, all of them without rows.
fn dirty_band_clips(mut clips: Vec<(usize, IRect)>, rows: Option<(i32, i32)>) -> Vec<(usize, IRect)> {
    if let Some((top, bottom)) = rows {
        clips.retain(|(_, clip)| clip.top < bottom && clip.bottom > top);
    }
    clips
}

/// What the bands of a frame walk and draw: every band walks every draw of the frame's picture
/// (`walked`), and draws those whose device rows meet its own (`inside`).
fn band_draw_counts(rows: &[(f32, f32)], clips: &[(usize, IRect)]) -> (u64, u64) {
    let walked = (rows.len() * clips.len()) as u64;
    let inside = clips
        .iter()
        .map(|(_, clip)| {
            let (top, bottom) = (clip.top as f32, clip.bottom as f32);
            rows.iter().filter(|(draw_top, draw_bottom)| *draw_top < bottom && *draw_bottom > top).count() as u64
        })
        .sum();
    (walked, inside)
}

/// How long the threads worked on each band of a frame, by the index of the band: on the stack.
struct BandTimes {
    times: [Duration; MAX_FRAME_PIPELINING_BANDS],
    bands: usize,
}

impl BandTimes {
    fn as_slice(&self) -> &[Duration] {
        &self.times[..self.bands]
    }
}

/// Whether the times of a collected frame move the bands: only a frame of which every band was
/// played and sent. A band left out because the frame did not change it took no time, and is not
/// a cheap band; counting it would move the band edges back and forth.
fn feeds_balance(frame: &PendingFrame) -> bool {
    frame.clips.len() == frame.bands && frame.sent == frame.bands
}

/// Receives the answers for the bands of a frame, which waits until the
/// threads have inserted them. Returns the bands that are not in the
/// context, and how long the threads worked on each band of the frame.
fn collect_bands(
    pool: &mut BandPool,
    recording_failed: &mut bool,
    layer: u64,
    pending: &PendingFrame,
    images: &mut Vec<(usize, Image)>,
) -> (Vec<IRect>, BandTimes) {
    // The bands of a frame that no flush has told to insert yet: a frame
    // that was drawn and not shown.
    pool.release();
    // By the index of the band; the indices of the bands of a frame are
    // below their number.
    let bands = pending.bands.min(MAX_FRAME_PIPELINING_BANDS);
    let mut inserted = [false; MAX_FRAME_PIPELINING_BANDS];
    let mut busy = BandTimes { times: [Duration::ZERO; MAX_FRAME_PIPELINING_BANDS], bands };
    let inserted = &mut inserted[..bands];
    for _ in 0..pending.sent {
        let Some(output) = pool.receive(layer) else {
            break;
        };
        if let Some(band) = busy.times[..bands].get_mut(output.index) {
            *band = output.busy;
        }
        if let Some(image) = output.image {
            images.push((output.index, image.0));
        }
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

    // Allocates only when a band is missing, which ends the threads.
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
    /// How many bands it may be divided into when its frames cost more than
    /// `bands` of them record in time ([`most_bands`]).
    most: usize,
    /// When the frame before was handed to the threads.
    last_played: Cell<Option<Instant>>,
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
    /// Where the bands of the next pipelined frame meet.
    balance: RefCell<BandBalance>,
    /// The frame the threads are playing.
    pending: RefCell<Option<PendingFrame>>,
    /// The paint of a blit that replaces the target's pixels (`BlendMode::Src`), made once.
    src_paint: Paint,
    /// The rows the next frame can change, as the compositor tells them
    /// (`ISkiaSurface::set_frame_dirty_rows`); taken by the frame when it begins.
    dirty_rows: Cell<Option<(i32, i32)>>,
    /// The rows the recorded frame can change: the bands outside them are not played.
    frame_rows: Cell<Option<(i32, i32)>>,
    /// The vector of the bands of the frame before, reused by the next.
    clips_scratch: RefCell<Vec<(usize, IRect)>>,
    /// The leases of the frame before, reused by the next when nothing else holds them.
    leases: RefCell<Option<Rc<RecordedLeases>>>,
    /// The device rows of the draws of the recorded frame (with the feature `perf-counters`).
    draw_rows: RefCell<Vec<(f32, f32)>>,
    /// With the band textures (`BandInsert::BandTextures`): the texture of each band, by band, with
    /// the rows of the layer it holds.
    band_images: RefCell<Vec<Option<(IRect, Image)>>>,
    /// Where the frame the layer shows is: on the layer's texture, on the textures of the bands, or
    /// on both.
    latest: Cell<Latest>,
}

/// Where the frame a layer shows is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Latest {
    /// On the layer's texture (every mode but the band textures, and a band-texture layer after a frame
    /// drawn directly).
    Surface,
    /// On the textures of the bands only.
    Bands,
    /// On both: the textures of the bands were copied onto the layer's texture for a read.
    Both,
}

impl PipelinedLayer {
    /// Draws the layer onto the canvas, which draws with the recorder of the context: the texture
    /// itself is drawn, not a copy of it, over the canvas (`SrcOver`) or, with `paint`, as the
    /// paint blends. The draw is inserted with the recording of that recorder by the next flush,
    /// and a frame the threads are playing is inserted after that flush.
    fn blit(&self, canvas: &Canvas, paint: Option<&Paint>) {
        let Some(mut surface) = self.surface.borrow().clone() else {
            return;
        };
        if self.latest.get() == Latest::Bands {
            // The textures of the bands side by side, not a copy of them.
            self.context.note_layer_blit();
            let old_matrix = canvas.local_to_device();
            canvas.reset_matrix();
            self.draw_band_images(canvas, paint);
            canvas.set_matrix(&old_matrix);
            return;
        }
        let image = match graphite::surfaces::as_image(&surface) {
            Some(image) => image,
            None => surface.image_snapshot(),
        };
        self.context.note_layer_blit();
        let old_matrix = canvas.local_to_device();
        canvas.reset_matrix();
        canvas.draw_image_with_sampling_options(image, Point::new(0.0, 0.0), SamplingOptions::default(), paint);
        canvas.set_matrix(&old_matrix);
    }

    /// Draws the textures of the bands at their rows onto `canvas`, whose matrix is the identity.
    fn draw_band_images(&self, canvas: &Canvas, paint: Option<&Paint>) {
        for (rows, image) in self.band_images.borrow().iter().flatten() {
            let at = Point::new(rows.left as f32, rows.top as f32);
            canvas.draw_image_with_sampling_options(image, at, SamplingOptions::default(), paint);
        }
    }

    /// Makes the layer's texture hold the frame the layer shows, for a read of it (`drawn`: false) or a
    /// frame drawn on it: the textures of the bands are copied onto it (on the GPU, replacing its pixels)
    /// when only they hold the frame. A frame drawn on the texture leaves the bands' textures behind.
    fn make_surface_latest(&self, drawn: bool) {
        let latest = self.latest.get();
        if latest == Latest::Bands {
            if let Some(mut surface) = self.surface.borrow().clone() {
                let canvas = surface.canvas();
                canvas.restore_to_count(1);
                canvas.reset_matrix();
                self.draw_band_images(canvas, Some(&self.src_paint));
            }
        }
        if latest != Latest::Surface {
            self.latest.set(if drawn { Latest::Surface } else { Latest::Both });
        }
    }

    fn release(&self) {
        self.flush_pending();
        self.band_images.borrow_mut().clear();
        self.latest.set(Latest::Surface);
        let had_surface = self.surface.borrow_mut().take().is_some();
        if had_surface {
            let _ = self.context.with_band_parts(|parts| parts.pool.forget(self.id));
        }
        let texture = self.texture.borrow_mut().take();
        if let Some(texture) = texture.filter(|texture| !texture.is_held_by_the_gpu()) {
            // A draw of the layer may still be in the recorder of the context, and the GPU may still be drawing
            // what was submitted: the context keeps the image until the GPU has finished with it.
            self.context.retire(Box::new(texture));
        }
    }

    /// Whether only the recorder of the context draws what was drawn with
    /// the context since the layer last asked.
    fn had_direct_only_draws(&self) -> bool {
        let draws = self.context.direct_only_draws();
        self.direct_only_draws.replace(draws) != draws
    }

    /// The beginning of a frame. Returns whether it is pipelined.
    fn begin_frame(&self, now: Instant) -> bool {
        // The bands of the frame before were inserted by the threads after the last flush; they go to the GPU now,
        // while this frame is recorded, not with this frame's flush.
        self.context.submit_inserted_bands();
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
                if ferroui_base::diagnostics::perf_counters::ENABLED {
                    // The draws of a frame the bands did not walk are not counted.
                    crate::drawing_context_impl::take_band_draw_rows(&mut self.draw_rows.borrow_mut());
                }
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
        let mut images = Vec::new();
        let collected = self.context.with_band_parts(|parts| {
            collect_bands(parts.pool, parts.recording_failed, self.id, &previous, &mut images)
        });
        // `None`: the context is gone, and the threads with it.
        let Some((missing, busy)) = collected else {
            return;
        };
        self.busy.set(busy.as_slice().iter().sum());
        if previous.band_textures {
            self.take_band_images(&previous, images, missing.is_empty());
        }
        if !missing.is_empty() {
            self.play_on_main(&previous.picture, Some(missing.as_slice()));
            self.policy.borrow_mut().frame_needed_direct();
        } else if feeds_balance(&previous)
            && self.balance.borrow_mut().frame_after(busy.as_slice(), previous.period)
        {
            ferroui_base::perf_count!(SkiaBandsMoved);
        }
        // The vector of the bands goes back to the layer for the next frame.
        *self.clips_scratch.borrow_mut() = previous.clips;
    }

    /// Keeps the textures the bands of a frame drew on (`BandInsert::BandTextures`): the textures of the
    /// layer from now on, with those of the bands the frame left out. When a band is missing, the frame
    /// goes on on the layer's texture instead: the textures of the frame before, then those of this
    /// frame are copied onto it, and the missing bands are played on it after.
    fn take_band_images(&self, frame: &PendingFrame, images: Vec<(usize, Image)>, complete: bool) {
        let rows_of = |index: usize| frame.clips.iter().find(|(band, _)| *band == index).map(|(_, rows)| *rows);
        if !complete {
            self.make_surface_latest(true);
            if let Some(mut surface) = self.surface.borrow().clone() {
                let canvas = surface.canvas();
                for (index, image) in &images {
                    if let Some(rows) = rows_of(*index) {
                        let at = Point::new(rows.left as f32, rows.top as f32);
                        let sampling = SamplingOptions::default();
                        canvas.draw_image_with_sampling_options(image, at, sampling, Some(&self.src_paint));
                    }
                }
            }
            self.band_images.borrow_mut().clear();
            return;
        }
        let mut kept = self.band_images.borrow_mut();
        for (index, image) in images {
            let Some(rows) = rows_of(index) else { continue };
            if kept.len() <= index {
                kept.resize(index + 1, None);
            }
            kept[index] = Some((rows, image));
        }
        // A layer with fewer bands than before keeps no texture of the bands it no longer has.
        kept.truncate(frame.bands);
        self.latest.set(Latest::Bands);
    }

    /// Hands the picture to the threads. `Err` tells why it was not.
    fn play_bands(&self, picture: &Picture) -> Result<(), Fallback> {
        let Some(texture) = self.texture.borrow().clone() else {
            return Err(Fallback::Disposed);
        };

        // The frame before this one is in the threads; its recordings come
        // before this frame's.
        self.flush_pending();

        // How far apart the frames are, when one follows the other: a
        // frame after a pause tells nothing.
        let now = Instant::now();
        let period = self
            .last_played
            .replace(Some(now))
            .map(|last| now.saturating_duration_since(last))
            .filter(|period| *period <= STREAM_GAP);

        let played = self.context.with_band_parts(|parts| {
            let BandParts { pool, recorder, context, recording_failed } = parts;
            let threads = pool.ensure(context, self.most);
            if threads == 0 {
                return Err(Fallback::PoolUnavailable);
            }
            let band_textures = pool.insert_mode() == Some(BandInsert::BandTextures);
            // A band has a thread of its own: a layer that asks for more
            // bands than the first pipelined layer of the context did has
            // as many as there are threads.
            let (clips, bands) = {
                let mut balance = self.balance.borrow_mut();
                let (least, most) = (self.bands.min(threads), self.most.min(threads));
                if !balance.follows(least, most) {
                    *balance = BandBalance::following(self.size.1, least, most);
                }
                let mut clips = std::mem::take(&mut *self.clips_scratch.borrow_mut());
                balance.clips_into(self.size.0, &mut clips);
                let bands = clips.len();
                let rows = self.frame_rows.take();
                if band_textures {
                    // A band keeps its texture while it keeps its rows and the textures hold the frame the layer
                    // shows; the bands the frame cannot change are left out then, as on one texture.
                    let current = self.latest.get() != Latest::Surface;
                    let kept = self.band_images.borrow();
                    let fresh = |index: usize, clip: IRect| {
                        !current || kept.get(index).and_then(Option::as_ref).is_none_or(|(rows, _)| *rows != clip)
                    };
                    (band_texture_clips(clips, rows, fresh), bands)
                } else {
                    // The bands the frame cannot change keep their pixels: the layer has one texture, which every
                    // frame draws on, so a band left out still shows the frame before, and every frame since that
                    // drew it.
                    (dirty_band_clips(clips, rows), bands)
                }
            };

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

            // With the band textures, what a new texture of a band starts from: the textures of the bands before
            // when they hold the frame the layer shows, else the layer's texture.
            let current = self.latest.get() != Latest::Surface;
            let layer_image = (band_textures && !current)
                .then(|| self.surface.borrow().as_ref().and_then(graphite::surfaces::as_image))
                .flatten();
            let mut sent = 0;
            for (index, clip) in &clips {
                let sources = band_textures
                    .then(|| {
                        let kept = self.band_images.borrow();
                        let kept_rows = kept.get(*index).and_then(Option::as_ref).map(|(rows, _)| *rows);
                        let keeps = current && kept_rows == Some(*clip);
                        match (keeps, &layer_image) {
                            (true, _) => None,
                            (false, Some(image)) if !current => {
                                let all = IRect::from_wh(self.size.0, self.size.1);
                                Some(vec![(all, BandImage(image.clone()))])
                            }
                            (false, _) => Some(band_texture_sources(&kept, *clip)),
                        }
                    })
                    .flatten();
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
                    sources,
                };
                if pool.play(band) {
                    sent += 1;
                }
            }
            if ferroui_base::diagnostics::perf_counters::ENABLED {
                let mut rows = self.draw_rows.borrow_mut();
                crate::drawing_context_impl::take_band_draw_rows(&mut rows);
                let (walked, inside) = band_draw_counts(&rows, &clips);
                use ferroui_base::diagnostics::perf_counters::{add, PerfCounter};
                add(PerfCounter::SkiaBandDrawsWalked, walked);
                add(PerfCounter::SkiaBandDrawsInside, inside);
            }
            // A band that was not sent is found missing when the frame is
            // collected, and played on the recorder of the context then.
            *self.pending.borrow_mut() =
                Some(PendingFrame { clips, bands, sent, period, picture: picture.clone(), band_textures });
            Ok(())
        });

        played.unwrap_or(Err(Fallback::Disposed))
    }

    /// Plays the picture on the recorder of the context, as an ordinary
    /// layer draws its frame: all of it, or the given bands of it.
    fn play_on_main(&self, picture: &Picture, bands: Option<&[IRect]>) {
        self.make_surface_latest(true);
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
    #[cfg(target_vendor = "apple")]
    pub(crate) fn try_new(
        context: &Rc<GraphiteGrContext>,
        device: *mut c_void,
        size: PixelSize,
        props: &SurfaceProps,
        mode: SkiaFramePipelining,
        bands: usize,
    ) -> Option<Self> {
        Self::try_with_texture(context, size, props, mode, bands, |width, height, rgba| {
            LayerTexture::metal(device, width, height, rgba)
        })
    }

    /// [`try_new`](Self::try_new) on a texture that `make_texture` makes of the width, the height
    /// and the order of the channels (red first, or blue first) of the layer: a texture of Metal
    /// or of Vulkan ([`LayerTexture`]).
    pub(crate) fn try_with_texture(
        context: &Rc<GraphiteGrContext>,
        size: PixelSize,
        props: &SurfaceProps,
        mode: SkiaFramePipelining,
        bands: usize,
        make_texture: impl FnOnce(i32, i32, bool) -> Option<LayerTexture>,
    ) -> Option<Self> {
        if mode == SkiaFramePipelining::Off || context.is_disposed() {
            return None;
        }
        // The size of the surface of an ordinary layer.
        let (width, height) = (size.width.max(1), size.height.max(1));
        let bands = band_count(height, bands)?;
        // What the layer may take when its frames cost more: as many as fit it.
        let most = band_count(height, most_bands(bands)).map_or(bands, |most| most.max(bands));

        // The colour type of an ordinary layer, in the format of the texture.
        let color_type = ColorType::N32;
        let rgba = if color_type == ColorType::RGBA8888 {
            true
        } else if color_type == ColorType::BGRA8888 {
            false
        } else {
            return None;
        };

        let texture = Arc::new(make_texture(width, height, rgba)?);

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
            most,
            last_played: Cell::new(None),
            color_type,
            props: *props,
            surface: RefCell::new(Some(surface)),
            texture: RefCell::new(Some(texture)),
            policy: RefCell::new(PipelinePolicy::new(mode)),
            frame: Cell::new(Frame::None),
            last_frame: Cell::new(None),
            direct_only_draws: Cell::new(context.direct_only_draws()),
            busy: Cell::new(Duration::ZERO),
            balance: RefCell::new(BandBalance::following(height, bands, most)),
            pending: RefCell::new(None),
            dirty_rows: Cell::new(None),
            frame_rows: Cell::new(None),
            clips_scratch: RefCell::new(Vec::new()),
            leases: RefCell::new(None),
            draw_rows: RefCell::new(Vec::new()),
            band_images: RefCell::new(Vec::new()),
            latest: Cell::new(Latest::Surface),
            src_paint: {
                let mut paint = Paint::default();
                paint.set_blend_mode(BlendMode::Src);
                paint
            },
        })))
    }
}

impl PipelinedLayerSurface {
    /// How many bands the layer was made to divide a pipelined frame into.
    #[cfg(test)]
    pub(crate) fn bands(&self) -> usize {
        self.0.bands
    }

    /// The bands the next pipelined frame of the layer is divided into.
    #[cfg(test)]
    pub(crate) fn band_clips(&self) -> Vec<(usize, IRect)> {
        self.0.balance.borrow().clips(self.0.size.0)
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
            Frame::DirectBegun => {
                layer.frame.set(Frame::Direct);
                layer.make_surface_latest(true);
            }
            Frame::None => {
                layer.flush_pending();
                layer.make_surface_latest(false);
            }
            Frame::Direct | Frame::Recorded => {
                ferroui_base::perf_count!(SkiaPipelinedLayerReadsInFrames);
                layer.flush_pending();
                layer.make_surface_latest(true);
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
        self.0.blit(canvas, None);
    }

    /// The full-window layer replaces the pixels of the window's canvas: with the `Src` blend mode
    /// (its cached paint) the GPU does not read the target, which a tile-based GPU otherwise loads,
    /// and the clear before it is left out.
    fn blit_replacing(&self, canvas: &Canvas) -> bool {
        self.0.blit(canvas, Some(&self.0.src_paint));
        true
    }

    fn set_frame_dirty_rows(&self, rows: Option<(i32, i32)>) {
        self.0.dirty_rows.set(rows);
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
        // The rows belong to this frame alone, whether it is recorded or drawn directly.
        let rows = layer.dirty_rows.take();
        if !layer.begin_frame(started) {
            return None;
        }
        layer.frame_rows.set(rows);

        let (width, height) = layer.size;
        let bounds = Rect::new(0.0, 0.0, width as f32, height as f32);
        // The recorder and the leases of an earlier frame, when they are free again.
        let mut recorder = crate::drawing_context_impl::take_picture_recorder();
        recorder.begin_recording(bounds, false);
        let kept = layer.leases.borrow_mut().take().filter(|leases| Rc::strong_count(leases) == 1);
        let leases = match kept {
            Some(leases) => {
                leases.reset();
                leases
            }
            None => Rc::new(RecordedLeases::new(bounds)),
        };
        *layer.leases.borrow_mut() = Some(leases.clone());

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

    /// While the layer pipelines its frames: a recorded frame needs nothing
    /// of the rendering session, and is on screen a tick after it is drawn
    /// either way. A layer that draws directly keeps the order of a frame,
    /// which is what its cost is measured in.
    fn draws_ahead_of_target(&self) -> bool {
        self.0.surface.borrow().is_some() && self.0.policy.borrow().is_pipelined()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn with_band_textures_a_band_is_played_when_the_frame_changes_it_or_its_texture_is_new() {
        let clips = band_clips((320, 256), 4);
        let rows = |index: usize| clips[index].1;
        // Nothing new: the dirty rows choose (the rows of the third band only).
        let played = band_texture_clips(clips.clone(), Some((150, 160)), |_, _| false);
        assert_eq!(played, vec![(2, rows(2))]);
        // A band whose texture is new is played although the frame does not change it.
        let played = band_texture_clips(clips.clone(), Some((150, 160)), |index, _| index == 0);
        assert_eq!(played, vec![(0, rows(0)), (2, rows(2))]);
        // A frame without rows plays every band.
        assert_eq!(band_texture_clips(clips.clone(), None, |_, _| false), clips);
    }

    #[test]
    fn a_new_band_texture_copies_from_the_textures_that_held_its_rows() {
        let image = skia_safe::surfaces::raster_n32_premul((4, 4)).expect("a raster surface").image_snapshot();
        let kept = vec![
            Some((IRect::from_ltrb(0, 0, 320, 100), image.clone())),
            None,
            Some((IRect::from_ltrb(0, 100, 320, 256), image)),
        ];
        let rows = |sources: Vec<(IRect, BandImage)>| sources.into_iter().map(|(rows, _)| rows).collect::<Vec<_>>();
        // A band that grew into the rows of the one below copies from both.
        assert_eq!(
            rows(band_texture_sources(&kept, IRect::from_ltrb(0, 0, 320, 140))),
            vec![IRect::from_ltrb(0, 0, 320, 100), IRect::from_ltrb(0, 100, 320, 256)]
        );
        // A band inside the rows of one texture copies from that one; rows that meet at an edge are not shared.
        assert_eq!(rows(band_texture_sources(&kept, IRect::from_ltrb(0, 140, 320, 256))), vec![
            IRect::from_ltrb(0, 100, 320, 256)
        ]);
        assert_eq!(rows(band_texture_sources(&kept, IRect::from_ltrb(0, 0, 320, 100))), vec![
            IRect::from_ltrb(0, 0, 320, 100)
        ]);
    }

    #[test]
    fn the_band_textures_are_asked_for_by_the_environment_and_draw_on_no_shared_target() {
        assert_eq!(BandInsert::BandTextures.with_shared_target(true), BandInsert::BandTextures);
        assert_eq!(BandInsert::BandTextures.with_shared_target(false), BandInsert::BandTextures);
    }

    #[test]
    fn without_a_shared_target_the_ordered_insertion_becomes_the_deferred_one() {
        // Metal: the recorders of the threads may draw on the layer's texture at the same time.
        assert_eq!(BandInsert::Ordered.with_shared_target(true), BandInsert::Ordered);
        assert_eq!(BandInsert::Deferred.with_shared_target(true), BandInsert::Deferred);
        // Graphite on Vulkan: they may not (#1365).
        assert_eq!(BandInsert::Ordered.with_shared_target(false), BandInsert::Deferred);
        assert_eq!(BandInsert::Deferred.with_shared_target(false), BandInsert::Deferred);
    }

    #[test]
    fn a_pool_without_a_shared_target_is_told_so_before_its_threads_are_made() {
        let mut pool = BandPool::default();
        assert!(!pool.no_shared_target);
        pool.forbid_shared_target();
        assert!(pool.no_shared_target);
    }

    const FRAME: Duration = Duration::from_micros(16_667);

    /// How long each band of `balance` works on a frame whose row `row` costs `cost(row)` seconds.
    fn band_times(balance: &BandBalance, cost: &dyn Fn(i32) -> f64) -> Vec<Duration> {
        let time = |clip: &IRect| Duration::from_secs_f64((clip.top..clip.bottom).map(cost).sum());
        balance.clips(100).iter().map(|(_, clip)| time(clip)).collect()
    }

    /// The time of the slowest band of such a frame against the mean of the bands.
    fn slowest_to_mean(balance: &BandBalance, cost: &dyn Fn(i32) -> f64) -> f64 {
        let times: Vec<f64> = band_times(balance, cost).iter().map(Duration::as_secs_f64).collect();
        times.iter().copied().fold(0.0, f64::max) / (times.iter().sum::<f64>() / times.len() as f64)
    }

    /// Draws `frames` such frames one after the other, each as far from the one before as a display that
    /// shows a frame every [`FRAME`] lets it be: one interval when its slowest band fits one, else as many
    /// as the band takes. Returns how many times the number of bands changed.
    fn draw_at_the_pace_of_the_display(balance: &mut BandBalance, frames: u32, cost: &dyn Fn(i32) -> f64) -> u32 {
        let mut changes = 0;
        for _ in 0..frames {
            let times = band_times(balance, cost);
            let slowest = times.iter().copied().max().unwrap_or_default();
            let intervals = (slowest.as_secs_f64() / FRAME.as_secs_f64()).ceil().max(1.0);
            let before = balance.bands();
            balance.frame_after(&times, Some(FRAME.mul_f64(intervals)));
            changes += u32::from(balance.bands() != before);
            let clips = balance.clips(100);
            assert_eq!(clips.len(), balance.bands());
            assert_eq!(clips[clips.len() - 1].1.bottom, balance.height);
            for (index, (_, clip)) in clips.iter().enumerate() {
                assert!(clip.height() >= MIN_BAND_HEIGHT, "band {index} is {} rows", clip.height());
            }
        }
        changes
    }

    /// Section 9.92: a layer whose slowest band works for most of the time between two frames takes a
    /// band more, up to the most it may have, and gives the bands back when the frames are light again.
    #[test]
    fn a_layer_takes_bands_while_its_frames_wait_for_the_slowest_and_gives_them_back() {
        let mut balance = BandBalance::following(1024, 2, 4);
        // 48 ms of work a frame, spread evenly: 24 ms a band of two, 16 of three, 12 of four.
        let heavy = |_row: i32| 0.048 / 1024.0;
        draw_at_the_pace_of_the_display(&mut balance, 400, &heavy);
        assert_eq!(4, balance.bands(), "as many as it may have");
        let changes = draw_at_the_pace_of_the_display(&mut balance, 400, &heavy);
        assert_eq!(0, changes, "and it rests there");

        // 3 ms of work a frame: two bands are more than enough.
        let light = |_row: i32| 0.003 / 1024.0;
        draw_at_the_pace_of_the_display(&mut balance, 600, &light);
        assert_eq!(2, balance.bands(), "the number it was asked for");
        assert_eq!(0, draw_at_the_pace_of_the_display(&mut balance, 400, &light));
    }

    /// A frame that fits its interval with the bands it has keeps them: 16 ms of work in two bands.
    #[test]
    fn a_layer_whose_frames_fit_keeps_its_bands() {
        let mut balance = BandBalance::following(1024, 2, 4);
        let fits = |_row: i32| 0.016 / 1024.0;
        assert_eq!(0, draw_at_the_pace_of_the_display(&mut balance, 600, &fits));
        assert_eq!(2, balance.bands());
    }

    /// A cost that cannot be divided stays in one band: the band taken for it is given back, and the
    /// layer asks again at intervals that double.
    #[test]
    fn a_band_that_does_not_help_is_given_back() {
        let mut balance = BandBalance::following(1024, 2, 4);
        // All the work of the frame in one strip of rows, lower than a band may be.
        let knot = |row: i32| if row < BALANCE_ROWS { 0.030 / f64::from(BALANCE_ROWS) } else { 0.0 };
        let changes = draw_at_the_pace_of_the_display(&mut balance, 3_000, &knot);
        assert!((2..=24).contains(&changes), "{changes} changes of the number of bands");
        // By then the layer asks once in more than five hundred frames.
        let later = draw_at_the_pace_of_the_display(&mut balance, 500, &knot);
        assert!(later <= 2, "{later} changes in five hundred frames");
        assert!(balance.bands() <= 3);
    }

    /// A layer with a number of bands of its own keeps it whatever its frames cost.
    #[test]
    fn a_layer_with_a_fixed_number_of_bands_keeps_it() {
        let mut balance = BandBalance::new(1024, 2);
        let heavy = |_row: i32| 0.048 / 1024.0;
        assert_eq!(0, draw_at_the_pace_of_the_display(&mut balance, 600, &heavy));
        assert_eq!(2, balance.bands());
        assert_eq!(4, most_bands(4), "a number an application chose is kept");
        assert!(most_bands(crate::skia_options::DEFAULT_FRAME_PIPELINING_BANDS) >= 2);
    }

    /// Draws `frames` such frames, and returns how many times the bands were moved. The bands are checked
    /// after every frame: disjoint, the whole layer, none lower than a band may be.
    fn draw(balance: &mut BandBalance, frames: u32, cost: &dyn Fn(i32) -> f64) -> u32 {
        let mut moves = 0;
        for _ in 0..frames {
            let times = band_times(balance, cost);
            moves += u32::from(balance.frame(&times));
            let clips = balance.clips(100);
            assert_eq!(clips.len(), balance.bands());
            assert_eq!(clips[0].1.top, 0);
            assert_eq!(clips[clips.len() - 1].1.bottom, balance.height);
            for (index, (band, clip)) in clips.iter().enumerate() {
                assert_eq!(*band, index);
                assert!(clip.height() >= MIN_BAND_HEIGHT, "band {index} is {} rows", clip.height());
                assert!(index == 0 || clip.top == clips[index - 1].1.bottom);
            }
        }
        moves
    }

    /// A frame of 34 ms whose rows from `from` on cost `times` the rows above.
    fn heavy_from(height: i32, from: i32, times: f64) -> impl Fn(i32) -> f64 {
        let unit = 0.034 / (f64::from(from) + f64::from(height - from) * times);
        move |row| if row >= from { unit * times } else { unit }
    }

    #[test]
    fn the_bands_begin_as_equal_strips() {
        assert_eq!(band_clips((300, 1000), 1), vec![(0, IRect::new(0, 0, 300, 1000))]);
        assert_eq!(
            band_clips((300, 1001), 3),
            vec![
                (0, IRect::new(0, 0, 300, 333)),
                (1, IRect::new(0, 333, 300, 667)),
                (2, IRect::new(0, 667, 300, 1001)),
            ]
        );
    }

    #[test]
    fn bands_that_work_as_long_as_each_other_stay() {
        for bands in [2, 3, 4] {
            let mut balance = BandBalance::new(1600, bands);
            // Within the slack: one half costs a tenth more than the other.
            assert_eq!(draw(&mut balance, 400, &heavy_from(1600, 800, 1.1)), 0);
            assert_eq!(balance.clips(100), band_clips((100, 1600), bands));
        }
    }

    #[test]
    fn the_bands_of_light_frames_stay() {
        // Three times the cost in the lower half, of a frame the threads draw in a millisecond.
        let heavy = heavy_from(1600, 800, 3.0);
        let light = |row: i32| heavy(row) / 34.0;
        let mut balance = BandBalance::new(1600, 2);
        assert_eq!(draw(&mut balance, 400, &light), 0);
    }

    #[test]
    fn the_bands_move_to_where_the_cost_is() {
        for (bands, from, times) in [(2, 800, 3.0), (2, 1200, 8.0), (3, 800, 3.0), (4, 400, 0.2), (2, 300, 0.1)] {
            let cost = heavy_from(1600, from, times);
            let mut balance = BandBalance::new(1600, bands);
            let before = slowest_to_mean(&balance, &cost);
            let moves = draw(&mut balance, 800, &cost);
            let after = slowest_to_mean(&balance, &cost);
            assert!(before > 1.0 + BALANCE_SLACK, "{bands} bands, {times} from {from}: {before}");
            assert!(after <= 1.0 + BALANCE_SLACK, "{bands} bands, {times} from {from}: {before} to {after}");
            assert!((1..=12).contains(&moves), "{bands} bands, {times} from {from}: {moves} moves");
            // And they rest there.
            assert_eq!(draw(&mut balance, 400, &cost), 0);
        }
    }

    #[test]
    fn the_bands_follow_a_cost_that_moves() {
        let mut balance = BandBalance::new(1600, 2);
        for (from, times) in [(800, 3.0), (800, 0.25), (1100, 6.0), (800, 1.0), (500, 0.3)] {
            let cost = heavy_from(1600, from, times);
            let moves = draw(&mut balance, 800, &cost);
            let after = slowest_to_mean(&balance, &cost);
            assert!(after <= 1.0 + BALANCE_SLACK, "{times} from {from}: {after} after {moves} moves");
            assert!(moves <= 12, "{times} from {from}: {moves} moves");
        }
    }

    #[test]
    fn a_cost_that_cannot_be_divided_stops_moving_the_bands() {
        // All of a frame in sixteen rows: whichever band has them works alone.
        let cost = |row: i32| if (784..800).contains(&row) { 0.034 / 16.0 } else { 0.0 };
        for bands in [2, 3] {
            let mut balance = BandBalance::new(1600, bands);
            let moves = draw(&mut balance, 8 * 400, &cost);
            assert!(moves <= 16, "{bands} bands: {moves} moves in 400 looks");
        }
    }

    #[test]
    fn frames_after_a_move_and_of_other_bands_are_not_counted() {
        let cost = heavy_from(1600, 800, 3.0);
        let mut balance = BandBalance::new(1600, 2);
        // The times of three bands tell nothing about two.
        assert_eq!(draw(&mut BandBalance::new(1600, 3), 1, &cost), 0);
        for _ in 0..40 {
            assert!(!balance.frame(&[Duration::from_millis(9); 3]));
        }
        assert_eq!(balance.frames, 0);
        assert_eq!(draw(&mut balance, BALANCE_FRAMES - 1, &cost), 0);
        assert_eq!(draw(&mut balance, 1, &cost), 1);
        assert_eq!(balance.settle, BALANCE_SETTLE_FRAMES);
        assert_eq!(draw(&mut balance, BALANCE_SETTLE_FRAMES, &cost), 0);
        assert_eq!(balance.frames, 0);
    }

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

        // The pause is what the frame before it did not cost.
        now += STREAM_GAP + Duration::from_millis(12 + 1);
        assert!(!policy.frame_begins(now, Some(Duration::from_millis(12))));
        assert_eq!(vec![false, false, false, true], run(&mut policy, &mut now, 4, FRAME, Duration::from_millis(12)));

        // A frame of which nothing was measured cost nothing.
        now += STREAM_GAP + Duration::from_millis(1);
        assert!(!policy.frame_begins(now, None));
    }

    #[test]
    fn frames_that_each_take_longer_than_a_pause_are_a_stream() {
        let (mut policy, mut now) = adaptive();
        // 118 ms of drawing in 120 ms: the frames follow each other without a pause.
        let (period, cost) = (Duration::from_millis(120), Duration::from_millis(118));
        assert!(period > STREAM_GAP);
        assert_eq!(vec![false, false, false, true, true], run(&mut policy, &mut now, 5, period, cost));

        // The same frames with a pause after each never make a stream.
        let (mut policy, mut now) = adaptive();
        let period = cost + STREAM_GAP + Duration::from_millis(1);
        assert!(run(&mut policy, &mut now, 8, period, cost).iter().all(|pipelined| !pipelined));
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
    fn the_draws_a_band_walks_and_those_in_its_rows_are_counted() {
        let clips = band_clips((100, 256), 4);
        // One draw in the top band, one across the first edge, one over all of the layer.
        let rows = [(2.0, 20.0), (60.0, 70.0), (0.0, 256.0)];
        assert_eq!(band_draw_counts(&rows, &clips), (12, 1 + 2 + 4));
        assert_eq!(band_draw_counts(&rows, &clips[..1]), (3, 3));
        assert_eq!(band_draw_counts(&[], &clips), (0, 0));
    }

    #[test]
    fn a_frame_plays_the_bands_its_dirty_rows_touch() {
        let clips = band_clips((100, 256), 4);
        let rows = |rows| dirty_band_clips(clips.clone(), rows).iter().map(|(index, _)| *index).collect::<Vec<_>>();
        assert_eq!(rows(None), vec![0, 1, 2, 3], "no rows: every band");
        assert_eq!(rows(Some((0, 10))), vec![0], "a clock at the top");
        assert_eq!(rows(Some((60, 70))), vec![0, 1], "rows across an edge");
        assert_eq!(rows(Some((200, 256))), vec![3]);
        assert_eq!(rows(Some((0, 256))), vec![0, 1, 2, 3]);
        assert!(rows(Some((300, 310))).is_empty(), "rows below the layer");
    }

    #[test]
    fn only_frames_of_which_every_band_was_played_move_the_bands() {
        let picture = {
            let mut recorder = skia_safe::PictureRecorder::new();
            recorder.begin_recording(Rect::from_wh(1.0, 1.0), false);
            recorder.finish_recording_as_picture(None).expect("a picture")
        };
        let frame = |played: usize, sent: usize| PendingFrame {
            clips: band_clips((100, 256), 4).into_iter().take(played).collect(),
            bands: 4,
            sent,
            period: None,
            picture: picture.clone(),
        };
        assert!(feeds_balance(&frame(4, 4)));
        assert!(!feeds_balance(&frame(1, 1)), "a frame that left bands out");
        assert!(!feeds_balance(&frame(4, 3)), "a band that was not sent");
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
