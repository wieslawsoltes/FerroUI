# The C ABI

FerroUI has one stable C ABI that every language can call: the crate `ferroui-capi`
(`src/FerroUI.CApi`) exports `extern "C"` functions over opaque integer handles, and the C
header `src/FerroUI.CApi/include/ferroui.h` is the contract every binding is written
against, the way Skia, SQLite and wgpu-native are consumed. This document describes phase 1
of #636, with the changes #950 proposed for languages that have neither pointers nor
closures in their FFI (Osprey, WebAssembly hosts, Dart, Go, Python).

For application and binding authors, docs/c-abi-languages.md is the guide: how to call the ABI
from each language and which path is the fastest for each kind of work.

Phase 1 is the "full" build of #636: the tables of types are on, so any registered type,
property and routed event can be found by name, and there is no markup. The by-use mode links
only the classes a program registers with their entries, and the units of the theme they need
(section 16); a library built for one application is phase 2 (section 11).

## 1. Building the library

```sh
cargo build -p ferroui-capi            # with the desktop platform (Skia, the Fluent theme)
cargo build -p ferroui-capi --no-default-features --features type-tables
                                       # without: no renderer, no ferro_app_run
cargo build -p ferroui-capi --no-default-features --features desktop
                                       # by use: no tables of types (section 16)
```

The crate is built as a dynamic library (`libferroui_capi.dylib`, `.so`, `ferroui_capi.dll`),
a static library (`libferroui_capi.a`, `ferroui_capi.lib`) and a Rust library. A C program
includes `ferroui.h` and links one of the first two; `samples/CApi/Makefile` does it with the
dynamic library:

```sh
make -C samples/CApi run     # the hello world (section 6)
make -C samples/CApi test    # the C tests of the ABI (section 9)
```

The feature `desktop` (on by default) is what `ferro_app_run` needs: `ferroui-desktop`, the
platform of the operating system with Skia and HarfBuzz, and the Fluent theme. Without it
everything else works on a thread the host drives (`ferro_init`, `ferro_run_jobs`), which is
how the tests and the benchmark run.

## 2. Handles and lifetime

- A `FerroHandle` is a `uint64_t`: the index of a slot of the handle table of the UI thread
  plus one in its low 32 bits, and the generation of the slot in its high 32 bits. `0` is no
  object.
