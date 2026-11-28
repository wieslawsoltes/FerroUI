# FerroUI NodeGraph (`ferroui-nodegraph`)

`ferroui-nodegraph` is a standalone, **Rhino Grasshopper-style** parametric visual node programming
engine built from **real FerroUI UI Visuals**, designed to build **anything** — parametric
CAD/geometry, live physics simulations, procedural UI, audio/DSP, shaders, or data pipelines.

## Key Capabilities

- **Built from Real FerroUI UI Visuals & Stock Theme (`feature = "ferroui"`)**:
  - Uses the host application's **stock current FerroUI theme** (Light or Dark `ThemeVariant`) automatically with zero custom theme overrides — all `Button`, `Slider`, `TextBox`, `TextBlock`, and `Border` controls inherit stock styling natively, while vector grid/wire layers and custom widgets (`GraphMapperControl`, `MdSliderControl`) adapt automatically to `actual_theme_variant()`.
  - Every node on the canvas is a genuine FerroUI control tree (`Border`, `Slider`, `TextBox`,
    `Button`, `StackPanel`, `DockPanel`, `GraphMapperControl`, `MdSliderControl`) keyed via
    `for_each` inside a GPU-transformed `Canvas` (`MatrixTransform` — zero layout passes while
    panning or zooming!).
  - `WireBackdropLayer` renders the adaptive infinite Grasshopper grid, colored node groups,
    **Fancy Bezier Wires** (Empty dashed amber, Item single line, List double line, DataTree
    dashed double line, Selected emerald glow), marquee selection, live port DataTree tooltips,
    and an interactive minimap — querying each real port socket's exact screen position via
    `Visual::translate_point`!
  - **Floating Rhino Grasshopper Overlays**: Double-click / `Space` **Quick-Search Popup** with
    fuzzy search, **Multi-Select Align & Distribute Bar**, and right-click / `⋯` **Node Context
    Menu** (Enable/Disable, Lacing mode, Wire Display `Default`/`Faint`/`Hidden`, Duplicate, Delete).
  - Double-click any wire to insert an inline **Relay**; `Shift`-drag to connect multiple wires to
    one input; `Alt`-click or right-click a socket/wire to disconnect; `🔒 Lock Solver` pauses
    evaluation while making batch edits.
- **True Grasshopper DataTrees (`DataTree` & `DataPath`)**: hierarchical `{0;1;2}` branches with
  **`PathMapper`** (`{A;B}(i) -> {B;A}`), `Flatten`, `Graft`, `Reverse`, `Simplify`, `FlipMatrix`,
  `ShiftPaths`, `Entwine`, `Weave`, `Dispatch`, `PartitionList`, `SortList`, per-port modifiers
  (`F/G/R/S`), and automatic `LongestList` / `ShortestList` / `CrossReference` lacing.
- **65+ Built-in Components & Reusable Clusters**: `Params` (`NumberSlider`, `IntSlider`,
  `BoolToggle`, `Button`, `GraphMapper`, `MdSlider`, `ValueList`, `ParamViewer`, `Panel`,
  `ColorSwatch`, `Domain`, `Relay`, `Scribble`, `Time`), `Math` (live editable `Expression`
  formula boxes, `Bounds`, `MassAddition`, `Average`), `Logic`, `Sets`, `Vector / Geometry`
  (`RectGrid`, `CirclePoints`, `LineDivide`, `PointAttractor`, `SineWave`, `Spiral`, `ColorHsv`),
  plus `ClusterComponent` sub-graphs and zero-dependency JSON save/load.
- **Parametric Physics Bridge (`feature = "physics"`)**: compiles directly into live
  `ferroui-physics` worlds with 5 templates (**Attractor & GraphMapper**, **Pyramid & Pendulum**,
  **Keplerian Solar System**, **Bridge & Rain**, **Cloth & Jelly Tunnel**) wired into
  `samples/PhysicsSandbox` under the **🧩 Node Graph** tab.

## Evaluation and integration

`evaluate_graph(&mut graph, &registry, time_secs)` evaluates changed components and all their
consumers in deterministic topological order. `configure_slider`, `configure_toggle`,
`configure_text`, `configure_color`, and the input configuration methods invalidate downstream
results. When changing public node fields directly, call `graph.mark_dirty(node_id)` afterwards.

`params.time` and expressions containing the variable `t` update whenever the caller supplies a
new solution time. Custom components can override `NodeComponent::is_time_dependent` for the same
behavior. Time dependence propagates through reusable clusters. The interactive editor uses a
monotonic clock and updates active clock-driven definitions at approximately 30 Hz; static graphs
only evaluate after edits. Locking the solution pauses evaluation. Momentary buttons produce a
true result for one solution and release on the following solution.

