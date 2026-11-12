# GauntletBenchmark

The two desktop screens of the framework comparison benchmark of the
[Cranpose](https://github.com/samoylenkodmitry/Cranpose) project (`benchmarks/compose-vs-cranpose`),
in FerroUI. It is ported from the benchmark's .NET desktop app, so it builds the same screens out
of the same kinds of controls and does the same work every frame; see `NOTICE.md` for the
attribution and the MIT license of the original.

- **Gauntlet** (the default): a screen heavy enough that no framework holds 60 fps. Every frame
  advances a frame index `k` and everything follows from `k`: a ticker strip whose prices, changes
  and bars change every frame (a `WrapPanel` of `Border` tiles), a list of cards scrolling 3 points
  a frame (an `ItemsControl` over a `VirtualizingStackPanel` in a `ScrollViewer`, whose card rows and
  cluster rows each recycle only their own kind), cards with a box shadow, a circle-clipped avatar
  (a `WriteableBitmap`), a two-line title, an annotated subtitle, a progress bar and a sparkline
  drawn by controls that override `render` (`DrawingContext`, `StreamGeometry`), a chip flow row and a
  footer of counters, a tilting badge on every fifth card, deep nested clusters, and translucent
  panels stacked over the list that drift and tilt by their `RenderTransform` alone. The width of
  the whole content follows `k` between 92% and 100%, so everything visible is measured again every
  frame.
- **Still wall** (`PERF_SCENARIO=wall`): thousands of tiles made once on a `Canvas`, under a clock
  line whose text changes every frame and a cursor outline moved by a `TranslateTransform`. A frame
  changes those two things and nothing else.

Frames are driven by `TopLevel::request_animation_frame`, one frame index step per callback.
The screens and the application are the library of the crate (`lib.rs`); the program
`perf-compare-ferroui` (`program.rs`) and the frame gate use them. The
window is 1280 x 820 logical points, not resizable, in the light Fluent theme, with Roboto loaded
from files as a font collection of its own (`fonts:Device#Roboto`).

## Running

```sh
PERF_TIER=16 PERF_FONTS=path/to/fonts cargo run --release -p gauntlet-benchmark
PERF_SCENARIO=wall PERF_TIER=20 PERF_FONTS=path/to/fonts cargo run --release -p gauntlet-benchmark
```

| Variable | Meaning | Default |
| --- | --- | --- |
| `PERF_TIER` | Load tier: 1 to 16 for the gauntlet, 1 to 20 for the still wall (clamped) | `5` |
| `PERF_FREEZE` | Stop on this frame and hold still, for picture comparisons; `0` runs on | `0` |
| `PERF_SCENARIO` | `wall` for the still wall, anything else for the gauntlet | the gauntlet |
| `PERF_FONTS` | The folder of `Roboto-Regular.ttf` and `Roboto-Bold.ttf` (the benchmark's `fonts` folder) | `fonts` |

The program writes the lines the harness reads to standard output: `PERF first_frame` on the first
animation frame and, with `PERF_FREEZE`, `PERF frozen frame=<k>` when it stops.

The renderer is chosen with the features of the crate, as in the other desktop samples: Skia by
default; `--features vello` adds the Vello backend, which `FERROUI_RENDERER=vello` (or
`vello-hybrid`, `vello-gpu`, `vello-cpu`) selects at start; on macOS `--features quartz` and
`--features quartz-metal` use the Quartz backend.

### Windows GPU backend comparison

Build separate release executables for Skia/ANGLE and the native
Direct2D/DirectWrite/D3D11 backend. Native builds use
`--no-default-features --features direct2d-runtime-shaders`; normal Windows
Skia builds keep the default features. The measurement mode below is compiled
only with the feature `renderer-measurement`, so both builds of a comparison
add it:

```powershell
cargo build --locked --release -p gauntlet-benchmark --features renderer-measurement --target-dir target/gauntlet-skia
cargo build --locked --release -p gauntlet-benchmark --no-default-features --features direct2d-runtime-shaders,renderer-measurement --target-dir target/gauntlet-native
```

The opt-in desktop measurement mode uses `PERF_WARMUP_FRAMES=60` and
`PERF_MEASURE_FRAMES=240`. Each scene advances on the UI thread after the
compositor reports the previous batch rendered. This keeps the same scene
frames in both backends even when the compositor can accept batches early.
The intervals include the UI and compositor scheduling cycle through render
submission. The program requires a hardware backend, writes all samples and
viewport metadata to `PERF_RESULTS_JSON`, prints `PERF_RESULT`, and closes the
window.

`PERF_CAPTURE_PATH` optionally saves a compositor snapshot after timing stops.
The measured scene-frame range and pointer-over counts are recorded. Keep the
pointer in one place, either outside the window or over it, for a whole
comparison, and do not compile, profile or capture the desktop during a timed
run. Windows can clamp the requested 1280×820 logical window to
the available desktop; compare the recorded client size and render scaling.

The [Windows comparison runner](../../scripts/compare-windows-gauntlet-renderers.ps1)
runs gauntlet tier 16 and still wall tier 20 with Skia first, then native, for
three repetitions. Pass prebuilt binaries with `-SkiaExecutable` and
`-NativeExecutable`; the runner refuses an executable built without
`renderer-measurement`. Use `-Scenarios gauntlet` for animated-only runs, and
`-GauntletTier` and `-WallTier` for other tiers. The runner checks executable
hashes, scene counts, viewport, scaling and pointer conditions.
It rejects viewport or DPI changes during the measured interval.
It also checks that Direct2D submitted a present for each measured scene and
recorded zero errors or pixel readbacks. Per-second `PERF fps` lines provide
progress. Full-precision scene intervals determine the results. The
[renderer benchmarks](../../docs/porting/windows-native-renderer-benchmarks.md)
hold the method and the measured results.

## The frame gate

`tests/frame_gate.rs` shows both screens in a headless window (1280 x 820 points at a scaling of
2, no pointer device) and renders them through the compositor into a raster Skia framebuffer, in
the render-thread mode of the desktop. The screens drive their frames as in the program; after
`GATE_WARMUP` frames (60) it measures `GATE_FRAMES` frames (240) and prints, per scenario, a line
for the local performance gate and a summary:

```text
GATE {"bench":"gauntlet/16","frames":240,"warmup":60,"ui_ms_per_frame":...,"render_ms_per_frame":...,"wall_ms_per_frame":...,"rendered":...}
GATE {"bench":"wall/20",...}
```

The times are milliseconds per frame of the UI thread's busy time, of the render thread's ticks
and of wall time. The tests are ignored by default; run them in an optimised build, alone:

```sh
cargo test -p gauntlet-benchmark --release --test frame_gate -- --ignored --nocapture --test-threads=1
```

`PERF_FONTS` defaults to the Roboto files of the Material theme in this checkout. The gate that
compares a base commit with the working tree is `scripts/perf/gate.py`; see
`docs/porting/desktop-performance.md`, section 8.1.

## In the benchmark's desktop harness

`desktop.py` of the benchmark measures every app it finds in its `APPS` table, each alone in its
window, from outside the app. To add this one, build it with `cargo build --release -p
gauntlet-benchmark` and add an entry pointing at the program, with the path of the FerroUI
checkout's target folder (or a copy of the program):

```python
APPS = {
    # ...
    'ferroui': [Path('path/to/FerroUI/target/release/perf-compare-ferroui')],
}
```

The harness passes the scenario, the tier, the frame to freeze on and the fonts in `PERF_SCENARIO`,
`PERF_TIER`, `PERF_FREEZE` and `PERF_FONTS`, as it does for the other desktop apps, for example:

```sh
python3 desktop.py --output results/desktop --scenario gauntlet:16 --scenario wall:20
```
