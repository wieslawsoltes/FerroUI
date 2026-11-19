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
MVP of the Osprey bindings (#950): it works end to end and is extended later (section 7).

## 1. The files

| File | Namespace | What |
| --- | --- | --- |
| `bindings/osprey/ferroui/abi.osp` | `ferroui` | generated: the extern declarations, the ID constants (`buttonTypeId()`, `textPropertyId()`, `clickEventId()`), the fingerprint, the enumerations |
| `bindings/osprey/ferroui/objects.osp` | `ferroobjects` | generated: layer 1 |
| `bindings/osprey/ferroui/views.osp` | `ferroviews` | generated: layer 2, elements and attributes |
| `bindings/osprey/ferroui/core.osp` | `ferroui` | hand-written: `failure()`, `checkApi()`, `add`, `children`, `run`, the one trampoline |
| `bindings/osprey/ferroui/runtime.osp` | `ferroviews` | hand-written: `Element`, `Attr`, writing a frame, `render`, `messageAt`, `eventText` |
| `bindings/osprey/overrides.json` | | names the generator uses instead of its own |

Generate them again after a change of the framework or the ABI:

```sh
python3 scripts/generate_api_model.py
python3 scripts/generate_osprey_bindings.py
```

CI fails when they are stale. The generated files are large (about 1 MB of Osprey for all 507
classes, 1042 properties and 96 events of FerroUI.Base and FerroUI.Controls); a program links
only what it calls (the layer 1 hello world is a 98 KB executable on macOS), but the compiler
reads all of it: a sample builds in about 10 to 16 seconds. A selection of what an
application uses is phase 2 (#636).

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
  the handles they return (section 7).

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

`samples/CApi/osprey-bindings/` has one Osprey project per sample: its `osprey.toml` names
`bindings/osprey` as a source root and allows wildcard imports.

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
built by Osprey every time; the same 100 as memo elements). It uses the clock of macOS.

The first run (2026-11-19, one Apple silicon Mac, the median of 9 samples; Osprey with
`--memory=arc` against the library in the `release` profile, the Rust benchmark in the
`bench` profile), before the work of docs/c-abi.md, section 13:

| Operation | Osprey | Rust through the ABI | Rust |
| --- | ---: | ---: | ---: |
| write `FontSize` | 42.6 ns | 41.8 ns | 31.1 ns |
| read `FontSize` (a `Result` in Osprey) | 52.7 ns | 8.6 ns | 3.4 ns |
| raise `Click` (Osprey: to the one dispatcher) | 102.9 ns | 68.8 ns | 50.4 ns |
| frame: one text block, its text changes | 1.45 us | 0.61 us | 89.0 ns |
| frame: 100 text blocks, one changes | 94.4 us | 27.7 us | 89.0 ns |

After it, the library side (Rust through the ABI, the same machine, docs/c-abi.md, section
13): a read 7.3 ns, the frame dispatcher 5.0 ns (27.0 ns before), a frame of one text block
204 ns, a frame of 100 text blocks of which one changes 3.18 us, the same as memo elements
1.81 us. What changed for Osprey besides the library:

- A getter is one call and returns the value (no `ferro_clear_error`, no second call for the
  code, no `Result` allocated per read): what was 52.7 ns is now the read of the ABI plus the
  call from Osprey.
- An event helper subscribes the dispatcher itself (`ferro_subscribe_dispatch`): the
  trampoline `dispatchEvent` and the call of `ferro_frame_dispatch` are gone from every event.
- A memo element (`memo(version)`) is written as one reuse record while its version stays, so
  a frame of 100 memo text blocks of which one changes writes one element in full.

The Osprey column after the work is not measured yet: the Osprey compiler does not finish
the bindings generated from the model after the merge of main into #968 (a hello world took
over 180 s where it took 10 to 16 s), which #968 fixes; the table is measured again then.

## 7. Limits of the MVP

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
- **Every class is generated.** A selection manifest that trims the bindings and the library
  together is phase 2.
- **Memo elements are built.** A memo element is skipped by the writer and the library, but
  the view builds it: Osprey 0.29 cannot call a function held in a value whose effects it
  does not track, so a lazy element (a function that builds the subtree only when the version
  changed) is left for a later version of the compiler.
- **CI** checks that the generated files are up to date and runs the Rust and C tests of the
  frame layer; the Osprey samples are built and run locally (the Osprey compiler is not
  installed on the runners yet).
