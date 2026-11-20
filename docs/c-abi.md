# The C ABI

FerroUI has one stable C ABI that every language can call: the crate `ferroui-capi`
(`src/FerroUI.CApi`) exports `extern "C"` functions over opaque integer handles, and the C
header `src/FerroUI.CApi/include/ferroui.h` is the contract every binding is written
against, the way Skia, SQLite and wgpu-native are consumed. This document describes phase 1
of #636, with the changes #950 proposed for languages that have neither pointers nor
closures in their FFI (Osprey, WebAssembly hosts, Dart, Go, Python).

Phase 1 is the "full" build of #636: the tables of types are on, so any registered type,
property and routed event can be found by name, and there is no markup. Selecting what an
application uses, and trimming the library to it, is phase 2 (section 11).

## 1. Building the library

```sh
cargo build -p ferroui-capi            # with the desktop platform (Skia, the Fluent theme)
cargo build -p ferroui-capi --no-default-features   # without: no renderer, no ferro_app_run
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
- `ferro_get_f64_or(object, property, fallback)`, `ferro_get_i64_or` and `ferro_get_bool_or`
  return the value itself, and `fallback` when the read fails (the failure is recorded as for
  every call): for a language that cannot pass a pointer to a result (Osprey has no address-of
  operator and its pointer cells hold pointers, not numbers; #950).

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
failure on the thread.

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
- No typed per-control functions: they are generated from the selection in phase 2 and in
  the C++ wrapper.
- No batch of writes (`ferro_set_values`): each scalar write is one direct call, which also
  serves languages without arrays across their FFI (#950).
- Mobile and the browser start through their hosts, not `ferro_app_run`.

## 11. Phase 2 and later

- **Selection and generation (#636, step 2):** a manifest of the controls, properties, events
  and features an application uses (written by hand or collected from its sources), and a
  generator of a per-application library from it: only those exports, the Cargo features
  that go with them, the roots of the theme units, `type-tables` off. Then a size measurement
  of a hello world built this way against the Rust hello world.
- **Bindings:** a header-only C++ wrapper with RAII handles and typed accessors generated from
  the selection; Go (cgo, `runtime.LockOSThread` for the UI thread); Dart and Flutter
  (`dart:ffi`, `ffigen`); C#, Python, Swift and Zig as demand shows.
- **Osprey (#950):** shape 1 works: Osprey executables over the dynamic library with
  named-function callbacks (`samples/CApi/osprey`). Next: generated `extern fn` declarations and
  ID constants from the same source as the header, then FerroUI owning the process with Osprey
  as a library (shape 2).
- **Values:** a `FerroValue` tagged union and `ferro_set_values` beside the scalar family.
