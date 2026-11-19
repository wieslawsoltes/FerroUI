# An Osprey Native host on FerroUI (`ferroui-osprey-native`)

[Osprey](https://github.com/Nimblesite/osprey) is a functional language whose user interfaces
are written with Osprey Native, an Elm-like application framework: a `view` of the state and
`update(message, store)`. An Osprey Native application is compiled as a C library for a host, and
the host draws what the application returns and sends back what the person does. Osprey ships
several hosts (SwiftUI, GPUI, WinUI, Android, a VS Code webview). `ferroui-osprey-native`
(`src/FerroUI.OspreyNative`) is one more, on FerroUI, with the same design as Osprey's GPUI host:
an Osprey Native application runs on FerroUI unchanged. This is option (d) of the Osprey research
of issue #950.

The crate is not published yet (`publish = false`).

## 1. What runs

| Application | Source | Result |
|---|---|---|
| The probe | `src/toolchain/hosts/gpui/probe` of Osprey | Runs: the count, Add, the note field and Load. The smoke run (start, tap Add, "Count 1") passes through the real boundary, in the binary (`--smoke`) and as a Rust test through the FerroUI tree |
| flock | `src/osprey/examples/gpui/flock` | Runs: the HUD badges, Pause, the field of 1000 birds, the hawk and the dog animated on the frame clock (a resident application: the birds are streamed through `native_shape`), the Flock size picker. Its recorded `smoke.jsonl` (11 steps: frames, Pause and Resume, a key, a pointer, the picker) passes through the real boundary, in the binary and as a Rust test |
| painting | `src/osprey/examples/gpui/painting` | Runs: the header with Dashed and Clear, and the canvas in window pixels with the squares, translucent discs, the logo path, the lightning bolt, the star and the bar with two-stop gradients, the pie of arcs and the zig-zag strip. Its `smoke.jsonl` (Dashed switches to Solid) passes |

## 2. Building and running

The host speaks the boundary of the released Osprey 0.29: the exports `start` and `dispatch`
with JSON envelopes (section 3). Osprey's main branch has since moved to a frame of fixed-width
rows leased from a shared bridge (`native.td`, `frame.rs`, `rows.rs` of its GPUI host); that is not
in a release yet, and an application compiled with 0.29 speaks the envelope. When a release ships
the rows, the host gets a second reader for them; the mapping, the tree and the scene drawing stay.

An application is compiled as a library for the host with the Osprey compiler:

```sh
osprey <app folder> --compile --target=macos -o <build folder>/libOspreyApp.a
```

The archive needs Osprey's runtime beside it: `libosprey_runtime_macos_arc.a`, which ships in the
folder of the compiler. `build.rs` links both when `OSPREY_NATIVE_APP_DIR` names the folder of the
archive and `OSPREY_RUNTIME_DIR` the folder of the runtime; it also links the Metal and Foundation
frameworks the runtime calls. A Linux or Windows archive bundles its runtime and needs only the
system libraries (`pthread`, `dl`, `m`; `ws2_32`). More libraries an application needs are named in
`OSPREY_NATIVE_LINK_SEARCH` and `OSPREY_NATIVE_LINK_LIBS`.

```sh
OSPREY_NATIVE_APP_DIR=<build folder> OSPREY_RUNTIME_DIR=<osprey folder> \
    cargo run -p ferroui-osprey-native --features desktop --profile fast-release
```

| Variable | Meaning |
|---|---|
| `OSPREY_NATIVE_APP_NAME` | The application's name: its store is `~/Library/Application Support/Osprey/<name>` on macOS, where the other hosts keep it |
| `OSPREY_NATIVE_TITLE` | The window's title (default "Osprey") |
| `OSPREY_NATIVE_WINDOW` | The window's size, `WIDTHxHEIGHT` (default `1200x800`) |

`--smoke` runs without a window (section 7). Without `OSPREY_NATIVE_APP_DIR` nothing is linked:
the library and its tests build and run against a stand-in application in Rust, and the binary
says how to link one. The feature `desktop` (the window: the desktop platform and the Fluent
theme) is off by default, so the library builds without the renderer.

## 3. The seam

The crate is only the seam between FerroUI and the application. No rule of an application lives
in it: it never decides what a tap means or what a row shows.

| Module | What it does |
|---|---|
| `boundary.rs` | The C calls: `osprey_main` once, then `osprey_app_start(context)` and `osprey_app_dispatch(state, event)`, on the UI thread, one at a time. Returned strings are copied and released with `osprey_release_string`. The imports the runtime calls back: `native_slice` (text the host holds for the application) and `native_shape` (scene primitives a resident application streams) |
| `protocol.rs`, `schema.rs` | The envelope (`version`, `state`, `slices`, `patches`, `removed`, `requests`, `diagnostics`, `theme`) and the events. An envelope is checked whole before any of it is used: every node of every patch against the schema of its kind (41 kinds, each member's type), every scene primitive. One malformed part refuses the whole envelope with a diagnostic that names it; the regions stay as they were |
| `session.rs` | The opaque session (sent back unchanged with the next event), the regions by key with their revisions, the theme, the slices, and the capabilities answered (section 6) |
| `view.rs` | One mapping per node kind to FerroUI controls (section 4) |
| `host.rs` | One FerroUI tree for the session: a component of `ferroui-view` that runs again for each envelope; the reconciler patches the controls it made before |
| `scene.rs`, `surface.rs` | Scene nodes: the primitives decoded, and the control that draws them with FerroUI's drawing context (section 5) |
| `desktop.rs` | The window, the frame clock, the light and dark appearance |
| `smoke.rs` | The windowless check (section 7) |

**Events.** A control sends back the region, its revision and the ordinal the framework issued
with the control, and the value the control delivered: `null` for a tap, the text of a field, the
state of a toggle, the key of a choice. The host adds nothing. A control's change that the host
itself made (patching a field's text to the application's value) is not sent.

**The retained tree.** Each envelope bumps a version that the tree's component reads; the
component maps the regions again, and the reconciler of `ferroui-view` (docs/declarative-ui.md,
section 8) matches the new elements to the controls of the last run: a region is keyed by its key,
so it keeps its controls wherever its slot moves, and the controls inside it are matched by class
and place. A control that survives keeps its focus, caret, scroll offset and selection; its handler
is swapped in place and sends with the new revision. An equal value is not set again.

## 4. Node kinds

Every node kind has exactly one mapping. A kind this host does not draw yet is drawn as a red
box that names it, with a diagnostic; it is never drawn as something else. `DRAWN` and `REFUSED`
in `view.rs` together list every kind of the schema once, and the tests fail when a kind is in
neither, in both, or drawn without its mapping. With `OSPREY_NATIVE_SOURCE` naming the `src` folder
of the framework's `@osprey/native` package, a test checks the schema against the framework's own
`Node` type, so a kind added there fails until this host lists it.

| Kind | FerroUI |
|---|---|
| `stack` | `StackPanel` (vertical or horizontal), the theme's second spacing step apart; a `Grid` when a `spacer` is among the children, the spacers star-sized |
| `text` | `TextBlock`, wrapping; the role sets the size and weight (`title` 20 semibold, `caption` 12, `mono` a monospaced family), the tone the colour |
| `rich-text` | A `WrapPanel` of `TextBlock` runs, each with its ink token, italics and emphasis (`strong` semibold, `code` monospaced) |
| `evidence`, `pair` | The label muted beside the value, in a horizontal `StackPanel` |
| `label` | The symbol's glyph beside a `TextBlock` |
| `icon` | The symbol's glyph at 28 points |
| `badge` | A rounded `Border` outlined in the tone's colour, the symbol and the label inside |
| `link` | `HyperlinkButton` with `NavigateUri`; an address that is not `https` is refused, as the framework asks |
| `spacer` | Free space in a stack |
| `divider` | `Separator` |
| `segments` | Small rounded `Border` blocks in the tones' colours, and the label muted |
| `empty` | A centred column: the symbol large, the title semibold, the detail muted |
| `progress` | The label, the detail muted, and an indeterminate `ProgressBar` |
| `meter` | `ProgressBar` from 0 to `total` at `value`, in the tone's colour |
| `swatch` | A 16-point `Border` in the light or dark colour of the current appearance, and the name |
| `brand` | The wordmark "Osprey", semibold, in the accent colour |
| `slot` | The region it names, in place, keyed by the region |
| `button` | `Button`: the symbol's glyph before the label, `IsEnabled`; `prominent` takes the accent style and colour, `destructive` the `danger` colour, a plain one the theme's `button`/`buttonText` colours when it names them; a click sends a tap |
| `field` | A label above a `TextBox` (`PlaceholderText` from the prompt, `PasswordChar` when secure, `AcceptsReturn` and wrapping when multiline); `edit` sends each change, `submit` the text on Return |
| `toggle` | `ToggleSwitch` with the label as its content; a change sends the new state |
| `picker` | The label beside a `ComboBox` of the choices' labels; choosing one sends its key |
| `disclosure` | `Expander` with the title as its header and the children as its content; expanding or collapsing sends the new state |
| `alert` | While visible, a card with the title, the message and an OK button that sends the dismissal |
| `section` | A card (outlined, rounded, padded): the title semibold, the children, the footer muted |
| `screen` | A `Grid`: the title and subtitle with the toolbar at the trailing edge, above the children |
| `split` | A `Grid` of star columns, one per pane, with a `GridSplitter` between each two |
| `layer` | A `Grid` with every child in the same cell, the first at the back |
| `framed` | A `Border` with the frame's size, insets, radius, `fill` and `line` tokens (on the edges `lineEdge` names), alignment, and a `ScrollViewer` when `scrollY`; a `ContentControl` inside carries its font, text size, italics and `ink`. A frame sized to the window (`viewportHeightInset` other than -1) is refused, as the GPUI host refuses it |
| `themed` | A `Border` on the scoped theme's `background`, padded three spacing steps; the scoped theme's colours and spacing apply to everything below |
| `dialog` | A card drawn in place: the title, the body and a Close button that sends the dismissal |
| `context-menu` | The body with a `ContextMenu`: each button among the actions is a `MenuItem` that sends its tap |
| `aside` | A `Grid` of the pane, a `GridSplitter` and the aside column at its leading or trailing edge, `width` wide within `least` and `most`; dragging sends the aside's new width |
| `tabs` | `TabControl`: a `TabItem` per entry, its label as the header and its content below; choosing a tab sends its key |
| `rows` | `ListBox`: a `ListBoxItem` per row (its symbol, then its content), a disabled header item where the group changes; choosing a row sends its key |
| `editor` | A monospaced multi-line `TextBox` (single-line when `inline`): its text goes to `event`, its caret to `placeEvent` |
| `scene` | `SceneSurface`, a control that draws the primitives with FerroUI's drawing context (section 5) |
| `math`, `image`, `video`, `board` | Drawn as a box naming them, with a diagnostic (section 8) |

**Symbols.** FerroUI bundles no symbol font (a `SymbolIcon` needs one the application supplies),
so each of the framework's 33 symbols is a character of the system font (`symbols.rs`); a test
fails when a symbol has none. An unknown symbol name is drawn as a box and reported.

## 5. Scenes

A `scene` node is drawn by `SceneSurface` (`surface.rs`), a control whose `render` replays the
primitives on FerroUI's drawing context, which the platform's renderer (Skia on the GPU on macOS)
draws; nothing is rasterised on the CPU. The scene is decoded once per region revision (`scene.rs`):
every primitive is checked, a malformed one refuses the envelope, and the shapes, with the ones a
resident application streamed through `native_shape`, are sorted by depth, stably.

| Code | Primitive | FerroUI |
|---|---|---|
| 0 | rectangle (centre, size, angle, colour) | `fill_rectangle` under a rotation about the centre |
| 1 | disc | `draw_ellipse_at` |
| 2 | line strip | a `StreamGeometry` polyline stroked with round joins |
| 3 | sprite | its tinted quad: the atlas images are not loaded yet, as the GPUI host draws a sprite whose image it has not loaded |
| 4 | glyphs | `FormattedText` with the top of its line at the point |
| 5 | path | a `StreamGeometry` replaying the steps in order (move, line, quadratic, cubic, arc, polygon, close); filled, or stroked with its width, bevel or miter joins and dashes; a solid paint or a two-stop `LinearGradientBrush` at the angle (0 up, 90 right); the path's own transform before the scene's |
| 6 | lettering | `FormattedText` at the weight, left, centre or right anchored at the point |

**Placement.** A sized scene is drawn from the control's top left at its `zoom`, an upper bound
that shrinks uniformly when the region would clip the scene; a zoom of 0 draws it as large as fits.
A scene of zero width and height is in window pixels: its origin is the window's top left and it
is not clipped (`Ui::canvas`).

**Input** goes through the scene's binding, in scene units: the pointer as
`{"x", "y", "down", "shift"}` on press, release, movement while pressed and once per movement
otherwise; keys as `{"key", "down"}` with the host-neutral names (`left`, `right`, `up`, `down`,
`space`, `enter`, `escape`, `tab`, `backspace`, a lowercase letter or digit; other keys are not
sent); and, for a sized scene, the region's size `{"width", "height"}` in scene units when it is
first drawn and whenever it changes. A press focuses the scene and captures the pointer.

## 6. Capabilities

| Kind | Answer |
|---|---|
| `animate` | Turns the frame clock on or off and completes. While it is on, the window sends one `frame` event per animation frame of FerroUI's media context, with the seconds since the one before (at most 0.1) |
| `store-read`, `store-write`, `store-list`, `store-delete` | Files under the application's store folder; a read puts the text into the slice the request names, a write takes it from one |
| `cancel` | Completes: every capability this host answers completes at once, so nothing is in flight |
| any other kind | Completed with the error "unsupported capability <kind>" and a diagnostic naming it, as the GPUI host does for what it does not answer |

The launch context carries the time, the local UTC offset, the platform as
`std::env::consts::OS` spells it, the home folder and the arguments. The window sends the
lifecycle phase `active` when it opens.

## 7. Verification

- `cargo test -p ferroui-osprey-native --lib`: the schema, the envelope, the scene decoder, the
  coverage of kinds and symbols, and a stand-in application in Rust that speaks the probe's
  protocol, driven through the FerroUI tree (a click on Add reaches the application, the tree
  shows "Count 1", and the button and the field keep their controls across the patch). It needs no
  Osprey.
- With the probe linked and `OSPREY_NATIVE_APP_NAME=probe`, `the_linked_probe_counts` drives
  Osprey's own probe through the real boundary (start, find Add by its node, send the tap, read
  "Count 1" in the root region) and then through the FerroUI tree. CI runs it where Osprey is
  installed; elsewhere it is not compiled.
- With an application linked and `OSPREY_NATIVE_SMOKE` naming its `smoke.jsonl`,
  `the_linked_application_replays_its_smoke_script` replays it through the real boundary.
- `ferroui-osprey-native --smoke`: the probe's check in the binary; `--smoke <smoke.jsonl>`
  replays an application's recorded script (the format of Osprey's examples: a `send` of an
  event, a control found by region and label, or null for the start envelope, and an `expect` of
  regions, patched regions, a text, the capabilities asked for and the count of diagnostics).

