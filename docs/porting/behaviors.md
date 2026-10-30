# Port of Xaml.Behaviors

Issue [#281](https://github.com/wieslawsoltes/FerroUI/issues/281): port the behaviors, triggers and actions of
[Xaml.Behaviors](https://github.com/wieslawsoltes/Xaml.Behaviors) to FerroUI, completely.

## 1. What the reference contains

The reference is Xaml.Behaviors at commit `45387e32` (branch `master`, MIT, Copyright (c) Wiesław Šoltés),
built on Avalonia 12. Its C# namespaces are `Avalonia.Xaml.Interactivity` and `Avalonia.Xaml.Interactions.*`,
mapped into the default XAML namespace of Avalonia, so XAML uses the types without a prefix.

| Project | Files | Lines | Purpose |
|---|---|---|---|
| Xaml.Behaviors.Interactivity | 51 | 5,908 | The object model: `Interaction.Behaviors`, `BehaviorCollection`, `ActionCollection`, behaviors, triggers and actions on `AvaloniaObject` and on `StyledElement`, the collection templates for styles, the event registry and the helpers that find members by name |
| Xaml.Behaviors.Interactions | 53 | 5,252 | The classic set: `EventTriggerBehavior`, `DataTriggerBehavior`, `MultiDataTriggerBehavior`, `ChangePropertyAction`, `CallMethodAction`, `InvokeCommandAction`, timers, clipboard, storage pickers, file system, network |
| Xaml.Behaviors.Interactions.Custom | 332 | 20,787 | About 330 behaviors, triggers and actions for specific controls and services |
| Xaml.Behaviors.Animations | 22 | 3,974 | Key-frame and composition animation presets, the selection indicator animation, transition helpers |
| Xaml.Behaviors.Interactions.Events | 43 | 1,655 | A trigger and a behavior per input routed event |
| Xaml.Behaviors.Interactions.Draggable | 9 | 1,677 | Dragging and reordering elements in place |
| Xaml.Behaviors.Interactions.DragAndDrop | 30 | 3,737 | Drag and drop between controls and with the system, with a managed drag preview |
| Xaml.Behaviors.Interactions.Responsive | 5 | 718 | Classes switched by size and aspect ratio |
| Xaml.Behaviors.Interactions.ReactiveUI | 8 | 411 | ReactiveUI routing actions and the trigger of an `Interaction` |
| Xaml.Behaviors.Interactions.DragAndDrop.DataGrid | 1 | 209 | Row reordering in a `DataGrid` |
| Xaml.Behaviors.Interactions.Scripting | 3 | 140 | `ExecuteScriptAction`: C# run with Roslyn scripting |
| Xaml.Behaviors.SourceGenerators | 18 | 6,532 | Roslyn generator of typed actions and triggers |
| Xaml.Behaviors, Xaml.Behaviors.Avalonia | 1 | 15 | The bundle and the meta package (no code) |

Tests: 527 xUnit test methods in five projects (Interactivity 124, Interactions 149, Animations 41, source
generator 211, its integration tests 2), headless with real rendering. Samples: BehaviorsTestApplication
(231 pages, one per behavior), AnimationsTestApplication (16), SourceGeneratorSample (11).

## 2. Crates and places

| Reference project | Crate | Path |
|---|---|---|
| Interactivity | `ferroui-xaml-interactivity` | `src/Behaviors/FerroUI.Xaml.Interactivity` |
| Interactions | `ferroui-xaml-interactions` | `src/Behaviors/FerroUI.Xaml.Interactions` |
| Interactions.Custom | `ferroui-xaml-interactions-custom` | `src/Behaviors/FerroUI.Xaml.Interactions.Custom` |
| Animations | `ferroui-xaml-behaviors-animations` | `src/Behaviors/FerroUI.Xaml.Behaviors.Animations` |
| Interactions.Events | `ferroui-xaml-interactions-events` | `src/Behaviors/FerroUI.Xaml.Interactions.Events` |
| Interactions.Draggable | `ferroui-xaml-interactions-draggable` | `src/Behaviors/FerroUI.Xaml.Interactions.Draggable` |
| Interactions.DragAndDrop | `ferroui-xaml-interactions-draganddrop` | `src/Behaviors/FerroUI.Xaml.Interactions.DragAndDrop` |
| Interactions.Responsive | `ferroui-xaml-interactions-responsive` | `src/Behaviors/FerroUI.Xaml.Interactions.Responsive` |
| Interactions.ReactiveUI | `ferroui-xaml-interactions-reactiveui` | `src/Behaviors/FerroUI.Xaml.Interactions.ReactiveUI` |
| Xaml.Behaviors (bundle) | `ferroui-xaml-behaviors` | `src/Behaviors/FerroUI.Xaml.Behaviors` (a facade with a feature per crate) |
| BehaviorsTestApplication | `behaviors-gallery` | `samples/BehaviorsGallery` |

The namespaces map `Avalonia.Xaml.Interactivity` to `FerroUI.Xaml.Interactivity` and
`Avalonia.Xaml.Interactions.X` to `FerroUI.Xaml.Interactions.X`, all in the default FerroUI XAML namespace
(`https://github.com/ferroui`), as the reference maps its namespaces into its framework's default namespace.
Each reference file is one Rust file in the same folder, with the same type and member names.

## 3. Decisions

- **Generic classes.** `Behavior<T>`, `StyledElementBehavior<T>`, `Trigger<T>` and the other generic bases are
  non-generic classes that store the expected type of the associated object and check it on attach, as the
  reference throws; a typed accessor returns the associated object as `T`. FerroUI classes are not generic.
- **Members by name.** The reference finds properties, methods and events with reflection. The port finds
  them in the markup metadata of the framework (`ferroui_base::metadata::members`, the routed event registry,
  the property registry), which every class registered for XAML declares. A member that is not in the metadata
  is not found, as a member that does not exist is not found by the reference.
- **Framework additions** (stage 1): public lookup of members by name, of routed events by name, plain events
  that can be unsubscribed, and an ordering of untyped values.
- **Waived** (recorded in DEVIATIONS.md, section Behaviors):
  - Interactions.Scripting: it runs C# source with Roslyn; there is no C# run time in a Rust application.
  - Interactions.DragAndDrop.DataGrid: FerroUI has no `DataGrid` (only the TreeDataGrid).
  - SourceGenerators: compile-time C# tooling. Typed actions and triggers in Rust are written directly; a
    procedural macro that generates them is a later addition, not part of this port.
  - The `BinaryFormatter` path of `GetClipboardDataAction` (obsolete and unsafe in .NET).
- **Platform services** (HTTP, network status, file watching, files and directories) are behind Cargo
  features of the crates that use them, and are not built for the browser.

## 4. Stages

Each stage ends with its ported tests passing on macOS and in CI.

1. Framework additions; the Animations crate (it does not depend on the behaviors).
2. Interactivity, with its 124 tests (the lifecycle tests of `InteractionTest` verbatim).
3. Interactions, Events, Responsive, Draggable, DragAndDrop and ReactiveUI, with the 149 tests of Interactions.
4. Interactions.Custom, by folder.
5. The facade crate, the gallery sample, the tracking pages ([`docs/porting/tracking/behaviors/`](tracking/behaviors/TRACKING.md)), the README
   table and the CHANGELOG.

## 5. Progress

| Stage | State |
|---|---|
| 1 | Done: the framework additions; `ferroui-xaml-behaviors-animations` (40 tests) |
| 2 | Done: `ferroui-xaml-interactivity` (121 tests) |
| 3 | Done: `ferroui-xaml-interactions` (63), `-events` (55), `-responsive` (23), `-draggable` (22), `-draganddrop` (46), `-reactiveui` (44) |
| 4 | Done: `ferroui-xaml-interactions-custom` (59 tests), in four parts (`part_a.rs` to `part_d.rs`) |
| 5 | In progress: the facade crate `ferroui-xaml-behaviors` is done; the tracking pages, the README and the CHANGELOG are done; the gallery is under way |

## 6. Tracking

[Generated tracking](tracking/behaviors/TRACKING.md) maps each project of the reference to its crate and counts
the files, the public types and members, and the tests that are ported, missing and waived, with one page per
project. It is the reference `behaviors` of `scripts/port-status/references.toml`; its data (the extraction, the
path overrides, the waivers and aliases, the test aliases and the test reports) is in `docs/porting/data/behaviors/`.
Regenerate it with `python3 scripts/port-status/test_gaps.py --reference behaviors --all`, then
`scripts/port-status/run.sh --reference behaviors` (`scripts/port-status/README.md`).
