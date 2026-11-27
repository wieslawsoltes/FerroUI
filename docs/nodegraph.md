# FerroUI NodeGraph (`ferroui-nodegraph`)

`ferroui-nodegraph` is a standalone, **Rhino Grasshopper-style** parametric visual node programming
engine and hardware-rendered FerroUI canvas control designed to build **anything** — parametric
CAD/geometry, live physics simulations, procedural UI, audio/DSP, shaders, or data pipelines.

## Key Capabilities

- **True Grasshopper DataTrees (`DataTree` & `DataPath`)**: hierarchical `{0;1;2}` branches with
  `Flatten`, `Graft`, `Reverse`, `Simplify`, and `FlipMatrix` plus per-port modifiers (`F/G/R/S`)
  and automatic `LongestList`, `ShortestList`, and `CrossReference` lacing.
- **Universal Polymorphic Values (`Value`)**: `Bool`, `Int`, `Float`, `Vec2`, `Vec3`, `Color`,
  `String`, `Domain`, `Curve`, `Mesh`, and arbitrary `Value::custom::<T>(...)` domain payloads.
- **Extensible Component Registry & Clusters**: implement `NodeComponent` (or use `FnComponent`)
  to register custom nodes in seconds; encapsulate any sub-graph into a reusable `ClusterComponent`.
- **Incremental Topological DAG Evaluator**: Kahn's topological sort, cycle prevention, dirty
  downstream propagation, multi-wire stream merging (`Shift`-drag), and microsecond per-node
  profiling.
- **45+ Built-in Standard Library Nodes**: `Params` (`NumberSlider`, `IntSlider`, `BoolToggle`,
  `Panel`, `ColorSwatch`, `Domain`, `Relay`, `Scribble`, `Time`), `Math` (including a recursive-descent
  math `Expression` evaluator), `Logic`, `Sets` (`Series`, `Range`, `Repeat`, `CullPattern`, `Merge`,
  `Flatten`, `Graft`, `FlipMatrix`, `Random`), and `Vector / Geometry` (`RectGrid`, `CirclePoints`,
  `LineDivide`, `ColorHsv`).
- **Interactive FerroUI Canvas (`feature = "ferroui"`)**: `NodeGraphCanvas` control +
  `node_graph_editor` view featuring smooth pan & cursor-centered zoom, Grasshopper **Fancy Wires**
  (single Item, double-line List, dashed double-line DataTree), inline draggable sliders & toggles,
  colored node groups, marquee selection, minimap, hover DataTree tooltips, auto-layout, undo/redo,
  and zero-dependency JSON save/load.
- **Parametric Physics Bridge (`feature = "physics"`)**: built-in physics nodes (`Box`, `Circle`,
  `Polygon`, `RigidBody`, `Wrecking Pendulum`, `Suspension Bridge`, `Newtonian Star`, `Wind/Vortex`,
  `Jelly SoftBody`, `Tearable Cloth`, `WorldConfig`, `SceneOutput`) + 4 live templates wired
  directly into `samples/PhysicsSandbox` under the **🧩 Node Graph** tab.
