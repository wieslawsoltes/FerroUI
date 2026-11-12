# GauntletBenchmark

The three desktop screens of the framework comparison benchmark of the
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
- **Patchwork** (`PERF_SCENARIO=patchwork`): nine patches of the still wall's tiles, three across
  and three down under the clock line, beside a chat 320 points wide. A patch is a third of the
  rest of the width and a third of the height, in whole points, cuts what lies past it (a `Canvas`
  that clips) and holds the wall's tiles at the tier's size, as many as fit. Each patch builds its
  controls once and a frame sets only what changed:
  - **Still:** tiles that never change.
  - **Charts:** line charts that never change, each over 2 by 2 tiles: 512 points on a line 1
    point wide, built once as `StreamGeometry` objects that one control draws in its `render`.
  - **Recolor:** the brushes of one row of tiles change a frame, the next row down the frame after.
  - **Cards:** cards of a header and rows of tiles (`StackPanel`s) that move every frame by a
    `TranslateTransform`. For 32 frames a new card slides in from above and the others move down by
    a card; for the next 32 the first and the third card change places and back. A card that leaves
    gives its controls to the one that comes in.
  - **Reorder:** a `WrapPanel` of tiles 1 to 3 tiles wide, two neighbours of which change places
    every frame (two moves in the panel's children).
  - **Deep:** cells of 12 `Border`s nested inside one another around three digits; the texts of
    one row of cells change a frame.
  - **Scroll:** an `ItemsControl` over a `VirtualizingStackPanel` in a `ScrollViewer`, about 100,000
    tiles in rows, that scrolls by one row a frame.
  - **Paragraph:** one wrapping `TextBlock` of five-digit words, three for each tile of a patch, one
    word of which changes every frame.
  - **Pile:** translucent tiles of 2 by 2 tiles' size that lie four deep; one of them is opaque,
    another one every frame.

  The chat shows an agent's answer as it arrives. A data thread sends tokens through a bounded
  queue and waits while the screen is behind; each frame the screen takes up to 4 tokens without
  waiting. Its list is an `ItemsControl` over a `VirtualizingStackPanel` whose items are the
  numbers of the blocks: a question in a bubble, headings and paragraphs (`TextBlock`s), a code
  block in DejaVu Sans Mono with colored `Run`s, a table whose rows appear a token each, an
  animated picture (the frames of `spin.gif`, decoded once into `WriteableBitmap`s) and a row of
  `Button`s. A block's control is made once per place in the turn and recycled for that place in
  the next turn; the list follows its end from the `ScrollViewer`'s `scroll_changed` event. Under
  it a status line shows a spinner (the paths of `progress.svg` in a `Viewbox`, turned by a
  `RotateTransform`, 12 degrees a frame) and what the agent does, under a `LinearGradientBrush`
  whose end points move 6 points a frame.

Frames are driven by `TopLevel::request_animation_frame`, one frame index step per callback.
The screens and the application are the library of the crate (`lib.rs`); the program
`perf-compare-ferroui` (`program.rs`) and the frame gate use them. The
window is 1280 x 820 logical points, not resizable, in the light Fluent theme, with Roboto loaded
from files as a font collection of its own (`fonts:Device#Roboto`) and, for the code of the
patchwork's chat, DejaVu Sans Mono as another (`fonts:Mono#DejaVu Sans Mono`). The window's title
names the screen: `Gauntlet`, `Still wall` or `Patchwork`.

## Running

```sh
PERF_TIER=16 PERF_FONTS=path/to/fonts cargo run --release -p gauntlet-benchmark
PERF_SCENARIO=wall PERF_TIER=20 PERF_FONTS=path/to/fonts cargo run --release -p gauntlet-benchmark
PERF_SCENARIO=patchwork PERF_TIER=17 PERF_FONTS=path/to/fonts PERF_CHAT=path/to/chat-assets \
    cargo run --release -p gauntlet-benchmark
```

| Variable | Meaning | Default |
| --- | --- | --- |
| `PERF_TIER` | Load tier: 1 to 16 for the gauntlet, 1 to 20 for the wall and the patchwork (clamped) | `5` |
| `PERF_FREEZE` | Stop on this frame and hold still, for picture comparisons; `0` runs on | `0` |
| `PERF_SCENARIO` | `wall` for the still wall, `patchwork` for the patchwork, else the gauntlet | the gauntlet |
| `PERF_FONTS` | The folder of `Roboto-Regular.ttf`, `Roboto-Bold.ttf` and `DejaVuSansMono.ttf` | `fonts` |
| `PERF_CHAT` | The folder of the chat's `spin.gif` and `progress.svg` (the patchwork only) | `chat-assets` |

The fonts are those of the benchmark's `fonts` folder (the monospaced face only the patchwork
loads), the chat's pictures those of its `chat-assets` folder.

