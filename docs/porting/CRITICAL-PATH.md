# Critical path

Subsystems the framework cannot work correctly without. Each is ported in full — no simplified stand-ins — and every gap left by a first pass gets a follow-up owner. Per-member status is in `TRACKING.md`; this page tracks the subsystems and their known gaps.

Status: **done** (ported, tested, integrated), **partial** (integrated with listed gaps), **in progress**, **queued** (blocked on the items named).

| # | Subsystem | Upstream location | Status | Known gaps / notes |
|---|---|---|---|---|
| 1 | Class model, property system | `Avalonia.Base` root, `PropertyStore/` | done | 29 upstream tests wait on styling / bindings / dispatcher integration |
| 2 | Dispatcher, timers, async on the dispatcher | `Threading/` | done | |
| 3 | Service locator, logging | `AvaloniaLocator.cs`, `Logging/` | done | |
| 4 | Collections | `Collections/` | partial | `FerroList`, `FerroDictionary` done; list extensions, weak events to port |
| 5 | Logical tree, `StyledElement` | `StyledElement.cs`, `LogicalTree/` | done | |
| 6 | Styling, themes, resources, name scopes | `Styling/`, `Controls/` (Base) | done | |
| 7 | Visual tree, `Visual` | `Visual*.cs`, `VisualTree/` | partial | composition attachment arrives with (12) |
| 8 | Layout | `Layout/` | partial | deferred bring-into-view requests in progress with the scrolling controls |
| 9 | Routed events, input, focus | `Interactivity/`, `Input/` | done | platform-side IME and drag-drop sources arrive with (18) |
| 10 | Media object model | `Media/` | done | |
| 11 | Imaging, effects, remaining media | `Media/Imaging`, `Media/Effects`, ... | done | |
| 12 | Drawing context, render data, compositor, render loop, `MediaContext` | `Media/DrawingContext*.cs`, `Rendering/` | partial | first frame on screen through `CompositingRenderer` (2026-10-05); composition brushes, animation groups / implicit / expression animations, custom visuals, drawing surfaces, server-side mutable tile/image/visual brushes, the Controls-based compositor hit-test suite and the render-thread contract decision remain |
| 13 | Fonts, text shaping contracts, text layout | `Media/Fonts`, `Media/TextFormatting` | done | |
| 14 | Data binding | `Data/`, `Data/Core/`, `Avalonia.Markup` | done | |
| 15 | Animation, transitions | `Animation/` | done | composition animations arrive with (12) |
| 16 | Render backend: Skia (Graphite/Metal, Ganesh, raster) | `Skia/Avalonia.Skia` | partial | drawing, text, effects, Graphite/Metal done; remaining text test suites, typeface downcast hook |
| 17 | Text shaping backend: HarfBuzz | `HarfBuzz/` | done | |
| 18 | macOS platform | `Avalonia.Native`, native lib | partial | windows, input, clipboard, drag source, Metal and software surfaces, native menus, application/dock menus, tray icon, `use_ferro_native`; storage, OpenGL, native control host, automation, file/URL activation wait for their contracts |
| 19 | Controls: `TopLevel`, `Window`, `Application`, lifetimes, platform contracts, templates, presenters, primitives, the control set | `Avalonia.Controls` | in progress | top-level/window/application/popups/layers, buttons, range, scrolling, shapes, text controls with selection handles, items controls, tooltips, flyouts, menus, tray icon, `ThemeVariantScope`, `SplitView`, `CommandBar`, `AutoCompleteBox`, `NumericUpDown`, storage contracts, pages including `DrawerPage`, calendar, date/time pickers, notifications, `FlexPanel`, pull-to-refresh, `TableView`, `PipsPager` and automation peers integrated; open: connected animations, `CommandBar` items source, peers for `DrawerPage` and `PipsPager`, composition animations in pull-to-refresh (wait for 12), the macOS automation wrapper |
| 20 | XAML: XamlX front end | `external/XamlX` | done | IL-coupled nodes listed for the back ends |
| 21 | XAML: Rust emitter, runtime interpreter, framework transformers, markup extensions, build integration | `Avalonia.Markup.Xaml*`, `Build.Tasks` | in progress | run-time library, compiler extensions and run-time loader integrated with the upstream test suites (540 passed, 11 ignored); class documents load with their includes; open: static-resource branches, removal of three loader fallbacks, converter culture, the gaps listed by the ControlCatalog sample; emitter designed in `xaml.md` §9, not started |
| 22 | Themes (Fluent, Simple) | `Avalonia.Themes.*` | in progress | Crates `ferroui-themes-simple` and `ferroui-themes-fluent`: all documents converted and embedded, both themes load through the run-time loader with every document of a ported control merged; the documents of unported controls are listed in `Controls/excluded.txt` of each crate. `themed_window` paints with both themes (`images/themed_window.png`, `images/themed_window_fluent.png`). Compiled loading needs the emitter of (21) |
| 23 | Desktop entry point | `Avalonia.Desktop` | partial | `use_platform_detect` on macOS; `hello_window` example paints (see `images/hello_window.png`) |
| 23a | ControlCatalog sample, ported page by page as the validation application for the port (owner request, 2026-10-04) | `samples/ControlCatalog`, `ControlCatalog.Desktop`, `samples/MiniMvvm`, later `ControlCatalog.Browser` | in progress | crates `control-catalog`, `control-catalog-desktop`, `mini-mvvm`; all 219 documents converted (`scripts/sync-control-catalog.sh`, `--check`) and embedded, loaded through the run-time loader; 57 documents load and their classes show in a window, the others are listed with what they wait for in `samples/ControlCatalog/excluded.txt` (unported controls: `DrawerPage`, `TabbedPage`, `CarouselPage`, `PipsPager`, calendar and pickers, colour picker, `TableView`, notifications; loader and metadata gaps with repro tests in `samples/ControlCatalog/tests/gaps*.rs`). The application starts with the Fluent theme and a temporary code-built shell (`samples/ControlCatalog/temporary.rs`) in place of `MainWindow`/`MainView`, which need `DrawerPage` (see `images/control_catalog.png`). |
| 24 | Browser platform | `Browser/` | queued | design in `browser-platform.md` |
| 25 | Vello render backend | – | queued | second renderer behind the same contracts |

Important but not on the critical path for a first working application: storage providers and dialogs, clipboard and drag-and-drop, automation/accessibility, native menus and tray icons, headless platform, remote protocol and designer support, the other OS backends.
