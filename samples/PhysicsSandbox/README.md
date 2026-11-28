# FerroUI Motion Studio

A WinUI 3 desktop sample for building useful interfaces with physical motion, compositor effects,
parametric graphs, and interactive 2D simulation. The navigation shell, inspectors, forms, buttons,
toggles, sliders, and split panes use FerroUI controls and the built-in WinUI theme. Light and dark
variants share semantic resources; the simulation canvas uses its own high-contrast plotting palette.

## Run

```sh
# Linux, Windows or macOS: Skia rendering.
cargo run --profile fast-release -p physics-sandbox --bin physics-sandbox

# macOS: native Quartz and Metal rendering.
cargo run --profile fast-release -p physics-sandbox --no-default-features --features quartz-metal --bin physics-sandbox

# Windows: native Direct2D rendering.
cargo run --profile fast-release -p physics-sandbox --no-default-features --features direct2d --bin physics-sandbox
```

`FERROUI_START_PAGE` selects a workspace directly. `FERROUI_PHYSICS_CPU_ONLY=1` skips GPU adapter
creation for repeatable CPU runs. `FERROUI_SMOKE_EXIT_MS=1800` closes the desktop sample after launch.

## Interface workspaces

| Startup key | Workspace | What to try |
| --- | --- | --- |
| `home` | Overview | Open a task-oriented entry point or choose a simulation from the catalog. |
| `ui` | Interactive workspace | Move the floating inspector while editing a document; archive and undo tasks; reveal a details drawer and edit its task; validate workspace actions. |
| `compositor` | Composition & effects | Edit real workspace cards, compare spring, rigid-body and harmonic motion, and opt into effect tuning. |
| `nodegraph` | Parametric workspace | Load a complete graph preset, edit its parameters, connect ports, and inspect the live simulation in a resizable split view. |

The interface examples keep motion attached to a task. A floating inspector still edits the document;
a swipe changes the inbox; a drawer exposes usable controls. Stock controls retain their normal focus,
keyboard, pressed, checked, and disabled behavior. Physics transforms move the existing control tree.

The parametric preview coalesces rapid changes. An unchanged immutable scene output leaves the current
simulation running. Pause, live preview, explicit apply, and restart are separate actions. Invalid graph
solutions retain the last valid scene and report their status.
Selecting the active page or template again preserves its edits. Graph previews do not allocate
playback snapshots for a timeline they do not expose.
Wide windows use a resizable split view. Compact windows switch between **Edit graph** and
**View simulation**, preserving both views and giving each a usable canvas.

## Simulation lab

| Startup key | Scene | Interaction and purpose |
| --- | --- | --- |
| `playground` | Playground | Drag and spawn bodies; inspect contacts and bounds; break a structure or apply a blast. |
| `particles` | Particle pool | Stir or repel particles; compare 2K/4K/8K workloads and available CPU/GPU compute. |
| `cloth` | Cloth & Jelly | Pull cloth, right-drag to cut fibers, add soft bodies, and change wind and gravity. |
| `springs` | Spring laboratory | Drag cards toward magnetic slots, compare response presets, and flick an inertial scroller. |
| `destruction` | Destruction | Explore welded beams, a wrecking ball, fracture shards, and articulated ragdolls. |
| `vehicle` | Vehicle | Follow spring suspension and powered wheels over ramps and through buoyancy. |
| `character` | Character | Move the pointer to guide the controller; click to jump over slopes and moving platforms. |
| `orbit` | Ray & Orbit | Observe inverse-square orbital motion, reflected rays, proximity queries, and trails. |

Each scene shows relevant controls. Particle settings are confined to the particle scene; spring
presets to the spring laboratory. Collision tools and diagnostic overlays appear where they apply.
**Controls** hides the inspector when more canvas space is useful. Transport buttons have text labels.
**Step** is available while paused; **Reset** clears transient interaction state and resumes the scene.

The history slider records **rigid-body world state** in the playground, destruction, vehicle, and
orbit scenes. Pause before scrubbing. Cloth, particles, spring UI, and character-controller state are
not represented by that rigid-body history, so those scenes do not expose a misleading rewind control.
Jelly creation belongs to Cloth & Jelly; rigid-body workspaces offer tools compatible with their
recorded state. Rewinding keeps the currently selected solver settings and releases pointer constraints.

## Timing and lifetime

- Display frames feed a fixed-step accumulator, so simulation is independent of dispatcher cadence.
  Long frames have bounded catch-up; pausing clears accumulated time.
- The viewport receives its engine through binding. It does not create a second world or GPU device.
- Timers start when a page is attached and stop when it is detached. Capture cancellation releases
  drag state. Compositor simulations unregister on unload.
- Inspectors refresh at 10 Hz while simulation and drawing retain the frame cadence. The graph
  preview coalesces updates at 20 Hz. These limits avoid repeatedly relaying out text on every frame.
- GPU particle computation currently reads results back for the retained drawing canvas. Performance
  therefore includes synchronization and transfer costs; a GPU label alone does not imply a faster
  end-to-end frame than the CPU path. The particle pool is a collision demo, not a general fluid solver.

## Capture the actual UI without a desktop

The optional preview binary runs the same page builders, theme, layout, and Skia renderer on FerroUI's
headless platform. It is intended for visual review and reproducible sample captures.

```sh
cargo run -p physics-sandbox --no-default-features --features preview --bin physics-studio-preview -- ui light workspace-light.png 1280 800
cargo run -p physics-sandbox --no-default-features --features preview --bin physics-studio-preview -- nodegraph dark graph-dark.png 1440 900
```

Arguments are `page theme output.png width height`; defaults are `home light physics-studio.png 1280 800`.
Use the startup keys in the tables. For `ui`, `FERROUI_UI_SAMPLE=inbox`, `details`, or `workspace`
selects an interaction directly (the default is the floating inspector). The renderer backend still needs its normal build prerequisites;
the preview does not need an X11 or Wayland session.
`FERROUI_GRAPH_VIEW=preview` selects the simulation view for a compact node-graph capture.

## Validate

```sh
cargo test -p ferroui-physics --no-default-features
cargo test -p ferroui-nodegraph --no-default-features
cargo test -p physics-sandbox --no-default-features --lib
cargo check -p physics-sandbox --bin physics-sandbox
```

The sample's library can build without a desktop renderer. Platform renderers are optional and enabled
by the desktop features, while the preview selects headless + Skia explicitly. On memory-constrained
build machines, `CARGO_BUILD_JOBS=1 CARGO_PROFILE_TEST_DEBUG=0 CARGO_INCREMENTAL=0` reduces compiler memory
use without changing test assertions.

## Source map

| Area | Files |
| --- | --- |
| Application and navigation | `main.rs`, `lib.rs`, `catalog.rs` |
| Scene controls and shared visual language | `workspace.rs`, `showcase_style.rs`, `icons.rs` |
| Simulation and drawing | `scenes.rs`, `scenes/`, `viewport.rs` |
| Practical control interactions | `live_ui_tree.rs`, `live_ui_tree/` |
| Composition and graph workspaces | `compositor_showcase.rs`, `node_graph_studio.rs` |
| Reproducible visual capture | `preview.rs` |

Engine architecture and API details: [physics](../../docs/physics.md) and [node graphs](../../docs/nodegraph.md).