The program writes the lines the harness reads to standard output: `PERF first_frame` on the first
animation frame and, with `PERF_FREEZE`, `PERF frozen frame=<k>` when it stops.

The renderer is chosen with the features of the crate, as in the other desktop samples: Skia by
default; `--features vello` adds the Vello backend, which `FERROUI_RENDERER=vello` (or
`vello-hybrid`, `vello-gpu`, `vello-cpu`) selects at start; on macOS `--features quartz` and
`--features quartz-metal` use the Quartz backend.

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

The harness passes the scenario, the tier, the frame to freeze on, the fonts and the chat's
pictures in `PERF_SCENARIO`, `PERF_TIER`, `PERF_FREEZE`, `PERF_FONTS` and `PERF_CHAT`, as it does
for the other desktop apps, for example:

```sh
python3 desktop.py --output results/desktop --scenario gauntlet:16 --scenario wall:20 --scenario patchwork:17
```

## Differences from the .NET app

The screens are built of the same controls as in the .NET app; FerroUI has a counterpart of each
control and brush they use. Where the .NET app uses a language or library feature with no
counterpart in Rust or in FerroUI, the port does the nearest thing:

- **Classes of the patches.** The .NET app derives a class for every patch from an abstract
  `Canvas` that clips (`PatchBox`, `TileCanvas` and their subclasses). The port has one `PatchBox`
  class, a `Canvas` that clips and calls the per-frame function its patch gives it, and keeps
  each patch's state in a plain Rust value that this function holds; the controls on the screen
  are the same.
- **Following the frame index.** The .NET app's `Clock.Follow` adds handlers to the attached and
  detached events of a control. The port overrides `on_attached_to_visual_tree` and
  `on_detached_from_visual_tree` of the classes that follow the index, as the gauntlet already
  does.
- **The blocks of the chat.** The .NET app finds a block's `Bind` through the interface `IBlock`
  its block classes implement (`container is IBlock`). The port has a Rust trait, `Block`, and
  finds a list's container as a block by downcasting it to each block class.
- **The chat's data thread.** The .NET app's `BlockingCollection` with a capacity of 16 is a
  `std::sync::mpsc::sync_channel` of the same capacity; its background thread is a thread of the
  standard library that ends when the screen's end of the channel is dropped.
- **The chat's pictures.** The GIF decoder is the .NET app's, ported bit for bit (`chat_gif.rs`).
  The .NET app reads `progress.svg` with an XML parser (`XDocument`); the port reads the attributes
  of its `svg` and `path` start tags from the text, as the sample has no XML parser among its
  dependencies. The decoded frames are made once per thread (a thread-local `OnceCell` in place of
  the .NET `Lazy`).
- **The cards patch.** The cards on screen and the spare ones are a `HashMap` and a `Vec` in place
  of a `Dictionary` and a `Stack`; which spare card's controls a new card reuses can differ, which
  is not visible.
- **The paragraph patch.** The paragraph's characters are a buffer of ASCII bytes in place of a
  `char` array, and a word's five digits are written into it directly in place of
  `int.TryFormat`; a number of more than five digits leaves the word as it was in both.
- **The still wall** builds its tiles with the same `TileView` class as the patches, as the .NET
  app's current still wall does; the tiles are the same controls as before.
