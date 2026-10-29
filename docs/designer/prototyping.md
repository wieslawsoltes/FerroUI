# Multiple views and interactive prototypes

**Prototype** connects existing preview stages into named journeys. A stage remains an
independent native XAML view with its own size, theme, definition, design-data setting and
visual-state scenario. The prototype adds a starting screen and source-anchored
interactions between those views. It does not add event handlers, navigation classes or
temporary visibility changes to the application's XAML.

The stage board answers “how does this view behave in these environments?” The prototype
answers “where can the user go from here, and what happens when they return?” Both use the
same source snapshots, anchors, native loader, preview factories and configuration model.
See [preview stages](xcode-previews.md) for native layout, state scenarios and environment
contracts.

![Native prototype overview with named flows and connected screens](images/designer-prototype-overview.png)

## Research and design choices

The comparison was reviewed on **2026-10-29** using the vendors' published documentation.
Historical material is identified explicitly.

| Reference | Relevant workflow | FerroUI implementation decision |
| --- | --- | --- |
| [Figma prototype flows](https://help.figma.com/hc/en-us/articles/360039823894-Create-and-manage-prototype-flows) | Named starting frames, shared screens and several journeys in one document. | Store named flows separately from screen configurations; a stage can participate in several flows. |
| [Figma connections](https://help.figma.com/hc/en-us/articles/4411431245335-View-prototype-connections) and [actions](https://help.figma.com/hc/en-us/articles/360040035874-Prototype-actions) | Inspect connections, choose a destination, and distinguish forward navigation from Back. | Keep an inspectable connection overview and explicit Navigate/Back actions. Resolve Back from the current run's history. |
| [Figma sections](https://help.figma.com/hc/en-us/articles/16194160540567-Use-sections-in-prototyping) | Sections group related views and can remember their last-visited child. | Add stage search and current-document/all-view browsing. Remembered group destinations remain a separate future player contract. |
| [Penpot prototyping](https://help.penpot.app/user-guide/prototyping-testing/prototyping/) and [View mode](https://help.penpot.app/user-guide/prototyping-testing/testing-view-mode/) | Connect source layers to screen boards, search destinations, choose flows and display interaction hints. | Use namespace-qualified source anchors, searchable stage choices, flow selection and optional native hotspot hints. |
| [Android Studio Navigation Editor](https://developer.android.com/guide/navigation/design/editor) and [navigation graphs](https://developer.android.com/guide/navigation/design) | A destination list, graph and attributes describe one graph; playback has a separate back stack. The visual editor targets XML navigation resources. | Share one graph between native editors, overview, persistence and automation. Keep the navigation cursor independent of authored source and source undo. |
| [Xcode storyboards](https://developer.apple.com/documentation/uikit/showing-and-hiding-view-controllers) and [unwind segues](https://developer.apple.com/documentation/uikit/dismissing-a-view-controller-with-an-unwind-segue) | An action connects a source object to another view; return navigation can resolve to an existing previous view. | Source-anchored activation with retained native controls when navigating Back. The prototype is independent of UIKit or application router APIs. |
| [Qt Design Studio Flow Mode introduction](https://www.qt.io/blog/introducing-flow-mode-qt-design-studio-1.5), **historical 2020 reference**, and [current Connections documentation](https://doc.qt.io/qtdesignstudio/quick-signals.html) | Flow Mode described action areas, connected screens, Go Back and live presentation. Connections associates native signals with actions and state changes. | Add a native prototype player and interaction model while retaining existing XAML behavior/state authoring for application logic. The historical article is not a claim about current Flow Mode availability. |
| [Microsoft SketchFlow description](https://learn.microsoft.com/en-us/archive/msdn-magazine/2009/september/the-tao-of-screen-design), **archived 2009 reference** | Screens form a visual navigation map; reusable Component Screens map to UserControls. | Reuse native document and definition stages for complete screens and components. SketchFlow is a historical design precedent, not a current Blend feature claim. |

The first implementation concentrates on finding views, describing connections and
playing reliable navigation with real controls. It uses the current native state and
binding systems for screen content. The additional contracts needed for overlays,
animated transitions and prototype variables are described under [Boundaries](#boundaries).

## Try the sample

From the repository root:

```sh
cargo run -p designer-studio -- --file docs/designer/samples/PrototypeFlow.xaml
```

The [sample](samples/PrototypeFlow.xaml) is a single ordinary `UserControl` named
`PrototypeScene`. Its **Screen** state group contains **Home**, **Details** and **Settings**.
The default authored view is Home. Named Buttons provide unambiguous interaction targets;
the text fields and checkboxes demonstrate retained native input.

1. Open **Stages**. Edit the initial stage, name it **Home**, and choose **Home** in the
   **Screen** group. A phone size such as 390 × 700 works well.
2. Duplicate it twice. Name the copies **Details** and **Settings**, and select the
   corresponding Screen state for each.
3. Open **Prototype** and add a flow named **Explore** with **Home** as its start. A second
   flow, **Settings review**, can start at **Settings** while sharing that stage with Explore.
4. Use a stage's **… → Add interaction…** to create the connections below. Select the named
   source element, action and destination, then apply the detached draft.
5. Play **Explore**. Type a new value in the Home note field, open the project, open settings,
   and use Back twice. The note retains the value entered during this run.
6. Try **Restart journey** to return to the flow's start while preserving this run's native
   controls. **Exit prototype** and start Play again to create fresh native views.
7. Use **Save set** to store the stage configurations and prototype graph together. Keep the
   referenced XAML source available when loading it again.

| Source stage | Named source element | Action | Destination |
| --- | --- | --- | --- |
| Home | `HomeOpenDetails` | Navigate | Details |
| Home | `HomeOpenSettings` | Navigate | Settings |
| Details | `DetailsOpenSettings` | Navigate | Settings |
| Details | `DetailsBack` | Back | Previous screen in this run |
| Settings | `SettingsBack` | Back | Previous screen in this run |

Settings can therefore return to Home or Details according to the path actually taken.
It does not require a separate fixed reverse connection for each possible origin. In
Settings review, Settings is the first screen and there is initially no previous screen.

The note field is `HomeNote`; the project checkbox is `DetailsChecklist`; the Settings
controls are `SettingsReminders`, `SettingsCompleted` and `SettingsName`. These names are
also useful for embedding tests and native inspection. The graph lives in the stage set,
so opening the sample alone does not silently install prototype connections.

## Finding and organizing views

The stage board provides **Current document and pinned** and **All views** browsing,
together with a search field. All views makes stages from other open source documents
available without changing their pinned settings. Filtering changes the board's presentation; it does not
delete stages, rewrite source, rebuild their native graphs or discard Live input.

Stage names, source identities and definition contexts distinguish otherwise similar
cards. A screen that appears in several flows is still one stage configuration. The
prototype overview lists the selected flow's reachable screens and their connections;
Navigate edges determine static reachability. Back is shown as a history action rather
than an invented edge to a fixed destination.

The overview search filters screen names, source identities and interaction rows. Clear
**Only screens reachable from this flow** to inspect stages outside the selected journey.
Select a screen to list its outgoing interactions; **Show all interactions** returns to
the complete list within the current flow/search scope.

Source and destination pickers use existing stages and native ComboBox type-ahead. They do
not open arbitrary paths or create placeholder application screens. Add or configure a
stage through the normal stage editor before connecting it. Document tabs, pinning, definition navigation, comparison
matrices and the stage board's sixteen-view capacity retain their existing contracts.

## Authoring connections

A flow has a unique, nonempty name and a starting stage. An interaction has a display name,
a source stage, a `PreviewSourceAnchor` and one of two actions:

- **Navigate** moves to a specified stage and records the previous screen in the current
  run's history.
- **Back** returns to the previous screen. Its destination depends on that run; it has no
  hard-coded target.

Each source control has at most one prototype interaction in a stage. Two stages showing
the same XAML may attach different interactions to that control. The anchor records the
namespace, type, authored name or resource key along the source path, so an unrelated
numeric AST identifier cannot become the target after a document is parsed again.

![Native interaction editor with authored hotspot and destination choices](images/designer-prototype-editor.png)

Editors work on detached drafts. Cancel leaves the graph unchanged. Applying a draft
checks the current stage/source model, validates the complete graph and rejects a stale
source or destination instead of attaching the action to a different control. Graph
changes remain designer presentation state and leave source text and source undo history
intact.

An authored control must also resolve to a current, usable native target before its
interaction can run. Removed or ambiguous anchors, stale source snapshots and repeated
template instances produce diagnostics. Use uniquely named controls or an isolated
definition stage where the authored target has one native instance.

## Playing a flow

The player owns a separate, bounded set of native stage graphs for the run. It copies the
accepted stage configurations and uses the same source snapshots, preview factories,
themes, design-data projection, content-fitting layout and selected state endpoints as
the board. A board card and the player never attempt to parent the same native control
simultaneously.

This isolation gives playback its own native text, checkbox and scroll state. Navigating
away and returning reuses that run's view, so entered values remain available. Those values
do not replace the board's Live values or the values authored in XAML.

Isolation is at the native control graph level. Host-provided factory services and view
models retain their existing lifetime and sharing behavior.

![Native prototype player retaining the note entered during the journey](images/designer-prototype-player.png)

Prototype hotspots are native transparent Buttons positioned over the resolved source
controls. They provide pointer/touch activation and native keyboard focus/activation.
Optional hints identify the available interactions. Linked hotspots activate the
prototype action; controls outside those hotspots continue to receive native Live input.
Focus and keyboard activation remain owned by the player instead of reaching the source
editor's authoring shortcuts.

The player exposes the current flow and screen, Back availability, Restart and Exit.
**Restart journey** clears navigation history and returns to the flow's start while
retaining this run's native controls. **Exit prototype**, followed by **Play**, starts fresh native instances.
Starting another flow also creates a new run.

Source, dependency, stage configuration or graph changes retire the running player. Reopen Play against the
new accepted model. A stale callback cannot advance a replacement run, and a pending or
invalid XAML draft cannot be represented as a current interactive screen.

## Persistence and limits

**Save set** writes a version-2 preview-stage manifest containing the stage configurations
and the prototype graph. Its flow starts and interaction endpoints use indices into that
manifest's stage list. Loading allocates fresh runtime stage IDs and remaps every graph
reference to those new IDs. Runtime control IDs and source `NodeId` values are never saved
as persistent interaction targets.

Version-1 stage sets still load, with an empty prototype graph. Validation of a complete
incoming set finishes before replacing the accepted set. Unknown fields, invalid indices,
duplicate flow names, duplicate source interactions, malformed anchors and oversized
descriptors are rejected. Reordering stages preserves logical connections; removing a
stage removes flows that start there and interactions to or from that stage.

The current bounds are:

| Resource | Limit |
| --- | ---: |
| Native stage configurations | 16 |
| Named prototype flows | 32 |
| Prototype interactions | 128 |
| Navigation history entries in one run | 64 |
| Flow and interaction labels | 128 UTF-8 bytes each |
| Prototype descriptor text | 512 KiB |
| Complete saved manifest | 1 MiB |

When history is full, the next forward navigation reports the limit without changing the
current screen. Go Back or Restart before continuing. A navigation action targeting the
current screen keeps the current history unchanged.

The manifest references source documents; it does not embed their source, ship application
assemblies, recreate factory registrations or grant filesystem/network access. The host
must keep those documents and dependencies available just as for ordinary saved stages.

## Embedding and automation

The portable model and the native views can be reused independently:

| Type | Responsibility |
| --- | --- |
| `PreviewPrototype`, `PreviewPrototypeFlow`, `PreviewPrototypeLink`, `PreviewPrototypeAction` | Bounded live graph, named starts, source anchors, Navigate/Back and reachability |
| `PreviewPrototypeManifest` | Positional persistence and fresh stage-ID remapping |
| `PreviewPrototypeSession` | Navigation cursor and bounded history; no source session or native control ownership |
| `PreviewPrototypeView` | Native flow and connection overview |
| `PreviewPrototypeFlowEditor`, `PreviewPrototypeLinkEditor` | Detached native drafts |
| `PreviewPrototypePlayerView` | Native presentation and interaction controls |

Workspace APIs expose the same behavior:

- `preview_prototype`, `set_preview_prototype` and `preview_prototype_model` inspect,
  validate and install the graph.
- `preview_prototype_playback` reports the current run.
- `set_preview_prototype_visible` opens or closes the overview.
- `start_preview_prototype`, `activate_preview_prototype_link`, `back_preview_prototype`,
  `restart_preview_prototype` and `stop_preview_prototype` control playback.

Hosts using `PreviewPrototypeSession` independently must validate source/native authority
before activating an interaction. The cursor alone deliberately has no access to native
views, document drafts, dependency snapshots or application input permissions.

| MCP tool | Behavior |
| --- | --- |
| `designer_prototype` | Read the graph, optimistic model, flow/reachability and playback state; optionally include source element choices for a stage. |
| `designer_prototype_action` | Apply `set_graph`, or execute `play`, `activate`, `back`, `restart` and `stop` against `expected_model`. |

Read operations are observational: they do not construct previews, advance playback or
apply pending source. Mutations use the host's write permission. `play`, `activate`, `back`
and `restart` additionally require input permission; stopping a run does not require input
permission. Native UI and automation share graph validation and current source checks.
The containing stage manifest remains available through the existing preview-stage
persistence API.

## Boundaries

This is a design-time prototype. It does not generate application router code, rewrite
event handlers, or claim the platform-specific behavior of UIKit segues or Android's
Navigation component. A production application continues to own its navigation service,
view models, bindings and native event handlers.

Overlays, overlay dismissal/focus stacks, animated transitions, delayed/hover triggers,
conditional branches, prototype variables, remembered group destinations and externally
shareable hosted prototypes are not implemented by this graph. Each needs an explicit
lifetime, state, rendering or hosting contract. In particular, an overlay should preserve
the underlying screen and define how Back, focus restoration and outside-click dismissal
interact; it should not be approximated as an ordinary forward screen edge.

Figma's [overlay documentation](https://help.figma.com/hc/en-us/articles/360039818254-Create-overlays-in-your-prototypes)
and [variable/conditional workflows](https://help.figma.com/hc/en-us/articles/14506587589399-Use-variables-in-prototypes)
are useful references for those extensions. FerroUI's existing native states, behaviors
and bindings remain available for application logic without introducing a second
expression language into prototype navigation.

Phone/tablet dimensions still describe the current host's native FerroUI layout. The
player does not emulate another operating system, inject device safe areas or simulate
platform accessibility settings. Native layout and interactive prototype checks complement
the application's normal platform and accessibility tests.
