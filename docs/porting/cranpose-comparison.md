# FerroUI against Cranpose: architecture and the gauntlet benchmark

Cranpose (<https://github.com/samoylenkodmitry/Cranpose>, MIT or Apache-2.0) brings the Jetpack
Compose model to Rust and publishes a cross-framework benchmark measured from outside every app
(<https://samoylenkodmitry.github.io/Cranpose/?tab=performance>). This document records what
FerroUI can learn from its architecture, how FerroUI does on its benchmark, where FerroUI loses
time, and the plan to fix it. The work is tracked in #644; the fixes and the deviations from upstream they need are in
[desktop-performance.md](desktop-performance.md), sections 8 and 9.

## 1. The benchmark

Each framework draws the same screens, written in its own idiom, and an external counter
(`FrameCount.app`, ScreenCaptureKit) counts the frames each window presents. Two screens matter on
the desktop:

- **Gauntlet**: every stage at once, so no framework holds 60 fps. A ticker strip whose texts and
  bars change every frame, a virtualized list of cards scrolling 3 points a frame (shadows,
  borders, a clipped avatar bitmap, wrapping text, a gradient sparkline, chips, a footer row of
  counters), nested clusters, translucent panels that drift and tilt over the list, and a content
  width that follows the frame so every visible node is measured again. Tier 16 on an Apple M3
  Pro.
- **Still wall**: thousands of tiles that never change, a clock line whose text changes and a
  cursor outline that moves. It measures what a frame costs when almost nothing changes. Tier 20.

Published results on an Apple M3 Pro, gauntlet tier 16, fps over 10 s from launch (2026-11-08):

| Cranpose | egui | Flutter | iced | Xilem | GPUI | Slint | AppKit | .NET original of FerroUI's API | SwiftUI |
|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| 55.6 | 53.4 | 15.0 | 12.9 | 11.6 | 11.3 | 10.0 | 7.9 | 7.9 | 2.7 |

FerroUI's port of the benchmark app is `samples/GauntletBenchmark` (a translation of the
benchmark's .NET desktop app, which uses the same API). `PERF_REPORT=1` prints the frame rate the
app's own animation-frame callback sees, used for A/B comparisons of FerroUI builds; the
cross-framework numbers use the benchmark's external counter.

## 2. FerroUI on the benchmark (same machine: Apple M3 Pro)

Own frame counter, median of the last 8 seconds of a 12-second run, `release` profile:

| Renderer | Gauntlet (tier 16) | Still wall (tier 20) |
|---|---:|---:|
| Skia (Graphite, Metal) | 15.5–16.6 | 26.8–29.4 with the pointer outside the window; 1–4 with it inside |
| Vello GPU | 7.9 | no frame within 12 s |
| Vello hybrid | no frame within 12 s | 21.9 |
| Quartz Metal (`ferroui-quartz`) | 1.4 | 0.3 |

FerroUI on Skia does twice the gauntlet frame rate of the .NET original and about Flutter's, a
third of Cranpose's and egui's (that was before the changes of desktop-performance.md section 9);
section 2.1 has the benchmark's own harness after them.

### 2.1 The benchmark's own harness (external counter, 60 Hz)

`benchmarks/compose-vs-cranpose/desktop.py` of the Cranpose repository, on the same machine with
the display at 60 Hz and a still wallpaper (the harness refuses to measure while anything else
moves on the display), two rounds, each app launched twice a round (the first launch after its
saved state was removed). FerroUI at main `18a21d966` (sections 9.12 to 9.15 of
desktop-performance.md; before 9.16 and 9.17). Frames a second as `FrameCount.app` counts the
frames the window presents, first launch / second launch, both rounds; CPU time a frame of the app
and what it started; the 99th percentile frame time:

| Scenario | App | fps | CPU a frame | p99 |
|---|---|---|---:|---:|
| Gauntlet, tier 16 | Cranpose | 55.6 / 58.4, 55.5 / 58.5 | 18–20 ms | 19–22 ms |
| | egui | 54.0 / 57.1, 53.7 / 57.2 | 20–23 ms | 33–37 ms |
| | FerroUI (Skia, Graphite) | 23.3 / 29.4, 23.1 / 29.7 | 63–66 ms | 50–68 ms |
| Still wall, tier 20 | Cranpose | 21.2 / 25.4, 23.4 / 24.9 | 47–53 ms | 170–188 ms |
| | egui | 22.6 / 25.0, 23.9 / 25.3 | 50–52 ms | 50–51 ms |
| | FerroUI (Skia, Graphite) | 26.6 / 36.7, 27.3 / 37.4 | 27–33 ms | 21–25 ms |

FerroUI leads the still wall in frame rate and in smoothness, and runs the gauntlet at about half
the frame rate of Cranpose and egui with about three times their CPU time a frame. Its first
launch is slower than its second (first frame 2.3 s against 0.6 s on the gauntlet, 5.5 s against
3.8 s on the wall), which points at work done once a process (pipelines compiled, fonts loaded).

## 3. Where FerroUI loses time

Profiles with macOS `sample` (8 s at 1 kHz) of the `release` build on Skia.

### 3.1 Gauntlet

The UI thread is the bottleneck: busy about 80 % of the time, against about 51 % for the render
thread.

| Cost | Share of the UI thread | Cause |
|---|---:|---|
| Layout pass | 46 % | the content width changes every frame, so the list (`Grid` > `VirtualizingStackPanel` > card `StackPanel`s > `TextBlock`) is measured and arranged again; text layout inside |
| `Compositor::dispose_on_next_batch` | 14 % | every visual that records a new draw list disposes the old render data, and queueing a disposal searched the list of pending disposals: quadratic in the number of changed visuals |
| `ValueStore::try_get_inherited_value` | 7 % (top of stack) | every read of an inherited property (font family, size, foreground) walks the inheritance ancestors; the nested clusters are 80 levels deep |
| allocation (`malloc`/`free`) | ~6 % | per-frame `Vec`s and `Rc`s in layout, text and composition |
| SipHash | ~2 % | `HashMap`s with the default hasher on hot paths |

On the render thread, text dominates (`SkCanvas::drawTextBlob`, glyph atlas uploads), then the
Graphite flush and one blur (the cards' box shadows).

### 3.2 Still wall

With the pointer inside the window, 97 % of the UI thread is hit testing:
`TopLevel::update_tool_tip` and `PointerOverPreProcessor::scene_invalidated` each hit-test the
pointer after every frame, because the renderer raises "scene invalidated" with the whole window
as the dirty rectangle (as upstream does). Each hit test revalidates every bucket of the
composition hit-test tree (its validity is keyed by a global readback revision that changes every
frame), and each bucket update copied the whole child collection (`items()`), which made it
quadratic in the number of tiles. With the pointer outside the window the wall runs at 27–29 fps:
then the cost is the clock text and the scene's composition, not hit testing.

### 3.3 Other renderers

Quartz Metal allocates a texture for every clip and composites a full-surface layer for every
opacity (issue #640); its frame rate is a twelfth of Skia's on the gauntlet. Vello stops
presenting frames in two of the four runs: to investigate.

## 4. Cranpose's architecture, and what FerroUI can take from it

| Cranpose | What it does | FerroUI today | Take |
|---|---|---|---|
| Phased state reads | a value read in the layout or draw phase invalidates only that phase; `graphics_layer { }` reads per-frame values at the layer stage, so a moving or tilting panel is never recorded again | render transforms and opacity are composition properties: a change does not record the draw list again, as long as nothing invalidates the visual | verify that render-transform and opacity changes never re-record; keep it covered by a test |
| Retained surfaces | a layer cache (96 MiB budget, 4096 entries) of rendered surfaces, flat surfaces packed into atlases, read in place on Metal | Skia renders each visual's draw list every frame; `BitmapCache` exists but is opt-in | an automatic cache for subtrees whose draw lists did not change while only their transform changes (the gauntlet's panels) |
| Opaque prefix cache | an opaque background drawn before everything else is kept as a texture and composited | none | candidate for Skia and Vello |
| Mapped uploads | frame uniforms, geometry, run tables and glyph runs written into mapped buffers on Metal: no copy per frame | Skia's own upload path; Vello's buffers per frame | Vello: reuse mapped buffers |
| Off-thread pipeline compilation | two worker lanes for demanded pipelines and warm-ups | Skia Graphite and Vello compile on first use | Vello: compile pipelines on a worker at start |
| Two queued frames on desktop | CPU and GPU overlap; +36 % on an M3 Pro in their measurement | one frame in flight on Metal | measure a second frame in flight on Metal and Vulkan |
| Presentation worker on mobile | present on a worker when four or more cores | render thread presents | measure on Android and iOS |
| Shaped-word cache | direct-mapped table over two ring buffers, no allocation per kept word; a word is kept the second time it is shaped (TinyLFU doorkeeper), so a number that changes every frame does not push stable words out; grows when it misses words it lost for room | a cache of shaped runs per text layout | adopt the admission rule and the ring storage for the shaper's cache |
| Text layout cache per width | a text keeps its layouts at several widths | a `TextBlock` keeps one layout, invalidated by any width change | keep a few layouts per width: the gauntlet's width animation re-lays every text each frame |
| Hit testing on pointer events only | pointer repasses only for nodes whose pointer input changed | hit test of the whole scene after every frame for pointer-over and tooltips | dirty-region scene invalidation, one hit test per frame, push-based tree maintenance (section 5) |
| Lazy grid | `LazyVerticalGrid` measures lines of cells | `VirtualizingStackPanel` of rows; `ItemsRepeater` with a uniform grid layout | none needed for the benchmark |
| Evidence rules | a change ships with calls/frame, saved time, pixel checks, ABAB/BABA device runs | `docs/binary-size.md` and the benchmarks crate follow the same idea | add the gauntlet and the wall to the benchmark runs |

## 5. Plan

In order of measured cost; each step is measured with `samples/GauntletBenchmark` (ABAB) and the
benchmarks crate, and keeps the behaviour upstream has unless the deviation is recorded in
[desktop-performance.md](desktop-performance.md), section 9.

1. Disposal queue indexed by id (done: gauntlet 15.6–16.6 to 19.0–19.3 fps with step 2).
2. Hit-test buckets read only their own children (done).
3. Scene invalidation with the frame's real dirty region instead of the whole window, and one hit
   test per frame shared by pointer-over and tooltips.
4. Hit-test tree maintenance pushed by the visuals that changed, instead of a revalidation of
   every bucket keyed by a global revision.
5. Inherited property reads: cache the resolved inherited value per store, invalidated by the
   inheritance notifications that already exist.
6. Text: keep layouts per width in `TextBlock`; the shaper's cache with a second-sighting
   admission rule; no per-frame line-break and bidi work for unchanged text.
7. Layout: avoid re-measuring children whose constraints did not change (the measure cache
   upstream has), and allocation-free measure and arrange of panels.
8. Faster hashing (`FxHash`/`ahash`-style) on the hot maps of the composition and property system.
9. Render thread: retained subtrees for transform-only changes; text blob reuse in Skia.
10. Quartz Metal (#640) and the Vello stalls.
11. Frame pacing: a second frame in flight on Metal, measured.