A solution builds incoming and outgoing edge indices once, making graph traversal and input lookup
linear in the number of nodes and wires. Clean nodes do not clone their widget state or port
metadata. Empty, disabled, failed, or cyclic upstream results clear downstream cached outputs.
An independent subgraph can still evaluate when a separate subgraph contains a cycle.

Cluster inputs preserve their entire data tree, including branch paths and all list items. Bind
inputs to actual input ports with `ClusterInputBinding`; invalid bindings return a component error.
`CrossReference` lacing streams its Cartesian product using temporary storage proportional to the
input count. The output itself still grows with the number of combinations, so large products
should be filtered or reduced in the component consuming them.

## Saving and restoring definitions

The editor's **Copy JSON** command places the serialized definition on the platform clipboard and
reports success only after the clipboard operation completes. Applications can use the same
checked serializer directly:

```rust,ignore
use ferroui_nodegraph::prelude::*;

let registry = NodeRegistry::with_standard_library();
let mut graph = NodeGraph::new("Motion settings");
let input = graph.add_node(&registry, "params.slider", 20.0, 20.0).unwrap();
graph.configure_slider(input, "Stiffness", 180.0, 10.0, 600.0, 1.0, false);

let json = try_graph_to_json(&graph)?;
let restored = graph_from_json(&json, &registry)?;
# Ok::<(), String>(())
```

The version-one format retains input defaults, input/output modifiers, lacing, preview and disabled
state, node dimensions, wire display, groups, view settings, and full floating-point precision.
UTF-8 text and JSON Unicode escapes (including surrogate pairs) are supported. The loader rejects
unknown component types, duplicate IDs, invalid ports, cyclic connections, malformed JSON, and
unsupported format versions instead of silently dropping those parts of a definition.

`try_graph_to_json` rejects non-finite numeric values and opaque `Value::Custom` input defaults.
Custom runtime payloads require an application-specific codec; their type-erased contents cannot
be reconstructed from a display summary. The original `graph_to_json` convenience function remains
available for callers whose input has already been validated. Evaluation caches and undo/redo
history are transient and are not serialized. Undo/redo restores inline widgets as well as the
document model, without rebuilding unrelated controls during normal slider interaction.

## Evaluation benchmark

The `evaluation` bench uses only this crate's existing dependencies:

```sh
cargo bench -p ferroui-nodegraph --no-default-features --bench evaluation
```

It constructs two documents with **2,001 nodes and 2,000 wires**: a slider followed by 2,000
addition components, and a slider feeding 1,000 independent two-stage addition/multiplication
branches. After an untimed warm-up solution, each run changes the source value 32 times and
explicitly marks it dirty. Invalidation and evaluation are timed separately; graph construction,
output assertions, and checksum calculation are outside those regions. Each workload runs five
times. Every edit must evaluate all 2,001 nodes and produce the exact expected numeric outputs.
The chain checksum is 64,528 and the fanout checksum is 1,120,000 per run.

A review measurement on 2026-11-28 compared the original PR head
`a31daffcb452a67c1325722954e698c92e8438f2` with the updated evaluator using identical harness code
and dependencies. Both were built as standalone CPU-only crates with Rust 1.100.1 on x86-64 Linux,
`opt-level = 3`, one code-generation unit, and no LTO. Execution order alternated between original
and updated across five runs. Results below are medians of milliseconds per edit; the combined
column is the median of combined measurements, rather than a sum of independently selected medians.

| Workload | Stage | Original | Updated | Speedup |
| --- | --- | ---: | ---: | ---: |
| 2,001-node chain | Invalidation | 1.4123 ms | 0.1973 ms | 7.16× |
| 2,001-node chain | Evaluation | 7.9942 ms | 1.8437 ms | 4.34× |
| 2,001-node chain | Combined | 9.4065 ms | 2.0409 ms | 4.61× |
| 1,000 two-stage branches | Invalidation | 1.3772 ms | 0.1315 ms | 10.47× |
| 1,000 two-stage branches | Evaluation | 7.6348 ms | 1.8470 ms | 4.13× |
| 1,000 two-stage branches | Combined | 9.1828 ms | 1.9859 ms | 4.62× |

All original and updated output assertions and checksums matched. These are measurements on a
shared execution host, not guarantees for another machine; rendering, layout, GPU compute, and
physics simulation are excluded. The repository's default bench profile may use different LTO
settings. Set `CARGO_PROFILE_BENCH_LTO=false` and `CARGO_PROFILE_BENCH_CODEGEN_UNITS=1` when comparing
with the standalone configuration. The main changes behind the improvement are indexed incoming
wire lookup, one outgoing adjacency build for invalidation, avoiding metadata clones for clean
nodes, and skipping no-op data-tree modifiers.
