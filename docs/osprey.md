# FerroUI from Osprey

Osprey programs use FerroUI through generated bindings over the C ABI (docs/c-abi.md), so
they never write `extern fn` declarations, ID lookups or status checks by hand. The bindings
are generated from the API model of the framework (bindings/README.md) for every class,
property and event, in two layers:

- **Layer 1, the fluent object model** (namespace `ferroobjects`): constructors, setters that
  take the handle and return it so that they chain with the pipe, getters that return a
  `Result`, and event helpers that send a tag to the one dispatcher of the application.
- **Layer 2, frames** (namespace `ferroviews`): an Elm-like loop. `view(model)` returns
  elements, `update(message, model)` returns the next model, and the library reconciles each
  new frame into the windows.

Both share the namespace `ferroui`: the `extern fn` declarations, the ID constants, the
enumerations as unions, the fingerprint check and the start of an application. This is the
MVP of the Osprey bindings (#950): it works end to end and is extended later (section 9).

## 1. The files and the selection of an application

An application builds against **its selection**: the generated files of the classes it names
in a manifest, `ferroui.json` beside its `osprey.toml`, and of their bases:

```json
{
  "layers": ["objects"],
  "classes": ["Window", "StackPanel", "TextBlock", "Button"]
}
```

`layers` is `objects` (layer 1), `views` (layer 2) or both. The selection has the
constructors or elements of the named classes; the setters, getters and attributes of every
property of those classes and their bases (`Width` of `Layoutable` for a `Button`); their
events; the enumerations those properties use; and the ID constants of all of it. Names are
the names of the full bindings, so adding a class to the manifest changes no code.

The selection also declares the entries of the classes it names (`ferro_type_button()`) and
`registerTypes()`, which calls them; `run` calls it before the main loop, and a program that
starts with `ferro_init` calls it itself. A library built by use knows only the classes
registered this way and keeps only those when it is linked statically (docs/c-abi.md,
section 16). The full bindings register nothing: the full library knows every class.

A library built for the application from the same manifest (`scripts/capi/app_library.py`,
docs/c-abi.md, section 17) exports only what the selection uses and returns the fingerprint of
the selection; `checkApi()` compares it with `selectionFingerprint()` of `abi.osp` and accepts that
library or the full one (which returns `0`). Build the application against it with
`LIBRARY_PATH=<its directory>` and ship it beside the executable.

| File | Namespace | What |
| --- | --- | --- |
| `<application>/ferroui/abi.osp` | `ferroui` | generated: the extern declarations, the ID constants (`buttonTypeId()`, `textPropertyId()`, `clickEventId()`), the fingerprint, the enumerations |
| `<application>/ferroui/objects.osp` | `ferroobjects` | generated: layer 1 |
| `<application>/ferroui/views.osp` | `ferroviews` | generated: layer 2, elements and attributes |
| `bindings/osprey/runtime/core.osp` | `ferroui` | hand-written: `failure()`, `checkApi()`, `add`, `children`, `run`, the one trampoline |
| `bindings/osprey/runtime/runtime.osp` | `ferroviews` | hand-written: `Element`, `Attr`, writing a frame, `render`, `messageAt`, `eventText` |
| `bindings/osprey/full/` | | generated: the same three files for every class of the model, for reference |
| `bindings/osprey/overrides.json` | | names the generator uses instead of its own |

The `osprey.toml` of an application lists its selection and the runtime as source roots:

```toml
[project]
source_roots = ["src", "ferroui", "../../../../bindings/osprey/runtime"]
entry = "src/main.osp"

[modules]
allow_wildcard_imports = true
```

Generate the bindings again after a change of the framework, the ABI or a manifest:

```sh
python3 scripts/generate_api_model.py
python3 scripts/generate_osprey_bindings.py                       # full, and every sample
python3 scripts/generate_osprey_bindings.py --select app/ferroui.json   # one application
```

CI fails when they are stale, and `scripts/test_generate_bindings.py` fails when the selection
of a sample grows past 200 KB of Osprey. The full bindings are about 1.1 MB (507 classes, 1042
properties, 96 events); a selection of four controls is 60 to 130 KB, because the bases of a
control (`Control`, `InputElement`, `Layoutable`, `Visual`) bring their properties.

Generated functions carry no type annotations that inference derives: the signature is the
first line of the comment above each one (`// fontSize(object: int, value: float) -> int:
sets FontSize`). That is the style the lints of Osprey ask for, and it keeps the build fast:
the check of redundant annotations of Osprey 0.29 is super-linear in the number of annotated
functions of a file (section 5).

## 2. Naming

Osprey conventions: camelCase functions, PascalCase types and union variants.

- A class `StackPanel` is `stackPanel()` (layer 1) and `stackPanel(attrs, children)` (layer 2).
- A property `FontSize` is `fontSize(object, 16.0)` (layer 1, a setter), `getFontSize(object)`
  (a getter) and `fontSize(16.0)` (layer 2, an attribute). When declarations of one name have
  different value types, the most general one has the plain name and the others a suffix of
  their kind (`height(object, 200.0)` for `Layoutable.Height`, `heightText(object, "Auto")`
  for `RowDefinition.Height`). An object value has an `Object` suffix
  (`contentObject(button, image)`).
- An event `Click` is `onClick(object, tag)` (layer 1) and `onClick(message)` (layer 2).
- An enumeration is a union whose variants carry its name, because Osprey variants are
  global: `orientation(OrientationHorizontal)`, `verticalAlignment(VerticalAlignmentTop)`.
- Keywords and built-ins get a suffix (`Of` for a property, `Element` for a class).

Arguments are positional in pipes (`button() |> content("Click")`): Osprey fills the first
parameter from the pipe only when the other arguments are positional.

## 3. Layer 1: the fluent object model

```osprey
import ferroui::*
import ferroobjects::*

fn clickedTag() -> int = 1

fn build() -> int = {
    let button = button() |> content("Click me") |> fontSize(16.0) |> onClick(clickedTag())
    let panel = stackPanel() |> spacing(12.0) |> margin("24")
        |> add(textBlock() |> text("Hello, World!"))
        |> add(button)
    let _ = window() |> title("Hello from Osprey") |> width(400.0) |> height(240.0)
        |> add(panel) |> showWindow
    report("build", failure())
    0
}

fn dispatch(tag: int, sender: int, arg: int) -> int = match tag {
    0 => build()
    _ => {
        let _ = sender |> content("Clicked from Osprey")
        0
    }
}

fn main() = {
    let rc = run(ferro_frame_dispatcher(function: dispatch))
    print("the application returned ${rc}")
}
```

- **One dispatcher.** Osprey passes a callback to C only as a named function at the call
  site, so an application has one, `dispatch(tag, sender, arg)`, and gives it to the library
  in `main`. `run` checks the library and runs the main loop; the tag 0 is the start. Every
  event helper (`onClick(object, tag)`) subscribes the dispatcher itself with the tag
  (`ferro_subscribe_dispatch`): no trampoline of the bindings runs between the library and
  `dispatch`.
- **Errors as values.** A setter records a failure in the library instead of returning a
  status; `failure()` returns `Error` with the message of the last one and clears it, so a
  chain is checked once. A getter returns the plain value (`getFontSize(text)` is a
  `float`), the fallback of its kind (`0`, `0.0`, `false`, `""`) when the read fails, and the
  failure is reported by the next `failure()` like that of a setter: no `Result` is allocated
  per read.
- **Children.** `add(parent, child)` returns the parent: children of a panel, the content of a
  window or a button, the child of a border.
- **Handles** are plain integers. The framework holds the tree; the bindings do not release
  the handles they return (section 9).

## 4. Layer 2: frames

```osprey
import ferroui::*
import ferroviews::*

type Message = Increment

type Model = { count: int }

fn update(message: Message, model: Model) -> Model = match message {
    Increment => Model { count: model.count + 1 }
}

fn view(model: Model) -> Element<Message> =
    window([title("Counter"), width(320.0), height(110.0)], [
        stackPanel([orientation(OrientationHorizontal), spacing(8.0), margin("16")], [
            textBox([key(1), text(toString(model.count)), isReadOnly(true), width(80.0)]),
            button([key(2), content("Count"), onClick(Increment)], [])
        ])
    ])

effect Store { put: fn(Model) -> Unit }

mut model = Model { count: 0 }

fn keep(next: Model) -> Unit = {
    handle Store { put value => { model = value } }
    perform Store.put(next)
}

fn dispatch(tag: int, sender: int, arg: int) -> int = {
    handle Arith {
        overflow op lhs rhs wrapped => wrapped
        divideByZero op lhs => 0.0
        remainderByZero lhs => 0
    }
    let next = match messageAt(view(model), tag) {
        Success { value } => update(value, model)
        Error { message } => model
    }
    keep(next)
    report("render", render(view(next)))
    0
}

fn main() = {
    let rc = run(ferro_frame_dispatcher(function: dispatch))
    print("the application returned ${rc}")
}
```

- **Elements** are `name(attrs, children)` for classes that hold children or a content, and
  `name(attrs)` for the others. **Attributes** are property values (`spacing(8.0)`), a key
  (`key(1)`) and events with their message (`onClick(Increment)`).
- **The loop.** `render(view)` writes the elements as a frame (`ferro_frame_*`) and commits it
  under the root `0`: the window is shown the first time and patched afterwards. Every event
  attribute gets a tag, its place among the event attributes of the tree; when an event fires
  the library calls `dispatch` with it, and `messageAt(view(model), tag)` finds its message in
  the view of the current model. The tag 0 of the start has no message: the first frame.
- **Messages, not callbacks.** Controls carry messages. A value event (`onTextChanged`) also
  carries a plain message; `update` reads the value with `eventText()` (the text of the text
  box), as the temperature converter does.
- **The model** lives in a file-scope cell that only the arm of an effect handler assigns: the
  rule of Osprey for mutable state. The loop is written in the dispatcher because Osprey 0.29
  does not call a function received as a value whose effects it cannot track (`update` uses
  arithmetic).
- **Keys** keep an element's control across frames (`key(n)`); without one an element is
  matched by its position. What the user typed is not written back, and what a commit writes
  does not come back as a message, so two text boxes that follow each other do not echo.
- **Memo elements.** `memo(version)` on an element with a key says that its subtree is what
  `version` says (a revision of the data it shows): while the key and the version stay, the
  writer asks the library (`ferro_frame_reuse`) and writes one reuse record in place of the
  subtree, and the library does not look into it. Give a new version whenever anything in
  the subtree changes (`textBlock([key(n + 1), memo(item.revision), text(item.label)])`).
  The view is still built, but not written: what a frame costs is what changed.

## 5. Building and running the samples

`samples/CApi/osprey-bindings/` has one Osprey project per sample, each with its manifest
(`ferroui.json`), its selection (`ferroui/`) and an `osprey.toml` that names the selection and
`bindings/osprey/runtime` as source roots and allows wildcard imports.

| Sample | Layer | What it shows |
| --- | --- | --- |
| `hello_world` | 1 | a text and a button; the click changes the button |
| `counter` | 1 | 7GUIs counter with the count in an effect cell |
| `hello_world_view` | 2 | a button whose caption follows the model |
| `counter_view` | 2 | 7GUIs counter |
| `temperature_view` | 2 | 7GUIs temperature converter: two text boxes that follow each other |
| `benchmark` | 1 and 2 | section 6 |

Build the library with the desktop platform first (`cargo build --release -p ferroui-capi`),
then, from a sample's directory, with the Osprey compiler on the `PATH`:

```sh
LIBRARY_PATH=<target>/release osprey build . --memory=arc -o hello_world
DYLD_LIBRARY_PATH=<target>/release ./hello_world            # LD_LIBRARY_PATH on Linux
```

Build times of `osprey build . --memory=arc` with warnings on (2026-11-19, one Apple silicon
Mac, Osprey 0.29.0, the wall time of the whole build and link):

| Sample | Selection | Build |
| --- | --- | ---: |
| `hello_world` | 4 classes, layer 1, 115 KB | 2 s |
| `counter` | 4 classes, layer 1, 132 KB | 2 s |
| `hello_world_view` | 4 classes, layer 2, 63 KB | 1 s |
| `counter_view` | 4 classes, layer 2, 69 KB | 2 s |
| `temperature_view` | 4 classes, layer 2, 71 KB | 1 s |
| `benchmark` | 4 classes, both layers, 124 KB | 2 s |

Against the full bindings, the same hello world took 8.5 s with `--no-warnings` and did not
finish in 90 s with warnings on, when the generated functions still carried type annotations:
the check of redundant annotations of Osprey 0.29 grows faster than linearly with the
annotated functions of a file (`osprey --check` with warnings: 0.3 s for 200 annotated setters,
1.1 s for 400, 8.2 s for 800). Without the annotations the full layer 1 checks in 2.7 s with
warnings on; with a selection a build is the 1 to 2 seconds above.

`FERROUI_SMOKE_EXIT_MS=<n>` closes the windows after `n` milliseconds, and
`FERROUI_SAMPLE_SCRIPT=1` makes each sample drive itself (clicks with `ferro_raise`, typing by
writing a text box) and print the resulting state, the scenarios of the Rust 7GUIs tests:

```
script: after two clicks the count shows '2': ok
script: Celsius 100 gives Fahrenheit: '212': ok
script: Fahrenheit 5 gives Celsius: '-15': ok
```

## 6. Cost

`samples/CApi/osprey-bindings/benchmark` runs the operations of the Rust benchmark of the ABI
(`cargo bench -p ferroui-capi --no-default-features --bench call_overhead`) from Osprey: a
property write and a read with layer 1, an event raised to the one dispatcher, and three
frames of layer 2 (one text block whose text changes; 100 text blocks of which one changes,
built by Osprey every time; the same 100 as memo elements), and building that view alone. It
uses the clock of macOS.

The first run (2026-11-19, one Apple silicon Mac, the median of 9 samples; Osprey with
`--memory=arc` and the selection of the sample against the library in the `release` profile,
the Rust benchmark in the `bench` profile), before the work of docs/c-abi.md, section 13:

| Operation | Osprey | Rust through the ABI | Rust |
| --- | ---: | ---: | ---: |
| write `FontSize` | 45.9 ns | 41.8 ns | 31.1 ns |
| read `FontSize` (a `Result` in Osprey) | 53.7 ns | 8.6 ns | 3.4 ns |
| raise `Click` (Osprey: to the one dispatcher) | 94.4 ns | 68.8 ns | 50.4 ns |
| frame: one text block, its text changes | 1.10 us | 0.61 us | 89.0 ns |
| frame: 100 text blocks, one changes | 74.2 us | 27.7 us | 89.0 ns |

After it (the same machine, the median of three runs, the library in the `release` profile
without the desktop platform; the Rust columns from docs/c-abi.md, section 13):

| Operation | Osprey | Rust through the ABI | Rust |
| --- | ---: | ---: | ---: |
| write `FontSize` | 39.1 ns | 34.8 ns | 28.5 ns |
| read `FontSize` (a plain value) | 10.6 ns | 7.3 ns | 3.4 ns |
| raise `Click` (Osprey: to the one dispatcher) | 58.4 ns | 53.4 ns | 44.3 ns |
| frame: one text block, its text changes | 0.72 us | 0.20 us | 73.6 ns |
| frame: 100 text blocks, one changes | 51.6 us | 3.18 us | 73.6 ns |
| frame: 100 memo text blocks, one changes | 48.3 us | 1.81 us | 73.6 ns |
| building the view of the 100 text blocks alone | 31.4 us | | |

- **A getter** is one call and returns the value (no `ferro_clear_error`, no second call for
  the code, no `Result` allocated per read): 53.7 ns to 10.6 ns, the read of the ABI plus the
  call from Osprey.
- **An event** goes from the library to `dispatch` with no trampoline of the bindings
  (`ferro_subscribe_dispatch`): 94.4 ns to 58.4 ns.
- **A frame** costs the library 3.2 us for 100 elements now, so what is left is Osprey's:
  building the view (31.4 us: the list, the attributes and the strings of 100 elements every
  time) and writing it (about 20 us: 302 calls of the writer, with a C string made from each
  Osprey string). Memo elements save the writing of the 99 unchanged elements but not their
  building and not the walk of their attributes that finds the key and the version (48.3 us);
  they pay off for subtrees larger than one text block, and fully once Osprey can skip
  building a subtree (section 9).

## 7. Signals

Signals are slots of the signal table of the library (docs/c-abi.md, section 14), named by an
`int` like a handle (`core.osp`): `signalsInit(capacity)`, `signalText(text)`,
`signalNumber(value)`, `signalInteger(value)`, `signalBool(value)`; `signalSetText(signal,
text)` and the others write one; `signalReadText(signal)` and the others read one;
`signalVersion(signal)` moves on every change. `signalBind(object, property, signal)` binds a
property to a signal (layer 1, for the pipe), `signalWatch(object, property, signal)` watches a
property into one, and in a view `bound(text(""), signal)` binds the property of a value
attribute. A bound property follows every write at the next apply, which the library makes at
the end of every dispatch and at the start of every frame commit; a watched one is written into
its signal by the library when it changes, so the dispatcher reads it with no call to the
control.

```osprey
fn signalBox(signal: int, tag: int) -> int =
    textBox() |> width(80.0) |> signalBind(textPropertyId(), signal) |> signalWatch(textPropertyId(), signal)
        |> onTextChanged(tag)
```

`samples/CApi/osprey-bindings/temperature_signals` is the temperature converter this way: each
box is bound to a text signal and watched into it, and the dispatcher reads the typed text from
the signal and writes the converted text into the other signal. Osprey cannot read or write
memory, so a read and a write are one small call each: a read of a watched value costs 6.7 ns
against 11.9 ns for the getter of the control, 100 writes and one apply 3.91 us against 3.60 us
for 100 setter calls (section 14.3 of docs/c-abi.md).

## 8. Controls

A control written in Osprey is a class registered with the library (docs/c-abi.md, section 15).
`controlClass(name, base)` registers it (`baseControl()`, `basePanel()`, `baseDecorator()`,
`baseContentControl()`), `numberProperty(classId, name, default, flags)` and `textProperty`
register its properties (`affectsMeasure()`, `affectsArrange()`, `affectsRender()`), and the
callbacks are named functions given to `ferro_class_on_measure`, `ferro_class_on_render`, ... at
the call site, the rule of Osprey for callbacks. A callback receives a data pointer (unused: an
Osprey control keeps its state in its registered properties), the handle of the instance and
scalars, and returns a status; a measure or arrange callback gives its size with
`layoutResult(width, height)`, and a render callback draws with the `ferro_draw_*` functions in
the size `ferro_layout_bounds_width` and `_height` give.

```osprey
fn measure(data: Ptr, self: int, width: float, height: float, outWidth: Ptr, outHeight: Ptr) -> int32 =
    layoutResult(smaller(width, 160.0), smaller(height, 16.0))

fn registerGauge() -> Unit = {
    let classId = controlClass("OspreyGauge", baseControl())
    let level = numberProperty(classId, "Level", 0.5, affectsRender())
    let _ = ferro_class_on_measure(classId: classId, callback: measure)
    let _ = ferro_class_on_render(classId: classId, callback: render)
    keepGauge(Gauge { gaugeClass: classId, gaugeLevel: level })
}
```

`samples/CApi/osprey-bindings/gauge_control` is the whole control (measure, arrange, render and
a pointer press that sets the level), the twin of the C control in `samples/CApi/gauge.h`.

## 9. Limits of the MVP

- **Handles are not released** by layer 1: a control removed from its parent lives while its
  handle does. Layer 2 releases what it removes.
- **Scalars only.** Properties whose values are callbacks, collections or templates are left
  out; object values are layer 1 only (`...Object` setters), and frames place elements as
  children, not as values of properties.
- **Event arguments**: the value of a frame event is the value of one property of the sender
  (`Text`, `Value`, `IsChecked`, `SelectedIndex`); pointer and key data are not delivered
  yet.
- **Two layers, two namespaces.** Importing both with wildcards makes their names ambiguous;
  import what one layer needs by name (`import ferroobjects::{fontSize as setFontSize}`).
- **The selection trims the bindings, not yet the library**: the library is still the full
  build (`type-tables`); trimming it with the same manifest is phase 2 of #636.
- **Hand-written files keep their annotations** (`core.osp`, `runtime.osp`), so a build prints
  their redundant-annotation lints (about 80); they are few and do not slow it down.
- **Memo elements are built.** A memo element is skipped by the writer and the library, but
  the view builds it: Osprey 0.29 cannot call a function held in a value whose effects it
  does not track, so a lazy element (a function that builds the subtree only when the version
  changed) is left for a later version of the compiler.
- **CI** checks that the generated files are up to date and runs the Rust and C tests of the
  frame layer; the Osprey samples are built and run locally (the Osprey compiler is not
  installed on the runners yet).
