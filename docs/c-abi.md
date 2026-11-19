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
  generator of a per-application library from it: only those exports, the Cargo features
  that go with them, the roots of the theme units, `type-tables` off. Then a size measurement
  of a hello world built this way against the Rust hello world.
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
