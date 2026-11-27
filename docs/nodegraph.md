# FerroUI NodeGraph (`ferroui-nodegraph`)

`ferroui-nodegraph` is a standalone, **Rhino Grasshopper-style** parametric visual node programming
engine built from **real FerroUI UI Visuals**, designed to build **anything** — parametric
CAD/geometry, live physics simulations, procedural UI, audio/DSP, shaders, or data pipelines.

## Key Capabilities

- **Built from Real FerroUI UI Visuals (`feature = "ferroui"`)**:
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