## 8. State and gaps

- Drawn: 37 of the 41 kinds (section 4). Not drawn yet, drawn as boxes that name them: `math`
  (typeset mathematics), `image` and `video` (media from the application's sources), `board`
  (the stroke canvas of the framework).
- `editor`: its marks (underlines, badges and notes by line) and `focusEpoch` are not applied.
- `framed`: the control metrics (control height and padding, symbol size, quiet, icon-only, flat
  rows, selection group, shadow) are not applied; FerroUI's controls keep their theme's metrics.
- `dialog` is drawn in place, not as a window-modal overlay.
- Scenes: sprites are drawn as their tinted quads (no atlas images); the accessibility labels of
  scene text are not exposed.
- Capabilities: only `animate`, the store and `cancel` are answered (section 6).
- Checked on macOS by the smoke runs, the tests and screenshots of the three windows; pointer
  and keyboard input in the windows (drawing in painting, steering the dog in flock, typing in
  the probe's note) was not driven by hand. No timings were taken.
- Only macOS was run. The Linux and Windows link lines of `build.rs` follow Osprey's library
  targets but were not tried.

## 9. Where this host differs from the GPUI host

- **Styling.** The GPUI host holds no style of its own beyond its kit. Here the application's look
  is applied where its rows carry it (the theme's colour roles, its spacing scale, the text roles,
  the button roles), and where a node leaves a value open FerroUI's theme draws it: a button
  without `button`/`buttonText` colours is a Fluent button, a field is a Fluent `TextBox`.
- **Symbols** are characters of the system font, not icons of a bundled set.
- **The boundary** is Osprey 0.29's envelope; the GPUI host of Osprey's main branch reads rows.