- A handle that was released, or never issued, is a checked error (`FERRO_BAD_HANDLE`), never
  a dangling pointer: a freed slot comes back with the next generation. A language can keep
  a handle as a plain integer (Osprey cannot keep a pointer in its state; #950).
- A handle counts references. `ferro_create` returns a handle with one reference,
  `ferro_retain` adds one and `ferro_release` gives one back; the last release frees the
  handle. A getter that returns an object (`ferro_get_handle`) counts one reference that the
  caller releases.
- One object has one handle at a time: asking for the handle of an object that has one gives
  that handle again (with one more reference).
- The handle table holds a strong reference to its objects. Beyond that the framework owns
  its tree: a panel holds its children, a content holds its control, and an open window shown
  with `ferro_show` is held until it closes. So a binding releases its handles once the tree
  is built, and nothing depends on a finalizer (Osprey has none; #950).

## 3. IDs and strings

| Function | Finds | Notes |
| --- | --- | --- |
| `ferro_type_id(name, len)` | a class in the tables of types | `Button`, or `FerroUI.Controls.Button` when two namespaces have the name |
| `ferro_property_id(name, len)` | an interned property name | `Text`; `Grid.Row` names the property a class owns (attached properties included) |
| `ferro_event_id(name, len)` | an interned event name | `Click`; `Button.Click` names the routed event a class owns |

- A string is UTF-8 with a length in bytes, borrowed for the duration of the call. **A
  negative length means a NUL-terminated string** (the SQLite convention): what a language
  whose strings are C strings passes. `NULL` with a length of `0` is the empty string.
- IDs are positive. A lookup that fails returns the negative status (`FERRO_NOT_FOUND`,
  `FERRO_INVALID_UTF8`, `FERRO_INVALID_ARGUMENT`).
- A property or event ID is the name, not a property of one class: one ID serves every class
  that has a property of the name (`Text` of a `TextBlock` and of a `TextBox`). It is checked
  against a class the first time it is used with an object of that class
  (`FERRO_NOT_FOUND`), and what it stands for is then kept per thread and class: later writes
  are an index into a table and a short scan, with no string and no allocation.
- The lookups work on any thread.
- **Stable IDs.** The library starts its tables with the names of the API model of the
  bindings (`bindings/model/ferroui-api.json`, compiled in as `api_ids.rs`), in the order of
  the model: the ID of a name of the model is the constant the model gives it, and generated
  bindings pass constants instead of looking names up. `ferro_api_fingerprint()` returns the
  fingerprint of those tables; a binding compares it with its own at start (section 12).

## 4. Values

One setter and one getter per kind of scalar cover every property, with no function per
control:

| Setter | Getter | Writes |
| --- | --- | --- |
| `ferro_set_str` | `ferro_get_str`, `ferro_get_cstr` | text; the member of that name of an enumeration (any case); a value parsed from text (`"4,2"` for a thickness); text as the content of a content property |
| `ferro_set_f64` | `ferro_get_f64` | a number, converted to the numeric type of the property |
| `ferro_set_i64` | `ferro_get_i64` | an integer; the member with that value of an enumeration (the bits of a set of flags) |
| `ferro_set_bool` | `ferro_get_bool` | a boolean |
| `ferro_set_handle` | `ferro_get_handle` | an object (`Content`, `Child`); `0` is null |
| `ferro_clear` | | clears the local value |

- A scalar written to a property of its own type (a `double` `FontSize`, a text `Text`, a
  `bool` `IsEnabled`, an `int32_t` `Grid.Row`) goes to the typed setter of the property: no
  value is boxed, and nothing is allocated beyond what the property stores (the string of a
  text). Any other type takes the conversions of markup.
- Every write is checked before it is made: the thread, the handle, the property on the class,
  read-only (`FERRO_READ_ONLY`), the type (`FERRO_TYPE_MISMATCH`) and the validation of the
  property (`FERRO_INVALID_VALUE`, a negative `Grid.Row`). That is what keeps the profiles
  that abort on a panic from aborting on a bad value.
- `ferro_get_str(object, property, buffer, capacity, &length)` writes the text and a NUL when
  they fit, and the length in bytes (without the NUL) in any case; when they do not fit it
  writes nothing and returns `FERRO_BUFFER_TOO_SMALL`. A `NULL` buffer with a capacity of `0`
  asks for the length.
- `ferro_get_cstr(object, property)` returns a NUL-terminated string the library owns, valid
  until the next `ferro_get_cstr` on the thread: for a language that cannot pass a buffer.
- For a language that cannot pass a pointer to a result, `ferro_get_f64_or`,
  `ferro_get_i64_or` and `ferro_get_bool_or` return the value, or the fallback given when the
  read fails (recorded as every failure), and `ferro_get_cstr_or` returns the caller's fallback
  string in place of `NULL`.
- `ferro_set_str16(object, property, text, units)` writes UTF-16 text (C#, Java, JavaScript),
  as `ferro_set_str` writes UTF-8.

Children go where the parent holds them, as the builders of `ferroui-view` place them:
`ferro_add_child(parent, child)` adds to the `Children` of a panel or the `Items` of an items
control, or sets the one `Content` of a content control (a window, a button), the content of
a content presenter, or the `Child` of a decorator or a popup (`FERRO_INVALID_STATE` when it
already holds one). `ferro_remove_child` takes it out again.

## 5. Errors, threads and callbacks

**Errors.** Every function returns a `FerroStatus` (`FERRO_OK` is `0`, errors are negative), a
positive ID or token, or a handle (`0` on a failure). `ferro_last_error()` is the message of
the last call that failed on the calling thread and `ferro_last_error_code()` its code; a
successful call leaves both as they are (as `errno`). The message stays valid until the next
failure on the thread. `ferro_clear_error()` forgets it, for a binding that reports errors as
values (a getter with a fallback is checked by clearing before and reading the code after).

**Nothing unwinds across the boundary.** Every exported function catches a panic and returns
`FERRO_PANIC` with the panic message; the shipped profiles (`release`, `dist`) abort on a panic
instead, which is why the checks of section 4 come first.

**Threads.** UI objects belong to the UI thread: the thread that called `ferro_init`, or
`ferro_app_run`, which calls it (on macOS the main thread of the process).

- A UI call from another thread returns `FERRO_WRONG_THREAD`; a UI call before any thread is
  the UI thread returns `FERRO_NOT_INITIALIZED`.
- `ferro_post(function, token)` and `ferro_post_data(function, user_data)` run work on the UI
  thread from any thread, after the work already queued there.
- `ferro_run_jobs()` runs what is queued, for a host that drives the UI thread itself.
- `ferro_is_ui_thread()` tells a binding where it is.
- The ID lookups, `ferro_abi_version`, `ferro_last_error` and the posts work on any thread.

**Callbacks.** Two forms, for every event and for posted work:

```c
typedef int64_t (*FerroEventFn)(int64_t token, FerroHandle sender, int64_t arg);
typedef int64_t (*FerroEventDataFn)(void *user_data, FerroHandle sender, int64_t arg);
typedef void (*FerroReleaseFn)(void *user_data);

int64_t ferro_subscribe(FerroHandle object, int64_t event, FerroEventFn function, int64_t token);
int64_t ferro_subscribe_data(FerroHandle object, int64_t event, FerroEventDataFn function,
                             void *user_data, FerroReleaseFn release);
FerroStatus ferro_unsubscribe(int64_t subscription);
```

- The token form is what every language can make: a named function and an integer the
  binding dispatches on (Osprey has no closures to pass and no meaningful `void *`; #950).
  Callbacks return `int64_t`, never `void`.
- The `void *` form carries a release function, called once the handler is gone: removed by
  `ferro_unsubscribe`, dropped with its object, or at once when the subscription fails.
- Handlers are called on the UI thread. The sender is the object the handler is attached to,
  its handle borrowed for the call (`ferro_retain` it to keep it). `arg` is `0` for routed
  events in ABI version 1. For a routed event a result other than `0` marks the event as
  handled, so the handlers further along the route do not see it.
- A handler may call the ABI (set a property, add a child, subscribe): the library holds no
  borrow of its own state while it calls out.
- `ferro_raise(object, event)` raises a routed event with plain arguments, as the control
  does (`Click`), for tests and for driving an interface from code.
- Nothing is allocated per event: the subscription remembers the handle of its sender, and a
  dispatch is a check of that handle and one indirect call.

**Timers.** `ferro_timer_start(interval, function, token)` calls `function(token, elapsed)` on
the UI thread every `interval` milliseconds, with the microseconds since the previous tick (or
the start) as `elapsed`, through the `DispatcherTimer` of the framework; the function returns `0`
to go on and any other value to stop. It returns a positive timer ID, and `ferro_timer_stop(id)`
stops the timer (also from inside its own tick). A timer that stopped itself is gone after its
last tick (`FERRO_NOT_FOUND`).

```c
typedef int64_t (*FerroTimerFn)(int64_t token, int64_t elapsed);
int64_t ferro_timer_start(int64_t interval, FerroTimerFn function, int64_t token);
FerroStatus ferro_timer_stop(int64_t timer);
```

**The data of the event being handled.** A handler receives scalars only. What a routed event
carries beyond the routing state is read from inside its handler, while it runs:

| Function | Returns | For |
| --- | --- | --- |
| `ferro_event_x(relative_to)`, `ferro_event_y(relative_to)` | the position of the pointer relative to a visual (`0`: the top level), NaN on a failure | pointer events, `Tapped`, `DoubleTapped` |
| `ferro_event_button()` | the button a press or a release is about: `1` left, `2` right, `3` middle, `4`, `5` the extra buttons, `0` for a move | pointer events |
| `ferro_event_key()` | the name of the member of `Key` (`Return`, which is also named `Enter`; `Escape`), owned by the library until the next call | `KeyDown`, `KeyUp` |

Outside a handler they fail with `FERRO_INVALID_STATE`, and for an event without the data with
`FERRO_NOT_SUPPORTED`. An event raised inside a handler is the current one while its own
handlers run. `ferro_focus(object)` moves the keyboard focus to an input element
(`FERRO_INVALID_STATE` when it cannot take it now, for instance before it is in a shown window:
posting the call with `ferro_post` focuses it once it is).

These are the calls the Osprey samples (section 6) needed beyond the core: each is general,
none is specific to a language.

**Application.** `ferro_app_run(options, on_start, token)` makes the thread the UI thread,
sets up the platform of the operating system with the Fluent theme, calls `on_start(token)`,
and runs the main loop until the last window shown with `ferro_show` closes or `ferro_quit`
is called. `on_start` returns `0` to run the loop; any other value ends the call with
`FERRO_START_FAILED`. `options` is reserved and must be `NULL` in ABI version 1 (a later
version adds a structure whose first field is its size). With `FERROUI_SMOKE_EXIT_MS=<n>` the
windows close after `n` milliseconds, as for every sample.

**Versioning.** `ferro_abi_version()` returns `FERRO_ABI_VERSION`, which a binding checks
before its first call. Additions keep the version; a change that breaks a binding raises it.

## 6. Hello, World! in C

`samples/CApi/hello_world.c` is the hello world of the declarative samples
(`samples/HelloWorld.Builders`) in C: a window with a text and a button whose clicks change
its text.

```c
static int64_t content;
static int64_t clicks;

static int64_t on_click(int64_t token, FerroHandle sender, int64_t arg) {
    char text[64];
    clicks += 1;
    snprintf(text, sizeof text, "Clicked %lld times", (long long)clicks);
    ferro_set_str(sender, content, text, -1);
    return 0;
}

static int64_t on_start(int64_t token) {
    content = ferro_property_id("Content", -1);
    FerroHandle window = ferro_create(ferro_type_id("Window", -1));
    ferro_set_str(window, ferro_property_id("Title", -1), "Hello, World!", -1);
    FerroHandle panel = ferro_create(ferro_type_id("StackPanel", -1));
    FerroHandle button = ferro_create(ferro_type_id("Button", -1));
    ferro_set_str(button, content, "Click me", -1);
    ferro_subscribe(button, ferro_event_id("Click", -1), on_click, 0);
    ferro_add_child(panel, button);
    ferro_add_child(window, panel);
    ferro_show(window);
    ferro_release(button);
    ferro_release(panel);
    ferro_release(window);
    return 0;
}

int main(void) {
    return ferro_app_run(NULL, on_start, 0) == FERRO_OK ? 0 : 1;
}
```

The sample checks every status; the workflow `capi.yml` builds it and runs it for three
seconds on macOS and under Xvfb on Linux.

`samples/CApi/osprey` has the same hello world in Osprey, and the seven tasks of 7GUIs (Counter,
Temperature Converter, Flight Booker, Timer, CRUD, Circle Drawer, Cells) as Osprey programs over
the raw ABI: `extern fn` declarations, integer handles, named functions with tokens as
callbacks, state in file-scope cells that effect handlers assign. Its README says how to build
and run them and what each one does through the ABI.

The functions the hello world needs are the subset #950 proposed for milestone 1 (with
`ferro_init` and `ferro_last_error_code` beside them), and they are the core every other
binding needs: typed per-control calls can be generated on top of `ferro_create` and the
setters.

## 7. The header

`include/ferroui.h` is generated by cbindgen from the sources of the crate (the configuration
is `src/FerroUI.CApi/cbindgen.toml`), never written by hand. cbindgen is a dev-dependency of
the crate, so an application that links the library does not build it. The unit test
`tests/header_tests.rs` writes the header from the sources and fails when the file differs;
after a change of the ABI it is written again with

```sh
FERROUI_CAPI_WRITE_HEADER=1 cargo test -p ferroui-capi --lib header
```

The workflow `capi.yml` runs the test on every pull request that changes the crate. cbindgen
rather than the MicroCom IDL toolchain of the native backend: that toolchain describes COM
interfaces (vtables), while this ABI is a flat list of functions, which the Rust sources state
directly.

## 8. Call overhead

The cost of a call through the ABI against the same operation in Rust is measured by the
benchmark of the crate (`cargo bench -p ferroui-capi --no-default-features --bench
call_overhead`): a property write (`ferro_set_f64` of `FontSize` and `ferro_set_str` of
`Text` against the typed setters) and an event dispatch (`ferro_raise` of `Click` to a handler
subscribed with `ferro_subscribe` against a Rust handler of the same event). The workflow
`capi.yml` runs it and prints the numbers. The first run (2026-11-19, a GitHub `ubuntu-latest`
runner, the `bench` profile, 100 000 operations per sample, the median of 9 samples):

| Operation | Rust | ABI | Overhead |
| --- | ---: | ---: | ---: |
| write `FontSize` (`ferro_set_f64`) | 37.8 ns | 49.5 ns | 11.7 ns |
| write `Text`, NUL-terminated (`ferro_set_str`, length `-1`) | 54.2 ns | 74.6 ns | 20.4 ns |
| write `Text` with a length (`ferro_set_str`) | 54.2 ns | 71.8 ns | 17.6 ns |
| read `FontSize` (`ferro_get_f64`) | 3.3 ns | 21.8 ns | 18.5 ns |
| dispatch `Click` to one handler (C function and token) | 43.4 ns | 52.3 ns | 8.9 ns |
| raise `Click` (`ferro_raise`, the whole call) | 43.4 ns | 66.6 ns | 23.3 ns |

The overhead of a write is the check of the thread, the lookup of the handle, the cached
resolution of the property ID for the class, the validation of the value and the boundary
that catches a panic; the string forms add the length (`strlen`) and the check of UTF-8. A
dispatch adds the check of the remembered sender handle and one indirect call. A read in Rust
is an inlined field access, so its overhead is the whole call. A shared runner is not a quiet
machine: the numbers say how large the overhead is, not its exact value.

## 9. Tests

- The unit tests of the crate (`src/FerroUI.CApi/tests`) call the ABI as a binding does, every
  UI call on one test UI thread: the version, ID lookups (by length, NUL-terminated, `NULL`,
  invalid UTF-8, qualified names, any thread), handle counting and stale handles, the wrong
  thread for every kind of UI call, posted work, every setter and getter, the buffer of
  `ferro_get_str`, read-only and invalid values, children, events with tokens and with user
  data, bubbling and handled events, calls into the ABI from a handler, and the header.
- The tests of the getters with a fallback, the timers, the data of the event being handled
  (a pointer press and a key press raised with their arguments, a click without them) and the
  focus are in `tests/osprey_tests.rs`.
- `samples/CApi/abi_test.c` tests the same rules from C, with a POSIX thread for the thread
  rule, without a window or a display.

## 10. What the ABI is not (yet)

- No markup: controls are created and composed through the calls above. XAML through the C
  API is step 7 of #636.
- No typed per-control functions in C: bindings generate them from the API model (section 12;
  Osprey first), and a selection trims them in phase 2.
- No batch of writes (`ferro_set_values`): each scalar write is one direct call, which also
  serves languages without arrays across their FFI (#950); a frame is the batch (section 13).
- Mobile and the browser start through their hosts, not `ferro_app_run`.

## 11. Phase 2 and later

- **Selection and generation (#636, step 2):** a manifest of the controls, properties, events
  and features an application uses (written by hand or collected from its sources), and a
  generator of a per-application library from it (#977): only those exports, the Cargo features
  that go with them, the class entries and theme units of section 16, `type-tables` off.
- **Bindings:** a header-only C++ wrapper with RAII handles and typed accessors generated from
  the selection; Go (cgo, `runtime.LockOSThread` for the UI thread); Dart and Flutter
  (`dart:ffi`, `ffigen`); C#, Python, Swift and Zig as demand shows.
- **Osprey (#950):** the generated bindings of section 12 (shape 1: an Osprey executable over
  the dynamic library with named-function callbacks); FerroUI owning the process with Osprey as
  a library is shape 2.
- **Values:** a `FerroValue` tagged union and `ferro_set_values` beside the scalar family, and
  event-specific arguments (`arg`) for pointer and key events.

## 12. Frames and generated bindings

Bindings are generated in two stages (bindings/README.md): a language-neutral API model
(`scripts/generate_api_model.py`) and a backend per language that reads it (Osprey:
`scripts/generate_osprey_bindings.py`, docs/osprey.md). The design they follow, for the
fastest FFI path and the idiom of each language, is docs/bindings-design.md.

For Elm-like and React-like bindings the ABI has **frames**: an interface described again on
every update and reconciled by the library into the retained tree of a root.

| Function | What it does |
| --- | --- |
| `ferro_frame_begin()` | empties the frame of the calling thread |
| `ferro_frame_open(type, key)`, `ferro_frame_close()` | an element of a class; `key` keeps it across frames (`0`: by position) |
| `ferro_frame_key_str(text, length)` | a text key for the open element |
| `ferro_frame_str`, `_str16`, `_f64`, `_i64`, `_bool` | a property value of the open element, as the setters of section 4 write it |
| `ferro_frame_on(event, tag, value_property)` | binds an event to a message tag; `value_property` is read as the value of the event |
| `ferro_frame_memo(version, tag_base)` | makes the open element a memo element: not looked into while its key and version stay |
| `ferro_frame_reuse(root, key, version, tag_base)` | writes one reuse record for the memo element of the last frame under `root`; the number of its tags, or a negative status (write it in full) |
| `ferro_frame_commit(root)` | reconciles the frame into the children of `root` (`0`: windows, shown and closed) |
| `ferro_frame_commit_bytes(root, data, length)` | the same for a buffer the caller wrote in the layout of the model |
| `ferro_frame_dispatcher(function)` | the one dispatcher of the process (any thread): `function(tag, sender, value)` |
| `ferro_frame_dispatch(tag, sender, arg)` | calls the dispatcher (the start of a loop, a test) |
| `ferro_subscribe_dispatch(object, event, tag)` | subscribes the dispatcher itself to an event of an object, with a tag |
| `ferro_frame_event_text()`, `ferro_frame_event_f64()` | the value of the event being dispatched |
| `ferro_frame_find(root, key)` | the handle of a keyed element of the retained tree |
| `ferro_frame_clear(root)` | takes the frames of `root` out and frees them |

The writer functions append records to a buffer of the calling thread (any thread); the
records are the byte layout of the model (`frame`), so a language that writes memory commits
a buffer of its own with one call. A commit writes only what changed, never writes back a
text the object already shows, binds events once and changes their tags in place, and does
not dispatch what it caused itself. What a frame costs, and memo elements, are in section 13.
The tests are `src/FerroUI.CApi/tests/frame_tests.rs` and
the frame part of `samples/CApi/abi_test.c`.


## 13. Performance

What a call through the ABI costs, what was made cheaper and what was left out (#968
follow-up). Measured on one Apple silicon Mac (2026-11-19) with the benchmark of the crate
(`cargo bench -p ferroui-capi --no-default-features --bench call_overhead`, the `bench`
profile) and its C twin (`samples/CApi/call_overhead.c`, `make -C samples/CApi bench`): the
median over five runs of the median of 9 samples. The machine was not idle (other builds ran
on other cores); single numbers moved by up to 10 % from run to run, the differences below are
far larger than that except where the text says otherwise.

| Operation | Rust | ABI before | ABI after |
| --- | ---: | ---: | ---: |
| write `FontSize` (`ferro_set_f64`) | 28.5 ns | 36.6 ns | 34.8 ns |
| write `Text` with a length (`ferro_set_str`) | 73.6 ns | 92.9 ns | 101.4 ns |
| read `FontSize` (`ferro_get_f64`) | 3.4 ns | 8.3 ns | 7.3 ns |
| the check of the thread (`ferro_is_ui_thread`) | | 1.4 ns | 0.6 ns |
| the handle table (`ferro_retain` and `ferro_release`) | | 6.7 ns | 4.6 ns |
| reset the error (`ferro_clear_error`) | | 15.9 ns | 0.4 ns |
| raise `Click` (`ferro_raise`, the whole call) | 44.3 ns | 55.2 ns | 53.4 ns |
| call the frame dispatcher (`ferro_frame_dispatch`) | | 27.0 ns | 5.0 ns |
| frame: one text block, its text changes | 73.6 ns | 510 ns | 204 ns |
| frame: 100 text blocks, one changes | 73.6 ns | 21.6 us | 3.18 us |
| frame: 100 text blocks, none changes | | 21.2 us | 2.97 us |
| frame: 100 text blocks, the scalar writer alone | | 2.79 us | 1.26 us |
| frame: 100 text blocks, the commit of a buffer alone | | 18.6 us | 1.89 us |
| frame: 1000 text blocks, one changes | | 211 us | 30.2 us |
| frame: 100 memo text blocks, one changes | | | 1.81 us |
| frame: 1000 memo text blocks, one changes | | | 17.8 us |

The Rust column of the frames is the one `set_text` a frame comes down to. From C (the
dynamic library): a frame of 100 text blocks of which one changes 19.9 us before, 3.2 us
after, 1.72 us as memo elements; the frame dispatcher 28.0 ns before, 4.6 ns after.

### 13.1 Where a call goes

Each line of the benchmark was broken down with a sampling profiler before it was changed:

- **A scalar call** (a write, a read) is the boundary that catches a panic (free unless one
  is raised), the check of the thread, the lookup of the handle, the resolution of the
  property ID for the class, and the typed setter or getter of the framework, which is most
  of a write. A read in Rust is an inlined field access; through the ABI about 4 ns are the
  call itself and its checks.
- **A frame** cost what the whole tree costs, not what changed: the decoder built a tree of
  elements (vectors and a string per element), the reconcile built a hash map per list of
  siblings, and the writer appended each field with a separate copy. about 87 % of the
  100-element frame was the commit, 13 % the writer.
- **A read with a `Result`** (the getters of a binding that reports errors as values) cleared
  the error of the thread first, which allocated a new empty C string every time.
- **An event to a binding with one dispatcher** went through a trampoline of the binding and
  `ferro_frame_dispatch`, which took a mutex and allocated an empty C string for the event text.

### 13.2 What was made cheaper

1. **Frames cost what changed** (`frame.rs`). The frame is indexed in one pass over its
   record headers into a flat list of elements that is kept from one commit to the next (no
   allocation per element); each element of the retained tree keeps the bytes of its
   attribute records, and an element whose bytes did not change costs one short comparison
   of bytes, with no decoding, no lookup and no write. Siblings whose keys and classes are the
   ones of the last frame, in the same order, are matched by position in place, with no table
   (the fast path of the keyed reconcilers of Inferno and ivi); only a reordered list builds the
   keyed map. The text of a value is checked for UTF-8 when it is written: an unchanged value
   was checked when it was first written (a text that is not UTF-8 fails its write like any
   other value; a key that is not UTF-8 still refuses the frame). The commit of a buffer of 100
   elements went from 18.6 us to 1.9 us, about 19 ns per element.
2. **Memo elements and reuse records** (`ferro_frame_memo`, `ferro_frame_reuse`; records 10
   and 11 of the layout). The idea of `Html.Lazy` and `React.memo` with an explicit version in
   place of a comparison of arguments: an element with a memo record is not looked into while
   its key and version stay, and a binding that asks `ferro_frame_reuse` first writes one
   record in place of the whole subtree, and need not build it. The library keeps the bytes of
   every memo element with an integer key, so a reuse record also works for an element that
   moved to another parent, and a new tag base only moves the tags of its events (positional
   tags of an Elm-like binding shift when an earlier element gains an event). From C, a reused
   text block costs about 17 ns, about half of writing and comparing it; the gain grows with
   the size of the subtree. In Osprey, building the view is most of a frame now (section 6 of
   docs/osprey.md), which a memo element does not skip yet.
3. **The writer appends words** (`Vec<u64>`), one store per field, in place of byte copies of
   variable length: 2.79 us to 1.26 us for the 302 writer calls of 100 elements (with step 5).
4. **No allocation on the paths of every call.** The error of the thread is a buffer that is
   cleared, not a new C string (`ferro_clear_error` 15.9 ns to 0.4 ns); `ferro_get_cstr` reads
   a text property in place into a kept buffer; the event text of a frame dispatch is a kept
   buffer; the dispatcher is an atomic pointer, not a mutex (27.0 ns to 5.0 ns).
5. **The UI thread is checked with one comparison** (`thread.rs`). The UI thread is remembered
   as a token that the calling thread reads without a call where the platform allows: the
   thread register on Apple arm64 (`TPIDRRO_EL0`), `pthread_self` on other Unix systems,
   `GetCurrentThreadId` on Windows. The handle table, the resolved property and event IDs and
   the state of the frames are statics behind `UiCell`, which checks that token and a busy flag
   on every access (the same guarantees as a thread-local `RefCell`: no access from another
   thread, no second access inside one), so a call reads no thread-local variable (a call into
   the dynamic loader on macOS, a table of keys on Android). The check went from 1.4 ns to
   0.6 ns, a retain and a release from 6.7 ns to 4.6 ns, a read from 8.3 ns to 7.3 ns. Every
   guarantee of section 5 stays: a status for every failure, `FERRO_WRONG_THREAD` from another
   thread, `FERRO_BAD_HANDLE` for a stale handle (handles keep their generation), nothing
   unwinds.
6. **One indirect call per event** for a binding with one dispatcher
   (`ferro_subscribe_dispatch(object, event, tag)`): the library calls the dispatcher of
   `ferro_frame_dispatcher` with the tag, with no function of the binding in between.

The write of `Text` measured 5 to 9 ns slower after step 5 in these runs (the write of a
number, the read and every other line got faster); the profile puts the difference at the end
of the lookup of the handle and the property, where the code is the same for both writes, and
no cause was found. It is left to a follow-up measurement on a quiet machine.

### 13.3 What was left out, and why

- **Subtree hashes** (Merkle hashing of each subtree, to skip an unchanged subtree with one
  comparison): a frame is checked record by record anyway (every commit validates the layout
  of what it is given), so an unchanged subtree still costs the walk over its headers, and
  the exact comparison of attribute bytes costs about as much as hashing them. A hash would
  also make a collision a missed update. Memo elements skip subtrees exactly, by the version
  the binding gives.
- **A batch scope** (check the thread and the handle once for several writes): after step 5
  the check of the thread is a comparison and the handle lookup an index, about 2 ns of a
  write of 35 ns; a frame is already the batch for many writes (one check of the thread and
  one lookup of the root per commit).
- **Fine-grained bound values for layer 1** (bind a value once, update it alone): layer 1 is
  already fine-grained, one setter call per changed value.
- **Borrowing the object for the length of a call** in place of counting a reference: a
  setter can run code of the caller (a handler of a property change) that releases the very
  handle, so the counted reference stays.
- **Lazy elements in Osprey** (a function that builds a subtree only when its version changed):
  Osprey 0.29 does not call a function held in a value whose effects it cannot track; the
  `memo(version)` attribute skips writing and reconciling, not building.

### 13.4 Link-time optimisation across C and Rust

A C or C++ application that links the static library can let the linker optimise across the
two languages: build the library with `RUSTFLAGS="-Clinker-plugin-lto"` (and `lto = false` in
the profile, the linker does it) for the target alone (`--target`, so that build scripts are
linked normally), and link with `clang -flto=thin` and `lld`, with a clang and an lld of the
same LLVM version as rustc (`rustc -vV`; for example Rust 1.93 with LLVM 21.1.8 and clang
21.1.8). The small functions of the ABI can then be inlined into the C program. Measured with
`call_overhead.c` (three runs each): a read 8.0 ns dynamic, 8.0 ns static, 7.4 ns with LTO;
a write and a frame within the noise of the three. The call itself is not where the cost of a
call is (section 13.1), and the link of the 100-element benchmark took 52 s on 6 cores, so
it is an option for an application that wants the last half nanosecond, not the default.
It does not apply to Osprey, which calls the dynamic library.

## 14. Signals shared through memory

A signal is a value both sides of the bridge read and write directly: a slot of a table the
library allocates once. A write of a signal from the language is a store, a read of a value
the user changed is a load, and the library learns what changed from a bitset, with no call
per value (#969 follow-up).

### 14.1 What others do

- **Seqlocks and versioned slots** (the Linux kernel, the shared-memory transports of trading
  and robotics systems): a writer bumps a version around a write so that a reader of a
  multi-word value can tell a torn read and retry. A value that fits one aligned word needs no
  seqlock: the store is atomic. A version per slot still tells a reader what changed.
- **Dirty bitsets and change rings**: game engines keep a changed bit per component (or per
  chunk of an ECS component buffer) that a system scans and clears; replication layers and
  audio engines pass parameter changes through a single-producer ring. A bitset holds one bit
  per slot whatever the number of writes; a ring holds one record per write.
- **React Native JSI / Hermes**: JavaScript holds host objects and calls C++ through them
  synchronously; shared state is a C++ object both sides reach, not a copy, but every access is
  still a call through the host object. Reanimated's shared values keep a value on the UI
  thread that worklets read without going through the bridge.
- **Flutter `dart:ffi`**: `Pointer<T>` and `asTypedList` give Dart direct loads and stores into
  native memory; leaf calls are cheap, but memory is cheaper still.
- **Godot GDExtension**: the engine hands the extension a table of function pointers and
  variants are copied per call; it has no shared value table.
- **WebAssembly and `SharedArrayBuffer`**: a UI on the main thread and a worker share a
  buffer and use `Atomics.store`/`Atomics.load`/`Atomics.or` and a change counter, which is the
  protocol below.
- **FerroUI's declarative signals** (`ferroui-view`, `reactive.rs`): a `Signal<T>` handle into
  a runtime of the thread, effects that run when a signal they read changes, and `batch` for
  a group of writes. A slot can feed such a signal at the apply point (the apply sets it and
  its effects run); the C ABI binds slots to properties directly instead, so the library does
  not depend on `ferroui-view`.

### 14.2 The table

`ferro_signal_table(capacity)` returns a `FerroSignalTable` that stays at its address until the
process ends (`ferro_signals_init` for a language that only calls):

| Field | Words | Written by |
| --- | --- | --- |
| `values` | one 8-byte word per slot: `int64_t`, the bits of a `double`, `0`/`1`, a handle; for text the length | the language (slots it binds), the library (slots it watches) |
| `versions` | one per slot, raised every time the value changes | the library |
| `dirty` | one bit per slot | the language sets, the library takes |
| `changed` | one bit per slot | the library sets, the language clears |
| `kinds` | one `uint32_t` per slot (`FERRO_SIGNAL_I64` .. `FERRO_SIGNAL_TEXT`, `0` free) | the library |

`ferro_signal_new(kind)` takes a slot; `ferro_signal_free` gives it back.

- **Language to UI.** The language stores the value (a relaxed atomic store, or a plain store
  on the UI thread), then sets bit `slot % 64` of `dirty[slot / 64]` (an atomic OR with release
  order; a plain read-modify-write when only the UI thread writes). Any thread may write a
  numeric slot this way. `ferro_signal_bind(slot, object, property)` binds a property to the
  slot; a frame does the same with the `bind` record (12: property, slot,
  `ferro_frame_bind`), and keeps the binding while its frames keep the record.
- **The apply** runs on the UI thread at defined points: `ferro_signals_apply()`, the start of
  every frame commit and the end of every dispatch. It takes each non-zero dirty word (an
  atomic swap with acquire order), raises the version of each slot and writes the value to
  every bound property through the typed setter of the property, resolved when the binding was
  made (no handle or ID lookup per write), converted and validated as `ferro_set_f64` does.
- **UI to language.** `ferro_signal_watch(slot, object, property)` subscribes to the changes of
  a registered property: on every change the library writes the value (release order), raises
  the version and sets the `changed` bit. The language reads with a load when it wants; the
  version or the bit says whether anything changed. A text slot keeps the text in the library
  (`ferro_signal_set_str`, `ferro_signal_get_cstr` when the version moved); its value word is
  the length. What an apply writes into a property is not watched back into the same slot, so
  a slot that is bound and watched (a text box) does not echo.
- **Safety.** Every call checks the slot against the capacity and its kind; the apply ignores
  bits of free slots, and `dirty` has exactly `capacity / 64` words, so no index is out of
  bounds by construction. A binding holds a counted reference to its object and is checked
  when it is made; a stale handle is `FERRO_BAD_HANDLE` there as everywhere. Direct stores and
  loads are the language's own memory accesses inside arrays of the length the table states.

### 14.3 What it costs

Measured as section 13 (the same Mac, the median of five runs; the machine was not idle:
other builds ran, load between 8 and 25, single numbers moved up to 30 % on the store line).

| Operation | Through a call | Through the table |
| --- | ---: | ---: |
| Rust: write `FontSize` (setter) / a store and an apply | 34.4 ns | 52.7 ns |
| Rust: 100 writes (100 setters / 100 stores and one apply) | 3.89 us | 4.48 us |
| Rust: read `FontSize` (getter with a fallback / a load of a watched slot) | 7.3 ns | 0.6 ns |
| Rust: one value of a frame of 100 elements (the frame written again / a bound value: a store and an apply) | 3.05 us | 52.5 ns |
| C: write `FontSize` / a store and an apply | 35.8 ns | 47.5 ns |
| C: 100 writes | 4.08 us | 4.28 us |
| C: read `FontSize` / a load | 7.4 ns | 0.3 ns |
| Osprey: 100 writes (100 setters / 100 `signalSetNumber` calls and one apply) | 3.60 us | 3.91 us |
| Osprey: read `FontSize` (getter / `signalReadNumber`) | 11.9 ns | 6.7 ns |

- **Reads pay**: a load of a watched value costs a tenth to a twentieth of a getter call, and
  the language learns what changed from the versions and bits, with no event per value.
- **Bound values in frames pay**: changing one value of a view through a bound slot costs
  the write of that one property, not the description of the view again (58 times less for
  100 elements). This is the fine-grained update that layer 2 lacked.
- **Writes do not pay by themselves**: the property write of the framework is most of a
  setter, and the apply adds the pass over the dirty words and its own bookkeeping, 5 to 15 %
  more for 100 writes. A write through the table is still the way to batch writes from another
  thread (any thread may store a numeric slot; a setter needs the UI thread and `ferro_post`)
  and to update bound frame values; it is not a faster setter.

### 14.4 What was left out

- **An event ring** (events appended to a single-producer ring in shared memory that the
  language drains with loads, next to the callback): implemented and measured, then removed.
  From C a raise into the ring and its drain cost 51.3 ns against 49.7 ns to a C callback, from
  Rust 42.8 against 40.7 ns; a language that cannot read memory (Osprey) drains it with a call
  per event, which is what the callback costs. It may pay where a callback into the language
  is expensive (a managed runtime's upcall); that is to be measured with such a binding.
- **A shared string arena**: a text crosses as a copy into the language's own string either
  way; the version tells when to read it, so a text slot costs one call per change, not per
  read. An arena would add allocation and reuse rules on both sides for no fewer copies.
- **The frame buffer in library memory**: `ferro_frame_commit_bytes` reads the caller's buffer
  in place already; there is no copy at commit to remove.
- **Seqlocks per slot**: every value is one aligned word, so a store is never torn; the text
  of a text slot is read through a call on the UI thread.
- **Feeding `ferroui-view` signals from slots**: possible at the apply point, but it would make
  the C ABI depend on the declarative crate; bindings bind slots to properties directly.

### 14.5 Which path to use

Frames for structure, the signal table for values that change, direct calls for the rest: a
view is built and restructured with frames, a value that changes after that is a bound slot
written with a store, a value the user changes is a watched slot read with a load, and setters
and getters serve setup, one-off writes and reads, and properties frames do not model. The
table of cases, what not to do and what generated bindings do by default are in
docs/bindings-design.md, section 2.1.

The tests are `src/FerroUI.CApi/tests/signals_tests.rs` and `test_signals` of
`samples/CApi/abi_test.c`; the Osprey sample is `samples/CApi/osprey-bindings/temperature_signals`.

## 15. Controls written in another language

A control and its code-behind can be written in C, and through the generated bindings in any
language (#974). The language registers a class on one of four bases, sets the callbacks of the
members it overrides, and registers the properties of the class; the framework then creates,
styles, lays out, renders and routes input to instances of the class as to any control.

### 15.1 How it works without a cost for Rust applications

Nothing outside `ferroui-capi` changed: the class model, the property system and the hot paths
of layout, styling and rendering are untouched.

- **One Rust class per base**, in this crate, written as any Rust control: `ForeignControl:
  Control`, `ForeignPanel: Panel`, `ForeignDecorator: Decorator`, `ForeignContentControl:
  ContentControl`. Their overrides (measure, arrange, render, property changed, attached,
  detached, apply template, pointer and key) read the class of the instance and call its
  callback, or call the base when the class has none.
- **A registered class is a type of its own.** `ferro_class_register(name, length, base)` makes a
  `TypeInfo` at run time with the public constructors of the type system (`TypeInfo::new`, the
  Rust class of the base as its base, a constructor of a fixed pool of 64), registers it with the
  tables of types and copies the virtual table of the Rust class of the base, changing only its
  type slot. Each instance is attached to the copy, so `get_type()` answers the registered
  class: styles select it by name, `ferro_type_id` and `ferro_create` find it, frames create it,
  and `is::<Control>()` holds.
- **The callbacks of a class** are a table of function pointers (`ferro_class_on_measure`, ...)
  set once; each instance holds a pointer to its class and the language's data pointer, so a
  virtual call reads two fields and calls one function: no lookup, no allocation, no boxed value.
- **Registered properties** (`ferro_property_register`) are styled (or attached) properties of the
  property system, owned by an internal class outside every hierarchy and registered on the
  registered class, so they are found by name only there (and on its subclasses; `Owner.Name`
  for attached ones). Setters and getters of the ABI, frames, signals, styles, bindings and
  animations use them as built-in ones; the flags make a change invalidate the measure, the
  arrange or the render, and the property-changed callback runs for the class's own properties
  only.

### 15.2 The calls

| Call | What it does |
| --- | --- |
| `ferro_class_register(name, length, base)` | a class on `FERRO_BASE_CONTROL`, `_PANEL`, `_DECORATOR` or `_CONTENT_CONTROL`: its type ID |
| `ferro_class_set_data(class_id, data)` | the data of the class, given to its create callback |
| `ferro_class_on_create` | each new instance: the callback gives it its data with `ferro_object_set_data(object, data, release)` |
| `ferro_class_on_measure`, `_on_arrange` | `(data, object, width_in, height_in, double *width, double *height)` |
| `ferro_class_on_render` | `(data, object, context)`: draw with `ferro_draw_*` on `context` |
| `ferro_class_on_property_changed` | `(data, object, property_id)` for the class's own properties |
| `ferro_class_on_attached`, `_on_detached`, `_on_apply_template` | `(data, object, 0)` |
| `ferro_class_on_pointer` | `(data, object, kind, x, y, modifiers)`, position relative to the instance; `1` handles it |
| `ferro_class_on_key` | `(data, object, kind, key, modifiers)`; `1` handles it |
| `ferro_property_register(owner, name, length, kind, default_number, default_text, flags)` | a property of a kind of signal (`FERRO_SIGNAL_I64`, `_F64`, `_BOOL`, `_TEXT`) with `FERRO_PROPERTY_AFFECTS_MEASURE`, `_ARRANGE`, `_RENDER`, `_INHERITS`, `_ATTACHED`: its property ID |
| `ferro_object_data(object)` | the data of an instance |
| `ferro_layout_measure`, `_arrange`, `_desired_width`, `_desired_height`, `_bounds_width`, `_bounds_height` | lay out children from the callbacks, read sizes |
| `ferro_layout_result(width, height)` | the result of the running measure or arrange callback, for a language that cannot write a `double *` |
| `ferro_panel_child_count`, `ferro_panel_child` | the children of a class on `FERRO_BASE_PANEL` |
| `ferro_template_part(object, name, length)` | a named part of the applied template, from the apply-template callback on |
| `ferro_draw_brush(argb)`, `ferro_draw_pen(argb, thickness)` | brush and pen IDs (the same colour is the same ID); make them once |
| `ferro_draw_rect`, `_ellipse`, `_line`, `_path`, `_text` | draw (a rounded rectangle, an ellipse, a line, path markup, text in the default typeface) |
| `ferro_draw_push_clip`, `_push_transform`, `_push_opacity`, `ferro_draw_pop` | state; what a callback leaves pushed is popped after it |

### 15.3 The rules for a callback

1. **A plain C function pointer and a data pointer per instance**: no closures and no thunks of
   the library. The data pointer is the language's own object (a `GCHandle` in .NET, a global
   reference in Java, `Unmanaged` in Swift, a `cgo.Handle` in Go), released by the function given
   to `ferro_object_set_data` when the instance goes away.
2. **Scalars only**: no structure passed or returned by value, no varargs; sizes come back through
   `double *` (or `ferro_layout_result`).
3. **Synchronous on the UI thread, and reentrant**: a callback may call the ABI again (measure and
   arrange its children, read and write properties, draw). The library holds none of its own state
   (no `UiCell` access) while it calls out.
4. **Errors as statuses**: a callback returns `0` (or `1` for a handled event), a negative status on
   a failure, which the library records (`ferro_last_error`) and answers with the base behaviour.
   Nothing may unwind out of a callback; the library catches what fails inside its own calls.
5. **What it costs**: an unimplemented member never calls the language; layout callbacks run only
   for invalidated elements; render is retained, called when the element is invalidated, and the
   GPU scene replays what it recorded.

Drawing records into the same retained GPU scene as the built-in controls (the render of a
foreign class runs through the recorder of the compositing renderer): there is no CPU drawing
path. Handles are checked as everywhere, every call is UI-thread only, and an ID or a context
that is not valid is a status.

### 15.4 What it costs

`cargo bench -p ferroui-capi --no-default-features --bench call_overhead`, the same Mac as section
13, the median of five runs (load 9 to 19 while it ran):

| Operation | The control in Rust | Through the ABI |
| --- | ---: | ---: |
| measure and arrange 1000 elements in a stack panel (each invalidated) | 60.2 us | 70.1 us |
| render 1000 elements (a filled rounded rectangle each) into the recorder | 25.0 us | 38.9 us |
| write a number property that affects the measure (`MinWidth` / a registered one) | 30.9 ns | 26.9 ns |
| read it | 7.2 ns | 7.4 ns |

A measure and an arrange through callbacks cost about 10 ns more per element than the same Rust
overrides (two indirect calls, the remembered handle checked, the result out-parameters kept for
`ferro_layout_result`); a render about 14 ns more (the context made reachable to the drawing calls
and the brush looked up by ID). A registered property is a styled property like the built-in ones:
it costs the same.

### 15.5 What is not there yet

- Text input (`on_text_input`), focus members, automation peers and custom hit testing are not
  forwarded; the base runs them.
- More bases (an items control, a templated control without a content) can follow the same pattern.
- Classes are found by name in the tables of types; the XAML loader creates them by name once it
  reads markup through the C ABI (step 7 of #636).
- At most 64 registered classes per process (the pool of constructors).

The tests are `src/FerroUI.CApi/tests/foreign_tests.rs` and `test_controls` of
`samples/CApi/abi_test.c`; the samples are `samples/CApi/gauge.h` (with `gauge.c`, `make -C
samples/CApi run-gauge`) and `samples/CApi/osprey-bindings/gauge_control`.

## 16. Classes by use

The full library (section 1) has the tables of types: every class of the framework is linked and
found by name, and the theme is the whole Fluent theme, so a statically linked C hello world is
as large as an application that uses every control. The by-use mode (#976) links what a program
registers instead.

### 16.1 The entries of the classes

Every class of the API model in the base crate and the controls (455 of them) has an **entry**,
an exported function without arguments that returns its type ID:

```c
int64_t window = ferro_type_window();
int64_t button = ferro_type_button();
int64_t combo  = ferro_type_combo_box();
```

An entry registers the class (it is then found by name too), its default constructor and the
type ID of the model (the constant of the generated bindings), and records the units of the
Fluent theme the class adds to the ambient ones. The model lists the entry of each class
(`entry` in `bindings/model/ferroui-api.json`); the entries are `src/FerroUI.CApi/api_types.rs`,
which `tests/types_tests.rs` writes from the model and compares with the file
(`FERROUI_CAPI_WRITE_TYPES=1 cargo test -p ferroui-capi --lib types` writes it again). The units
of a class are the closure the build tool computes for a Rust application that names the class
(`ferroui_build::theme_roots::kept_units`, docs/porting/theme-trimming.md), over the type models of
the dependencies that the build script of the crate hands to the test: the template of a
`ComboBox` brings the units of its popup, its items and its scroll bars.

### 16.2 The two modes

| | Full (default features) | By use (`--no-default-features --features desktop`) |
| --- | --- | --- |
| Tables of types | on: every class found by name | off: the classes registered by their entries, or created by the framework |
| An entry | returns the ID (the class is known already) | registers the class and its units, returns the ID |
| `ferro_type_id("Button")` | finds it | finds it once its entry was called |
| Theme of `ferro_app_run` | the whole Fluent theme | the ambient units and the units of the registered classes |
| A static program keeps | every class | the classes it registers and what they use |

In the by-use mode a program calls the entries of the classes it uses **before `ferro_app_run`**,
which constructs the theme from the units registered so far. An entry called later registers its
class, but the theme does not grow: its control theme is missing. A class that is not registered
is not found (`FERRO_NOT_FOUND`, with a message that names the entries). Generated bindings do it
for the program: the Osprey bindings of a selection declare the entries of the classes it names
and call them in `registerTypes()`, which `run` calls before the main loop (docs/osprey.md); the
full bindings register nothing. `samples/CApi/hello_world.c` and `gauge.c` call them in `main`.

A class written in another language (section 15) needs no entry: its registration registers it,
and its default constructor is registered beside its type, so it is created in both modes.

### 16.3 What it saves

macOS (arm64), release, the C samples linked statically against `libferroui_capi.a` with
`-Wl,-dead_strip` and stripped; the start-up is the wall time of 15 interleaved runs that close the
window 200 ms after the start callback, less those 200 ms (a loaded machine: compare the columns,
not the absolute numbers):

| Program | Full | By use | Rust hello world |
| --- | ---: | ---: | ---: |
| `hello_world.c` (Window, StackPanel, TextBlock, Button) | 36.97 MB | 20.12 MB | 16.7 to 20.4 MB |
| `gauge.c` (Window, StackPanel, a class written in C) | 36.99 MB | 20.17 MB | |
| a window alone (`ferro_type_window()`) | | 19.99 MB | |
| start-up of `hello_world.c`, median | 238 ms | 235 ms | |
| start-up of `gauge.c`, median | 234 ms | 233 ms | |

What one entry keeps beyond a window (the same link): `Button` and `TextBlock` nothing (a text block
has no control theme, and the closure of the ambient units already keeps the one of a button), `ListBox` 2.03 MB,
`TreeView` 2.12 MB, `TextBox` 2.70 MB, `ComboBox` 2.89 MB, `CalendarDatePicker` 3.23 MB.

By crate (`scripts/capi/size_by_crate.py` over the link map, MB of symbols):

| Crate | Full | By use |
| --- | ---: | ---: |
| ferroui_base | 14.77 | 8.01 |
| ferroui_controls | 6.13 | 1.64 |
| native (Skia, HarfBuzz, codecs) | 4.60 | 4.60 |
| core | 3.78 | 1.77 |
| ferroui_themes_fluent | 2.06 | 0.38 |
| total | 34.89 | 18.97 |

A dynamic library exports every entry, so it keeps every class whatever the program calls: the
by-use mode pays off where the program is linked statically, or with a library built for one
application with the exports of its selection only (#977).
